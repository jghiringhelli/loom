//! Unified error types for the Loom compiler pipeline.
//!
//! Every pipeline stage (lexer, parser, type checker, effect checker, codegen)
//! produces `LoomError` values, enabling consistent error reporting at the CLI
//! boundary with source-position information.

use crate::ast::Span;
use thiserror::Error;

/// Unified compiler error.
///
/// Each variant corresponds to one pipeline stage and carries a human-readable
/// message plus the source [`Span`] that triggered the error.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LoomError {
    /// Tokenization failure: unknown character or malformed token.
    #[error("lex error at {span}: {msg}")]
    LexError { msg: String, span: Span },

    /// Parse failure: unexpected token or missing syntactic construct.
    #[error("parse error at {span}: {msg}")]
    ParseError { msg: String, span: Span },

    /// Type-check failure: unknown symbol, type mismatch, or bad pattern.
    #[error("type error at {span}: {msg}")]
    TypeError { msg: String, span: Span },

    /// Effect-check failure: undeclared effect or impure call in pure function.
    #[error("effect error at {span}: {msg}")]
    EffectError { msg: String, span: Span },

    /// Code-generation failure: unsupported construct or internal emitter bug.
    #[error("codegen error at {span}: {msg}")]
    CodegenError { msg: String, span: Span },

    /// Non-exhaustive `match` expression: one or more enum variants not covered.
    #[error("non-exhaustive match at {span}: missing variants: {}", missing.join(", "))]
    NonExhaustiveMatch { missing: Vec<String>, span: Span },

    /// WASM code-generation failure: construct not supported by the WASM back-end.
    #[error("wasm unsupported at {span}: {feature}")]
    WasmUnsupported { feature: String, span: Span },

    /// Type unification failure: two types could not be unified.
    #[error("type unification error at {span}: {msg}")]
    UnificationError { msg: String, span: Span },
}

impl LoomError {
    /// Returns the source [`Span`] associated with this error.
    pub fn span(&self) -> &Span {
        match self {
            LoomError::LexError { span, .. }
            | LoomError::ParseError { span, .. }
            | LoomError::TypeError { span, .. }
            | LoomError::EffectError { span, .. }
            | LoomError::CodegenError { span, .. }
            | LoomError::NonExhaustiveMatch { span, .. }
            | LoomError::WasmUnsupported { span, .. }
            | LoomError::UnificationError { span, .. } => span,
        }
    }

    /// Returns a short category label (e.g. `"LexError"`).
    pub fn kind(&self) -> &'static str {
        match self {
            LoomError::LexError { .. } => "LexError",
            LoomError::ParseError { .. } => "ParseError",
            LoomError::TypeError { .. } => "TypeError",
            LoomError::EffectError { .. } => "EffectError",
            LoomError::CodegenError { .. } => "CodegenError",
            LoomError::NonExhaustiveMatch { .. } => "NonExhaustiveMatch",
            LoomError::WasmUnsupported { .. } => "WasmUnsupported",
            LoomError::UnificationError { .. } => "UnificationError",
        }
    }

    /// Convenience constructor for a `LexError`.
    pub fn lex(msg: impl Into<String>, span: Span) -> Self {
        LoomError::LexError { msg: msg.into(), span }
    }

    /// Convenience constructor for a `ParseError`.
    pub fn parse(msg: impl Into<String>, span: Span) -> Self {
        LoomError::ParseError { msg: msg.into(), span }
    }

    /// Convenience constructor for a `TypeError`.
    pub fn type_err(msg: impl Into<String>, span: Span) -> Self {
        LoomError::TypeError { msg: msg.into(), span }
    }

    /// Convenience constructor for an `EffectError`.
    pub fn effect(msg: impl Into<String>, span: Span) -> Self {
        LoomError::EffectError { msg: msg.into(), span }
    }
}
