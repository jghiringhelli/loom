//! Rust source emitter — translates a Loom [`Module`] AST into valid Rust code.
//!
//! # Mapping summary
//!
//! | Loom construct | Emitted Rust |
//! |---|---|
//! | `module M … end` | `pub mod m { … }` + `pub trait M` for the provides interface |
//! | `type Point = x: Float, y: Float end` | `#[derive(…)] pub struct Point { pub x: f64, pub y: f64 }` |
//! | `enum E = \| A \| B of T end` | `#[derive(…)] pub enum E { A, B(T) }` |
//! | `type Email = String where pred` | newtype `pub struct Email(String)` + `TryFrom` |
//! | `fn f :: A -> Effect<[E], B>` | `pub fn f(a: A) -> Result<B, LoomError>` |
//! | `fn f :: A -> B` (pure) | `pub fn f(a: A) -> B` |
//! | `let x = e` | `let x = e;` |
//! | `match x \| Arm -> body end` | `match x { Arm => body }` |
//! | `require: cond` | `debug_assert!(cond, "precondition violated");` |
//! | `ensure: cond` | `debug_assert!(cond_using_loom_result, "ensure: ...");` |
//! | `a \|> f` | intermediate let binding |

use crate::ast::*;

mod beings;
mod contracts;
mod disciplines;
mod exprs;
mod functions;
mod stores;
mod structures;
pub(crate) mod template;
mod telos;
mod types;

