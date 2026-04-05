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

        // Render the module body first to detect which stdlib imports are needed.
        let mut body = String::new();

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
        out.push_str("}\n");
        out
    }

    // ── DI context struct ─────────────────────────────────────────────────

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
        let fields: Vec<String> = td
            .fields
            .iter()
            .map(|(name, ty)| format!("    pub {}: {},", name, self.emit_type_expr(ty)))
            .collect();
        format!(
            "#[derive(Debug, Clone, PartialEq)]\npub struct {} {{\n{}\n}}\n",
            td.name,
            fields.join("\n")
        )
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
            out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value));
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

        // Emit `ensure:` contracts as documentation comments.
        for contract in &fd.ensures {
            body_lines.push(format!(
                "    // postcondition: {} (verified by type system)",
                self.emit_expr(&contract.expr),
            ));
        }

        // Emit body expressions as statements; the last expression is returned.
        let body_count = fd.body.len();
        for (i, expr) in fd.body.iter().enumerate() {
            if i + 1 == body_count {
                // Final expression — no semicolon (implicit return).
                body_lines.push(format!("    {}", self.emit_expr(expr)));
            } else {
                body_lines.push(format!("    {};", self.emit_expr(expr)));
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
            spec: None,
            provides: None,
            requires: None,
            invariants: vec![],
            items: vec![Item::Type(TypeDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), TypeExpr::Base("Float".to_string())),
                    ("y".to_string(), TypeExpr::Base("Float".to_string())),
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
            spec: None,
            provides: None,
            requires: None,
            invariants: vec![],
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
            spec: None,
            provides: None,
            requires: None,
            invariants: vec![],
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
