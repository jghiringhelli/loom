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
//! | `ensure: cond` | `// postcondition: cond` |
//! | `a \|> f` | intermediate let binding |

use crate::ast::*;
use crate::checker::units::{capitalize, collect_unit_labels, extract_unit};

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

        // Emit impl blocks for `implements`.
        for iface_name in &module.implements {
            // Find the interface def in this module (or a stub if not found)
            if let Some(iface) = module.interface_defs.iter().find(|i| &i.name == iface_name) {
                body.push('\n');
                body.push_str(&self.emit_implements_block(&module.name, iface_name, iface, &module.items));
            } else {
                // Interface defined elsewhere; emit stub impl
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

    /// Emit all ecosystem definitions as Rust submodules.
    fn emit_ecosystem(&self, eco: &EcosystemDef) -> String {
        let mut out = String::new();
        let mod_name = to_snake_case(&eco.name);

        out.push_str(&format!("// Ecosystem: {}\n", eco.name));
        if let Some(telos) = &eco.telos {
            out.push_str(&format!("// telos: {:?}\n", telos));
        }
        if !eco.members.is_empty() {
            out.push_str(&format!("// members: {}\n", eco.members.join(", ")));
        }
        out.push_str(&format!("pub mod {} {{\n", mod_name));
        out.push_str("    use super::*;\n");

        for sig in &eco.signals {
            out.push_str(&format!("\n    /// Signal: {} ({} → {})\n", sig.name, sig.from, sig.to));
            out.push_str(&format!("    pub struct {} {{\n", sig.name));
            out.push_str(&format!(
                "        pub payload: {}, // {}\n",
                self.payload_to_rust_type(&sig.payload),
                sig.payload
            ));
            out.push_str("    }\n");
        }

        // coordinate fn
        let params: Vec<String> = eco.members.iter()
            .map(|m| format!("{}: &mut {}", to_snake_case(m), m))
            .collect();
        if let Some(telos) = &eco.telos {
            out.push_str("\n    /// Coordinate the ecosystem: route signals between members.\n");
            out.push_str(&format!("    /// telos: {}\n", telos));
        } else {
            out.push_str("\n    /// Coordinate the ecosystem: route signals between members.\n");
        }
        out.push_str(&format!("    pub fn coordinate({}) {{\n", params.join(", ")));
        out.push_str("        todo!(\"implement ecosystem coordination toward telos\")\n");
        out.push_str("    }\n");

        out.push_str("}\n");
        out
    }

    /// Map a payload type string to a Rust type.
    fn payload_to_rust_type(&self, payload: &str) -> String {
        // Strip generic parameter for primitive mappings: Float<nutrients> → f64
        let base = payload.split('<').next().unwrap_or(payload).trim();
        match base {
            "Float" => "f64".to_string(),
            "Int" => "i64".to_string(),
            "String" | "Str" => "String".to_string(),
            "Bool" => "bool".to_string(),
            other => other.to_string(),
        }
    }

    // ── DI context struct ─────────────────────────────────────────────────

    /// Emit a named interface as a Rust `pub trait`.
    fn emit_interface_trait(&self, iface: &InterfaceDef) -> String {
        let mut out = String::new();
        out.push_str(&format!("/// Auto-generated trait for the `{}` interface.\n", iface.name));
        out.push_str(&format!("pub trait {} {{\n", iface.name));
        for (method_name, sig) in &iface.methods {
            let params: Vec<String> = sig.params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                .collect();
            let ret = self.emit_type_expr(&sig.return_type);
            out.push_str(&format!("    fn {}({}) -> {};\n", method_name, params.join(", "), ret));
        }
        out.push_str("}\n");
        out
    }

    /// Emit an `impl InterfaceName for ModuleImpl { fn method(...) { ... } }` block.
    fn emit_implements_block(
        &self,
        module_name: &str,
        iface_name: &str,
        iface: &InterfaceDef,
        items: &[Item],
    ) -> String {
        let mut out = String::new();
        let impl_struct = format!("{}Impl", module_name);
        // Emit a newtype struct for the impl
        out.push_str(&format!("pub struct {};\n", impl_struct));
        out.push_str(&format!("impl {} for {} {{\n", iface_name, impl_struct));
        for (method_name, sig) in &iface.methods {
            let ret = self.emit_type_expr(&sig.return_type);
            // Find matching FnDef — reuse its parameter names and body.
            if let Some(Item::Fn(fd)) = items.iter().find(|i| matches!(i, Item::Fn(fd) if fd.name == *method_name)) {
                let params: Vec<String> = fd.type_sig.params
                    .iter()
                    .zip(self.fn_param_names(fd).into_iter())
                    .map(|(ty, name)| format!("{}: {}", name, self.emit_type_expr(ty)))
                    .collect();
                let body_exprs: Vec<String> = fd.body.iter().map(|e| self.emit_expr(e)).collect();
                let body = if body_exprs.is_empty() {
                    "        todo!()".to_string()
                } else {
                    body_exprs.iter().enumerate()
                        .map(|(i, e)| if i + 1 == body_exprs.len() {
                            format!("        {}", e)
                        } else {
                            format!("        {};", e)
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                out.push_str(&format!("    fn {}({}) -> {} {{\n{}\n    }}\n",
                    method_name, params.join(", "), ret, body));
            } else {
                let params: Vec<String> = sig.params
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                    .collect();
                out.push_str(&format!("    fn {}({}) -> {} {{\n        todo!(\"not implemented\")\n    }}\n",
                    method_name, params.join(", "), ret));
            }
        }
        out.push_str("}\n");
        out
    }

    fn fn_param_names(&self, fd: &FnDef) -> Vec<String> {
        collect_body_param_names(fd, fd.type_sig.params.len())
    }

    /// Emit `#[cfg(test)] mod tests { #[test] fn name() { body } }`.
    fn emit_test_mod(&self, test_defs: &[TestDef]) -> String {
        let mut out = String::new();
        out.push_str("#[cfg(test)]\n");
        out.push_str("mod tests {\n");
        out.push_str("    use super::*;\n");
        for td in test_defs {
            out.push('\n');
            out.push_str("    #[test]\n");
            // Convert name to snake_case for Rust test fn names.
            let fn_name = td.name.replace('-', "_").to_lowercase();
            out.push_str(&format!("    fn {}() {{\n", fn_name));
            out.push_str(&format!("        {};\n", self.emit_expr(&td.body)));
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        out
    }

    /// Emit `#[cfg(debug_assertions)] fn _check_invariants() { debug_assert!(...) }`.
    fn emit_check_invariants(&self, invariants: &[Invariant]) -> String {
        let mut out = String::new();
        out.push_str("#[cfg(debug_assertions)]\n");
        out.push_str("pub fn _check_invariants() {\n");
        for inv in invariants {
            let cond = self.emit_expr(&inv.condition);
            out.push_str(&format!(
                "    debug_assert!({cond}, \"invariant '{}' violated\");\n",
                inv.name
            ));
        }
        out.push_str("}\n");
        out
    }

    /// Emit a being definition as a Rust struct + impl with fitness/regulate/evolve methods.
    fn emit_being(&self, being: &BeingDef) -> String {
        let mut out = String::new();
        let telos_desc = being.telos.as_ref().map(|t| t.description.as_str()).unwrap_or("");

        out.push_str(&format!("// Being: {}\n", being.name));
        if let Some(telos) = &being.telos {
            out.push_str(&format!("// telos: {:?}\n", telos.description));
        }
        if let Some(desc) = &being.describe {
            out.push_str(&format!("/// {}\n", desc));
        }

        out.push_str("#[derive(Debug, Clone)]\n");
        out.push_str(&format!("pub struct {} {{\n", being.name));
        if let Some(matter) = &being.matter {
            for field in &matter.fields {
                out.push_str(&format!("    pub {}: {},\n", field.name, self.emit_type_expr(&field.ty)));
            }
        }
        out.push_str("}\n\n");

        out.push_str(&format!("impl {} {{\n", being.name));

        let fitness_todo = if let Some(t) = &being.telos {
            if let Some(ff) = &t.fitness_fn {
                format!("implement fitness: {}", ff)
            } else {
                format!("implement fitness toward telos: {}", t.description)
            }
        } else {
            "implement fitness".to_string()
        };
        out.push_str("    /// Returns the fitness score relative to telos.\n");
        out.push_str(&format!("    /// telos: {:?}\n", telos_desc));
        out.push_str(&format!("    pub fn fitness(&self) -> f64 {{\n        todo!({:?})\n    }}\n", fitness_todo));

        for reg in &being.regulate_blocks {
            let var_snake = to_snake_case(&reg.variable);
            let (low, high) = reg.bounds.as_ref()
                .map(|(l, h)| (l.as_str(), h.as_str()))
                .unwrap_or(("?", "?"));
            out.push_str(&format!(
                "\n    /// Homeostatic regulation: {} → target {} within [{}, {}]\n",
                reg.variable, reg.target, low, high
            ));
            out.push_str(&format!("    pub fn regulate_{}(&mut self) {{\n", var_snake));
            out.push_str(&format!("        // target: {}, bounds: ({}, {})\n", reg.target, low, high));
            if !reg.response.is_empty() {
                let resp: Vec<String> = reg.response.iter().map(|(c, a)| format!("{} -> {}", c, a)).collect();
                out.push_str(&format!("        // response: {}\n", resp.join(", ")));
            }
            out.push_str(&format!(
                "        todo!({:?})\n    }}\n",
                format!("implement homeostatic regulation for {}", reg.variable)
            ));
        }

        if let Some(evolve) = &being.evolve_block {
            // Emit a strategy-specific method for each search case
            for sc in &evolve.search_cases {
                let method = strategy_rust_method(&sc.strategy);
                let strategy_name = strategy_rust_label(&sc.strategy);
                let step_comment = strategy_rust_step_comment(&sc.strategy);
                out.push_str(&format!("\n    /// Search strategy: {}\n", strategy_name));
                if !sc.when.trim().is_empty() {
                    out.push_str(&format!("    /// Condition: when {}\n", sc.when));
                }
                out.push_str("    /// Part of directed evolution toward telos. E[distance_to_telos] non-increasing.\n");
                out.push_str(&format!("    pub fn {}(&mut self) -> f64 {{\n", method));
                out.push_str(&format!("        // {}\n", step_comment));
                out.push_str(&format!("        // constraint: {}\n", evolve.constraint));
                out.push_str(&format!(
                    "        todo!({:?})\n    }}\n",
                    format!("implement {} step toward telos", strategy_name)
                ));
            }

            // Emit the dispatcher
            let strategy_list: Vec<&str> = evolve.search_cases.iter()
                .map(|sc| strategy_rust_label(&sc.strategy))
                .collect();
            let default_method = evolve.search_cases.first()
                .map(|sc| strategy_rust_method(&sc.strategy))
                .unwrap_or("evolve_step_impl");
            out.push_str("\n    /// Select and apply the appropriate search strategy based on current landscape.\n");
            out.push_str("    /// Directed evolution: E[distance_to_telos] must be non-increasing.\n");
            out.push_str("    pub fn evolve_step(&mut self) -> f64 {\n");
            out.push_str("        // dispatcher: select strategy based on landscape topology\n");
            if !strategy_list.is_empty() {
                out.push_str(&format!("        // strategies available: {}\n", strategy_list.join(", ")));
            }
            out.push_str(&format!("        self.{}()  // default to first strategy\n    }}\n", default_method));
        }

        out.push_str("}\n");
        out
    }

    /// Emit a `pub struct <ModName>Context { pub <dep>: <Type>, … }`.
    fn emit_context_struct(&self, module_name: &str, requires: &Requires) -> String {
        let fields: Vec<String> = requires
            .deps
            .iter()
            .map(|(name, ty)| format!("    pub {}: {},", name, self.emit_type_expr(ty)))
            .collect();
        format!(
            "#[derive(Debug)]\npub struct {}Context {{\n{}\n}}\n",
            module_name,
            fields.join("\n")
        )
    }

    /// Emit a function definition, optionally prepending `ctx: &<ModName>Context`
    /// when the function has `with_deps` and the module has a `requires` block.
    fn emit_fn_def_with_context(
        &self,
        fd: &FnDef,
        module_name: &str,
        module_has_requires: bool,
    ) -> String {
        let inject_ctx = module_has_requires && !fd.with_deps.is_empty();
        self.emit_fn_def_inner(fd, if inject_ctx { Some(module_name) } else { None })
    }

    // ── Provides trait ────────────────────────────────────────────────────

    fn emit_provides_trait(&self, module_name: &str, provides: &Provides) -> String {
        let mut out = String::new();
        out.push_str(&format!("/// Auto-generated trait for the `{}` provides interface.\n", module_name));
        out.push_str(&format!("pub trait {} {{\n", module_name));
        for (op_name, sig) in &provides.ops {
            let params: Vec<String> = sig
                .params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                .collect();
            let ret = self.emit_type_expr(&sig.return_type);
            out.push_str(&format!(
                "    fn {}({}) -> {};\n",
                op_name,
                params.join(", "),
                ret
            ));
        }
        out.push_str("}\n\n");
        out
    }

    // ── Type definition ───────────────────────────────────────────────────

    fn emit_type_def(&self, td: &TypeDef) -> String {
        let has_pii = td.fields.iter().any(|f| f.annotations.iter().any(|a| a.key == "pii"));
        let mut out = String::new();
        if has_pii {
            out.push_str("// loom: module contains PII fields — handle with care\n");
        }
        let fields: Vec<String> = td
            .fields
            .iter()
            .map(|f| {
                let mut field_out = String::new();
                for ann in &f.annotations {
                    match ann.key.as_str() {
                        "pii"             => field_out.push_str("    #[loom_pii]\n"),
                        "secret"          => field_out.push_str("    #[loom_secret]\n"),
                        "encrypt-at-rest" => field_out.push_str("    #[loom_encrypt_at_rest]\n"),
                        "never-log"       => field_out.push_str(&format!("    // NEVER LOG: {}\n", f.name)),
                        _ => {}
                    }
                }
                field_out.push_str(&format!("    pub {}: {},", f.name, self.emit_type_expr(&f.ty)));
                field_out
            })
            .collect();
        out.push_str(&format!(
            "#[derive(Debug, Clone, PartialEq)]\npub struct {} {{\n{}\n}}\n",
            td.name,
            fields.join("\n")
        ));
        out
    }

    // ── Enum definition ───────────────────────────────────────────────────

    fn emit_enum_def(&self, ed: &EnumDef) -> String {
        let variants: Vec<String> = ed
            .variants
            .iter()
            .map(|v| match &v.payload {
                Some(ty) => format!("    {}({}),", v.name, self.emit_type_expr(ty)),
                None => format!("    {},", v.name),
            })
            .collect();
        format!(
            "#[derive(Debug, Clone, PartialEq)]\npub enum {} {{\n{}\n}}\n",
            ed.name,
            variants.join("\n")
        )
    }

    // ── Refined type ──────────────────────────────────────────────────────

    fn emit_refined_type(&self, rt: &RefinedType) -> String {
        let base = self.emit_type_expr(&rt.base_type);
        let pred = self.emit_expr(&rt.predicate);
        format!(
            "#[derive(Debug, Clone, PartialEq)]\n\
             pub struct {name}({base});\n\n\
             impl TryFrom<{base}> for {name} {{\n\
             \x20\x20\x20\x20type Error = String;\n\
             \x20\x20\x20\x20fn try_from(value: {base}) -> Result<Self, Self::Error> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20debug_assert!({pred}, \"refined type invariant violated for {name}\");\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Ok({name}(value))\n\
             \x20\x20\x20\x20}}\n\
             }}\n",
            name = rt.name,
            base = base,
            pred = pred,
        )
    }

    // ── Function definition ───────────────────────────────────────────────

    fn emit_fn_def(&self, fd: &FnDef) -> String {
        self.emit_fn_def_inner(fd, None)
    }

    fn emit_fn_def_inner(&self, fd: &FnDef, ctx_module: Option<&str>) -> String {
        let is_effectful =
            matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _));

        let mut out = String::new();

        // Emit describe: as a Rust doc comment.
        if let Some(desc) = &fd.describe {
            for line in desc.lines() {
                out.push_str(&format!("/// {}\n", line));
            }
        }

        // Emit @annotations as doc comments, and #[deprecated] for @deprecated.
        for ann in &fd.annotations {
            let desc = algebraic_annotation_desc(&ann.key);
            if let Some(d) = desc {
                out.push_str(&format!("/// @{} — {}\n", ann.key, d));
            } else {
                out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value));
            }
            if ann.key == "deprecated" {
                out.push_str(&format!(
                    "#[deprecated(note = \"{}\")]\n",
                    ann.value.replace('"', "\\\"")
                ));
            }
        }

        // Emit consequence tiers from Effect<[X@tier, ...]> as doc comments.
        for (eff, tier) in &fd.effect_tiers {
            let tier_str = match tier {
                ConsequenceTier::Pure         => "pure",
                ConsequenceTier::Reversible   => "reversible",
                ConsequenceTier::Irreversible => "irreversible",
            };
            out.push_str(&format!("// effect-tier: {} -> {}\n", eff, tier_str));
        }

        let mut params: Vec<String> = Vec::new();

        // Inject `ctx: &<ModName>Context` as the first parameter when requested.
        if let Some(mod_name) = ctx_module {
            params.push(format!("ctx: &{}Context", mod_name));
        }

        params.extend(
            fd.type_sig
                .params
                .iter()
                .enumerate()
                .zip(collect_body_param_names(fd, fd.type_sig.params.len()))
                .map(|((_, ty), name)| format!("{}: {}", name, self.emit_type_expr(ty))),
        );

        let ret = if is_effectful {
            match fd.type_sig.return_type.as_ref() {
                TypeExpr::Effect(_, inner) => {
                    format!("Result<{}, Box<dyn std::error::Error>>", self.emit_type_expr(inner))
                }
                _ => self.emit_type_expr(&fd.type_sig.return_type),
            }
        } else {
            self.emit_type_expr(&fd.type_sig.return_type)
        };

        let mut body_lines: Vec<String> = Vec::new();

        // Emit `require:` contracts as `debug_assert!`.
        for contract in &fd.requires {
            body_lines.push(format!(
                "    debug_assert!({}, \"precondition violated: {}\");",
                self.emit_expr(&contract.expr),
                // Escape the predicate text for use in a string literal.
                self.emit_expr(&contract.expr).replace('"', "\\\""),
            ));
        }

        // Emit `ensure:` contracts as real `debug_assert!` — but AFTER the body.
        // When ensures are present: capture return value in `_loom_result`, assert, then return it.
        let has_ensures = !fd.ensures.is_empty();

        // Emit body expressions.
        let body_count = fd.body.len();
        if has_ensures && body_count > 0 {
            // Emit all but the last expression as statements.
            for expr in &fd.body[..body_count - 1] {
                body_lines.push(format!("    {};", self.emit_expr(expr)));
            }
            // Capture the last expression as `_loom_result`.
            let last = &fd.body[body_count - 1];
            body_lines.push(format!("    let _loom_result = {};", self.emit_expr(last)));
            // Emit ensure: as debug_assert! using _loom_result for `result`.
            for contract in &fd.ensures {
                let raw = self.emit_expr(&contract.expr);
                // Replace `result` identifier references with `_loom_result`.
                let cond = raw.replace("result", "_loom_result");
                body_lines.push(format!(
                    "    debug_assert!({cond}, \"ensure: {}\");",
                    cond.replace('"', "\\\""),
                ));
            }
            body_lines.push("    _loom_result".to_string());
        } else {
            for (i, expr) in fd.body.iter().enumerate() {
                if i + 1 == body_count {
                    body_lines.push(format!("    {}", self.emit_expr(expr)));
                } else {
                    body_lines.push(format!("    {};", self.emit_expr(expr)));
                }
            }
        }

        if body_lines.is_empty() {
            body_lines.push("    todo!(\"Phase 1 stub — body not yet implemented\")".to_string());
        }

        out.push_str(&format!(
            "pub fn {}{name_generics}({params}) -> {ret} {{\n{body}\n}}\n",
            fd.name,
            name_generics = if fd.type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", fd.type_params.join(", "))
            },
            params = params.join(", "),
            ret = ret,
            body = body_lines.join("\n"),
        ));
        out
    }

    // ── Type expressions ──────────────────────────────────────────────────

    fn emit_type_expr(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Base(name) => self.map_base_type(name),
            TypeExpr::Generic(name, params) => {
                // Unit-annotated primitives: Float<usd> → Usd, Int<meters> → Meters
                if let Some(unit) = extract_unit(ty) {
                    return capitalize(unit);
                }
                let ps: Vec<String> = params.iter().map(|p| self.emit_type_expr(p)).collect();
                // Map Loom stdlib collection types to Rust equivalents.
                match name.as_str() {
                    "List" if ps.len() == 1 => format!("Vec<{}>", ps[0]),
                    "Map"  if ps.len() == 2 => format!("HashMap<{}, {}>", ps[0], ps[1]),
                    "Set"  if ps.len() == 1 => format!("HashSet<{}>", ps[0]),
                    _ => format!("{}<{}>", name, ps.join(", ")),
                }
            }
            TypeExpr::Effect(_, inner) => {
                format!("Result<{}, Box<dyn std::error::Error>>", self.emit_type_expr(inner))
            }
            TypeExpr::Option(inner) => format!("Option<{}>", self.emit_type_expr(inner)),
            TypeExpr::Result(ok, err) => format!(
                "Result<{}, {}>",
                self.emit_type_expr(ok),
                self.emit_type_expr(err)
            ),
            TypeExpr::Tuple(elems) => {
                let es: Vec<String> = elems.iter().map(|e| self.emit_type_expr(e)).collect();
                format!("({})", es.join(", "))
            }
            // TypeVar should be resolved before codegen; emit a placeholder if it leaks.
            TypeExpr::TypeVar(id) => format!("/* infer:?{} */", id),
        }
    }

    /// Map Loom primitive type names to Rust equivalents.
    fn map_base_type(&self, name: &str) -> String {
        match name {
            "Int" => "i64".to_string(),
            "Float" => "f64".to_string(),
            "String" | "Str" => "String".to_string(),
            "Bool" => "bool".to_string(),
            "Unit" => "()".to_string(),
            other => other.to_string(),
        }
    }

    // ── Unit newtypes ─────────────────────────────────────────────────────

    /// Emit newtype structs for every unit label used in the module.
    ///
    /// For each unique unit (e.g. `usd`) this emits:
    /// ```rust,ignore
    /// #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    /// pub struct Usd(pub f64);
    /// impl std::ops::Add for Usd { … }
    /// impl std::ops::Sub for Usd { … }
    /// impl std::ops::Mul<f64> for Usd { … }
    /// ```
    pub fn emit_unit_types(&self, module: &Module) -> String {
        let units = collect_unit_labels(module);
        if units.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for unit in &units {
            let tn = capitalize(unit);
            out.push_str(&format!(
                "#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]\npub struct {}(pub f64);\n",
                tn
            ));
            out.push_str(&format!(
                "impl std::ops::Add for {0} {{ type Output = {0}; fn add(self, rhs: {0}) -> {0} {{ {0}(self.0 + rhs.0) }} }}\n",
                tn
            ));
            out.push_str(&format!(
                "impl std::ops::Sub for {0} {{ type Output = {0}; fn sub(self, rhs: {0}) -> {0} {{ {0}(self.0 - rhs.0) }} }}\n",
                tn
            ));
            out.push_str(&format!(
                "impl std::ops::Mul<f64> for {0} {{ type Output = {0}; fn mul(self, rhs: f64) -> {0} {{ {0}(self.0 * rhs) }} }}\n",
                tn
            ));
            out.push('\n');
        }
        out
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn emit_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Let { name, value, .. } => {
                format!("let {} = {}", name, self.emit_expr(value))
            }
            Expr::Literal(lit) => self.emit_literal(lit),
            // `todo` is a Loom placeholder that maps to Rust's `todo!()` macro.
            Expr::Ident(name) if name == "todo" => "todo!()".to_string(),
            Expr::Ident(name) => name.clone(),
            Expr::Call { func, args, .. } => {
                // Recognize built-in HOF call forms and emit as iterator chains.
                if let Expr::Ident(name) = func.as_ref() {
                    match (name.as_str(), args.len()) {
                        ("map", 2) => {
                            return format!(
                                "{}.iter().map({}).collect::<Vec<_>>()",
                                self.emit_expr(&args[0]),
                                self.emit_expr(&args[1])
                            );
                        }
                        ("filter", 2) => {
                            return format!(
                                "{}.iter().filter({}).cloned().collect::<Vec<_>>()",
                                self.emit_expr(&args[0]),
                                self.emit_expr(&args[1])
                            );
                        }
                        ("fold", 3) => {
                            return format!(
                                "{}.iter().fold({}, {})",
                                self.emit_expr(&args[0]),
                                self.emit_expr(&args[1]),
                                self.emit_expr(&args[2])
                            );
                        }
                        // for_all(|x: T| pred) — property test over edge cases
                        ("for_all", 1) => {
                            if let Expr::Lambda { params, body, .. } = &args[0] {
                                if let Some((param_name, _)) = params.first() {
                                    let pred = self.emit_expr(body);
                                    return format!(
                                        "{{ \
                                            let _edge_cases: &[i64] = &[0, 1, -1, i64::MAX, i64::MIN]; \
                                            for &{pn} in _edge_cases {{ \
                                                assert!({pred}, \"for_all property failed for {{}} = {{}}\", \"{pn}\", {pn}); \
                                            }} \
                                        }}",
                                        pn = param_name,
                                        pred = pred,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let f = self.emit_expr(func);
                let as_str: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{}({})", f, as_str.join(", "))
            }
            Expr::Pipe { left, right, .. } => {
                // `a |> f` → `{ let _p = a; f(_p) }`
                let l = self.emit_expr(left);
                let r = self.emit_expr(right);
                format!("{{ let _pipe = {}; {}(_pipe) }}", l, r)
            }
            Expr::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.emit_expr(object), field)
            }
            Expr::BinOp { op, left, right, .. } => {
                let op_str = match op {
                    BinOpKind::Add => "+",
                    BinOpKind::Sub => "-",
                    BinOpKind::Mul => "*",
                    BinOpKind::Div => "/",
                    BinOpKind::Eq => "==",
                    BinOpKind::Ne => "!=",
                    BinOpKind::Lt => "<",
                    BinOpKind::Le => "<=",
                    BinOpKind::Gt => ">",
                    BinOpKind::Ge => ">=",
                    BinOpKind::And => "&&",
                    BinOpKind::Or => "||",
                };
                format!(
                    "({} {} {})",
                    self.emit_expr(left),
                    op_str,
                    self.emit_expr(right)
                )
            }
            Expr::InlineRust(code) => code.clone(),
            Expr::As(inner, ty) => {
                format!("({} as {})", self.emit_expr(inner), self.emit_type_expr(ty))
            }
            Expr::Lambda { params, body, .. } => {
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|(name, ty)| {
                        if let Some(t) = ty {
                            format!("{}: {}", name, self.emit_type_expr(t))
                        } else {
                            name.clone()
                        }
                    })
                    .collect();
                format!("|{}| {}", param_strs.join(", "), self.emit_expr(body))
            }
            Expr::ForIn { var, iter, body, .. } => {
                format!(
                    "for {} in ({}).iter() {{ {} }}",
                    var,
                    self.emit_expr(iter),
                    self.emit_expr(body)
                )
            }
            Expr::Tuple(elems, _) => {
                let inner: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", inner.join(", "))
            }
            Expr::Try(inner, _) => {
                format!("{}?", self.emit_expr(inner))
            }
            Expr::Match { subject, arms, .. } => {
                let s = self.emit_expr(subject);
                let arms_str: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let pat = self.emit_pattern(&arm.pattern);
                        let guard = arm
                            .guard
                            .as_ref()
                            .map(|g| format!(" if {}", self.emit_expr(g)))
                            .unwrap_or_default();
                        format!("        {}{} => {}", pat, guard, self.emit_expr(&arm.body))
                    })
                    .collect();
                format!("match {} {{\n{}\n    }}", s, arms_str.join(",\n"))
            }
        }
    }

    fn emit_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                // Ensure the float literal always has a decimal point in Rust.
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Literal::Str(s) => format!("{:?}", s),
            Literal::Bool(b) => b.to_string(),
            Literal::Unit => "()".to_string(),
        }
    }

    fn emit_pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Variant(name, sub_pats) => {
                if sub_pats.is_empty() {
                    name.clone()
                } else {
                    let subs: Vec<String> =
                        sub_pats.iter().map(|p| self.emit_pattern(p)).collect();
                    format!("{}({})", name, subs.join(", "))
                }
            }
            Pattern::Ident(name) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(lit) => self.emit_literal(lit),
        }
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Returns the snake_case method name for a search strategy.
fn strategy_rust_method(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent   => "evolve_gradient_descent",
        SearchStrategy::StochasticGradient => "evolve_stochastic_gradient",
        SearchStrategy::SimulatedAnnealing => "evolve_simulated_annealing",
        SearchStrategy::DerivativeFree    => "evolve_derivative_free",
        SearchStrategy::Mcmc              => "evolve_mcmc",
    }
}

