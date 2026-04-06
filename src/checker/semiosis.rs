//! M79: Semiosis checker — Peircean sign interpretation validation.
//!
//! Validates that if a being declares `telos.sign = TypeName`, it has at least
//! one function in its `function:` block whose parameters include that type.
//! A warning (not an error) is emitted if no such handler is found — the being
//! may receive signals from outside its declared function block.

use crate::ast::*;
use crate::error::LoomError;

/// Semiosis checker.
///
/// Verifies that beings with a declared `telos.sign` type have at least one
/// function handler that accepts that sign type as a parameter.
pub struct SemiosisChecker;

impl SemiosisChecker {
    /// Create a new semiosis checker.
    pub fn new() -> Self {
        SemiosisChecker
    }

    /// Check all beings in `module` for semiosis consistency.
    ///
    /// Returns warnings (as non-fatal errors) for beings that declare a sign
    /// type but have no function handler for it.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut warnings = Vec::new();
        for being in &module.being_defs {
            self.check_being(being, &mut warnings);
        }
        warnings
    }

    fn check_being(&self, being: &BeingDef, warnings: &mut Vec<LoomError>) {
        let sign_type = match being.telos.as_ref().and_then(|t| t.sign.as_ref()) {
            Some(s) => s.clone(),
            None => return,
        };

        // Collect all parameter type names across all functions in this being.
        let handler_types: Vec<String> = being
            .function
            .as_ref()
            .map(|fb| {
                fb.fns
                    .iter()
                    .flat_map(|f| f.type_sig.params.iter())
                    .filter_map(|ty| extract_base_name(ty))
                    .collect()
            })
            .unwrap_or_default();

        if !handler_types.iter().any(|t| t == &sign_type) {
            warnings.push(LoomError::type_err(
                format!(
                    "semiosis warning: being '{}' declares telos.sign '{}' but has no function \
                     handler with that parameter type — the being may receive this sign from outside",
                    being.name, sign_type
                ),
                being.span.clone(),
            ));
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
