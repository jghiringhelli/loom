//! Loom standard library sources — embedded at compile time.
//!
//! The sense_stdlib is the complete measurable universe as typed signal channels.
//! It is available in all Loom modules via `use SenseStdlib`.

/// The sense standard library source, embedded at compile time.
pub const SENSE_STDLIB: &str = include_str!("sense_stdlib.loom");
