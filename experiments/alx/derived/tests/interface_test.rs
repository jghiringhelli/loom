//! M16 tests — `import` declarations + explicit `interface` definitions (GS Composable property).
//!
//! Verifies:
//! - `import ModuleName` emits `use super::module_name::*;`
//! - `interface Greeter fn greet :: String -> String end` emits a Rust trait
//! - `implements Greeter` with matching fn → compiles cleanly
//! - `implements Greeter` with missing method → TypeError
//! - Multiple imports emit multiple use lines
//! - Existing `provides`-based tests still pass
//! - E2E: interface + implements compiles to valid Rust

use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::codegen::rust::RustEmitter;
use loom::checker::types::TypeChecker;

fn compile(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

fn type_check(src: &str) -> Result<(), Vec<loom::error::LoomError>> {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    TypeChecker::new().check(&module)
}

// ── import declarations ────────────────────────────────────────────────────────

#[test]
fn import_emits_use_super() {
    let src = r#"module Payments
import MathLib
fn get :: Int
  1
end
end"#;
    let out = compile(src);
    assert!(
        out.contains("use super::math_lib::*;"),
        "expected use super::math_lib::* in:\n{out}"
    );
}

#[test]
fn multiple_imports_emit_multiple_use_lines() {
    let src = r#"module App
import UserService
import PaymentService
fn run :: Int
  1
end
end"#;
    let out = compile(src);
    assert!(out.contains("use super::user_service::*;"), "missing user_service import");
    assert!(out.contains("use super::payment_service::*;"), "missing payment_service import");
}

#[test]
fn no_imports_no_use_lines() {
    let src = r#"module Simple
fn id :: Int -> Int
  x
end
end"#;
    let out = compile(src);
    // Only `use super::*;` from the module wrapper, no extra use statements
    let use_count = out.matches("use super::").count();
    assert_eq!(use_count, 1, "expected only the module use super::*, got:\n{out}");
}

// ── interface definitions ──────────────────────────────────────────────────────

#[test]
fn interface_emits_rust_trait() {
    let src = r#"module M
interface Greeter
  fn greet :: String -> String
end
fn greet :: String -> String
  name
end
end"#;
    let out = compile(src);
    assert!(
        out.contains("pub trait Greeter"),
        "expected pub trait Greeter in:\n{out}"
    );
    assert!(
        out.contains("fn greet("),
        "expected greet method in trait:\n{out}"
    );
}

#[test]
fn interface_with_multiple_methods_emits_all() {
    let src = r#"module M
interface Calculator
  fn add :: Int -> Int -> Int
  fn mul :: Int -> Int -> Int
end
fn add :: Int -> Int -> Int
  x + y
end
fn mul :: Int -> Int -> Int
  x * y
end
end"#;
    let out = compile(src);
    assert!(out.contains("pub trait Calculator"), "missing trait");
    assert!(out.contains("fn add("), "missing add in trait");
    assert!(out.contains("fn mul("), "missing mul in trait");
}

// ── implements conformance ─────────────────────────────────────────────────────

#[test]
fn implements_with_matching_method_is_ok() {
    let src = r#"module Impl
interface Greeter
  fn greet :: String -> String
end
implements Greeter
fn greet :: String -> String
  name
end
end"#;
    let result = type_check(src);
    assert!(result.is_ok(), "expected OK for conforming implements, got: {:?}", result);
}

#[test]
fn implements_missing_method_is_error() {
    let src = r#"module Broken
interface Greeter
  fn greet :: String -> String
end
implements Greeter
fn unrelated :: Int
  1
end
end"#;
    let result = type_check(src);
    assert!(result.is_err(), "expected error for missing method");
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| format!("{:?}", e).contains("greet")),
        "expected 'greet' in error message, got: {:?}", errs
    );
}

#[test]
fn implements_emits_impl_block() {
    let src = r#"module M
interface Greeter
  fn greet :: String -> String
end
implements Greeter
fn greet :: String -> String
  name
end
end"#;
    let out = compile(src);
    assert!(
        out.contains("impl Greeter for MImpl"),
        "expected impl Greeter for MImpl in:\n{out}"
    );
}

// ── E2E: interface + implements compiles to valid Rust ────────────────────────

#[test]
fn e2e_interface_implements_compiles() {
    let src = r#"module Adder
interface Addable
  fn add :: Int -> Int -> Int
end
implements Addable
fn add :: Int -> Int -> Int
  x + y
end
end"#;
    let rust_src = compile(src);

    let tmp_dir = std::env::temp_dir();
    let rs_path = tmp_dir.join("loom_m16_e2e.rs");
    let bin_path = tmp_dir.join("loom_m16_e2e");
    let full_src = format!("{}\nfn main() {{}}", rust_src);
    std::fs::write(&rs_path, &full_src).unwrap();
    let status = std::process::Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&bin_path)
        .arg(&rs_path)
        .status();
    match status {
        Ok(s) => assert!(s.success(), "rustc failed on:\n{rust_src}"),
        Err(e) => eprintln!("rustc not available: {e} — skipping E2E"),
    }
    let _ = std::fs::remove_file(&rs_path);
    let _ = std::fs::remove_file(&bin_path);
}
