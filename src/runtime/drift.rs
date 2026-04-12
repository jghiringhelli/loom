//! Telos drift engine — R2.
//!
//! The drift engine evaluates incoming signals against the declared `telos:` bounds
//! for each entity, computes a normalised 0.0–1.0 **drift score**, and emits
//! [`DriftEvent`]s when the score exceeds a configurable emission threshold.
//!
//! Score semantics:
//! - `0.0` — signal is perfectly on-target (at `telos_bounds.target`).
//! - `0.5` — signal is half-way between target and the bound (warning zone).
//! - `1.0` — signal is at or beyond the declared divergence bound.
//!
//! When no `target` is set but both `min` and `max` exist, the target is inferred
//! as the midpoint. When no bound applies the score is `0.0` (no evidence of drift).

use crate::runtime::{
    signal::{EntityId, MetricName, Signal, Timestamp, now_ms},
    store::{SignalStore, TelosBound},
};
use rusqlite::Result as SqlResult;

// ── Public types ──────────────────────────────────────────────────────────────

/// A measured drift event emitted by the [`DriftEngine`].
#[derive(Debug, Clone, PartialEq)]
pub struct DriftEvent {
    /// The entity whose telos drifted.
    pub entity_id: EntityId,
    /// The metric that triggered the event.
    pub triggering_metric: MetricName,
    /// Normalised drift score: 0.0 (on target) → 1.0 (fully diverged).
    pub score: f64,
    /// Unix-ms timestamp when the event was computed.
    pub ts: Timestamp,
}

/// Severity level derived from the drift score and configured thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    /// Score < `warning_threshold` — entity is healthy.
    Healthy,
    /// Score ≥ `warning_threshold` but < `critical_threshold`.
    Warning,
    /// Score ≥ `critical_threshold` — Tier 1 polycephalum engine should fire.
    Critical,
}

/// The drift engine.  Holds configuration thresholds and evaluates signals.
pub struct DriftEngine {
    /// Emit a [`DriftEvent`] when score ≥ this value. Default: `0.3`.
    pub emission_threshold: f64,
    /// Severity changes to `Warning` at this score. Default: `0.3`.
    pub warning_threshold: f64,
    /// Severity changes to `Critical` at this score. Default: `0.7`.
    pub critical_threshold: f64,
}

impl DriftEngine {
    /// Create a drift engine with sensible defaults.
    pub fn new() -> Self {
        Self {
            emission_threshold: 0.3,
            warning_threshold: 0.3,
            critical_threshold: 0.7,
        }
    }

    /// Evaluate a single signal against stored telos bounds for its entity.
    ///
    /// Returns `Some(DriftEvent)` if the score is ≥ `emission_threshold`.
    /// Also persists the drift event and updates the entity state in `store` when
    /// an event is emitted.
    ///
    /// # Errors
    /// Propagates SQLite errors from the store.
    pub fn evaluate(
        &self,
        signal: &Signal,
        store: &SignalStore,
    ) -> SqlResult<Option<DriftEvent>> {
        let bounds = store.telos_bounds_for_entity(&signal.entity_id)?;
        let score = self.compute_score(signal, &bounds);

        if score < self.emission_threshold {
            return Ok(None);
        }

        let ts = now_ms();
        let event = DriftEvent {
            entity_id: signal.entity_id.clone(),
            triggering_metric: signal.metric.clone(),
            score,
            ts,
        };

        store.record_drift_event(
            &event.entity_id,
            event.score,
            event.ts,
            Some(&event.triggering_metric),
        )?;

        // Escalate entity state when score crosses critical threshold.
        let new_state = match self.severity(score) {
            DriftSeverity::Warning => "warning",
            DriftSeverity::Critical => "diverging",
            DriftSeverity::Healthy => "active",
        };
        store.set_entity_state(&event.entity_id, new_state)?;

        Ok(Some(event))
    }

    /// Compute drift score for a signal value against a list of telos bounds.
    ///
    /// Only the bound whose `metric` matches `signal.metric` is used.
    /// Returns `0.0` when no matching bound is found.
    pub fn compute_score(&self, signal: &Signal, bounds: &[TelosBound]) -> f64 {
        let Some(bound) = bounds.iter().find(|b| b.metric == signal.metric) else {
            return 0.0;
        };
        score_against_bound(signal.value, bound)
    }

