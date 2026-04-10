//! M69: Cell cycle checkpoint checker (Hartwell).
//!
//! Verifies that `checkpoint` blocks in lifecycle definitions are well-formed:
//! - `requires` must be non-empty.
//! - `on_fail` must be non-empty.
//! - No two checkpoints in the same lifecycle may have the same name.

use crate::ast::Module;
use crate::error::LoomError;
use std::collections::HashSet;

/// Checker for lifecycle checkpoints (M69).
pub struct CheckpointChecker;

impl CheckpointChecker {
    /// Create a new [`CheckpointChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run checkpoint checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for lc in &module.lifecycle_defs {
            let mut seen: HashSet<&str> = HashSet::new();
            for cp in &lc.checkpoints {
                if cp.requires.is_empty() {
                    errors.push(LoomError::parse(
                        format!(
                            "lifecycle '{}' checkpoint '{}': requires must be non-empty",
                            lc.type_name, cp.name
                        ),
                        cp.span.clone(),
                    ));
                }
                if cp.on_fail.is_empty() {
                    errors.push(LoomError::parse(
                        format!(
                            "lifecycle '{}' checkpoint '{}': on_fail must be non-empty",
                            lc.type_name, cp.name
                        ),
                        cp.span.clone(),
                    ));
                }
                if !seen.insert(cp.name.as_str()) {
                    errors.push(LoomError::parse(
                        format!(
                            "lifecycle '{}': duplicate checkpoint name '{}'",
                            lc.type_name, cp.name
                        ),
                        cp.span.clone(),
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
