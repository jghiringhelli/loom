//! M14 tests — `invariant:` declarations + consequence tiers (Defended GS property).
//!
//! Verifies:
//! - `invariant name :: cond` emits `debug_assert!` inside `_check_invariants()`
//! - Multiple invariants are all emitted
//! - Module with no invariants compiles normally
//! - Consequence tier doc comments emitted for `Effect<[IO@reversible]>`
//! - `@irreversible` callee from `@reversible` function → EffectError
//! - `@pure` annotation on function calling effectful fn → EffectError
//! - E2E: module with invariant compiles to valid Rust

use loom::checker::effects::EffectChecker;
use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse_emit(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

fn effect_check(src: &str) -> Result<(), Vec<loom::error::LoomError>> {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    EffectChecker::new().check(&module)
}

// ── Invariant emission ────────────────────────────────────────────────────────

#[test]
fn invariant_emits_debug_assert() {
    let src = r#"module Account
invariant non_negative :: amount >= 0
fn get :: Int
  42
end
end"#;
    let out = parse_emit(src);
    assert!(
        out.contains("_check_invariants"),
        "expected _check_invariants fn in:\n{out}"
    );
    assert!(
        out.contains("debug_assert!((amount >= 0)"),
        "expected debug_assert! for invariant in:\n{out}"
    );
    assert!(
        out.contains("\"invariant 'non_negative' violated\""),
        "expected invariant name in panic message:\n{out}"
    );
}

#[test]
fn multiple_invariants_all_emitted() {
    let src = r#"module Wallet
invariant non_negative :: balance >= 0
invariant under_limit :: balance <= 10000
fn get :: Int
  0
end
end"#;
    let out = parse_emit(src);
    assert!(out.contains("balance >= 0"), "first invariant missing");
    assert!(out.contains("balance <= 10000"), "second invariant missing");
}

#[test]
fn no_invariants_compiles_normally() {
    let src = r#"module Simple
fn id :: Int -> Int
  x
end
end"#;
    let out = parse_emit(src);
    assert!(
        !out.contains("_check_invariants"),
        "unexpected invariant fn"
    );
    assert!(out.contains("pub fn id"), "expected fn in output");
}

#[test]
fn cfg_debug_assertions_wraps_check_invariants() {
    let src = r#"module Guard
invariant positive :: n > 0
fn get :: Int
  1
end
end"#;
    let out = parse_emit(src);
    assert!(
        out.contains("#[cfg(debug_assertions)]"),
        "expected #[cfg(debug_assertions)] around _check_invariants:\n{out}"
    );
}

// ── Consequence tier doc comments ─────────────────────────────────────────────

#[test]
fn effect_tier_emits_doc_comment() {
    let src = r#"module PaymentService
fn charge :: Effect<[IO@irreversible], Int>
  42
end
end"#;
    let out = parse_emit(src);
    assert!(
        out.contains("// effect-tier: IO -> irreversible"),
        "expected tier doc comment in:\n{out}"
    );
}

#[test]
fn reversible_tier_emits_doc_comment() {
    let src = r#"module DB
fn save :: Effect<[DB@reversible], Int>
  1
end
end"#;
    let out = parse_emit(src);
    assert!(
        out.contains("// effect-tier: DB -> reversible"),
        "expected reversible tier comment in:\n{out}"
    );
}

// ── @pure annotation enforcement ─────────────────────────────────────────────

#[test]
fn pure_annotation_calling_effectful_fn_is_error() {
    // save is effectful (IO); compute calls save and is annotated @pure
    let src = r#"module M
fn save :: Effect<[IO], Int>
  42
end
fn compute
  @pure
  :: Int
  save()
end
end"#;
    let result = effect_check(src);
    assert!(
        result.is_err(),
        "expected error when @pure calls effectful fn"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| format!("{:?}", e).contains("pure")),
        "expected a 'pure' error, got: {:?}",
        errs
    );
}

// ── Consequence tier enforcement ──────────────────────────────────────────────

#[test]
fn irreversible_callee_from_reversible_caller_is_error() {
    // send_email is irreversible; save_draft is reversible; save_draft calls send_email
    let src = r#"module M
fn send_email :: Effect<[IO@irreversible], Int>
  42
end
fn save_draft :: Effect<[IO@reversible], Int>
  send_email()
end
end"#;
    let result = effect_check(src);
    assert!(result.is_err(), "expected tier violation error");
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| {
            let s = format!("{:?}", e);
            s.contains("irreversible") || s.contains("tier")
        }),
        "expected tier error, got: {:?}",
        errs
    );
}

#[test]
fn same_tier_callee_is_allowed() {
    // Both irreversible — no error
    let src = r#"module M
fn send_email :: Effect<[IO@irreversible], Int>
  42
end
fn notify :: Effect<[IO@irreversible], Int>
  send_email()
end
end"#;
    let result = effect_check(src);
    assert!(
        result.is_ok(),
        "same-tier should be allowed, got: {:?}",
        result
    );
}

// ── E2E: module with invariant compiles to valid Rust ────────────────────────

#[test]
fn e2e_invariant_module_compiles_to_valid_rust() {
    // Use a literal invariant (true) so _check_invariants() compiles standalone.
    let src = r#"module Balance
invariant always_valid :: true
fn get_amount :: Int
  100
end
end"#;
    let rust_src = parse_emit(src);

    // Wrap in a main so rustc is happy
    let full_src = format!("{}\nfn main() {{}}", rust_src);
    let tmp_dir = std::env::temp_dir();
    let rs_path = tmp_dir.join("loom_m14_e2e.rs");
    let bin_path = tmp_dir.join("loom_m14_e2e");
    std::fs::write(&rs_path, &full_src).unwrap();
    let status = std::process::Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&bin_path)
        .arg(&rs_path)
        .status();
    match status {
        Ok(s) => assert!(s.success(), "rustc failed on:\n{full_src}"),
        Err(e) => eprintln!("rustc not available: {e} — skipping E2E"),
    }
    let _ = std::fs::remove_file(&rs_path);
    let _ = std::fs::remove_file(&bin_path);
}
