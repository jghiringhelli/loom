//! Epigenome — cross-cutting E axis of the CEMS runtime.
//!
//! The Epigenome is the institutional memory of a Loom entity. It mirrors how
//! epigenetic marks in biology record *how genes were expressed* without altering
//! the genome itself, enabling rapid adaptation within a lineage.
//!
//! # Three-tier architecture
//!
//! | Tier     | Biological analogue          | Lifespan       | Write path              |
//! |----------|------------------------------|----------------|-------------------------|
//! | Buffer   | Histone marks                | Decays (hours) | Every admitted signal   |
//! | Working  | Short-term chromatin state   | Rolling window | Automatic compression   |
//! | Core     | DNA methylation (long-term)  | Permanent       | Ganglion / Cortex write |
//!
//! A fourth tier — **Security** — is conceptually part of Core but seeded exclusively
//! by the Membrane immune layer (Stage 0) and used by the Reflex stage (Stage 1) for
//! Bayesian allostery.
//!
//! # Five memory types (Chronicle-inspired)
//!
//! Each entry carries a [`MemoryType`] tag that governs how downstream stages
//! interpret and weight the information:
//!
//! - **Episodic**: "X happened at time T" — Buffer entries, security events
//! - **Semantic**: "X tends to be Y" — Working tier summaries, patterns
//! - **Procedural**: "When X, do Y" — Core rules, distilled Polycephalum rules
//! - **Relational**: "X correlates with Y across entities" — Mycelium writes Core
//! - **Declarative**: "X is defined as Y" — telos bounds, entity genome hashes
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md) §E-axis.

use std::collections::{HashMap, VecDeque};

use crate::runtime::signal::{EntityId, MetricName, Timestamp};
use crate::runtime::store::{SecurityEvent, SignalStore};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default Buffer entry TTL: 4 hours in milliseconds.
pub const BUFFER_TTL_MS: Timestamp = 4 * 60 * 60 * 1_000;

/// Default Working window length (number of signals to summarise).
pub const WORKING_WINDOW_SIZE: usize = 100;

/// Maximum Core entries kept in-memory per entity (oldest evicted first).
pub const CORE_MAX_ENTRIES: usize = 1_000;

// ── Memory type ───────────────────────────────────────────────────────────────

/// Biological memory type tag.
///
/// Controls how downstream stages (Reflex, Ganglion, Cortex) weight each entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    /// "X happened at time T" — raw temporal record.
    Episodic,
    /// "X tends to be Y" — statistical pattern derived from repeated observations.
    Semantic,
    /// "When X, do Y" — distilled action rule.
    Procedural,
    /// "X correlates with Y across entities" — cross-entity structural insight.
    Relational,
    /// "X is defined as Y" — constitutive fact (telos, genome, schema).
    Declarative,
}

impl MemoryType {
    /// String representation for serialisation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
            MemoryType::Relational => "relational",
            MemoryType::Declarative => "declarative",
        }
    }
}

// ── Epigenome tiers ───────────────────────────────────────────────────────────

/// A single Buffer entry — a raw episodic record of one signal outcome.
///
/// Entries decay after [`BUFFER_TTL_MS`] and are never written to persistent
/// storage (intentional: the Buffer is hot, in-process memory only).
#[derive(Debug, Clone)]
pub struct BufferEntry {
    /// Entity that emitted the signal.
    pub entity_id: EntityId,
    /// Metric name.
    pub metric: MetricName,
    /// Observed value.
    pub value: f64,
    /// Wall-clock timestamp of the signal.
    pub ts: Timestamp,
    /// Drift score at the time of admission (0.0 = on-target, 1.0 = max drift).
    pub drift_score: f64,
}

