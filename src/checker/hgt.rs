//! M75: Horizontal gene transfer (HGT) checker.
//!
//! Verifies that `adopt` declarations are well-formed:
//! - `interface` must be non-empty.
//! - `from_module` must be non-empty.

use crate::ast::{Item, Module};
use crate::error::LoomError;

/// Checker for HGT adopt declarations (M75).
pub struct HgtChecker;

impl HgtChecker {
    /// Create a new [`HgtChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run HGT checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::Adopt(decl) = item {
                if decl.interface.is_empty() {
                    errors.push(LoomError::parse(
                        "adopt: interface name must be non-empty",
                        decl.span.clone(),
                    ));
                }
                if decl.from_module.is_empty() {
                    errors.push(LoomError::parse(
                        "adopt: from_module must be non-empty",
                        decl.span.clone(),
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
