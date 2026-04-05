//! Effect checker for the Loom language.
//!
//! The [`EffectChecker`] validates that every function's *declared* effect set
//! covers all effects that appear transitively in its call graph.
//!
//! Phase 1 rules:
//! - Build a map of `fn_name → declared_effects` from type signatures.
//! - Walk each function body and collect all *called* function names.
//! - Compute the transitive effect set of those callees.
//! - Emit [`LoomError::EffectError`] if:
//!   - A function declared as pure (no `Effect<…>` wrapper) calls an effectful
//!     function.
//!   - A function's declared effect set does not cover all transitive effects.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

// ── Effect checker ────────────────────────────────────────────────────────────

/// Phase-1 effect checker.
pub struct EffectChecker;

impl EffectChecker {
    /// Create a new `EffectChecker`.
    pub fn new() -> Self {
        EffectChecker
    }

    /// Check `module` and return `Ok(())` or `Err(errors)`.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        // Map fn_name → declared effect set (empty = pure).
        let declared: HashMap<String, HashSet<String>> = module
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Fn(fd) = item {
                    let effects = extract_declared_effects(&fd.type_sig.return_type);
                    Some((fd.name.clone(), effects))
                } else {
                    None
                }
            })
            .collect();

        // Map fn_name → max consequence tier.
        let tiers: HashMap<String, ConsequenceTier> = module
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Fn(fd) = item {
                    let tier = effective_tier(fd);
                    Some((fd.name.clone(), tier))
                } else {
                    None
                }
            })
            .collect();

        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                let fn_declared = declared
                    .get(&fd.name)
                    .cloned()
                    .unwrap_or_default();

                let fn_tier = tiers.get(&fd.name).unwrap_or(&ConsequenceTier::Pure);
                let is_annotated_pure = fd.annotations.iter().any(|a| a.key == "pure");

                // Collect all identifiers called in the body.
                let mut called_fns: HashSet<String> = HashSet::new();
                for expr in &fd.body {
                    collect_calls(expr, &mut called_fns);
                }
                for contract in fd.requires.iter().chain(fd.ensures.iter()) {
                    collect_calls(&contract.expr, &mut called_fns);
                }

                // Compute transitive effects from callees.
                let mut transitive_effects: HashSet<String> = HashSet::new();
                for callee in &called_fns {
                    if let Some(callee_effects) = declared.get(callee) {
                        transitive_effects.extend(callee_effects.iter().cloned());
                    }
                }

                // @pure annotation: function must not call any effectful function.
                if is_annotated_pure && !transitive_effects.is_empty() {
                    errors.push(LoomError::effect(
                        format!(
                            "@pure function `{}` calls effectful function(s); \
                             transitive effects: {:?}",
                            fd.name,
                            transitive_effects.iter().cloned().collect::<Vec<_>>()
                        ),
                        fd.span.clone(),
                    ));
                }

                // Pure function (no Effect<> wrapper) calling an effectful function is an error.
                if fn_declared.is_empty() && !is_annotated_pure && !transitive_effects.is_empty() {
                    errors.push(LoomError::effect(
                        format!(
                            "pure function `{}` calls effectful function(s); \
                             transitive effects: {:?}",
                            fd.name,
                            transitive_effects
                                .iter()
                                .cloned()
                                .collect::<Vec<_>>()
                        ),
                        fd.span.clone(),
                    ));
                }

                // Declared effects must cover transitive effects.
                for eff in &transitive_effects {
                    if !fn_declared.contains(eff) && !is_annotated_pure {
                        errors.push(LoomError::effect(
                            format!(
                                "function `{}` uses effect `{}` but does not declare it",
                                fd.name, eff
                            ),
                            fd.span.clone(),
                        ));
                    }
                }

                // Consequence tier enforcement: callee tier must not exceed caller tier.
                for callee in &called_fns {
                    if let Some(callee_tier) = tiers.get(callee) {
                        if tier_severity(callee_tier) > tier_severity(fn_tier) {
                            errors.push(LoomError::effect(
                                format!(
                                    "function `{}` (tier={}) calls `{}` (tier={}) which has a more severe consequence tier",
                                    fd.name,
                                    tier_name(fn_tier),
                                    callee,
                                    tier_name(callee_tier),
                                ),
                                fd.span.clone(),
                            ));
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
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the effect names from a return type.
///
/// Returns an empty set for pure (non-`Effect`) types.
fn extract_declared_effects(ty: &TypeExpr) -> HashSet<String> {
    match ty {
        TypeExpr::Effect(effects, _) => effects.iter().cloned().collect(),
        _ => HashSet::new(),
    }
}

/// Determine the effective consequence tier for a function:
/// - `@pure` annotation → Pure (overrides everything)
/// - effect_tiers present → max tier of all declared effects
/// - no effects and no annotation → Pure
fn effective_tier(fd: &FnDef) -> ConsequenceTier {
    if fd.annotations.iter().any(|a| a.key == "pure") {
        return ConsequenceTier::Pure;
    }
    fd.effect_tiers
        .iter()
        .map(|(_, t)| t.clone())
        .max_by_key(tier_severity)
        .unwrap_or(ConsequenceTier::Pure)
}

fn tier_severity(tier: &ConsequenceTier) -> u8 {
    match tier {
        ConsequenceTier::Pure         => 0,
        ConsequenceTier::Reversible   => 1,
        ConsequenceTier::Irreversible => 2,
    }
}

fn tier_name(tier: &ConsequenceTier) -> &'static str {
    match tier {
        ConsequenceTier::Pure         => "pure",
        ConsequenceTier::Reversible   => "reversible",
        ConsequenceTier::Irreversible => "irreversible",
    }
}

/// Collect all directly-called function names reachable from `expr`.
fn collect_calls(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Call { func, args, .. } => {
            if let Expr::Ident(name) = func.as_ref() {
                out.insert(name.clone());
            }
            collect_calls(func, out);
            for arg in args {
                collect_calls(arg, out);
            }
        }
        Expr::Let { value, .. } => collect_calls(value, out),
        Expr::Match { subject, arms, .. } => {
            collect_calls(subject, out);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    collect_calls(g, out);
                }
                collect_calls(&arm.body, out);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_calls(left, out);
            collect_calls(right, out);
        }
        Expr::FieldAccess { object, .. } => collect_calls(object, out),
        Expr::BinOp { left, right, .. } => {
            collect_calls(left, out);
            collect_calls(right, out);
        }
        Expr::Ident(_) | Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque — cannot inspect inline Rust for effects
        Expr::As(inner, _) => collect_calls(inner, out),
        Expr::Lambda { body, .. } => collect_calls(body, out),
        Expr::ForIn { iter, body, .. } => {
            collect_calls(iter, out);
            collect_calls(body, out);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| collect_calls(e, out)),
        Expr::Try(inner, _) => collect_calls(inner, out),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check(src: &str) -> Result<(), Vec<LoomError>> {
        let tokens = Lexer::tokenize(src).unwrap();
        let module = Parser::new(&tokens).parse_module().unwrap();
        EffectChecker::new().check(&module)
    }

    #[test]
    fn accepts_pure_module_with_no_fns() {
        assert!(check("module M end").is_ok());
    }

    #[test]
    fn accepts_pure_fn_with_no_calls() {
        let src = "module M fn add :: Int -> Int -> Int end end";
        assert!(check(src).is_ok());
    }
}
