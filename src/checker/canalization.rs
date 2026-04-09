//! M70: Canalization checker (Waddington).
//!
//! Verifies that `canalize` blocks on beings are well-formed:
//! - `toward` must be non-empty.
//! - `despite` must have at least one entry.

use crate::ast::Module;
use crate::error::LoomError;

/// Checker for canalization blocks (M70).
pub struct CanalizationChecker;

impl CanalizationChecker {
    /// Create a new [`CanalizationChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run canalization checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for being in &module.being_defs {
            if let Some(can) = &being.canalization {
                if can.toward.is_empty() {
                    errors.push(LoomError::parse(
                        format!("being '{}': canalize block has empty toward", being.name),
                        can.span.clone(),
                    ));
                }
                if can.despite.is_empty() {
                    errors.push(LoomError::parse(
                        format!(
                            "being '{}': canalize block must list at least one despite perturbation",
                            being.name
                        ),
                        can.span.clone(),
                    ));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}
