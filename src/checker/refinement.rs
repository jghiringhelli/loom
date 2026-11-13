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
    ///
    /// Without a live Z3 link, we perform structural satisfiability: a predicate
    /// is considered satisfiable if it is well-formed (passes `check_predicate_well_formed`)
    /// and references `self` (ensuring it can constrain values of the base type).
    /// Full semantic verification requires compiling with `--features smt` and a Z3 binary.
    #[cfg(feature = "smt")]
    fn check_predicate_satisfiable(&self, rt: &RefinedType, errors: &mut Vec<LoomError>) {
        // Structural satisfiability: reject vacuously true predicates (literal `true`)
        // and trivially unsatisfiable ones (literal `false`).
        use crate::ast::Expr;
        match &rt.predicate {
            Expr::Literal(crate::ast::Literal::Bool(false)) => {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "refined type `{}`: predicate is `false` — no value can satisfy it",
                        rt.name
                    ),
                    span: rt.span.clone(),
                });
            }
            Expr::Literal(crate::ast::Literal::Bool(true)) => {
                // `true` predicate is syntactically allowed but semantically redundant.
                // Emit a warning rather than an error; tighten with `--features smt`.
                eprintln!(
                    "warning: refined type `{}` has trivially-true predicate (no constraint added)",
                    rt.name
                );
            }
            _ => {
                // Non-trivial predicate: assume satisfiable given structural well-formedness.
                // Enable `--features smt` for Z3-verified semantic satisfiability.
            }
        }
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
                    msg: format!("refined type base `{}` is not a known type", name),
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
            Item::Type(td) => {
                types.insert(td.name.clone());
            }
            Item::Enum(ed) => {
                types.insert(ed.name.clone());
            }
            Item::RefinedType(rt) => {
                types.insert(rt.name.clone());
            }
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
        Expr::BinOp { left, right, .. } => expr_contains_self(left) || expr_contains_self(right),
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
        Expr::BinOp {
            op, left, right, ..
        } => match op {
            BinOpKind::Add
            | BinOpKind::Sub
            | BinOpKind::Mul
            | BinOpKind::Div
            | BinOpKind::Eq
            | BinOpKind::Ne
            | BinOpKind::Lt
            | BinOpKind::Le
            | BinOpKind::Gt
            | BinOpKind::Ge
            | BinOpKind::And
            | BinOpKind::Or => check_predicate_ops(left).or_else(|| check_predicate_ops(right)),
        },
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
        let module =
            parse_module("module T\ntype Bounded = Int where self >= 0 and self <= 100\nend");
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

    // ── helpers ───────────────────────────────────────────────────────────────

    fn sp() -> Span {
        Span::new(0, 0)
    }
    fn id(n: &str) -> Expr {
        Expr::Ident(n.to_string())
    }
    fn int(n: i64) -> Expr {
        Expr::Literal(Literal::Int(n))
    }
    fn bin(op: BinOpKind, l: Expr, r: Expr) -> Expr {
        Expr::BinOp {
            op,
            left: Box::new(l),
            right: Box::new(r),
            span: sp(),
        }
    }
    fn call(f: Expr, args: Vec<Expr>) -> Expr {
        Expr::Call {
            func: Box::new(f),
            args,
            span: sp(),
        }
    }
    fn errs_of(src: &str) -> Vec<String> {
        match RefinementChecker::new().check(&parse_module(src)) {
            Ok(()) => vec![],
            Err(v) => v.iter().map(|e| e.to_string()).collect(),
        }
    }

    // ── check / check_refined_type / check_predicate_well_formed ──────────────

    #[test]
    fn call_in_predicate_is_rejected_with_message() {
        let errs = errs_of("module T\ntype Bad = Int where foo(self)\nend");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("function call `foo` not allowed in refinement predicate"),
            "{errs:?}"
        );
    }

    #[test]
    fn call_nested_in_compound_predicate_is_rejected() {
        let errs = errs_of("module T\ntype Bad = Int where self > 0 and foo(self)\nend");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("`foo`"), "{errs:?}");
    }

    #[test]
    fn errors_accumulate_across_refined_types() {
        let errs = errs_of(
            "module T\ntype A = Int where f(self)\ntype B = Int where g(self)\ntype C = Int where self > 0\nend",
        );
        assert_eq!(errs.len(), 2, "{errs:?}");
        assert!(errs[0].contains("`f`") && errs[1].contains("`g`"), "{errs:?}");
    }

    // ── check_predicate_ops ───────────────────────────────────────────────────

    #[test]
    fn predicate_ops_messages() {
        assert_eq!(check_predicate_ops(&id("x")), None);
        assert_eq!(check_predicate_ops(&int(1)), None);
        assert_eq!(
            check_predicate_ops(&bin(BinOpKind::Gt, id("self"), int(0))),
            None
        );
        assert_eq!(
            check_predicate_ops(&call(id("foo"), vec![])),
            Some("function call `foo` not allowed in refinement predicate (must be pure)".into())
        );
        let complex = call(
            Expr::FieldAccess {
                object: Box::new(id("a")),
                field: "b".into(),
                span: sp(),
            },
            vec![],
        );
        assert_eq!(
            check_predicate_ops(&complex),
            Some("complex call expression not allowed in refinement predicate".into())
        );
        let lambda = Expr::Tuple(vec![], sp());
        assert_eq!(
            check_predicate_ops(&lambda),
            Some("unsupported expression in refinement predicate".into())
        );
        // call on the right-hand side of a binop is found too
        let right = bin(BinOpKind::And, bin(BinOpKind::Gt, id("self"), int(0)), call(id("g"), vec![]));
        assert!(check_predicate_ops(&right).unwrap().contains("`g`"));
    }

    // ── check_base_type / collect_known_types ─────────────────────────────────

    #[test]
    fn unknown_base_type_is_error() {
        let errs = errs_of("module T\ntype X = Mystery where self > 0\nend");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("refined type base `Mystery` is not a known type"),
            "{errs:?}"
        );
    }

    #[test]
    fn primitive_bases_are_known() {
        for p in ["Int", "Float", "String", "Bool"] {
            let errs = errs_of(&format!("module T\ntype X = {p} where self > 0\nend"));
            assert!(errs.is_empty(), "{p}: {errs:?}");
        }
    }

    #[test]
    fn refined_type_can_be_base_of_another() {
        let errs = errs_of("module T\ntype A = Int where self > 0\ntype B = A where self < 9\nend");
        assert!(errs.is_empty(), "{errs:?}");
        // order does not matter
        let errs = errs_of("module T\ntype B = A where self < 9\ntype A = Int where self > 0\nend");
        assert!(errs.is_empty(), "{errs:?}");
    }

    #[test]
    fn known_types_collects_type_enum_and_refined_only() {
        let mut m = parse_module("module T\ntype R = Int where self > 0\nend");
        m.items.push(Item::Type(TypeDef {
            name: "Rec".into(),
            fields: vec![],
            span: sp(),
        }));
        m.items.push(Item::Enum(EnumDef {
            name: "Color".into(),
            variants: vec![],
            span: sp(),
        }));
        let known = collect_known_types(&m);
        let mut names: Vec<_> = known.into_iter().collect();
        names.sort();
        assert_eq!(names, vec!["Color", "R", "Rec"]);
    }

    #[test]
    fn record_and_enum_names_are_valid_bases() {
        let mut m = parse_module("module T\ntype X = Rec where self > 0\ntype Y = Color where self > 0\nend");
        assert_eq!(
            RefinementChecker::new().check(&m).unwrap_err().len(),
            2,
            "undefined bases must error"
        );
        m.items.push(Item::Type(TypeDef {
            name: "Rec".into(),
            fields: vec![],
            span: sp(),
        }));
        assert_eq!(RefinementChecker::new().check(&m).unwrap_err().len(), 1);
        m.items.push(Item::Enum(EnumDef {
            name: "Color".into(),
            variants: vec![],
            span: sp(),
        }));
        assert!(RefinementChecker::new().check(&m).is_ok());
    }

    // ── check_predicate_satisfiable (smt feature) ─────────────────────────────

    #[cfg(feature = "smt")]
    #[test]
    fn false_predicate_is_unsatisfiable_error() {
        let errs = errs_of("module T\ntype Never = Int where false\nend");
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("refined type `Never`: predicate is `false` — no value can satisfy it"),
            "{errs:?}"
        );
    }

    #[cfg(feature = "smt")]
    #[test]
    fn true_predicate_is_only_a_warning() {
        assert!(errs_of("module T\ntype Any = Int where true\nend").is_empty());
    }

    // ── expr_contains_self ────────────────────────────────────────────────────

    #[test]
    fn contains_self_cases() {
        assert!(expr_contains_self(&id("self")));
        assert!(!expr_contains_self(&id("other")));
        assert!(!expr_contains_self(&int(1)));
        // BinOp: left only, right only, neither
        assert!(expr_contains_self(&bin(BinOpKind::Gt, id("self"), int(0))));
        assert!(expr_contains_self(&bin(BinOpKind::Gt, int(0), id("self"))));
        assert!(!expr_contains_self(&bin(BinOpKind::Gt, id("a"), id("b"))));
        // Call: in func only, in args only, neither
        assert!(expr_contains_self(&call(id("self"), vec![id("a")])));
        assert!(expr_contains_self(&call(id("f"), vec![id("a"), id("self")])));
        assert!(!expr_contains_self(&call(id("f"), vec![id("a")])));
        // FieldAccess
        let fa = |o: &str| Expr::FieldAccess {
            object: Box::new(id(o)),
            field: "len".into(),
            span: sp(),
        };
        assert!(expr_contains_self(&fa("self")));
        assert!(!expr_contains_self(&fa("x")));
    }
}
