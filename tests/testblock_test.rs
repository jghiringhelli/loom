//! M15 tests — `test:` blocks + real `ensure:` assertions (GS Verifiable property).
//!
//! Verifies:
//! - `ensure: result > 0` emits `debug_assert!(...)` not a comment
//! - `test name :: expr` emits `#[test] fn name() { expr }`
//! - Test module is wrapped in `#[cfg(test)] mod tests { ... }`
//! - `for_all(|x: Int| pred)` emits an edge-case loop assertion
//! - Module without test_defs has no `#[cfg(test)]` block
//! - E2E: emitted test module compiles and tests pass

use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn compile(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

// ── ensure: now emits debug_assert! ──────────────────────────────────────────

#[test]
fn ensure_emits_debug_assert() {
    let src = r#"module Math
fn add :: Int -> Int -> Int
  ensure: result > 0
  x + y
end
end"#;
    let out = compile(src);
    assert!(
        out.contains("debug_assert!"),
        "expected debug_assert! for ensure: in:\n{out}"
    );
    assert!(
        !out.contains("// postcondition"),
        "ensure: should not emit a comment anymore:\n{out}"
    );
}

#[test]
fn ensure_uses_condition_text_in_message() {
    let src = r#"module M
fn get :: Int
  ensure: n > 0
  42
end
end"#;
    let out = compile(src);
    assert!(
        out.contains("ensure:"),
        "expected ensure: label in debug_assert! message:\n{out}"
    );
}

// ── test: blocks ──────────────────────────────────────────────────────────────

#[test]
fn test_block_emits_cfg_test_mod() {
    let src = r#"module Calc
fn add :: Int -> Int -> Int
  x + y
end
test add_works :: inline { assert_eq!(add(1, 2), 3) }
end"#;
    let out = compile(src);
    assert!(
        out.contains("#[cfg(test)]"),
        "expected #[cfg(test)] in:\n{out}"
    );
    assert!(out.contains("mod tests"), "expected mod tests in:\n{out}");
}

#[test]
fn test_block_emits_test_fn() {
    let src = r#"module Calc
fn add :: Int -> Int -> Int
  x + y
end
test add_works :: inline { assert_eq!(add(1, 2), 3) }
end"#;
    let out = compile(src);
    assert!(
        out.contains("#[test]"),
        "expected #[test] attribute in:\n{out}"
    );
    assert!(
        out.contains("fn add_works()"),
        "expected test fn name in:\n{out}"
    );
    assert!(
        out.contains("assert_eq!(add(1, 2), 3)"),
        "expected test body in:\n{out}"
    );
}

#[test]
fn test_block_uses_super_import() {
    let src = r#"module M
fn f :: Int
  1
end
test my_test :: f()
end"#;
    let out = compile(src);
    assert!(
        out.contains("use super::*"),
        "expected use super::* in test mod:\n{out}"
    );
}

#[test]
fn no_test_defs_no_cfg_test_block() {
    let src = r#"module Simple
fn id :: Int -> Int
  x
end
end"#;
    let out = compile(src);
    assert!(
        !out.contains("#[cfg(test)]"),
        "unexpected #[cfg(test)] when no test_defs:\n{out}"
    );
}

#[test]
fn multiple_test_blocks_all_emitted() {
    let src = r#"module M
fn add :: Int -> Int -> Int
  x + y
end
test first_test :: inline { assert_eq!(add(1, 1), 2) }
test second_test :: inline { assert_eq!(add(0, 5), 5) }
end"#;
    let out = compile(src);
    assert!(out.contains("fn first_test()"), "first test missing");
    assert!(out.contains("fn second_test()"), "second test missing");
}

// ── for_all property tests ────────────────────────────────────────────────────

#[test]
fn for_all_emits_edge_case_loop() {
    // Use a simple property: x + 0 >= x (always true for i64)
    // We avoid == since Loom doesn't have == yet; use >= which evaluates to bool
    let src = r#"module M
test prop :: for_all(|x: Int| x + 0 >= x)
end"#;
    let out = compile(src);
    assert!(
        out.contains("_edge_cases"),
        "expected edge case loop in:\n{out}"
    );
    assert!(
        out.contains("i64::MAX"),
        "expected i64::MAX edge case in:\n{out}"
    );
    assert!(
        out.contains("assert!"),
        "expected assert! in for_all expansion:\n{out}"
    );
}

// ── E2E: test module compiles and tests run ───────────────────────────────────

#[test]
fn e2e_test_block_compiles_and_runs() {
    let src = r#"module Adder
fn add :: Int -> Int -> Int
  x + y
end
test add_one_plus_one :: inline { assert_eq!(add(1, 1), 2) }
end"#;
    let rust_src = compile(src);

    // Write to temp and compile with rustc --test
    let tmp_dir = std::env::temp_dir();
    let rs_path = tmp_dir.join("loom_m15_e2e.rs");
    let bin_path = tmp_dir.join("loom_m15_e2e");
    std::fs::write(&rs_path, &rust_src).unwrap();
    let status = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--test", "-o"])
        .arg(&bin_path)
        .arg(&rs_path)
        .status();
    match status {
        Ok(s) => {
            assert!(s.success(), "rustc --test failed on:\n{rust_src}");
            // Run the test binary
            let run_status = std::process::Command::new(&bin_path).status();
            match run_status {
                Ok(rs) => assert!(rs.success(), "test binary failed"),
                Err(e) => eprintln!("could not run test binary: {e}"),
            }
        }
        Err(e) => eprintln!("rustc not available: {e} — skipping E2E"),
    }
    let _ = std::fs::remove_file(&rs_path);
    let _ = std::fs::remove_file(&bin_path);
}
