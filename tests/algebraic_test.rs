/// Tests for M21 — Algebraic Operation Properties.
use loom::{compile, compile_openapi, compile_typescript};

// ── AlgebraicChecker tests ────────────────────────────────────────────────────

#[test]
fn idempotent_on_effectful_fn_is_valid() {
    let src = r#"
module Demo
  fn update_status @idempotent :: Int -> Effect<[DB], String>
    id
  end
end
"#;
    compile(src).expect("@idempotent on effectful fn should be valid");
}

#[test]
fn commutative_on_two_param_fn_is_valid() {
    let src = r#"
module Demo
  fn add @commutative :: Int -> Int -> Int
    todo
  end
end
"#;
    compile(src).expect("@commutative with 2 params should be valid");
}

#[test]
fn commutative_on_one_param_fn_is_error() {
    let src = r#"
module Demo
  fn not_commutative @commutative :: Int -> Int
    todo
  end
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "1-param @commutative should error");
    let errs = result.unwrap_err();
    let msg = errs
        .iter()
        .map(|e| format!("{e}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("commutative requires at least 2 parameters"),
        "expected commutative error, got: {msg}"
    );
}

#[test]
fn at_most_once_and_exactly_once_together_is_error() {
    let src = r#"
module Demo
  fn send_email @at-most-once @exactly-once :: String -> Effect<[IO], String>
    addr
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "@at-most-once + @exactly-once should error"
    );
    let errs = result.unwrap_err();
    let msg = errs
        .iter()
        .map(|e| format!("{e}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("conflicting multiplicity annotations"),
        "expected conflicting multiplicity error, got: {msg}"
    );
}

#[test]
fn idempotent_and_exactly_once_together_is_error() {
    let src = r#"
module Demo
  fn pay @idempotent @exactly-once :: Int -> Effect<[Payment], String>
    amount
  end
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "@idempotent + @exactly-once should error");
    let errs = result.unwrap_err();
    let msg = errs
        .iter()
        .map(|e| format!("{e}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("idempotent and exactly-once are contradictory"),
        "expected contradictory error, got: {msg}"
    );
}

#[test]
fn exactly_once_on_non_effectful_fn_is_error() {
    let src = r#"
module Demo
  fn pure_fn @exactly-once :: Int -> Int
    n
  end
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "@exactly-once on pure fn should error");
    let errs = result.unwrap_err();
    let msg = errs
        .iter()
        .map(|e| format!("{e}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("exactly-once requires an effectful function"),
        "expected exactly-once effectful error, got: {msg}"
    );
}

// ── OpenAPI emission tests ────────────────────────────────────────────────────

#[test]
fn idempotent_fn_emits_x_idempotent_in_openapi() {
    let src = r#"
module Orders
  fn submit_order @idempotent :: Int -> Effect<[DB], String>
    id
  end
end
"#;
    let out = compile_openapi(src).expect("should compile to OpenAPI");
    assert!(
        out.contains("\"x-idempotent\": true"),
        "expected x-idempotent in OpenAPI, got:\n{out}"
    );
}

#[test]
fn idempotent_post_fn_gets_promoted_to_put_in_openapi() {
    let src = r#"
module Orders
  fn submit_order @idempotent :: Int -> Effect<[DB], String>
    id
  end
end
"#;
    let out = compile_openapi(src).expect("should compile to OpenAPI");
    // submit_order would normally infer as POST but @idempotent should force PUT
    assert!(
        out.contains("\"put\""),
        "expected PUT (not POST) for @idempotent fn, got:\n{out}"
    );
    assert!(
        !out.contains("\"post\""),
        "@idempotent fn should not emit POST, got:\n{out}"
    );
}

#[test]
fn at_most_once_emits_retry_policy_never_in_openapi() {
    let src = r#"
module Payments
  fn charge_card @at-most-once :: Int -> Effect<[IO], String>
    amount
  end
end
"#;
    let out = compile_openapi(src).expect("should compile to OpenAPI");
    assert!(
        out.contains("\"x-at-most-once\": true"),
        "expected x-at-most-once in OpenAPI, got:\n{out}"
    );
    assert!(
        out.contains("\"x-retry-policy\": \"never\""),
        "expected x-retry-policy: never in OpenAPI, got:\n{out}"
    );
}

#[test]
fn commutative_emits_x_commutative_in_openapi() {
    let src = r#"
module Math
  fn add_values @commutative :: Int -> Int -> Effect<[DB], Int>
    todo
  end
end
"#;
    let out = compile_openapi(src).expect("should compile to OpenAPI");
    assert!(
        out.contains("\"x-commutative\": true"),
        "expected x-commutative in OpenAPI, got:\n{out}"
    );
}

// ── Rust codegen doc comment tests ───────────────────────────────────────────

#[test]
fn idempotent_emits_doc_comment_in_rust() {
    let src = r#"
module Demo
  fn retry_safe @idempotent :: Int -> Effect<[DB], Int>
    n
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("@idempotent") && out.contains("safe to retry"),
        "expected '@idempotent — safe to retry' doc comment in Rust, got:\n{out}"
    );
}

#[test]
fn commutative_emits_doc_comment_in_rust() {
    let src = r#"
module Demo
  fn combine @commutative :: Int -> Int -> Int
    todo
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("@commutative") && out.contains("argument order does not matter"),
        "expected commutative doc comment in Rust, got:\n{out}"
    );
}

// ── TypeScript codegen JSDoc tests ────────────────────────────────────────────

#[test]
fn idempotent_emits_jsdoc_in_typescript() {
    let src = r#"
module Demo
  fn safe_update @idempotent :: Int -> Effect<[DB], Int>
    n
  end
end
"#;
    let out = compile_typescript(src).expect("should compile to TypeScript");
    assert!(
        out.contains("@idempotent") && out.contains("safe to retry"),
        "expected '@idempotent — safe to retry' JSDoc in TypeScript, got:\n{out}"
    );
}

#[test]
fn associative_emits_x_associative_in_openapi() {
    let src = r#"
module Math
  fn fold_values @associative :: Int -> Int -> Effect<[DB], Int>
    todo
  end
end
"#;
    let out = compile_openapi(src).expect("should compile to OpenAPI");
    assert!(
        out.contains("\"x-associative\": true"),
        "expected x-associative in OpenAPI, got:\n{out}"
    );
}