/// Rolling summary for one metric on one entity — the Working tier.
///
/// Maintained as a circular buffer. When the window is full the oldest entry is
/// dropped and the running statistics are updated in O(1).
#[derive(Debug, Clone)]
pub struct WorkingSummary {
    /// Metric name.
    pub metric: MetricName,
    /// Maximum number of observations to retain.
    capacity: usize,
    /// Most recent values in arrival order.
    pub window: VecDeque<f64>,
    /// Running sum (for O(1) mean update).
    sum: f64,
    /// Running sum-of-squares (for O(1) variance update).
    sum_sq: f64,
    /// Timestamp of the most recently observed value.
    pub last_ts: Timestamp,
}

impl WorkingSummary {
    /// Create an empty summary for a metric with the default window size.
    pub fn new(metric: impl Into<MetricName>) -> Self {
        Self::new_with_window(metric, WORKING_WINDOW_SIZE)
    }

    /// Ingest a new observation. Evicts the oldest value when the window is full.
    pub fn push(&mut self, value: f64, ts: Timestamp) {
        if self.window.len() == self.capacity {
            let evicted = self.window.pop_front().unwrap_or(0.0);
            self.sum -= evicted;
            self.sum_sq -= evicted * evicted;
        }
        self.window.push_back(value);
        self.sum += value;
        self.sum_sq += value * value;
        self.last_ts = ts;
    }

    /// Arithmetic mean of the current window. Returns `None` when empty.
    pub fn mean(&self) -> Option<f64> {
        if self.window.is_empty() {
            return None;
        }
        Some(self.sum / self.window.len() as f64)
    }

    /// Population variance of the current window. Returns `None` when fewer than 2 values.
    pub fn variance(&self) -> Option<f64> {
        let n = self.window.len();
        if n < 2 {
            return None;
        }
        let mean = self.sum / n as f64;
        Some(self.sum_sq / n as f64 - mean * mean)
    }

    /// Standard deviation. `None` when variance is `None` or negative (float rounding).
    pub fn std_dev(&self) -> Option<f64> {
        self.variance().and_then(|v| if v >= 0.0 { Some(v.sqrt()) } else { None })
    }

    /// Number of observations currently in the window.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// `true` when no observations have been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }
}

/// A long-term Core memory entry — written explicitly by Ganglion or Cortex.
///
/// Core entries persist for the lifetime of the entity (bounded by [`CORE_MAX_ENTRIES`]
/// to prevent unbounded growth — oldest entries are evicted).
#[derive(Debug, Clone)]
pub struct CoreEntry {
    /// Entity this entry belongs to (may be `"__global__"` for colony-wide facts).
    pub entity_id: EntityId,
    /// Free-form textual content of the memory.
    pub content: String,
    /// Memory type classification.
    pub memory_type: MemoryType,
    /// Who wrote this entry (`"polycephalum"`, `"ganglion"`, `"cortex"`, `"mycelium"`).
    pub source: String,
    /// Timestamp when this entry was created.
    pub ts: Timestamp,
}

// ── Epigenome ─────────────────────────────────────────────────────────────────

/// The Epigenome — institutional memory across Buffer, Working, Core, and Security tiers.
///
/// Lives on [`Runtime`](super::Runtime) as the `epigenome` field and is consulted
/// by the Reflex (Stage 1) and Ganglion (Stage 2) stages for Bayesian allostery:
/// prior knowledge shapes how new signals are interpreted.
pub struct Epigenome {
    buffer_ttl_ms: Timestamp,
    working_window: usize,
    /// Buffer tier: in-process, time-decaying episodic records.
    /// Key: entity_id.
    buffer: HashMap<EntityId, VecDeque<BufferEntry>>,
    /// Working tier: rolling metric summaries per entity.
    /// Key: (entity_id, metric).
    working: HashMap<(EntityId, MetricName), WorkingSummary>,
    /// Core tier: long-term institutional memory.
    /// Key: entity_id → ordered deque (oldest first, evict from front).
    core: HashMap<EntityId, VecDeque<CoreEntry>>,
}

