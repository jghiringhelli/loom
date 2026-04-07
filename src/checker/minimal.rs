//! M107: Minimal checker — Dead Declaration Detection.
//!
//! Rejects beings with unused declarations. Every declared element must be
//! load-bearing. Biological equivalent: selection pressure sheds metabolically
//! expensive unused machinery.
//!
//! Rules enforced:
//! 1. `sense:` channel (module-level SenseDef) never referenced in any being's
//!    `evolve:` constraint or `regulate:` variable → WARNING.
//! 2. `regulate:` variable not found in any `matter:` field → ERROR.

use crate::ast::{BeingDef, Item, Module, SenseDef};
use crate::error::LoomError;

/// Validates that every declared element in a being is load-bearing.
pub struct MinimalChecker;

impl MinimalChecker {
    /// Create a new minimal checker.
    pub fn new() -> Self {
        MinimalChecker
    }

    /// Check all beings in `module` for dead declarations.
    ///
    /// Returns accumulated errors and warnings — `[error]`-prefixed messages are
    /// hard structural errors; `[warn]`-prefixed messages are warnings.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();

        // Collect module-level sense declarations for cross-reference.
        let sense_defs: Vec<&SenseDef> = module
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Sense(s) = item { Some(s) } else { None }
            })
            .collect();

        for being in &module.being_defs {
            errors.extend(self.check_being(being, &sense_defs));
        }
        errors
    }

    /// Validate a single being for dead declarations.
    fn check_being(&self, being: &BeingDef, sense_defs: &[&SenseDef]) -> Vec<LoomError> {
        let mut errors = Vec::new();

        // Rule 2: regulate: variable must exist in matter: fields — hard error.
        if let Some(matter) = &being.matter {
            let field_names: Vec<&str> =
                matter.fields.iter().map(|f| f.name.as_str()).collect();
            for reg in &being.regulate_blocks {
                if !reg.variable.is_empty() && !field_names.contains(&reg.variable.as_str()) {
                    errors.push(LoomError::type_err(
                        format!(
                            "[error] regulate '{}' in being '{}': regulated field '{}' \
                             not found in matter: — regulate: bounds must reference a \
                             declared matter: field",
                            reg.variable, being.name, reg.variable
                        ),
                        reg.span.clone(),
                    ));
                }
            }
        }

        // Rule 1: sense channels not referenced in evolve:/regulate: → warning.
        // Collect all text that counts as a "reference" for sense channels.
        let reference_corpus = self.build_reference_corpus(being);

        for sense in sense_defs {
            for channel in &sense.channels {
                if !reference_corpus.iter().any(|r| r.contains(channel.as_str())) {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] sense channel '{}' (from sense '{}') is never referenced \
                             in any being's evolve: or regulate: blocks — unused sense channels \
                             add specification weight with no load-bearing purpose",
                            channel, sense.name
                        ),
                        sense.span.clone(),
                    ));
                }
            }
        }

        errors
    }

    /// Build the corpus of strings that count as references for sense channel lookup.
    ///
    /// Includes: evolve constraint, regulate variable names, regulate target names.
    fn build_reference_corpus(&self, being: &BeingDef) -> Vec<String> {
        let mut corpus = Vec::new();

        if let Some(evolve) = &being.evolve_block {
            corpus.push(evolve.constraint.clone());
        }

        for reg in &being.regulate_blocks {
            corpus.push(reg.variable.clone());
            corpus.push(reg.target.clone());
        }

        corpus
    }
}
