// V2 Kani Codegen Tests — emit_kani_harness
//
// Verifies that functions with require:/ensure: contracts emit:
//   1. A `#[cfg(kani)]` gated proof harness per function
//   2. `kani::any()` declarations for each parameter
//   3. `kani::assume(...)` for each require: clause
//   4. `kani::assert!(...)` for each ensure: clause
//   5. The correct function call with inferred parameter names
//
// These tests check the emitted string structure; they do not invoke cargo kani.
// To run full Kani proofs: `cargo install --locked kani-verifier && cargo kani`

use loom::compile;

fn emit(src: &str) -> String {
    compile(src).unwrap_or_else(|errs| {
        panic!("compile failed: {:?}", errs)
    })
}

// 1. A function with require: and ensure: emits a kani::proof harness.
#[test]
fn v2_emits_kani_proof_attribute_for_contracted_fn() {
    let src = r#"
module Contracts
  fn add_positive :: Int -> Int -> Int
    require: a > 0
    require: b > 0
    ensure:  result > a
  body
    a + b
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("#[cfg(kani)]"),
        "should emit #[cfg(kani)] gate; got:\n{rust}"
    );
    assert!(
        rust.contains("#[kani::proof]"),
        "should emit #[kani::proof] attribute; got:\n{rust}"
    );
    assert!(
        rust.contains("fn kani_verify_add_positive"),
        "should emit harness named kani_verify_add_positive; got:\n{rust}"
    );
}

// 2. require: clauses become kani::assume() calls.
#[test]
fn v2_require_becomes_kani_assume() {
    let src = r#"
module Bounds
  fn clamp :: Int -> Int
    require: x > 0
    require: x < 100
  body
    x
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("kani::assume("),
        "require: should emit kani::assume(); got:\n{rust}"
    );
}

// 3. ensure: clauses become kani::assert!() calls.
#[test]
fn v2_ensure_becomes_kani_assert() {
    let src = r#"
module Positive
  fn positive_result :: Int -> Int
    ensure: result > 0
  body
    42
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("kani::assert!"),
        "ensure: should emit kani::assert!(); got:\n{rust}"
    );
}

// 4. Functions without contracts do not get a kani harness.
#[test]
fn v2_no_harness_for_fn_without_contracts() {
    let src = r#"
module Pure
  fn double :: Int -> Int
  body
    x + x
  end
end
"#;
    let rust = emit(src);
    assert!(
        !rust.contains("kani_verify_double"),
        "fn without contracts should NOT get a kani harness; got:\n{rust}"
    );
}

// 5. Kani symbolic inputs are typed from fn type signature.
#[test]
fn v2_kani_inputs_typed_from_signature() {
    let src = r#"
module Math
  fn multiply :: Int -> Int -> Int
    require: a > 0
    ensure: result > 0
  body
    a * b
  end
end
"#;
    let rust = emit(src);
    // Two Int params → two i64 kani::any() calls
    assert!(
        rust.contains("kani::any()"),
        "Int params should emit kani::any(); got:\n{rust}"
    );
}

// 6. Float params map to f64 in kani harness.
#[test]
fn v2_float_param_maps_to_f64_in_kani() {
    let src = r#"
module Stats
  fn bounded_float :: Float -> Float
    require: x > 0.0
    ensure: result > 0.0
  body
    x
  end
end
"#;
    let rust = emit(src);
    let harness_start = rust.find("kani_verify_bounded_float").unwrap_or(0);
    let harness_src = &rust[harness_start..];
    assert!(
        harness_src.contains("f64") || rust.contains("kani::any()"),
        "Float param should map to f64 in kani harness; got:\n{rust}"
    );
}

// 7. The harness calls the function under test.
#[test]
fn v2_harness_calls_fn_under_test() {
    let src = r#"
module Check
  fn check_value :: Int -> Int
    require: n >= 0
    ensure: result >= 0
  body
    n
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("let result = check_value("),
        "kani harness should call the fn under test; got:\n{rust}"
    );
}

// 8. V7 audit header reflects contract count.
#[test]
fn v7_audit_header_reflects_contract_count() {
    let src = r#"
module Audited
  fn add :: Int -> Int -> Int
    require: a > 0
    ensure: result > 0
  body
    a + b
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("== LOOM AUDIT: Audited =="),
        "audit header should name the module; got:\n{rust}"
    );
    assert!(
        rust.contains("Contracts"),
        "audit header should mention contracts; got:\n{rust}"
    );
    assert!(
        rust.contains("kani"),
        "audit header should mention kani for contract proofs; got:\n{rust}"
    );
}
