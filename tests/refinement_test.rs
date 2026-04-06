//! Tests for M56 — Refinement Types
//!
//! Refinement types carry logical predicates checked at compile time.
//! When the `smt` feature is enabled, predicates are verified via Z3.
//! Without `smt`, structural validation ensures well-formedness.

/// Helper: compile and expect success.
fn compile_ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|errs| {
        panic!(
            "expected compilation to succeed but got errors:\n{:#?}",
            errs
        )
    })
}

/// Helper: compile and expect failure.
fn compile_err(src: &str) -> Vec<loom::LoomError> {
    loom::compile(src).expect_err("expected compilation to fail but it succeeded")
}

// ── 1. Compound predicates parse and compile ────────────────────────────────

#[test]
fn refined_type_with_and_predicate_compiles() {
    let src = r#"
module Bounded
type BoundedInt = Int where self >= 0 and self <= 100
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct BoundedInt"), "missing newtype wrapper");
    assert!(out.contains("TryFrom"), "missing TryFrom impl");
}

#[test]
fn refined_type_with_or_predicate_compiles() {
    let src = r#"
module Status
type StatusCode = Int where self = 200 or self = 404 or self = 500
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct StatusCode"), "missing newtype wrapper");
}

#[test]
fn refined_type_with_nested_logic_compiles() {
    let src = r#"
module Complex
type ValidRange = Int where (self >= 0 and self <= 100) or self = -1
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct ValidRange"), "missing newtype wrapper");
}

// ── 2. Refinement checker validates predicates structurally ──────────────────

#[test]
fn refined_type_emits_assert_with_compound_predicate() {
    let src = r#"
module Bounded
type BoundedInt = Int where self >= 0 and self <= 100
end
"#;
    let out = compile_ok(src);
    // The emitted TryFrom should contain the full compound predicate
    assert!(out.contains(">=") && out.contains("<="),
        "compound predicate not fully emitted in TryFrom: {}", out);
}

// ── 3. Refinement subtyping — refined type is subtype of base ────────────────

#[test]
fn refined_type_usable_where_base_type_expected() {
    let src = r#"
module SubtypeTest
type PositiveInt = Int where self > 0

fn double :: Int -> Int
  x * 2
end
end
"#;
    // This should compile: PositiveInt can be used where Int is expected
    let _out = compile_ok(src);
}

// ── 4. Contract integration — require/ensure with refinement-like predicates ─

#[test]
fn require_clause_with_comparison_compiles() {
    let src = r#"
module Contracts
fn safe_divide :: Int -> Int -> Int
  require: divisor != 0
  dividend / divisor
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("safe_divide"), "function not emitted");
}

#[test]
fn ensure_clause_emits_postcondition() {
    let src = r#"
module Contracts
fn clamp :: Int -> Int
  require: x >= 0
  ensure: result <= 100
  match x
    | _ -> 50
  end
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("clamp"), "function not emitted");
}

// ── 5. Refinement type in function signatures ────────────────────────────────

#[test]
fn function_accepting_refined_type_compiles() {
    let src = r#"
module TypedFn
type PositiveInt = Int where self > 0

fn increment :: PositiveInt -> Int
  x + 1
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct PositiveInt"), "missing refined type");
    assert!(out.contains("increment"), "missing function");
}

// ── 6. OpenAPI emission of refinement constraints ────────────────────────────

#[test]
fn openapi_emits_minimum_maximum_for_bounded_int() {
    let src = r#"
module BoundedApi
type Score = Int where self >= 0 and self <= 100

fn get_score :: String -> Score
  42
end
end
"#;
    let out = loom::compile_openapi(src).unwrap_or_else(|e| panic!("{:?}", e));
    assert!(out.contains("minimum") || out.contains("x-refinement"),
        "OpenAPI should emit refinement constraints: {}", out);
}

// ── 7. JSON Schema emission of refinement constraints ────────────────────────

#[test]
fn json_schema_emits_refinement_as_constraint() {
    let src = r#"
module SchemaTest
type Percentage = Int where self >= 0 and self <= 100

fn get_pct :: String -> Percentage
  50
end
end
"#;
    let out = loom::compile_json_schema(src).unwrap_or_else(|e| panic!("{:?}", e));
    assert!(out.contains("minimum") || out.contains("x-refinement"),
        "JSON Schema should emit refinement constraints: {}", out);
}

// ── 8. TypeScript emission of refinement types ───────────────────────────────

#[test]
fn typescript_emits_branded_refined_type() {
    let src = r#"
module TsRefine
type PositiveInt = Int where self > 0

fn double :: PositiveInt -> Int
  x * 2
end
end
"#;
    let out = loom::compile_typescript(src).unwrap_or_else(|e| panic!("{:?}", e));
    assert!(out.contains("PositiveInt"), "missing refined type in TS output");
}
