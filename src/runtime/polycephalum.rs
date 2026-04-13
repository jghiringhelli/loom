//! Polycephalum — Tier 1 deterministic rule engine (R3-2, R3-3).
//!
//! Named after *Physarum polycephalum* (slime mould): a massively parallel,
//! network-forming organism that solves optimisation problems by gradient
//! following with no central coordination.
//!
//! The Polycephalum engine operates locally on each entity: it reads the most
//! recent [`DriftEvent`] for the entity, matches it against a registry of
//! configurable [`Rule`]s, and emits ranked [`MutationProposal`]s.
//!
//! Properties:
//! - No network calls.
//! - Deterministic (same inputs → same proposals).
//! - Completes in < 50 ms for up to 1 000 rules.
//! - Escalates to Tier 2 (Ganglion) when no rule matches or proposals fail the gate.

use std::collections::HashMap;

use crate::runtime::{
    drift::{DriftEvent, DriftSeverity},
    mutation::MutationProposal,
    sampler::MutationSampler,
    signal::EntityId,
};

// ── Rule types ────────────────────────────────────────────────────────────────

/// Condition under which a rule fires.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleCondition {
    /// The metric name to match (exact string match).
    pub metric: String,
    /// Minimum drift score required to fire.  Default: `0.0`.
    pub min_score: f64,
    /// Maximum drift score below which this rule fires.  Default: `1.01` (always).
    pub max_score: f64,
    /// Minimum severity required.  Default: `Healthy` (matches all).
    pub min_severity: DriftSeverity,
}

impl RuleCondition {
    /// Create a condition that matches any drift event for `metric`.
    pub fn for_metric(metric: impl Into<String>) -> Self {
        Self {
            metric: metric.into(),
            min_score: 0.0,
            max_score: 1.01,
            min_severity: DriftSeverity::Healthy,
        }
    }

    /// Check whether this condition matches the given drift event.
    pub fn matches(&self, event: &DriftEvent, severity: DriftSeverity) -> bool {
        event.triggering_metric == self.metric
            && event.score >= self.min_score
            && event.score <= self.max_score
            && severity_ge(severity, self.min_severity)
    }
}

/// A single rule in the Polycephalum registry.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Human-readable name for debugging and audit logs.
    pub name: String,
    /// The condition that must hold for this rule to fire.
    pub condition: RuleCondition,
    /// The action to take when the rule fires.
    pub action: RuleAction,
    /// Priority — higher values are tried first and rank higher in proposals.
    pub priority: i32,
}

/// The action a rule emits when its condition is satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleAction {
    /// Adjust a parameter by a fixed delta (gradient step).
    AdjustParam {
        /// The parameter to adjust.
        param: String,
        /// How to compute the delta: `Fixed(value)` or `Proportional(factor)`.
        delta: DeltaSpec,
    },
    /// Prune the entity — remove it from the ecosystem.
    PruneEntity { reason: String },
    /// Roll the entity back to its latest checkpoint.
    RollbackToCheckpoint { reason: String },
}

/// Specifies how a parameter delta is calculated.
#[derive(Debug, Clone, PartialEq)]
pub enum DeltaSpec {
    /// A constant delta regardless of the drift score.
    Fixed(f64),
    /// Delta = `factor * drift_score` — proportional to observed drift.
    Proportional(f64),
    /// Use the [`MutationSampler`] — combines telos guidance force with stochastic noise.
    ///
    /// Falls back to guidance-only when no sampler context is available.
    Sampled {
        /// Declared telos target for this parameter.
        target: f64,
        /// Hard bounds `(min, max)` from the telos declaration.
        bounds: (f64, f64),
    },
}

impl DeltaSpec {
    /// Evaluate the delta given the current drift score (no sampler — deterministic only).
    ///
    /// For `Sampled`, returns the guidance-force component without stochastic noise.
    pub fn evaluate(&self, drift_score: f64) -> f64 {
        match self {
            Self::Fixed(v) => *v,
            Self::Proportional(f) => f * drift_score,
            Self::Sampled { target, bounds } => {
                // Guidance-force fallback: normalised pull toward target scaled by drift.
                let width = (bounds.1 - bounds.0).abs().max(1e-12);
                (*target / width) * drift_score * 0.4 // 0.4 = DEFAULT_LEARNING_RATE
            }
        }
    }

    /// Evaluate using the sampler when context is available.
    ///
    /// `current` is the current observed value of the parameter (from Epigenome Working tier).
    /// `relative_telomere` is the entity's telomere fraction [0, 1].
    pub fn evaluate_with_sampler(
        &self,
        drift_score: f64,
        current: f64,
        sampler: &mut MutationSampler,
        relative_telomere: f64,
    ) -> f64 {
        match self {
            Self::Fixed(v) => *v,
            Self::Proportional(f) => f * drift_score,
            Self::Sampled { target, bounds } => {
                sampler.sample(current, *target, *bounds, drift_score, relative_telomere)
            }
        }
    }
}

