//! TypeScript source emitter — translates a Loom [`Module`] AST into valid TypeScript.
//!
//! # Mapping summary
//!
//! | Loom construct | Emitted TypeScript |
//! |---|---|
//! | `module M … end` | `export namespace M { … }` |
//! | `type Point = x: Float, y: Float end` | `export interface Point { x: number; y: number; }` |
//! | `enum E = \| A \| B of T end` | `export type E = "A" \| { tag: "B"; value: T }` |
//! | `type Email = String where pred` | `export type Email = string & { readonly _brand: "Email" }` |
//! | `fn f :: A -> Effect<[E], B>` | `export async function f(a: A): Promise<B>` |
//! | `fn f :: A -> B` (pure) | `export function f(a: A): B` |
//! | `let x = e` | `const x = e` |
//! | `match x \| Arm -> body end` | `switch`-style if-else chain |
//! | `require: cond` | `if (!(cond)) throw new Error(…)` |
//! | `ensure: cond` | `if (!(cond)) throw new Error(…)` (after result capture) |
//! | `interface Greeter fn greet … end` | `export interface Greeter { greet(…): T; }` |
//! | `implements Greeter` | `export class MImpl implements Greeter { … }` |
//! | `import ModName` | `import * as ModName from "./mod_name"` |
//! | `describe:` / `@annotations` | JSDoc comments |

use crate::ast::*;

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Stateless TypeScript source emitter.
///
/// # Examples
///
/// ```rust,ignore
/// let ts_src = TypeScriptEmitter::new().emit(&module);
/// ```
pub struct TypeScriptEmitter;

impl TypeScriptEmitter {
    pub fn new() -> Self {
        TypeScriptEmitter
    }

