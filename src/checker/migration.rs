//! M106: Migration checker — Interface Evolution Contract.
//!
//! Validates `migration:` blocks declared inside `being:` definitions.
//!
//! Rules enforced:
//! 1. `from:` field name that doesn't match any `sense:` or `matter:` field → warning.
//! 2. `adapter: none` with `breaking: false` → error (non-breaking migration requires adapter).
//! 3. Two migration blocks with the same `name` → error (duplicate migration name).
//! 4. Being with `autopoietic: true` and no `migration:` blocks → info hint.

use crate::ast::{BeingDef, Module};
use crate::error::LoomError;

/// Validates migration blocks in all being declarations.
pub struct MigrationChecker;

impl MigrationChecker {
    /// Create a new migration checker.
    pub fn new() -> Self {
        MigrationChecker
    }

    /// Check all beings in `module` for migration contract validity.
    ///
    /// Returns accumulated errors and warnings — only hard errors block compilation.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            self.check_being(being, &mut errors);
        }
        errors
    }

    /// Validate migration blocks within a single being.
    fn check_being(&self, being: &BeingDef, errors: &mut Vec<LoomError>) {
        // Rule 3: duplicate migration names.
        let mut seen_names: Vec<&str> = Vec::new();
        for migration in &being.migrations {
            if seen_names.contains(&migration.name.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "[error] being '{}': duplicate migration name '{}' — each migration must have a unique version transition name",
                        being.name, migration.name
                    ),
                    migration.span.clone(),
                ));
            } else {
                seen_names.push(migration.name.as_str());
            }
        }

        // Collect declared field names from matter: and sense: for rule 1.
        let matter_fields: Vec<&str> = being
            .matter
            .as_ref()
            .map(|m| m.fields.iter().map(|f| f.name.as_str()).collect())
            .unwrap_or_default();

        for migration in &being.migrations {
            // Rule 2: non-breaking migration without adapter is an error.
            if !migration.breaking && migration.adapter.is_none() {
                errors.push(LoomError::type_err(
                    format!(
                        "[error] migration '{}' in being '{}': breaking: false requires an adapter: — \
                         non-breaking migrations must supply a conversion function",
                        migration.name, being.name
                    ),
                    migration.span.clone(),
                ));
            }

            // Rule 1: from: field not found in matter: → warning (best-effort, field names are
            // embedded in a free-form string so we check for the first token as the field name).
            if !matter_fields.is_empty() {
                let from_field_name = migration.from_field.split_whitespace().next().unwrap_or("");
                // Extract the raw identifier from the debug-format token (e.g. `Ident("foo")` → `foo`).
                let normalized = extract_ident_from_debug(from_field_name);
                if !normalized.is_empty() && !matter_fields.contains(&normalized.as_str()) {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] migration '{}' in being '{}': from: field '{}' not found in matter: \
                             — migrating a nonexistent field has no effect",
                            migration.name, being.name, normalized
                        ),
                        migration.span.clone(),
                    ));
                }
            }
        }

        // Rule 4: autopoietic being with no migrations → info hint.
        if being.autopoietic && being.migrations.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "[info] autopoietic being '{}' declares no migration: blocks — \
                     autopoietic beings should declare their evolution path via migration: \
                     (Maturana/Varela 1972: operational closure requires managing interface evolution)",
                    being.name
                ),
                being.span.clone(),
            ));
        }
    }
}

/// Extract a plain identifier from a debug-format token string.
///
/// The parser serialises tokens as `format!("{:?}", tok)`, so an identifier
/// `foo` becomes the string `Ident("foo")`. This helper extracts `foo` from
/// both that debug representation and from a plain identifier string.
fn extract_ident_from_debug(s: &str) -> String {
    if let Some(inner) = s.strip_prefix("Ident(\"").and_then(|t| t.strip_suffix("\")")) {
        inner.to_string()
    } else {
        s.to_string()
    }
}
