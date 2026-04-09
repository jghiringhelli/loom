//! M72: Symbiosis checker.
//!
//! Verifies that `symbiotic` import items are well-formed:
//! - `kind` must be one of: mutualistic, commensal, parasitic.
//! - `module` must be non-empty.

use crate::ast::{Item, Module};
use crate::error::LoomError;

const VALID_KINDS: &[&str] = &["mutualistic", "commensal", "parasitic"];

/// Checker for symbiotic imports (M72).
pub struct SymbiosisChecker;

impl SymbiosisChecker {
    /// Create a new [`SymbiosisChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run symbiosis checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::SymbioticImport { module: mod_name, kind, span } = item {
                if mod_name.is_empty() {
                    errors.push(LoomError::parse(
                        "symbiotic import: module name must be non-empty",
                        span.clone(),
                    ));
                }
                if !VALID_KINDS.contains(&kind.as_str()) {
                    errors.push(LoomError::parse(
                        format!(
                            "symbiotic import: kind '{}' is not valid; expected one of: {}",
                            kind,
                            VALID_KINDS.join(", ")
                        ),
                        span.clone(),
                    ));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}
