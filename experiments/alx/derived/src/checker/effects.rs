// ALX: derived from loom.loom §"check_effects"
// Effect tracking: every effectful call must be in an Effect context.
// Effects propagate transitively through the call chain.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;
use std::collections::{HashMap, HashSet};

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

    // Build map: fn_name → effect_set (empty = pure, non-empty = has effects)
    let mut fn_effects: HashMap<String, HashSet<String>> = HashMap::new();
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

            // Collect called function names from body text
            let mut called_fns: HashSet<String> = HashSet::new();
            for stmt in &f.body {
                for callee_name in fn_effects.keys() {
                    if callee_name == &f.name { continue; }
                    if body_calls_fn(stmt, callee_name) {
                        called_fns.insert(callee_name.clone());
                    }
                }
            }
            if let Some(inline) = &f.inline_body {
                for callee_name in fn_effects.keys() {
                    if callee_name == &f.name { continue; }
                    if body_calls_fn(inline, callee_name) {
                        called_fns.insert(callee_name.clone());
                    }
                }
            }

            // Compute transitive effects from callees
            let mut transitive_effects: HashSet<String> = HashSet::new();
            for callee in &called_fns {
                if let Some(callee_effs) = fn_effects.get(callee) {
                    transitive_effects.extend(callee_effs.iter().cloned());
                }
            }

            if transitive_effects.is_empty() { continue; }

            // Pure function calling effectful function
            if caller_effects.is_empty() {
                errors.push(LoomError::effect(
                    format!(
                        "pure function `{}` calls effectful function(s); transitive effects: {:?}",
                        f.name,
                        transitive_effects.iter().cloned().collect::<Vec<_>>()
                    ),
                    f.span,
                ));
                continue;
            }

            // Check all needed effects are in declared set
            for needed in &transitive_effects {
                if !caller_effects.contains(needed) {
                    errors.push(LoomError::effect(
                        format!(
                            "function `{}` uses effect `{}` but does not declare it",
                            f.name, needed
                        ),
                        f.span,
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

fn extract_effects(ty: &TypeExpr) -> HashSet<String> {
    match ty {
        TypeExpr::Effect(effects, _) => effects.iter().cloned().collect(),
        _ => HashSet::new(),
    }
}

fn body_calls_fn(body: &str, fn_name: &str) -> bool {
    let pattern = format!("{}(", fn_name);
    body.contains(&pattern)
}
