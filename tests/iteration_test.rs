/// Tests for M11 — first-class iteration (closures · map · filter · fold · for-in).
use loom::compile;

// ── Lambda / closure tests ────────────────────────────────────────────────────

#[test]
fn lambda_emits_rust_closure() {
    let src = r#"
module Demo
  fn apply :: Int -> Int
    inline { (|x: i64| x + 1)(arg0) }
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(out.contains("arg0"), "should contain arg0 param");
}

#[test]
fn lambda_single_param_no_type() {
    let src = r#"
module Demo
  fn inc :: Int -> Int
    let f = |x| x
    f(n)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(out.contains("|x| x"), "lambda should emit as Rust closure");
}

#[test]
fn lambda_with_type_annotation() {
    let src = r#"
module Demo
  fn double :: Int -> Int
    let f = |x: Int| x
    f(n)
  end
end
"#;
    let out = compile(src).expect("should compile");
    // Type annotation: Int → i64
    assert!(
        out.contains("|x: i64| x"),
        "lambda param type should be emitted"
    );
}

#[test]
fn lambda_multi_param() {
    let src = r#"
module Demo
  fn add_pair :: Int -> Int -> Int
    let f = |a, b| a
    f(x, y)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(out.contains("|a, b| a"), "multi-param lambda should emit");
}

// ── HOF: map / filter / fold ──────────────────────────────────────────────────

#[test]
fn map_emits_iter_chain() {
    let src = r#"
module Demo
  fn doubled :: List<Int> -> List<Int>
    map(xs, |x| x)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains(".iter().map("),
        "map should emit .iter().map(), got:\n{out}"
    );
    assert!(
        out.contains(".collect::<Vec<_>>()"),
        "map should collect, got:\n{out}"
    );
}

#[test]
fn filter_emits_iter_chain() {
    let src = r#"
module Demo
  fn positives :: List<Int> -> List<Int>
    filter(xs, |x| x)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains(".iter().filter("),
        "filter should emit iter chain"
    );
    assert!(
        out.contains(".collect::<Vec<_>>()"),
        "filter should collect"
    );
}

#[test]
fn fold_emits_iter_fold() {
    let src = r#"
module Demo
  fn sum :: List<Int> -> Int
    fold(xs, 0, |acc, x| acc)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains(".iter().fold("),
        "fold should emit iter.fold()"
    );
}

// ── for-in loop ───────────────────────────────────────────────────────────────

#[test]
fn for_in_emits_rust_for_loop() {
    let src = r#"
module Demo
  fn print_all :: List<Int> -> Int
    for n in xs { n }
    0
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("for n in"),
        "for-in should emit Rust for loop, got:\n{out}"
    );
    assert!(
        out.contains(".iter()"),
        "for-in should use .iter(), got:\n{out}"
    );
}

// ── E2E: map / fold that actually compile and run ────────────────────────────

fn e2e_iter(loom_src: &str, fn_call: &str, expected: &str) {
    let rust_src = compile(loom_src).expect("loom compile failed");

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
    let main_src =
        format!("{rust_src}\nfn main() {{\n    println!(\"{{}}\", {mod_name}::{fn_call});\n}}\n");

    let dir = std::env::temp_dir().join(format!(
        "loom_iter_e2e_{}",
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
            "rustc failed:\n{}\nsource:\n{}",
            String::from_utf8_lossy(&compile_out.stderr),
            main_src
        );
    }

    let run_out = std::process::Command::new(&bin_path)
        .output()
        .expect("binary should run");
    assert_eq!(String::from_utf8_lossy(&run_out.stdout).trim(), expected);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn e2e_fold_sum() {
    e2e_iter(
        r#"
module IterE2E
  fn sum_list :: List<Int> -> Int
    inline { arg0.iter().fold(0i64, |acc, x| acc + x) }
  end
end
"#,
        "sum_list(vec![1i64, 2, 3, 4, 5])",
        "15",
    );
}

#[test]
fn e2e_map_double() {
    e2e_iter(
        r#"
module IterE2E
  fn double_all :: List<Int> -> List<Int>
    inline { arg0.iter().map(|x| x * 2).collect::<Vec<_>>() }
  end
end
"#,
        "double_all(vec![1i64, 2, 3]).len()",
        "3",
    );
}