// ── Rule registry ─────────────────────────────────────────────────────────────

/// The registry of rules for the Polycephalum engine.
///
/// Rules are stored per-entity-id.  A `"*"` key matches all entities (global rules).
pub struct RuleRegistry {
    /// Rules keyed by entity id.  `"*"` is the global fallback.
    rules: Vec<(Option<EntityId>, Rule)>,
}

impl RuleRegistry {
    /// Create an empty rule registry.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a rule that applies only to `entity_id`.
    pub fn add_for_entity(&mut self, entity_id: impl Into<EntityId>, rule: Rule) {
        self.rules.push((Some(entity_id.into()), rule));
    }

    /// Register a rule that applies to all entities (global rule).
    pub fn add_global(&mut self, rule: Rule) {
        self.rules.push((None, rule));
    }

    /// Return all rules that apply to `entity_id`, sorted by descending priority.
    pub fn rules_for(&self, entity_id: &str) -> Vec<&Rule> {
        let mut matched: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|(eid, _)| match eid {
                None => true,
                Some(id) => id == entity_id,
            })
            .map(|(_, rule)| rule)
            .collect();
        matched.sort_by(|a, b| b.priority.cmp(&a.priority));
        matched
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Polycephalum engine ───────────────────────────────────────────────────────

/// Tier 1 synthesis engine — deterministic, local, no network.
pub struct Polycephalum {
    /// Rule registry used by this engine.
    pub registry: RuleRegistry,
    /// Maximum number of proposals to generate per drift event. Default: 3.
    pub max_proposals: usize,
}

impl Polycephalum {
    /// Create a new engine with an empty rule registry.
    pub fn new() -> Self {
        Self {
            registry: RuleRegistry::new(),
            max_proposals: 3,
        }
    }

    /// Create an engine pre-loaded with the given registry.
    pub fn with_registry(registry: RuleRegistry) -> Self {
        Self { registry, max_proposals: 3 }
    }

    /// Evaluate a drift event and return ranked mutation proposals.
    ///
    /// Rules are evaluated in priority order.  At most `self.max_proposals` proposals
    /// are returned.  Returns an empty vec if no rule matches — the caller should
    /// escalate to Tier 2 in that case.
    pub fn evaluate(
        &self,
        event: &DriftEvent,
        severity: DriftSeverity,
        checkpoint_id: Option<i64>,
    ) -> Vec<MutationProposal> {
        let rules = self.registry.rules_for(&event.entity_id);
        let mut proposals = Vec::new();

        for rule in rules {
            if proposals.len() >= self.max_proposals {
                break;
            }
            if !rule.condition.matches(event, severity) {
                continue;
            }
            if let Some(proposal) = self.apply_action(event, &rule.action, checkpoint_id) {
                proposals.push(proposal);
            }
        }
        proposals
    }

    /// Evaluate with sampler context — `Sampled` deltas call the biased stochastic sampler.
    ///
    /// `current_values` maps metric names to current observed parameter values (from
    /// Epigenome Working tier). `relative_telomere` governs exploration temperature.
    pub fn evaluate_with_sampler(
        &self,
        event: &DriftEvent,
        severity: DriftSeverity,
        checkpoint_id: Option<i64>,
        sampler: &mut MutationSampler,
        current_values: &HashMap<String, f64>,
        relative_telomere: f64,
    ) -> Vec<MutationProposal> {
        let rules = self.registry.rules_for(&event.entity_id);
        let mut proposals = Vec::new();

        for rule in rules {
            if proposals.len() >= self.max_proposals {
                break;
            }
            if !rule.condition.matches(event, severity) {
                continue;
            }
            if let Some(proposal) = self.apply_action_sampled(
                event,
                &rule.action,
                checkpoint_id,
                sampler,
                current_values,
                relative_telomere,
            ) {
                proposals.push(proposal);
            }
        }
        proposals
    }

    fn apply_action(
        &self,
        event: &DriftEvent,
        action: &RuleAction,
        checkpoint_id: Option<i64>,
    ) -> Option<MutationProposal> {
        match action {
            RuleAction::AdjustParam { param, delta } => Some(MutationProposal::ParameterAdjust {
                entity_id: event.entity_id.clone(),
                param: param.clone(),
                delta: delta.evaluate(event.score),
                reason: format!(
                    "polycephalum: drift score {:.3} on metric '{}'",
                    event.score, event.triggering_metric
                ),
            }),
            RuleAction::PruneEntity { reason } => Some(MutationProposal::EntityPrune {
                entity_id: event.entity_id.clone(),
                reason: reason.clone(),
            }),
            RuleAction::RollbackToCheckpoint { reason } => checkpoint_id.map(|id| {
                MutationProposal::EntityRollback {
                    entity_id: event.entity_id.clone(),
                    checkpoint_id: id,
                    reason: reason.clone(),
                }
            }),
        }
    }

