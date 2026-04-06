/// Tests for M10 — numeric coercion (`as` operator) + parenthesized expressions.

use loom::compile;

// ── as coercion ───────────────────────────────────────────────────────────────

#[test]
fn as_int_to_float_emits_cast() {
    let src = r#"
module Demo
  fn to_float :: Int -> Float
    x as Float
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("as f64"),
        "Int as Float should emit 'as f64', got:\n{out}"
    );
}

#[test]
fn as_float_to_int_emits_cast() {
    let src = r#"
module Demo
  fn truncate :: Float -> Int
    x as Int
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("as i64"),
        "Float as Int should emit 'as i64', got:\n{out}"
    );
}

#[test]
fn as_in_arithmetic_expression() {
    let src = r#"
module Demo
  fn mixed :: Int -> Float -> Float
    x as Float * y
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("as f64"),
        "Int as Float in binop should emit as f64, got:\n{out}"
    );
    // Should multiply as floats
    assert!(out.contains("*"), "should contain multiplication");
}

#[test]
fn pricing_engine_corpus_compiles() {
    // The pricing_engine corpus was fixed to use `quantity as Float`.
    let src = std::fs::read_to_string("corpus/pricing_engine.loom")
        .expect("corpus/pricing_engine.loom should exist");
    let out = compile(&src).expect("pricing_engine should compile after coercion fix");
    assert!(
        out.contains("as f64"),
        "pricing_engine should use as f64 cast, got:\n{out}"
    );
}

// ── parenthesized expressions (already worked, regression guard) ──────────────

#[test]
fn parenthesized_expression_parses() {
    let src = r#"
module Demo
  fn demo :: Int -> Int -> Int
    (x + y) * 2
  end
end
"#;
    let out = compile(src).expect("parenthesized expression should compile");
    assert!(out.contains("*"), "multiplication should appear");
    assert!(out.contains("+"), "addition should appear");
}

// ── E2E: as coercion compiles and produces correct runtime output ─────────────

fn e2e_as(loom_src: &str, fn_call: &str, expected: &str) {
    let rust_src = compile(loom_src).expect("loom compile failed");

    let mod_name = {
        let line = rust_src.lines().find(|l| l.contains("pub mod")).unwrap_or("pub mod demo {");
        line.trim()
            .strip_prefix("pub mod ")
            .unwrap_or("demo")
            .trim_end_matches(" {")
            .to_string()
    };
    let main_src = format!(
        "{rust_src}\nfn main() {{\n    println!(\"{{}}\", {mod_name}::{fn_call});\n}}\n"
    );

    let dir = std::env::temp_dir().join(format!(
        "loom_coerce_e2e_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let rs_path = dir.join("main.rs");
    let bin_path = dir.join("main");
    std::fs::write(&rs_path, &main_src).unwrap();

    let compile_out = std::process::Command::new("rustc")
        .args(["--edition", "2021", rs_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
        .output()
        .expect("rustc should be available");

    if !compile_out.status.success() {
        panic!(
            "rustc failed:\n{}\nsource:\n{}",
            String::from_utf8_lossy(&compile_out.stderr),
            main_src
        );
    }

    let run_out = std::process::Command::new(&bin_path).output().expect("binary should run");
    assert_eq!(String::from_utf8_lossy(&run_out.stdout).trim(), expected);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn e2e_int_as_float_runtime() {
    e2e_as(
        r#"
module CoerceE2E
  fn int_to_float :: Int -> Float
    inline { arg0 as f64 + 0.5 }
  end
end
"#,
        "int_to_float(3)",
        "3.5",
    );
}
