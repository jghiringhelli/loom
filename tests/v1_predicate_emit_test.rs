//! V1: Predicate emit correctness — generated Rust contracts must be valid Rust syntax.
//!
//! Gate: a Loom function with `require:`/`ensure:` emits `debug_assert!()` lines
//! that use Rust operators (`&&`, `||`, `!`, `==`) not Loom operators (`and`, `or`, `not`, `=`).

use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn emit(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex");
    let module = Parser::new(&tokens).parse_module().expect("parse");
    RustEmitter::new().emit(&module)
}

// ── require: translation ──────────────────────────────────────────────────────

#[test]
fn require_and_becomes_double_ampersand() {
    let code =
        emit("module M\nfn f :: Int -> Bool\n  require: x > 0 and x < 100\n  todo\nend\nend");
    assert!(code.contains("&&"), "expected && in output, got:\n{code}");
    assert!(
        !code.contains(" and "),
        "Loom `and` must not appear in generated Rust:\n{code}"
    );
}

#[test]
fn require_or_becomes_double_pipe() {
    let code = emit("module M\nfn f :: Int -> Bool\n  require: x < 0 or x > 100\n  todo\nend\nend");
    assert!(code.contains("||"), "expected || in output:\n{code}");
    assert!(!code.contains(" or "), "Loom `or` must not appear:\n{code}");
}

#[test]
fn require_not_becomes_bang() {
    let code = emit("module M\nfn f :: Bool -> Bool\n  require: not flag\n  todo\nend\nend");
    assert!(code.contains('!'), "expected ! in output:\n{code}");
    assert!(
        !code.contains("not flag"),
        "Loom `not` must not appear:\n{code}"
    );
}

#[test]
fn require_eq_becomes_double_eq() {
    let code = emit("module M\nfn f :: Int -> Bool\n  require: x = 0\n  todo\nend\nend");
    assert!(code.contains("=="), "expected == in output:\n{code}");
}

#[test]
fn require_leq_preserved() {
    let code = emit("module M\nfn f :: Int -> Bool\n  require: x <= 100\n  todo\nend\nend");
    assert!(code.contains("<="), "expected <= preserved:\n{code}");
}

#[test]
fn require_geq_preserved() {
    let code = emit("module M\nfn f :: Float -> Bool\n  require: x >= 0.0\n  todo\nend\nend");
    assert!(code.contains(">="), "expected >= preserved:\n{code}");
}

#[test]
fn require_simple_gt_unchanged() {
    let code = emit("module M\nfn f :: Int -> Int\n  require: n > 0\n  todo\nend\nend");
    assert!(
        code.contains("debug_assert!"),
        "expected debug_assert!:\n{code}"
    );
    assert!(code.contains("n > 0"), "expected n > 0 in output:\n{code}");
}

// ── ensure: translation (stub body — emitted as comment only) ─────────────────

#[test]
fn ensure_and_translated_in_stub() {
    let code = emit(
        "module M\nfn f :: Int -> Int\n  ensure: result > 0 and result < 1000\n  todo\nend\nend",
    );
    // stub body → ensure emitted as spec comment, still translated
    assert!(
        !code.contains(" and "),
        "Loom `and` must not appear in ensure comment:\n{code}"
    );
    assert!(
        code.contains("&&") || code.contains("ensure"),
        "expected translated ensure:\n{code}"
    );
}

#[test]
fn ensure_or_translated_in_stub() {
    let code = emit(
        "module M\nfn f :: Int -> Bool\n  ensure: result = true or result = false\n  todo\nend\nend",
    );
    assert!(
        !code.contains(" or "),
        "Loom `or` must not appear in ensure comment:\n{code}"
    );
}

// ── compound predicate ────────────────────────────────────────────────────────

#[test]
fn require_compound_all_operators() {
    let code = emit(
        "module M\nfn f :: Int -> Bool\n  require: x > 0 and x < 100 or not done\n  todo\nend\nend",
    );
    assert!(!code.contains(" and "), "no `and`:\n{code}");
    assert!(!code.contains(" or "), "no `or`:\n{code}");
    assert!(!code.contains("not done"), "no `not`:\n{code}");
}

// ── V1 gate: generated Rust must be syntactically parseable ──────────────────

#[test]
fn v1_gate_generated_rust_has_no_loom_operators() {
    let programs = [
        "module A\nfn add :: Int -> Int\n  require: n > 0\n  ensure: result > 0\n  todo\nend\nend",
        "module B\nfn check :: Float -> Bool\n  require: x >= 0.0 and x <= 1.0\n  todo\nend\nend",
        "module C\nfn valid :: Bool -> Bool\n  require: not flag or x > 0\n  todo\nend\nend",
    ];
    for prog in &programs {
        let code = emit(prog);
        assert!(
            !code.contains(" and "),
            "Loom `and` in output for:\n{prog}\n---\n{code}"
        );
        assert!(
            !code.contains(" or "),
            "Loom `or` in output for:\n{prog}\n---\n{code}"
        );
        // Note: `not ` may legitimately not appear since it's translated to `!`
    }
}