impl Epigenome {
    /// Create a new Epigenome with default parameters.
    pub fn new() -> Self {
        Self::with_config(BUFFER_TTL_MS, WORKING_WINDOW_SIZE)
    }

    /// Create with explicit config — useful in tests.
    pub fn with_config(buffer_ttl_ms: Timestamp, working_window: usize) -> Self {
        Self {
            buffer_ttl_ms,
            working_window,
            buffer: HashMap::new(),
            working: HashMap::new(),
            core: HashMap::new(),
        }
    }

    // ── Buffer tier ───────────────────────────────────────────────────────────

    /// Ingest an admitted signal into the Buffer tier (Episodic memory).
    ///
    /// The `drift_score` is the telos drift score at the time of admission.
    /// Call this from the orchestration loop immediately after `Runtime::emit` returns `true`.
    pub fn record_signal(&mut self, entry: BufferEntry) {
        let bucket = self.buffer.entry(entry.entity_id.clone()).or_default();
        bucket.push_back(entry.clone());
        // Also update the Working tier immediately.
        self.working
            .entry((entry.entity_id, entry.metric))
            .or_insert_with(|| WorkingSummary::new_with_window("", self.working_window))
            .push(entry.value, entry.ts);
    }

    /// Remove expired Buffer entries for all entities.
    ///
    /// Call periodically (e.g., once per orchestration tick) to avoid unbounded growth.
    /// Entries older than `now - buffer_ttl_ms` are dropped.
    pub fn expire_buffer(&mut self, now: Timestamp) {
        let cutoff = now.saturating_sub(self.buffer_ttl_ms);
        for bucket in self.buffer.values_mut() {
            while bucket.front().is_some_and(|e| e.ts < cutoff) {
                bucket.pop_front();
            }
        }
    }

    /// Return recent Buffer entries for an entity, newest first.
    ///
    /// Entries older than `buffer_ttl_ms` are *included* until the next call to
    /// [`expire_buffer`] — callers that need freshness should expire first.
    pub fn buffer_for(&self, entity_id: &str) -> Vec<&BufferEntry> {
        match self.buffer.get(entity_id) {
            None => vec![],
            Some(bucket) => bucket.iter().rev().collect(),
        }
    }

    /// Count Buffer entries for an entity (includes expired-but-not-yet-evicted).
    pub fn buffer_len(&self, entity_id: &str) -> usize {
        self.buffer.get(entity_id).map_or(0, |b| b.len())
    }

    // ── Working tier ──────────────────────────────────────────────────────────

    /// Return the Working summary for a metric on an entity, if it exists.
    pub fn working_summary(&self, entity_id: &str, metric: &str) -> Option<&WorkingSummary> {
        self.working.get(&(entity_id.to_string(), metric.to_string()))
    }

    // ── Core tier ─────────────────────────────────────────────────────────────

    /// Write a Core memory entry (long-term institutional memory).
    ///
    /// Call from Ganglion or Cortex after a successful mutation cycle.
    /// Evicts the oldest entry when [`CORE_MAX_ENTRIES`] is reached.
    pub fn write_core(
        &mut self,
        entity_id: impl Into<EntityId>,
        content: impl Into<String>,
        memory_type: MemoryType,
        source: impl Into<String>,
        ts: Timestamp,
    ) {
        let entity_id = entity_id.into();
        let bucket = self.core.entry(entity_id.clone()).or_default();
        if bucket.len() >= CORE_MAX_ENTRIES {
            bucket.pop_front();
        }
        bucket.push_back(CoreEntry {
            entity_id,
            content: content.into(),
            memory_type,
            source: source.into(),
            ts,
        });
    }

    /// Return all Core entries for an entity, oldest first.
    pub fn core_entries(&self, entity_id: &str) -> Vec<&CoreEntry> {
        match self.core.get(entity_id) {
            None => vec![],
            Some(bucket) => bucket.iter().collect(),
        }
    }

