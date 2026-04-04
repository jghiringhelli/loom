//! Semantic analysis for the Loom compiler.
//!
//! Re-exports the two checker passes so callers can write:
//!
//! ```rust,ignore
//! use loom::checker::{TypeChecker, EffectChecker};
//! ```

pub mod effects;
pub mod types;

pub use effects::EffectChecker;
pub use types::TypeChecker;
