//! M80: Umwelt checker — perceptual world validation (Uexküll 1909).
//!
//! Beings are omnisensory by default. If a being declares an `umwelt:` block,
//! it restricts its perceptual world:
//! - `detects:` — only these signal types are visible
//! - `blind_to:` — these signal types are explicitly excluded
//!
//! Checker rules:
//! - If `blind_to` is non-empty and a function handler accepts a type in `blind_to`,
//!   emit an error (this being cannot perceive that signal).
//! - If `detects` is non-empty and a function handler accepts a type NOT in `detects`,
//!   emit a warning.

use crate::ast::*;
use crate::error::LoomError;

/// Umwelt checker.
///
/// Validates that a being's declared perceptual world is consistent with its
/// function handlers. A being that is `blind_to` a signal type must not have
/// handlers for it.
pub struct UmweltChecker;

impl UmweltChecker {
    /// Create a new umwelt checker.
    pub fn new() -> Self {
        UmweltChecker
    }

    /// Check all beings in `module` for umwelt consistency.
    ///
    /// Returns errors for `blind_to` violations and warnings for `detects` mismatches.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            self.check_being(being, &mut errors);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_being(&self, being: &BeingDef, errors: &mut Vec<LoomError>) {
        let umwelt = match &being.umwelt {
            Some(u) => u,
            None => return, // omnisensory by default — no restrictions
        };

        let fns = match &being.function {
            Some(fb) => &fb.fns,
            None => return,
        };

        for f in fns {
            for param_ty in &f.type_sig.params {
                let type_name = match extract_base_name(param_ty) {
                    Some(n) => n,
                    None => continue,
                };

                // Error: handler for a type in blind_to.
                if umwelt.blind_to.contains(&type_name) {
                    errors.push(LoomError::type_err(
                        format!(
                            "umwelt violation: function '{}' handles signal type '{}' which is in \
                             umwelt.blind_to — this being is blind to that signal",
                            f.name, type_name
                        ),
                        f.span.clone(),
                    ));
                }

                // Warning: handler for a type not in detects (when detects is non-empty).
                if !umwelt.detects.is_empty() && !umwelt.detects.contains(&type_name) {
                    errors.push(LoomError::type_err(
                        format!(
                            "umwelt warning: function '{}' handles signal type '{}' not in \
                             umwelt.detects",
                            f.name, type_name
                        ),
                        f.span.clone(),
                    ));
                }
            }
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