// ── M151: BinaryPersist trait ─────────────────────────────────────────────────
//
// Emitted once inside any module that declares at least one store.
// Gives every `{Name}Snapshot` struct save/load to disk via bincode serialization.
// Deps: serde = { version = "1", features = ["derive"] }, bincode = "1"
const BINARY_PERSIST_TRAIT: &str = r#"
    // LOOM[persist:binary]: M151 — binary snapshot persistence trait
    // Deps: serde = { version = "1", features = ["derive"] }, bincode = "1"
    pub trait BinaryPersist: serde::Serialize + for<'de> serde::Deserialize<'de> + Sized {
        /// Serialize this snapshot to a binary file using bincode.
        fn save_snapshot(&self, path: &std::path::Path) -> std::io::Result<()> {
            let bytes = bincode::serialize(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            std::fs::write(path, bytes)
        }
        /// Deserialize a snapshot from a binary file.
        fn load_snapshot(path: &std::path::Path) -> std::io::Result<Self> {
            let bytes = std::fs::read(path)?;
            bincode::deserialize(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    }
"#;

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Stateless Rust source emitter.
///
/// # Examples
///
/// ```rust,ignore
/// let rust_src = RustEmitter::new().emit(&module);
/// ```
pub struct RustEmitter;

impl RustEmitter {
    /// Create a new `RustEmitter`.
    pub fn new() -> Self {
        RustEmitter
    }

    /// Emit a complete Rust source file from a [`Module`].
    pub fn emit(&self, module: &Module) -> String {
        let mut out = String::with_capacity(4096);

        // File-level attributes and imports.
        out.push_str("#![allow(unused)]\n");
        out.push_str("use std::convert::TryFrom;\n");
        out.push_str(&emit_audit_header(module));

        // Module wrapper with doc comments.
        let mod_name = to_snake_case(&module.name);
        self.emit_module_doc_comments(module, &mut out);
        out.push_str(&format!("pub mod {} {{\n", mod_name));
        out.push_str("    use super::*;\n");
        self.emit_flow_labels(module, &mut out);
        for imp in &module.imports {
            out.push_str(&format!("    use super::{}::*;\n", to_snake_case(imp)));
        }

        // Build body first so we can detect which stdlib imports are needed.
        let mut body = String::new();
        self.emit_module_declarations(module, &mut body);
        self.emit_items_body(module, &mut body);

        // Inject stdlib collection imports when needed.
        if body.contains("HashMap") {
            out.push_str("    use std::collections::HashMap;\n");
        }
        if body.contains("HashSet") {
            out.push_str("    use std::collections::HashSet;\n");
        }
        // Inject BinaryPersist trait once if any stores are present.
        let has_stores = module.items.iter().any(|i| matches!(i, Item::Store(_)));
        if has_stores {
            out.push_str(BINARY_PERSIST_TRAIT);
        }
        for item in &module.items {
            if let Item::Enum(ed) = item {
                out.push_str(&format!("    use self::{}::*;\n", ed.name));
            }
        }

        out.push_str(&body);

        if !module.test_defs.is_empty() {
            let tests_src = self.emit_test_mod(&module.test_defs);
            out.push('\n');
            out.push_str(&indent_block(&tests_src));
        }

        out.push_str("}\n");
        out
    }

    /// Emit module-level `describe:` and `@annotation` doc comments.
    fn emit_module_doc_comments(&self, module: &Module, out: &mut String) {
        if let Some(desc) = &module.describe {
            for line in desc.lines() {
                out.push_str(&format!("/// {}\n", line));
            }
        }
        for ann in &module.annotations {
            out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value));
        }
    }

    /// Emit information-flow label summary comment block.
    fn emit_flow_labels(&self, module: &Module, out: &mut String) {
        if !module.flow_labels.is_empty() {
            out.push_str("    // information-flow labels:\n");
            for fl in &module.flow_labels {
                out.push_str(&format!("    //   {}: {}\n", fl.label, fl.types.join(", ")));
            }
        }
    }

    /// Emit all module-level structural declarations into `body`.
    ///
    /// Covers: unit types, interface traits, lifecycle state markers, temporal
    /// properties, aspect blocks, implements, provides, DI context.
    fn emit_module_declarations(&self, module: &Module, body: &mut String) {
        let unit_types_src = self.emit_unit_types(module);
        if !unit_types_src.is_empty() {
            body.push('\n');
            body.push_str(&indent_block(&unit_types_src));
        }

        for iface in &module.interface_defs {
            body.push('\n');
            body.push_str(&self.emit_interface_trait(iface));
        }

        for lc in &module.lifecycle_defs {
            body.push('\n');
            body.push_str(&format!("// Lifecycle states for {}\n", lc.type_name));
            for state in &lc.states {
                body.push_str(&format!("pub struct {};\n", state));
            }
        }

        for temporal in &module.temporal_defs {
            body.push('\n');
            self.emit_temporal_comments(temporal, body);
        }

        for aspect in &module.aspect_defs {
            body.push('\n');
            self.emit_aspect_comment(aspect, body);
        }

        for iface_name in &module.implements {
            body.push('\n');
            if let Some(iface) = module.interface_defs.iter().find(|i| &i.name == iface_name) {
                body.push_str(&self.emit_implements_block(
                    &module.name,
                    iface_name,
                    iface,
                    &module.items,
                ));
            } else {
                body.push_str(&format!(
                    "// impl {} for {}Impl {{ /* external interface */ }}\n",
                    iface_name, module.name
                ));
            }
        }

        if let Some(provides) = &module.provides {
            body.push('\n');
            body.push_str(&self.emit_provides_trait(&module.name, provides));
        }

        if let Some(requires) = &module.requires {
            body.push('\n');
            body.push_str(&self.emit_context_struct(&module.name, requires));
            body.push('\n');
        }
    }

    /// Emit temporal property block as doc comments.
    fn emit_temporal_comments(&self, temporal: &TemporalDef, body: &mut String) {
        body.push_str(&format!("// temporal invariants: {}\n", temporal.name));
        for prop in &temporal.properties {
            match prop {
                TemporalProperty::Always { .. } => {
                    body.push_str("//   always: [invariant verified at compile time]\n");
                }
                TemporalProperty::Eventually {
                    type_name,
                    target_state,
                    ..
                } => {
                    body.push_str(&format!(
                        "//   eventually: {} reaches {}\n",
                        type_name, target_state
                    ));
                }
                TemporalProperty::Never {
                    from_state,
                    to_state,
                    ..
                } => {
                    body.push_str(&format!(
                        "//   never: {} transitions to {} [enforced]\n",
                        from_state, to_state
                    ));
                }
                TemporalProperty::Precedes { first, second, .. } => {
                    body.push_str(&format!(
                        "//   precedes: {} before {} [enforced]\n",
                        first, second
                    ));
                }
            }
        }
    }

    /// Emit aspect block as doc comment.
    fn emit_aspect_comment(&self, aspect: &AspectDef, body: &mut String) {
        body.push_str(&format!(
            "// aspect: {} (order: {})\n",
            aspect.name,
            aspect.order.map_or("unset".to_string(), |o| o.to_string())
        ));
        if let Some(pointcut) = &aspect.pointcut {
            body.push_str(&format!(
                "//   pointcut: {}\n",
                Self::fmt_pointcut(pointcut)
            ));
        }
        for b in &aspect.before {
            body.push_str(&format!("//   before: {}\n", b));
        }
        for a in &aspect.after {
            body.push_str(&format!("//   after: {}\n", a));
        }
        for a in &aspect.after_throwing {
            body.push_str(&format!("//   after_throwing: {}\n", a));
        }
        for a in &aspect.around {
            body.push_str(&format!("//   around: {}\n", a));
        }
        if let Some(f) = &aspect.on_failure {
            body.push_str(&format!("//   on_failure: {}\n", f));
        }
    }

    /// Emit all items, invariants, beings, and ecosystems into `body`.
    fn emit_items_body(&self, module: &Module, body: &mut String) {
        for item in &module.items {
            let item_src = self.emit_item(item, module);
            body.push('\n');
            body.push_str(&indent_block(&item_src));
        }

        if !module.invariants.is_empty() {
            body.push('\n');
            body.push_str(&indent_block(
                &self.emit_check_invariants(&module.invariants),
            ));
        }

        for being in &module.being_defs {
            body.push('\n');
            body.push_str(&indent_block(&self.emit_being(being)));
        }

        for eco in &module.ecosystem_defs {
            body.push('\n');
            body.push_str(&indent_block(&self.emit_ecosystem(eco)));
        }
    }

    /// Dispatch a single module item to its emitter.
    fn emit_item(&self, item: &Item, module: &Module) -> String {
        match item {
            Item::Type(td) => self.emit_type_def(td),
            Item::Enum(ed) => self.emit_enum_def(ed),
            Item::Fn(fd) => self.emit_fn_def_with_context(fd, &module.name, module.requires.is_some()),
            Item::RefinedType(rt) => self.emit_refined_type(rt),
            Item::Proposition(prop) => self.emit_proposition(prop),
            Item::Functor(f) => self.emit_functor(f),
            Item::Monad(m) => self.emit_monad(m),
            Item::Certificate(cert) => self.emit_certificate(cert),
            Item::AnnotationDecl(decl) => emit_annotation_decl(decl),
            Item::CorrectnessReport(report) => emit_correctness_report(report),
            Item::Pathway(pw) => emit_pathway(pw),
            Item::SymbioticImport { module, kind, .. } => {
                format!("// symbiotic: kind: {}, module: {}\n", kind, module)
            }
            Item::Adopt(decl) => format!(
                "// adopt: {} from {}\n// LOOM[adopt:M75]: implement {} for this module via trait bounds\n",
                decl.interface, decl.from_module, decl.interface
            ),
            Item::NicheConstruction(nc) => emit_niche_construction(nc),
            Item::Sense(sd) => emit_sense(sd),
            Item::Store(sd) => self.codegen_store(sd),
            Item::TypeAlias(name, ty, _) => {
                format!("pub type {} = {};\n", name, self.emit_type_expr(ty))
            }
            Item::Session(sd) => {
                let mut buf = String::new();
                self.emit_session_state_machine(sd, &mut buf);
                buf
            }
            Item::Effect(ed) => {
                let mut buf = String::new();
                self.emit_effect_handler(ed, &mut buf);
                buf
            }
            Item::UseCase(uc) => self.emit_usecase(uc),
            Item::Property(pb) => self.emit_property_test(pb),
            Item::BoundaryBlock(bb) => format!(
                "// boundary: exports=[{}] private=[{}] sealed=[{}]\n",
                bb.exports.join(", "), bb.private.join(", "), bb.sealed.join(", ")
            ),
            Item::TelosFunction(tf) => telos::emit_telos_function(tf),
            Item::Entity(ed) => emit_entity(ed),
            Item::IntentCoordinator(ic) => format!(
                "// intent_coordinator: {} — governance gate (GovernanceClass: {:?})\n\
                 // LOOM[intent_coordinator]: Part IX — intent vivo with human governance\n",
                ic.name, ic.governance_class
            ),
            Item::MessagingPrimitive(mp) => {
                let mut buf = String::new();
                self.emit_messaging_channel(mp, &mut buf);
                buf
            }
            Item::Discipline(dd) => {
                let mut buf = String::new();
                emit_discipline(dd, &mut buf);
                buf
            }
        }
    }
}

// ── Helpers used across submodules ────────────────────────────────────────────

// ── M141-M145: discipline codegen ────────────────────────────────────────────

/// Emit Rust constructs for an explicit `discipline` declaration.
///
/// Reuses the pre-existing emit methods in `disciplines.rs` (CQRS, EventSourcing,
/// Saga, UnitOfWork, CircuitBreaker) and wires them to the AST `DisciplineDecl`.
fn emit_discipline(dd: &DisciplineDecl, out: &mut String) {
    let emitter = RustEmitter::new();
    let target = &dd.target;

    out.push_str(&format!(
        "// ── discipline {:?} for {} ──────────────────────────────────────────\n",
        dd.kind, target
    ));
    out.push_str("// LOOM[discipline]: compiler-verified architectural contract\n\n");

    match &dd.kind {
        DisciplineKind::Cqrs => {
            emitter.emit_cqrs_for_store(target, out);
        }
        DisciplineKind::EventSourcing => {
            // Extract optional `events: [...]` list
            let events = list_param(&dd.params, "events");
            emitter.emit_event_sourcing(target, &events, out);
            emitter.emit_domain_event_bus(target, out);
        }
        DisciplineKind::DependencyInjection => {
            // Extract `binds: [IPortA, IPortB]`
            let binds = list_param(&dd.params, "binds");
            emit_di_container(target, &binds, out);
        }
        DisciplineKind::CircuitBreaker => {
            let max_attempts = num_param(&dd.params, "max_attempts").unwrap_or(3) as u32;
            emitter.emit_circuit_breaker(target, max_attempts, out);
            let retry_attempts = num_param(&dd.params, "retry_attempts").unwrap_or(3) as u32;
            emitter.emit_retry_policy(target, retry_attempts, out);
        }
        DisciplineKind::Saga => {
            // Extract optional `steps: [...]` list
            let steps = list_param(&dd.params, "steps");
            emit_saga_with_steps(target, &steps, out);
        }
        DisciplineKind::UnitOfWork => {
            let tables = list_param(&dd.params, "tables");
            emitter.emit_unit_of_work(target, &tables, out);
        }
    }
}

/// M141: Dependency Injection container — one `{Target}Container` struct that
/// owns all injected port implementations.
fn emit_di_container(target: &str, binds: &[String], out: &mut String) {
    out.push_str(&format!(
        "// Martin 2003 DI — port-based injection; Fowler 2004 IoC container\n"
    ));

    // Port method stubs on the container
    let port_fields: String = binds
        .iter()
        .map(|port| {
            format!(
                "    pub {field}: Box<dyn {port}>,\n",
                field = to_snake(port),
                port = port
            )
        })
        .collect();

    let field_block = if port_fields.is_empty() {
        "    // add your port fields here\n".to_string()
    } else {
        port_fields
    };

    out.push_str(&format!(
        "/// Dependency injection container for `{target}`.\n\
         /// Owns all port implementations; wire at the composition root.\n\
         pub struct {target}Container {{\n\
         {field_block}\
         }}\n\n",
        target = target,
        field_block = field_block,
    ));

    // Trait for each bound port (stub — user fills in the `trait` body)
    for port in binds {
        out.push_str(&format!(
            "// LOOM[di:port]: implement this trait in your adapter layer\n\
             pub trait {port}: Send + Sync {{\n    \
             // TODO: declare port methods\n\
             }}\n\n",
            port = port
        ));
    }
}

/// M145: Saga with named steps — extends the generic saga coordinator with step types.
fn emit_saga_with_steps(target: &str, steps: &[String], out: &mut String) {
    // Emit the generic saga coordinator
    let emitter = RustEmitter::new();
    emitter.emit_saga_coordinator(target, out);

    if steps.is_empty() {
        return;
    }

    out.push_str(&format!(
        "// ── {target} saga steps ────────────────────────────────────────\n"
    ));
    for step in steps {
        out.push_str(&format!(
            "pub struct {step}Step;\n\
             impl {target}SagaStep for {step}Step {{\n    \
             type Error = String;\n    \
             fn execute(&self) -> Result<(), Self::Error> {{ todo!(\"implement {step}\") }}\n    \
             fn compensate(&self) {{ todo!(\"compensate {step}\") }}\n\
             }}\n\n",
            target = target,
            step = step
        ));
    }
}

/// Extract a `List` param by key name; returns empty vec if absent.
fn list_param(params: &[(String, DisciplineParam)], key: &str) -> Vec<String> {
    params.iter().find_map(|(k, v)| {
        if k == key {
            if let DisciplineParam::List(items) = v { Some(items.clone()) } else { None }
        } else {
            None
        }
    }).unwrap_or_default()
}

/// Extract a `Number` param by key name; returns `None` if absent.
fn num_param(params: &[(String, DisciplineParam)], key: &str) -> Option<i64> {
    params.iter().find_map(|(k, v)| {
        if k == key {
            if let DisciplineParam::Number(n) = v { Some(*n) } else { None }
        } else {
            None
        }
    })
}

/// Convert PascalCase to snake_case for field names.
fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    out
}

