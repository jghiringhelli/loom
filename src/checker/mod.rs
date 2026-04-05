//! Semantic analysis for the Loom compiler.
//!
//! Re-exports the two checker passes so callers can write:
//!
//! ```rust,ignore
//! use loom::checker::{TypeChecker, EffectChecker};
//! ```

pub mod algebraic;
pub mod effects;
pub mod exhaustiveness;
pub mod infer;
pub mod infoflow;
pub mod privacy;
pub mod safety;
pub mod teleos;
pub mod typestate;
pub mod types;
pub mod units;

pub use algebraic::AlgebraicChecker;
pub use effects::EffectChecker;
pub use exhaustiveness::ExhaustivenessChecker;
pub use infer::InferenceEngine;
pub use infoflow::InfoFlowChecker;
pub use privacy::PrivacyChecker;
pub use safety::SafetyChecker;
pub use teleos::check as check_teleos;
pub use typestate::TypestateChecker;
pub use types::TypeChecker;
pub use units::UnitsChecker;
