// ALX: derived from loom.loom §"Error type" — LoomError holds message + span.
// thiserror used for Display/Error impls.
// G-lsp / G-exhaustiveness: LoomError is an enum so tests can pattern-match on variants.

use thiserror::Error;

/// Source position (byte offsets, half-open interval [start, end)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Merge two spans into the smallest span that covers both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// A synthetic span (zero-width at origin) for AST nodes constructed programmatically.
    pub fn synthetic() -> Self {
        Span { start: 0, end: 0 }
    }
}

/// The unified error type for all pipeline stages.
/// Each variant corresponds to a distinct compiler phase or error class.
#[derive(Debug, Clone, Error)]
pub enum LoomError {
    #[error("lex error: {msg} at {span:?}")]
    LexError { msg: String, span: Span },

    #[error("parse error: {msg} at {span:?}")]
    ParseError { msg: String, span: Span },

    #[error("type error: {msg} at {span:?}")]
    TypeError { msg: String, span: Span },

    #[error("unification error: {msg} at {span:?}")]
    UnificationError { msg: String, span: Span },

    #[error("non-exhaustive match: missing pattern(s) for {missing:?} at {span:?}")]
    NonExhaustiveMatch { missing: Vec<String>, span: Span },

    #[error("undeclared dependency: {name} at {span:?}")]
    UndeclaredDependency { name: String, span: Span },

    #[error("wasm unsupported: {feature} at {span:?}")]
    WasmUnsupported { feature: String, span: Span },

    #[error("{message} at {span:?}")]
    General { message: String, span: Span },
}

impl LoomError {
    /// Create a general error (for all pipeline stages that don't need a specific variant).
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        LoomError::General { message: message.into(), span }
    }

    pub fn at(message: impl Into<String>, start: usize, end: usize) -> Self {
        LoomError::new(message, Span::new(start, end))
    }

    pub fn zero(message: impl Into<String>) -> Self {
        LoomError::new(message, Span::default())
    }

    /// Return the error kind name. Used by tests to assert on specific error categories.
    pub fn kind(&self) -> &str {
        match self {
            LoomError::LexError { .. }          => "LexError",
            LoomError::ParseError { .. }        => "ParseError",
            LoomError::TypeError { .. }         => "TypeError",
            LoomError::UnificationError { .. }  => "UnificationError",
            LoomError::NonExhaustiveMatch { .. }=> "NonExhaustiveMatch",
            LoomError::UndeclaredDependency { .. } => "UndeclaredDependency",
            LoomError::WasmUnsupported { .. }   => "WasmUnsupported",
            LoomError::General { .. }           => "LoomError",
        }
    }

    /// Extract the span from any variant.
    pub fn span(&self) -> Span {
        match self {
            LoomError::LexError { span, .. }          => *span,
            LoomError::ParseError { span, .. }        => *span,
            LoomError::TypeError { span, .. }         => *span,
            LoomError::UnificationError { span, .. }  => *span,
            LoomError::NonExhaustiveMatch { span, .. }=> *span,
            LoomError::UndeclaredDependency { span, .. } => *span,
            LoomError::WasmUnsupported { span, .. }   => *span,
            LoomError::General { span, .. }           => *span,
        }
    }
}
