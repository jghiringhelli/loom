//! M76: Criticality bounds checker (Langton).
//!
//! Verifies that `criticality` blocks on beings are well-formed:
//! - `lower` must be >= 0.0.
//! - `upper` must be > `lower`.
//! - Both values must be within [0.0, 1.0].

use crate::ast::Module;
use crate::error::LoomError;

/// Checker for criticality bounds (M76).
pub struct CriticalityChecker;

impl CriticalityChecker {
    /// Create a new [`CriticalityChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run criticality checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for being in &module.being_defs {
            if let Some(crit) = &being.criticality {
                if crit.lower < 0.0 {
                    errors.push(LoomError::parse(
                        format!(
                            "being '{}': criticality lower bound {} must be >= 0.0",
                            being.name, crit.lower
                        ),
                        crit.span.clone(),
                    ));
                }
                if crit.upper > 1.0 {
                    errors.push(LoomError::parse(
                        format!(
                            "being '{}': criticality upper bound {} must be <= 1.0",
                            being.name, crit.upper
                        ),
                        crit.span.clone(),
                    ));
                }
                if crit.upper <= crit.lower {
                    errors.push(LoomError::parse(
                        format!(
                            "being '{}': criticality upper ({}) must be > lower ({})",
                            being.name, crit.upper, crit.lower
                        ),
                        crit.span.clone(),
                    ));
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
