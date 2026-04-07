// M100: SMT Contract Verification Bridge — Hoare (1969) → Dijkstra WP → Z3.
//
// Tests for the SmtBridgeChecker: contract translation, status, lineage.
// Translation tests exercise the pure-Rust SMT-LIB2 translator independently
// of any Z3 installation.

use loom::ast::*;
use loom::checker::SmtBridgeChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// 1. SmtBridgeChecker::check returns results for functions with contracts.
#[test]
fn test_m100_smt_checker_exists() {
    let src = r#"
module Checks
  fn positive_input :: Int -> Int
    require: x > 0
    ensure: result >= 0
  end
end
"#;
    let module = parse(src);
    let results = SmtBridgeChecker::check(&module.items);
    assert!(!results.is_empty(), "should return at least one verification result");
    assert_eq!(results[0].function, "positive_input");
}

// 2. Without the `smt` feature, every result has status Skipped.
#[test]
fn test_m100_simple_contract_skipped_without_z3() {
    let src = r#"
module NoZ3
  fn double :: Int -> Int
    require: x >= 0
    ensure: result >= 0
  end
end
"#;
    let module = parse(src);
    let results = SmtBridgeChecker::check(&module.items);
    assert!(!results.is_empty());
    // Without smt feature, all contracts must be Skipped.
    for result in &results {
        assert_eq!(
            result.status,
            SmtStatus::Skipped,
            "expected Skipped for '{}' without Z3 feature, got {:?}",
            result.function,
            result.status
        );
    }
}

// 3. require: x > 0 translates to the correct SMT-LIB2 string.
#[test]
fn test_m100_positive_precondition_translates() {
    let translated = SmtBridgeChecker::translate_expr("x > 0");
    assert_eq!(
        translated, "(> x 0)",
        "x > 0 should translate to (> x 0), got: {translated}"
    );
}

// 4. ensure: result >= 0 translates correctly.
#[test]
fn test_m100_postcondition_translates() {
    let translated = SmtBridgeChecker::translate_expr("result >= 0");
    assert_eq!(
        translated, "(>= result 0)",
        "result >= 0 should translate to (>= result 0), got: {translated}"
    );
}

// 5. Arithmetic operators translate: x + y, x * y, x - y.
#[test]
fn test_m100_arithmetic_translates() {
    let add = SmtBridgeChecker::translate_expr("x + y");
    assert_eq!(add, "(+ x y)", "x + y → (+ x y), got: {add}");

    let mul = SmtBridgeChecker::translate_expr("x * y");
    assert_eq!(mul, "(* x y)", "x * y → (* x y), got: {mul}");

    let sub = SmtBridgeChecker::translate_expr("x - y");
    assert_eq!(sub, "(- x y)", "x - y → (- x y), got: {sub}");
}

// 6. detect_contradiction: require: x > 5 + ensure: x < 3 → contradictory.
#[test]
fn test_m100_contradiction_detected() {
    // Precondition says x > 5; postcondition says x < 3.
    // Since 5 >= 3, the spec is impossible — detect_contradiction returns true.
    let is_contradiction = SmtBridgeChecker::detect_contradiction("(> x 5)", "(< x 3)");
    assert!(
        is_contradiction,
        "require: x > 5 + ensure: x < 3 must be detected as contradictory"
    );

    // Non-contradictory: x > 0, result >= 0 — should return false.
    let not_contradiction = SmtBridgeChecker::detect_contradiction("(> x 0)", "(>= result 0)");
    assert!(
        !not_contradiction,
        "require: x > 0 + ensure: result >= 0 should not be detected as contradictory"
    );
}

// 7. Functions without contracts produce no SMT results.
#[test]
fn test_m100_fn_without_contracts_skipped() {
    let src = r#"
module NoContracts
  fn identity :: Int -> Int
  end
end
"#;
    let module = parse(src);
    let results = SmtBridgeChecker::check(&module.items);
    assert!(
        results.is_empty(),
        "function with no require:/ensure: should produce no SMT results, got: {:?}",
        results
    );
}

// 8. The lineage comment must exist in the smt_bridge.rs source — validates academic
//    attribution of Hoare (1969) → Dijkstra (1975) → Dafny (2009) → M100.
#[test]
fn test_m100_hoare_lineage_comment() {
    let source = include_str!("../src/checker/smt_bridge.rs");
    assert!(
        source.contains("Hoare (1969)"),
        "smt_bridge.rs must cite Hoare (1969)"
    );
    assert!(
        source.contains("Dijkstra"),
        "smt_bridge.rs must cite Dijkstra"
    );
    assert!(
        source.contains("Dafny"),
        "smt_bridge.rs must cite Dafny (2009)"
    );
}
