// ALX: derived from loom.loom §"check_typestate"
// Lifecycle state transitions must be respected.
// A function taking Payment<Pending> cannot receive Payment<Completed>.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;
use std::collections::HashMap;

/// TypestateChecker struct — tests call `TypestateChecker::new().check(&module)`.
pub struct TypestateChecker;

impl TypestateChecker {
    pub fn new() -> Self { TypestateChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_typestate(module)
    }
}

pub fn check_typestate(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Build transition map: type_name -> ordered Vec<state>
    let mut transitions: HashMap<String, Vec<String>> = HashMap::new();
    for lc in &module.lifecycle_defs {
        transitions.insert(lc.type_name.clone(), lc.states.clone());
    }

    // For each function, check that typestate transitions are valid:
    // if a param is TypeName<StateA> and return is TypeName<StateB>,
    // StateA -> StateB must be a valid adjacent pair in the lifecycle.
    for item in &module.items {
        if let Item::Fn(f) = item {
            let param_states: Vec<Option<(String, String)>> =
                f.type_sig.params.iter().map(|p| extract_typestate(p)).collect();
            let ret_state = extract_typestate(&f.type_sig.return_type);

            for ps in param_states.iter().flatten() {
                if let Some(rs) = &ret_state {
                    if ps.0 == rs.0 {
                        // Same base type — check transition validity
                        if let Some(states) = transitions.get(&ps.0) {
                            if !is_valid_transition(states, &ps.1, &rs.1) {
                                errors.push(LoomError::new(
                                    format!(
                                        "function '{}': invalid lifecycle transition {}::{} -> {}::{}",
                                        f.name, ps.0, ps.1, rs.0, rs.1
                                    ),
                                    f.span,
                                ));
                            }
                        }
                    }
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

/// Extract (TypeName, State) from a Generic type application like Payment<Pending>.
fn extract_typestate(ty: &TypeExpr) -> Option<(String, String)> {
    if let TypeExpr::Generic(name, args) = ty {
        if args.len() == 1 {
            if let TypeExpr::Base(state) = &args[0] {
                // Check if this looks like a typestate (capital first letter, not a unit)
                if state.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    return Some((name.clone(), state.clone()));
                }
            }
        }
    }
    None
}

/// Returns true if from -> to is an adjacent pair in the state sequence.
fn is_valid_transition(states: &[String], from: &str, to: &str) -> bool {
    for window in states.windows(2) {
        if window[0] == from && window[1] == to {
            return true;
        }
    }
    false
}