// ── M123-M125: entity codegen ─────────────────────────────────────────────────

/// Known structural patterns and their mathematical descriptions.
/// Keyed by `alias_of` name (lowercase). Used to emit richer doc comments.
fn known_alias_description(alias: &str) -> Option<&'static str> {
    match alias.to_lowercase().as_str() {
        "markovchain" | "markov_chain" => Some(
            "Discrete-time Markov chain: transition probabilities form a row-stochastic matrix. \
             Each row must sum to 1.0."
        ),
        "dag" => Some(
            "Directed Acyclic Graph: topological ordering always exists. \
             Enables dependency resolution and causal reasoning."
        ),
        "tree" => Some(
            "Rooted tree: every node has exactly one parent except the root. \
             Encodes hierarchical containment."
        ),
        "fsm" | "finitestate" | "finite_state_machine" => Some(
            "Finite State Machine: finite set of states, deterministic transition function. \
             Accepts exactly the strings in a regular language."
        ),
        "neuralnet" | "neural_net" | "neuralnetwork" => Some(
            "Neural network: directed weighted graph where weights are updated via gradient descent. \
             Forward pass: f(x) = activation(W·x + b)."
        ),
        "knowledgegraph" | "knowledge_graph" => Some(
            "Knowledge graph: undirected semantic network of concepts and relations. \
             Supports inference via graph traversal."
        ),
        "causalgraph" | "causal_graph" => Some(
            "Causal graph: DAG encoding cause→effect relationships. \
             Supports do-calculus (Pearl 2000) and counterfactual reasoning."
        ),
        _ => None,
    }
}

