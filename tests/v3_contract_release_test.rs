//! V3 contract release-mode tests.
//!
//! Gate: `RustEmitter::new().with_release_contracts(true)` emits `assert!` instead of
//! `debug_assert!` for `require:` and `ensure:` contracts, so they survive `--release` builds.

use loom::{codegen::rust::RustEmitter, parser::Parser, lexer::Lexer};

fn compile_default(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

fn compile_release(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().with_release_contracts(true).emit(&module)
}

// ── Default mode: debug_assert! ──────────────────────────────────────────────

#[test]
fn default_require_emits_debug_assert() {
    let src = r#"
module M
  fn add :: Int -> Int
    require: n > 0
  end
end
"#;
    let out = compile_default(src);
    assert!(
        out.contains("debug_assert!"),
        "default mode must use debug_assert!, got:\n{}",
        out
    );
    assert!(
        !out.contains("assert!(") || out.contains("debug_assert!("),
        "must not emit bare assert! in default mode"
    );
}

#[test]
fn default_ensure_emits_debug_assert() {
    let src = r#"
module M
  fn positive :: Int -> Int
    ensure: result > 0
    body
      42
    end
  end
end
"#;
    let out = compile_default(src);
    assert!(
        out.contains("debug_assert!"),
        "default ensure must use debug_assert!, got:\n{}",
        out
    );
}

// ── Release mode: assert! ─────────────────────────────────────────────────────

#[test]
fn release_require_emits_assert() {
    let src = r#"
module M
  fn add :: Int -> Int
    require: n > 0
  end
end
"#;
    let out = compile_release(src);
    assert!(
        out.contains("assert!(") && !out.contains("debug_assert!("),
        "release mode must use assert! not debug_assert!, got:\n{}",
        out
    );
}

#[test]
fn release_ensure_emits_assert() {
    let src = r#"
module M
  fn positive :: Int -> Int
    ensure: result > 0
    body
      42
    end
  end
end
"#;
    let out = compile_release(src);
    assert!(
        out.contains("assert!(") && !out.contains("debug_assert!("),
        "release ensure must use assert!, got:\n{}",
        out
    );
}

#[test]
fn release_require_keeps_translated_predicate() {
    let src = r#"
module M
  fn check :: Int -> Int
    require: x > 0 and x < 100
  end
end
"#;
    let out = compile_release(src);
    assert!(
        out.contains("x > 0") && out.contains("x < 100") && out.contains("&&"),
        "predicate translation must still work in release mode, got:\n{}",
        out
    );
}

#[test]
fn release_mode_comment_says_all_builds() {
    let src = r#"
module M
  fn check :: Int -> Int
    require: x >= 0
  end
end
"#;
    let out = compile_release(src);
    assert!(
        out.contains("all builds"),
        "release mode comment must say 'all builds', got:\n{}",
        out
    );
}

#[test]
fn default_mode_comment_says_debug_builds_only() {
    let src = r#"
module M
  fn check :: Int -> Int
    require: x >= 0
  end
end
"#;
    let out = compile_default(src);
    assert!(
        out.contains("debug builds only"),
        "default mode comment must say 'debug builds only', got:\n{}",
        out
    );
}

// ── Builder method chaining ──────────────────────────────────────────────────

#[test]
fn builder_chaining_works() {
    let src = r#"
module M
  fn f :: Int -> Int
    require: n != 0
  end
end
"#;
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    let out = RustEmitter::new().with_release_contracts(true).emit(&module);
    assert!(out.contains("assert!("));
}

#[test]
fn default_and_release_differ_for_same_source() {
    let src = r#"
module M
  fn f :: Int -> Int
    require: x > 0
  end
end
"#;
    let default_out = compile_default(src);
    let release_out = compile_release(src);
    assert_ne!(
        default_out, release_out,
        "default and release outputs must differ"
    );
    assert!(default_out.contains("debug_assert!"));
    assert!(release_out.contains("assert!("));
    assert!(!release_out.contains("debug_assert!"));
}
