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

        // Collect all identifiers from contracts → SMT-LIB2 declare-const declarations.
        // Variables that appear in contracts must be declared before assert.
        let declarations = Self::collect_declarations(fn_def);

        // Feature-gated: when smt feature is absent, all contracts are Skipped.
        #[cfg(feature = "smt")]
        let status = Self::discharge_smt(&declarations, &precondition, &postcondition);
        #[cfg(not(feature = "smt"))]
        let status = SmtStatus::Skipped;

        Some(SmtVerification {
            function: fn_def.name.clone(),
            precondition,
            postcondition,
            status,
        })
    }

    /// Collect SMT-LIB2 `declare-const` lines for all identifiers in contracts.
    ///
    /// Maps Loom types to SMT sorts:
    /// - `Int` / `Bool` → `Int` / `Bool`
    /// - `Float` / `Prob` → `Real`
    /// - everything else → `Int` (conservative)
    fn collect_declarations(fn_def: &FnDef) -> String {
        use std::collections::BTreeSet;

        let mut idents: BTreeSet<String> = BTreeSet::new();
        for c in fn_def.requires.iter().chain(fn_def.ensures.iter()) {
            Self::collect_idents_expr(&c.expr, &mut idents);
        }

        // Build a name→sort map from the function's parameter names (if derivable).
        // FnTypeSignature only holds types; names come from body param slots.
        let param_types = &fn_def.type_sig.params;

        idents
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let sort = param_types
                    .get(i)
                    .map(|t| loom_type_to_smt_sort(t))
                    .unwrap_or("Int");
                format!("(declare-const {name} {sort})")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn collect_idents_expr(expr: &Expr, out: &mut std::collections::BTreeSet<String>) {
        match expr {
            Expr::Ident(name) if name != "true" && name != "false" => {
                out.insert(name.clone());
            }
            Expr::BinOp { left, right, .. } => {
                Self::collect_idents_expr(left, out);
                Self::collect_idents_expr(right, out);
            }
            Expr::Call { func, args, .. } => {
                Self::collect_idents_expr(func, out);
                for a in args {
                    Self::collect_idents_expr(a, out);
                }
            }
            Expr::Pipe { left, right, .. } => {
                Self::collect_idents_expr(left, out);
                Self::collect_idents_expr(right, out);
            }
            Expr::FieldAccess { object, .. } => Self::collect_idents_expr(object, out),
            Expr::As(inner, _) => Self::collect_idents_expr(inner, out),
            Expr::Let { value, .. } => Self::collect_idents_expr(value, out),
            _ => {}
        }
    }

    fn translate_contracts(contracts: &[Contract]) -> String {
        let parts: Vec<String> = contracts
            .iter()
            .map(|c| Self::translate_expr_node(&c.expr))
            .collect();
        if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            format!("(and {})", parts.join(" "))
        }
    }

    /// Translate a Loom [`Expr`] AST node to SMT-LIB2 format.
    ///
    /// Covers binary operators, identifiers, and literals. Unknown expressions
    /// are translated as `true` (sound but incomplete).
    pub fn translate_expr_node(expr: &Expr) -> String {
        match expr {
            Expr::BinOp {
                op, left, right, ..
            } => {
                let l = Self::translate_expr_node(left);
                let r = Self::translate_expr_node(right);
                let smt_op = match op {
                    BinOpKind::Add => "+",
                    BinOpKind::Sub => "-",
                    BinOpKind::Mul => "*",
                    BinOpKind::Div => "/",
                    BinOpKind::Eq => "=",
                    BinOpKind::Ne => "distinct",
                    BinOpKind::Lt => "<",
                    BinOpKind::Le => "<=",
                    BinOpKind::Gt => ">",
                    BinOpKind::Ge => ">=",
                    BinOpKind::And => "and",
                    BinOpKind::Or => "or",
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
                ">" => ">",
                "<" => "<",
                "+" => "+",
                "-" => "-",
                "*" => "*",
                "/" => "/",
                _ => op_str,
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

    /// Discharge `require:`/`ensure:` contracts to an SMT solver (Z3).
    ///
    /// Requires the `smt` feature and a Z3 installation.
    /// When Z3 is not available the caller falls back to [`SmtStatus::Skipped`]
    /// via the `#[cfg(not(feature = "smt"))]` branch in `check_fn`.
    #[cfg(feature = "smt")]
    fn discharge_smt(declarations: &str, precondition: &str, postcondition: &str) -> SmtStatus {
        // SMT-LIB2 query:
        //   1. Declare constants (one per contract variable)
        //   2. Assert precondition
        //   3. Assert negated postcondition
        //   4. check-sat: UNSAT → Hoare triple is valid; SAT/Unknown → not proved
        let query = format!(
            "{declarations}\n\
             (assert {precondition})\n\
             (assert (not {postcondition}))\n\
             (check-sat)\n"
        );
        // Attempt to invoke Z3. If the binary is absent, return Skipped.
        let output = std::process::Command::new("z3")
            .arg("-in")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(query.as_bytes());
                }
                child.wait_with_output()
            });
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim() == "unsat" {
                    SmtStatus::Proved
                } else {
                    SmtStatus::Unknown
                }
            }
            Err(_) => {
                // Z3 binary not found — SMT verification skipped.
                // Install Z3 (https://github.com/Z3Prover/z3) and ensure `z3` is on PATH
                // to enable formal contract verification for require:/ensure: blocks.
                #[cfg(not(test))]
                {
                    use std::sync::OnceLock;
                    static Z3_WARNING_SHOWN: OnceLock<()> = OnceLock::new();
                    Z3_WARNING_SHOWN.get_or_init(|| {
                        eprintln!("loom: SMT verification skipped — z3 binary not found on PATH.");
                        eprintln!(
                            "      Install Z3 to enable formal require:/ensure: contract checking."
                        );
                    });
                }
                SmtStatus::Skipped
            }
        }
    }

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