/// M123-M125: Emit a rich Rust type alias + doc comments for an `entity<N, E>` declaration.
///
/// Chooses the appropriate petgraph backing type based on structural annotations,
/// emits known-alias mathematical descriptions, and adds guidance comments for
/// semantic annotations (`@stochastic`, `@learnable`, `@telos_guided`).
fn emit_entity(ed: &crate::ast::EntityDef) -> String {
    let node = ed.node_type.as_deref().unwrap_or("()");
    let edge = ed.edge_type.as_deref().unwrap_or("()");
    let ann = &ed.annotations;
    let has = |s: &str| ann.iter().any(|a| a == s);

    // Choose petgraph backing type
    let rust_type = if has("directed") || has("acyclic") || has("hierarchical") || has("causal") || has("temporal") {
        format!("petgraph::graph::DiGraph<{}, {}>", node, edge)
    } else if has("undirected") || has("semantic") {
        format!("petgraph::graph::UnGraph<{}, {}>", node, edge)
    } else {
        format!("petgraph::graph::Graph<{}, {}>", node, edge)
    };

    let ann_str = if ann.is_empty() {
        String::new()
    } else {
        format!(" @{}", ann.join(" @"))
    };

    let mut buf = String::new();

    // Doc comment
    buf.push_str(&format!("/// `entity<{}, {}>{}`\n", node, edge, ann_str));

    // Known alias description
    if let Some(alias) = &ed.alias_of {
        if let Some(desc) = known_alias_description(alias) {
            buf.push_str(&format!("///\n/// **{alias}**: {desc}\n"));
        } else {
            buf.push_str(&format!("/// Instance of: {alias}\n"));
        }
    }

    // Describe string
    if let Some(desc) = &ed.describe {
        buf.push_str(&format!("/// {desc}\n"));
    }

    // Semantic guidance comments
    if has("stochastic") {
        buf.push_str(
            "// LOOM[stochastic]: edge weights must be probabilities in [0.0, 1.0];\n\
             //   per-node outgoing weights must sum to 1.0 (row-stochastic).\n"
        );
    }
    if has("learnable") {
        buf.push_str(
            "// LOOM[learnable]: implement a weight-update method (e.g. gradient descent);\n\
             //   edge weights are free parameters optimised during training.\n"
        );
    }
    if has("telos_guided") {
        buf.push_str(
            "// LOOM[telos_guided]: edge activation is modulated by a telos score;\n\
             //   high-telos paths are reinforced, low-telos paths are pruned over time.\n"
        );
    }
    if has("causal") {
        buf.push_str(
            "// LOOM[causal]: supports Pearl do-calculus — \
             use petgraph topological_sort for causal ordering.\n"
        );
    }
    if has("temporal") {
        buf.push_str(
            "// LOOM[temporal]: edges encode temporal precedence; \
             topological order yields event sequence.\n"
        );
    }

    buf.push_str(&format!(
        "// LOOM[entity]: {}<{}, {}>{}\n",
        ed.name, node, edge, ann_str
    ));

    let alias_comment = match &ed.alias_of {
        Some(a) => format!(" // instance of: {a}"),
        None => String::new(),
    };

    buf.push_str(&format!(
        "pub type {} = {};{}\n",
        ed.name, rust_type, alias_comment
    ));

    buf
}



