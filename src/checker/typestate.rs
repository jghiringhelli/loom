//! Typestate / lifecycle checker.
//!
//! Verifies that functions transitioning typed state values
//! only perform transitions declared in the module's `lifecycle` blocks.

use std::collections::HashSet;

use crate::ast::{FnDef, Item, LifecycleDef, Module, TypeExpr};
use crate::error::LoomError;

/// Checks that all typestate transitions in function signatures are declared.
pub struct TypestateChecker;

impl TypestateChecker {
    pub fn new() -> Self {
        TypestateChecker
    }

    /// Check all lifecycle transition constraints in `module`.
    ///
    /// Returns `Ok(())` when every transition is valid, or a list of errors for
    /// any invalid (undeclared) transition found in a function signature.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();

        for lc in &module.lifecycle_defs {
            let valid = valid_transitions(lc);

            for item in &module.items {
                if let Item::Fn(fd) = item {
                    if let Some(err) = check_fn(fd, lc, &valid) {
                        errors.push(err);
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

/// Build the set of valid (from_state, to_state) pairs from adjacent states.
fn valid_transitions(lc: &LifecycleDef) -> HashSet<(String, String)> {
    lc.states
        .windows(2)
        .map(|w| (w[0].clone(), w[1].clone()))
        .collect()
}

/// Check a single function for invalid lifecycle transitions.
///
/// Looks for parameters of type `TypeName<StateA>` and a return type containing
/// `TypeName<StateB>`, then validates the (StateA, StateB) pair.
fn check_fn(
    fd: &FnDef,
    lc: &LifecycleDef,
    valid: &HashSet<(String, String)>,
) -> Option<LoomError> {
    // Collect all input states from parameters.
    let input_states: Vec<String> = fd
        .type_sig
        .params
        .iter()
        .filter_map(|ty| extract_state(ty, &lc.type_name))
        .collect();

    if input_states.is_empty() {
        return None;
    }

    // Find the output state in the return type (unwrap Effect if needed).
    let output_state = extract_state_from_return(&fd.type_sig.return_type, &lc.type_name)?;

    // Check each input → output transition.
    for from in &input_states {
        if !valid.contains(&(from.clone(), output_state.clone())) {
            return Some(LoomError::type_err(
                format!(
                    "invalid lifecycle transition: {}<{}> -> {}<{}> \
                     — declared sequence requires a valid adjacent transition",
                    lc.type_name, from, lc.type_name, output_state
                ),
                fd.span.clone(),
            ));
        }
    }

    None
}

/// Extract the state parameter from `TypeName<State>`.
fn extract_state(ty: &TypeExpr, type_name: &str) -> Option<String> {
    match ty {
        TypeExpr::Generic(name, params) if name == type_name => {
            if let Some(TypeExpr::Base(state)) = params.first() {
                Some(state.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract a state from a return type, descending through `Effect<[...], T>`.
fn extract_state_from_return(ty: &TypeExpr, type_name: &str) -> Option<String> {
    match ty {
        TypeExpr::Effect(_, inner) => extract_state_from_return(inner, type_name),
        other => extract_state(other, type_name),
    }
}
