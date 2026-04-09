// ALX: derived from loom.loom §"check_algebraic"
// Validate @idempotent/@exactly-once/@commutative constraints.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;

pub fn check_algebraic(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    for item in &module.items {
        if let Item::Fn(f) = item {
            let idempotent = f.annotations.iter().any(|a| a.key == "idempotent");
            let exactly_once = f.annotations.iter().any(|a| a.key == "exactly-once");
            let at_most_once = f.annotations.iter().any(|a| a.key == "at-most-once");
            let commutative = f.annotations.iter().any(|a| a.key == "commutative");

            let has_effects = !f.effect_tiers.is_empty()
                || matches!(f.type_sig.return_type, TypeExpr::Effect(_, _));

            // @idempotent and @exactly-once are contradictory.
            if idempotent && exactly_once {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': idempotent and exactly-once are contradictory",
                        f.name
                    ),
                    f.span,
                ));
            }

            // @exactly-once and @at-most-once have conflicting multiplicity.
            if exactly_once && at_most_once {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': conflicting multiplicity annotations: @exactly-once and @at-most-once",
                        f.name
                    ),
                    f.span,
                ));
            }

            // @exactly-once requires an effectful function.
            if exactly_once && !has_effects {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': exactly-once requires an effectful function (Effect<[...], T> return type)",
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