    fn apply_action_sampled(
        &self,
        event: &DriftEvent,
        action: &RuleAction,
        checkpoint_id: Option<i64>,
        sampler: &mut MutationSampler,
        current_values: &HashMap<String, f64>,
        relative_telomere: f64,
    ) -> Option<MutationProposal> {
        match action {
            RuleAction::AdjustParam { param, delta } => {
                let current = current_values.get(param).copied().unwrap_or(0.0);
                let computed = delta.evaluate_with_sampler(
                    event.score,
                    current,
                    sampler,
                    relative_telomere,
                );
                Some(MutationProposal::ParameterAdjust {
                    entity_id: event.entity_id.clone(),
                    param: param.clone(),
                    delta: computed,
                    reason: format!(
                        "polycephalum[sampled]: drift score {:.3} on metric '{}'",
                        event.score, event.triggering_metric
                    ),
                })
            }
            RuleAction::PruneEntity { reason } => Some(MutationProposal::EntityPrune {
                entity_id: event.entity_id.clone(),
                reason: reason.clone(),
            }),
            RuleAction::RollbackToCheckpoint { reason } => checkpoint_id.map(|id| {
                MutationProposal::EntityRollback {
                    entity_id: event.entity_id.clone(),
                    checkpoint_id: id,
                    reason: reason.clone(),
                }
            }),
        }
    }
}

impl Default for Polycephalum {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn severity_ge(a: DriftSeverity, b: DriftSeverity) -> bool {
    severity_rank(a) >= severity_rank(b)
}

fn severity_rank(s: DriftSeverity) -> u8 {
    match s {
        DriftSeverity::Healthy => 0,
        DriftSeverity::Warning => 1,
        DriftSeverity::Critical => 2,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::drift::DriftEvent;

    fn make_event(entity_id: &str, metric: &str, score: f64) -> DriftEvent {
        DriftEvent {
            entity_id: entity_id.into(),
            triggering_metric: metric.into(),
            score,
            ts: 1_000_000,
        }
    }

    fn adjust_rule(metric: &str, param: &str, delta: f64, priority: i32) -> Rule {
        Rule {
            name: format!("adjust_{param}"),
            condition: RuleCondition {
                metric: metric.into(),
                min_score: 0.3,
                max_score: 1.01,
                min_severity: DriftSeverity::Warning,
            },
            action: RuleAction::AdjustParam {
                param: param.into(),
                delta: DeltaSpec::Fixed(delta),
            },
            priority,
        }
    }

    #[test]
    fn engine_produces_parameter_adjust_when_rule_matches() {
        let mut engine = Polycephalum::new();
        engine.registry.add_for_entity(
            "climate_1",
            adjust_rule("temperature", "albedo", -0.01, 10),
        );

        let event = make_event("climate_1", "temperature", 0.75);
        let proposals = engine.evaluate(&event, DriftSeverity::Critical, None);
        assert_eq!(proposals.len(), 1);
        match &proposals[0] {
            MutationProposal::ParameterAdjust { param, delta, .. } => {
                assert_eq!(param, "albedo");
                assert!((delta - (-0.01)).abs() < 1e-9);
            }
            other => panic!("unexpected proposal: {other:?}"),
        }
    }

    #[test]
    fn engine_returns_empty_when_no_rule_matches() {
        let engine = Polycephalum::new();
        let event = make_event("x", "unknown_metric", 0.9);
        let proposals = engine.evaluate(&event, DriftSeverity::Critical, None);
        assert!(proposals.is_empty());
    }

    #[test]
    fn engine_respects_max_proposals_limit() {
        let mut engine = Polycephalum::new();
        engine.max_proposals = 2;
        for i in 0..5 {
            engine.registry.add_global(adjust_rule("temp", &format!("p{i}"), 0.01 * i as f64, i));
        }
        let event = make_event("any_entity", "temp", 0.8);
        let proposals = engine.evaluate(&event, DriftSeverity::Critical, None);
        assert_eq!(proposals.len(), 2);
    }

    #[test]
    fn rule_condition_does_not_match_below_min_score() {
        let cond = RuleCondition {
            metric: "temp".into(),
            min_score: 0.5,
            max_score: 1.01,
            min_severity: DriftSeverity::Warning,
        };
        let low_event = make_event("e1", "temp", 0.2);
        assert!(!cond.matches(&low_event, DriftSeverity::Warning));
    }

    #[test]
    fn prune_rule_produces_entity_prune_proposal() {
        let mut engine = Polycephalum::new();
        engine.registry.add_for_entity(
            "sick",
            Rule {
                name: "prune_on_critical".into(),
                condition: RuleCondition {
                    metric: "health".into(),
                    min_score: 0.9,
                    max_score: 1.01,
                    min_severity: DriftSeverity::Critical,
                },
                action: RuleAction::PruneEntity {
                    reason: "unrecoverable divergence".into(),
                },
                priority: 100,
            },
        );

        let event = make_event("sick", "health", 0.95);
        let proposals = engine.evaluate(&event, DriftSeverity::Critical, None);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], MutationProposal::EntityPrune { .. }));
    }

