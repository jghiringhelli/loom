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
        let declared = collect_declared_effects(module);
        let tiers = collect_consequence_tiers(module);
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                check_fn_effects(fd, &declared, &tiers, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Build a map of `fn_name → declared effect set` (empty = pure).
fn collect_declared_effects(module: &Module) -> HashMap<String, HashSet<String>> {
    module
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
        .collect()
}

/// Build a map of `fn_name → max consequence tier`.
fn collect_consequence_tiers(module: &Module) -> HashMap<String, ConsequenceTier> {
    module
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(fd) = item {
                Some((fd.name.clone(), effective_tier(fd)))
            } else {
                None
            }
        })
        .collect()
}

/// Check one function's effect declarations against its transitive call graph.
fn check_fn_effects(
    fd: &FnDef,
    declared: &HashMap<String, HashSet<String>>,
    tiers: &HashMap<String, ConsequenceTier>,
    errors: &mut Vec<LoomError>,
) {
    let fn_declared = declared.get(&fd.name).cloned().unwrap_or_default();
    let fn_tier = tiers.get(&fd.name).unwrap_or(&ConsequenceTier::Pure);
    let is_pure = fd.annotations.iter().any(|a| a.key == "pure");

    let called_fns = collect_called_fns(fd);
    let transitive_effects = transitive_effects_of(&called_fns, declared);

    if is_pure && !transitive_effects.is_empty() {
        errors.push(LoomError::effect(
            format!(
                "@pure function `{}` calls effectful function(s); transitive effects: {:?}",
                fd.name,
                transitive_effects.iter().cloned().collect::<Vec<_>>()
            ),
            fd.span.clone(),
        ));
    }

    if fn_declared.is_empty() && !is_pure && !transitive_effects.is_empty() {
        errors.push(LoomError::effect(
            format!(
                "pure function `{}` calls effectful function(s); transitive effects: {:?}",
                fd.name,
                transitive_effects.iter().cloned().collect::<Vec<_>>()
            ),
            fd.span.clone(),
        ));
    }

    for eff in &transitive_effects {
        if !fn_declared.contains(eff) && !is_pure {
            errors.push(LoomError::effect(
                format!(
                    "function `{}` uses effect `{}` but does not declare it",
                    fd.name, eff
                ),
                fd.span.clone(),
            ));
        }
    }

    for callee in &called_fns {
        if let Some(callee_tier) = tiers.get(callee) {
            if tier_severity(callee_tier) > tier_severity(fn_tier) {
                errors.push(LoomError::effect(
                    format!(
                        "function `{}` (tier={}) calls `{}` (tier={}) which has a more severe consequence tier",
                        fd.name, tier_name(fn_tier), callee, tier_name(callee_tier),
                    ),
                    fd.span.clone(),
                ));
            }
        }
    }
}

/// Collect all function identifiers called in `fd`'s body and contracts.
fn collect_called_fns(fd: &FnDef) -> HashSet<String> {
    let mut called = HashSet::new();
    for expr in &fd.body {
        collect_calls(expr, &mut called);
    }
    for contract in fd.requires.iter().chain(fd.ensures.iter()) {
        collect_calls(&contract.expr, &mut called);
    }
    called
}

/// Compute the union of effects declared by every callee in `called_fns`.
fn transitive_effects_of(
    called_fns: &HashSet<String>,
    declared: &HashMap<String, HashSet<String>>,
) -> HashSet<String> {
    let mut effects = HashSet::new();
    for callee in called_fns {
        if let Some(callee_effects) = declared.get(callee) {
            effects.extend(callee_effects.iter().cloned());
        }
    }
    effects
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
        ConsequenceTier::Pure => 0,
        ConsequenceTier::Reversible => 1,
        ConsequenceTier::Irreversible => 2,
    }
}

fn tier_name(tier: &ConsequenceTier) -> &'static str {
    match tier {
        ConsequenceTier::Pure => "pure",
        ConsequenceTier::Reversible => "reversible",
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
        Expr::Index(collection, index, _) => {
            collect_calls(collection, out);
            collect_calls(index, out);
        }
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
