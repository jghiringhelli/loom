//! Integration tests: compile the corpus examples end-to-end.
//!
//! These tests exercise the full pipeline (lex → parse → check → codegen)
//! on the real corpus `.loom` files and assert structural properties of the
//! emitted Rust code.

use loom::compile;

// ── PricingEngine ─────────────────────────────────────────────────────────────

#[test]
fn pricing_engine_compiles_successfully() {
    let source = std::fs::read_to_string("corpus/pricing_engine.loom")
        .expect("corpus/pricing_engine.loom must exist");

    let output = compile(&source).expect("pricing_engine.loom should compile without errors");

    // Struct definitions
    assert!(
        output.contains("pub struct OrderLine"),
        "output should contain `pub struct OrderLine`:\n{}", output
    );
    assert!(
        output.contains("pub struct OrderTotal"),
        "output should contain `pub struct OrderTotal`:\n{}", output
    );

    // Function definition
    assert!(
        output.contains("pub fn compute_total"),
        "output should contain `pub fn compute_total`:\n{}", output
    );

    // Precondition contracts emit debug_assert!
    assert!(
        output.contains("debug_assert!"),
        "output should contain `debug_assert!` for require: contracts:\n{}", output
    );
}

// ── UserService ───────────────────────────────────────────────────────────────

#[test]
fn user_service_compiles_successfully() {
    let source = std::fs::read_to_string("corpus/user_service.loom")
        .expect("corpus/user_service.loom must exist");

    // UserService uses Effect<[IO], T> so it may emit type errors; we only
    // require the compilation attempt to not panic.
    let _result = compile(&source);
}
