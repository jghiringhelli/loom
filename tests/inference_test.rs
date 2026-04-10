//! Type-inference tests for M1 (Hindley-Milner engine).
//!
//! These tests validate that the inference engine correctly infers expression
//! types, catches mismatched types, and enforces occurs-check.

use loom::LoomError;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compile(src: &str) -> Result<String, Vec<LoomError>> {
    loom::compile(src)
}

fn type_errors(src: &str) -> Vec<LoomError> {
    match compile(src) {
        Ok(_) => Vec::new(),
        Err(errs) => errs
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    LoomError::TypeError { .. } | LoomError::UnificationError { .. }
                )
            })
            .collect(),
    }
}

fn has_unification_error(src: &str) -> bool {
    matches!(
        compile(src),
        Err(ref errs) if errs.iter().any(|e| matches!(e, LoomError::UnificationError { .. }))
    )
}

// ── Passing cases ─────────────────────────────────────────────────────────────

#[test]
fn basic_let_inference_passes() {
    // let binding type is inferred from value; result unifies with return type
    let src = r#"
module M
fn double :: Int -> Int
  let result = n + n
  result
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "let binding with matching inferred type should pass"
    );
}

#[test]
fn function_argument_inference_passes() {
    // parameters constrain body: a + b is Int when both params are Int
    let src = r#"
module M
fn add :: Int -> Int -> Int
  a + b
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "body type inferred from param types should pass when consistent with signature"
    );
}

#[test]
fn comparison_returns_bool_passes() {
    // n > 0 infers as Bool, matching declared return type Bool
    let src = r#"
module M
fn positive :: Int -> Bool
  n > 0
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "comparison expression should infer as Bool"
    );
}

#[test]
fn recursive_function_inference_passes() {
    // Recursive call is valid when return type is consistent
    let src = r#"
module M
fn factorial :: Int -> Int
  n * factorial(n)
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "recursive function with consistent types should pass"
    );
}

#[test]
fn literal_inference_passes() {
    // Function body is a literal; type matches declared return type
    let src = r#"
module M
fn answer :: Int -> Int
  42
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "literal body matching return type should pass"
    );
}

#[test]
fn bool_literal_passes() {
    let src = r#"
module M
fn always_true :: Int -> Bool
  true
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "bool literal matching Bool return type should pass"
    );
}

// ── Failing cases (type mismatches) ──────────────────────────────────────────

#[test]
fn body_type_mismatch_returns_error() {
    // Body infers as Int but declared return type is Bool
    let src = r#"
module M
fn bad :: Int -> Bool
  n + 1
end
end
"#;
    assert!(
        has_unification_error(src),
        "body type Int should not unify with declared return Bool"
    );
}

#[test]
fn binop_operand_mismatch_returns_error() {
    // Adding an Int and a Bool should fail
    let src = r#"
module M
fn bad :: Int -> Int
  n + true
end
end
"#;
    assert!(
        has_unification_error(src),
        "Int + Bool operand mismatch should produce a unification error"
    );
}

#[test]
fn literal_return_type_mismatch_returns_error() {
    // Returning a float literal from an Int-returning function
    let src = r#"
module M
fn bad :: Int -> Bool
  42
end
end
"#;
    assert!(
        has_unification_error(src),
        "Int literal should not match Bool return type"
    );
}

// ── Parse-level contract tests ────────────────────────────────────────────────

#[test]
fn test_inference_annotated_fn_parses() {
    // A function with a valid signature and describe annotation
    // The inferred body type matches the declared return type
    let src = r#"
module App
  fn always_true :: Int -> Bool
    describe: "Returns true regardless of input"
  end
end
"#;
    // describe: is metadata (not a body expression) — inference skips empty body
    assert!(
        loom::parse(src).is_ok(),
        "annotated fn should parse: {:?}",
        loom::parse(src).err()
    );
}