    /// Emit a complete TypeScript source file from a [`Module`].
    pub fn emit(&self, module: &Module) -> String {
        let mut out = String::with_capacity(4096);

        // File-level JSDoc from module describe: / @annotations.
        if module.describe.is_some() || !module.annotations.is_empty() {
            out.push_str("/**\n");
            if let Some(desc) = &module.describe {
                for line in desc.lines() {
                    out.push_str(&format!(" * {}\n", line));
                }
            }
            for ann in &module.annotations {
                out.push_str(&format!(" * @{} {}\n", ann.key, ann.value));
            }
            out.push_str(" */\n");
        }

        // ES module imports for each `import ModName`.
        for imp in &module.imports {
            out.push_str(&format!(
                "import * as {} from \"./{}\"\n",
                imp,
                to_kebab_case(imp)
            ));
        }
        if !module.imports.is_empty() {
            out.push('\n');
        }

        // Namespace wrapper.
        out.push_str(&format!("export namespace {} {{\n", module.name));

        let mut body = String::new();

        // Interface definitions → TS interface.
        for iface in &module.interface_defs {
            body.push('\n');
            body.push_str(&self.emit_interface_def(iface));
        }

        // implements → TS class implementing the interface.
        for iface_name in &module.implements {
            if let Some(iface) = module.interface_defs.iter().find(|i| &i.name == iface_name) {
                body.push('\n');
                body.push_str(&self.emit_implements_class(&module.name, iface_name, iface, &module.items));
            }
        }

        // Items.
        for item in &module.items {
            body.push('\n');
            let item_src = match item {
                Item::Type(td) => self.emit_type_def(td),
                Item::Enum(ed) => self.emit_enum_def(ed),
                Item::Fn(fd) => self.emit_fn_def(fd),
                Item::RefinedType(rt) => self.emit_refined_type(rt),
            };
            for line in item_src.lines() {
                if line.is_empty() {
                    body.push('\n');
                } else {
                    body.push_str("  ");
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        // Indented interface/implements blocks.
        for line in body.lines() {
            if line.trim().is_empty() {
                out.push('\n');
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }

        out.push_str("}\n");
        out
    }

    // ── Type definitions ──────────────────────────────────────────────────

    fn emit_type_def(&self, td: &TypeDef) -> String {
        let fields: Vec<String> = td.fields.iter()
            .map(|(name, ty)| format!("  {}: {};", name, self.emit_type_expr(ty)))
            .collect();
        format!("export interface {} {{\n{}\n}}", td.name, fields.join("\n"))
    }

    fn emit_enum_def(&self, ed: &EnumDef) -> String {
        let variants: Vec<String> = ed.variants.iter().map(|v| {
            match &v.payload {
                None => format!("\"{}\"", v.name),
                Some(ty) => format!("{{ tag: \"{}\"; value: {} }}", v.name, self.emit_type_expr(ty)),
            }
        }).collect();
        format!("export type {} =\n  | {};", ed.name, variants.join("\n  | "))
    }

    fn emit_refined_type(&self, rt: &RefinedType) -> String {
        let inner = self.emit_type_expr(&rt.base_type);
        format!(
            "export type {} = {} & {{ readonly _brand: \"{}\" }}\n\
             export function make{}(value: {}): {} {{\n  \
             if (!({condition})) throw new Error(`Refined type {name} precondition failed`);\n  \
             return value as {};\n}}",
            rt.name, inner, rt.name,
            rt.name, inner, rt.name,
            inner,
            condition = self.emit_expr(&rt.predicate),
            name = rt.name,
        )
    }

    // ── Interface and implements ───────────────────────────────────────────

    fn emit_interface_def(&self, iface: &InterfaceDef) -> String {
        let mut out = String::new();
        out.push_str(&format!("export interface {} {{\n", iface.name));
        for (method_name, sig) in &iface.methods {
            let params: Vec<String> = sig.params.iter().enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                .collect();
            let ret = self.emit_type_expr(&sig.return_type);
            out.push_str(&format!("  {}({}): {};\n", method_name, params.join(", "), ret));
        }
        out.push('}');
        out
    }

    fn emit_implements_class(
        &self,
        module_name: &str,
        iface_name: &str,
        iface: &InterfaceDef,
        items: &[Item],
    ) -> String {
        let mut out = String::new();
        let class_name = format!("{}Impl", module_name);
        out.push_str(&format!("export class {} implements {} {{\n", class_name, iface_name));
        for (method_name, sig) in &iface.methods {
            let ret = self.emit_type_expr(&sig.return_type);
            if let Some(Item::Fn(fd)) = items.iter().find(|i| matches!(i, Item::Fn(fd) if fd.name == *method_name)) {
                let params: Vec<String> = fd.type_sig.params.iter()
                    .zip(ts_param_names(fd).into_iter())
                    .map(|(ty, name)| format!("{}: {}", name, self.emit_type_expr(ty)))
                    .collect();
                let body = self.emit_fn_body(fd);
                out.push_str(&format!("  {}({}): {} {{\n{}\n  }}\n",
                    method_name, params.join(", "), ret, body));
            } else {
                let params: Vec<String> = sig.params.iter().enumerate()
                    .map(|(i, ty)| format!("arg{}: {}", i, self.emit_type_expr(ty)))
                    .collect();
                out.push_str(&format!("  {}({}): {} {{\n    throw new Error(\"not implemented\");\n  }}\n",
                    method_name, params.join(", "), ret));
            }
        }
        out.push('}');
        out
    }

    // ── Function definitions ──────────────────────────────────────────────

    fn emit_fn_def(&self, fd: &FnDef) -> String {
        let is_async = matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _));

        let mut out = String::new();

        // JSDoc.
        let has_jsdoc = fd.describe.is_some() || !fd.annotations.is_empty() || !fd.effect_tiers.is_empty();
        if has_jsdoc {
            out.push_str("/**\n");
            if let Some(desc) = &fd.describe {
                for line in desc.lines() {
                    out.push_str(&format!(" * {}\n", line));
                }
            }
            for ann in &fd.annotations {
                out.push_str(&format!(" * @{} {}\n", ann.key, ann.value));
            }
            for (eff, tier) in &fd.effect_tiers {
                let tier_str = match tier {
                    ConsequenceTier::Pure         => "pure",
                    ConsequenceTier::Reversible   => "reversible",
                    ConsequenceTier::Irreversible => "irreversible",
                };
                out.push_str(&format!(" * @effect-tier {} {}\n", eff, tier_str));
            }
            out.push_str(" */\n");
        }

        let params: Vec<String> = fd.type_sig.params.iter()
            .zip(ts_param_names(fd).into_iter())
            .map(|(ty, name)| format!("{}: {}", name, self.emit_type_expr(ty)))
            .collect();

        let ret = match fd.type_sig.return_type.as_ref() {
            TypeExpr::Effect(_, inner) => format!("Promise<{}>", self.emit_type_expr(inner)),
            ty => self.emit_type_expr(ty),
        };

        let type_params = if fd.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", fd.type_params.join(", "))
        };

        let async_kw = if is_async { "async " } else { "" };
        out.push_str(&format!(
            "export {}function {}{}({}): {} {{\n",
            async_kw, fd.name, type_params, params.join(", "), ret
        ));

        out.push_str(&self.emit_fn_body(fd));
        out.push_str("\n}");
        out
    }

    fn emit_fn_body(&self, fd: &FnDef) -> String {
        let mut lines: Vec<String> = Vec::new();

        // require: → if (!cond) throw
        for c in &fd.requires {
            let cond = self.emit_expr(&c.expr);
            lines.push(format!(
                "  if (!({cond})) throw new Error(\"precondition violated: {esc}\");",
                cond = cond,
                esc = cond.replace('"', "\\\""),
            ));
        }

        let has_ensures = !fd.ensures.is_empty();
        let body_count = fd.body.len();

        if has_ensures && body_count > 0 {
            for expr in &fd.body[..body_count - 1] {
                lines.push(format!("  {};", self.emit_expr(expr)));
            }
            let last = &fd.body[body_count - 1];
            lines.push(format!("  const _loomResult = {};", self.emit_expr(last)));
            for c in &fd.ensures {
                let raw = self.emit_expr(&c.expr);
                let cond = raw.replace("result", "_loomResult");
                lines.push(format!(
                    "  if (!({cond})) throw new Error(\"ensure: {esc}\");",
                    cond = cond,
                    esc = cond.replace('"', "\\\""),
                ));
            }
            lines.push("  return _loomResult;".to_string());
        } else if !fd.body.is_empty() {
            for (i, expr) in fd.body.iter().enumerate() {
                if i + 1 == body_count {
                    lines.push(format!("  return {};", self.emit_expr(expr)));
                } else {
                    lines.push(format!("  {};", self.emit_expr(expr)));
                }
            }
        } else {
            lines.push("  throw new Error(\"not implemented\");".to_string());
        }

        lines.join("\n")
    }

    // ── Type expressions ──────────────────────────────────────────────────

    fn emit_type_expr(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Base(name) => self.map_base_type(name).to_string(),
            TypeExpr::Generic(name, params) => {
                let ps: Vec<String> = params.iter().map(|p| self.emit_type_expr(p)).collect();
                match name.as_str() {
                    "List" if ps.len() == 1 => format!("{}[]", ps[0]),
                    "Map"  if ps.len() == 2 => format!("Map<{}, {}>", ps[0], ps[1]),
                    "Set"  if ps.len() == 1 => format!("Set<{}>", ps[0]),
                    _ => format!("{}<{}>", name, ps.join(", ")),
                }
            }
            TypeExpr::Effect(_, inner) => format!("Promise<{}>", self.emit_type_expr(inner)),
            TypeExpr::Option(inner) => format!("{} | null", self.emit_type_expr(inner)),
            TypeExpr::Result(ok, err) => format!(
                "{{ ok: {} }} | {{ err: {} }}",
                self.emit_type_expr(ok),
                self.emit_type_expr(err)
            ),
            TypeExpr::Tuple(elems) => {
                let es: Vec<String> = elems.iter().map(|e| self.emit_type_expr(e)).collect();
                format!("[{}]", es.join(", "))
            }
            TypeExpr::TypeVar(id) => format!("/* infer:?{} */", id),
        }
    }

