// ALX: derived from loom.loom §"Pipeline: Entry points"
// compile() is the primary entry point. Full pipeline:
// lex → parse → check_inference → check_types → check_exhaustiveness →
// check_effects → check_algebraic → check_units → check_typestate →
// check_privacy → check_infoflow → check_teleos → check_safety → emit_rust

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod checker;
pub mod codegen;
pub mod project;
pub mod lsp;

pub use error::LoomError;
use ast::Module;

// ── Internal pipeline helpers ─────────────────────────────────────────────────

fn lex_and_parse(source: &str) -> Result<Module, Vec<LoomError>> {
    let tokens = lexer::lex(source).map_err(|e| e)?;
    parser::parse(tokens).map_err(|e| vec![e])
}

fn run_checks(module: &Module) -> Result<(), Vec<LoomError>> {
    checker::run_all(module)
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Full compilation pipeline: source → Rust.
///
/// Runs: lex → parse → all 11 checkers → emit_rust
pub fn compile(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    Ok(codegen::rust::emit_rust(&module))
}

/// Full pipeline: source → TypeScript.
pub fn compile_typescript(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    Ok(codegen::typescript::emit_typescript(&module))
}

/// Full pipeline: source → OpenAPI 3.0 YAML.
pub fn compile_openapi(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    Ok(codegen::openapi::emit_openapi(&module))
}

/// Full pipeline: source → JSON Schema draft-07.
pub fn compile_json_schema(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    Ok(codegen::json_schema::emit_json_schema(&module))
}

/// Full pipeline: source → WebAssembly text format (WAT).
pub fn compile_wasm(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    codegen::wasm::emit_wasm(&module)
}

/// Full pipeline: source → Mesa ABM Python simulation (M52).
pub fn compile_simulation(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    Ok(codegen::simulation::emit_simulation(&module))
}

/// Full pipeline: source → NeuroML 2 XML (M53).
///
/// ensure: result starts with "<neuroml"
pub fn compile_neuroml(source: &str) -> Result<String, Vec<LoomError>> {
    let module = lex_and_parse(source)?;
    run_checks(&module)?;
    let result = codegen::neuroml::emit_neuroml(&module);
    debug_assert!(
        result.contains("<neuroml"),
        "emit_neuroml ensure: result must contain <neuroml"
    );
    Ok(result)
}
