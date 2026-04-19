//! Function, interface, use-case, and contract emitters.

use super::{to_snake_case, RustEmitter};
use crate::ast::*;

/// Returns true if `expr_text` is a bare PascalCase identifier — a struct/enum
/// type name used as a stub body rather than a real expression.
fn is_type_name_stub(expr_text: &str) -> bool {
    let t = expr_text.trim();
    t.len() > 1
        && t.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && t.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !t.starts_with("true")
        && !t.starts_with("false")
}

impl RustEmitter {
    /// Emit a property-based test from a `property:` block.
    ///
    /// V3 implementation: deterministic edge-case `#[test]` fn.
    /// V3+ implementation: `proptest!` block with unlimited random sampling.
    ///
    /// The invariant string is translated from Loom surface syntax to Rust:
    /// - `x = y`  → `x == y`   (standalone `=` only; `<=`/`>=`/`!=` preserved)
    /// - ` and `  → ` && `
    /// - ` or `   → ` || `
    /// - `not `   → `!`
    ///
    /// QuickCheck (Claessen & Hughes 2000) + proptest (Hypothesis-style) approach.
    pub(super) fn emit_property_test(&self, pb: &PropertyBlock) -> String {
        let fn_name = to_snake_case(&pb.name);
        let invariant_rust = property_invariant_to_rust(&pb.invariant, &pb.var_name);
        let (var_type_rust, edge_cases) = property_edge_cases(&pb.var_type);
        let strategy = property_proptest_strategy(&pb.var_type, var_type_rust);

        let mut out = String::new();
        out.push_str(&format!(
            "/// Property test: {} — forall {}: {}\n",
            pb.name, pb.var_name, pb.var_type
        ));
        out.push_str(&format!("/// invariant: {}\n", pb.invariant));
        out.push_str(&format!(
            "/// samples (edge cases): {}, shrink: {}\n",
            pb.samples, pb.shrink
        ));
        out.push_str("/// V3: QuickCheck edge cases. V3+: proptest random sampling.\n");

        // V3 — deterministic edge cases
        out.push_str("#[test]\n");
        out.push_str(&format!("fn property_{}_edge_cases() {{\n", fn_name));
        out.push_str(&format!(
            "    let edge_cases: &[{vt}] = &[{cases}];\n",
            vt = var_type_rust,
            cases = edge_cases
        ));
        out.push_str(&format!(
            "    for &{vn} in edge_cases {{\n",
            vn = pb.var_name
        ));
        out.push_str(&format!(
            "        assert!({inv}, \"property '{name}' failed for {vn}={{}}\", {vn});\n",
            inv = invariant_rust,
            name = pb.name,
            vn = pb.var_name,
        ));
        out.push_str("    }\n");
        out.push_str("}\n\n");

        // V3+ — proptest random sampling
        // Gate: add to Cargo.toml: [features] loom_proptest = []
        //                          [dev-dependencies] proptest = "1"
        //       then run: cargo test --features loom_proptest
        // 1024 random cases per run; bounded Int range avoids debug overflow.
        // NOTE: must be #[cfg(test)] so dev-dependencies are visible to the macro.
        out.push_str(
            "// V3+: add `proptest` to [dev-dependencies] and `loom_proptest = []` to [features]\n",
        );
        out.push_str("#[cfg(all(test, feature = \"loom_proptest\"))]\n");
        out.push_str(&format!("mod property_{fn_name}_proptest {{\n"));
        out.push_str("    use super::*;\n");
        out.push_str("    use proptest::prelude::*;\n\n");
        out.push_str("    proptest! {\n");
        out.push_str(
            "        #![proptest_config(proptest::test_runner::Config::with_cases(1024))]\n",
        );
        out.push_str(&format!(
            "        #[test]\n        fn property_{fn_name}_random({vn} in {strat}) {{\n",
            fn_name = fn_name,
            vn = pb.var_name,
            strat = strategy,
        ));
        out.push_str(&format!(
            "            prop_assert!({inv}, \"property '{name}' failed for {vn}={{}}\", {vn});\n",
            inv = invariant_rust,
            name = pb.name,
            vn = pb.var_name,
        ));
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n");

        out
    }
}

