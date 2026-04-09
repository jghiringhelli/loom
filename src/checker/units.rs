//! Units of Measure checker.
//!
//! Detects unit-annotated primitives `Float<usd>`, `Int<meters>` and enforces
//! arithmetic consistency within function bodies and across call sites.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

// ── Public helpers ─────────────────────────────────────────────────────────────

/// Extract the unit label from a unit-annotated type (`Float<usd>` → `Some("usd")`).
pub fn extract_unit(ty: &TypeExpr) -> Option<&str> {
    if let TypeExpr::Generic(name, params) = ty {
        if (name == "Float" || name == "Int") && params.len() == 1 {
            if let TypeExpr::Base(unit) = &params[0] {
                return Some(unit.as_str());
            }
        }
    }
    None
}

/// Capitalize the first letter of a label (`"usd"` → `"Usd"`).
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Collect all unique unit labels appearing in a module's type signatures.
///
/// The order is stable (first-seen, declaration order).
pub fn collect_unit_labels(module: &Module) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut labels = Vec::new();

    for item in &module.items {
        match item {
            Item::Fn(fd) => {
                for ty in &fd.type_sig.params {
                    collect_units_from_type(ty, &mut seen, &mut labels);
                }
                collect_units_from_type(&fd.type_sig.return_type, &mut seen, &mut labels);
            }
            Item::Type(td) => {
                for field in &td.fields {
                    collect_units_from_type(&field.ty, &mut seen, &mut labels);
                }
            }
            Item::Enum(ed) => {
                for v in &ed.variants {
                    if let Some(ty) = &v.payload {
                        collect_units_from_type(ty, &mut seen, &mut labels);
                    }
                }
            }
            Item::RefinedType(rt) => {
                collect_units_from_type(&rt.base_type, &mut seen, &mut labels);
            }
            _ => {}
        }
    }

    if let Some(provides) = &module.provides {
        for (_, sig) in &provides.ops {
            for ty in &sig.params {
                collect_units_from_type(ty, &mut seen, &mut labels);
            }
            collect_units_from_type(&sig.return_type, &mut seen, &mut labels);
        }
    }

    if let Some(requires) = &module.requires {
        for (_, ty) in &requires.deps {
            collect_units_from_type(ty, &mut seen, &mut labels);
        }
    }

    for iface in &module.interface_defs {
        for (_, sig) in &iface.methods {
            for ty in &sig.params {
                collect_units_from_type(ty, &mut seen, &mut labels);
            }
            collect_units_from_type(&sig.return_type, &mut seen, &mut labels);
        }
    }

    labels
}

fn collect_units_from_type(ty: &TypeExpr, seen: &mut HashSet<String>, labels: &mut Vec<String>) {
    match ty {
        TypeExpr::Generic(name, params)
            if (name == "Float" || name == "Int") && params.len() == 1 =>
        {
            if let TypeExpr::Base(unit) = &params[0] {
                if seen.insert(unit.clone()) {
                    labels.push(unit.clone());
                }
            }
        }
        TypeExpr::Generic(_, params) => {
            for p in params {
                collect_units_from_type(p, seen, labels);
            }
        }
        TypeExpr::Option(inner) => collect_units_from_type(inner, seen, labels),
        TypeExpr::Result(ok, err) => {
            collect_units_from_type(ok, seen, labels);
            collect_units_from_type(err, seen, labels);
        }
        TypeExpr::Tuple(elems) => {
            for e in elems {
                collect_units_from_type(e, seen, labels);
            }
        }
        TypeExpr::Effect(_, inner) => collect_units_from_type(inner, seen, labels),
        _ => {}
    }
}

// ── Units checker ──────────────────────────────────────────────────────────────

/// Phase-N units of measure checker.
///
/// Walks all `FnDef` bodies and enforces that `Add`/`Sub` operations only
/// combine values that carry the same unit label.  The checker is intentionally
/// *lenient*: it only reports an error when it can conclusively determine that
/// two different unit labels are being combined.
pub struct UnitsChecker;

impl UnitsChecker {
    /// Create a new `UnitsChecker`.
    pub fn new() -> Self {
        UnitsChecker
    }

