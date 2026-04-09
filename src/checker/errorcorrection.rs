//! M73: Error correction checker.
//!
//! Verifies that `on_violation`/`repair_fn` fields on refined types are well-formed:
//! - `on_violation` must be a non-empty identifier if present.
//! - `repair_fn` must be a non-empty identifier if present.

use crate::ast::{Item, Module};
use crate::error::LoomError;

/// Checker for error-correction annotations on refined types (M73).
pub struct ErrorCorrectionChecker;

impl ErrorCorrectionChecker {
    /// Create a new [`ErrorCorrectionChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run error correction checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::RefinedType(rt) = item {
                if let Some(ov) = &rt.on_violation {
                    if ov.is_empty() {
                        errors.push(LoomError::parse(
                            format!(
                                "type '{}': on_violation must be non-empty if present",
                                rt.name
                            ),
                            rt.span.clone(),
                        ));
                    }
                }
                if let Some(rf) = &rt.repair_fn {
                    if rf.is_empty() {
                        errors.push(LoomError::parse(
                            format!("type '{}': repair_fn must be non-empty if present", rt.name),
                            rt.span.clone(),
                        ));
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
