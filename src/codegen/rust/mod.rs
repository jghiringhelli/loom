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
mod exprs;
mod functions;
mod stores;
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
        out.push_str("use std::convert::TryFrom;\n\n");

        // Module wrapper.
        let mod_name = to_snake_case(&module.name);

        // Emit module-level describe: and @annotations as doc comments.
        if let Some(desc) = &module.describe {
            for line in desc.lines() {
                out.push_str(&format!("/// {}\n", line));
            }
        }
        for ann in &module.annotations {
            out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value));
        }

        out.push_str(&format!("pub mod {} {{\n", mod_name));
        out.push_str("    use super::*;\n");

        // Emit information-flow label summary comment.
        if !module.flow_labels.is_empty() {
            out.push_str("    // information-flow labels:\n");
            for fl in &module.flow_labels {
                let types_str = fl.types.join(", ");
                out.push_str(&format!("    //   {}: {}\n", fl.label, types_str));
            }
        }

        // Emit `use super::snake_module::*;` for each import.
        for imp in &module.imports {
            out.push_str(&format!("    use super::{}::*;\n", to_snake_case(imp)));
        }

        // Render the module body first to detect which stdlib imports are needed.
        let mut body = String::new();

        // Emit unit newtypes first (before all other items).
        let unit_types_src = self.emit_unit_types(module);
        if !unit_types_src.is_empty() {
            body.push('\n');
            for line in unit_types_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("    ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Emit interface trait definitions.
        for iface in &module.interface_defs {
            body.push('\n');
            body.push_str(&self.emit_interface_trait(iface));
        }

        // Emit phantom state types for each lifecycle declaration.
        for lc in &module.lifecycle_defs {
            body.push('\n');
            body.push_str(&format!("// Lifecycle states for {}\n", lc.type_name));
            for state in &lc.states {
                body.push_str(&format!("pub struct {};\n", state));
            }
        }

        // Emit temporal property blocks as documentation comments.
        for temporal in &module.temporal_defs {
            body.push('\n');
            body.push_str(&format!("// temporal invariants: {}\n", temporal.name));
            for prop in &temporal.properties {
                match prop {
                    TemporalProperty::Always { .. } => {
                        body.push_str("//   always: [invariant verified at compile time]\n");
                    }
                    TemporalProperty::Eventually { type_name, target_state, .. } => {
                        body.push_str(&format!(
                            "//   eventually: {} reaches {}\n", type_name, target_state
                        ));
                    }
                    TemporalProperty::Never { from_state, to_state, .. } => {
                        body.push_str(&format!(
                            "//   never: {} transitions to {} [enforced]\n", from_state, to_state
                        ));
                    }
                    TemporalProperty::Precedes { first, second, .. } => {
                        body.push_str(&format!(
                            "//   precedes: {} before {} [enforced]\n", first, second
                        ));
                    }
                }
            }
        }

        // Emit aspect blocks as doc comments (M66).
        for aspect in &module.aspect_defs {
            body.push('\n');
            body.push_str(&format!(
                "// aspect: {} (order: {})\n",
                aspect.name,
                aspect.order.map_or("unset".to_string(), |o| o.to_string())
            ));
            if let Some(pointcut) = &aspect.pointcut {
                body.push_str(&format!("//   pointcut: {}\n", Self::fmt_pointcut(pointcut)));
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

        // Emit impl blocks for `implements`.
        for iface_name in &module.implements {
            if let Some(iface) = module.interface_defs.iter().find(|i| &i.name == iface_name) {
                body.push('\n');
                body.push_str(&self.emit_implements_block(&module.name, iface_name, iface, &module.items));
            } else {
                body.push('\n');
                body.push_str(&format!("// impl {} for {}Impl {{ /* external interface */ }}\n", iface_name, module.name));
            }
        }

        // The provides trait lives inside the module so type references resolve.
        if let Some(provides) = &module.provides {
            body.push('\n');
            body.push_str(&self.emit_provides_trait(&module.name, provides));
        }

        // DI context struct.
        if let Some(requires) = &module.requires {
            body.push('\n');
            body.push_str(&self.emit_context_struct(&module.name, requires));
            body.push('\n');
        }

        for item in &module.items {
            let item_src = match item {
                Item::Type(td) => self.emit_type_def(td),
                Item::Enum(ed) => self.emit_enum_def(ed),
                Item::Fn(fd) => self.emit_fn_def_with_context(fd, &module.name, module.requires.is_some()),
                Item::RefinedType(rt) => self.emit_refined_type(rt),
                Item::Proposition(prop) => self.emit_proposition(prop),
                Item::Functor(f) => self.emit_functor(f),
                Item::Monad(m) => self.emit_monad(m),
                Item::Certificate(cert) => self.emit_certificate(cert),
                Item::AnnotationDecl(decl) => {
                    let mut src = format!("// annotation {}(", decl.name);
                    let params: Vec<String> = decl.params.iter()
                        .map(|(n, t)| format!("{}: {}", n, t))
                        .collect();
                    src.push_str(&params.join(", "));
                    src.push(')');
                    if !decl.meta_annotations.is_empty() {
                        let meta: Vec<String> = decl.meta_annotations.iter()
                            .map(|a| format!("@{}", a.key))
                            .collect();
                        src.push_str(&format!(" [meta: {}]", meta.join(", ")));
                    }
                    src
                }
                Item::CorrectnessReport(report) => {
                    let mut src = String::from("// correctness_report:\n");
                    if !report.proved.is_empty() {
                        src.push_str("//   proved:\n");
                        for claim in &report.proved {
                            src.push_str(&format!(
                                "//     - {}: {}\n",
                                claim.property, claim.checker
                            ));
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
                Item::Pathway(pw) => {
                    let mut src = format!("// pathway {}:\n", pw.name);
                    for step in &pw.steps {
                        src.push_str(&format!("//   {} -[{}]-> {}\n", step.from, step.via, step.to));
                    }
                    if let Some(c) = &pw.compensate {
                        src.push_str(&format!("//   compensate: {}\n", c));
                    }
                    src
                }
                Item::SymbioticImport { module, kind, .. } => {
                    format!("// symbiotic: kind: {}, module: {}\n", kind, module)
                }
                Item::Adopt(decl) => {
                    format!(
                        "// adopt: {} from {}\nuse {}::{};\n",
                        decl.interface, decl.from_module, decl.from_module, decl.interface
                    )
                }
                Item::NicheConstruction(nc) => {
                    let mut src = format!("// niche_construction: modifies: {}\n", nc.modifies);
                    if !nc.affects.is_empty() {
                        src.push_str(&format!("//   affects: [{}]\n", nc.affects.join(", ")));
                    }
                    if let Some(p) = &nc.probe_fn {
                        src.push_str(&format!("//   probe_fn: {}\n", p));
                    }
                    src
                }
                Item::Sense(sd) => {
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
                Item::Store(sd) => self.codegen_store(sd),
                Item::TypeAlias(name, ty, _) => {
                    format!("pub type {} = {};\n", name, self.emit_type_expr(ty))
                }
                Item::Session(sd) => {
                    format!("// session_type: {}\n", sd.name)
                }
                Item::Effect(ed) => {
                    format!("// effect_def: {}\n", ed.name)
                }
                Item::UseCase(uc) => self.emit_usecase(uc),
                Item::Property(pb) => {
                    // M109: property-based test — QuickCheck (Claessen & Hughes 2000).
                    // V3 target: replace todo!() with actual proptest! invocation.
                    format!(
                        "#[test]\n#[doc = \"Property: {} — forall {}: {}\"]\nfn property_{}() {{\n    // Property-based test: forall {}: {}\n    // invariant: {}\n    // samples: {}, shrink: {}\n    todo!(\"property: {} — implement with proptest or similar\")\n}}\n",
                        pb.name, pb.var_name, pb.var_type,
                        to_snake_case(&pb.name),
                        pb.var_name, pb.var_type,
                        pb.invariant,
                        pb.samples, pb.shrink,
                        pb.name
                    )
                }
                Item::BoundaryBlock(bb) => {
                    format!(
                        "// boundary: exports=[{}] private=[{}] sealed=[{}]\n",
                        bb.exports.join(", "),
                        bb.private.join(", "),
                        bb.sealed.join(", ")
                    )
                }
                Item::MessagingPrimitive(mp) => {
                    let mut src = format!("// messaging_primitive {}:\n", mp.name);
                    src.push_str(&format!("//   pattern: {:?}\n", mp.pattern));
                    if !mp.guarantees.is_empty() {
                        src.push_str(&format!("//   guarantees: [{}]\n", mp.guarantees.join(", ")));
                    }
                    if mp.timeout_mandatory {
                        src.push_str("//   timeout: mandatory\n");
                    }
                    src
                }
            };
            body.push('\n');
            for line in item_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("    ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Emit _check_invariants() when invariants are declared.
        if !module.invariants.is_empty() {
            let inv_src = self.emit_check_invariants(&module.invariants);
            body.push('\n');
            for line in inv_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("    ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Emit being definitions.
        for being in &module.being_defs {
            let being_src = self.emit_being(being);
            body.push('\n');
            for line in being_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("    ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Emit ecosystem definitions.
        for eco in &module.ecosystem_defs {
            let eco_src = self.emit_ecosystem(eco);
            body.push('\n');
            for line in eco_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("    ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Inject stdlib collection imports when they appear in the rendered body.
        if body.contains("HashMap") {
            out.push_str("    use std::collections::HashMap;\n");
        }
        if body.contains("HashSet") {
            out.push_str("    use std::collections::HashSet;\n");
        }

        // Bring all enum variants into scope so match patterns work unqualified.
        for item in &module.items {
            if let Item::Enum(ed) = item {
                out.push_str(&format!("    use self::{}::*;\n", ed.name));
            }
        }

        out.push_str(&body);

        // Emit `#[cfg(test)] mod tests { ... }` if test_defs are present.
        if !module.test_defs.is_empty() {
            let tests_src = self.emit_test_mod(&module.test_defs);
            out.push('\n');
            for line in tests_src.lines() {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    out.push_str("    ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }

        out.push_str("}\n");
        out
    }
}

// ── Helpers used across submodules ────────────────────────────────────────────

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
                    FieldDef { name: "x".to_string(), ty: TypeExpr::Base("Float".to_string()), annotations: vec![], span: Span::synthetic() },
                    FieldDef { name: "y".to_string(), ty: TypeExpr::Base("Float".to_string()), annotations: vec![], span: Span::synthetic() },
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
                    EnumVariant { name: "Red".to_string(), payload: None, span: Span::synthetic() },
                    EnumVariant { name: "Green".to_string(), payload: None, span: Span::synthetic() },
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