/// Translate a Loom predicate expression string to a valid Rust boolean expression.
///
/// Transformations applied:
/// - standalone `=`  → `==`  (preserves `<=`, `>=`, `!=`, `==`)
/// - ` and `         → ` && `
/// - ` or `          → ` || `
/// - `not `          → `!`
/// - `implies`       → `|| !`
pub(super) fn loom_predicate_to_rust(expr: &str) -> String {
    let mut result = String::with_capacity(expr.len() + 8);
    let chars: Vec<char> = expr.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c == '=' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < n { Some(chars[i + 1]) } else { None };
            if matches!(prev, Some('<') | Some('>') | Some('!') | Some('='))
                || matches!(next, Some('='))
            {
                result.push(c);
            } else {
                result.push_str("==");
            }
        } else {
            result.push(c);
        }
        i += 1;
    }
    let result = result.replace(" and ", " && ");
    let result = result.replace(" or ", " || ");
    let result = result.replace("not ", "!");
    let result = result.replace("implies", "|| !");
    result
}

/// Translate a Loom invariant string to a Rust boolean expression.
fn property_invariant_to_rust(invariant: &str, var_name: &str) -> String {
    let _ = var_name;
    loom_predicate_to_rust(invariant)
}

/// Map a Loom type name to (rust_type, comma-separated edge case literals).
///
/// Edge cases are chosen to be safe for arithmetic operations in debug builds.
/// `i64::MIN` is excluded because operations like `n * n` overflow in debug mode.
/// For exhaustive numeric testing, the proptest block (loom_proptest feature) uses
/// a bounded range strategy.
fn property_edge_cases(loom_type: &str) -> (&'static str, &'static str) {
    match loom_type {
        "Int" | "Integer" => ("i64", "-1000, -1, 0, 1, 1000"),
        "Float" => ("f64", "-1000.0, -1.0, 0.0, 1.0, 1000.0"),
        "Bool" => ("bool", "false, true"),
        _ => ("i64", "0, 1, -1"),
    }
}

/// Map a Loom type name to a proptest strategy expression.
///
/// Int uses a bounded range (-1_000_000..=1_000_000) to avoid arithmetic overflow
/// in debug builds. Float uses NORMAL | ZERO to exclude NaN/Inf.
fn property_proptest_strategy(loom_type: &str, rust_type: &'static str) -> String {
    match loom_type {
        "Int" | "Integer" => "(-1_000_000_i64..=1_000_000_i64)".to_string(),
        "Float" => "proptest::num::f64::NORMAL | proptest::num::f64::ZERO".to_string(),
        "Bool" => "bool".to_string(),
        _ => rust_type.to_string(),
    }
}

