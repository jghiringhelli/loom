//! M77: Niche construction checker (Odling-Smee).
//!
//! Verifies that `niche_construction` items are well-formed:
//! - `modifies` must be non-empty.
//! - `affects` must have at least one entry.

use crate::ast::{Item, Module};
use crate::error::LoomError;

/// Checker for niche construction definitions (M77).
pub struct NicheConstructionChecker;

impl NicheConstructionChecker {
    /// Create a new [`NicheConstructionChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run niche construction checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::NicheConstruction(nc) = item {
                if nc.modifies.is_empty() {
                    errors.push(LoomError::parse(
                        "niche_construction: modifies must be non-empty",
                        nc.span.clone(),
                    ));
                }
                if nc.affects.is_empty() {
                    errors.push(LoomError::parse(
                        "niche_construction: affects must have at least one entry",
                        nc.span.clone(),
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
