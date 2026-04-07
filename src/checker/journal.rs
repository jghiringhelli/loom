//! Journal checker — validates `journal:` blocks inside `being:` declarations.
//!
//! Rules:
//! 1. `record: every evolve_step` without an `evolve:` block → warning (records nothing).
//! 2. `record: every telos_progress` without a `telos:` block → warning (records nothing).
//! 3. `keep: last 0` → error (zero-size ring buffer is invalid).
//! 4. `autopoietic: true` being without `journal:` → warning (autopoietic beings should
//!    have episodic memory to trace their self-modification history).

use crate::ast::{BeingDef, JournalRecord, Module};
use crate::error::LoomError;

/// Validate `journal:` blocks across all beings in a module.
pub struct JournalChecker;

impl JournalChecker {
    /// Create a new checker instance.
    pub fn new() -> Self {
        JournalChecker
    }

    /// Run all journal checks on a module.
    ///
    /// Returns `Ok(())` when no hard errors are found. Warnings are also
    /// returned as errors so the pipeline can display them uniformly.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            self.check_being(being, &mut errors);
        }
        errors
    }

    fn check_being(&self, being: &BeingDef, errors: &mut Vec<LoomError>) {
        // Rule 4: autopoietic beings should have journal:
        if being.autopoietic && being.journal.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "[warn] autopoietic being '{}' has no journal: — \
                     autopoietic beings should record their self-modification history",
                    being.name
                ),
                being.span.clone(),
            ));
        }

        let Some(journal) = &being.journal else { return };

        // Rule 3: keep: last 0 is invalid
        if let Some(0) = journal.keep_last {
            errors.push(LoomError::type_err(
                format!(
                    "journal: in being '{}' has keep: last 0 — \
                     zero-size ring buffer is invalid",
                    being.name
                ),
                journal.span.clone(),
            ));
        }

        for record in &journal.records {
            match record {
                // Rule 1: record: every evolve_step without evolve: block
                JournalRecord::EvolveStep if being.evolve_block.is_none() => {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] journal in being '{}' records evolve_step \
                             but there is no evolve: block — nothing will be recorded",
                            being.name
                        ),
                        journal.span.clone(),
                    ));
                }
                // Rule 2: record: every telos_progress without telos: block
                JournalRecord::TelosProgress if being.telos.is_none() => {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] journal in being '{}' records telos_progress \
                             but there is no telos: block — nothing will be recorded",
                            being.name
                        ),
                        journal.span.clone(),
                    ));
                }
                _ => {}
            }
        }
    }
}
