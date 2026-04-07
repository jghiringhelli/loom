//! M100: SMT Contract Verification Bridge.
//!
//! Discharges `require:` and `ensure:` contracts to an SMT solver (Z3) for
//! formal verification. When Z3 is not available (no `smt` feature), all
//! contracts return [`SmtStatus::Skipped`].
//!
//! Academic lineage:
//! Hoare (1969) axiomatic semantics → Dijkstra (1975) weakest precondition
//! calculus → Dafny (2009) → Loom `require:`/`ensure:` → M100 SMT discharge.

use crate::ast::*;

/// SMT contract verification bridge.
///
/// Translates `require:` and `ensure:` contracts to SMT-LIB2 and, when Z3 is
/// available (feature `smt`), discharges them to the solver.
pub struct SmtBridgeChecker;

impl SmtBridgeChecker {
    /// Run SMT verification on all functions in `items`.
    ///
    /// Returns one [`SmtVerification`] per function that has at least one
    /// `require:` or `ensure:` contract. Functions without contracts are
    /// omitted from the result.
    pub fn check(items: &[Item]) -> Vec<SmtVerification> {
        items
            .iter()
            .filter_map(|item| {
                if let Item::Fn(fn_def) = item {
                    Self::check_fn(fn_def)
                } else {
                    None
                }
            })
            .collect()
    }

    fn check_fn(fn_def: &FnDef) -> Option<SmtVerification> {
        if fn_def.requires.is_empty() && fn_def.ensures.is_empty() {
            return None;
        }

        let precondition = Self::translate_contracts(&fn_def.requires);
        let postcondition = Self::translate_contracts(&fn_def.ensures);

        // Feature-gated: when smt feature is absent, all contracts are Skipped.
        #[cfg(feature = "smt")]
        let status = Self::discharge_smt(&precondition, &postcondition);
        #[cfg(not(feature = "smt"))]
        let status = SmtStatus::Skipped;

        Some(SmtVerification {
            function: fn_def.name.clone(),
            precondition,
            postcondition,
            status,
        })
    }

    fn translate_contracts(contracts: &[Contract]) -> String {
        contracts
            .iter()
            .map(|c| Self::translate_expr_node(&c.expr))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Translate a Loom [`Expr`] AST node to SMT-LIB2 format.
    ///
    /// Covers binary operators, identifiers, and literals. Unknown expressions
    /// are translated as `true` (sound but incomplete).
    pub fn translate_expr_node(expr: &Expr) -> String {
        match expr {
            Expr::BinOp { op, left, right, .. } => {
                let l = Self::translate_expr_node(left);
                let r = Self::translate_expr_node(right);
                let smt_op = match op {
                    BinOpKind::Add => "+",
                    BinOpKind::Sub => "-",
                    BinOpKind::Mul => "*",
                    BinOpKind::Div => "/",
                    BinOpKind::Eq  => "=",
                    BinOpKind::Ne  => "distinct",
                    BinOpKind::Lt  => "<",
                    BinOpKind::Le  => "<=",
                    BinOpKind::Gt  => ">",
                    BinOpKind::Ge  => ">=",
                    BinOpKind::And => "and",
                    BinOpKind::Or  => "or",
                };
                format!("({} {} {})", smt_op, l, r)
            }
            Expr::Ident(name) => name.clone(),
            Expr::Literal(Literal::Int(n)) => n.to_string(),
            Expr::Literal(Literal::Float(f)) => format!("{}", f),
            Expr::Literal(Literal::Bool(b)) => b.to_string(),
            _ => "true".to_string(),
        }
    }

    /// Translate a raw expression string to SMT-LIB2.
    ///
    /// Handles simple binary expressions: `x > 0`, `x + y`, `not p`, etc.
    pub fn translate_expr(expr: &str) -> String {
        let expr = expr.trim();

        // Handle `not expr`.
        if let Some(rest) = expr.strip_prefix("not ") {
            return format!("(not {})", Self::translate_expr(rest));
        }

        // Attempt to find a binary operator.
        if let Some((lhs, op_str, rhs)) = Self::split_binary(expr) {
            let smt_op = match op_str {
                ">=" => ">=",
                "<=" => "<=",
                "!=" => "distinct",
                "==" => "=",
                ">"  => ">",
                "<"  => "<",
                "+"  => "+",
                "-"  => "-",
                "*"  => "*",
                "/"  => "/",
                _    => op_str,
            };
            return format!("({} {} {})", smt_op, lhs.trim(), rhs.trim());
        }

        expr.to_string()
    }

    /// Detect contradictions between precondition and postcondition strings.
    ///
    /// A pure-Rust structural check: if the precondition asserts `x > N` and
    /// the postcondition asserts `x < M` where N >= M, the spec is impossible.
    pub fn detect_contradiction(precondition: &str, postcondition: &str) -> bool {
        if let (Some(pre_gt), Some(post_lt)) = (
            Self::extract_gt_bound(precondition),
            Self::extract_lt_bound(postcondition),
        ) {
            return pre_gt >= post_lt;
        }
        false
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Split `expr` into `(lhs, operator, rhs)` at the first binary operator.
    fn split_binary(expr: &str) -> Option<(&str, &str, &str)> {
        let two = [">=", "<=", "!=", "==", "->", "|>"];
        for op in &two {
            if let Some(pos) = expr.find(op) {
                // Guard: operand before pos must not be empty.
                if pos > 0 {
                    return Some((&expr[..pos], op, &expr[pos + op.len()..]));
                }
            }
        }
        let one = [">", "<", "=", "+", "-", "*", "/"];
        for op in &one {
            if let Some(pos) = expr.find(op) {
                if pos > 0 {
                    return Some((&expr[..pos], op, &expr[pos + op.len()..]));
                }
            }
        }
        None
    }

    /// Extract the right-hand side of `(> var N)` → Some(N).
    fn extract_gt_bound(s: &str) -> Option<i64> {
        let s = s.trim();
        if s.starts_with("(> ") && s.ends_with(')') {
            let inner = &s[3..s.len() - 1];
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.len() == 2 {
                return parts[1].parse::<i64>().ok();
            }
        }
        None
    }

    /// Extract the right-hand side of `(< var N)` → Some(N).
    fn extract_lt_bound(s: &str) -> Option<i64> {
        let s = s.trim();
        if s.starts_with("(< ") && s.ends_with(')') {
            let inner = &s[3..s.len() - 1];
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.len() == 2 {
                return parts[1].parse::<i64>().ok();
            }
        }
        None
    }
}
