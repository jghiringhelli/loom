//! M110: Use-case triple-derivation checker.
//!
//! Validates `usecase:` blocks for structural completeness and semantic consistency.
//! Jacobson (1992) → Beck (2003) → Connextra format → GS triple derivation.
//!
//! Rules enforced:
//! 1. Empty `acceptance:` list → warning (no verifiable criteria).
//! 2. `precondition:` that is just the literal word "true" → warning (tautology).
//! 3. Duplicate acceptance criterion names → error.
//! 4. `postcondition:` identical to `precondition:` → warning (no state change).

use crate::ast::*;
use crate::error::LoomError;

/// Use-case checker.
///
/// Validates all `usecase:` blocks in a module against the rules above.
pub struct UseCaseChecker;

impl UseCaseChecker {
    /// Create a new `UseCaseChecker`.
    pub fn new() -> Self {
        UseCaseChecker
    }

    /// Check all use-case blocks in `module`.
    ///
    /// Returns accumulated diagnostics. Warnings are encoded as errors prefixed
    /// with `[warn]` so they propagate through the pipeline without blocking
    /// compilation (the pipeline filters them).
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::UseCase(uc) = item {
                self.check_usecase(uc, &mut errors);
            }
        }
        errors
    }

    fn check_usecase(&self, uc: &UseCaseBlock, errors: &mut Vec<LoomError>) {
        // Rule 1: empty acceptance list.
        if uc.acceptance.is_empty() {
            errors.push(LoomError::parse(
                format!(
                    "[warn] usecase '{}': acceptance: block is empty — use case has no verifiable criteria",
                    uc.name
                ),
                uc.span.clone(),
            ));
        }

        // Rule 2: tautological precondition.
        let pre_trimmed = uc.precondition.trim().to_lowercase();
        if pre_trimmed == "true" {
            errors.push(LoomError::parse(
                format!(
                    "[warn] usecase '{}': precondition: 'true' is a tautology — add a meaningful constraint",
                    uc.name
                ),
                uc.span.clone(),
            ));
        }

        // Rule 3: duplicate criterion names.
        let mut seen: Vec<&str> = Vec::new();
        for criterion in &uc.acceptance {
            if seen.contains(&criterion.name.as_str()) {
                errors.push(LoomError::parse(
                    format!(
                        "usecase '{}': duplicate acceptance criterion name '{}'",
                        uc.name, criterion.name
                    ),
                    uc.span.clone(),
                ));
            } else {
                seen.push(&criterion.name);
            }
        }

        // Rule 4: postcondition same as precondition.
        let post_trimmed = uc.postcondition.trim();
        if !pre_trimmed.is_empty()
            && !post_trimmed.is_empty()
            && pre_trimmed == post_trimmed.to_lowercase()
        {
            errors.push(LoomError::parse(
                format!(
                    "[warn] usecase '{}': postcondition: is identical to precondition: — no state change declared",
                    uc.name
                ),
                uc.span.clone(),
            ));
        }
    }
}