impl RustEmitter {
    /// Emit a `pub trait` for an `interface:` block.
    pub(super) fn emit_interface_trait(&self, iface: &InterfaceDef) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "/// Auto-generated trait for the `{}` interface.\n",
            iface.name
        ));
        out.push_str(&format!("pub trait {} {{\n", iface.name));
        for (method_name, sig) in &iface.methods {
            let params: Vec<String> = sig
                .params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                .collect();
            let ret = self.emit_type_expr(&sig.return_type);
            out.push_str(&format!(
                "    fn {}({}) -> {};\n",
                method_name,
                params.join(", "),
                ret
            ));
        }
        out.push_str("}\n");
        out
    }

    /// Emit an `impl InterfaceName for ModuleImpl { fn method(...) { ... } }` block.
    pub(super) fn emit_implements_block(
        &self,
        module_name: &str,
        iface_name: &str,
        iface: &InterfaceDef,
        items: &[Item],
    ) -> String {
        let mut out = String::new();
        let impl_struct = format!("{}Impl", module_name);
        out.push_str(&format!("pub struct {};\n", impl_struct));
        out.push_str(&format!("impl {} for {} {{\n", iface_name, impl_struct));
        for (method_name, sig) in &iface.methods {
            let ret = self.emit_type_expr(&sig.return_type);
            if let Some(Item::Fn(fd)) = items
                .iter()
                .find(|i| matches!(i, Item::Fn(fd) if fd.name == *method_name))
            {
                let params: Vec<String> = fd
                    .type_sig
                    .params
                    .iter()
                    .zip(self.fn_param_names(fd).into_iter())
                    .map(|(ty, name)| format!("{}: {}", name, self.emit_type_expr(ty)))
                    .collect();
                let body_exprs: Vec<String> = fd.body.iter().map(|e| self.emit_expr(e)).collect();
                let body = if body_exprs.is_empty() {
                    "        todo\x21()".to_string()
                } else {
                    body_exprs
                        .iter()
                        .enumerate()
                        .map(|(i, e)| {
                            if i + 1 == body_exprs.len() {
                                format!("        {}", e)
                            } else {
                                format!("        {};", e)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                out.push_str(&format!(
                    "    fn {}({}) -> {} {{\n{}\n    }}\n",
                    method_name,
                    params.join(", "),
                    ret,
                    body
                ));
            } else {
                let params: Vec<String> = sig
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                    .collect();
                out.push_str(&format!(
                    "    fn {}({}) -> {} {{\n        todo\x21(\"not implemented\")\n    }}\n",
                    method_name,
                    params.join(", "),
                    ret
                ));
            }
        }
        out.push_str("}\n");
        out
    }

    pub(super) fn fn_param_names(&self, fd: &FnDef) -> Vec<String> {
        collect_body_param_names(fd, fd.type_sig.params.len())
    }

    /// Emit the three derived artifacts for a `usecase:` block:
    /// 1. A doc comment with actor + trigger (documentation).
    /// 2. Hoare-style `require:`/`ensure:` comment block (implementation contract).
    /// 3. One `#[test]` stub per acceptance criterion (test stubs).
    pub(super) fn emit_usecase(&self, uc: &UseCaseBlock) -> String {
        let mut out = String::new();

        out.push_str(&format!("// usecase: {} — Actor: {}\n", uc.name, uc.actor));
        if !uc.trigger.is_empty() {
            out.push_str(&format!("// trigger: {}\n", uc.trigger));
        }

        out.push_str(&format!("// Derived contracts from usecase {}:\n", uc.name));
        if !uc.precondition.is_empty() {
            out.push_str(&format!("// require: {}\n", uc.precondition));
        }
        if !uc.postcondition.is_empty() {
            out.push_str(&format!("// ensure: {}\n", uc.postcondition));
        }

        if !uc.acceptance.is_empty() {
            let mod_name = format!("uc_{}_tests", to_snake_case(&uc.name));
            out.push_str("#[cfg(test)]\n");
            out.push_str(&format!("mod {} {{\n", mod_name));
            for criterion in &uc.acceptance {
                let fn_name = format!(
                    "uc_{}_{}",
                    to_snake_case(&uc.name),
                    criterion.name.replace('-', "_")
                );
                out.push_str("    #[test]\n");
                out.push_str(&format!(
                    "    #[doc = \"UC: {} - {}\"]\n",
                    uc.name, criterion.name
                ));
                out.push_str(&format!("    #[doc = \"{}\"]\n", criterion.description));
                out.push_str(&format!("    fn {}() {{\n", fn_name));
                out.push_str(&format!(
                    "        todo\x21(\"UC: {} - {}\")\n",
                    uc.name, criterion.name
                ));
                out.push_str("    }\n");
            }
            out.push_str("}\n");
        }

        out
    }

    /// Emit `#[cfg(test)] mod tests { #[test] fn name() { body } }`.
    pub(super) fn emit_test_mod(&self, test_defs: &[TestDef]) -> String {
        let mut out = String::new();
        out.push_str("#[cfg(test)]\n");
        out.push_str("mod tests {\n");
        out.push_str("    use super::*;\n");
        for td in test_defs {
            out.push('\n');
            let fn_name = td.name.replace('-', "_").to_lowercase();
            let body_src = self.emit_expr(&td.body);
            // If the test body references function calls (domain fixtures not yet defined),
            // emit as an #[ignore] stub with the spec preserved as a comment.
            let needs_fixtures = body_src.contains('(') && body_src.contains(')');
            if needs_fixtures {
                out.push_str("    #[test]\n");
                out.push_str("    #[ignore = \"stub — provide domain fixtures\"]\n");
                out.push_str(&format!("    fn {}() {{\n", fn_name));
                out.push_str(&format!("        // spec: {};\n", body_src));
                out.push_str("        todo!(\"implement test fixtures\");\n");
                out.push_str("    }\n");
            } else {
                out.push_str("    #[test]\n");
                out.push_str(&format!("    fn {}() {{\n", fn_name));
                out.push_str(&format!("        {};\n", body_src));
                out.push_str("    }\n");
            }
        }
        out.push_str("}\n");
        out
    }

    /// Emit `#[cfg(debug_assertions)] fn _check_invariants()` with invariants as spec comments.
    /// Invariants reference domain variables (struct fields) that are not in scope in a
    /// standalone free function. Emit as comments so the generated Rust compiles without
    /// domain fixtures, while preserving the specification for documentation and tooling.
    pub(super) fn emit_check_invariants(&self, invariants: &[Invariant]) -> String {
        let mut out = String::new();
        out.push_str("#[cfg(debug_assertions)]\n");
        out.push_str("pub fn _check_invariants() {\n");
        for inv in invariants {
            let cond = self.emit_expr(&inv.condition);
            out.push_str(&format!(
                "    // LOOM[invariant] '{}': {}\n",
                inv.name, cond
            ));
        }
        out.push_str("}\n");
        out
    }

    /// Emit a `#[derive(Debug)] pub struct <Module>Context` for `requires:` deps.
    pub(super) fn emit_context_struct(&self, module_name: &str, requires: &Requires) -> String {
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
    pub(super) fn emit_fn_def_with_context(
        &self,
        fd: &FnDef,
        module_name: &str,
        module_has_requires: bool,
    ) -> String {
        let inject_ctx = module_has_requires && !fd.with_deps.is_empty();
        let mut out = self.emit_fn_def_inner(fd, if inject_ctx { Some(module_name) } else { None });
        // Emit annotation contracts: Kani harnesses, stochastic structs, distribution samplers,
        // separation logic audit comments, timing safety hints, etc.
        let contracts = self.emit_fn_contracts(fd);
        if !contracts.is_empty() {
            out.push('\n');
            out.push_str(&contracts);
        }
        out
    }

    /// Emit a `pub trait` for a `provides:` block.
    pub(super) fn emit_provides_trait(&self, module_name: &str, provides: &Provides) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "/// Auto-generated trait for the `{}` provides interface.\n",
            module_name
        ));
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

    /// Emit a refinement predicate expression, replacing `self` with `value`.
    pub(super) fn emit_predicate(&self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(name) if name == "self" => "value".to_string(),
            Expr::BinOp {
                op, left, right, ..
            } => {
                let l = self.emit_predicate(left);
                let r = self.emit_predicate(right);
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
                format!("({} {} {})", l, op_str, r)
            }
            _ => self.emit_expr(expr),
        }
    }

    pub(super) fn emit_fn_def(&self, fd: &FnDef) -> String {
        let mut out = self.emit_fn_def_inner(fd, None);
        // Implicit disciplines: emit all structural/mathematical patterns from fn annotations
        let disciplines = self.emit_fn_contracts(fd);
        if !disciplines.is_empty() {
            out.push('\n');
            out.push_str(&disciplines);
        }
        out
    }

    pub(super) fn emit_fn_def_inner(&self, fd: &FnDef, ctx_module: Option<&str>) -> String {
        let is_effectful = matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _));

        let mut out = String::new();

        if let Some(desc) = &fd.describe {
            for line in desc.lines() {
                out.push_str(&format!("/// {}\n", line));
            }
        }

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

        for (eff, tier) in &fd.effect_tiers {
            let tier_str = match tier {
                ConsequenceTier::Pure => "pure",
                ConsequenceTier::Reversible => "reversible",
                ConsequenceTier::Irreversible => "irreversible",
            };
            out.push_str(&format!("// effect-tier: {} -> {}\n", eff, tier_str));
        }

        if let Some(sep) = &fd.separation {
            out.push_str("// separation logic:\n");
            for owned in &sep.owns {
                out.push_str(&format!("//   owns: {}\n", owned));
            }
            for (a, b) in &sep.disjoint {
                out.push_str(&format!("//   disjoint: {} * {}\n", a, b));
            }
            for f in &sep.frame {
                out.push_str(&format!("//   frame: {}\n", f));
            }
            if let Some(proof) = &sep.proof {
                out.push_str(&format!("//   proof: {}\n", proof));
            }
        }

        if let Some(gradual) = &fd.gradual {
            out.push_str("// gradual typing:\n");
            if let Some(it) = &gradual.input_type {
                out.push_str(&format!("//   input_type: {}\n", it));
            }
            if let Some(b) = &gradual.boundary {
                out.push_str(&format!("//   boundary: {}\n", b));
            }
            if let Some(ot) = &gradual.output_type {
                out.push_str(&format!("//   output_type: {}\n", ot));
            }
            if let Some(cf) = &gradual.on_cast_failure {
                out.push_str(&format!("//   on_cast_failure: {}\n", cf));
            }
            if let Some(bl) = &gradual.blame {
                out.push_str(&format!("//   blame: {}\n", bl));
            }
        }

        if let Some(dist) = &fd.distribution {
            out.push_str("// distribution:\n");
            out.push_str(&format!("//   model: {}\n", dist.model));
            if let Some(m) = &dist.mean {
                out.push_str(&format!("//   mean: {}\n", m));
            }
            if let Some(v) = &dist.variance {
                out.push_str(&format!("//   variance: {}\n", v));
            }
            if let Some(c) = &dist.convergence {
                out.push_str(&format!("//   convergence: {}\n", c));
            }
        }

        if let Some(ts) = &fd.timing_safety {
            out.push_str("// timing_safety:\n");
            out.push_str(&format!("//   constant_time: {}\n", ts.constant_time));
            if let Some(lb) = &ts.leaks_bits {
                out.push_str(&format!("//   leaks_bits: {}\n", lb));
            }
            if let Some(m) = &ts.method {
                out.push_str(&format!("//   method: {}\n", m));
            }
        }

        if let Some(t) = &fd.termination {
            out.push_str(&format!("// termination: {}\n", t));
        }

        for proof in &fd.proofs {
            out.push_str(&format!("// proof: {}\n", proof.strategy));
        }

        if let Some(dg) = &fd.degenerate {
            self.emit_degenerate_fallback(&fd.name, dg, &mut out);
        }

        let mut params: Vec<String> = Vec::new();

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
                    format!(
                        "Result<{}, Box<dyn std::error::Error>>",
                        self.emit_type_expr(inner)
                    )
                }
                _ => self.emit_type_expr(&fd.type_sig.return_type),
            }
        } else {
            self.emit_type_expr(&fd.type_sig.return_type)
        };

        let mut body_lines: Vec<String> = Vec::new();

        // V7: emit audit trail before each contract assertion.
        for contract in &fd.requires {
            let raw = self.emit_expr(&contract.expr);
            let expr_text = loom_predicate_to_rust(&raw);
            let (macro_name, note) = if self.release_contracts {
                ("assert", "all builds")
            } else {
                ("debug_assert", "debug builds only")
            };
            body_lines.push(format!(
                "    // LOOM[require]: {} — {}! (runtime, {})",
                expr_text, macro_name, note
            ));
            body_lines.push(format!(
                "    {}!({}, \"precondition violated: {}\");",
                macro_name,
                expr_text,
                expr_text.replace('"', "\\\""),
            ));
        }

        let has_ensures = !fd.ensures.is_empty();
        let body_count = fd.body.len();
        // Only use the _loom_result binding pattern when at least one ensure
        // condition actually references "result" — otherwise it's a type-name
        // expression that can't be bound as a value.
        let last_is_stub =
            body_count > 0 && is_type_name_stub(&self.emit_expr(&fd.body[body_count - 1]));
        let needs_result_binding = has_ensures
            && !last_is_stub
            && fd
                .ensures
                .iter()
                .any(|c| self.emit_expr(&c.expr).contains("result"));
        if has_ensures && body_count > 0 {
            for expr in &fd.body[..body_count - 1] {
                body_lines.push(format!("    {};", self.emit_expr(expr)));
            }
            let last = &fd.body[body_count - 1];
            if needs_result_binding {
                body_lines.push(format!("    let _loom_result = {};", self.emit_expr(last)));
            }
            for contract in &fd.ensures {
                let raw = self.emit_expr(&contract.expr);
                let raw = loom_predicate_to_rust(&raw);
                let cond = if needs_result_binding {
                    raw.replace("result", "_loom_result")
                } else {
                    raw.clone()
                };
                if last_is_stub {
                    // Body is a stub — emit ensure as a spec comment only (no assert)
                    body_lines.push(format!(
                        "    // LOOM[ensure]: {} — implement body to activate",
                        cond
                    ));
                } else {
                    let (macro_name, note) = if self.release_contracts {
                        ("assert", "all builds")
                    } else {
                        ("debug_assert", "debug builds only")
                    };
                    body_lines.push(format!(
                        "    // LOOM[ensure]: {} — checked on return value via _loom_result ({})",
                        cond, note
                    ));
                    body_lines.push(format!(
                        "    {}!({cond}, \"ensure: {}\");",
                        macro_name,
                        cond.replace('"', "\\\""),
                    ));
                }
            }
            if needs_result_binding {
                body_lines.push("    _loom_result".to_string());
            } else {
                let last_text = self.emit_expr(last);
                if is_type_name_stub(&last_text) {
                    body_lines.push(format!(
                        "    todo\x21(\"stub body — implement return value of type {}\")",
                        last_text
                    ));
                } else {
                    body_lines.push(format!("    {}", last_text));
                }
            }
        } else {
            for (i, expr) in fd.body.iter().enumerate() {
                let text = self.emit_expr(expr);
                if i + 1 == body_count {
                    if is_type_name_stub(&text) {
                        body_lines.push(format!(
                            "    todo\x21(\"stub body — implement return value of type {}\")",
                            text
                        ));
                    } else {
                        body_lines.push(format!("    {}", text));
                    }
                } else {
                    body_lines.push(format!("    {};", text));
                }
            }
        }

        if body_lines.is_empty() {
            body_lines
                .push("    todo\x21(\"Phase 1 stub — body not yet implemented\")".to_string());
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
}