/// Returns a short label for a search strategy (used in comments).
fn strategy_rust_label(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent   => "gradient_descent",
        SearchStrategy::StochasticGradient => "stochastic_gradient",
        SearchStrategy::SimulatedAnnealing => "simulated_annealing",
        SearchStrategy::DerivativeFree    => "derivative_free",
        SearchStrategy::Mcmc              => "mcmc",
    }
}

/// Returns a one-line implementation comment for a search strategy step.
fn strategy_rust_step_comment(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent   => "gradient descent step: adjust parameters along negative gradient",
        SearchStrategy::StochasticGradient => "stochastic gradient step: noisy gradient estimation",
        SearchStrategy::SimulatedAnnealing => "simulated annealing step: probabilistic uphill acceptance",
        SearchStrategy::DerivativeFree    => "derivative-free step: explore without gradient information",
        SearchStrategy::Mcmc              => "MCMC step: sample from posterior landscape",
    }
}

/// Returns a human-readable description for known algebraic annotation keys,
/// or `None` if the key is not a recognised algebraic property.
fn algebraic_annotation_desc(key: &str) -> Option<&'static str> {
    match key {
        "idempotent"   => Some("safe to retry"),
        "commutative"  => Some("argument order does not matter"),
        "associative"  => Some("grouping does not matter"),
        "at-most-once" => Some("must not be called more than once"),
        "exactly-once" => Some("must be called exactly once"),
        "pure"         => Some("no side effects"),
        "monotonic"    => Some("output only increases"),
        _ => None,
    }
}

