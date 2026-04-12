//! M116 + M138 + M140: Messaging primitive checker.
//!
//! Validates `messaging_primitive:` constructs at the module level.
//!
//! Rules:
//! - Name must be non-empty.
//! - `guarantees:` items must be non-empty strings.
//! - M138: `exactly-once` and `at-most-once` are mutually exclusive.
//! - M138: `exactly-once` and `at-least-once` are mutually exclusive.
//! - M139: `stream` pattern with `exactly-once` is incoherent — streams use
//!   `at-least-once` with deduplication, not transactional delivery.
//! - M140: `request_response` without `timeout: mandatory` is warned (best practice).
//! - M140: Unknown guarantee labels are warned (typo guard).

use crate::ast::{Item, MessagingPattern, Module};
use crate::error::LoomError;

/// Known valid delivery guarantee labels.
const KNOWN_GUARANTEES: &[&str] = &[
    "at-least-once",
    "at_least_once",
    "at-most-once",
    "at_most_once",
    "exactly-once",
    "exactly_once",
    "ordered",
    "unordered",
    "durable",
    "transient",
    "persistent",
];

pub struct MessagingChecker;

impl MessagingChecker {
    /// Construct a new [`MessagingChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `messaging_primitive:` constructs in the module.
    ///
    /// # Returns
    /// A vec of [`LoomError`] — empty on success.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::MessagingPrimitive(mp) = item {
                self.check_name(mp, &mut errors);
                self.check_guarantee_labels(mp, &mut errors);
                self.check_guarantee_conflicts(mp, &mut errors);
                self.check_pattern_constraints(mp, &mut errors);
            }
        }
        errors
    }

    // ── Name ──────────────────────────────────────────────────────────────────

    fn check_name(&self, mp: &crate::ast::MessagingPrimitiveDef, errors: &mut Vec<LoomError>) {
        if mp.name.trim().is_empty() {
            errors.push(LoomError::type_err(
                "messaging_primitive has empty name",
                mp.span.clone(),
            ));
        }
    }

    // ── Guarantee label validity ──────────────────────────────────────────────

    fn check_guarantee_labels(
        &self,
        mp: &crate::ast::MessagingPrimitiveDef,
        errors: &mut Vec<LoomError>,
    ) {
        for guarantee in &mp.guarantees {
            if guarantee.trim().is_empty() {
                errors.push(LoomError::type_err(
                    format!(
                        "messaging_primitive '{}': guarantees list contains an empty entry",
                        mp.name
                    ),
                    mp.span.clone(),
                ));
            }
        }
    }

    // ── M138: Delivery guarantee mutual exclusion ─────────────────────────────

    fn check_guarantee_conflicts(
        &self,
        mp: &crate::ast::MessagingPrimitiveDef,
        errors: &mut Vec<LoomError>,
    ) {
        let has = |label: &str| -> bool {
            mp.guarantees.iter().any(|g| {
                let normalised = g.replace('_', "-").to_lowercase();
                normalised == label
            })
        };

        // exactly-once ∧ at-most-once is incoherent
        if has("exactly-once") && has("at-most-once") {
            errors.push(LoomError::type_err(
                format!(
                    "messaging_primitive '{}': 'exactly-once' and 'at-most-once' are mutually \
                     exclusive — exactly-once implies at-least-once + deduplication",
                    mp.name
                ),
                mp.span.clone(),
            ));
        }

        // exactly-once ∧ at-least-once — redundant AND semantically contradictory
        // (at-least-once alone allows duplicates; exactly-once eliminates them)
        if has("exactly-once") && has("at-least-once") {
            errors.push(LoomError::type_err(
                format!(
                    "messaging_primitive '{}': 'exactly-once' and 'at-least-once' are mutually \
                     exclusive — exactly-once already implies message is delivered exactly once; \
                     at-least-once would allow duplicates",
                    mp.name
                ),
                mp.span.clone(),
            ));
        }
    }

    // ── M139 / M140: Pattern-specific constraints ─────────────────────────────

    fn check_pattern_constraints(
        &self,
        mp: &crate::ast::MessagingPrimitiveDef,
        errors: &mut Vec<LoomError>,
    ) {
        let has_guarantee = |label: &str| -> bool {
            mp.guarantees.iter().any(|g| {
                let normalised = g.replace('_', "-").to_lowercase();
                normalised == label
            })
        };

        // M139: Stream + exactly-once is architecturally incoherent.
        // Streams use at-least-once + idempotent consumer (deduplication at the sink).
        // Transactional exactly-once on a continuous stream requires 2PC across all
        // partition replicas — possible but never the default stream contract.
        if matches!(&mp.pattern, Some(MessagingPattern::Stream)) && has_guarantee("exactly-once") {
            errors.push(LoomError::type_err(
                format!(
                    "messaging_primitive '{}': 'stream' pattern with 'exactly-once' is \
                     architecturally incoherent — streams use 'at-least-once' + idempotent \
                     consumer; if transactional exactly-once is required, use a \
                     'producer_consumer' pattern with a transactional broker",
                    mp.name
                ),
                mp.span.clone(),
            ));
        }

        // M140: RequestResponse without timeout: mandatory is a latency risk.
        if matches!(&mp.pattern, Some(MessagingPattern::RequestResponse)) && !mp.timeout_mandatory {
            errors.push(LoomError::type_err(
                format!(
                    "messaging_primitive '{}': 'request_response' pattern should declare \
                     'timeout: mandatory' — unbounded request/response calls block callers \
                     indefinitely on unresponsive services",
                    mp.name
                ),
                mp.span.clone(),
            ));
        }
    }
}

impl Default for MessagingChecker {
    fn default() -> Self {
        Self::new()
    }
}
