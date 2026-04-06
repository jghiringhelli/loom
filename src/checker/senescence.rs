//! M74: Senescence checker (Campisi).
//!
//! Verifies that `senescence` blocks on beings are well-formed:
//! - `onset` must be non-empty.
//! - `degradation` must be non-empty.

use crate::ast::Module;
use crate::error::LoomError;

/// Checker for senescence blocks (M74).
pub struct SenescenceChecker;

impl SenescenceChecker {
    /// Create a new [`SenescenceChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run senescence checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for being in &module.being_defs {
            if let Some(sen) = &being.senescence {
                if sen.onset.is_empty() {
                    errors.push(LoomError::parse(
                        format!("being '{}': senescence block has empty onset", being.name),
                        sen.span.clone(),
                    ));
                }
                if sen.degradation.is_empty() {
                    errors.push(LoomError::parse(
                        format!("being '{}': senescence block has empty degradation", being.name),
                        sen.span.clone(),
                    ));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}
