// ALX: derived from loom.loom §"check_algebraic"
// Validate @idempotent/@exactly-once/@commutative constraints.

use crate::ast::{Module, Item};
use crate::error::LoomError;

pub fn check_algebraic(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    for item in &module.items {
        if let Item::Fn(f) = item {
            let idempotent = f.annotations.iter().any(|a| a.key == "idempotent");
            let exactly_once = f.annotations.iter().any(|a| a.key == "exactly-once");
            let at_most_once = f.annotations.iter().any(|a| a.key == "at-most-once");
            let commutative = f.annotations.iter().any(|a| a.key == "commutative");

            // @idempotent and @exactly-once are mutually exclusive.
            if idempotent && exactly_once {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': @idempotent and @exactly-once are mutually exclusive",
                        f.name
                    ),
                    f.span,
                ));
            }

            // @exactly-once and @at-most-once are mutually exclusive.
            if exactly_once && at_most_once {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': @exactly-once and @at-most-once are mutually exclusive",
                        f.name
                    ),
                    f.span,
                ));
            }

            // @commutative requires at least 2 parameters.
            if commutative && f.type_sig.params.len() < 2 {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': @commutative requires at least 2 parameters, got {}",
                        f.name,
                        f.type_sig.params.len()
                    ),
                    f.span,
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
