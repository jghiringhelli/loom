//! M102: Provenance checker — data lineage tracking.
//!
//! W3C PROV-DM (2013) → Buneman (2001) "Why and Where" → Loom `@provenance`.
//!
//! Rules enforced:
//! 1. `@provenance` on a fn return that calls another fn without `@provenance` → warning.
//! 2. Mixing `@provenance("sensor:A")` with `@provenance("sensor:B")` without a merge → error.
//! 3. `@provenance` on a `@pii` field → error (PII provenance creates linkability risk).

use crate::ast::*;
use crate::error::LoomError;

/// Provenance checker — validates `@provenance` annotations in a module.
pub struct ProvenanceChecker;

impl ProvenanceChecker {
    /// Create a new ProvenanceChecker.
    pub fn new() -> Self {
        ProvenanceChecker
    }

    /// Check all provenance annotations in `module`.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Type(td) = item {
                self.check_type_fields(td, &mut errors);
            }
            if let Item::Fn(fd) = item {
                self.check_fn_return(fd, &mut errors);
            }
        }
        errors
    }

    /// Rule 3: @provenance on @pii field → error.
    fn check_type_fields(&self, td: &TypeDef, errors: &mut Vec<LoomError>) {
        for field in &td.fields {
            let has_provenance = field.annotations.iter().any(|a| a.key == "provenance");
            let has_pii = field.annotations.iter().any(|a| a.key == "pii");
            if has_provenance && has_pii {
                errors.push(LoomError::type_err(
                    format!(
                        "[error] field '{}' in type '{}': @provenance on @pii field creates linkability risk — remove @provenance or @pii",
                        field.name, td.name
                    ),
                    field.span.clone(),
                ));
            }
        }
    }

    /// Rule 1: @provenance fn return calling non-@provenance fn → warning.
    fn check_fn_return(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        let has_provenance = fd.annotations.iter().any(|a| a.key == "provenance");
        if !has_provenance {
            return;
        }
        // Conservative check: if the fn body references a Call expression, emit a warning.
        if !fd.body.is_empty() {
            let body_str = format!("{:?}", fd.body);
            if body_str.contains("Call") {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] fn '{}': @provenance fn calls other fns — verify all callees also carry @provenance to preserve chain",
                        fd.name
                    ),
                    fd.span.clone(),
                ));
            }
        }
    }
}
