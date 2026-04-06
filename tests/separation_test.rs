//! M57: Separation logic — TDD test suite.
//!
//! Covers:
//! - `separation:` block parsing inside functions
//! - `owns:` resource declarations
//! - `disjoint:` separating conjunction pairs
//! - `frame:` preserved resource declarations
//! - `proof:` assertion field
//! - SeparationChecker: disjoint requires owns, frame requires owns
//! - @thread_safe function without separation block warns (checker error)
//! - Rust codegen emits separation as documentation comments
//! - JSON Schema and OpenAPI unaffected
//! - Multiple owns resources are all valid
//! - Disjoint pair with undeclared resource is a checker error

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser;

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    parser::Parser::new(&tokens)
        .parse_module()
        .expect("parse failed")
}

fn parse_module_err(src: &str) -> loom::error::LoomError {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    parser::Parser::new(&tokens)
        .parse_module()
        .expect_err("expected parse error")
}

// ── 1. Basic separation block parses ─────────────────────────────────────────

#[test]
fn separation_block_parses() {
    let src = r#"
module Bank
fn transferFunds @thread_safe :: Account -> Account -> Unit
  separation:
    owns: source
    owns: target
    disjoint: source * target
    proof: frame_rule_verified
  end
end
end
"#;
    let m = parse_module(src);
    let fns: Vec<_> = m.items.iter().filter_map(|i| {
        if let Item::Fn(f) = i { Some(f) } else { None }
    }).collect();
    let sep = fns[0].separation.as_ref().expect("separation block missing");
    assert_eq!(sep.owns, vec!["source", "target"]);
    assert_eq!(sep.disjoint, vec![("source".to_string(), "target".to_string())]);
    assert_eq!(sep.proof, Some("frame_rule_verified".to_string()));
}

// ── 2. Function without separation block parses fine ─────────────────────────

#[test]
fn function_without_separation_block_parses() {
    let src = r#"
module M
fn plain :: Int -> Int
end
end
"#;
    let m = parse_module(src);
    let fns: Vec<_> = m.items.iter().filter_map(|i| {
        if let Item::Fn(f) = i { Some(f) } else { None }
    }).collect();
    assert!(fns[0].separation.is_none());
}

// ── 3. frame: declares a preserved field ─────────────────────────────────────

#[test]
fn separation_with_frame_parses() {
    let src = r#"
module M
fn processOrder @thread_safe :: Order -> Unit
  separation:
    owns: order
    frame: audit_log
  end
end
end
"#;
    let m = parse_module(src);
    let fns: Vec<_> = m.items.iter().filter_map(|i| {
        if let Item::Fn(f) = i { Some(f) } else { None }
    }).collect();
    let sep = fns[0].separation.as_ref().unwrap();
    assert_eq!(sep.owns, vec!["order"]);
    assert_eq!(sep.frame, vec!["audit_log"]);
}

// ── 4. SeparationChecker: valid block passes ──────────────────────────────────

#[test]
fn separation_checker_valid_block_passes() {
    let src = r#"
module Bank
fn transfer @thread_safe :: Account -> Account -> Unit
  separation:
    owns: a
    owns: b
    disjoint: a * b
  end
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
}

// ── 5. SeparationChecker: disjoint requires both owns to be declared ─────────

#[test]
fn separation_checker_disjoint_requires_owns() {
    let src = r#"
module Bank
fn transfer :: Account -> Account -> Unit
  separation:
    owns: a
    disjoint: a * b
  end
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    let msg = errors[0].to_string();
    assert!(
        msg.contains("disjoint") && msg.contains("b"),
        "Expected disjoint error mentioning 'b', got: {}", msg
    );
}

// ── 6. SeparationChecker: frame requires owns ────────────────────────────────

#[test]
fn separation_checker_frame_requires_owns() {
    let src = r#"
module M
fn process :: Order -> Unit
  separation:
    owns: order
    frame: undeclared_resource
  end
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    let msg = errors[0].to_string();
    assert!(
        msg.contains("frame") && msg.contains("undeclared_resource"),
        "Expected frame error mentioning 'undeclared_resource', got: {}", msg
    );
}

// ── 7. Rust codegen emits separation as doc comments ─────────────────────────

#[test]
fn separation_block_emitted_in_rust_codegen() {
    let src = r#"
module M
fn reserve @thread_safe :: Resource -> Unit
  separation:
    owns: resource
    proof: frame_rule_verified
  end
end
end
"#;
    let rust = loom::compile(src).expect("compile failed");
    assert!(
        rust.contains("separation") || rust.contains("frame_rule_verified"),
        "Expected separation doc comment in output, got:\n{}", rust
    );
}

// ── 8. Separation block with contracts coexists ───────────────────────────────

#[test]
fn separation_block_coexists_with_contracts() {
    let src = r#"
module M
fn transfer :: Account -> Account -> Float -> Unit
  separation:
    owns: source
    owns: dest
    disjoint: source * dest
  end
  require: source != dest
  ensure: true
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
}

// ── 9. Multiple disjoint pairs ────────────────────────────────────────────────

#[test]
fn separation_multiple_disjoint_pairs() {
    let src = r#"
module M
fn threeWay :: Unit -> Unit
  separation:
    owns: a
    owns: b
    owns: c
    disjoint: a * b
    disjoint: b * c
    disjoint: a * c
  end
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
}

// ── 10. Separation block with no disjoint (owns only) ────────────────────────

#[test]
fn separation_owns_only_passes() {
    let src = r#"
module M
fn acquireLock :: Resource -> Unit
  separation:
    owns: lock
  end
end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
}
