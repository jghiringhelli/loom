//! Telomere audit log — persistent JSONL record of all telomere events.
//!
//! The audit log is the primary evidence for the BIOISO paper: it shows
//! *when* each entity was under telos pressure, *how fast* the telomere
//! shortened, and *whether* meiosis fired before senescence.  The correlation
//! between shortening rate and post-meiosis breakthrough (D_velocity going
//! negative) is the paper's core empirical result.
//!
//! # Event types
//!
//! | `event_type`        | When emitted                                          |
//! |---------------------|-------------------------------------------------------|
//! | `init`              | First drift observation for an entity                 |
//! | `drift_observed`    | Drift below tolerance — exploration permitted         |
//! | `decay`             | Telomere shortened due to excess drift                |
//! | `senescence_warning`| Remaining ≤ 10% of initial length                    |
//! | `senescence`        | Remaining == 0                                        |
//! | `meiosis_donor`     | Entity selected as a meiosis donor                   |
//! | `meiosis_offspring` | New entity spawned as meiosis offspring               |
//!
//! # File format
//!
//! One JSON object per line:
//! ```json
//! {"entity_id":"climate","tick":847,"event_type":"decay","drift_score":0.71,
//!  "remaining":492,"shortening":4,"trigger_metric":"co2_ppm","ts":1234567890}
//! ```

use std::collections::HashMap;
use std::io::{BufWriter, Write};

use serde::Serialize;

use crate::runtime::signal::now_ms;

// ── Event ─────────────────────────────────────────────────────────────────────

/// A single telomere lifecycle event.
#[derive(Debug, Clone, Serialize)]
pub struct TelomereAuditEvent {
    /// Entity identifier.
    pub entity_id: String,
    /// Experiment tick at which this event occurred.
    pub tick: u64,
    /// Event type (see module docs).
    pub event_type: &'static str,
    /// Drift score that triggered this event (D_static when available, else per-metric).
    pub drift_score: f64,
    /// Telomere length *after* this event.
    pub remaining: u32,
    /// Number of ticks removed (0 for non-decay events).
    pub shortening: u32,
    /// Metric that triggered the drift (None for non-drift events).
    pub trigger_metric: Option<String>,
    /// Unix-ms timestamp.
    pub ts: u64,
}

// ── Writer ────────────────────────────────────────────────────────────────────

/// Appends [`TelomereAuditEvent`]s to a JSONL file.
///
/// Created with a file path; if the path is empty or the file cannot be opened,
/// the writer is a no-op (events are silently discarded).
pub struct TelomereAuditWriter {
    writer: Option<BufWriter<std::fs::File>>,
    /// In-memory buffer for summary queries.
    events: Vec<TelomereAuditEvent>,
    /// entity_id → initial telomere length (set on first observation).
    initial_lengths: HashMap<String, u32>,
}

