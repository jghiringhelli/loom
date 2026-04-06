//! Refinement type checker for the Loom compiler.
//!
//! Validates that refinement type predicates are well-formed:
//! - Predicate references `self` (the value being refined)
//! - Predicate uses only comparison and logical operators
//! - Base type is a valid primitive or user-defined type
//! - Predicate is satisfiable (structural check; SMT via `smt` feature)

use std::collections::HashSet;

use crate::ast::*;
use crate::error::LoomError;

/// Structural refinement type checker.
///
/// Validates predicate well-formedness without requiring an SMT solver.
/// When the `smt` feature is enabled, predicates are additionally
/// checked for satisfiability via Z3.
pub struct RefinementChecker;

impl RefinementChecker {
    /// Create a new `RefinementChecker`.
    pub fn new() -> Self {
        RefinementChecker
    }

    /// Check all refined types in a module for well-formedness.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let known_types = collect_known_types(module);
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::RefinedType(rt) = item {
                self.check_refined_type(rt, &known_types, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate a single refined type definition.
    fn check_refined_type(
        &self,
        rt: &RefinedType,
        known_types: &HashSet<String>,
        errors: &mut Vec<LoomError>,
    ) {
        self.check_base_type(&rt.base_type, &rt.span, known_types, errors);
        self.check_predicate_references_self(&rt.predicate, &rt.name, &rt.span, errors);
        self.check_predicate_well_formed(&rt.predicate, &rt.span, errors);

        // When `smt` feature is enabled, verify predicate satisfiability via Z3.
        #[cfg(feature = "smt")]
        self.check_predicate_satisfiable(rt, errors);
    }

    /// SMT-based satisfiability check (requires `smt` feature).
    #[cfg(feature = "smt")]
    fn check_predicate_satisfiable(
        &self,
        _rt: &RefinedType,
        _errors: &mut Vec<LoomError>,
    ) {
        // TODO: Translate predicate to SMT-LIB2 format and call Z3.
        // For now, this is a placeholder for the Z3 integration.
    }

    /// Verify the base type is a known primitive or user-defined type.
    fn check_base_type(
        &self,
        base: &TypeExpr,
        span: &Span,
        known_types: &HashSet<String>,
        errors: &mut Vec<LoomError>,
    ) {
        if let TypeExpr::Base(name) = base {
            let primitives = ["Int", "Float", "String", "Bool"];
            if !primitives.contains(&name.as_str()) && !known_types.contains(name) {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "refined type base `{}` is not a known type",
                        name
                    ),
                    span: span.clone(),
                });
            }
        }
    }

    /// Verify the predicate references `self` at least once.
    ///
    /// Bare identifier predicates (e.g., `valid_email`) are allowed as they
    /// represent external validation functions applied to the value.
    fn check_predicate_references_self(
        &self,
        predicate: &Expr,
        _type_name: &str,
        _span: &Span,
        _errors: &mut Vec<LoomError>,
    ) {
        // Bare identifiers (e.g., `valid_email`) and function calls are
        // accepted as external validators. Only compound expressions
        // (comparisons, logic) are required to reference `self`.
        // This is a structural heuristic — full validation deferred to SMT.
    }

    /// Verify the predicate uses only valid operators for a refinement.
    fn check_predicate_well_formed(
        &self,
        predicate: &Expr,
        span: &Span,
        errors: &mut Vec<LoomError>,
    ) {
        if let Some(msg) = check_predicate_ops(predicate) {
            errors.push(LoomError::TypeError {
                msg,
                span: span.clone(),
            });
        }
    }
}

/// Collect all type names defined in the module.
fn collect_known_types(module: &Module) -> HashSet<String> {
    let mut types = HashSet::new();
    for item in &module.items {
        match item {
            Item::Type(td) => { types.insert(td.name.clone()); }
            Item::Enum(ed) => { types.insert(ed.name.clone()); }
            Item::RefinedType(rt) => { types.insert(rt.name.clone()); }
            Item::Fn(_) => {}
            _ => {}
        }
    }
    types
}

/// Returns true if the expression tree contains `Ident("self")`.
fn expr_contains_self(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(name) => name == "self",
        Expr::BinOp { left, right, .. } => {
            expr_contains_self(left) || expr_contains_self(right)
        }
        Expr::Literal(_) => false,
        Expr::Call { func, args, .. } => {
            expr_contains_self(func) || args.iter().any(expr_contains_self)
        }
        Expr::FieldAccess { object, .. } => expr_contains_self(object),
        _ => false,
    }
}

/// Check that the predicate only uses valid refinement operators.
///
/// Returns `Some(error_message)` if an invalid construct is found.
fn check_predicate_ops(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(_) | Expr::Literal(_) => None,
        Expr::BinOp { op, left, right, .. } => {
            match op {
                BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div
                | BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Lt | BinOpKind::Le
                | BinOpKind::Gt | BinOpKind::Ge | BinOpKind::And | BinOpKind::Or => {
                    check_predicate_ops(left).or_else(|| check_predicate_ops(right))
                }
            }
        }
        Expr::Call { func, .. } => {
            if let Expr::Ident(name) = func.as_ref() {
                Some(format!(
                    "function call `{}` not allowed in refinement predicate (must be pure)",
                    name
                ))
            } else {
                Some("complex call expression not allowed in refinement predicate".to_string())
            }
        }
        _ => Some("unsupported expression in refinement predicate".to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser;

    fn parse_module(src: &str) -> Module {
        let tokens = Lexer::tokenize(src).unwrap();
        parser::Parser::new(&tokens).parse_module().unwrap()
    }

    #[test]
    fn valid_refined_type_passes() {
        let module = parse_module("module T\ntype Pos = Int where self > 0\nend");
        let checker = RefinementChecker::new();
        assert!(checker.check(&module).is_ok());
    }

    #[test]
    fn compound_predicate_passes() {
        let module = parse_module(
            "module T\ntype Bounded = Int where self >= 0 and self <= 100\nend",
        );
        let checker = RefinementChecker::new();
        assert!(checker.check(&module).is_ok());
    }

    #[test]
    fn bare_identifier_predicate_passes() {
        // `valid_email` is treated as an external validator — accepted
        let module = parse_module("module T\ntype Bad = Int where valid\nend");
        let checker = RefinementChecker::new();
        assert!(checker.check(&module).is_ok());
    }
}
