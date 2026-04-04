//! Loom compiler library.
//!
//! Exposes all pipeline stages and the high-level [`compile`] function that
//! orchestrates the full lexer → parser → type-checker → effect-checker →
//! code-generator pipeline.
//!
//! # Quick start
//!
//! ```rust,ignore
//! match loom::compile(source) {
//!     Ok(rust_src) => println!("{}", rust_src),
//!     Err(errors)  => errors.iter().for_each(|e| eprintln!("{}", e)),
//! }
//! ```

#![allow(missing_docs)] // Phase 1: docs are on public items; fields documented in Phase 2

pub mod ast;
pub mod checker;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod project;

pub use error::LoomError;

// ── Public pipeline entry point ───────────────────────────────────────────────

/// Compile a Loom source string to a Rust source string.
///
/// Runs the full pipeline:
///
/// 1. **Lexer** — tokenise `source` into `(Token, Span)` pairs.
/// 2. **Parser** — parse the token stream into an [`ast::Module`].
/// 3. **Type inference** — HM unification, validates body types match signatures.
/// 4. **Type checker** — validate symbols and patterns.
/// 5. **Exhaustiveness checker** — verify all `match` arms are exhaustive.
/// 6. **Effect checker** — validate effect declarations.
/// 7. **Code generator** — emit Rust source.
///
/// Returns `Ok(rust_source)` on success.  On failure, returns all accumulated
/// [`LoomError`]s so the caller can display the complete diagnostic list.
pub fn compile(source: &str) -> Result<String, Vec<LoomError>> {
    // ── Stage 1: lex ──────────────────────────────────────────────────────
    let tokens = lexer::Lexer::tokenize(source)?;

    // ── Stage 2: parse ────────────────────────────────────────────────────
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;

    // ── Stage 3: type inference ───────────────────────────────────────────
    checker::InferenceEngine::new().check(&module)?;

    // ── Stage 4: type check ───────────────────────────────────────────────
    checker::TypeChecker::new().check(&module)?;

    // ── Stage 5: exhaustiveness check ────────────────────────────────────
    checker::ExhaustivenessChecker::new().check(&module)?;

    // ── Stage 6: effect check ─────────────────────────────────────────────
    checker::EffectChecker::new().check(&module)?;

    // ── Stage 7: code generation ──────────────────────────────────────────
    Ok(codegen::RustEmitter::new().emit(&module))
}

// ── WASM pipeline entry point ─────────────────────────────────────────────────

/// Compile a Loom source string to a WebAssembly Text format (WAT) string.
///
/// Runs the lex → parse → inference → type-check → exhaustiveness-check
/// pipeline, then emits WAT instead of Rust.  Only the M3 supported subset
/// is accepted; any unsupported construct (effect types, enums, refined types,
/// match expressions) returns a [`LoomError::WasmUnsupported`] error.
///
/// Returns `Ok(wat_source)` on success.  On failure, returns all accumulated
/// [`LoomError`]s.
pub fn compile_wasm(source: &str) -> Result<String, Vec<LoomError>> {
    // ── Stage 1: lex ──────────────────────────────────────────────────────
    let tokens = lexer::Lexer::tokenize(source)?;

    // ── Stage 2: parse ────────────────────────────────────────────────────
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;

    // ── Stage 3: type inference ───────────────────────────────────────────
    checker::InferenceEngine::new().check(&module)?;

    // ── Stage 4: type check ───────────────────────────────────────────────
    checker::TypeChecker::new().check(&module)?;

    // ── Stage 5: exhaustiveness check ────────────────────────────────────
    checker::ExhaustivenessChecker::new().check(&module)?;

    // ── Stage 6: WASM code generation ────────────────────────────────────
    codegen::WasmEmitter::new().emit(&module)
}
