//! M13 tests — `describe:` blocks and `@`-annotation support.
//!
//! Verifies that:
//! - `describe: "text"` on a module emits `/// text` before `pub mod`
//! - `describe: "text"` on a fn emits `/// text` before `pub fn`
//! - `@since("v1.0")` emits `/// @since: v1.0`
//! - `@deprecated("use v2")` emits both `/// @deprecated: use v2` and `#[deprecated(note = "use v2")]`
//! - `@decision("reason")` emits `/// @decision: reason`
//! - Missing `describe:` / no annotations compile normally (fields are optional)
//! - Multiple annotations emit in order

use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn compile_src(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

// ── Module-level describe ────────────────────────────────────────────────────

#[test]
fn module_describe_emits_doc_comment() {
    let src = r#"module Greeter
  describe: "A friendly greeting module"
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// A friendly greeting module"),
        "expected doc comment in:\n{out}"
    );
    assert!(out.contains("pub mod greeter"), "expected mod in:\n{out}");
    // Doc comment must appear before the mod declaration
    let doc_pos = out.find("/// A friendly greeting module").unwrap();
    let mod_pos = out.find("pub mod greeter").unwrap();
    assert!(doc_pos < mod_pos, "doc comment must precede pub mod");
}

// ── Function-level describe ──────────────────────────────────────────────────

#[test]
fn fn_describe_emits_doc_comment() {
    let src = r#"module Math
fn add
  describe: "adds two integers"
  :: Int -> Int -> Int
  x + y
end
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// adds two integers"),
        "expected fn doc comment in:\n{out}"
    );
}

// ── @since annotation ────────────────────────────────────────────────────────

#[test]
fn at_since_annotation_emits_doc_comment() {
    let src = r#"module Api
fn get_user
  @since("v1.0")
  :: String -> String
  "user"
end
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// @since: v1.0"),
        "expected @since doc comment in:\n{out}"
    );
}

// ── @deprecated annotation ───────────────────────────────────────────────────

#[test]
fn at_deprecated_emits_doc_comment_and_attribute() {
    let src = r#"module Api
fn old_user
  @deprecated("use get_user_v2")
  :: String -> String
  "user"
end
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// @deprecated: use get_user_v2"),
        "expected @deprecated doc comment in:\n{out}"
    );
    assert!(
        out.contains("#[deprecated(note = \"use get_user_v2\")]"),
        "expected deprecated attribute in:\n{out}"
    );
}

// ── @decision annotation ─────────────────────────────────────────────────────

#[test]
fn at_decision_emits_doc_comment() {
    let src = r#"module Billing
fn calculate
  @decision("use Stripe for payments")
  :: Int -> Int
  x
end
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// @decision: use Stripe for payments"),
        "expected @decision doc comment in:\n{out}"
    );
}

// ── Multiple annotations in order ────────────────────────────────────────────

#[test]
fn multiple_annotations_emitted_in_order() {
    let src = r#"module Api
fn versioned
  @since("v1.0")
  @author("alice")
  :: Int -> Int
  n
end
end"#;
    let out = compile_src(src);
    let since_pos = out.find("/// @since: v1.0").unwrap();
    let author_pos = out.find("/// @author: alice").unwrap();
    assert!(since_pos < author_pos, "annotations must preserve order");
}

// ── Missing describe compiles normally ───────────────────────────────────────

#[test]
fn missing_describe_compiles_normally() {
    let src = r#"module Simple
fn id
  :: Int -> Int
  x
end
end"#;
    let out = compile_src(src);
    assert!(out.contains("pub mod simple"), "expected module in:\n{out}");
    assert!(out.contains("pub fn id"), "expected fn in:\n{out}");
    assert!(
        !out.contains("///"),
        "no doc comments expected when none declared:\n{out}"
    );
}

// ── Module-level @annotation ─────────────────────────────────────────────────

#[test]
fn module_at_annotation_emits_doc_comment() {
    let src = r#"module PaymentService
  @since("v2.0")
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// @since: v2.0"),
        "expected module annotation in:\n{out}"
    );
}

// ── describe + annotations together ──────────────────────────────────────────

#[test]
fn describe_and_annotation_both_emitted() {
    let src = r#"module Docs
fn documented
  describe: "this fn is documented"
  @since("v0.1")
  :: Int -> Int
  n
end
end"#;
    let out = compile_src(src);
    assert!(
        out.contains("/// this fn is documented"),
        "expected describe comment"
    );
    assert!(out.contains("/// @since: v0.1"), "expected @since comment");
    // describe should come before @annotations
    let desc_pos = out.find("/// this fn is documented").unwrap();
    let ann_pos = out.find("/// @since: v0.1").unwrap();
    assert!(desc_pos < ann_pos, "describe must precede annotations");
}
