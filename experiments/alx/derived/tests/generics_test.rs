//! Generic function tests for M7 — user-defined type parameters.

fn compile_ok(src: &str) -> String {
    loom::compile(src).expect("expected compilation to succeed")
}

// ── Codegen: type parameter emission ─────────────────────────────────────────

#[test]
fn single_type_param_emits_generic_fn() {
    let src = r#"
module M
fn identity<T> :: T -> T
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub fn identity<T>"), "expected generic fn in:\n{}", out);
}

#[test]
fn two_type_params_emit_correctly() {
    let src = r#"
module M
fn map_fn<A, B> :: A -> B
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub fn map_fn<A, B>"), "expected <A, B> in:\n{}", out);
}

#[test]
fn generic_fn_with_list_param() {
    let src = r#"
module M
fn first<T> :: List<T> -> T
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub fn first<T>"), "expected generic fn in:\n{}", out);
    assert!(out.contains("Vec<T>"), "expected Vec<T> in:\n{}", out);
}

#[test]
fn non_generic_fn_emits_no_angle_brackets() {
    let src = r#"
module M
fn add :: Int -> Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    // The fn signature should not have angle brackets (generics)
    let fn_line = out.lines().find(|l| l.contains("pub fn add")).unwrap_or("");
    assert!(!fn_line.contains('<'), "unexpected generic in non-generic fn: {}", fn_line);
}

// ── Type checking: type params in scope ───────────────────────────────────────

#[test]
fn type_param_names_are_valid_in_function_scope() {
    // T should not cause a TypeError (unknown type) in the function body
    let src = r#"
module M
fn wrap<T> :: T -> T
  todo
end
end
"#;
    compile_ok(src);  // just asserting it doesn't panic/fail
}

#[test]
fn multiple_type_params_are_all_valid_in_scope() {
    let src = r#"
module M
fn pair<A, B> :: A -> B -> A
  todo
end
end
"#;
    compile_ok(src);
}

// ── Backward compatibility ────────────────────────────────────────────────────

#[test]
fn all_existing_corpus_still_compiles_after_generics() {
    for path in &[
        "corpus/pricing_engine.loom",
        "corpus/user_service.loom",
        "corpus/wasm_demo.loom",
        "corpus/di_demo.loom",
        "corpus/collections_demo.loom",
    ] {
        let src = std::fs::read_to_string(path).unwrap();
        loom::compile(&src).unwrap_or_else(|e| panic!("{} failed: {:?}", path, e));
    }
}