// ── Free helpers used only by this module ─────────────────────────────────────

/// Returns a human-readable description for known algebraic annotation keys.
fn algebraic_annotation_desc(key: &str) -> Option<&'static str> {
    match key {
        "idempotent" => Some("safe to retry"),
        "commutative" => Some("argument order does not matter"),
        "associative" => Some("grouping does not matter"),
        "at-most-once" => Some("must not be called more than once"),
        "exactly-once" => Some("must be called exactly once"),
        "pure" => Some("no side effects"),
        "monotonic" => Some("output only increases"),
        _ => None,
    }
}

/// Identifiers that look like free variables but are actually language keywords
/// or built-in macros and must not be used as parameter names.
const PARAM_NAME_BUILTINS: &[&str] = &["todo", "panic", "unreachable", "unimplemented"];

/// Collect free variable names from a function body in first-appearance order.
///
/// Returns at most `max_params` names; falls back to `arg{i}` for any slot
/// that couldn't be filled from the body.
fn collect_body_param_names(fd: &FnDef, max_params: usize) -> Vec<String> {
    use std::collections::HashSet;

    let mut let_bound: HashSet<String> = HashSet::new();
    for expr in &fd.body {
        collect_let_names(expr, &mut let_bound);
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();

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
        Expr::InlineRust(_) => {}
        Expr::As(inner, _) => collect_let_names(inner, out),
        Expr::Lambda { body, .. } => collect_let_names(body, out),
        Expr::ForIn { iter, body, .. } => {
            collect_let_names(iter, out);
            collect_let_names(body, out);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| collect_let_names(e, out)),
        Expr::Try(inner, _) => collect_let_names(inner, out),
        Expr::Index(collection, index, _) => {
            collect_let_names(collection, out);
            collect_let_names(index, out);
        }
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
                && name
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
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
        Expr::InlineRust(_) => {}
        Expr::As(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
        Expr::Lambda { params, body, .. } => {
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
        Expr::Tuple(elems, _) => elems
            .iter()
            .for_each(|e| scan_free_idents(e, let_bound, seen, ordered)),
        Expr::Try(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
        Expr::Index(collection, index, _) => {
            scan_free_idents(collection, let_bound, seen, ordered);
            scan_free_idents(index, let_bound, seen, ordered);
        }
    }
}
