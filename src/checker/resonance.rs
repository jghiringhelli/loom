//! M82: Resonance checker — cross-signal correlation validation.
//!
//! For each `CorrelationPair` in a being's `resonance:` block, verifies that both
//! signal types are either:
//! (a) declared in a `sense:` item in the module, OR
//! (b) appear as parameter types in some function in scope.
//!
//! If neither, emits a warning — the correlation references an unknown signal type.

use std::collections::HashSet;

use crate::ast::*;
use crate::error::LoomError;

/// Resonance checker.
///
/// Validates that signal type references in `resonance:` blocks are grounded
/// in the module's declared sense channels or function parameter types.
pub struct ResonanceChecker;

impl ResonanceChecker {
    /// Create a new resonance checker.
    pub fn new() -> Self {
        ResonanceChecker
    }

    /// Check all beings in `module` for resonance consistency.
    ///
    /// Returns warnings for correlation pairs that reference unknown signal types.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let known_types = collect_known_signal_types(module);
        let mut warnings = Vec::new();

        for being in &module.being_defs {
            self.check_being(being, &known_types, &mut warnings);
        }
        warnings
    }

    fn check_being(
        &self,
        being: &BeingDef,
        known_types: &HashSet<String>,
        warnings: &mut Vec<LoomError>,
    ) {
        let resonance = match &being.resonance {
            Some(r) => r,
            None => return,
        };

        for pair in &resonance.correlations {
            for signal_type in [&pair.signal_a, &pair.signal_b] {
                if !known_types.contains(signal_type) {
                    warnings.push(LoomError::type_err(
                        format!(
                            "resonance warning: correlation references signal type '{}' \
                             which is not declared in any sense: block or function parameter",
                            signal_type
                        ),
                        pair.span.clone(),
                    ));
                }
            }
        }
    }
}

/// Collect all signal types known in this module:
/// - channel names from `sense:` items
/// - parameter type names from all functions (module-level and inside beings)
fn collect_known_signal_types(module: &Module) -> HashSet<String> {
    let mut known = HashSet::new();

    // From sense: items — sense channels.
    for item in &module.items {
        if let Item::Sense(sd) = item {
            known.insert(sd.name.clone());
            for channel in &sd.channels {
                known.insert(channel.clone());
            }
        }
    }

    // From module-level function parameter types.
    for item in &module.items {
        if let Item::Fn(fd) = item {
            collect_fn_param_types(fd, &mut known);
        }
    }

    // From function parameters inside beings.
    for being in &module.being_defs {
        if let Some(fb) = &being.function {
            for f in &fb.fns {
                collect_fn_param_types(f, &mut known);
            }
        }
    }

    known
}

/// Collect all base type names from a function's parameter list into `known`.
fn collect_fn_param_types(fd: &FnDef, known: &mut HashSet<String>) {
    for param_ty in &fd.type_sig.params {
        if let Some(name) = extract_base_name(param_ty) {
            known.insert(name);
        }
    }
}

/// Extract the base type name from a TypeExpr.
fn extract_base_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Base(name) => Some(name.clone()),
        TypeExpr::Effect(_, inner) => extract_base_name(inner),
        TypeExpr::Generic(name, _) => Some(name.clone()),
        TypeExpr::Option(inner) => extract_base_name(inner),
        TypeExpr::Result(ok, _) => extract_base_name(ok),
        _ => None,
    }
}