    /// Check `module` for unit-mismatch errors.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut errors);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        // Build initial env from the declared parameter types.
        let mut env: HashMap<String, Option<String>> = HashMap::new();
        let param_names = collect_param_names(fd);
        for (i, ty) in fd.type_sig.params.iter().enumerate() {
            if let Some(name) = param_names.get(i) {
                env.insert(name.clone(), extract_unit(ty).map(|u| u.to_string()));
            }
        }
        for expr in &fd.body {
            self.check_expr(expr, &mut env, errors);
        }
    }

    /// Walk an expression, propagating known unit labels through the environment.
    ///
    /// Returns the inferred unit label for the expression, or `None` when the
    /// result is dimensionless or when the unit cannot be determined.
    fn check_expr(
        &self,
        expr: &Expr,
        env: &mut HashMap<String, Option<String>>,
        errors: &mut Vec<LoomError>,
    ) -> Option<String> {
        match expr {
            Expr::Let { name, value, .. } => {
                let unit = self.check_expr(value, env, errors);
                env.insert(name.clone(), unit);
                None
            }

            Expr::BinOp {
                op,
                left,
                right,
                span,
            } => {
                let lu = self.check_expr(left, env, errors);
                let ru = self.check_expr(right, env, errors);
                match op {
                    BinOpKind::Add | BinOpKind::Sub => {
                        // Only report when BOTH sides have known, differing units.
                        if let (Some(l), Some(r)) = (&lu, &ru) {
                            if l != r {
                                errors.push(LoomError::type_err(
                                    format!(
                                        "unit mismatch: cannot add/subtract Float<{}> and Float<{}>",
                                        l, r
                                    ),
                                    span.clone(),
                                ));
                            }
                        }
                        lu.or(ru)
                    }
                    // Mul/Div: result is dimensionless.
                    _ => None,
                }
            }

            Expr::Ident(name) => env.get(name).cloned().flatten(),
            Expr::Literal(_) => None,

            Expr::Call { func, args, .. } => {
                for a in args {
                    self.check_expr(a, env, errors);
                }
                self.check_expr(func, env, errors);
                None
            }

            Expr::Pipe { left, right, .. } => {
                self.check_expr(left, env, errors);
                self.check_expr(right, env, errors);
                None
            }

            Expr::Match { subject, arms, .. } => {
                self.check_expr(subject, env, errors);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.check_expr(g, env, errors);
                    }
                    self.check_expr(&arm.body, env, errors);
                }
                None
            }

            Expr::FieldAccess { object, .. } => {
                self.check_expr(object, env, errors);
                None
            }

            Expr::Lambda { body, .. } => {
                self.check_expr(body, env, errors);
                None
            }

            Expr::ForIn { iter, body, .. } => {
                self.check_expr(iter, env, errors);
                self.check_expr(body, env, errors);
                None
            }

            Expr::Tuple(elems, _) => {
                for e in elems {
                    self.check_expr(e, env, errors);
                }
                None
            }

            Expr::Try(inner, _) => self.check_expr(inner, env, errors),
            Expr::As(inner, _) => {
                self.check_expr(inner, env, errors);
                None
            }
            Expr::InlineRust(_) => None,
        }
    }
}

// ── Parameter name inference ──────────────────────────────────────────────────
//
// Mirrors the logic in `codegen/rust.rs` so the checker uses the same naming
// convention as the emitter without creating a circular dependency.

fn collect_param_names(fd: &FnDef) -> Vec<String> {
    let n = fd.type_sig.params.len();
    let mut let_bound = HashSet::new();
    for e in &fd.body {
        collect_let_names(e, &mut let_bound);
    }

    let mut seen = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();

    let all: Vec<&Expr> = fd
        .body
        .iter()
        .chain(fd.requires.iter().map(|c| &c.expr))
        .chain(fd.ensures.iter().map(|c| &c.expr))
        .collect();

    for e in all {
        scan_free_idents(e, &let_bound, &mut seen, &mut ordered);
        if ordered.len() >= n {
            break;
        }
    }

    (0..n)
        .map(|i| {
            ordered
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("arg{}", i))
        })
        .collect()
}

fn collect_let_names(expr: &Expr, out: &mut HashSet<String>) {
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
        Expr::Lambda { body, .. } => collect_let_names(body, out),
        Expr::ForIn { iter, body, .. } => {
            collect_let_names(iter, out);
            collect_let_names(body, out);
        }
        Expr::Tuple(elems, _) => {
            for e in elems {
                collect_let_names(e, out);
            }
        }
        Expr::Try(inner, _) => collect_let_names(inner, out),
        Expr::As(inner, _) => collect_let_names(inner, out),
        Expr::Ident(_) | Expr::Literal(_) | Expr::InlineRust(_) => {}
    }
}

fn scan_free_idents(
    expr: &Expr,
    let_bound: &HashSet<String>,
    seen: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) {
    match expr {
        Expr::Ident(name) => {
            if !let_bound.contains(name) && seen.insert(name.clone()) {
                ordered.push(name.clone());
            }
        }
        Expr::Let { value, .. } => scan_free_idents(value, let_bound, seen, ordered),
        Expr::BinOp { left, right, .. } => {
            scan_free_idents(left, let_bound, seen, ordered);
            scan_free_idents(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            scan_free_idents(func, let_bound, seen, ordered);
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
        Expr::Lambda { body, .. } => scan_free_idents(body, let_bound, seen, ordered),
        Expr::ForIn { iter, body, .. } => {
            scan_free_idents(iter, let_bound, seen, ordered);
            scan_free_idents(body, let_bound, seen, ordered);
        }
        Expr::Tuple(elems, _) => {
            for e in elems {
                scan_free_idents(e, let_bound, seen, ordered);
            }
        }
        Expr::Try(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
        Expr::As(inner, _) => scan_free_idents(inner, let_bound, seen, ordered),
        Expr::Literal(_) | Expr::InlineRust(_) => {}
    }
}
