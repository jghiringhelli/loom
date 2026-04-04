//! WASM back-end integration tests.
//!
//! Verifies that `loom::compile_wasm` emits valid WAT structure for supported
//! constructs and returns `WasmUnsupported` errors for unsupported ones.

use loom::LoomError;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compile_wasm(src: &str) -> Result<String, Vec<LoomError>> {
    loom::compile_wasm(src)
}

fn wasm_errors(src: &str) -> Vec<LoomError> {
    match compile_wasm(src) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .filter(|e| matches!(e, LoomError::WasmUnsupported { .. }))
            .collect(),
    }
}

// ── Passing cases ─────────────────────────────────────────────────────────────

#[test]
fn simple_int_function_emits_module_and_func() {
    let src = r#"
module M
fn add :: Int -> Int -> Int
  a + b
end
end
"#;
    let wat = compile_wasm(src).expect("simple int function should compile to WASM");
    assert!(wat.contains("(module"), "WAT must start with (module");
    assert!(wat.contains("(func $add"), "WAT must declare func $add");
    assert!(wat.contains("(export \"add\")"), "func must be exported");
    assert!(wat.contains("i64.add"), "must emit i64.add for Int addition");
}

#[test]
fn single_param_let_binding_emits_local() {
    let src = r#"
module M
fn double :: Int -> Int
  let result = n + n
  result
end
end
"#;
    let wat = compile_wasm(src).expect("let-binding function should compile");
    assert!(wat.contains("(local $result i64)"), "must declare local for let binding");
    assert!(wat.contains("local.set $result"), "must set the local after computation");
    assert!(wat.contains("local.get $result"), "must get the local for return");
}

#[test]
fn bool_return_type_emits_i32_result() {
    let src = r#"
module M
fn is_positive :: Int -> Bool
  n > 0
end
end
"#;
    let wat = compile_wasm(src).expect("bool-returning function should compile");
    assert!(wat.contains("(result i32)"), "Bool return type must map to i32");
    assert!(wat.contains("i64.gt_s"), "greater-than on Int must use i64.gt_s");
}

#[test]
fn wasm_demo_corpus_compiles_successfully() {
    let source = std::fs::read_to_string("corpus/wasm_demo.loom")
        .expect("corpus/wasm_demo.loom must exist");
    let wat = compile_wasm(&source).expect("wasm_demo.loom should compile to WASM without errors");
    assert!(wat.starts_with("(module"), "output must be a WAT module");
    assert!(wat.ends_with(")\n"), "module must be properly closed");
}

// ── Error cases (unsupported features) ───────────────────────────────────────

#[test]
fn effectful_function_returns_wasm_unsupported() {
    let src = r#"
module M
fn fetch :: Int -> Effect<[IO], Int>
  n
end
end
"#;
    let errors = wasm_errors(src);
    assert!(!errors.is_empty(), "effectful function must produce WasmUnsupported error");
    assert!(
        errors.iter().any(|e| match e {
            LoomError::WasmUnsupported { feature, .. } => feature.contains("effectful"),
            _ => false,
        }),
        "error must mention 'effectful'"
    );
}

#[test]
fn refined_type_returns_wasm_unsupported() {
    let src = r#"
module M
type Email = String where valid_email
fn f :: Int -> Int
  n
end
end
"#;
    let errors = wasm_errors(src);
    assert!(!errors.is_empty(), "refined type must produce WasmUnsupported error");
    assert!(
        errors.iter().any(|e| match e {
            LoomError::WasmUnsupported { feature, .. } => feature.contains("Email"),
            _ => false,
        }),
        "error must mention the refined type name"
    );
}

#[test]
fn match_expression_returns_wasm_unsupported() {
    let src = r#"
module M
enum Color = | Red | Green end
fn f :: Int -> Int
  match x
  | Red -> 1
  | Green -> 2
  end
end
end
"#;
    let errors = wasm_errors(src);
    assert!(
        !errors.is_empty(),
        "match expression or enum must produce WasmUnsupported error"
    );
}

#[test]
fn rust_compile_still_works_after_wasm_added() {
    // Ensure adding the WASM path didn't break the existing Rust pipeline.
    let source = std::fs::read_to_string("corpus/pricing_engine.loom")
        .expect("corpus/pricing_engine.loom must exist");
    assert!(
        loom::compile(&source).is_ok(),
        "pricing_engine.loom must still compile to Rust"
    );
}
