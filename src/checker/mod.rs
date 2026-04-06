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
pub mod refinement;
pub mod safety;
pub mod separation;
pub mod teleos;
pub mod temporal;
pub mod typestate;
pub mod types;
pub mod units;
pub mod gradual;
pub mod probabilistic;
pub mod dependent;
pub mod sidechannel;
pub mod category;
pub mod curryhow;
pub mod selfcert;

pub use algebraic::AlgebraicChecker;
pub use effects::EffectChecker;
pub use exhaustiveness::ExhaustivenessChecker;
pub use infer::InferenceEngine;
pub use infoflow::InfoFlowChecker;
pub use privacy::PrivacyChecker;
pub use refinement::RefinementChecker;
pub use safety::SafetyChecker;
pub use separation::SeparationChecker;
pub use teleos::check as check_teleos;
pub use temporal::TemporalChecker;
pub use typestate::TypestateChecker;
pub use types::TypeChecker;
pub use units::UnitsChecker;
pub use gradual::GradualChecker;
pub use probabilistic::ProbabilisticChecker;
pub use dependent::DependentChecker;
pub use sidechannel::SideChannelChecker;
pub use category::CategoryChecker;
pub use curryhow::CurryHowardChecker;
pub use selfcert::SelfCertChecker;
