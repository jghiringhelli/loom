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
mod types;

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
                    &module.name, iface_name, iface, &module.items,
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
                TemporalProperty::Eventually { type_name, target_state, .. } => {
                    body.push_str(&format!("//   eventually: {} reaches {}\n", type_name, target_state));
                }
                TemporalProperty::Never { from_state, to_state, .. } => {
                    body.push_str(&format!("//   never: {} transitions to {} [enforced]\n", from_state, to_state));
                }
                TemporalProperty::Precedes { first, second, .. } => {
                    body.push_str(&format!("//   precedes: {} before {} [enforced]\n", first, second));
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
            body.push_str(&format!("//   pointcut: {}\n", Self::fmt_pointcut(pointcut)));
        }
        for b in &aspect.before { body.push_str(&format!("//   before: {}\n", b)); }
        for a in &aspect.after { body.push_str(&format!("//   after: {}\n", a)); }
        for a in &aspect.after_throwing { body.push_str(&format!("//   after_throwing: {}\n", a)); }
        for a in &aspect.around { body.push_str(&format!("//   around: {}\n", a)); }
        if let Some(f) = &aspect.on_failure { body.push_str(&format!("//   on_failure: {}\n", f)); }
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
            body.push_str(&indent_block(&self.emit_check_invariants(&module.invariants)));
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
                "// adopt: {} from {}\nuse {}::{};\n",
                decl.interface, decl.from_module, decl.from_module, decl.interface
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
            Item::MessagingPrimitive(mp) => {
                let mut buf = String::new();
                self.emit_messaging_channel(mp, &mut buf);
                buf
            }
        }
    }
}

// ── Helpers used across submodules ────────────────────────────────────────────

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
    let params: Vec<String> = decl.params.iter()
        .map(|(n, t)| format!("{}: {}", n, t))
        .collect();
    let mut src = format!("// annotation {}({})", decl.name, params.join(", "));
    if !decl.meta_annotations.is_empty() {
        let meta: Vec<String> = decl.meta_annotations.iter()
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
        src.push_str(&format!("//   {} -[{}]-> {}\n", step.from, step.via, step.to));
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
    if let Some(r) = &sd.range { src.push_str(&format!("//   range: {}\n", r)); }
    if let Some(u) = &sd.unit  { src.push_str(&format!("//   unit: {}\n",  u)); }
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