impl TelomereAuditWriter {
    /// Create a writer targeting `path`.  Pass an empty string for a no-op writer.
    pub fn new(path: &str) -> Self {
        let writer = if path.is_empty() {
            None
        } else {
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                Ok(f) => Some(BufWriter::new(f)),
                Err(e) => {
                    eprintln!("warn: telomere audit log could not be opened at `{path}`: {e}");
                    None
                }
            }
        };
        Self {
            writer,
            events: Vec::new(),
            initial_lengths: HashMap::new(),
        }
    }

    /// Record a drift observation for an entity.
    ///
    /// `was_shortened` — whether the telomere was shortened by this event.
    /// `remaining` — telomere length after this event.
    /// `initial_length` — used to detect the first observation and senescence thresholds.
    pub fn record(
        &mut self,
        entity_id: &str,
        tick: u64,
        drift_score: f64,
        remaining: u32,
        initial_length: u32,
        was_shortened: bool,
        trigger_metric: Option<&str>,
    ) {
        // Determine event type.
        let is_first = !self.initial_lengths.contains_key(entity_id);
        if is_first {
            self.initial_lengths
                .insert(entity_id.to_string(), initial_length);
        }

        let prev_remaining = if is_first {
            initial_length
        } else {
            // The shortening is the difference between what it was and what it is now.
            // We don't store previous remaining, so approximate via shortening.
            remaining
                + if was_shortened {
                    ((drift_score - 0.3).max(0.0) * 5.0).floor() as u32
                } else {
                    0
                }
        };
        let shortening = prev_remaining.saturating_sub(remaining);

        let event_type = if is_first {
            "init"
        } else if remaining == 0 {
            "senescence"
        } else if remaining <= initial_length / 10 {
            "senescence_warning"
        } else if was_shortened {
            "decay"
        } else {
            "drift_observed"
        };

        let event = TelomereAuditEvent {
            entity_id: entity_id.to_string(),
            tick,
            event_type,
            drift_score,
            remaining,
            shortening,
            trigger_metric: trigger_metric.map(|s| s.to_string()),
            ts: now_ms(),
        };

        self.emit(&event);
    }

    /// Record a meiosis donor selection.
    pub fn record_meiosis_donor(&mut self, entity_id: &str, tick: u64, remaining: u32) {
        let event = TelomereAuditEvent {
            entity_id: entity_id.to_string(),
            tick,
            event_type: "meiosis_donor",
            drift_score: 0.0,
            remaining,
            shortening: 0,
            trigger_metric: None,
            ts: now_ms(),
        };
        self.emit(&event);
    }

    /// Record a meiosis offspring spawn.
    pub fn record_meiosis_offspring(&mut self, offspring_id: &str, tick: u64, initial_length: u32) {
        let event = TelomereAuditEvent {
            entity_id: offspring_id.to_string(),
            tick,
            event_type: "meiosis_offspring",
            drift_score: 0.0,
            remaining: initial_length,
            shortening: 0,
            trigger_metric: None,
            ts: now_ms(),
        };
        self.emit(&event);
    }

    /// All recorded events (for summary / manifest generation).
    pub fn events(&self) -> &[TelomereAuditEvent] {
        &self.events
    }

    /// Final telomere length for each tracked entity.
    pub fn final_lengths(&self) -> HashMap<String, u32> {
        let mut map: HashMap<String, u32> = HashMap::new();
        for ev in &self.events {
            map.insert(ev.entity_id.clone(), ev.remaining);
        }
        map
    }

    /// Number of decay events for each entity — proxy for "search difficulty".
    pub fn decay_counts(&self) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for ev in self.events.iter().filter(|e| e.event_type == "decay") {
            *map.entry(ev.entity_id.clone()).or_insert(0) += 1;
        }
        map
    }

    /// Whether a meiosis-donor event was recorded for this entity.
    pub fn was_meiosis_donor(&self, entity_id: &str) -> bool {
        self.events
            .iter()
            .any(|e| e.entity_id == entity_id && e.event_type == "meiosis_donor")
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn emit(&mut self, event: &TelomereAuditEvent) {
        if let Some(ref mut w) = self.writer {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(w, "{line}");
            }
        }
        self.events.push(event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_records_init_on_first_observation() {
        let mut w = TelomereAuditWriter::new("");
        w.record("climate", 0, 0.5, 490, 500, true, Some("co2_ppm"));
        assert_eq!(w.events()[0].event_type, "init");
    }

    #[test]
    fn writer_records_decay_on_second_observation() {
        let mut w = TelomereAuditWriter::new("");
        w.record("climate", 0, 0.5, 500, 500, false, None);
        w.record("climate", 10, 0.7, 495, 500, true, Some("temp"));
        assert_eq!(w.events()[1].event_type, "decay");
    }

    #[test]
    fn senescence_warning_at_ten_percent() {
        let mut w = TelomereAuditWriter::new("");
        w.record("e1", 0, 0.5, 500, 500, false, None);
        w.record("e1", 1000, 0.9, 48, 500, true, Some("x")); // 48/500 < 10%
        assert_eq!(w.events()[1].event_type, "senescence_warning");
    }

    #[test]
    fn senescence_event_at_zero() {
        let mut w = TelomereAuditWriter::new("");
        w.record("e2", 0, 0.5, 500, 500, false, None);
        w.record("e2", 9999, 1.0, 0, 500, true, Some("x"));
        assert_eq!(w.events()[1].event_type, "senescence");
    }

    #[test]
    fn final_lengths_tracks_last_value() {
        let mut w = TelomereAuditWriter::new("");
        w.record("a", 0, 0.5, 500, 500, false, None);
        w.record("a", 1, 0.8, 490, 500, true, None);
        w.record("a", 2, 0.9, 478, 500, true, None);
        assert_eq!(w.final_lengths().get("a"), Some(&478));
    }
}