/// V7: Emit a dynamic audit header that honestly records what the module declares
/// and what verification tier backs each claim.
///
/// Format: `// == LOOM AUDIT: ModuleName ==` block with one line per claim category.
/// Distinguishes "proved" (Kani, proptest, typestate) from "declared only" (Prusti,
/// ctgrind, Dafny) — the latter require external tools to discharge.
fn emit_audit_header(module: &Module) -> String {
    let mut fn_count = 0u32;
    let mut contract_fns = 0u32;
    let mut props = 0u32;
    let mut sessions = 0u32;
    let mut effects = 0u32;
    let mut stores = 0u32;
    let mut stochastic = 0u32;
    let mut distributions = 0u32;
    let mut separation = 0u32;
    let mut timing = 0u32;
    let mut termination = 0u32;

    for item in &module.items {
        match item {
            Item::Fn(fd) => {
                fn_count += 1;
                if !fd.requires.is_empty() || !fd.ensures.is_empty() {
                    contract_fns += 1;
                }
                if fd.stochastic_process.is_some() {
                    stochastic += 1;
                }
                if fd.distribution.is_some() {
                    distributions += 1;
                }
                if fd.separation.is_some() {
                    separation += 1;
                }
                if fd.timing_safety.is_some() {
                    timing += 1;
                }
                if fd.termination.is_some() {
                    termination += 1;
                }
            }
            Item::Property(_) => props += 1,
            Item::Session(_) => sessions += 1,
            Item::Effect(_) => effects += 1,
            Item::Store(_) => stores += 1,
            _ => {}
        }
    }

    let name = &module.name;
    let mut h = String::new();
    h.push_str(&format!("// == LOOM AUDIT: {name} ==\n"));
    h.push_str(&format!("// Functions  : {fn_count}\n"));

    if contract_fns > 0 {
        h.push_str(&format!(
            "// Contracts  : {contract_fns} fn(s) → debug_assert!(runtime) + #[cfg(kani)] proof harness\n"
        ));
    }
    if props > 0 {
        h.push_str(&format!(
            "// Properties : {props} block(s) → edge-case #[test] + proptest (--cfg loom_proptest)\n"
        ));
    }
    if sessions > 0 {
        h.push_str(&format!(
            "// Sessions   : {sessions} → typestate compile-time protocol enforcement (Honda 1993)\n"
        ));
    }
    if effects > 0 {
        h.push_str(&format!(
            "// Effects    : {effects} → algebraic effect dispatch (Plotkin & Pretnar 2009)\n"
        ));
    }
    if stores > 0 {
        h.push_str(&format!(
            "// Stores     : {stores} → typed persistence + CRUD + HATEOAS\n"
        ));
    }
    if stochastic > 0 {
        h.push_str(&format!(
            "// Stochastic : {stochastic} process(es) → Wiener/GBM/OU/Poisson/Markov struct\n"
        ));
    }
    if distributions > 0 {
        h.push_str(&format!(
            "// Distr      : {distributions} → rejection-sampling; verify with proptest\n"
        ));
    }

    // Declared-only claims (external verifiers required).
    let mut declared_only: Vec<String> = Vec::new();
    if separation > 0 {
        declared_only.push(format!(
            "separation-logic ({separation} fn — Prusti for full proof)"
        ));
    }
    if timing > 0 {
        declared_only.push(format!(
            "timing-safety ({timing} fn — ctgrind/dudect for full proof)"
        ));
    }
    if termination > 0 {
        declared_only.push(format!(
            "termination ({termination} fn — cargo kani / Dafny for proof)"
        ));
    }
    if !declared_only.is_empty() {
        h.push_str("// Declared   : ");
        h.push_str(&declared_only.join("; "));
        h.push('\n');
    }

    h.push_str(
        "// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.\n",
    );
    h.push('\n');
    h
}

