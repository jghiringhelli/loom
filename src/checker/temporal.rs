//! Temporal logic checker for the Loom compiler.
//!
//! Validates temporal properties (`always`, `eventually`, `never`, `precedes`)
//! against the module's lifecycle declarations and function signatures.
//!
//! Checks performed:
//! - `never: A transitions to B` — verifies no function has signature `Type<A> -> Type<B>`
//! - `precedes: A before B` — verifies A appears before B in the lifecycle ordering
//!   and no function skips from a state before A directly to B
//! - `always` and `eventually` — structural validation only (full model checking
//!   requires the `temporal` feature with an embedded model checker)

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

/// Temporal logic property checker.
///
/// Validates temporal properties against lifecycle state machines and
/// function signatures in the module.
pub struct TemporalChecker;

impl TemporalChecker {
    /// Create a new `TemporalChecker`.
    pub fn new() -> Self {
        TemporalChecker
    }

    /// Check all temporal property blocks in the module.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();

        // Build lifecycle state ordering map: type_name → ordered states
        let lifecycle_map: HashMap<String, Vec<String>> = module
            .lifecycle_defs
            .iter()
            .map(|lc| (lc.type_name.clone(), lc.states.clone()))
            .collect();

        // Collect function transitions: (from_state, to_state) pairs
        let fn_transitions = collect_fn_transitions(&module.items);

        for temporal in &module.temporal_defs {
            for prop in &temporal.properties {
                match prop {
                    TemporalProperty::Never {
                        from_state,
                        to_state,
                        span,
                    } => {
                        check_never_transition(
                            from_state,
                            to_state,
                            &fn_transitions,
                            span,
                            &mut errors,
                        );
                    }
                    TemporalProperty::Precedes {
                        first,
                        second,
                        span,
                    } => {
                        check_precedes(
                            first,
                            second,
                            &lifecycle_map,
                            &fn_transitions,
                            span,
                            &mut errors,
                        );
                    }
                    TemporalProperty::Always { .. } | TemporalProperty::Eventually { .. } => {
                        // Structural validation only — full model checking
                        // requires the `temporal` Cargo feature.
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

/// Extract (from_state, to_state) pairs from function signatures.
///
/// A function `fn f :: Type<A> -> Type<B>` represents a transition from A to B.
fn collect_fn_transitions(items: &[Item]) -> Vec<(String, String)> {
    let mut transitions = Vec::new();
    for item in items {
        if let Item::Fn(fd) = item {
            // Look for Generic type params in input → output
            let input_state = extract_state_from_params(&fd.type_sig.params);
            let output_state = extract_state_from_return(&fd.type_sig.return_type);
            if let (Some(from), Some(to)) = (input_state, output_state) {
                transitions.push((from, to));
            }
        }
    }
    transitions
}

fn extract_state_from_params(params: &[TypeExpr]) -> Option<String> {
    for p in params {
        if let TypeExpr::Generic(_, type_params) = p {
            if let Some(TypeExpr::Base(state)) = type_params.first() {
                return Some(state.clone());
            }
        }
    }
    None
}

fn extract_state_from_return(ret: &TypeExpr) -> Option<String> {
    if let TypeExpr::Generic(_, type_params) = ret {
        if let Some(TypeExpr::Base(state)) = type_params.first() {
            return Some(state.clone());
        }
    }
    None
}

/// Check that no function performs a forbidden transition.
fn check_never_transition(
    from: &str,
    to: &str,
    fn_transitions: &[(String, String)],
    span: &Span,
    errors: &mut Vec<LoomError>,
) {
    for (f, t) in fn_transitions {
        if f == from && t == to {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "temporal violation: transition from `{}` to `{}` is forbidden by `never:` property",
                    from, to
                ),
                span: span.clone(),
            });
        }
    }
}

/// Check that `first` always precedes `second` in lifecycle ordering,
/// and no function skips from a pre-first state directly to second.
fn check_precedes(
    first: &str,
    second: &str,
    lifecycle_map: &HashMap<String, Vec<String>>,
    fn_transitions: &[(String, String)],
    span: &Span,
    errors: &mut Vec<LoomError>,
) {
    // Find which lifecycle contains these states
    for (_type_name, states) in lifecycle_map {
        let first_idx = states.iter().position(|s| s == first);
        let second_idx = states.iter().position(|s| s == second);

        if let (Some(fi), Some(si)) = (first_idx, second_idx) {
            if fi >= si {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "temporal violation: `{}` does not precede `{}` in lifecycle ordering",
                        first, second
                    ),
                    span: span.clone(),
                });
                continue;
            }

            // Check that no function skips from a state before `first` directly to `second`
            let pre_first_states: HashSet<&str> = states[..fi].iter().map(|s| s.as_str()).collect();
            for (from, to) in fn_transitions {
                if pre_first_states.contains(from.as_str()) && to == second {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "temporal violation: function transitions from `{}` to `{}`, \
                             skipping required predecessor `{}` (precedes: {} before {})",
                            from, to, first, first, second
                        ),
                        span: span.clone(),
                    });
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser;

    fn parse_module(src: &str) -> Module {
        let tokens = Lexer::tokenize(src).unwrap();
        parser::Parser::new(&tokens).parse_module().unwrap()
    }

    #[test]
    fn empty_temporal_block_passes() {
        let module = parse_module("module T\nlifecycle P :: A -> B -> C\ntemporal Rules\nend\nend");
        let checker = TemporalChecker::new();
        assert!(checker.check(&module).is_ok());
    }

    #[test]
    fn always_property_passes_structural() {
        let module =
            parse_module("module T\nlifecycle P :: A -> B\ntemporal R\nalways: true\nend\nend");
        let checker = TemporalChecker::new();
        assert!(checker.check(&module).is_ok());
    }
}
