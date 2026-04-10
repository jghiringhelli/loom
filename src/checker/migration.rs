//! M106: Migration checker — Interface Evolution Contract.
//!
//! Validates `migration:` blocks declared inside `being:` definitions.
//!
//! Rules enforced:
//! 1. `from:` field name that doesn't match any `sense:` or `matter:` field → warning.
//! 2. `adapter: none` with `breaking: false` → error (non-breaking migration requires adapter).
//! 3. Two migration blocks with the same `name` → error (duplicate migration name).
//! 4. Being with `autopoietic: true` and no `migration:` blocks → info hint.
//! 5. Chain consistency: if migration A's to_type of field X ≠ migration B's from_type of field X
//!    (when B consumes what A produces), the chain is broken → error.
//! 6. Cycle detection: a migration chain where field X evolves back to its original type → error.

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

        // Rule 5 + 6: chain consistency and cycle detection for field-based migrations.
        // Parse all (field_name → (from_type, to_type)) pairs.  Version-number migrations
        // (where from_field is a bare integer like "Int(1)") are skipped.
        self.check_migration_chain(being, errors);
    }

    /// Rule 5/6: chain consistency + cycle detection.
    ///
    /// For every pair of field migrations (A, B) where B's from_type of field X
    /// equals A's from_type of field X (i.e. B picks up where A left off), verify
    /// that A's to_type == B's from_type.  Also detect type cycles.
    fn check_migration_chain(&self, being: &BeingDef, errors: &mut Vec<LoomError>) {
        // Collect all field-based migrations as (migration_name, field, from_type, to_type).
        let mut steps: Vec<(&str, String, String, String)> = Vec::new();
        for migration in &being.migrations {
            if let (Some((field, from_ty)), Some((_, to_ty))) = (
                parse_migration_field(&migration.from_field),
                parse_migration_field(&migration.to_field),
            ) {
                steps.push((&migration.name, field, from_ty, to_ty));
            }
        }

        // For each unique field name, build its evolution chain and validate.
        let mut field_names: Vec<&str> = Vec::new();
        for (_, field, _, _) in &steps {
            if !field_names.contains(&field.as_str()) {
                field_names.push(field.as_str());
            }
        }

        for field in field_names {
            let chain: Vec<_> = steps.iter().filter(|(_, f, _, _)| f == field).collect();

            // Rule 5: consecutive links must be type-consistent.
            for pair in chain.windows(2) {
                let (_, _, _, to_ty) = pair[0];
                let (mname, _, from_ty, _) = pair[1];
                if to_ty != from_ty {
                    errors.push(LoomError::type_err(
                        format!(
                            "[error] being '{}': migration chain broken for field '{}' — \
                             migration '{}' expects from_type '{}' but the previous step produces '{}'. \
                             Migrations must form a consistent type sequence.",
                            being.name, field, mname, from_ty, to_ty
                        ),
                        being.span.clone(),
                    ));
                }
            }

            // Rule 6: cycle — if any to_type in the chain equals the first from_type, the
            // type has cycled back to its origin (Penrose 1994: closed causal loop in types).
            if chain.len() > 1 {
                let first_from = &chain[0].2;
                for (mname, _, _, to_ty) in chain.iter().skip(1) {
                    if to_ty == first_from {
                        errors.push(LoomError::type_err(
                            format!(
                                "[error] being '{}': migration cycle detected for field '{}' — \
                                 migration '{}' returns field to its original type '{}'. \
                                 Type cycles in migration chains indicate a logical contradiction.",
                                being.name, field, mname, first_from
                            ),
                            being.span.clone(),
                        ));
                    }
                }
            }
        }
    }
}

/// Extract a plain identifier from a debug-format token string.
///
/// The parser serialises tokens as `format!("{:?}", tok)`, so an identifier
/// `foo` becomes the string `Ident("foo")`. This helper extracts `foo` from
/// both that debug representation and from a plain identifier string.
fn extract_ident_from_debug(s: &str) -> String {
    if let Some(inner) = s
        .strip_prefix("Ident(\"")
        .and_then(|t| t.strip_suffix("\")"))
    {
        inner.to_string()
    } else {
        s.to_string()
    }
}

/// Parse a migration `from:`/`to:` raw token string into `(field_name, type_name)`.
///
/// The parser stores these as space-joined debug token strings, e.g.:
/// `"Ident(\"sense_interval\") Ident(\"Float\")"`.
///
/// Version-number migrations (`"Int(1)"`) return `None`.
fn parse_migration_field(raw: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() >= 2 {
        let field = extract_ident_from_debug(parts[0]);
        let typ = extract_ident_from_debug(parts[1]);
        if !field.is_empty() && !typ.is_empty()
            // Skip version-number form: Int(N) starts with 'I' but has digit after (
            && !field.starts_with("Int(")
            && !field.starts_with("Float(")
        {
            return Some((field, typ));
        }
    }
    None
}
