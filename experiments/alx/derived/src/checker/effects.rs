// ALX: derived from loom.loom §"check_effects"
// Effect tracking: every effectful call must be in an Effect context.
// Effects propagate transitively through the call chain.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;
use std::collections::HashMap;

/// G4: EffectChecker struct — tests call `EffectChecker::new().check(&module)`.
pub struct EffectChecker;

impl EffectChecker {
    pub fn new() -> Self { EffectChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_effects(module)
    }
}

pub fn check_effects(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Build map: fn_name → effect_set (None = pure, Some(vec) = has effects)
    let mut fn_effects: HashMap<String, Option<Vec<String>>> = HashMap::new();
    for item in &module.items {
        if let Item::Fn(f) = item {
            let effect_set = extract_effects(&f.type_sig.return_type);
            fn_effects.insert(f.name.clone(), effect_set);
        }
    }

    // Check each function's body for calls to effectful functions
    for item in &module.items {
        if let Item::Fn(f) = item {
            let caller_effects = extract_effects(&f.type_sig.return_type);

            // Scan body for function calls
            for stmt in &f.body {
                for (callee_name, callee_effects) in &fn_effects {
                    if callee_name == &f.name { continue; } // no self-check
                    // Check if callee is called in this body statement
                    if body_calls_fn(stmt, callee_name) {
                        if let Some(needed_effects) = callee_effects {
                            if needed_effects.is_empty() { continue; }
                            match &caller_effects {
                                None => {
                                    // Pure function calling effectful function
                                    errors.push(LoomError::new(
                                        format!(
                                            "function '{}': pure function calls effectful function '{}' (effects: {:?})",
                                            f.name, callee_name, needed_effects
                                        ),
                                        f.span,
                                    ));
                                }
                                Some(declared) => {
                                    // Check all needed effects are in declared set
                                    for needed in needed_effects {
                                        if !declared.contains(needed) {
                                            errors.push(LoomError::new(
                                                format!(
                                                    "function '{}': calls '{}' which requires effect '{}', but only declares {:?}",
                                                    f.name, callee_name, needed, declared
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
            }

            // Also check inline_body for calls
            if let Some(inline) = &f.inline_body {
                for (callee_name, callee_effects) in &fn_effects {
                    if callee_name == &f.name { continue; }
                    if body_calls_fn(inline, callee_name) {
                        if let Some(needed_effects) = callee_effects {
                            if needed_effects.is_empty() { continue; }
                            if caller_effects.is_none() {
                                errors.push(LoomError::new(
                                    format!(
                                        "function '{}': pure function calls effectful function '{}'",
                                        f.name, callee_name
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

fn extract_effects(ty: &TypeExpr) -> Option<Vec<String>> {
    match ty {
        TypeExpr::Effect(effects, _) => Some(effects.clone()),
        _ => None,
    }
}

fn body_calls_fn(body: &str, fn_name: &str) -> bool {
    // Check if the body text contains a call like fn_name(
    let pattern = format!("{}(", fn_name);
    body.contains(&pattern)
}