    /// Return the most recent `n` Core entries for an entity, newest first.
    pub fn recent_core_entries(&self, entity_id: &str, n: usize) -> Vec<&CoreEntry> {
        match self.core.get(entity_id) {
            None => vec![],
            Some(bucket) => bucket.iter().rev().take(n).collect(),
        }
    }

    // ── Security tier ─────────────────────────────────────────────────────────

    /// Absorb security events from the store into the Epigenome Security tier.
    ///
    /// Security events are a specialised Episodic memory written by Stage 0 (Membrane)
    /// and stored in SQLite. Calling this method pulls recent events and writes them
    /// into Core as Procedural entries so they survive Buffer expiry and inform
    /// Bayesian allostery in the Reflex stage.
    ///
    /// Typically called once per orchestration tick after signal processing.
    pub fn absorb_security_events(
        &mut self,
        entity_id: &str,
        store: &SignalStore,
        limit: usize,
        now: Timestamp,
    ) {
        let events = match store.security_events_for_entity(entity_id, limit) {
            Ok(e) => e,
            Err(_) => return,
        };
        for ev in events {
            self.write_core(
                entity_id,
                format!("[security:{}] {}", ev.category, ev.description),
                MemoryType::Procedural,
                "membrane",
                now,
            );
        }
    }

    // ── Cross-tier queries ────────────────────────────────────────────────────

    /// Compute the mean drift score in the Buffer for an entity.
    ///
    /// Used by the Reflex stage as the Bayesian prior for current health.
    /// Returns `None` when the Buffer is empty.
    pub fn mean_drift(&self, entity_id: &str) -> Option<f64> {
        let bucket = self.buffer.get(entity_id)?;
        if bucket.is_empty() {
            return None;
        }
        let sum: f64 = bucket.iter().map(|e| e.drift_score).sum();
        Some(sum / bucket.len() as f64)
    }

    /// Count distinct metrics observed for an entity across the Buffer.
    pub fn observed_metrics(&self, entity_id: &str) -> Vec<MetricName> {
        match self.buffer.get(entity_id) {
            None => vec![],
            Some(bucket) => {
                let mut seen: std::collections::HashSet<MetricName> = std::collections::HashSet::new();
                for e in bucket {
                    seen.insert(e.metric.clone());
                }
                seen.into_iter().collect()
            }
        }
    }

    // ── Distillation ──────────────────────────────────────────────────────────

    /// Distil Working tier summaries into Core Semantic memories.
    ///
    /// For each `(entity_id, metric)` pair that has accumulated at least
    /// `min_observations` data points, a Semantic Core entry is written
    /// capturing the statistical profile (mean, std_dev, min, max, n).
    ///
    /// This is the Working → Core transition: short-term statistical patterns
    /// become long-term institutional memory, analogous to short-term chromatin
    /// state consolidating into stable DNA methylation marks.
    ///
    /// The caller (orchestrator) is responsible for timing: call this on a
    /// periodic tick (e.g., every N minutes) rather than every signal.
    ///
    /// Returns the number of Core entries written.
    pub fn distil_working_to_core(
        &mut self,
        min_observations: usize,
        source: &str,
        now: Timestamp,
    ) -> usize {
        let mut written = 0;

        // Collect distillable (entity, metric, stats) triples first to avoid
        // borrow conflicts between self.working (read) and self.core (write).
        let distillable: Vec<(EntityId, MetricName, f64, f64, f64, f64, usize)> = self
            .working
            .iter()
            .filter_map(|((entity_id, metric), summary)| {
                if summary.len() < min_observations {
                    return None;
                }
                let mean = summary.mean()?;
                let std_dev = summary.std_dev().unwrap_or(0.0);
                let min = summary.window.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = summary.window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                Some((entity_id.clone(), metric.clone(), mean, std_dev, min, max, summary.len()))
            })
            .collect();

        for (entity_id, metric, mean, std_dev, min, max, n) in distillable {
            let content = format!(
                "metric '{metric}' over {n} obs: mean={mean:.4} std={std_dev:.4} \
                 min={min:.4} max={max:.4}"
            );
            self.write_core(&entity_id, content, MemoryType::Semantic, source, now);
            written += 1;
        }

        written
    }

