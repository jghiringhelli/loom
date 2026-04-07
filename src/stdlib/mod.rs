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

/// The chemistry standard library source.
pub const CHEMISTRY_STDLIB: &str = include_str!("chemistry_stdlib.loom");
