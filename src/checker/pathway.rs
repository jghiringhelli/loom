//! M71: Metabolic pathway checker (Krebs).
//!
//! Verifies that `pathway` items are well-formed:
//! - A pathway must have at least one step.
//! - Each step's `from`, `via`, and `to` must be non-empty.
//! - No step may have `from == to` (trivial cycle at a single node).

use crate::ast::{Item, Module};
use crate::error::LoomError;

/// Checker for metabolic pathway definitions (M71).
pub struct PathwayChecker;

impl PathwayChecker {
    /// Create a new [`PathwayChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run pathway checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::Pathway(pw) = item {
                if pw.steps.is_empty() {
                    errors.push(LoomError::parse(
                        format!("pathway '{}': must have at least one step", pw.name),
                        pw.span.clone(),
                    ));
                }
                for step in &pw.steps {
                    if step.from.is_empty() || step.via.is_empty() || step.to.is_empty() {
                        errors.push(LoomError::parse(
                            format!("pathway '{}': step has empty from/via/to", pw.name),
                            step.span.clone(),
                        ));
                    }
                    if step.from == step.to {
                        errors.push(LoomError::parse(
                            format!(
                                "pathway '{}': step from '{}' and to '{}' are identical",
                                pw.name, step.from, step.to
                            ),
                            step.span.clone(),
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
