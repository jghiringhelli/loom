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

        // Pure-Rust structural check: clear contradictions between simple
        // comparisons of the same variable make the contract unsatisfiable.
        let status = match Self::find_contract_contradiction(fn_def) {
            Some(msg) => SmtStatus::Counterexample(msg),
            None => status,
        };

        Some(SmtVerification {
            function: fn_def.name.clone(),
            precondition,
            postcondition,
            status,
        })
    }

    /// Flatten `a && b` into conjuncts.
    fn conjuncts<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
        match e {
            Expr::BinOp {
                op: BinOpKind::And,
                left,
                right,
                ..
            } => {
                Self::conjuncts(left, out);
                Self::conjuncts(right, out);
            }
            other => out.push(other),
        }
    }

    /// Extract `(var, op, bound)` from `var <op> number` or `number <op> var`.
    fn simple_bound(e: &Expr) -> Option<(String, BinOpKind, f64)> {
        let Expr::BinOp {
            op, left, right, ..
        } = e
        else {
            return None;
        };
        let num = |x: &Expr| match x {
            Expr::Literal(Literal::Int(i)) => Some(*i as f64),
            Expr::Literal(Literal::Float(f)) if f.is_finite() => Some(*f),
            _ => None,
        };
        match (&**left, &**right) {
            (Expr::Ident(v), r) => num(r).map(|n| (v.clone(), op.clone(), n)),
            (l, Expr::Ident(v)) => {
                let flipped = match op {
                    BinOpKind::Lt => BinOpKind::Gt,
                    BinOpKind::Le => BinOpKind::Ge,
                    BinOpKind::Gt => BinOpKind::Lt,
                    BinOpKind::Ge => BinOpKind::Le,
                    BinOpKind::Eq => BinOpKind::Eq,
                    _ => return None,
                };
                num(l).map(|n| (v.clone(), flipped, n))
            }
            _ => None,
        }
    }

    /// Detect a clear contradiction: the conjunction of all simple comparisons
    /// (`x > 5`, `x <= 3`, `x == 2`, ...) of one variable across `require:` and
    /// `ensure:` is unsatisfiable over the reals. Anything non-trivial is ignored
    /// (no false positives); `result` is skipped as it names the return value.
    fn find_contract_contradiction(fn_def: &FnDef) -> Option<String> {
        use std::collections::HashMap;
        // var -> (lower, lower_strict, upper, upper_strict)
        let mut ranges: HashMap<String, (f64, bool, f64, bool)> = HashMap::new();
        let mut exprs = Vec::new();
        for c in fn_def.requires.iter().chain(fn_def.ensures.iter()) {
            Self::conjuncts(&c.expr, &mut exprs);
        }
        for e in exprs {
            let Some((v, op, n)) = Self::simple_bound(e) else {
                continue;
            };
            if v == "result" {
                continue;
            }
            let r = ranges
                .entry(v.clone())
                .or_insert((f64::NEG_INFINITY, false, f64::INFINITY, false));
            let lower = |r: &mut (f64, bool, f64, bool), n: f64, strict: bool| {
                if n > r.0 || (n == r.0 && strict) {
                    r.0 = n;
                    r.1 = strict;
                }
            };
            let upper = |r: &mut (f64, bool, f64, bool), n: f64, strict: bool| {
                if n < r.2 || (n == r.2 && strict) {
                    r.2 = n;
                    r.3 = strict;
                }
            };
            match op {
                BinOpKind::Gt => lower(r, n, true),
                BinOpKind::Ge => lower(r, n, false),
                BinOpKind::Lt => upper(r, n, true),
                BinOpKind::Le => upper(r, n, false),
                BinOpKind::Eq => {
                    lower(r, n, false);
                    upper(r, n, false);
                }
                _ => {}
            }
            let (lo, lo_s, hi, hi_s) = *r;
            if lo > hi || (lo == hi && (lo_s || hi_s)) {
                return Some(format!(
                    "contract constraints on '{v}' are unsatisfiable (lower bound {lo}, upper bound {hi})"
                ));
            }
        }
        None
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

    // ── structural contradiction detector ─────────────────────────────────────

    fn flt(f: f64) -> Expr {
        Expr::Literal(Literal::Float(f))
    }
    fn contra(f: &FnDef) -> Option<String> {
        SmtBridgeChecker::find_contract_contradiction(f)
    }
    fn contra_of(reqs: Vec<Expr>, ens: Vec<Expr>) -> Option<String> {
        contra(&fn_with("Int -> Int", reqs, ens))
    }
    const OPS: [BinOpKind; 5] = [
        BinOpKind::Gt,
        BinOpKind::Ge,
        BinOpKind::Lt,
        BinOpKind::Le,
        BinOpKind::Eq,
    ];
    fn holds(op: &BinOpKind, n: i64, x: f64) -> bool {
        let n = n as f64;
        match op {
            BinOpKind::Gt => x > n,
            BinOpKind::Ge => x >= n,
            BinOpKind::Lt => x < n,
            BinOpKind::Le => x <= n,
            BinOpKind::Eq => x == n,
            _ => unreachable!(),
        }
    }
    /// Independent oracle: is any x on a half-integer grid a solution?
    fn satisfiable(cs: &[(BinOpKind, i64)]) -> bool {
        (0..=20).any(|k| {
            let x = k as f64 * 0.5;
            cs.iter().all(|(op, n)| holds(op, *n, x))
        })
    }
    fn all_constraints() -> Vec<(BinOpKind, i64)> {
        let mut all = Vec::new();
        for op in OPS.iter() {
            for n in [4i64, 5, 6] {
                all.push((op.clone(), n));
            }
        }
        all
    }

    #[test]
    fn contradiction_exhaustive_pairs_match_oracle_in_every_placement() {
        let all = all_constraints();
        for a in &all {
            for b in &all {
                let want = !satisfiable(&[a.clone(), b.clone()]);
                let ea = bin(a.0.clone(), id("x"), int(a.1));
                let eb = bin(b.0.clone(), id("x"), int(b.1));
                let placements = [
                    (vec![ea.clone(), eb.clone()], vec![]),
                    (vec![], vec![ea.clone(), eb.clone()]),
                    (vec![ea.clone()], vec![eb.clone()]),
                    (vec![eb.clone()], vec![ea.clone()]),
                ];
                for (r, e) in placements {
                    assert_eq!(contra_of(r, e).is_some(), want, "{a:?} with {b:?}");
                }
            }
        }
    }

    #[test]
    fn contradiction_exhaustive_triples_match_oracle() {
        let all = all_constraints();
        let mut f = fn_with("Int -> Int", vec![], vec![]);
        for a in &all {
            for b in &all {
                for c in &all {
                    let want = !satisfiable(&[a.clone(), b.clone(), c.clone()]);
                    f.requires = [a, b, c]
                        .iter()
                        .map(|(op, n)| contract(bin(op.clone(), id("x"), int(*n))))
                        .collect();
                    assert_eq!(contra(&f).is_some(), want, "{a:?} {b:?} {c:?}");
                }
            }
        }
    }

    #[test]
    fn contradiction_number_on_left_is_flipped_correctly() {
        let all = all_constraints();
        let flip = |op: &BinOpKind| match op {
            BinOpKind::Gt => BinOpKind::Lt,
            BinOpKind::Ge => BinOpKind::Le,
            BinOpKind::Lt => BinOpKind::Gt,
            BinOpKind::Le => BinOpKind::Ge,
            o => o.clone(),
        };
        for a in &all {
            for b in &all {
                let want = !satisfiable(&[a.clone(), b.clone()]);
                // `n op x` means `x (flip op) n`
                let ea = bin(flip(&a.0), int(a.1), id("x"));
                let eb = bin(b.0.clone(), id("x"), int(b.1));
                assert_eq!(
                    contra_of(vec![ea.clone(), eb.clone()], vec![]).is_some(),
                    want,
                    "left-literal {a:?} with {b:?}"
                );
                assert_eq!(
                    contra_of(vec![eb], vec![ea]).is_some(),
                    want,
                    "left-literal ensure {a:?} with {b:?}"
                );
            }
        }
    }

    #[test]
    fn contradiction_spec_examples() {
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Lt, id("x"), int(3))]
        )
        .is_some());
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Le, id("x"), int(5))]
        )
        .is_some());
        // over the reals 5 < x < 6 is satisfiable
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Lt, id("x"), int(6))]
        )
        .is_none());
        assert!(contra_of(
            vec![bin(BinOpKind::Ge, id("x"), int(5))],
            vec![bin(BinOpKind::Le, id("x"), int(5))]
        )
        .is_none());
    }

    #[test]
    fn contradiction_message_reports_variable_and_bounds() {
        let m = contra_of(
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Lt, id("x"), int(3))],
        )
        .unwrap();
        assert_eq!(
            m,
            "contract constraints on 'x' are unsatisfiable (lower bound 5, upper bound 3)"
        );
    }

    #[test]
    fn contradiction_tighter_bound_kept_regardless_of_order() {
        let m = contra_of(
            vec![
                bin(BinOpKind::Lt, id("x"), int(3)),
                bin(BinOpKind::Lt, id("x"), int(10)),
                bin(BinOpKind::Gt, id("x"), int(4)),
            ],
            vec![],
        );
        assert!(m.unwrap().contains("lower bound 4, upper bound 3"));
        let m = contra_of(
            vec![
                bin(BinOpKind::Gt, id("x"), int(7)),
                bin(BinOpKind::Gt, id("x"), int(1)),
                bin(BinOpKind::Lt, id("x"), int(6)),
            ],
            vec![],
        );
        assert!(m.unwrap().contains("lower bound 7, upper bound 6"));
    }

    #[test]
    fn contradiction_conjunctions_are_flattened() {
        let conj = bin(
            BinOpKind::And,
            bin(BinOpKind::Gt, id("x"), int(5)),
            bin(BinOpKind::Lt, id("x"), int(3)),
        );
        assert!(contra_of(vec![conj], vec![]).is_some());
        let nested = bin(
            BinOpKind::And,
            bin(
                BinOpKind::And,
                bin(BinOpKind::Ge, id("y"), int(0)),
                bin(BinOpKind::Le, id("y"), int(10)),
            ),
            bin(
                BinOpKind::And,
                bin(BinOpKind::Lt, id("x"), int(2)),
                bin(BinOpKind::Gt, id("x"), int(2)),
            ),
        );
        let m = contra_of(vec![nested], vec![]).unwrap();
        assert!(m.contains("'x'"), "{m}");
        let ok = bin(
            BinOpKind::And,
            bin(BinOpKind::Ge, id("x"), int(2)),
            bin(BinOpKind::Le, id("x"), int(2)),
        );
        assert!(contra_of(vec![ok], vec![]).is_none());
        let right = bin(
            BinOpKind::And,
            bin(BinOpKind::Ge, id("z"), int(0)),
            bin(
                BinOpKind::And,
                bin(BinOpKind::Gt, id("x"), int(9)),
                bin(BinOpKind::Lt, id("x"), int(9)),
            ),
        );
        assert!(contra_of(vec![right], vec![]).is_some());
    }

    #[test]
    fn contradiction_result_variable_is_skipped() {
        assert!(contra_of(
            vec![],
            vec![
                bin(BinOpKind::Gt, id("result"), int(5)),
                bin(BinOpKind::Lt, id("result"), int(3)),
            ]
        )
        .is_none());
        let m = contra_of(
            vec![
                bin(BinOpKind::Gt, id("result"), int(5)),
                bin(BinOpKind::Gt, id("x"), int(5)),
            ],
            vec![
                bin(BinOpKind::Lt, id("result"), int(3)),
                bin(BinOpKind::Lt, id("x"), int(3)),
            ],
        )
        .unwrap();
        assert!(m.contains("'x'"), "{m}");
    }

    #[test]
    fn contradiction_variables_are_independent() {
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Lt, id("y"), int(3))]
        )
        .is_none());
        let m = contra_of(
            vec![
                bin(BinOpKind::Gt, id("x"), int(1)),
                bin(BinOpKind::Gt, id("y"), int(5)),
            ],
            vec![bin(BinOpKind::Lt, id("y"), int(3))],
        )
        .unwrap();
        assert!(m.contains("'y'"), "{m}");
    }

    #[test]
    fn contradiction_ignores_non_simple_constraints() {
        let ignored = vec![
            bin(BinOpKind::Gt, id("x"), id("y")),
            bin(BinOpKind::Gt, bin(BinOpKind::Add, id("x"), int(1)), int(5)),
            bin(BinOpKind::Ne, id("x"), int(5)),
            bin(BinOpKind::Ne, int(5), id("x")),
            bin(BinOpKind::Gt, int(5), int(3)),
            bin(BinOpKind::Add, int(5), id("x")),
            bin(BinOpKind::Sub, int(5), id("x")),
            bin(BinOpKind::Gt, id("x"), Expr::Literal(Literal::Bool(true))),
            id("x"),
        ];
        for e in ignored {
            let r = contra_of(
                vec![e.clone(), bin(BinOpKind::Gt, id("x"), int(100))],
                vec![bin(BinOpKind::Lt, id("x"), int(200))],
            );
            assert!(r.is_none(), "{e:?} must be ignored");
        }
        assert!(contra_of(
            vec![
                bin(BinOpKind::Ne, id("x"), int(5)),
                bin(BinOpKind::Eq, id("x"), int(5))
            ],
            vec![]
        )
        .is_none());
        let or = bin(
            BinOpKind::Or,
            bin(BinOpKind::Gt, id("x"), int(5)),
            bin(BinOpKind::Lt, id("x"), int(3)),
        );
        assert!(contra_of(vec![or], vec![]).is_none());
    }

    #[test]
    fn contradiction_float_literals() {
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("p"), flt(0.5))],
            vec![bin(BinOpKind::Lt, id("p"), flt(0.25))]
        )
        .is_some());
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("p"), flt(0.25))],
            vec![bin(BinOpKind::Lt, id("p"), flt(0.5))]
        )
        .is_none());
        assert!(contra_of(
            vec![bin(BinOpKind::Ge, id("p"), flt(2.0))],
            vec![bin(BinOpKind::Le, id("p"), int(2))]
        )
        .is_none());
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("p"), flt(2.0))],
            vec![bin(BinOpKind::Le, id("p"), int(2))]
        )
        .is_some());
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("p"), flt(f64::NAN))],
            vec![bin(BinOpKind::Lt, id("p"), flt(f64::INFINITY))]
        )
        .is_none());
        assert!(contra_of(
            vec![bin(BinOpKind::Gt, id("p"), flt(f64::INFINITY))],
            vec![bin(BinOpKind::Lt, id("p"), int(0))]
        )
        .is_none());
    }

    #[test]
    fn simple_bound_extraction() {
        let sb = SmtBridgeChecker::simple_bound;
        let x = "x".to_string();
        assert_eq!(
            sb(&bin(BinOpKind::Ge, id("x"), int(3))),
            Some((x.clone(), BinOpKind::Ge, 3.0))
        );
        assert_eq!(
            sb(&bin(BinOpKind::Lt, int(3), id("x"))),
            Some((x.clone(), BinOpKind::Gt, 3.0))
        );
        assert_eq!(
            sb(&bin(BinOpKind::Le, int(3), id("x"))),
            Some((x.clone(), BinOpKind::Ge, 3.0))
        );
        assert_eq!(
            sb(&bin(BinOpKind::Gt, int(3), id("x"))),
            Some((x.clone(), BinOpKind::Lt, 3.0))
        );
        assert_eq!(
            sb(&bin(BinOpKind::Ge, int(3), id("x"))),
            Some((x.clone(), BinOpKind::Le, 3.0))
        );
        assert_eq!(
            sb(&bin(BinOpKind::Eq, int(3), id("x"))),
            Some((x.clone(), BinOpKind::Eq, 3.0))
        );
        assert_eq!(sb(&bin(BinOpKind::Ne, int(3), id("x"))), None);
        assert_eq!(sb(&int(3)), None);
        assert_eq!(
            sb(&bin(BinOpKind::Gt, id("x"), flt(1.5))),
            Some((x, BinOpKind::Gt, 1.5))
        );
    }

    #[test]
    fn conjuncts_flattens_only_and() {
        let e = bin(
            BinOpKind::And,
            bin(BinOpKind::And, id("a"), id("b")),
            bin(BinOpKind::Or, id("c"), id("d")),
        );
        let mut out = Vec::new();
        SmtBridgeChecker::conjuncts(&e, &mut out);
        assert_eq!(out.len(), 3);
        assert!(matches!(out[0], Expr::Ident(n) if n == "a"));
        assert!(matches!(out[1], Expr::Ident(n) if n == "b"));
        assert!(matches!(
            out[2],
            Expr::BinOp {
                op: BinOpKind::Or,
                ..
            }
        ));
    }

    #[test]
    fn check_fn_reports_counterexample_only_for_contradictions() {
        let bad = fn_with(
            "Int -> Int",
            vec![bin(BinOpKind::Gt, id("x"), int(5))],
            vec![bin(BinOpKind::Lt, id("x"), int(3))],
        );
        let v = SmtBridgeChecker::check_fn(&bad).unwrap();
        assert!(
            matches!(v.status, SmtStatus::Counterexample(_)),
            "{:?}",
            v.status
        );
        let ok = fn_with(
            "Int -> Int",
            vec![bin(BinOpKind::Gt, id("x"), int(3))],
            vec![bin(BinOpKind::Gt, id("x"), int(2))],
        );
        let v = SmtBridgeChecker::check_fn(&ok).unwrap();
        assert!(
            !matches!(v.status, SmtStatus::Counterexample(_)),
            "{:?}",
            v.status
        );
    }
}
