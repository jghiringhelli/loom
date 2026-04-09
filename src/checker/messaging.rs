//! M116: Messaging primitive checker.
//!
//! Validates `messaging_primitive:` constructs at the module level.
//!
//! Rules:
//! - Name must be non-empty.
//! - `guarantees:` items must be non-empty strings.

use crate::ast::{Item, Module};
use crate::error::LoomError;

pub struct MessagingChecker;

impl MessagingChecker {
    /// Construct a new [`MessagingChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `messaging_primitive:` constructs in the module.
    ///
    /// # Returns
    /// A vec of [`LoomError`] — empty on success.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::MessagingPrimitive(mp) = item {
                if mp.name.trim().is_empty() {
                    errors.push(LoomError::type_err(
                        "messaging_primitive has empty name",
                        mp.span.clone(),
                    ));
                }
                for guarantee in &mp.guarantees {
                    if guarantee.trim().is_empty() {
                        errors.push(LoomError::type_err(
                            format!(
                                "messaging_primitive '{}': guarantees list contains an empty entry",
                                mp.name
                            ),
                            mp.span.clone(),
                        ));
                    }
                }
            }
        }
        errors
    }
}