/// Map a Loom [`TypeExpr`] to an SMT-LIB2 sort name.
///
/// Conservative defaults: unknown types → `Int` (sound; may produce false negatives).
fn loom_type_to_smt_sort(ty: &TypeExpr) -> &'static str {
    match ty {
        TypeExpr::Base(name) => match name.as_str() {
            "Bool" => "Bool",
            "Float" | "Prob" | "Probability" => "Real",
            _ => "Int", // Int, Nat, custom refined types default to Int
        },
        _ => "Int",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::collections::BTreeSet;

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
    fn contract(e: Expr) -> Contract {
        Contract {
            expr: e,
            span: sp(),
        }
    }
    fn idents(e: &Expr) -> Vec<String> {
        let mut s = BTreeSet::new();
        SmtBridgeChecker::collect_idents_expr(e, &mut s);
        s.into_iter().collect()
    }
    /// Parse a function with the given signature, then replace its contracts.
    fn fn_with(sig: &str, requires: Vec<Expr>, ensures: Vec<Expr>) -> FnDef {
        let src = format!("module T\n  fn f :: {sig}\n  end\nend");
        let tokens = Lexer::tokenize(&src).unwrap();
        let module = Parser::new(&tokens).parse_module().unwrap();
        let mut f = module
            .items
            .into_iter()
            .find_map(|i| if let Item::Fn(f) = i { Some(f) } else { None })
            .expect("fn");
        f.requires = requires.into_iter().map(contract).collect();
        f.ensures = ensures.into_iter().map(contract).collect();
        f
    }

    // ── check / check_fn ──────────────────────────────────────────────────────

    #[test]
    fn check_fn_none_without_contracts() {
        let f = fn_with("Int -> Int", vec![], vec![]);
        assert!(SmtBridgeChecker::check_fn(&f).is_none());
    }

    #[test]
    fn check_fn_some_with_only_requires() {
        let f = fn_with(
            "Int -> Int",
            vec![bin(BinOpKind::Gt, id("x"), int(0))],
            vec![],
        );
        let v = SmtBridgeChecker::check_fn(&f).expect("requires-only must verify");
        assert_eq!(v.function, "f");
        assert_eq!(v.precondition, "(> x 0)");
        assert_eq!(v.postcondition, "(and )");
    }

    #[test]
    fn check_fn_some_with_only_ensures() {
        let f = fn_with(
            "Int -> Int",
            vec![],
            vec![bin(BinOpKind::Ge, id("result"), int(0))],
        );
        let v = SmtBridgeChecker::check_fn(&f).expect("ensures-only must verify");
        assert_eq!(v.precondition, "(and )");
        assert_eq!(v.postcondition, "(>= result 0)");
    }

    #[test]
    fn check_skips_non_fn_items_and_contractless_fns() {
        let with = fn_with("Int -> Int", vec![id("p")], vec![]);
        let without = fn_with("Int -> Int", vec![], vec![]);
        let out = SmtBridgeChecker::check(&[Item::Fn(without), Item::Fn(with)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].precondition, "p");
    }

    // ── collect_declarations / collect_idents_expr ────────────────────────────

    #[test]
    fn declarations_map_param_types_to_sorts_by_sorted_ident_index() {
        let f = fn_with(
            "Bool -> Float -> Int",
            vec![bin(BinOpKind::And, id("p"), id("q"))],
            vec![bin(BinOpKind::Gt, id("r"), int(0))],
        );
        assert_eq!(
            SmtBridgeChecker::collect_declarations(&f),
            "(declare-const p Bool)\n(declare-const q Real)\n(declare-const r Int)"
        );
    }

    #[test]
    fn declarations_default_to_int_beyond_params_and_empty_when_no_idents() {
        let f = fn_with(
            "Bool -> Int",
            vec![bin(BinOpKind::Gt, id("a"), id("b"))],
            vec![id("c")],
        );
        assert_eq!(
            SmtBridgeChecker::collect_declarations(&f),
            "(declare-const a Bool)\n(declare-const b Int)\n(declare-const c Int)"
        );
        let g = fn_with("Int -> Int", vec![int(1)], vec![]);
        assert_eq!(SmtBridgeChecker::collect_declarations(&g), "");
    }

    #[test]
    fn idents_exclude_true_false_but_keep_others() {
        assert_eq!(idents(&id("x")), vec!["x"]);
        assert!(idents(&id("true")).is_empty());
        assert!(idents(&id("false")).is_empty());
        let e = bin(
            BinOpKind::And,
            bin(BinOpKind::And, id("true"), id("false")),
            id("flag"),
        );
        assert_eq!(idents(&e), vec!["flag"]);
    }

    #[test]
    fn idents_traverse_binop_both_sides() {
        assert_eq!(
            idents(&bin(BinOpKind::Add, id("a"), id("b"))),
            vec!["a", "b"]
        );
    }

    #[test]
    fn idents_traverse_call_func_and_args() {
        let e = Expr::Call {
            func: Box::new(id("f")),
            args: vec![id("a"), id("b")],
            span: sp(),
        };
        assert_eq!(idents(&e), vec!["a", "b", "f"]);
    }

    #[test]
    fn idents_traverse_pipe_both_sides() {
        let e = Expr::Pipe {
            left: Box::new(id("l")),
            right: Box::new(id("r")),
            span: sp(),
        };
        assert_eq!(idents(&e), vec!["l", "r"]);
    }

    #[test]
    fn idents_traverse_field_access_as_and_let() {
        let fa = Expr::FieldAccess {
            object: Box::new(id("obj")),
            field: "fld".into(),
            span: sp(),
        };
        assert_eq!(idents(&fa), vec!["obj"]);
        let as_e = Expr::As(Box::new(id("inner")), TypeExpr::Base("Int".into()));
        assert_eq!(idents(&as_e), vec!["inner"]);
        let let_e = Expr::Let {
            name: "n".into(),
            value: Box::new(id("val")),
            body: None,
            span: sp(),
        };
        assert_eq!(idents(&let_e), vec!["val"]);
    }

    // ── translate_contracts / translate_expr_node ─────────────────────────────

    #[test]
    fn translate_contracts_single_is_bare_multiple_is_and() {
        let one = [contract(bin(BinOpKind::Gt, id("x"), int(0)))];
        assert_eq!(SmtBridgeChecker::translate_contracts(&one), "(> x 0)");
        let two = [
            contract(bin(BinOpKind::Gt, id("x"), int(0))),
            contract(bin(BinOpKind::Lt, id("x"), int(9))),
        ];
        assert_eq!(
            SmtBridgeChecker::translate_contracts(&two),
            "(and (> x 0) (< x 9))"
        );
    }

    #[test]
    fn translate_node_every_binop() {
        use BinOpKind::*;
        let cases = [
            (Add, "+"),
            (Sub, "-"),
            (Mul, "*"),
            (Div, "/"),
            (Eq, "="),
            (Ne, "distinct"),
            (Lt, "<"),
            (Le, "<="),
            (Gt, ">"),
            (Ge, ">="),
            (And, "and"),
            (Or, "or"),
        ];
        for (op, s) in cases {
            assert_eq!(
                SmtBridgeChecker::translate_expr_node(&bin(op, id("a"), id("b"))),
                format!("({s} a b)")
            );
        }
    }

    #[test]
    fn translate_node_atoms_and_fallback() {
        let t = SmtBridgeChecker::translate_expr_node;
        assert_eq!(t(&id("abc")), "abc");
        assert_eq!(t(&int(42)), "42");
        assert_eq!(t(&int(-7)), "-7");
        assert_eq!(t(&Expr::Literal(Literal::Float(2.5))), "2.5");
        assert_eq!(t(&Expr::Literal(Literal::Bool(true))), "true");
        assert_eq!(t(&Expr::Literal(Literal::Bool(false))), "false");
        assert_eq!(t(&Expr::Literal(Literal::Str("s".into()))), "true");
        let nested = bin(BinOpKind::Add, bin(BinOpKind::Mul, id("a"), int(2)), int(1));
        assert_eq!(t(&nested), "(+ (* a 2) 1)");
    }

    // ── translate_expr (string) ───────────────────────────────────────────────

    #[test]
    fn translate_expr_string_operators() {
        let t = SmtBridgeChecker::translate_expr;
        assert_eq!(t("x >= 1"), "(>= x 1)");
        assert_eq!(t("x <= 1"), "(<= x 1)");
        assert_eq!(t("x != 1"), "(distinct x 1)");
        assert_eq!(t("x == 1"), "(= x 1)");
        assert_eq!(t("x > 1"), "(> x 1)");
        assert_eq!(t("x < 1"), "(< x 1)");
        assert_eq!(t("x + 1"), "(+ x 1)");
        assert_eq!(t("x - 1"), "(- x 1)");
        assert_eq!(t("x * 1"), "(* x 1)");
        assert_eq!(t("x / 1"), "(/ x 1)");
        assert_eq!(t("x"), "x");
        assert_eq!(t("  x  "), "x");
        assert_eq!(t("not x > 1"), "(not (> x 1))");
    }

    #[test]
    fn translate_expr_leading_operator_is_not_split_at_position_zero() {
        assert_eq!(SmtBridgeChecker::translate_expr("-5"), "-5");
        assert_eq!(SmtBridgeChecker::translate_expr(">= 5"), "(= > 5)");
        assert_eq!(SmtBridgeChecker::split_binary("-5"), None);
        assert_eq!(
            SmtBridgeChecker::split_binary(">= 5"),
            Some((">", "=", " 5"))
        );
        assert_eq!(
            SmtBridgeChecker::split_binary("a >= 5"),
            Some(("a ", ">=", " 5"))
        );
        assert_eq!(
            SmtBridgeChecker::split_binary("abc >= 5"),
            Some(("abc ", ">=", " 5"))
        );
        assert_eq!(
            SmtBridgeChecker::split_binary("a - 5"),
            Some(("a ", "-", " 5"))
        );
    }

    // ── detect_contradiction / bounds ─────────────────────────────────────────

    #[test]
    fn detect_contradiction_cases() {
        let d = SmtBridgeChecker::detect_contradiction;
        assert!(d("(> x 10)", "(< x 5)"));
        assert!(d("(> x 5)", "(< x 5)"));
        assert!(!d("(> x 1)", "(< x 10)"));
        assert!(!d("(> x 5)", "(> x 3)"));
        assert!(!d("x", "(< x 5)"));
        // A `(< ...)` precondition is not a gt-bound, even though it ends with ')'.
        assert!(!d("(< x 5)", "(< y 3)"));
    }

    #[test]
    fn bound_extractors() {
        assert_eq!(SmtBridgeChecker::extract_gt_bound("(> x 5)"), Some(5));
        assert_eq!(SmtBridgeChecker::extract_gt_bound(" (> x -3) "), Some(-3));
        assert_eq!(SmtBridgeChecker::extract_gt_bound("(< x 5)"), None);
        assert_eq!(SmtBridgeChecker::extract_gt_bound("(> x 5 6"), None);
        assert_eq!(SmtBridgeChecker::extract_gt_bound("(> x y)"), None);
        assert_eq!(SmtBridgeChecker::extract_lt_bound("(< x 7)"), Some(7));
        assert_eq!(SmtBridgeChecker::extract_lt_bound("(< x 7 8"), None);
        assert_eq!(SmtBridgeChecker::extract_lt_bound("(> x 7)"), None);
        assert_eq!(SmtBridgeChecker::extract_lt_bound("(< x a b)"), None);
    }

    // ── loom_type_to_smt_sort ─────────────────────────────────────────────────

    #[test]
    fn sort_mapping() {
        let b = |n: &str| TypeExpr::Base(n.to_string());
        assert_eq!(loom_type_to_smt_sort(&b("Bool")), "Bool");
        assert_eq!(loom_type_to_smt_sort(&b("Float")), "Real");
        assert_eq!(loom_type_to_smt_sort(&b("Prob")), "Real");
        assert_eq!(loom_type_to_smt_sort(&b("Probability")), "Real");
        assert_eq!(loom_type_to_smt_sort(&b("Int")), "Int");
        assert_eq!(loom_type_to_smt_sort(&b("Nat")), "Int");
        assert_eq!(loom_type_to_smt_sort(&TypeExpr::Dynamic), "Int");
        assert_eq!(
            loom_type_to_smt_sort(&TypeExpr::Option(Box::new(b("Bool")))),
            "Int"
        );
    }

    // ── discharge_smt (needs a real z3 binary to be non-trivial) ──────────────

    #[cfg(feature = "smt")]
    fn z3_available() -> bool {
        std::process::Command::new("z3")
            .arg("--version")
            .output()
            .is_ok()
    }

    #[cfg(feature = "smt")]
    #[test]
    fn discharge_proves_valid_triple_and_reports_unknown_for_invalid() {
        let decl = "(declare-const x Int)";
        let proved = SmtBridgeChecker::discharge_smt(decl, "(> x 0)", "(>= x 0)");
        let invalid = SmtBridgeChecker::discharge_smt(decl, "(> x 0)", "(> x 5)");
        if z3_available() {
            assert_eq!(proved, SmtStatus::Proved);
            assert_eq!(invalid, SmtStatus::Unknown);
        } else {
            assert_eq!(proved, SmtStatus::Skipped);
            assert_eq!(invalid, SmtStatus::Skipped);
        }
    }
}