    /// Distil high-drift Buffer entries into Episodic Core memories.
    ///
    /// Buffer entries with `drift_score >= drift_threshold` are promoted to
    /// Core as Episodic memories before they expire. This preserves the
    /// record of significant anomalies even after the Buffer TTL elapses —
    /// analogous to a traumatic memory bypassing normal memory consolidation
    /// and writing directly to long-term storage.
    ///
    /// Returns the number of Core entries written.
    pub fn distil_high_drift_to_core(
        &mut self,
        drift_threshold: f64,
        source: &str,
        now: Timestamp,
    ) -> usize {
        let high_drift: Vec<(EntityId, MetricName, f64, f64, Timestamp)> = self
            .buffer
            .iter()
            .flat_map(|(_, bucket)| bucket.iter())
            .filter(|e| e.drift_score >= drift_threshold)
            .map(|e| {
                (
                    e.entity_id.clone(),
                    e.metric.clone(),
                    e.value,
                    e.drift_score,
                    e.ts,
                )
            })
            .collect();

        for (entity_id, metric, value, drift, ts) in &high_drift {
            let content = format!(
                "high-drift event: metric '{metric}' value={value:.4} \
                 drift={drift:.4} at ts={ts}"
            );
            self.write_core(entity_id, content, MemoryType::Episodic, source, now);
        }
        high_drift.len()
    }
}

