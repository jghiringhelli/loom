//! Loom standard library sources — embedded at compile time.
//!
//! The sense_stdlib is the complete measurable universe as typed signal channels.
//! It is available in all Loom modules via `use SenseStdlib`.
//!
//! The chemistry_stdlib models stoichiometry, kinetics, and thermodynamics using
//! M3 units, M56 refinement types, M86 @conserved, M87 tensors, and M92 graph
//! stores — no compiler changes needed.

/// The sense standard library source, embedded at compile time.
pub const SENSE_STDLIB: &str = include_str!("sense_stdlib.loom");

/// The chemistry standard library source (M89).
pub const CHEMISTRY_STDLIB: &str = include_str!("chemistry_stdlib.loom");

/// The finance standard library source (M90).
///
/// Stochastic processes (GBM), Black-Scholes pricing, Markowitz portfolio theory,
/// VaR/CVaR risk measures, and fixed-income analytics — all written in Loom
/// using M3 units, M56 refinement types, M84 probabilistic types, and M92 stores.
pub const FINANCE_STDLIB: &str = include_str!("finance_stdlib.loom");

/// The quantum mechanics standard library source (M91).
///
/// Qubit state representation, unitary gate operators, Born-rule measurement,
/// Heisenberg uncertainty verification, Schrödinger time evolution, and
/// quantum information measures — written in Loom using M56 refinement types,
/// M84 probabilistic types, and M86 @conserved annotations.
pub const QUANTUM_STDLIB: &str = include_str!("quantum_stdlib.loom");
