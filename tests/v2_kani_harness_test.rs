//! V2: Kani integration — generated Rust includes proof harnesses for require:/ensure:.
//!
//! Gate: `#[cfg(kani)] #[kani::proof]` harnesses are emitted alongside functions with
//! contracts. Harnesses use `kani::any()`, `kani::assume()`, and `kani::assert!()`.

use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn emit(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex");
    let module = Parser::new(&tokens).parse_module().expect("parse");
    RustEmitter::new().emit(&module)
}

// ── harness structure ─────────────────────────────────────────────────────────

#[test]
fn kani_harness_emitted_for_fn_with_require() {
    let code = emit("module M\nfn add :: Int -> Int\n  require: n > 0\n  todo\nend\nend");
    assert!(
        code.contains("#[cfg(kani)]"),
        "missing #[cfg(kani)]:\n{code}"
    );
    assert!(
        code.contains("#[kani::proof]"),
        "missing #[kani::proof]:\n{code}"
    );
}

#[test]
fn kani_harness_emitted_for_fn_with_ensure() {
    let code = emit("module M\nfn positive :: Int -> Int\n  ensure: result > 0\n  todo\nend\nend");
    assert!(
        code.contains("#[cfg(kani)]"),
        "missing #[cfg(kani)]:\n{code}"
    );
    assert!(
        code.contains("kani::assert"),
        "missing kani::assert:\n{code}"
    );
}

#[test]
fn kani_harness_not_emitted_for_fn_without_contracts() {
    let code = emit("module M\nfn pure_fn :: Int -> Int\n  todo\nend\nend");
    // No contracts → no harness overhead
    assert!(
        !code.contains("#[kani::proof]"),
        "no harness expected for contract-free fn:\n{code}"
    );
}

#[test]
fn kani_harness_uses_kani_any_for_inputs() {
    let code = emit(
        "module M\nfn clamp :: Int -> Int\n  require: n >= 0\n  ensure: result >= 0\n  todo\nend\nend",
    );
    assert!(
        code.contains("kani::any()"),
        "expected kani::any() for symbolic input:\n{code}"
    );
}

#[test]
fn kani_harness_uses_kani_assume_for_requires() {
    let code = emit("module M\nfn f :: Int -> Int\n  require: x > 0\n  todo\nend\nend");
    assert!(
        code.contains("kani::assume"),
        "expected kani::assume for require:\n{code}"
    );
}

#[test]
fn kani_harness_uses_kani_assert_for_ensures() {
    let code = emit("module M\nfn f :: Int -> Int\n  ensure: result > 0\n  todo\nend\nend");
    assert!(
        code.contains("kani::assert"),
        "expected kani::assert for ensure:\n{code}"
    );
}

#[test]
fn kani_harness_calls_function_under_test() {
    let code = emit(
        "module AddPos\nfn add_pos :: Int -> Int\n  require: n > 0\n  ensure: result > 0\n  todo\nend\nend",
    );
    assert!(
        code.contains("add_pos"),
        "expected function name in harness:\n{code}"
    );
}

// ── operator correctness in kani harness ─────────────────────────────────────

#[test]
fn kani_harness_uses_rust_and_operator() {
    let code = emit("module M\nfn f :: Int -> Int\n  require: x > 0 and x < 100\n  todo\nend\nend");
    // The harness kani::assume should use && not `and`
    assert!(
        !code.contains("kani::assume(x > 0 and"),
        "kani assume must use &&:\n{code}"
    );
}

#[test]
fn kani_harness_uses_double_eq() {
    let code = emit("module M\nfn f :: Int -> Bool\n  require: n = 0\n  todo\nend\nend");
    // = in predicate → == in output (not kani::assume(n = 0))
    assert!(code.contains("=="), "expected == in kani output:\n{code}");
}

// ── V2 gate: full contract round-trip ────────────────────────────────────────

#[test]
fn v2_gate_add_two_positives_harness_complete() {
    let src = "module V2Gate\nfn add_pos :: Int -> Int\n  require: a > 0 and b > 0\n  ensure: result > 0\n  todo\nend\nend";
    let code = emit(src);
    // All Kani structural elements present
    assert!(
        code.contains("#[cfg(kani)]"),
        "V2 gate: #[cfg(kani)]:\n{code}"
    );
    assert!(
        code.contains("#[kani::proof]"),
        "V2 gate: #[kani::proof]:\n{code}"
    );
    assert!(
        code.contains("kani::any()"),
        "V2 gate: kani::any():\n{code}"
    );
    assert!(
        code.contains("kani::assume"),
        "V2 gate: kani::assume:\n{code}"
    );
    assert!(
        code.contains("kani::assert"),
        "V2 gate: kani::assert:\n{code}"
    );
    // No Loom operators in harness
    assert!(
        !code.contains(" and "),
        "V2 gate: no `and` operator:\n{code}"
    );
    assert!(!code.contains(" or "), "V2 gate: no `or` operator:\n{code}");
}
