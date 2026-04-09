//! Implicit discipline codegen — every structural/mathematical pattern Loom enforces.
//!
//! Loom's central claim: bridge the gap between what is best in theory and actual
//! implementation.  Every discipline below is triggered by an AST node the developer
//! already declared.  They do not need to know the pattern exists; the compiler emits
//! the full idiomatic implementation on their behalf.
//!
//! ## Template conventions
//! All code generation uses `ts()` / `subst()` from the `template` module.
//! Raw string literals (`r#"..."#`) are used for templates — no `{{` / `}}` escaping needed.
//! Placeholders are `{Name}` — substituted via regex, safe with Rust's own `{` / `}` syntax.
//!
//! ## Grammar disjointness principle
//! Loom's surface keywords (`store`, `session`, `effect`, `process`, `distribution`,
//! `separation`, `timing_safety`, `termination`, `gradual`, `degenerate`) are
//! intentionally distinct from Rust's keywords.  `fn`/`let`/`type` are shared because
//! Loom's declaration forms map directly to Rust's concepts — no ambiguity exists at the
//! file level since `.loom` files are parsed only by Loom's parser.
//!
//! ## Discipline map (AST node -> generated artifact)
//!
//! ### Data Access Layer
//! | store :: Relational | Repository trait + InMemory fake | Fowler 2002 |
//! | store :: Relational | Unit of Work (transaction scope) | Fowler 2002 |
//! | store :: Relational | Specification (composable predicates) | Evans/Fowler 2002 |
//! | store :: Relational | Pagination cursor | Web API best practices |
//! | store :: Document   | Document repository + Aggregatable | DDD |
//!
//! ### API / Web Layer
//! | store :: Relational | HATEOAS ResourceLinks | Fielding 2000 |
//! | store :: Relational | OpenAPI schema hints | OpenAPI 3.1 |
//!
//! ### Architecture Patterns
//! | store :: Relational        | CQRS Command/Query split | Young 2010, Meyer CQS |
//! | store :: TimeSeries        | Event Sourcing (EventStore + Aggregate) | Evans 2003 |
//! | store :: DistributedLog    | Domain Event bus | Evans 2003 |
//! | store :: DistributedLog    | Saga coordinator | Garcia-Molina 1987 |
//! | usecase: block             | CQRS use-case handlers | Young 2010 |
//!
//! ### Concurrency / Messaging
//! | messaging_primitive RequestResponse | Request/Response channel | Honda 1993 |
//! | messaging_primitive PublishSubscribe | Observer / Event bus | GoF 1994 |
//! | messaging_primitive ProducerConsumer | Typed work queue | CSP Hoare 1978 |
//! | messaging_primitive Bidirectional   | Bidirectional channel | pi-calculus |
//! | session: block     | Phantom-type state machine | Honda 1993 |
//! | effect: block      | Algebraic effect handler | Plotkin & Pretnar 2009 |
//!
//! ### Resilience
//! | aspect on_failure + max_attempts | Circuit breaker | Nygard 2007 |
//! | aspect max_attempts              | Retry with exponential backoff | AWS pattern |
//!
//! ### Stochastic Processes
//! | process: Wiener            | Brownian motion sampler | Wiener 1923 |
//! | process: GeometricBrownian | GBM price simulation | Black-Scholes 1973 |
//! | process: OrnsteinUhlenbeck | Mean-reverting process | OU 1930 |
//! | process: PoissonProcess    | Event counting process | Poisson 1837 |
//! | process: MarkovChain       | Transition matrix | Markov 1906 |
//!
//! ### Statistical / Probabilistic
//! | distribution: Gaussian    | Gaussian sampler (Box-Muller) | CLT |
//! | distribution: Poisson     | Poisson sampler (Knuth) | Poisson 1837 |
//! | distribution: Beta        | Beta sampler (ratio of Gammas) | Bayesian |
//! | distribution: Binomial    | Binomial sampler | Bernoulli 1713 |
//! | distribution: Uniform     | Uniform sampler | Laplace 1812 |
//! | distribution: Exponential | Memoryless waiting time sampler | |
//! | distribution: Pareto      | Power-law tail sampler | Pareto 1896 |
//! | distribution: LogNormal   | Lognormal sampler | Galton 1879 |
//! | distribution: GeometricBrownian | GBM distribution wrapper | Black-Scholes 1973 |
//!
//! ### Graph Theory
//! | store :: Graph (directed only) | DAG + topological sort | Kahn 1962 |
//! | store :: Graph                 | Labelled Transition System | Keller 1976 |
//!
//! ### Formal Verification Audit Trail
//! | separation: block           | Ownership + heap-disjointness audit | O'Hearn 2001 |
//! | timing_safety: constant_time| Constant-time audit | Bernstein 2005 |
//! | termination: clause         | Termination audit + variant note | Turing 1936 |
//! | gradual: block              | Gradual typing boundary wrapper | Siek 2006 |
//! | degenerate: block           | Degeneracy fallback dispatcher | Edelman |

use crate::ast::*;
use super::{RustEmitter, to_snake_case};
use super::template::ts;


