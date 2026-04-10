//! M68: Degeneracy checker (Edelman).
//!
//! Verifies that `degenerate` blocks on functions are well-formed:
//! - `primary` and `fallback` must be non-empty.
//! - `primary` and `fallback` must be distinct.

use crate::ast::{Item, Module};
use crate::error::LoomError;

/// Checker for degeneracy blocks (M68).
pub struct DegeneracyChecker;

impl DegeneracyChecker {
    /// Create a new [`DegeneracyChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run degeneracy checks over the module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                if let Some(dg) = &fd.degenerate {
                    if dg.primary.is_empty() {
                        errors.push(LoomError::parse(
                            format!("fn '{}': degenerate block has empty primary", fd.name),
                            dg.span.clone(),
                        ));
                    }
                    if dg.fallback.is_empty() {
                        errors.push(LoomError::parse(
                            format!("fn '{}': degenerate block has empty fallback", fd.name),
                            dg.span.clone(),
                        ));
                    }
                    if !dg.primary.is_empty() && dg.primary == dg.fallback {
                        errors.push(LoomError::parse(
                            format!(
                                "fn '{}': degenerate block primary and fallback are identical ('{}')",
                                fd.name, dg.primary
                            ),
                            dg.span.clone(),
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
