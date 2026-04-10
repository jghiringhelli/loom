//! Expression, literal, pattern, and type-expression emitters.

use super::RustEmitter;
use crate::ast::*;
use crate::checker::units::{capitalize, collect_unit_labels, extract_unit};

impl RustEmitter {
    /// Recursively lower a Loom `TypeExpr` to a Rust type string.
    pub fn emit_type_expr(&self, ty: &TypeExpr) -> String {
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
                    "Map" if ps.len() == 2 => format!("HashMap<{}, {}>", ps[0], ps[1]),
                    "Set" if ps.len() == 1 => format!("HashSet<{}>", ps[0]),
                    _ => format!("{}<{}>", name, ps.join(", ")),
                }
            }
            TypeExpr::Effect(_, inner) => {
                format!(
                    "Result<{}, Box<dyn std::error::Error>>",
                    self.emit_type_expr(inner)
                )
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
            TypeExpr::Dynamic => "Box<dyn std::any::Any>".to_string(),
            // TypeVar should be resolved before codegen; emit a placeholder if it leaks.
            TypeExpr::TypeVar(id) => format!("/* infer:?{} */", id),
            // Tensor<rank, shape, unit> — emit as nested Vec<> (rank dimensions of unit type).
            TypeExpr::Tensor { rank, unit, .. } => {
                let inner = self.emit_type_expr(unit);
                (0..*rank).fold(inner, |acc, _| format!("Vec<{}>", acc))
            }
        }
    }

    /// Map Loom primitive type names to Rust equivalents.
    pub(super) fn map_base_type(&self, name: &str) -> String {
        match name {
            "Int" => "i64".to_string(),
            "Float" => "f64".to_string(),
            "String" | "Str" => "String".to_string(),
            "Bool" => "bool".to_string(),
            "Unit" => "()".to_string(),
            other => other.to_string(),
        }
    }

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
            // Allow contracts to compare against f64 literals (e.g. `amount > 0.0`)
            out.push_str(&format!(
                "impl PartialEq<f64> for {0} {{ fn eq(&self, rhs: &f64) -> bool {{ self.0 == *rhs }} }}\n",
                tn
            ));
            out.push_str(&format!(
                "impl PartialOrd<f64> for {0} {{ fn partial_cmp(&self, rhs: &f64) -> Option<std::cmp::Ordering> {{ self.0.partial_cmp(rhs) }} }}\n",
                tn
            ));
            out.push('\n');
        }
        out
    }

    /// Recursively lower a Loom `Expr` to a Rust expression string.
    pub(super) fn emit_expr(&self, expr: &Expr) -> String {
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
            Expr::BinOp {
                op, left, right, ..
            } => {
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
            Expr::ForIn {
                var, iter, body, ..
            } => {
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

    pub(super) fn emit_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
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

    pub(super) fn emit_pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Variant(name, sub_pats) => {
                if sub_pats.is_empty() {
                    name.clone()
                } else {
                    let subs: Vec<String> = sub_pats.iter().map(|p| self.emit_pattern(p)).collect();
                    format!("{}({})", name, subs.join(", "))
                }
            }
            Pattern::Ident(name) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(lit) => self.emit_literal(lit),
        }
    }
}