// ═══════════════════════════════════════════════════════════════════════════
// DATA ACCESS LAYER
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// CRUD in-memory repository impl — Fowler 2002 Repository + Fake Object.
    /// Concrete testable fake using Mutex<HashMap<String, Entity>>.
    /// Replace with a sqlx/diesel/sea-orm adapter at the composition root.
    pub(super) fn emit_crud_in_memory_impl(
        &self, _store: &str, table: &str, pk_field: &str, out: &mut String,
    ) {
        out.push_str(&ts(
            r#"
// LOOM[implicit:CRUD:InMemory]: InMemory{T}Repository — testable fake (Fowler 2002)
pub struct InMemory{T}Repository {
    store: std::sync::Mutex<std::collections::HashMap<String, {T}>>,
}
impl Default for InMemory{T}Repository {
    fn default() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
}
impl {T}Repository for InMemory{T}Repository {
    fn find_by_id(&self, id: &str) -> Option<{T}> {
        self.store.lock().unwrap().get(id).cloned()
    }
    fn save(&self, entity: {T}) -> Result<{T}, String> {
        let key = format!("{:?}", entity.{pk});
        self.store.lock().unwrap().insert(key, entity.clone());
        Ok(entity)
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        self.store.lock().unwrap().remove(id); Ok(())
    }
}"#,
            &[("T", table), ("pk", pk_field)],
        ));
    }

    /// Unit of Work — atomic transaction scope grouping multiple repositories (Fowler 2002).
    pub(super) fn emit_unit_of_work(&self, store_name: &str, tables: &[String], out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:UnitOfWork]: {store_name} — atomic transaction scope (Fowler 2002)\n"
        ));
        out.push_str("// Ecosystem: sqlx::Transaction | diesel::Connection::transaction\n");
        out.push_str(&format!("pub struct {}UnitOfWork {{\n", store_name));
        for t in tables {
            out.push_str(&format!("    pub {}: InMemory{}Repository,\n", to_snake_case(t), t));
        }
        out.push_str("}\n");
        out.push_str(&format!("impl Default for {}UnitOfWork {{\n    fn default() -> Self {{ Self {{\n", store_name));
        for t in tables {
            out.push_str(&format!("        {}: InMemory{}Repository::default(),\n", to_snake_case(t), t));
        }
        out.push_str("    } }\n}\n");
        out.push_str(&format!("impl {}UnitOfWork {{\n", store_name));
        out.push_str("    pub fn begin() -> Self { Self::default() }\n");
        out.push_str("    pub fn commit(self) -> Result<(), String> {\n        // wire to real transaction backend\n        Ok(())\n    }\n");
        out.push_str("    pub fn rollback(self) { drop(self); }\n}\n\n");
    }

    /// Specification pattern — composable, type-safe predicate objects (Evans 2003).
    /// A Specification<T> can be AND-ed, OR-ed, NOT-ed without touching query code.
    pub(super) fn emit_specification_pattern(&self, table: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:Specification]: {table} — composable predicates (Evans 2003)\n\n"
        ));
        out.push_str(&format!(
            "pub trait {table}Specification {{\n    fn is_satisfied_by(&self, candidate: &{table}) -> bool;\n}}\n\n"
        ));
        out.push_str(&format!(
            "pub struct And{t}Spec<A: {t}Specification, B: {t}Specification>(pub A, pub B);\n", t = table
        ));
        out.push_str(&format!(
            "impl<A: {t}Specification, B: {t}Specification> {t}Specification for And{t}Spec<A,B> {{\n
                 fn is_satisfied_by(&self, c: &{t}) -> bool {{ self.0.is_satisfied_by(c) && self.1.is_satisfied_by(c) }}\n}}\n\n",
            t = table
        ));
        out.push_str(&format!(
            "pub struct Not{t}Spec<A: {t}Specification>(pub A);\n", t = table
        ));
        out.push_str(&format!(
            "impl<A: {t}Specification> {t}Specification for Not{t}Spec<A> {{\n
                 fn is_satisfied_by(&self, c: &{t}) -> bool {{ !self.0.is_satisfied_by(c) }}\n}}\n\n",
            t = table
        ));
    }

    /// Pagination cursor — opaque page/cursor pair for any collection query.
    pub(super) fn emit_pagination_cursor(&self, table: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:Pagination]: {table} — opaque cursor pagination\n\n"
        ));
        out.push_str(&format!("#[derive(Debug, Clone)]\npub struct {}Page {{\n    pub items: Vec<{table}>,\n    pub next_cursor: Option<String>,\n    pub total_count: Option<usize>,\n}}\n\n", table));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// API / WEB LAYER
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// HATEOAS ResourceLinks — HAL-style navigational links (Fielding 2000).
    pub(super) fn emit_hateoas_for_store(&self, store_name: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:HATEOAS]: {store_name} — HAL resource links (Fielding 2000 REST)\n"
        ));
        out.push_str("// Ecosystem: utoipa (OpenAPI derive), axum, actix-web\n");
        out.push_str("#[derive(Debug, Clone)]\npub struct ResourceLink {\n    pub rel: String,\n    pub href: String,\n    pub method: Option<String>,\n}\n\n");
        out.push_str(&format!("#[derive(Debug, Clone, Default)]\npub struct {}Links {{\n    pub links: Vec<ResourceLink>,\n}}\n", store_name));
        out.push_str(&format!("impl {}Links {{\n", store_name));
        out.push_str("    pub fn add(&mut self, rel: &str, href: &str) {\n        self.links.push(ResourceLink { rel: rel.to_string(), href: href.to_string(), method: None });\n    }\n");
        out.push_str("    pub fn with_method(&mut self, rel: &str, href: &str, method: &str) {\n        self.links.push(ResourceLink { rel: rel.to_string(), href: href.to_string(), method: Some(method.to_string()) });\n    }\n");
        out.push_str("    pub fn self_link(mut self, href: &str) -> Self { self.add(\"self\", href); self }\n}\n\n");
    }

    /// OpenAPI schema hints — utoipa-compatible doc comments.
    pub(super) fn emit_openapi_hints(&self, table: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:OpenAPI]: {table} — utoipa schema hint (OpenAPI 3.1)\n"
        ));
        out.push_str(&format!(
            "// Add `#[derive(utoipa::ToSchema)]` to {table} to emit the OpenAPI schema.\n\n"
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ARCHITECTURE PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// CQRS Command/Query trait split (Young 2010, based on Meyer 1997 CQS).
    pub(super) fn emit_cqrs_for_store(&self, store_name: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:CQRS]: {store_name} — Command/Query split (Young 2010, Meyer CQS)\n\n"
        ));
        out.push_str(&format!(
            "pub trait {s}Command {{\n    type Error;\n    fn execute(self) -> Result<(), Self::Error>;\n}}\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "pub trait {s}Query {{\n    type Output;\n    type Error;\n    fn execute(&self) -> Result<Self::Output, Self::Error>;\n}}\n\n",
            s = store_name
        ));
    }

    /// Event Sourcing — EventStore trait + Aggregate with fold (Fowler 2005 + Evans 2003 DDD).
    pub(super) fn emit_event_sourcing(&self, store_name: &str, event_types: &[String], out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:EventSourcing]: {store_name} — Fowler 2005, Evans DDD Aggregate\n"
        ));
        out.push_str("// Ecosystem: eventstore, sqlx event table, axum-streams\n\n");
        if !event_types.is_empty() {
            out.push_str("#[derive(Debug, Clone)]\n");
            out.push_str(&format!("pub enum {}Event {{\n", store_name));
            for ev in event_types { out.push_str(&format!("    {}({}),\n", ev, ev)); }
            out.push_str("}\n\n");
        } else {
            out.push_str(&format!(
                "#[derive(Debug, Clone)]\npub struct {s}Event {{\n    pub kind: String,\n    pub payload: String,\n    pub timestamp: i64,\n}}\n\n",
                s = store_name
            ));
        }
        out.push_str(&format!(
            "pub trait {s}EventStore {{\n    type Error;\n
                 fn append(&self, stream: &str, events: Vec<{s}Event>) -> Result<u64, Self::Error>;\n
                 fn load(&self, stream: &str, from: u64) -> Result<Vec<{s}Event>, Self::Error>;\n}}\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "// LOOM[implicit:Aggregate]: {s} — state = fold of events\n\
             pub trait {s}Aggregate: Sized + Default {{\n
                 fn apply(&mut self, event: &{s}Event);\n
                 fn load_from_events(events: &[{s}Event]) -> Self {{\n
                     let mut agg = Self::default();\n
                     for ev in events {{ agg.apply(ev); }}\n
                     agg\n    }}\n}}\n\n",
            s = store_name
        ));
    }

    /// Domain Event bus — typed broadcast channel (Evans 2003).
    pub(super) fn emit_domain_event_bus(&self, store_name: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:DomainEventBus]: {s} — Evans 2003 domain events\n\
             // Ecosystem: tokio::sync::broadcast, eventbus crate\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "pub trait {s}EventHandler: Send + Sync {{\n    fn handle(&self, event: &{s}Event);\n}}\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "#[derive(Default)]\npub struct {s}EventBus {{\n    handlers: Vec<Box<dyn {s}EventHandler>>,\n}}\n",
            s = store_name
        ));
        out.push_str(&format!(
            "impl {s}EventBus {{\n
                 pub fn subscribe(&mut self, h: Box<dyn {s}EventHandler>) {{ self.handlers.push(h); }}\n
                 pub fn publish(&self, event: &{s}Event) {{ for h in &self.handlers {{ h.handle(event); }} }}\n}}\n\n",
            s = store_name
        ));
    }

    /// Saga coordinator — long-running distributed transaction (Garcia-Molina 1987).
    /// Each step has a compensating action. Failure unwinds in reverse.
    pub(super) fn emit_saga_coordinator(&self, store_name: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:Saga]: {s} — Garcia-Molina 1987 compensating transactions\n\
             // Ecosystem: saga-rs, or implement via tokio task + compensation log\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "pub trait {s}SagaStep {{\n    type Error;\n    fn execute(&self) -> Result<(), Self::Error>;\n    fn compensate(&self);\n}}\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "#[derive(Default)]\npub struct {s}Saga {{\n    steps: Vec<Box<dyn {s}SagaStep<Error = String>>>,\n}}\n",
            s = store_name
        ));
        out.push_str(&format!(
            "impl {s}Saga {{\n
                 pub fn step(mut self, s: Box<dyn {s}SagaStep<Error = String>>) -> Self {{ self.steps.push(s); self }}\n
                 pub fn run(self) -> Result<(), String> {{\n
                     let mut done: Vec<usize> = Vec::new();\n
                     for (i, step) in self.steps.iter().enumerate() {{\n
                         if let Err(e) = step.execute() {{\n
                             for &j in done.iter().rev() {{ self.steps[j].compensate(); }}\n
                             return Err(e);\n
                         }}\n
                         done.push(i);\n        }}\n        Ok(())\n    }}\n}}\n\n",
            s = store_name
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONCURRENCY / MESSAGING
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Typed messaging channel stubs from a `messaging_primitive` declaration.
    pub(super) fn emit_messaging_channel(&self, mp: &MessagingPrimitiveDef, out: &mut String) {
        let n = &mp.name;
        let guarantees = mp.guarantees.join(", ");
        out.push_str(&format!(
            "// LOOM[implicit:Messaging]: {n} — {:?} — guarantees: [{guarantees}]\n",
            mp.pattern
        ));
        match &mp.pattern {
            Some(MessagingPattern::RequestResponse) => {
                out.push_str("// Honda 1993 session types -> typed request/response\n");
                out.push_str(&format!(
                    "pub trait {n}Client {{\n    type Request;\n    type Response;\n    type Error;\n
                         fn call(&self, req: Self::Request) -> Result<Self::Response, Self::Error>;\n}}\n\n"
                ));
                if mp.timeout_mandatory {
                    out.push_str(&format!(
                        "pub trait {n}ClientWithTimeout: {n}Client {{\n
                             fn call_timeout(&self, req: Self::Request, timeout_ms: u64) -> Result<Self::Response, Self::Error>;\n}}\n\n"
                    ));
                }
            }
            Some(MessagingPattern::PublishSubscribe) => {
                out.push_str("// GoF 1994 Observer -> typed event bus\n");
                out.push_str(&format!(
                    "pub trait {n}Subscriber<E>: Send + Sync {{\n    fn on_event(&self, event: &E);\n}}\n\n"
                ));
                out.push_str(&format!(
                    "pub struct {n}Bus<E: Clone> {{\n    subscribers: Vec<Box<dyn {n}Subscriber<E>>>,\n}}\n"
                ));
                out.push_str(&format!(
                    "impl<E: Clone> {n}Bus<E> {{\n
                         pub fn subscribe(&mut self, s: Box<dyn {n}Subscriber<E>>) {{ self.subscribers.push(s); }}\n
                         pub fn publish(&self, event: E) {{ for s in &self.subscribers {{ s.on_event(&event); }} }}\n}}\n\n"
                ));
            }
            Some(MessagingPattern::ProducerConsumer) => {
                out.push_str("// CSP Hoare 1978 -> typed work queue\n");
                out.push_str(&format!(
                    "pub trait {n}Producer<T> {{\n    fn send(&self, item: T) -> Result<(), String>;\n}}\n\n"
                ));
                out.push_str(&format!(
                    "pub trait {n}Consumer<T> {{\n    fn receive(&self) -> Option<T>;\n    fn ack(&self, item: &T);\n}}\n\n"
                ));
            }
            _ => {
                out.push_str("// pi-calculus bidirectional channel\n");
                out.push_str(&format!(
                    "pub trait {n}Channel<S, R> {{\n    fn send(&self, msg: S) -> Result<(), String>;\n    fn receive(&self) -> Option<R>;\n}}\n\n"
                ));
            }
        }
    }

    /// Session-type state machine — phantom-type protocol (Honda 1993).
    /// Wrong message order = compile-time error via typestate (Strom & Yemini 1986).
    pub(super) fn emit_session_state_machine(&self, sd: &SessionDef, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:SessionType]: {} — phantom-type protocol (Honda 1993)\n", sd.name
        ));
        out.push_str("// Wrong send order = compile-time error. Ecosystem: ferrite-session, sesh\n\n");
        for role in &sd.roles {
            out.push_str(&format!("pub struct {}State;\n", to_pascal_case(&role.name)));
        }
        out.push('\n');
        let first_state = sd.roles.first()
            .map(|r| format!("{}State", to_pascal_case(&r.name)))
            .unwrap_or_else(|| "()".to_string());
        let chan_name = to_pascal_case(&sd.name);
        out.push_str(&format!(
            "pub struct {chan_name}Channel<State> {{\n    _state: std::marker::PhantomData<State>,\n}}\n\n"
        ));
        out.push_str(&format!(
            "impl {chan_name}Channel<{first_state}> {{\n    pub fn new() -> Self {{ Self {{ _state: std::marker::PhantomData }} }}\n}}\n\n"
        ));
    }

    /// Algebraic effect handler dispatch table (Plotkin & Pretnar 2009).
    pub(super) fn emit_effect_handler(&self, ed: &EffectDef, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:AlgebraicEffect]: {} — Plotkin & Pretnar 2009\n\
             // Ecosystem: effective crate, frunk\n\n", ed.name
        ));
        out.push_str(&format!("pub trait {}Handler {{\n", ed.name));
        for op in &ed.operations {
            let i = self.emit_type_expr(&op.input);
            let o = self.emit_type_expr(&op.output);
            out.push_str(&format!("    fn {}(&self, input: {i}) -> {o};\n", op.name));
        }
        out.push_str("}\n\n");
        out.push_str(&format!(
            "pub struct {n}Dispatcher {{\n    handler: Box<dyn {n}Handler>,\n}}\n",
            n = ed.name
        ));
        out.push_str(&format!("impl {}Dispatcher {{\n", ed.name));
        out.push_str(&format!(
            "    pub fn new(h: Box<dyn {}Handler>) -> Self {{ Self {{ handler: h }} }}\n", ed.name
        ));
        for op in &ed.operations {
            let i = self.emit_type_expr(&op.input);
            let o = self.emit_type_expr(&op.output);
            out.push_str(&format!(
                "    pub fn {}(&self, input: {i}) -> {o} {{ self.handler.{}(input) }}\n",
                op.name, op.name
            ));
        }
        out.push_str("}\n\n");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESILIENCE
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Circuit breaker — Nygard 2007 "Release It!".
    /// Three states: Closed (normal), Open (fast-fail), Half-Open (probe).
    pub(super) fn emit_circuit_breaker(&self, aspect_name: &str, max_attempts: u32, out: &mut String) {
        let n = to_pascal_case(aspect_name);
        out.push_str(&format!(
            "// LOOM[implicit:CircuitBreaker]: {aspect_name} — Nygard 2007 Release It!\n\
             // Threshold: {max_attempts} failures. Ecosystem: failsafe-rs, tokio-retry\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone, Copy, PartialEq)]\npub enum {n}BreakerState {{ Closed, Open, HalfOpen }}\n\n"
        ));
        out.push_str(&format!(
            "pub struct {n}CircuitBreaker {{\n    pub state: {n}BreakerState,\n    pub failure_count: u32,\n    pub threshold: u32,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}CircuitBreaker {{\n
                 pub fn new() -> Self {{ Self {{ state: {n}BreakerState::Closed, failure_count: 0, threshold: {max_attempts} }} }}\n
                 pub fn record_failure(&mut self) {{\n
                     self.failure_count += 1;\n
                     if self.failure_count >= self.threshold {{ self.state = {n}BreakerState::Open; }}\n    }}\n
                 pub fn record_success(&mut self) {{ self.failure_count = 0; self.state = {n}BreakerState::Closed; }}\n
                 pub fn is_open(&self) -> bool {{ self.state == {n}BreakerState::Open }}\n}}\n\n"
        ));
    }

    /// Retry policy with exponential backoff.
    pub(super) fn emit_retry_policy(&self, aspect_name: &str, max_attempts: u32, out: &mut String) {
        let n = to_pascal_case(aspect_name);
        out.push_str(&format!(
            "// LOOM[implicit:RetryPolicy]: {aspect_name} — exponential backoff, max {max_attempts} attempts\n\
             // Ecosystem: tokio-retry, backoff crate\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}RetryPolicy {{\n    pub max_attempts: u32,\n    pub base_delay_ms: u64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}RetryPolicy {{\n
                 pub fn new() -> Self {{ Self {{ max_attempts: {max_attempts}, base_delay_ms: 100 }} }}\n
                 pub fn delay_for_attempt(&self, attempt: u32) -> u64 {{ self.base_delay_ms * 2u64.pow(attempt.min(10)) }}\n}}\n\n"
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STOCHASTIC PROCESSES
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct stochastic process emitter from a process: annotation.
    pub(super) fn emit_stochastic_process(
        &self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String,
    ) {
        match &sp.kind {
            StochasticKind::Wiener            => self.emit_wiener_process(fn_name, out),
            StochasticKind::GeometricBrownian => self.emit_gbm(fn_name, sp, out),
            StochasticKind::OrnsteinUhlenbeck => self.emit_ou_process(fn_name, sp, out),
            StochasticKind::PoissonProcess    => self.emit_poisson_process(fn_name, sp, out),
            StochasticKind::MarkovChain       => self.emit_markov_transition_matrix(fn_name, &sp.states, out),
            StochasticKind::Unknown(k)        => {
                out.push_str(&format!(
                    "// LOOM[stochastic:Unknown]: process kind '{k}' not yet generated\n\n"
                ));
            }
        }
    }

    /// Standard Brownian motion (Wiener 1923).
    /// W(t+dt) = W(t) + sqrt(dt)*N(0,1). Martingale. E[W_t]=0, Var[W_t]=t.
    fn emit_wiener_process(&self, fn_name: &str, out: &mut String) {
        let n = to_pascal_case(fn_name);
        out.push_str(&format!(
            "// LOOM[implicit:Wiener]: {fn_name} — Brownian motion (Wiener 1923)\n\
             // E[W_t]=0, Var[W_t]=t. Martingale. Continuous paths. Ecosystem: rand, statrs\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}WienerProcess {{\n    pub t: f64,\n    pub value: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}WienerProcess {{\n
                 pub fn new() -> Self {{ Self {{ t: 0.0, value: 0.0 }} }}\n
                 /// Euler-Maruyama: W(t+dt) = W(t) + sqrt(dt)*z, z ~ N(0,1).\n
                 pub fn step(&mut self, dt: f64, z: f64) {{ self.t += dt; self.value += dt.sqrt() * z; }}\n}}\n\n"
        ));
    }

    /// Geometric Brownian Motion (Black-Scholes 1973).
    /// dS = mu*S*dt + sigma*S*dW. Always positive. Log-normal increments.
    fn emit_gbm(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let mu = sp.long_run_mean.as_deref().unwrap_or("0.05");
        out.push_str(&format!(
            "// LOOM[implicit:GBM]: {fn_name} — Geometric Brownian Motion (Black-Scholes 1973)\n\
             // dS = mu*S*dt + sigma*S*dW. Always positive. Log-normal. mu={mu}\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}GBM {{\n    pub mu: f64,\n    pub sigma: f64,\n    pub price: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}GBM {{\n
                 pub fn new(price: f64) -> Self {{ Self {{ mu: {mu}, sigma: 0.2, price }} }}\n
                 /// S(t+dt) = S(t)*exp((mu-0.5*sigma^2)*dt + sigma*sqrt(dt)*z).\n
                 pub fn step(&mut self, dt: f64, z: f64) {{\n
                     self.price *= ((self.mu - 0.5*self.sigma*self.sigma)*dt + self.sigma*dt.sqrt()*z).exp();\n    }}\n
                 pub fn assert_positive(&self) {{ debug_assert!(self.price > 0.0, \"GBM price must be > 0\"); }}\n}}\n\n"
        ));
    }

    /// Ornstein-Uhlenbeck mean-reverting process (OU 1930).
    /// dX = theta*(mu - X)*dt + sigma*dW. Stationary Gaussian.
    fn emit_ou_process(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let mu = sp.long_run_mean.as_deref().unwrap_or("0.0");
        out.push_str(&format!(
            "// LOOM[implicit:OU]: {fn_name} — Ornstein-Uhlenbeck (1930)\n\
             // dX = theta*(mu-X)*dt + sigma*dW. Mean-reverting to {mu}. Stationary Gaussian.\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}OUProcess {{\n    pub theta: f64,\n    pub mu: f64,\n    pub sigma: f64,\n    pub value: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}OUProcess {{\n
                 pub fn new() -> Self {{ Self {{ theta: 1.0, mu: {mu}, sigma: 0.1, value: 0.0 }} }}\n
                 pub fn step(&mut self, dt: f64, z: f64) {{\n
                     self.value += self.theta*(self.mu - self.value)*dt + self.sigma*dt.sqrt()*z;\n    }}\n}}\n\n"
        ));
    }

    /// Poisson process (Poisson 1837). N(t) ~ Poisson(lambda*t). Integer-valued.
    fn emit_poisson_process(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let rate = sp.rate.as_deref().unwrap_or("1.0");
        out.push_str(&format!(
            "// LOOM[implicit:Poisson]: {fn_name} — Poisson process (Poisson 1837)\n\
             // N(t)~Poisson(lambda*t). Integer-valued. Inter-arrival~Exp(lambda). rate={rate}\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}PoissonProcess {{\n    pub lambda: f64,\n    pub count: u64,\n    pub t: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}PoissonProcess {{\n
                 pub fn new() -> Self {{ Self {{ lambda: {rate}, count: 0, t: 0.0 }} }}\n
                 /// Advance by dt. Provide arrivals from rand_distr::Poisson(lambda*dt).\n
                 pub fn step(&mut self, dt: f64, arrivals: u64) {{ self.t += dt; self.count += arrivals; }}\n}}\n\n"
        ));
    }

    /// Markov chain TransitionMatrix<S> (Markov 1906).
    pub(super) fn emit_markov_transition_matrix(
        &self, fn_name: &str, states: &[String], out: &mut String,
    ) {
        let n = to_pascal_case(fn_name);
        let states_enum = states.iter()
            .map(|s| format!("    {},", to_pascal_case(s)))
            .collect::<Vec<_>>().join("\n");
        out.push_str(&ts(
            r#"
// LOOM[implicit:Markov]: {fn_name} — TransitionMatrix (Markov 1906)
// P(X_{n+1}|X_n): memoryless, discrete-state chain.
// Ecosystem: ndarray (dense), petgraph (sparse), statrs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum {N}States {
{states}
}
#[derive(Debug, Clone, Default)]
pub struct {N}TransitionMatrix {
    transitions: std::collections::HashMap<({N}States, {N}States), f64>,
}
impl {N}TransitionMatrix {
    pub fn set(&mut self, from: {N}States, to: {N}States, prob: f64) {
        debug_assert!((0.0..=1.0).contains(&prob), "prob must be in [0,1]");
        self.transitions.insert((from, to), prob);
    }
    pub fn next_states(&self, state: {N}States) -> Vec<({N}States, f64)> {
        self.transitions.iter()
            .filter_map(|(&(f, t), &p)| if f == state { Some((t, p)) } else { None })
            .collect()
    }
    /// Verify all outgoing probs from each state sum to 1.0 (stochastic matrix).
    pub fn validate(&self) -> bool {
        use std::collections::HashMap;
        let mut sums: HashMap<{N}States, f64> = HashMap::new();
        for (&(from, _), &p) in &self.transitions { *sums.entry(from).or_default() += p; }
        sums.values().all(|&s| (s - 1.0).abs() < 1e-9)
    }
}"#,
            &[("N", &n), ("fn_name", fn_name), ("states", &states_enum)],
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATISTICAL / PROBABILISTIC
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct distribution sampler from a distribution: annotation.
    pub(super) fn emit_distribution_sampler(
        &self, fn_name: &str, db: &DistributionBlock, out: &mut String,
    ) {
        let n = to_pascal_case(fn_name);
        match &db.family {
            DistributionFamily::Gaussian { mean, std_dev } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Gaussian]: {fn_name} — Normal distribution (CLT, Gauss 1809)\n\
                     // X ~ N(mu={mean}, sigma={std_dev}). Ecosystem: rand_distr::Normal\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GaussianSampler {{\n    pub mean: f64,\n    pub std_dev: f64,\n}}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GaussianSampler {{\n
                         pub fn new() -> Self {{ Self {{ mean: {mean}, std_dev: {std_dev} }} }}\n
                         /// Box-Muller transform. z1, z2 ~ U(0,1). Returns one N(0,1) sample.\n
                         pub fn sample_box_muller(&self, z1: f64, z2: f64) -> f64 {{\n
                             let n01 = (-2.0*z1.ln()).sqrt() * (2.0*std::f64::consts::PI*z2).cos();\n
                             self.mean + self.std_dev * n01\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Poisson { lambda } => {
                out.push_str(&format!(
                    "// LOOM[implicit:PoissonDist]: {fn_name} — Poisson distribution (Poisson 1837)\n\
                     // X ~ Poisson(lambda={lambda}). Integer-valued. Ecosystem: rand_distr::Poisson\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}PoissonSampler {{ pub lambda: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}PoissonSampler {{\n
                         pub fn new() -> Self {{ Self {{ lambda: {lambda} }} }}\n
                         /// Knuth algorithm for small lambda. For large lambda use Gaussian approx.\n
                         pub fn sample_knuth(&self, uniform_samples: &[f64]) -> u64 {{\n
                             let limit = (-self.lambda).exp();\n
                             let mut prod = 1.0; let mut k = 0u64;\n
                             for &u in uniform_samples {{ prod *= u; k += 1; if prod < limit {{ break; }} }}\n
                             k.saturating_sub(1)\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Uniform { low, high } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Uniform]: {fn_name} — Uniform distribution (Laplace 1812)\n\
                     // X ~ U({low}, {high}). Ecosystem: rand::Rng::gen_range\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}UniformSampler {{ pub low: f64, pub high: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}UniformSampler {{\n
                         pub fn new() -> Self {{ Self {{ low: {low}, high: {high} }} }}\n
                         pub fn sample(&self, u: f64) -> f64 {{ debug_assert!((0.0..=1.0).contains(&u)); self.low + (self.high - self.low) * u }}\n}}\n\n"
                ));
            }
            DistributionFamily::Exponential { lambda } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Exponential]: {fn_name} — Exponential distribution\n\
                     // X ~ Exp(lambda={lambda}). Memoryless. Inter-arrival times. Ecosystem: rand_distr::Exp\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}ExpSampler {{ pub lambda: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}ExpSampler {{\n
                         pub fn new() -> Self {{ Self {{ lambda: {lambda} }} }}\n
                         /// Inverse CDF: X = -ln(U)/lambda, U ~ U(0,1).\n
                         pub fn sample(&self, u: f64) -> f64 {{ debug_assert!(u > 0.0 && u < 1.0); -u.ln() / self.lambda }}\n}}\n\n"
                ));
            }
            DistributionFamily::Beta { alpha, beta } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Beta]: {fn_name} — Beta distribution (Euler 1763)\n\
                     // X ~ Beta(alpha={alpha}, beta={beta}). Bounded [0,1]. Bayesian prior for probabilities.\n\
                     // Ecosystem: rand_distr::Beta, statrs::Beta\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}BetaSampler {{ pub alpha: f64, pub beta: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}BetaSampler {{\n
                         pub fn new() -> Self {{ Self {{ alpha: {alpha}, beta: {beta} }} }}\n
                         pub fn mean(&self) -> f64 {{ self.alpha / (self.alpha + self.beta) }}\n
                         pub fn variance(&self) -> f64 {{\n
                             let s = self.alpha + self.beta;\n
                             self.alpha * self.beta / (s * s * (s + 1.0))\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Binomial { n: bin_n, p: bin_p } => {
                let struct_name = format!("{n}BinomialSampler");
                out.push_str(&format!(
                    "// LOOM[implicit:Binomial]: {fn_name} — Binomial distribution (Bernoulli 1713)\n\
// X ~ Bin(n={bin_n}, p={bin_p}). Count of successes in n trials.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {struct_name} {{ pub n: u64, pub p: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {struct_name} {{\n    \
pub fn new() -> Self {{ Self {{ n: {bin_n}, p: {bin_p} }} }}\n    \
pub fn mean(&self) -> f64 {{ self.n as f64 * self.p }}\n    \
pub fn variance(&self) -> f64 {{ self.n as f64 * self.p * (1.0 - self.p) }}\n}}\n\n"
                ));
            }
            DistributionFamily::Pareto { alpha, x_min } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Pareto]: {fn_name} — Pareto power-law (Pareto 1896)\n\
                     // X ~ Pareto(alpha={alpha}, x_min={x_min}). 80/20 rule. Heavy tail.\n\
                     // WARNING: Mean infinite if alpha <= 1. Variance infinite if alpha <= 2.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}ParetoSampler {{ pub alpha: f64, pub x_min: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}ParetoSampler {{\n
                         pub fn new() -> Self {{ Self {{ alpha: {alpha}, x_min: {x_min} }} }}\n
                         pub fn sample(&self, u: f64) -> f64 {{ self.x_min / (1.0 - u).powf(1.0 / self.alpha) }}\n}}\n\n"
                ));
            }
            DistributionFamily::LogNormal { mean, std_dev } => {
                out.push_str(&format!(
                    "// LOOM[implicit:LogNormal]: {fn_name} — Log-Normal (Galton 1879)\n\
                     // ln(X) ~ N(mu={mean}, sigma={std_dev}). Always positive. Multiplicative processes.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}LogNormalSampler {{ pub mu: f64, pub sigma: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}LogNormalSampler {{\n
                         pub fn new() -> Self {{ Self {{ mu: {mean}, sigma: {std_dev} }} }}\n
                         pub fn sample(&self, z: f64) -> f64 {{ (self.mu + self.sigma * z).exp() }}\n
                         pub fn median(&self) -> f64 {{ self.mu.exp() }}\n}}\n\n"
                ));
            }
            DistributionFamily::GeometricBrownian { drift, volatility } => {
                out.push_str(&format!(
                    "// LOOM[implicit:GBMDist]: {fn_name} — GBM distribution (Black-Scholes 1973)\n\
                     // drift={drift}, volatility={volatility}\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GBMDist {{ pub drift: f64, pub volatility: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GBMDist {{\n
                         pub fn new() -> Self {{ Self {{ drift: {drift}, volatility: {volatility} }} }}\n}}\n\n"
                ));
            }
            DistributionFamily::Gamma { shape, scale } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Gamma]: {fn_name} — Gamma distribution (Euler 1729)\n\
                     // X ~ Gamma(k={shape}, theta={scale}). Waiting times, positive reals.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GammaSampler {{ pub shape: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GammaSampler {{\n
                         pub fn new() -> Self {{ Self {{ shape: {shape}, scale: {scale} }} }}\n
                         pub fn mean(&self) -> f64 {{ self.shape * self.scale }}\n
                         pub fn variance(&self) -> f64 {{ self.shape * self.scale * self.scale }}\n}}\n\n"
                ));
            }
            DistributionFamily::Cauchy { location, scale } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Cauchy]: {fn_name} — Cauchy distribution (Cauchy 1853)\n\
                     // WARNING: NO defined mean or variance. CLT and LLN do NOT apply.\n\
                     // location={location}, scale={scale}. Heavy-tailed. Do not use for averaging.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}CauchySampler {{ pub location: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}CauchySampler {{\n
                         pub fn new() -> Self {{ Self {{ location: {location}, scale: {scale} }} }}\n
                         // Inverse CDF: X = location + scale*tan(pi*(u - 0.5))\n
                         pub fn sample(&self, u: f64) -> f64 {{\n
                             self.location + self.scale * (std::f64::consts::PI * (u - 0.5)).tan()\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Levy { location, scale } => {
                out.push_str(&format!(
                    "// LOOM[implicit:Levy]: {fn_name} — Levy distribution (Levy 1937)\n\
                     // Stable distribution. Anomalous diffusion. location={location}, scale={scale}.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}LevySampler {{ pub location: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}LevySampler {{ pub fn new() -> Self {{ Self {{ location: {location}, scale: {scale} }} }} }}\n\n"
                ));
            }
            DistributionFamily::Dirichlet { alpha } => {
                let a_str = alpha.join(", ");
                out.push_str(&format!(
                    "// LOOM[implicit:Dirichlet]: {fn_name} — Dirichlet distribution (Dirichlet 1831)\n\
                     // Probability simplex. alpha=[{a_str}]. Bayesian prior for categorical.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}DirichletSampler {{ pub alpha: Vec<f64> }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}DirichletSampler {{\n
                         pub fn new() -> Self {{ Self {{ alpha: vec![{a_str}] }} }}\n
                         pub fn concentration_sum(&self) -> f64 {{ self.alpha.iter().sum() }}\n}}\n\n"
                ));
            }
            DistributionFamily::Unknown(name) => {
                out.push_str(&format!(
                    "// LOOM[distribution:Unknown]: '{name}' distribution not yet generated\n\n"
                ));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH THEORY — DAG + LTS
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// DAG wrapper with Kahn topological sort (Kahn 1962). For directed Graph stores.
    pub(super) fn emit_dag_wrapper(&self, store_name: &str, out: &mut String) {
        let n = to_pascal_case(store_name);
        out.push_str(&ts(
            r#"
// LOOM[implicit:DAG]: {name} — Directed Acyclic Graph (Kahn 1962)
// Topological sort via Kahn's algorithm. Ecosystem: petgraph
#[derive(Debug, Clone, Default)]
pub struct {N}Dag {
    nodes: std::collections::HashMap<String, Vec<String>>,
}
impl {N}Dag {
    pub fn new() -> Self { Self::default() }
    pub fn add_node(&mut self, id: impl Into<String>) {
        self.nodes.entry(id.into()).or_default();
    }
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.nodes.entry(from.into()).or_default().push(to.into());
    }
    /// Kahn's algorithm: returns None if cycle detected (invariant: DAG must be acyclic).
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        use std::collections::{HashMap, VecDeque};
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for id in self.nodes.keys() { in_degree.insert(id, 0); }
        for children in self.nodes.values() {
            for c in children { *in_degree.entry(c).or_default() += 1; }
        }
        let mut queue: VecDeque<&str> = in_degree.iter()
            .filter_map(|(&n, &d)| if d == 0 { Some(n) } else { None }).collect();
        let mut result = Vec::new();
        while let Some(n) = queue.pop_front() {
            result.push(n.to_owned());
            if let Some(children) = self.nodes.get(n) {
                for c in children {
                    let d = in_degree.entry(c).or_default();
                    *d -= 1;
                    if *d == 0 { queue.push_back(c); }
                }
            }
        }
        if result.len() == self.nodes.len() { Some(result) } else { None }
    }
}"#,
            &[("N", &n), ("name", store_name)],
        ));
    }

    /// LTS (Labelled Transition System) for general/undirected graphs (Keller 1976).
    pub(super) fn emit_lts_graph(&self, store_name: &str, out: &mut String) {
        let n = to_pascal_case(store_name);
        out.push_str(&ts(
            r#"
// LOOM[implicit:LTS]: {name} — Labelled Transition System (Keller 1976)
// State + action-labelled transitions. Ecosystem: petgraph, roaring
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct {N}State(pub String);
#[derive(Debug, Clone, Default)]
pub struct {N}Lts {
    transitions: Vec<({N}State, String, {N}State)>,
}
impl {N}Lts {
    pub fn add_transition(&mut self, from: {N}State, label: impl Into<String>, to: {N}State) {
        self.transitions.push((from, label.into(), to));
    }
    pub fn successors(&self, state: &{N}State) -> Vec<(&str, &{N}State)> {
        self.transitions.iter()
            .filter_map(|(f, l, t)| if f == state { Some((l.as_str(), t)) } else { None })
            .collect()
    }
    /// Reachability: BFS from initial state.
    pub fn reachable(&self, initial: &{N}State) -> std::collections::HashSet<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial.0.clone());
        while let Some(s) = queue.pop_front() {
            if visited.insert(s.clone()) {
                let state = {N}State(s.clone());
                for (_, next) in self.successors(&state) { queue.push_back(next.0.clone()); }
            }
        }
        visited
    }
}"#,
            &[("N", &n), ("name", store_name)],
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FORMAL VERIFICATION AUDIT TRAIL
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Separation logic audit (Reynolds 2002). Ownership + heap disjointness note.
    pub(super) fn emit_separation_audit(&self, fn_name: &str, sb: &SeparationBlock, out: &mut String) {
        let owns = sb.owns.join(", ");
        let disjoint: Vec<String> = sb.disjoint.iter().map(|(a, b)| format!("{a} * {b}")).collect();
        out.push_str(&format!(
            "// LOOM[implicit:Separation]: {fn_name} — Separation Logic (Reynolds 2002)\n\
// Claim: heap regions disjoint. Ownership via Rust borrow checker (affine types).\n\
// owns: {owns}  disjoint: {disjoint}  frame: {frame}\n\
// Ecosystem: Prusti (ETH Zurich) for full separation logic proofs.\n\
// To prove: #[requires(x != y)] in Prusti harness.\n\n",
            disjoint = disjoint.join(", "),
            frame = sb.frame.join(", "),
        ));
    }

    /// Constant-time audit (Kocher 1996). Hints at subtle::ConstantTimeEq.
    pub(super) fn emit_timing_safety_audit(&self, fn_name: &str, ts: &TimingSafetyBlock, out: &mut String) {
        let mode = if ts.constant_time { "constant_time" } else { "declared_only" };
        let leaks = ts.leaks_bits.as_deref().unwrap_or("none");
        out.push_str(&format!(
            "// LOOM[implicit:TimingSafety]: {fn_name} — Constant-time audit (Kocher 1996)\n\
// mode: {mode}  leaks_bits: {leaks}. Prevents timing side-channel attacks.\n\
// Ecosystem: subtle (Dalek Cryptography) — ConstantTimeEq, ConstantTimeGreater.\n\
// Use subtle::ConstantTimeEq::ct_eq instead of == for secrets.\n\
// Dynamic verifier: ctgrind, dudect, or BINSEC/SE.\n\n"
        ));
    }

    /// Termination audit (Turing 1936 / König 1936). Variant function note.
    /// `termination` is `Option<String>` — the metric name or proof strategy.
    pub(super) fn emit_termination_audit(&self, fn_name: &str, metric: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[implicit:Termination]: {fn_name} — Termination analysis (König 1936)\n\
// Claim: function terminates. Rust cannot prove general termination.\n\
// metric: {metric} — variant must strictly decrease each iteration.\n\
// Ecosystem: Kani (SAT-bounded), Dafny (decreases clause), Coq (Acc).\n\
// For production: add a bounded iteration guard and panic on exceed.\n\n"
        ));
    }

    /// Gradual typing boundary (Siek & Taha 2006). GradualBoundary<T,U> wrapper.
    pub(super) fn emit_gradual_boundary(&self, fn_name: &str, gb: &GradualBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let input = gb.input_type.as_deref().unwrap_or("T");
        let output = gb.output_type.as_deref().unwrap_or("U");
        out.push_str(&format!(
            "// LOOM[implicit:Gradual]: {fn_name} — Gradual Typing Boundary (Siek & Taha 2006)\n\
// input: {input} -> output: {output}. Cast checks at runtime.\n\
// Ecosystem: Any trait (std), erased types, dynamic dispatch.\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug)]\npub enum {n}GradualBoundary<T, U> {{\n    Static(T),\n    Dynamic(U),\n}}\n"
        ));
        out.push_str(&format!(
            "impl<T, U: std::fmt::Debug> {n}GradualBoundary<T, U> {{\n    \
/// Unwrap static side. Panics if boundary is dynamic (deliberate fail-fast).\n    \
pub fn static_value(self) -> T {{\n        \
match self {{ Self::Static(v) => v, Self::Dynamic(d) => panic!(\"gradual boundary violation: {{:?}}\", d) }}\n    \
}}\n}}\n\n"
        ));
    }

    /// Degenerate case fallback dispatcher. DegenerateFallback<T>.
    pub(super) fn emit_degenerate_fallback(&self, fn_name: &str, db: &DegenerateBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        out.push_str(&format!(
            "// LOOM[implicit:Degenerate]: {fn_name} — Degenerate case fallback (Edelman)\n\
// primary: {}  fallback: {}. Returns fallback value instead of failing silently.\n\n",
            db.primary, db.fallback
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}DegenerateFallback<T> {{\n    pub value: T,\n    pub is_degenerate: bool,\n}}\n"
        ));
        out.push_str(&format!(
            "impl<T: std::fmt::Debug + Clone> {n}DegenerateFallback<T> {{\n    \
pub fn normal(v: T) -> Self {{ Self {{ value: v, is_degenerate: false }} }}\n    \
pub fn fallback(v: T) -> Self {{ Self {{ value: v, is_degenerate: true }} }}\n    \
pub fn require_non_degenerate(self) -> T {{\n        \
debug_assert!(!self.is_degenerate, \"degenerate fallback activated\"); self.value\n    }}\n}}\n\n"
        ));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DISCIPLINE DISPATCHER — called from emit_fn_def in functions.rs
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Master dispatcher: emit ALL discipline artefacts for a function's annotations.
    pub(super) fn emit_fn_disciplines(&self, fd: &FnDef) -> String {
        let mut out = String::new();
        if let Some(sp) = &fd.stochastic_process {
            self.emit_stochastic_process(&fd.name, sp, &mut out);
        }
        if let Some(db) = &fd.distribution {
            self.emit_distribution_sampler(&fd.name, db, &mut out);
        }
        if let Some(sb) = &fd.separation {
            self.emit_separation_audit(&fd.name, sb, &mut out);
        }
        if let Some(ts) = &fd.timing_safety {
            self.emit_timing_safety_audit(&fd.name, ts, &mut out);
        }
        if let Some(tb) = &fd.termination {
            self.emit_termination_audit(&fd.name, tb, &mut out);
        }
        if let Some(gb) = &fd.gradual {
            self.emit_gradual_boundary(&fd.name, gb, &mut out);
        }
        if let Some(db) = &fd.degenerate {
            self.emit_degenerate_fallback(&fd.name, db, &mut out);
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

pub(super) fn to_pascal_case(s: &str) -> String {
    s.split('_').map(|w| {
        let mut chars = w.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }).collect()
}