    fn map_base_type(&self, name: &str) -> &'static str {
        match name {
            "Int" | "Float" => "number",
            "String" | "Str" => "string",
            "Bool" => "boolean",
            "Unit" => "void",
            _ => "unknown",
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn emit_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Let { name, value, .. } => {
                format!("const {} = {}", name, self.emit_expr(value))
            }
            Expr::Literal(lit) => self.emit_literal(lit),
            Expr::Ident(name) if name == "todo" => "(() => { throw new Error(\"todo\"); })()".to_string(),
            Expr::Ident(name) => name.clone(),
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(name) = func.as_ref() {
                    match (name.as_str(), args.len()) {
                        ("map", 2) => return format!(
                            "{}.map({})",
                            self.emit_expr(&args[0]),
                            self.emit_expr(&args[1])
                        ),
                        ("filter", 2) => return format!(
                            "{}.filter({})",
                            self.emit_expr(&args[0]),
                            self.emit_expr(&args[1])
                        ),
                        ("fold", 3) => return format!(
                            "{}.reduce({}, {})",
                            self.emit_expr(&args[0]),
                            self.emit_expr(&args[2]),
                            self.emit_expr(&args[1])
                        ),
                        _ => {}
                    }
                }
                let f = self.emit_expr(func);
                let as_str: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{}({})", f, as_str.join(", "))
            }
            Expr::Pipe { left, right, .. } => {
                let l = self.emit_expr(left);
                let r = self.emit_expr(right);
                format!("((_pipe) => {}(_pipe))({})", r, l)
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
                    BinOpKind::Eq => "===",
                    BinOpKind::Ne => "!==",
                    BinOpKind::Lt => "<",
                    BinOpKind::Le => "<=",
                    BinOpKind::Gt => ">",
                    BinOpKind::Ge => ">=",
                    BinOpKind::And => "&&",
                    BinOpKind::Or => "||",
                };
                format!("({} {} {})", self.emit_expr(left), op_str, self.emit_expr(right))
            }
            Expr::InlineRust(code) => format!("/* inline */ {}", code),
            Expr::As(inner, _ty) => self.emit_expr(inner),
            Expr::Lambda { params, body, .. } => {
                let param_strs: Vec<String> = params.iter()
                    .map(|(name, ty)| {
                        if let Some(t) = ty {
                            format!("{}: {}", name, self.emit_type_expr(t))
                        } else {
                            name.clone()
                        }
                    })
                    .collect();
                format!("({}) => {}", param_strs.join(", "), self.emit_expr(body))
            }
            Expr::ForIn { var, iter, body, .. } => {
                format!("for (const {} of {}) {{ {} }}",
                    var, self.emit_expr(iter), self.emit_expr(body))
            }
            Expr::Tuple(elems, _) => {
                let inner: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("[{}]", inner.join(", "))
            }
            Expr::Try(inner, _) => {
                // TypeScript doesn't have `?` — wrap in an await-or-throw helper comment.
                format!("/* try */ {}", self.emit_expr(inner))
            }
            Expr::Match { subject, arms, .. } => {
                let s = self.emit_expr(subject);
                let tmp = "_m";
                let mut chain = String::new();
                for (i, arm) in arms.iter().enumerate() {
                    let cond = self.emit_pattern_cond(tmp, &arm.pattern);
                    let guard = arm.guard.as_ref()
                        .map(|g| format!(" && ({})", self.emit_expr(g)))
                        .unwrap_or_default();
                    let body = self.emit_expr(&arm.body);
                    if i == 0 {
                        chain.push_str(&format!("(({tmp}) => {{\n  if ({cond}{guard}) return {body};\n"));
                    } else if i + 1 == arms.len() {
                        chain.push_str(&format!("  return {body};\n}})({s})"));
                    } else {
                        chain.push_str(&format!("  if ({cond}{guard}) return {body};\n"));
                    }
                }
                if arms.len() == 1 {
                    chain.push_str(&format!("  throw new Error(\"non-exhaustive match\");\n}})({s})"));
                }
                chain
            }
        }
    }

    fn emit_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') { s } else { format!("{}.0", s) }
            }
            Literal::Str(s) => format!("{:?}", s),
            Literal::Bool(b) => b.to_string(),
            Literal::Unit => "undefined".to_string(),
        }
    }

    fn emit_pattern_cond(&self, subject: &str, pat: &Pattern) -> String {
        match pat {
            Pattern::Wildcard => "true".to_string(),
            Pattern::Ident(_) => "true".to_string(),
            Pattern::Literal(lit) => format!("{} === {}", subject, self.emit_literal(lit)),
            Pattern::Variant(name, sub_pats) => {
                if sub_pats.is_empty() {
                    format!("{} === {:?}", subject, name)
                } else {
                    format!("({}.tag === {:?})", subject, name)
                }
            }
        }
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// PascalCase / camelCase → kebab-case for file names.
fn to_kebab_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

/// Collect parameter names from a FnDef body (mirrors the Rust emitter logic).
fn ts_param_names(fd: &FnDef) -> Vec<String> {
    use std::collections::HashSet;

    let max = fd.type_sig.params.len();
    let mut let_bound: HashSet<String> = HashSet::new();
    for expr in &fd.body {
        collect_let_names_ts(expr, &mut let_bound);
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();

    let all_exprs: Vec<&Expr> = fd.body.iter()
        .chain(fd.requires.iter().map(|c| &c.expr))
        .chain(fd.ensures.iter().map(|c| &c.expr))
        .collect();

    for expr in all_exprs {
        scan_free_idents_ts(expr, &let_bound, &mut seen, &mut ordered);
        if ordered.len() >= max { break; }
    }

    (0..max)
        .map(|i| ordered.get(i).cloned().unwrap_or_else(|| format!("arg{}", i)))
        .collect()
}

fn collect_let_names_ts(expr: &Expr, out: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Let { name, value, .. } => {
            out.insert(name.clone());
            collect_let_names_ts(value, out);
        }
        _ => {}
    }
}

fn scan_free_idents_ts(
    expr: &Expr,
    let_bound: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
    ordered: &mut Vec<String>,
) {
    const BUILTINS: &[&str] = &["todo", "map", "filter", "fold", "true", "false", "null"];
    match expr {
        Expr::Ident(name) => {
            if !let_bound.contains(name) && !BUILTINS.contains(&name.as_str()) && seen.insert(name.clone()) {
                ordered.push(name.clone());
            }
        }
        Expr::BinOp { left, right, .. } => {
            scan_free_idents_ts(left, let_bound, seen, ordered);
            scan_free_idents_ts(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            scan_free_idents_ts(func, let_bound, seen, ordered);
            for a in args { scan_free_idents_ts(a, let_bound, seen, ordered); }
        }
        Expr::Let { value, .. } => scan_free_idents_ts(value, let_bound, seen, ordered),
        Expr::FieldAccess { object, .. } => scan_free_idents_ts(object, let_bound, seen, ordered),
        Expr::Pipe { left, right, .. } => {
            scan_free_idents_ts(left, let_bound, seen, ordered);
            scan_free_idents_ts(right, let_bound, seen, ordered);
        }
        Expr::Lambda { body, .. } => scan_free_idents_ts(body, let_bound, seen, ordered),
        Expr::Match { subject, arms, .. } => {
            scan_free_idents_ts(subject, let_bound, seen, ordered);
            for arm in arms { scan_free_idents_ts(&arm.body, let_bound, seen, ordered); }
        }
        Expr::Tuple(elems, _) => {
            for e in elems { scan_free_idents_ts(e, let_bound, seen, ordered); }
        }
        Expr::Try(inner, _) | Expr::As(inner, _) => scan_free_idents_ts(inner, let_bound, seen, ordered),
        Expr::ForIn { iter, body, .. } => {
            scan_free_idents_ts(iter, let_bound, seen, ordered);
            scan_free_idents_ts(body, let_bound, seen, ordered);
        }
        Expr::Literal(_) | Expr::InlineRust(_) => {}
    }
}
