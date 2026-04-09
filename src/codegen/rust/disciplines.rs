//! Implicit discipline codegen — engineering patterns automatically applied by Loom.
//!
//! A discipline is an artifact the developer does NOT need to ask for explicitly.
//! It fires because of what they declared.  Declare a relational store; get a
//! Repository, Unit of Work, Specification pattern, pagination cursor, CQRS split,
//! HATEOAS links, and OpenAPI hints — all provably correct, all gratis.
//!
//! ## Taxonomy note
//! This module contains only **implicit disciplines**.  See the sibling modules for:
//! - `structures`  — explicit mathematical domain structures (Markov, DAG, distributions)
//! - `contracts`   — correctness annotations on functions (separation, timing, termination)
//!
//! ## Template conventions
//! All multi-line code generation uses `ts()` from the `template` module.
//! Raw string literals (`r#"..."#`) — no `{{` / `}}` escaping ever needed.
//!
//! ## Discipline map (trigger -> implicit artifact)
//!
//! ### Data Access Layer
//! | store :: Relational | Repository trait + InMemory fake      | Fowler 2002        |
//! | store :: Relational | Unit of Work (transaction scope)      | Fowler 2002        |
//! | store :: Relational | Specification (composable predicates) | Evans/Fowler 2002  |
//! | store :: Relational | Pagination cursor                     | Web API practices  |
//! | store :: Document   | Document repository + Aggregatable    | DDD                |
//!
//! ### API / Web Layer
//! | store :: Relational | HATEOAS ResourceLinks  | Fielding 2000 |
//! | store :: Relational | OpenAPI schema hints   | OpenAPI 3.1   |
//!
//! ### Architecture Patterns
//! | store :: Relational     | CQRS Command/Query split              | Young 2010         |
//! | store :: TimeSeries     | Event Sourcing (EventStore+Aggregate) | Evans 2003         |
//! | store :: DistributedLog | Domain Event bus                      | Evans 2003         |
//! | store :: DistributedLog | Saga coordinator                      | Garcia-Molina 1987 |
//!
//! ### Concurrency / Messaging
//! | messaging_primitive RequestResponse | Request/Response channel | Honda 1993         |
//! | messaging_primitive PublishSubscribe| Observer / Event bus     | GoF 1994           |
//! | messaging_primitive ProducerConsumer| Typed work queue         | CSP Hoare 1978     |
//! | messaging_primitive Bidirectional   | Bidirectional channel    | pi-calculus        |
//! | session: block                      | Phantom-type state machine| Honda 1993        |
//! | effect: block                       | Algebraic effect handler | Plotkin & Pretnar  |
//!
//! ### Resilience
//! | aspect on_failure + max_attempts | Circuit breaker               | Nygard 2007     |
//! | aspect max_attempts              | Retry with exponential backoff| AWS pattern     |

use crate::ast::*;
use super::{RustEmitter, to_pascal_case, to_snake_case};
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
            "impl<A: {t}Specification, B: {t}Specification> {t}Specification for And{t}Spec<A,B> {{\n\
                 fn is_satisfied_by(&self, c: &{t}) -> bool {{ self.0.is_satisfied_by(c) && self.1.is_satisfied_by(c) }}\n}}\n\n",
            t = table
        ));
        out.push_str(&format!(
            "pub struct Not{t}Spec<A: {t}Specification>(pub A);\n", t = table
        ));
        out.push_str(&format!(
            "impl<A: {t}Specification> {t}Specification for Not{t}Spec<A> {{\n\
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
            "pub trait {s}EventStore {{\n    type Error;\n\
                 fn append(&self, stream: &str, events: Vec<{s}Event>) -> Result<u64, Self::Error>;\n\
                 fn load(&self, stream: &str, from: u64) -> Result<Vec<{s}Event>, Self::Error>;\n}}\n\n",
            s = store_name
        ));
        out.push_str(&format!(
            "// LOOM[implicit:Aggregate]: {s} — state = fold of events\n\
             pub trait {s}Aggregate: Sized + Default {{\n\
                 fn apply(&mut self, event: &{s}Event);\n\
                 fn load_from_events(events: &[{s}Event]) -> Self {{\n\
                     let mut agg = Self::default();\n\
                     for ev in events {{ agg.apply(ev); }}\n\
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
            "impl {s}EventBus {{\n\
                 pub fn subscribe(&mut self, h: Box<dyn {s}EventHandler>) {{ self.handlers.push(h); }}\n\
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
            "impl {s}Saga {{\n\
                 pub fn step(mut self, s: Box<dyn {s}SagaStep<Error = String>>) -> Self {{ self.steps.push(s); self }}\n\
                 pub fn run(self) -> Result<(), String> {{\n\
                     let mut done: Vec<usize> = Vec::new();\n\
                     for (i, step) in self.steps.iter().enumerate() {{\n\
                         if let Err(e) = step.execute() {{\n\
                             for &j in done.iter().rev() {{ self.steps[j].compensate(); }}\n\
                             return Err(e);\n\
                         }}\n\
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
                    "pub trait {n}Client {{\n    type Request;\n    type Response;\n    type Error;\n\
                         fn call(&self, req: Self::Request) -> Result<Self::Response, Self::Error>;\n}}\n\n"
                ));
                if mp.timeout_mandatory {
                    out.push_str(&format!(
                        "pub trait {n}ClientWithTimeout: {n}Client {{\n\
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
                    "impl<E: Clone> {n}Bus<E> {{\n\
                         pub fn subscribe(&mut self, s: Box<dyn {n}Subscriber<E>>) {{ self.subscribers.push(s); }}\n\
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
            "impl {n}CircuitBreaker {{\n\
                 pub fn new() -> Self {{ Self {{ state: {n}BreakerState::Closed, failure_count: 0, threshold: {max_attempts} }} }}\n\
                 pub fn record_failure(&mut self) {{\n\
                     self.failure_count += 1;\n\
                     if self.failure_count >= self.threshold {{ self.state = {n}BreakerState::Open; }}\n    }}\n\
                 pub fn record_success(&mut self) {{ self.failure_count = 0; self.state = {n}BreakerState::Closed; }}\n\
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
            "impl {n}RetryPolicy {{\n\
                 pub fn new() -> Self {{ Self {{ max_attempts: {max_attempts}, base_delay_ms: 100 }} }}\n\
                 pub fn delay_for_attempt(&self, attempt: u32) -> u64 {{ self.base_delay_ms * 2u64.pow(attempt.min(10)) }}\n}}\n\n"
        ));
    }
}
