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
pub mod parser;

pub use error::LoomError;

// ── Public pipeline entry point ───────────────────────────────────────────────

/// Compile a Loom source string to a Rust source string.
///
/// Runs the full pipeline:
///
/// 1. **Lexer** — tokenise `source` into `(Token, Span)` pairs.
/// 2. **Parser** — parse the token stream into an [`ast::Module`].
/// 3. **Type checker** — validate symbols and patterns.
/// 4. **Effect checker** — validate effect declarations.
/// 5. **Code generator** — emit Rust source.
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

    // ── Stage 3: type check ───────────────────────────────────────────────
    checker::TypeChecker::new().check(&module)?;

    // ── Stage 4: effect check ─────────────────────────────────────────────
    checker::EffectChecker::new().check(&module)?;

    // ── Stage 5: code generation ──────────────────────────────────────────
    Ok(codegen::RustEmitter::new().emit(&module))
}