/// Convert a PascalCase module name to snake_case for the Rust `mod` declaration.
fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

/// Identifiers that look like free variables but are actually language keywords
/// or built-in macros and must not be used as parameter names.
const PARAM_NAME_BUILTINS: &[&str] = &["todo", "panic", "unreachable", "unimplemented"];

/// Collect free variable names from a function body in first-appearance order.
///
/// These become the parameter names in the emitted Rust signature, matching the
/// names the programmer already uses in the body (e.g., `p.x`, `line.quantity`).
///
/// Returns at most `max_params` names; falls back to `arg{i}` for any slot
/// that couldn't be filled from the body.
fn collect_body_param_names(fd: &FnDef, max_params: usize) -> Vec<String> {
    use std::collections::HashSet;

    // Collect let-bound names to exclude them.
    let mut let_bound: HashSet<String> = HashSet::new();
    for expr in &fd.body {
        collect_let_names(expr, &mut let_bound);
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();

    // Scan body expressions first, then contract expressions.
    let all_exprs: Vec<&Expr> = fd
        .body
        .iter()
        .chain(fd.requires.iter().map(|c| &c.expr))
        .chain(fd.ensures.iter().map(|c| &c.expr))
        .collect();

    for expr in all_exprs {
        scan_free_idents(expr, &let_bound, &mut seen, &mut ordered);
        if ordered.len() >= max_params {
            break;
        }
    }

    // Pad with `arg{i}` if the body doesn't give us enough names.
    (0..max_params)
        .map(|i| {
            ordered
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("arg{}", i))
        })
        .collect()
}

