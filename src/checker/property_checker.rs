//! M109: Property-based test checker.
//!
//! Validates `property:` blocks in the Loom pipeline.
//!
//! Rules:
//! 1. `samples: 0` → compile error (zero samples is meaningless).
//! 2. `invariant:` that does not reference `var_name` → warning.

use crate::ast::*;
use crate::error::LoomError;

/// M109: Property-based test checker.
///
/// Validates structural correctness of `property:` blocks.
/// QuickCheck (Claessen & Hughes 2000) lineage.
pub struct PropertyChecker;

impl PropertyChecker {
    /// Create a new property checker.
    pub fn new() -> Self {
        PropertyChecker
    }

    /// Check all `property:` blocks in `module`.
    ///
    /// Returns accumulated errors — the pipeline receives all errors at once.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Property(pb) = item {
                self.check_property(pb, &mut errors);
            }
        }
        errors
    }

    fn check_property(&self, pb: &PropertyBlock, errors: &mut Vec<LoomError>) {
        // Rule 1: samples: 0 is a hard error — zero samples produces no evidence.
        if pb.samples == 0 {
            errors.push(LoomError::parse(
                format!("property '{}': samples must be > 0, got 0", pb.name),
                pb.span.clone(),
            ));
        }

        // Rule 2: invariant must reference the quantified variable.
        if !pb.invariant.contains(&pb.var_name) {
            errors.push(LoomError::parse(
                format!(
                    "[warn] property '{}': invariant expression does not reference '{}' — \
                     the quantified variable appears unused",
                    pb.name, pb.var_name
                ),
                pb.span.clone(),
            ));
        }
    }
}
