//! ALX — Autonomous Loom eXperiment tooling.
//!
//! Provides convergence tracing for ALX experiments: measuring S_realized at
//! each checker stage, not just at the final gate. This gives a convergence
//! curve that diagnoses which stages contribute most to correctness proofs.

pub mod convergence;
pub use convergence::{
    compute_convergence_trace, ConvergenceStep, ConvergenceTrace, ConvergenceTraceMode,
};
