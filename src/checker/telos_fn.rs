//! M135: TelosFunction semantic checker.
//!
//! Validates `telos_function` declarations for three categories of defect:
//!
//! 1. **Completeness** — warns when `measured_by` or `guides` is absent (telos without
//!    metric is a mere prose statement with no enforcement path).
//!
//! 2. **Threshold coherence** — when `thresholds` is present, verifies that
//!    `convergence > divergence` (inverted thresholds make convergence semantics
//!    incoherent) and that `warning` is strictly between divergence and convergence.
//!
//! 3. **Propagation bound** — when `propagation` is present, it must be >= convergence
//!    (you cannot propagate from a state below convergence).

use crate::ast::{Item, Module};
use crate::error::LoomError;
use crate::checker::LoomChecker;

pub struct TelosFunctionChecker;

impl TelosFunctionChecker {
    /// Construct a new [`TelosFunctionChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `telos_function` declarations in the module.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::TelosFunction(tf) = item {
                self.check_completeness(tf, &mut errors);
                self.check_threshold_coherence(tf, &mut errors);
            }
        }
        errors
    }

    // ── Completeness ──────────────────────────────────────────────────────────

    fn check_completeness(
        &self,
        tf: &crate::ast::TelosFunctionDef,
        errors: &mut Vec<LoomError>,
    ) {
        // measured_by is required for TelosMetric to be a typed function.
        // Without it the telos is just a string — no convergence can be tracked.
        if tf.measured_by.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "telos_function '{}': missing 'measured_by' — \
                     a telos without a metric function cannot be evaluated; \
                     add: measured_by: \"<InputType> -> Float\"",
                    tf.name
                ),
                tf.span.clone(),
            ));
        }

        // guides must be non-empty; otherwise the telos guides no decision axis.
        if tf.guides.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "telos_function '{}': 'guides' list is empty — \
                     declare at least one decision axis \
                     (e.g. guides: [signal_attention, experiment_selection])",
                    tf.name
                ),
                tf.span.clone(),
            ));
        }
    }

    // ── Threshold coherence ───────────────────────────────────────────────────

    fn check_threshold_coherence(
        &self,
        tf: &crate::ast::TelosFunctionDef,
        errors: &mut Vec<LoomError>,
    ) {
        let thresholds = match &tf.thresholds {
            Some(t) => t,
            None => return,
        };

        let convergence = thresholds.convergence;
        let divergence = thresholds.divergence;

        // convergence must be strictly greater than divergence
        if convergence <= divergence {
            errors.push(LoomError::type_err(
                format!(
                    "telos_function '{}': convergence threshold ({}) must be \
                     strictly greater than divergence threshold ({}) — \
                     inverted thresholds make convergence semantics incoherent",
                    tf.name, convergence, divergence
                ),
                tf.span.clone(),
            ));
        }

        // warning (if present) must lie strictly between divergence and convergence
        if let Some(warning) = thresholds.warning {
            if warning <= divergence || warning >= convergence {
                errors.push(LoomError::type_err(
                    format!(
                        "telos_function '{}': warning threshold ({}) must be \
                         strictly between divergence ({}) and convergence ({}) — \
                         it marks the stress zone between the two extremes",
                        tf.name, warning, divergence, convergence
                    ),
                    tf.span.clone(),
                ));
            }
        }

        // propagation (if present) must be >= convergence
        if let Some(propagation) = thresholds.propagation {
            if propagation < convergence {
                errors.push(LoomError::type_err(
                    format!(
                        "telos_function '{}': propagation threshold ({}) must be \
                         >= convergence threshold ({}) — \
                         a system cannot propagate its pattern before converging",
                        tf.name, propagation, convergence
                    ),
                    tf.span.clone(),
                ));
            }
        }
    }
}

impl LoomChecker for TelosFunctionChecker {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        self.check(module)
    }
}

impl Default for TelosFunctionChecker {
    fn default() -> Self {
        Self::new()
    }
}