impl Default for Epigenome {
    fn default() -> Self {
        Self::new()
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

impl WorkingSummary {
    fn new_with_window(metric: impl Into<MetricName>, window: usize) -> Self {
        Self {
            metric: metric.into(),
            capacity: window,
            window: VecDeque::with_capacity(window),
            sum: 0.0,
            sum_sq: 0.0,
            last_ts: 0,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::store::SignalStore;

    fn epigenome() -> Epigenome {
        // Short TTL and small window for tests.
        Epigenome::with_config(100, 5)
    }

    fn entry(entity: &str, metric: &str, value: f64, ts: u64, drift: f64) -> BufferEntry {
        BufferEntry {
            entity_id: entity.to_string(),
            metric: metric.to_string(),
            value,
            ts,
            drift_score: drift,
        }
    }

    fn store() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    // ── Buffer tier ───────────────────────────────────────────────────────────

    #[test]
    fn buffer_records_signal_and_retrieves_newest_first() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.3, 100, 0.1));
        ep.record_signal(entry("e1", "cpu", 0.6, 200, 0.5));
        let buf = ep.buffer_for("e1");
        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0].value, 0.6); // newest first
        assert_eq!(buf[1].value, 0.3);
    }

    #[test]
    fn buffer_expire_drops_old_entries() {
        let mut ep = epigenome(); // ttl = 100ms
        ep.record_signal(entry("e1", "cpu", 1.0, 10, 0.0));
        ep.record_signal(entry("e1", "cpu", 2.0, 50, 0.0));
        ep.record_signal(entry("e1", "cpu", 3.0, 200, 0.0));
        ep.expire_buffer(200); // cutoff = 200 - 100 = 100; drops ts=10 and ts=50
        assert_eq!(ep.buffer_len("e1"), 1);
        assert_eq!(ep.buffer_for("e1")[0].value, 3.0);
    }

    #[test]
    fn buffer_for_unknown_entity_returns_empty() {
        let ep = epigenome();
        assert!(ep.buffer_for("nobody").is_empty());
    }

    // ── Working tier ──────────────────────────────────────────────────────────

    #[test]
    fn working_summary_computes_mean_and_std_dev() {
        let mut s = WorkingSummary::new("latency");
        for v in [10.0_f64, 20.0, 30.0] {
            s.push(v, 0);
        }
        let mean = s.mean().unwrap();
        assert!((mean - 20.0).abs() < 1e-9, "mean should be 20.0, got {mean}");
        let sd = s.std_dev().unwrap();
        // population std dev of {10,20,30}: variance = ((10-20)²+(20-20)²+(30-20)²)/3 = 200/3
        let expected_sd = (200.0_f64 / 3.0).sqrt();
        assert!((sd - expected_sd).abs() < 1e-9);
    }

    #[test]
    fn working_summary_evicts_oldest_when_window_full() {
        let mut s = WorkingSummary::new_with_window("x", 3);
        s.push(1.0, 1);
        s.push(2.0, 2);
        s.push(3.0, 3);
        s.push(4.0, 4); // evicts 1.0
        assert_eq!(s.len(), 3);
        let mean = s.mean().unwrap();
        assert!((mean - 3.0).abs() < 1e-9); // (2+3+4)/3
    }

    #[test]
    fn record_signal_updates_working_summary() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "mem", 0.4, 1, 0.0));
        ep.record_signal(entry("e1", "mem", 0.8, 2, 0.0));
        let s = ep.working_summary("e1", "mem").unwrap();
        assert_eq!(s.len(), 2);
        assert!((s.mean().unwrap() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn working_summary_empty_returns_none_for_stats() {
        let s = WorkingSummary::new("x");
        assert!(s.mean().is_none());
        assert!(s.variance().is_none());
        assert!(s.std_dev().is_none());
    }

    // ── Core tier ─────────────────────────────────────────────────────────────

    #[test]
    fn core_write_and_retrieve_entries() {
        let mut ep = epigenome();
        ep.write_core("e1", "latency stays below 100ms", MemoryType::Semantic, "ganglion", 1_000);
        ep.write_core("e1", "reduce batch size on OOM", MemoryType::Procedural, "cortex", 2_000);
        let entries = ep.core_entries("e1");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].memory_type, MemoryType::Semantic);
        assert_eq!(entries[1].memory_type, MemoryType::Procedural);
    }

    #[test]
    fn core_evicts_oldest_when_at_capacity() {
        let mut ep = Epigenome::with_config(BUFFER_TTL_MS, WORKING_WINDOW_SIZE);
        for i in 0..CORE_MAX_ENTRIES + 1 {
            ep.write_core("e1", format!("entry {i}"), MemoryType::Episodic, "test", i as u64);
        }
        let entries = ep.core_entries("e1");
        assert_eq!(entries.len(), CORE_MAX_ENTRIES);
        assert_eq!(entries[0].content, "entry 1"); // entry 0 evicted
    }

    #[test]
    fn recent_core_entries_returns_newest_first() {
        let mut ep = epigenome();
        ep.write_core("e1", "old", MemoryType::Episodic, "test", 1);
        ep.write_core("e1", "new", MemoryType::Episodic, "test", 2);
        let recent = ep.recent_core_entries("e1", 1);
        assert_eq!(recent[0].content, "new");
    }

    #[test]
    fn core_entries_unknown_entity_returns_empty() {
        let ep = epigenome();
        assert!(ep.core_entries("nobody").is_empty());
    }

    // ── Security tier ─────────────────────────────────────────────────────────

    #[test]
    fn absorb_security_events_writes_procedural_core_entries() {
        let mut ep = epigenome();
        let s = store();
        s.write_security_event("e1", "rate_limit_exceeded", "too many signals", 1_000).unwrap();
        s.write_security_event("e1", "unregistered_source", "foreign entity", 2_000).unwrap();
        ep.absorb_security_events("e1", &s, 10, 3_000);
        let entries = ep.core_entries("e1");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.memory_type == MemoryType::Procedural));
        assert!(entries.iter().all(|e| e.source == "membrane"));
    }

    #[test]
    fn absorb_security_events_noop_when_no_events() {
        let mut ep = epigenome();
        let s = store();
        ep.absorb_security_events("e1", &s, 10, 1_000);
        assert!(ep.core_entries("e1").is_empty());
    }

    // ── Cross-tier queries ────────────────────────────────────────────────────

    #[test]
    fn mean_drift_computes_across_buffer() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.5, 100, 0.2));
        ep.record_signal(entry("e1", "mem", 0.8, 200, 0.6));
        let mean = ep.mean_drift("e1").unwrap();
        assert!((mean - 0.4).abs() < 1e-9); // (0.2 + 0.6) / 2
    }

    #[test]
    fn mean_drift_returns_none_for_empty_buffer() {
        let ep = epigenome();
        assert!(ep.mean_drift("e1").is_none());
    }

    #[test]
    fn observed_metrics_lists_distinct_metrics() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.1, 1, 0.0));
        ep.record_signal(entry("e1", "cpu", 0.2, 2, 0.0));
        ep.record_signal(entry("e1", "mem", 0.3, 3, 0.0));
        let mut metrics = ep.observed_metrics("e1");
        metrics.sort();
        assert_eq!(metrics, vec!["cpu", "mem"]);
    }

    // ── Distillation ──────────────────────────────────────────────────────────

    #[test]
    fn distil_working_to_core_writes_semantic_entries() {
        let mut ep = epigenome(); // window = 5
        for i in 0..5 {
            ep.record_signal(entry("e1", "cpu", i as f64 * 0.1, i, 0.0));
        }
        let written = ep.distil_working_to_core(3, "ganglion", 10_000);
        assert_eq!(written, 1);
        let cores = ep.core_entries("e1");
        assert_eq!(cores.len(), 1);
        assert_eq!(cores[0].memory_type, MemoryType::Semantic);
        assert_eq!(cores[0].source, "ganglion");
        assert!(cores[0].content.contains("cpu"));
        assert!(cores[0].content.contains("mean="));
    }

    #[test]
    fn distil_working_to_core_skips_underfull_window() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.5, 1, 0.0));
        // Only 1 observation; min_observations = 5
        let written = ep.distil_working_to_core(5, "ganglion", 1_000);
        assert_eq!(written, 0);
        assert!(ep.core_entries("e1").is_empty());
    }

    #[test]
    fn distil_high_drift_to_core_writes_episodic_entries() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.9, 100, 0.85)); // high drift
        ep.record_signal(entry("e1", "cpu", 0.1, 200, 0.05)); // normal
        ep.record_signal(entry("e1", "mem", 0.7, 300, 0.92)); // high drift
        let written = ep.distil_high_drift_to_core(0.8, "orchestrator", 5_000);
        assert_eq!(written, 2);
        let cores = ep.core_entries("e1");
        assert_eq!(cores.len(), 2);
        assert!(cores.iter().all(|e| e.memory_type == MemoryType::Episodic));
        assert!(cores.iter().all(|e| e.content.contains("high-drift")));
    }

    #[test]
    fn distil_high_drift_noop_when_all_drift_below_threshold() {
        let mut ep = epigenome();
        ep.record_signal(entry("e1", "cpu", 0.5, 100, 0.1));
        ep.record_signal(entry("e1", "cpu", 0.6, 200, 0.2));
        let written = ep.distil_high_drift_to_core(0.5, "orchestrator", 1_000);
        assert_eq!(written, 0);
    }

    #[test]
    fn distil_multiple_metrics_writes_one_entry_per_metric() {
        let mut ep = epigenome();
        for i in 0..5_u64 {
            ep.record_signal(entry("e1", "cpu", 0.3, i, 0.0));
            ep.record_signal(entry("e1", "mem", 0.7, i + 100, 0.0));
        }
        let written = ep.distil_working_to_core(3, "cortex", 20_000);
        assert_eq!(written, 2);
    }
}
