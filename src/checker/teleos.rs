//! Teleological checker — validates `being:` blocks.
//!
//! Rules:
//! - Every `being` must declare a `telos:` (final cause).
//! - Every `regulate` block must declare `bounds:`.
//! - Every `evolve` block must have a non-empty `constraint:`.

use crate::ast::{BeingDef, Module};
use crate::error::LoomError;

/// Run all teleological checks on a module.
pub fn check(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();
    for being in &module.being_defs {
        check_being(being, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_being(being: &BeingDef, errors: &mut Vec<LoomError>) {
    if being.telos.is_none() {
        errors.push(LoomError::type_err(
            format!(
                "being '{}' has no telos: — every being must declare its final cause",
                being.name
            ),
            being.span.clone(),
        ));
    }
    for reg in &being.regulate_blocks {
        if reg.bounds.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "regulate '{}' in being '{}' has no bounds:",
                    reg.variable, being.name
                ),
                reg.span.clone(),
            ));
        }
    }
    if let Some(evolve) = &being.evolve_block {
        if evolve.constraint.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("evolve block in being '{}' has no constraint:", being.name),
                evolve.span.clone(),
            ));
        }
    }
}
