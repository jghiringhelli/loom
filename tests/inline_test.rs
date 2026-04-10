/// Tests for M9 — `inline rust {}` escape hatch.
///
/// Verifies that:
/// - `inline { ... }` parses and emits the content verbatim
/// - Existing `todo` bodies still work (no regression)
/// - Inline bodies survive the full pipeline (lex → parse → check → emit)
/// - E2E: emitted Rust compiles and runs via rustc
use loom::compile;

// ── Parse + emit tests ────────────────────────────────────────────────────────

#[test]
fn inline_body_emits_verbatim() {
    let src = r#"
module Demo
  fn forty_two :: Int
    inline { 42i64 }
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("42i64"),
        "inline body should be emitted verbatim, got:\n{out}"
    );
}

#[test]
fn inline_body_not_wrapped() {
    let src = r#"
module Demo
  fn greet :: String -> String
    inline { format!("hello, {}!", arg0) }
  end
end
"#;
    let out = compile(src).expect("should compile");
    // The inline content must appear verbatim — NOT wrapped in todo!() or similar
    assert!(
        out.contains(r#"format!("hello, {}!", arg0)"#),
        "inline body should be emitted verbatim, got:\n{out}"
    );
    assert!(
        !out.contains("todo!()"),
        "inline body should not produce todo!(), got:\n{out}"
    );
}

#[test]
fn inline_body_with_braces() {
    // Nested braces inside an inline block must be handled correctly.
    let src = r#"
module Demo
  fn make_struct :: Int
    inline { { let x = 1i64; x + 1 } }
  end
end
"#;
    let out = compile(src).expect("should compile with nested braces");
    assert!(
        out.contains("let x = 1i64; x + 1"),
        "nested braces should be captured, got:\n{out}"
    );
}

#[test]
fn todo_body_still_works() {
    // Regression: existing todo bodies must still emit todo!()
    let src = r#"
module Demo
  fn stub :: Int -> Int
    todo
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("todo!()"),
        "todo body should still emit todo!(), got:\n{out}"
    );
}

#[test]
fn inline_with_multiline_content() {
    let src = "module Demo\n  fn calc :: Int\n    inline {\n      let a = 1i64;\n      let b = 2i64;\n      a + b\n    }\n  end\nend\n";
    let out = compile(src).expect("should compile multiline inline");
    assert!(
        out.contains("let a = 1i64;"),
        "multiline inline body should be emitted, got:\n{out}"
    );
    assert!(
        out.contains("a + b"),
        "multiline inline body should contain final expr, got:\n{out}"
    );
}

#[test]
fn inline_demo_corpus_compiles() {
    let src = std::fs::read_to_string("corpus/inline_demo.loom")
        .expect("corpus/inline_demo.loom should exist");
    let out = compile(&src).expect("inline_demo corpus should compile");
    assert!(
        out.contains("42i64"),
        "forty_two body should be verbatim, got:\n{out}"
    );
}

// ── E2E test: inline Rust that rustc accepts ──────────────────────────────────

#[cfg(target_os = "windows")]
const RUSTC: &str = "rustc.exe";
#[cfg(not(target_os = "windows"))]
const RUSTC: &str = "rustc";

/// Compile a Loom source → Rust → binary → run and check stdout.
fn e2e(loom_src: &str, expected_stdout: &str) {
    let rust_src = compile(loom_src).expect("loom compile failed");

    let dir = std::env::temp_dir().join(format!(
        "loom_inline_e2e_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    // Wrap in a main that calls the module function.
    let mod_name = {
        let line = rust_src
            .lines()
            .find(|l| l.contains("pub mod"))
            .unwrap_or("pub mod demo {");
        line.trim()
            .strip_prefix("pub mod ")
            .unwrap_or("demo")
            .trim_end_matches(" {")
            .to_string()
    };
    let main_src = format!(
        "{rust_src}\nfn main() {{\n    let result = {mod_name}::forty_two();\n    println!(\"{{}}\", result);\n}}\n"
    );

    let rs_path = dir.join("main.rs");
    let bin_path = dir.join("main");
    std::fs::write(&rs_path, &main_src).unwrap();

    let compile_out = std::process::Command::new(RUSTC)
        .args([
            "--edition",
            "2021",
            rs_path.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
        ])
        .output()
        .expect("rustc should be available");

    if !compile_out.status.success() {
        panic!(
            "rustc failed:\nstdout: {}\nstderr: {}\nsource:\n{}",
            String::from_utf8_lossy(&compile_out.stdout),
            String::from_utf8_lossy(&compile_out.stderr),
            main_src
        );
    }

    let run_out = std::process::Command::new(&bin_path)
        .output()
        .expect("binary should run");
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    assert_eq!(stdout.trim(), expected_stdout, "unexpected runtime output");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn e2e_inline_forty_two() {
    e2e(
        r#"
module InlineE2E
  fn forty_two :: Int
    inline { 42i64 }
  end
end
"#,
        "42",
    );
}
