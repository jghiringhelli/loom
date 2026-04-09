//! Scenario checker — validates `scenario:` blocks inside `being:` declarations.
//!
//! Rules:
//! 1. `then:` assertion that is empty → error.
//! 2. `given:` or `when:` that is empty → error.
//! 3. `within: 0 <unit>` → error (zero-tick deadline is unreachable).
//! 4. `autopoietic: true` being with no `scenario:` blocks → warning
//!    (autopoietic beings should have executable acceptance criteria).

use crate::ast::{BeingDef, Module, ScenarioBlock};
use crate::error::LoomError;

/// Validate `scenario:` blocks across all beings in a module.
pub struct ScenarioChecker;

impl ScenarioChecker {
    /// Create a new checker instance.
    pub fn new() -> Self {
        ScenarioChecker
    }

    /// Run all scenario checks on a module.
    ///
    /// Returns a (possibly empty) list of errors. Warnings are also returned
    /// as errors so the pipeline can display them uniformly.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            self.check_being(being, &mut errors);
        }
        errors
    }

    fn check_being(&self, being: &BeingDef, errors: &mut Vec<LoomError>) {
        // Rule 4: autopoietic beings should have at least one scenario:
        if being.autopoietic && being.scenarios.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "[warn] autopoietic being '{}' has no scenario: blocks — \
                     autopoietic beings should have executable acceptance criteria",
                    being.name
                ),
                being.span.clone(),
            ));
        }

        for scenario in &being.scenarios {
            self.check_scenario(scenario, &being.name, errors);
        }
    }

    fn check_scenario(
        &self,
        scenario: &ScenarioBlock,
        being_name: &str,
        errors: &mut Vec<LoomError>,
    ) {
        // Rule 2: empty given:
        if scenario.given.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "scenario '{}' in being '{}' has empty given: — \
                     a precondition is required",
                    scenario.name, being_name
                ),
                scenario.span.clone(),
            ));
        }

        // Rule 2: empty when:
        if scenario.when.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "scenario '{}' in being '{}' has empty when: — \
                     a trigger is required",
                    scenario.name, being_name
                ),
                scenario.span.clone(),
            ));
        }

        // Rule 1: empty then:
        if scenario.then.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "scenario '{}' in being '{}' has empty then: — \
                     an assertion is required",
                    scenario.name, being_name
                ),
                scenario.span.clone(),
            ));
        }

        // Rule 3: within: 0 is unreachable
        if let Some((0, _)) = &scenario.within {
            errors.push(LoomError::type_err(
                format!(
                    "scenario '{}' in being '{}' has within: 0 — \
                     zero-tick deadline is unreachable",
                    scenario.name, being_name
                ),
                scenario.span.clone(),
            ));
        }
    }
}