    #[test]
    fn rollback_rule_requires_checkpoint_id() {
        let mut engine = Polycephalum::new();
        engine.registry.add_global(Rule {
            name: "rollback".into(),
            condition: RuleCondition::for_metric("stability"),
            action: RuleAction::RollbackToCheckpoint {
                reason: "restore last known good".into(),
            },
            priority: 5,
        });

        let event = make_event("e1", "stability", 0.6);

        // Without checkpoint id → no proposal
        let no_cp = engine.evaluate(&event, DriftSeverity::Warning, None);
        assert!(no_cp.is_empty());

        // With checkpoint id → rollback proposal
        let with_cp = engine.evaluate(&event, DriftSeverity::Warning, Some(42));
        assert_eq!(with_cp.len(), 1);
        assert!(matches!(
            with_cp[0],
            MutationProposal::EntityRollback { checkpoint_id: 42, .. }
        ));
    }

    #[test]
    fn proportional_delta_scales_with_score() {
        let spec = DeltaSpec::Proportional(10.0);
        assert!((spec.evaluate(0.5) - 5.0).abs() < 1e-9);
        assert!((spec.evaluate(0.8) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn global_rule_applies_to_any_entity() {
        let mut engine = Polycephalum::new();
        engine
            .registry
            .add_global(adjust_rule("pressure", "flow_rate", 0.05, 1));

        for id in ["entity_a", "entity_b", "entity_c"] {
            let event = make_event(id, "pressure", 0.7);
            let proposals = engine.evaluate(&event, DriftSeverity::Critical, None);
            assert_eq!(proposals.len(), 1, "expected proposal for {id}");
        }
    }

    #[test]
    fn rules_sorted_by_priority_descending() {
        let mut registry = RuleRegistry::new();
        registry.add_global(adjust_rule("x", "low", 0.01, 1));
        registry.add_global(adjust_rule("x", "high", 0.01, 99));
        registry.add_global(adjust_rule("x", "mid", 0.01, 50));

        let rules = registry.rules_for("anything");
        assert_eq!(rules[0].priority, 99);
        assert_eq!(rules[1].priority, 50);
        assert_eq!(rules[2].priority, 1);
    }

    #[test]
    fn sampled_delta_uses_sampler_and_moves_toward_target() {
        use crate::runtime::sampler::MutationSampler;

        let mut engine = Polycephalum::new();
        engine.registry.add_for_entity(
            "climate_1",
            Rule {
                name: "reduce_co2".into(),
                condition: RuleCondition::for_metric("co2_ppm"),
                action: RuleAction::AdjustParam {
                    param: "co2_ppm".into(),
                    delta: DeltaSpec::Sampled {
                        target: 350.0,
                        bounds: (300.0, 500.0),
                    },
                },
                priority: 10,
            },
        );

        let event = make_event("climate_1", "co2_ppm", 0.8);
        let mut sampler = MutationSampler::with_seed(42);
        let current_values: HashMap<String, f64> =
            [("co2_ppm".to_string(), 420.0)].into_iter().collect();

        let proposals = engine.evaluate_with_sampler(
            &event,
            DriftSeverity::Critical,
            None,
            &mut sampler,
            &current_values,
            0.8,
        );
        assert_eq!(proposals.len(), 1);
        match &proposals[0] {
            MutationProposal::ParameterAdjust { param, delta, reason, .. } => {
                assert_eq!(param, "co2_ppm");
                // Should be negative (current=420 > target=350)
                assert!(*delta < 0.0, "expected negative delta, got {delta}");
                assert!(reason.contains("sampled"), "expected 'sampled' tag in reason");
            }
            other => panic!("unexpected proposal: {other:?}"),
        }
    }

    #[test]
    fn sampled_delta_evaluate_fallback_is_nonzero_on_drift() {
        let spec = DeltaSpec::Sampled { target: 100.0, bounds: (0.0, 200.0) };
        let fallback = spec.evaluate(0.8);
        assert!(fallback != 0.0, "guidance fallback should be nonzero when drifted");
    }
}