    /// Map a drift score to a [`DriftSeverity`] using the configured thresholds.
    pub fn severity(&self, score: f64) -> DriftSeverity {
        if score >= self.critical_threshold {
            DriftSeverity::Critical
        } else if score >= self.warning_threshold {
            DriftSeverity::Warning
        } else {
            DriftSeverity::Healthy
        }
    }

    /// Evaluate all recent signals for every entity against their telos bounds.
    ///
    /// Returns all drift events that exceeded the emission threshold.
    /// Used by the orchestration loop (R7) for batch processing.
    pub fn evaluate_all(
        &self,
        entity_ids: &[EntityId],
        store: &SignalStore,
        lookback: usize,
    ) -> SqlResult<Vec<DriftEvent>> {
        let mut events = Vec::new();
        for id in entity_ids {
            let signals = store.signals_for_entity(id, lookback)?;
            for signal in &signals {
                if let Some(event) = self.evaluate(signal, store)? {
                    events.push(event);
                }
            }
        }
        Ok(events)
    }
}

impl Default for DriftEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Compute a normalised [0.0, 1.0] drift score for a value against a bound.
///
/// The score measures how far the value is from the target (or midpoint), scaled
/// so that reaching the divergence bound → 1.0.
fn score_against_bound(value: f64, bound: &TelosBound) -> f64 {
    // Determine the reference target: explicit target > midpoint > None
    let target = match (bound.target, bound.min, bound.max) {
        (Some(t), _, _) => t,
        (None, Some(lo), Some(hi)) => (lo + hi) / 2.0,
        _ => return 0.0,
    };

    // Maximum allowed deviation: distance from target to the nearest limit.
    let max_deviation = match (bound.min, bound.max) {
        (Some(lo), Some(hi)) => f64::max((target - lo).abs(), (hi - target).abs()),
        (Some(lo), None) => (target - lo).abs(),
        (None, Some(hi)) => (hi - target).abs(),
        (None, None) => return 0.0,
    };

    if max_deviation <= 0.0 {
        return 0.0;
    }

    let deviation = (value - target).abs();
    (deviation / max_deviation).clamp(0.0, 1.0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        signal::Signal,
        store::{SignalStore, TelosBound},
    };

    fn mem_store() -> SignalStore {
        let store = SignalStore::new(":memory:").unwrap();
        store
    }

    fn make_signal(entity_id: &str, metric: &str, value: f64) -> Signal {
        Signal {
            entity_id: entity_id.into(),
            metric: metric.into(),
            value,
            timestamp: 1_000_000,
        }
    }

    fn bound_with_target(metric: &str, min: f64, max: f64, target: f64) -> TelosBound {
        TelosBound { metric: metric.into(), min: Some(min), max: Some(max), target: Some(target) }
    }

    // ── Unit: score_against_bound ─────────────────────────────────────────────

    #[test]
    fn score_is_zero_when_value_equals_target() {
        let bound = bound_with_target("temp", 0.0, 4.0, 2.0);
        let score = score_against_bound(2.0, &bound);
        assert!(score.abs() < 1e-9, "expected 0.0, got {score}");
    }

    #[test]
    fn score_is_one_when_value_reaches_limit() {
        let bound = bound_with_target("temp", 0.0, 4.0, 2.0);
        // value == max (4.0): deviation = 2.0 / max_deviation(2.0) = 1.0
        let score = score_against_bound(4.0, &bound);
        assert!((score - 1.0).abs() < 1e-9, "expected 1.0, got {score}");
    }

    #[test]
    fn score_is_half_when_value_halfway_to_limit() {
        let bound = bound_with_target("temp", 0.0, 4.0, 2.0);
        // value == 3.0: deviation = 1.0 / 2.0 = 0.5
        let score = score_against_bound(3.0, &bound);
        assert!((score - 0.5).abs() < 1e-9, "expected 0.5, got {score}");
    }

    #[test]
    fn score_is_zero_when_no_bounds() {
        let bound = TelosBound { metric: "x".into(), min: None, max: None, target: None };
        assert_eq!(score_against_bound(99.0, &bound), 0.0);
    }

    #[test]
    fn score_uses_midpoint_as_target_when_target_is_none() {
        let bound = TelosBound {
            metric: "co2".into(),
            min: Some(0.0),
            max: Some(10.0),
            target: None,
        };
        // midpoint = 5.0; value = 5.0 → deviation 0
        let s = score_against_bound(5.0, &bound);
        assert!(s.abs() < 1e-9);
        // value = 10.0 → deviation 5.0 / 5.0 = 1.0
        let s2 = score_against_bound(10.0, &bound);
        assert!((s2 - 1.0).abs() < 1e-9);
    }

    // ── Unit: DriftEngine::severity ───────────────────────────────────────────

    #[test]
    fn severity_healthy_below_warning_threshold() {
        let engine = DriftEngine::new();
        assert_eq!(engine.severity(0.1), DriftSeverity::Healthy);
        assert_eq!(engine.severity(0.0), DriftSeverity::Healthy);
    }

    #[test]
    fn severity_warning_between_thresholds() {
        let engine = DriftEngine::new();
        assert_eq!(engine.severity(0.3), DriftSeverity::Warning);
        assert_eq!(engine.severity(0.5), DriftSeverity::Warning);
        assert_eq!(engine.severity(0.69), DriftSeverity::Warning);
    }

    #[test]
    fn severity_critical_at_or_above_critical_threshold() {
        let engine = DriftEngine::new();
        assert_eq!(engine.severity(0.7), DriftSeverity::Critical);
        assert_eq!(engine.severity(1.0), DriftSeverity::Critical);
    }

    // ── Integration: evaluate writes to store and returns event ───────────────

    #[test]
    fn evaluate_emits_event_when_score_above_threshold() {
        let store = mem_store();
        store.register_entity("e1", "ClimateModel", "{}", 0).unwrap();
        store.set_telos_bounds("e1", "temp", Some(0.0), Some(4.0), Some(2.0)).unwrap();

        let signal = make_signal("e1", "temp", 3.5); // deviation = 1.5/2.0 = 0.75 → emits
        let engine = DriftEngine::new();
        let event = engine.evaluate(&signal, &store).unwrap();
        assert!(event.is_some());
        let ev = event.unwrap();
        assert_eq!(ev.entity_id, "e1");
        assert!((ev.score - 0.75).abs() < 1e-9);
    }

    #[test]
    fn evaluate_returns_none_when_score_below_threshold() {
        let store = mem_store();
        store.register_entity("e2", "EpiModel", "{}", 0).unwrap();
        store.set_telos_bounds("e2", "reproduction_rate", Some(0.0), Some(2.0), Some(1.0)).unwrap();

        let signal = make_signal("e2", "reproduction_rate", 1.1); // deviation = 0.1/1.0 = 0.1 < 0.3
        let engine = DriftEngine::new();
        let result = engine.evaluate(&signal, &store).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_persists_drift_event_in_store() {
        let store = mem_store();
        store.register_entity("e3", "SoilModel", "{}", 0).unwrap();
        store.set_telos_bounds("e3", "carbon_stock", Some(0.0), Some(100.0), Some(50.0)).unwrap();

        let signal = make_signal("e3", "carbon_stock", 90.0); // deviation = 40/50 = 0.8
        let engine = DriftEngine::new();
        engine.evaluate(&signal, &store).unwrap();

        let latest = store.latest_drift_score("e3").unwrap();
        assert!(latest.is_some());
        assert!((latest.unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn evaluate_escalates_entity_state_on_critical_score() {
        let store = mem_store();
        store.register_entity("e4", "PandemicModel", "{}", 0).unwrap();
        store.set_telos_bounds("e4", "cases", Some(0.0), Some(1000.0), Some(0.0)).unwrap();

        // value = 900 → deviation = 900/1000 = 0.9 → Critical → "diverging"
        let signal = make_signal("e4", "cases", 900.0);
        let engine = DriftEngine::new();
        engine.evaluate(&signal, &store).unwrap();

        let entities = store.all_entities().unwrap();
        let e4 = entities.iter().find(|e| e.id == "e4").unwrap();
        assert_eq!(e4.state, "diverging");
    }

    #[test]
    fn compute_score_returns_zero_for_unbound_metric() {
        let engine = DriftEngine::new();
        let signal = make_signal("e5", "unknown_metric", 999.0);
        let score = engine.compute_score(&signal, &[]);
        assert_eq!(score, 0.0);
    }
}
