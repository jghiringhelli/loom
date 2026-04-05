//! Privacy label checker.
//!
//! Validates that:
//! - `@pci` fields also carry `@encrypt-at-rest` and `@never-log`
//! - `@hipaa` fields also carry `@encrypt-at-rest`
//! - `@pii` fields are acknowledged (informational — no error)

use crate::ast::*;
use crate::error::LoomError;

/// Checker that enforces privacy-label co-occurrence rules.
pub struct PrivacyChecker;

impl PrivacyChecker {
    pub fn new() -> Self {
        PrivacyChecker
    }

    /// Check all type definitions in `module` for privacy-label violations.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();

        for item in &module.items {
            if let Item::Type(td) = item {
                for field in &td.fields {
                    let has = |key: &str| -> bool {
                        field.annotations.iter().any(|a| a.key == key)
                    };

                    // @pci requires @encrypt-at-rest AND @never-log
                    if has("pci") {
                        if !has("encrypt-at-rest") {
                            errors.push(LoomError::type_err(
                                format!(
                                    "field `{}.{}` has @pci but is missing @encrypt-at-rest",
                                    td.name, field.name
                                ),
                                field.span.clone(),
                            ));
                        }
                        if !has("never-log") {
                            errors.push(LoomError::type_err(
                                format!(
                                    "field `{}.{}` has @pci but is missing @never-log",
                                    td.name, field.name
                                ),
                                field.span.clone(),
                            ));
                        }
                    }

                    // @hipaa requires @encrypt-at-rest
                    if has("hipaa") && !has("encrypt-at-rest") {
                        errors.push(LoomError::type_err(
                            format!(
                                "field `{}.{}` has @hipaa but is missing @encrypt-at-rest",
                                td.name, field.name
                            ),
                            field.span.clone(),
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