/// Convert a PascalCase module name to snake_case for the Rust `mod` declaration.
pub(super) fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

/// Convert a snake_case or lower identifier to PascalCase.
pub(super) fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

// ── Module-level helpers ──────────────────────────────────────────────────────

/// Indent every non-empty line in `src` by 4 spaces.
///
/// Used to nest generated code inside a `pub mod { }` wrapper.
pub(crate) fn indent_block(src: &str) -> String {
    let mut out = String::new();
    for line in src.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Emit an annotation declaration as a comment.
fn emit_annotation_decl(decl: &AnnotationDecl) -> String {
    let params: Vec<String> = decl
        .params
        .iter()
        .map(|(n, t)| format!("{}: {}", n, t))
        .collect();
    let mut src = format!("// annotation {}({})", decl.name, params.join(", "));
    if !decl.meta_annotations.is_empty() {
        let meta: Vec<String> = decl
            .meta_annotations
            .iter()
            .map(|a| format!("@{}", a.key))
            .collect();
        src.push_str(&format!(" [meta: {}]", meta.join(", ")));
    }
    src
}

/// Emit a correctness report as doc comments.
fn emit_correctness_report(report: &CorrectnessReport) -> String {
    let mut src = String::from("// correctness_report:\n");
    if !report.proved.is_empty() {
        src.push_str("//   proved:\n");
        for claim in &report.proved {
            src.push_str(&format!("//     - {}: {}\n", claim.property, claim.checker));
        }
    }
    if !report.unverified.is_empty() {
        src.push_str("//   unverified:\n");
        for (property, reason) in &report.unverified {
            src.push_str(&format!("//     - {}: {}\n", property, reason));
        }
    }
    src
}

/// Emit a pathway declaration as doc comments.
fn emit_pathway(pw: &PathwayDef) -> String {
    let mut src = format!("// pathway {}:\n", pw.name);
    for step in &pw.steps {
        src.push_str(&format!(
            "//   {} -[{}]-> {}\n",
            step.from, step.via, step.to
        ));
    }
    if let Some(c) = &pw.compensate {
        src.push_str(&format!("//   compensate: {}\n", c));
    }
    src
}

/// Emit a niche construction declaration as doc comments.
fn emit_niche_construction(nc: &NicheConstructionDef) -> String {
    let mut src = format!("// niche_construction: modifies: {}\n", nc.modifies);
    if !nc.affects.is_empty() {
        src.push_str(&format!("//   affects: [{}]\n", nc.affects.join(", ")));
    }
    if let Some(p) = &nc.probe_fn {
        src.push_str(&format!("//   probe_fn: {}\n", p));
    }
    src
}

/// Emit a sense declaration as doc comments.
fn emit_sense(sd: &SenseDef) -> String {
    let mut src = format!("// sense {}:\n", sd.name);
    if !sd.channels.is_empty() {
        src.push_str(&format!("//   channels: [{}]\n", sd.channels.join(", ")));
    }
    if let Some(r) = &sd.range {
        src.push_str(&format!("//   range: {}\n", r));
    }
    if let Some(u) = &sd.unit {
        src.push_str(&format!("//   unit: {}\n", u));
    }
    src
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_snake_case_converts_correctly() {
        assert_eq!(to_snake_case("PricingEngine"), "pricing_engine");
        assert_eq!(to_snake_case("UserService"), "user_service");
        assert_eq!(to_snake_case("M"), "m");
    }

    #[test]
    fn emits_struct_for_type_def() {
        let module = Module {
            name: "M".to_string(),
            describe: None,
            annotations: vec![],
            imports: vec![],
            spec: None,
            interface_defs: vec![],
            implements: vec![],
            provides: None,
            requires: None,
            invariants: vec![],
            test_defs: vec![],
            lifecycle_defs: vec![],
            temporal_defs: vec![],
            aspect_defs: vec![],
            being_defs: vec![],
            ecosystem_defs: vec![],
            flow_labels: vec![],
            items: vec![Item::Type(TypeDef {
                name: "Point".to_string(),
                fields: vec![
                    FieldDef {
                        name: "x".to_string(),
                        ty: TypeExpr::Base("Float".to_string()),
                        annotations: vec![],
                        span: Span::synthetic(),
                    },
                    FieldDef {
                        name: "y".to_string(),
                        ty: TypeExpr::Base("Float".to_string()),
                        annotations: vec![],
                        span: Span::synthetic(),
                    },
                ],
                span: Span::synthetic(),
            })],
            span: Span::synthetic(),
        };
        let out = RustEmitter::new().emit(&module);
        assert!(out.contains("pub struct Point"));
        assert!(out.contains("pub x: f64"));
        assert!(out.contains("pub y: f64"));
    }

    #[test]
    fn emits_enum_for_enum_def() {
        let module = Module {
            name: "M".to_string(),
            describe: None,
            annotations: vec![],
            imports: vec![],
            spec: None,
            interface_defs: vec![],
            implements: vec![],
            provides: None,
            requires: None,
            invariants: vec![],
            test_defs: vec![],
            lifecycle_defs: vec![],
            temporal_defs: vec![],
            aspect_defs: vec![],
            being_defs: vec![],
            ecosystem_defs: vec![],
            flow_labels: vec![],
            items: vec![Item::Enum(EnumDef {
                name: "Color".to_string(),
                variants: vec![
                    EnumVariant {
                        name: "Red".to_string(),
                        payload: None,
                        span: Span::synthetic(),
                    },
                    EnumVariant {
                        name: "Green".to_string(),
                        payload: None,
                        span: Span::synthetic(),
                    },
                ],
                span: Span::synthetic(),
            })],
            span: Span::synthetic(),
        };
        let out = RustEmitter::new().emit(&module);
        assert!(out.contains("pub enum Color"));
        assert!(out.contains("Red,"));
        assert!(out.contains("Green,"));
    }

    #[test]
    fn emits_debug_assert_for_require() {
        let module = Module {
            name: "M".to_string(),
            describe: None,
            annotations: vec![],
            imports: vec![],
            spec: None,
            interface_defs: vec![],
            implements: vec![],
            provides: None,
            requires: None,
            invariants: vec![],
            test_defs: vec![],
            lifecycle_defs: vec![],
            temporal_defs: vec![],
            aspect_defs: vec![],
            being_defs: vec![],
            ecosystem_defs: vec![],
            flow_labels: vec![],
            items: vec![Item::Fn(FnDef {
                name: "f".to_string(),
                describe: None,
                annotations: vec![],
                effect_tiers: vec![],
                type_params: vec![],
                type_sig: FnTypeSignature {
                    params: vec![TypeExpr::Base("Int".to_string())],
                    return_type: Box::new(TypeExpr::Base("Int".to_string())),
                },
                requires: vec![Contract {
                    expr: Expr::BinOp {
                        op: BinOpKind::Gt,
                        left: Box::new(Expr::Ident("n".to_string())),
                        right: Box::new(Expr::Literal(Literal::Int(0))),
                        span: Span::synthetic(),
                    },
                    span: Span::synthetic(),
                }],
                ensures: vec![],
                with_deps: vec![],
                separation: None,
                gradual: None,
                distribution: None,
                timing_safety: None,
                termination: None,
                proofs: vec![],
                degenerate: None,
                stochastic_process: None,
                handle_block: None,
                body: vec![],
                span: Span::synthetic(),
            })],
            span: Span::synthetic(),
        };
        let out = RustEmitter::new().emit(&module);
        assert!(out.contains("debug_assert!"));
    }
}