/// Collect names introduced by `let` bindings (to exclude them from free-var scan).
fn collect_let_names(expr: &Expr, out: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Let { name, value, .. } => {
            out.insert(name.clone());
            collect_let_names(value, out);
        }
        Expr::BinOp { left, right, .. } => {
            collect_let_names(left, out);
            collect_let_names(right, out);
        }
        Expr::Match { subject, arms, .. } => {
            collect_let_names(subject, out);
            for arm in arms {
                collect_let_names(&arm.body, out);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_let_names(left, out);
            collect_let_names(right, out);
        }
        Expr::Call { func, args, .. } => {
            collect_let_names(func, out);
            for a in args {
                collect_let_names(a, out);
            }
        }
        Expr::FieldAccess { object, .. } => collect_let_names(object, out),
        Expr::Ident(_) | Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque — no let bindings inside
        Expr::As(inner, _) => collect_let_names(inner, out),
        Expr::Lambda { body, .. } => collect_let_names(body, out),
        Expr::ForIn { iter, body, .. } => {
            collect_let_names(iter, out);
            collect_let_names(body, out);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| collect_let_names(e, out)),
        Expr::Try(inner, _) => collect_let_names(inner, out),
    }
}

/// Walk an expression and collect free identifiers in first-appearance order.
fn scan_free_idents(
    expr: &Expr,
    let_bound: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
    ordered: &mut Vec<String>,
) {
    match expr {
        Expr::Ident(name) => {
            if !let_bound.contains(name)
                && !seen.contains(name)
                && !PARAM_NAME_BUILTINS.contains(&name.as_str())
                // Skip uppercase identifiers — they're type/variant names, not params.
                && name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
            {
                seen.insert(name.clone());
                ordered.push(name.clone());
            }
        }
        Expr::Let { value, .. } => scan_free_idents(value, let_bound, seen, ordered),
        Expr::BinOp { left, right, .. } => {
            scan_free_idents(left, let_bound, seen, ordered);
            scan_free_idents(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            if !matches!(func.as_ref(), Expr::Ident(_)) {
                scan_free_idents(func, let_bound, seen, ordered);
            }
            for a in args {
                scan_free_idents(a, let_bound, seen, ordered);
            }
        }
        Expr::Pipe { left, right, .. } => {
            scan_free_idents(left, let_bound, seen, ordered);
            scan_free_idents(right, let_bound, seen, ordered);
        }
        Expr::Match { subject, arms, .. } => {
            scan_free_idents(subject, let_bound, seen, ordered);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    scan_free_idents(g, let_bound, seen, ordered);
                }
                scan_free_idents(&arm.body, let_bound, seen, ordered);
            }
        }
        Expr::FieldAccess { object, .. } => scan_free_idents(object, let_bound, seen, ordered),
        Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque — no free variable names to scan
        Expr::As(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
        Expr::Lambda { params, body, .. } => {
            // Lambda params are bound within the body — extend let_bound to exclude them.
            let mut inner_bound = let_bound.clone();
            for (name, _) in params {
                inner_bound.insert(name.clone());
            }
            scan_free_idents(body, &inner_bound, seen, ordered);
        }
        Expr::ForIn { iter, body, .. } => {
            scan_free_idents(iter, let_bound, seen, ordered);
            scan_free_idents(body, let_bound, seen, ordered);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| scan_free_idents(e, let_bound, seen, ordered)),
        Expr::Try(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

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
                body: vec![],
                span: Span::synthetic(),
            })],
            span: Span::synthetic(),
        };
        let out = RustEmitter::new().emit(&module);
        assert!(out.contains("debug_assert!"));
    }
}

