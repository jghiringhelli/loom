//! Standard library type mapping tests for M6.
//!
//! Verifies that List<T>, Map<K,V>, and Set<T> map to the correct Rust types
//! and that necessary `use` imports are injected automatically.

fn compile_ok(src: &str) -> String {
    loom::compile(src).expect("expected compilation to succeed")
}

// ── Type emission ─────────────────────────────────────────────────────────────

#[test]
fn list_int_emits_vec_i64() {
    let src = r#"
module M
fn nums :: List<Int> -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("Vec<i64>"), "expected Vec<i64> in:\n{}", out);
}

#[test]
fn list_string_emits_vec_string() {
    let src = r#"
module M
fn words :: List<String> -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("Vec<String>"), "expected Vec<String> in:\n{}", out);
}

#[test]
fn map_string_int_emits_hashmap() {
    let src = r#"
module M
fn lookup :: Map<String, Int> -> Bool
  true
end
end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("HashMap<String, i64>"),
        "expected HashMap<String, i64> in:\n{}", out
    );
}

#[test]
fn set_bool_emits_hashset() {
    let src = r#"
module M
fn flags :: Set<Bool> -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("HashSet<bool>"), "expected HashSet<bool> in:\n{}", out);
}

// ── Import injection ──────────────────────────────────────────────────────────

#[test]
fn module_using_map_gets_hashmap_import() {
    let src = r#"
module M
fn lookup :: Map<String, Int> -> Bool
  true
end
end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("use std::collections::HashMap"),
        "expected HashMap import in:\n{}", out
    );
}

#[test]
fn module_using_set_gets_hashset_import() {
    let src = r#"
module M
fn flags :: Set<Int> -> Bool
  true
end
end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("use std::collections::HashSet"),
        "expected HashSet import in:\n{}", out
    );
}

#[test]
fn module_not_using_collections_gets_no_extra_imports() {
    let src = r#"
module M
fn add :: Int -> Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(!out.contains("HashMap"), "unexpected HashMap in pure module:\n{}", out);
    assert!(!out.contains("HashSet"), "unexpected HashSet in pure module:\n{}", out);
}

#[test]
fn module_using_both_collections_gets_combined_import() {
    let src = r#"
module M
fn first :: Map<String, Int> -> Bool
  true
end
fn second :: Set<Bool> -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("HashMap"), "expected HashMap in:\n{}", out);
    assert!(out.contains("HashSet"), "expected HashSet in:\n{}", out);
}

// ── Type-check acceptance ─────────────────────────────────────────────────────

#[test]
fn list_type_does_not_produce_type_error() {
    let src = r#"
module M
fn process :: List<Int> -> List<String>
  todo
end
end
"#;
    // Should compile without TypeError for unknown type "List"
    compile_ok(src);
}

// ── Corpus regression ─────────────────────────────────────────────────────────

#[test]
fn all_existing_corpus_still_compiles() {
    for path in &["corpus/pricing_engine.loom", "corpus/user_service.loom", "corpus/wasm_demo.loom", "corpus/di_demo.loom"] {
        let src = std::fs::read_to_string(path).unwrap();
        loom::compile(&src).unwrap_or_else(|e| panic!("{} failed: {:?}", path, e));
    }
}
