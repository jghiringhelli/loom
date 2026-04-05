/// Tests for M12 — Tuples · Option<T> · Result<T,E> · `?` propagation.

use loom::compile;

// ── Tuple tests ───────────────────────────────────────────────────────────────

#[test]
fn tuple_emits_rust_tuple() {
    let src = r#"
module Demo
  fn make_pair :: Int -> Int -> (Int, Int)
    (a, b)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(out.contains("(a, b)"), "tuple should emit (a, b), got:\n{out}");
}

#[test]
fn tuple_type_maps_to_rust() {
    let src = r#"
module Demo
  fn pair :: (Int, String)
    todo
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("(i64, String)"),
        "tuple type should emit (i64, String), got:\n{out}"
    );
}

#[test]
fn triple_tuple() {
    let src = r#"
module Demo
  fn triple :: Int -> Int -> Int -> (Int, Int, Int)
    (a, b, c)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(out.contains("(a, b, c)"), "triple tuple should emit");
}

// ── Option<T> tests ───────────────────────────────────────────────────────────

#[test]
fn option_return_type_maps() {
    let src = r#"
module Demo
  fn maybe :: Option<Int>
    todo
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("Option<i64>"),
        "Option<Int> should emit Option<i64>, got:\n{out}"
    );
}

#[test]
fn match_on_option_exhaustive() {
    let src = r#"
module Demo
  fn unwrap_or :: Option<Int> -> Int
    match opt
    | Some(x) -> x
    | None -> 0
    end
  end
end
"#;
    // Should compile without exhaustiveness error
    let out = compile(src).expect("Option match should compile");
    assert!(out.contains("Some(x)"), "Some arm should be emitted");
    assert!(out.contains("None"), "None arm should be emitted");
}

#[test]
fn match_on_result_exhaustive() {
    let src = r#"
module Demo
  fn unwrap_result :: Result<Int, String> -> Int
    match res
    | Ok(v) -> v
    | Err(e) -> 0
    end
  end
end
"#;
    let out = compile(src).expect("Result match should compile");
    assert!(out.contains("Ok(v)"), "Ok arm should emit");
    assert!(out.contains("Err(e)"), "Err arm should emit");
}

// ── ? propagation operator ────────────────────────────────────────────────────

#[test]
fn try_operator_emits_question_mark() {
    let src = r#"
module Demo
  fn fallible :: Result<Int, String> -> Result<Int, String>
    let v = parse_int(s)?
    Ok(v)
  end
end
"#;
    let out = compile(src).expect("should compile");
    assert!(
        out.contains("parse_int(s)?"),
        "? operator should emit as ?, got:\n{out}"
    );
}

// ── E2E: tuple compile and run ────────────────────────────────────────────────

#[test]
fn e2e_tuple_runtime() {
    let rust_src = compile(r#"
module TupleE2E
  fn make_pair :: Int -> Int -> (Int, Int)
    inline { (arg0, arg1) }
  end
end
"#).expect("loom compile failed");

    let mod_name = {
        let line = rust_src.lines().find(|l| l.contains("pub mod")).unwrap_or("pub mod demo {");
        line.trim()
            .strip_prefix("pub mod ")
            .unwrap_or("demo")
            .trim_end_matches(" {")
            .to_string()
    };
    let main_src = format!(
        "{rust_src}\nfn main() {{\n    let (a, b) = {mod_name}::make_pair(3, 4);\n    println!(\"{{}}\", a + b);\n}}\n"
    );

    let dir = std::env::temp_dir().join(format!(
        "loom_alg_e2e_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let rs = dir.join("main.rs");
    let bin = dir.join("main");
    std::fs::write(&rs, &main_src).unwrap();

    let co = std::process::Command::new("rustc")
        .args(["--edition", "2021", rs.to_str().unwrap(), "-o", bin.to_str().unwrap()])
        .output()
        .expect("rustc available");
    if !co.status.success() {
        panic!("rustc failed:\n{}\nsrc:\n{main_src}", String::from_utf8_lossy(&co.stderr));
    }
    let ro = std::process::Command::new(&bin).output().expect("run");
    assert_eq!(String::from_utf8_lossy(&ro.stdout).trim(), "7");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn non_exhaustive_option_match_errors() {
    // Missing None arm should produce a NonExhaustiveMatch error.
    let src = r#"
module Demo
  fn bad :: Option<Int> -> Int
    match opt
    | Some(x) -> x
    end
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "missing None arm should be a compile error"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.kind() == "NonExhaustiveMatch"),
        "should produce NonExhaustiveMatch, got: {:?}",
        errs.iter().map(|e| e.kind()).collect::<Vec<_>>()
    );
}
