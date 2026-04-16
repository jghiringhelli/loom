//! Signal — the atomic unit of telemetry emitted by a running Loom entity.

/// A unique identifier for a running entity instance.
pub type EntityId = String;

/// A named metric channel on an entity.
pub type MetricName = String;

/// Unix timestamp in milliseconds.
pub type Timestamp = u64;

/// A telemetry reading emitted by a running Loom entity.
///
/// Signals are the observable behaviour of a being at runtime. They flow
/// into the signal store and are the primary input to the telos drift engine.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// The entity that emitted this signal.
    pub entity_id: EntityId,
    /// The metric being reported (e.g. `"temperature_delta"`, `"co2_ppm"`).
    pub metric: MetricName,
    /// The numeric value of the metric at emission time.
    pub value: f64,
    /// Unix timestamp in milliseconds.
    pub timestamp: Timestamp,
}

impl Signal {
    /// Create a new signal with the current wall-clock timestamp.
    pub fn new(entity_id: impl Into<EntityId>, metric: impl Into<MetricName>, value: f64) -> Self {
        Self {
            entity_id: entity_id.into(),
            metric: metric.into(),
            value,
            timestamp: now_ms(),
        }
    }

    /// Create a signal with an explicit timestamp (useful in tests).
    pub fn with_timestamp(
        entity_id: impl Into<EntityId>,
        metric: impl Into<MetricName>,
        value: f64,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            entity_id: entity_id.into(),
            metric: metric.into(),
            value,
            timestamp,
        }
    }
}

/// Returns the current unix time in milliseconds.
pub fn now_ms() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_new_sets_entity_and_metric() {
        let s = Signal::new("e1", "temperature", 1.5);
        assert_eq!(s.entity_id, "e1");
        assert_eq!(s.metric, "temperature");
        assert_eq!(s.value, 1.5);
        assert!(s.timestamp > 0);
    }

    #[test]
    fn signal_with_timestamp_preserves_given_ts() {
        let s = Signal::with_timestamp("e2", "co2_ppm", 420.0, 999_000);
        assert_eq!(s.timestamp, 999_000);
    }

    #[test]
    fn now_ms_is_plausible() {
        let t = now_ms();
        // Any value after 2024-01-01 00:00 UTC in milliseconds
        assert!(t > 1_704_067_200_000);
    }
}
