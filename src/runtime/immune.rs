//! Membrane — Stage 0 of the CEMS pipeline.
//!
//! The first contact filter. Every incoming signal and genome mutation passes
//! through the Membrane before entering the pipeline. The Membrane:
//!
//! 1. Rejects signals from unregistered sources.
//! 2. Enforces per-source rate limits (token bucket) — DoS protection.
//! 3. Verifies SHA-256 genome hash lineage for mutation proposals.
//! 4. Flags implausible values (faulty hardware) as security events without blocking.
//! 5. Manages the quarantine state machine: exponential-backoff isolation with
//!    automatic probe-and-clear after the window expires.
//!
//! # Biological analog
//!
//! Cell membrane + innate immune system. Pattern recognition receptors reject
//! non-self molecules at the boundary. White blood cell quarantine: the affected
//! compartment is isolated; the rest of the organism continues operating.
//!
//! # Security events
//!
//! All threat detections are written to [`SignalStore::write_security_event`].
//! These records will be absorbed into the Epigenome Security memory tier (R9).

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::runtime::signal::{now_ms, EntityId, Signal, Timestamp};
use crate::runtime::store::SignalStore;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Quarantine backoff windows in milliseconds: 1m → 5m → 30m → 1h → 24h.
const QUARANTINE_WINDOWS_MS: [u64; 5] = [60_000, 300_000, 1_800_000, 3_600_000, 86_400_000];

// ── Public types ──────────────────────────────────────────────────────────────

/// The outcome of a Membrane evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum MembraneVerdict {
    /// Signal or genome admitted — pipeline may proceed.
    Admit,
    /// Hard rejection — signal does not belong in this system.
    Reject(RejectReason),
    /// Threat confirmed — entity enters or extends quarantine; signal dropped.
    Quarantine(QuarantineEntry),
}

/// Why a signal or mutation was hard-rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    /// The emitting entity is not registered as a trusted source.
    UnregisteredSource { source_id: String },
    /// The genome hash does not match the registered lineage.
    GenomeHashMismatch { expected: String, actual: String },
}

/// Details of an active quarantine event.
#[derive(Debug, Clone, PartialEq)]
pub struct QuarantineEntry {
    pub entity_id: String,
    pub category: SecurityCategory,
    pub description: String,
    /// Index into `QUARANTINE_WINDOWS_MS` — higher = longer isolation.
    pub window_index: u8,
    /// Unix timestamp (ms) when this quarantine window expires.
    pub until_ms: Timestamp,
}

/// The type of security threat detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityCategory {
    /// Signal emission rate exceeded the configured token bucket.
    RateLimitExceeded,
    /// Signal value is outside the configured plausible range (hardware fault).
    ImplausibleValue,
    /// Genome hash does not match the registered lineage (tampering).
    GenomeHashMismatch,
    /// Signal emitted by an entity that is not registered as a trusted source.
    UnregisteredSource,
}

impl SecurityCategory {
    /// String key used when writing to the security_events store table.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::ImplausibleValue => "implausible_value",
            Self::GenomeHashMismatch => "genome_hash_mismatch",
            Self::UnregisteredSource => "unregistered_source",
        }
    }
}

/// Configuration for the Membrane.
#[derive(Debug, Clone)]
pub struct MembraneConfig {
    /// Token bucket capacity: maximum burst of signals per source before rate-limiting.
    pub rate_capacity: u32,
    /// Token refill rate in tokens per millisecond.
    pub rate_refill_per_ms: f64,
    /// If set, signals with values outside `(min, max)` are flagged (not rejected).
    pub plausible_range: Option<(f64, f64)>,
}

impl Default for MembraneConfig {
    fn default() -> Self {
        Self {
            rate_capacity: 200,
            rate_refill_per_ms: 0.01, // 10 tokens/second
            plausible_range: None,
        }
    }
}

// ── Internal types ────────────────────────────────────────────────────────────

/// Token bucket for per-source rate limiting.
struct TokenBucket {
    capacity: u32,
    tokens: f64,
    last_refill_ms: Timestamp,
    refill_per_ms: f64,
}

impl TokenBucket {
    fn new(capacity: u32, refill_per_ms: f64, now: Timestamp) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            last_refill_ms: now,
            refill_per_ms,
        }
    }

    /// Consume one token. Returns `true` if the request is allowed.
    fn try_consume(&mut self, now: Timestamp) -> bool {
        let elapsed = now.saturating_sub(self.last_refill_ms) as f64;
        self.tokens = (self.tokens + elapsed * self.refill_per_ms).min(self.capacity as f64);
        self.last_refill_ms = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Active quarantine state persisted in memory for an entity.
struct QuarantineState {
    window_index: u8,
    until_ms: Timestamp,
    category: SecurityCategory,
}

// ── Membrane ──────────────────────────────────────────────────────────────────

/// Stage 0 — Membrane / Immune layer.
///
/// Maintains registration, genome hashes, rate limiters, and quarantine state.
/// All checks are O(1) or O(log n) — the Membrane never blocks the hot path.
pub struct Membrane {
    config: MembraneConfig,
    registered: HashSet<EntityId>,
    genome_registry: HashMap<EntityId, String>,
    rate_limiters: HashMap<EntityId, TokenBucket>,
    quarantine: HashMap<EntityId, QuarantineState>,
}

impl Membrane {
    /// Create a new Membrane with the given configuration.
    pub fn new(config: MembraneConfig) -> Self {
        Self {
            config,
            registered: HashSet::new(),
            genome_registry: HashMap::new(),
            rate_limiters: HashMap::new(),
            quarantine: HashMap::new(),
        }
    }

    /// Register an entity as a trusted signal source.
    ///
    /// Must be called (alongside [`SignalStore::register_entity`]) when an entity
    /// is spawned. Signals from unregistered sources are hard-rejected.
    pub fn register_entity(&mut self, entity_id: impl Into<EntityId>) {
        self.registered.insert(entity_id.into());
    }

    /// Record the SHA-256 hash of an entity's genome (telos JSON or `.loom` source).
    ///
    /// Used to detect tampering in mutation proposals (genome hash lineage check).
    pub fn register_genome(&mut self, entity_id: impl Into<EntityId>, genome_source: &str) {
        self.genome_registry
            .insert(entity_id.into(), sha256_hex(genome_source));
    }

    /// Return `Some(true)` if the genome matches the registered hash,
    /// `Some(false)` on mismatch, or `None` if no genome is registered.
    pub fn verify_genome(&self, entity_id: &str, genome_source: &str) -> Option<bool> {
        self.genome_registry
            .get(entity_id)
            .map(|expected| sha256_hex(genome_source) == *expected)
    }

    /// Evaluate an incoming signal against the Membrane.
    ///
    /// Returns the verdict and writes security events to `store` on any threat.
    /// The caller must only write the signal to the store when `Admit` is returned.
    pub fn evaluate(&mut self, signal: &Signal, store: &SignalStore) -> MembraneVerdict {
        let now = now_ms();
        let src = &signal.entity_id;

        if let Some(verdict) = self.check_registration(src, &signal.metric, store, now) {
            return verdict;
        }
        if let Some(verdict) = self.check_quarantine(src, now) {
            return verdict;
        }
        if let Some(verdict) = self.check_rate_limit(src, store, now) {
            return verdict;
        }
        self.check_plausibility(src, &signal.metric, signal.value, store, now);
        MembraneVerdict::Admit
    }

    /// Evaluate a proposed genome against the registered hash for an entity.
    ///
    /// Call this before applying any mutation. Returns `Admit` when there is no
    /// registered hash (no check possible) or the hash matches. Rejects on mismatch.
    pub fn evaluate_genome(
        &mut self,
        entity_id: &str,
        genome_source: &str,
        store: &SignalStore,
    ) -> MembraneVerdict {
        match self.verify_genome(entity_id, genome_source) {
            None | Some(true) => MembraneVerdict::Admit,
            Some(false) => {
                let expected = self.genome_registry[entity_id].clone();
                let actual = sha256_hex(genome_source);
                let now = now_ms();
                let _ = store.write_security_event(
                    entity_id,
                    SecurityCategory::GenomeHashMismatch.as_str(),
                    &format!(
                        "genome hash mismatch: expected {}…, got {}…",
                        &expected[..8],
                        &actual[..8]
                    ),
                    now,
                );
                MembraneVerdict::Reject(RejectReason::GenomeHashMismatch { expected, actual })
            }
        }
    }

    /// Manually quarantine an entity (e.g., triggered by an upstream threat signal).
    pub fn quarantine_entity(
        &mut self,
        entity_id: &str,
        category: SecurityCategory,
        now: Timestamp,
    ) -> QuarantineEntry {
        self.enter_quarantine(entity_id, category, now)
    }

    /// Returns `true` if the entity is within an active quarantine window.
    pub fn is_quarantined(&self, entity_id: &str) -> bool {
        self.quarantine
            .get(entity_id)
            .is_some_and(|s| now_ms() < s.until_ms)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn check_registration(
        &self,
        src: &str,
        metric: &str,
        store: &SignalStore,
        now: Timestamp,
    ) -> Option<MembraneVerdict> {
        if self.registered.contains(src) {
            return None;
        }
        let _ = store.write_security_event(
            src,
            SecurityCategory::UnregisteredSource.as_str(),
            &format!("unregistered source emitted metric '{metric}'"),
            now,
        );
        Some(MembraneVerdict::Reject(RejectReason::UnregisteredSource {
            source_id: src.to_string(),
        }))
    }

    fn check_quarantine(&mut self, src: &str, now: Timestamp) -> Option<MembraneVerdict> {
        let state = self.quarantine.get(src)?;
        if now < state.until_ms {
            return Some(MembraneVerdict::Quarantine(QuarantineEntry {
                entity_id: src.to_string(),
                category: state.category.clone(),
                description: "entity in active quarantine".into(),
                window_index: state.window_index,
                until_ms: state.until_ms,
            }));
        }
        // Window expired — preserve the entry so violation history is maintained.
        // The next call to enter_quarantine will advance window_index correctly.
        // is_quarantined() returns false because until_ms is in the past.
        None
    }

    fn check_rate_limit(
        &mut self,
        src: &str,
        store: &SignalStore,
        now: Timestamp,
    ) -> Option<MembraneVerdict> {
        let allowed = self
            .rate_limiters
            .entry(src.to_string())
            .or_insert_with(|| {
                TokenBucket::new(
                    self.config.rate_capacity,
                    self.config.rate_refill_per_ms,
                    now,
                )
            })
            .try_consume(now);

        if allowed {
            return None;
        }
        let entry = self.enter_quarantine(src, SecurityCategory::RateLimitExceeded, now);
        let window_ms = entry.until_ms - now;
        let _ = store.write_security_event(
            src,
            SecurityCategory::RateLimitExceeded.as_str(),
            &format!("rate limit exceeded; quarantine window {window_ms}ms"),
            now,
        );
        Some(MembraneVerdict::Quarantine(entry))
    }

    /// Flags implausible values as security events but does NOT block the signal.
    ///
    /// Faulty hardware should not halt the entity — it should be noted and
    /// down-weighted by Bayesian allostery in the Reflex stage.
    fn check_plausibility(
        &self,
        src: &str,
        metric: &str,
        value: f64,
        store: &SignalStore,
        now: Timestamp,
    ) {
        if let Some((min, max)) = self.config.plausible_range {
            if value < min || value > max {
                let _ = store.write_security_event(
                    src,
                    SecurityCategory::ImplausibleValue.as_str(),
                    &format!(
                        "metric '{metric}' value {value:.4} outside plausible range \
                         [{min:.4}, {max:.4}]"
                    ),
                    now,
                );
            }
        }
    }

    /// Advance or create the quarantine state for `entity_id`.
    ///
    /// First call → window index 0 (1 minute).
    /// Each subsequent call → next index, up to the maximum window.
    fn enter_quarantine(
        &mut self,
        entity_id: &str,
        category: SecurityCategory,
        now: Timestamp,
    ) -> QuarantineEntry {
        let next_index = match self.quarantine.get(entity_id) {
            None => 0,
            Some(s) => (s.window_index + 1).min((QUARANTINE_WINDOWS_MS.len() - 1) as u8),
        };
        let window_ms = QUARANTINE_WINDOWS_MS[next_index as usize];
        let until_ms = now + window_ms;

        self.quarantine.insert(
            entity_id.to_string(),
            QuarantineState {
                window_index: next_index,
                until_ms,
                category: category.clone(),
            },
        );

        QuarantineEntry {
            entity_id: entity_id.to_string(),
            category,
            description: format!("quarantine window {next_index} active for {window_ms}ms"),
            window_index: next_index,
            until_ms,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compute the lowercase hex SHA-256 digest of a UTF-8 string.
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;
    use crate::runtime::store::SignalStore;

    fn store() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    fn membrane() -> Membrane {
        Membrane::new(MembraneConfig::default())
    }

    fn tight_membrane() -> Membrane {
        Membrane::new(MembraneConfig {
            rate_capacity: 2,
            rate_refill_per_ms: 0.0, // never refills — makes rate-limit tests deterministic
            plausible_range: None,
        })
    }

    // ── Registration ─────────────────────────────────────────────────────────

    #[test]
    fn admits_registered_entity_signal() {
        let s = store();
        let mut m = membrane();
        m.register_entity("e1");
        assert_eq!(
            m.evaluate(&Signal::new("e1", "temperature", 1.5), &s),
            MembraneVerdict::Admit
        );
    }

    #[test]
    fn rejects_unregistered_source() {
        let s = store();
        let mut m = membrane();
        assert!(matches!(
            m.evaluate(&Signal::new("rogue", "temperature", 1.5), &s),
            MembraneVerdict::Reject(RejectReason::UnregisteredSource { .. })
        ));
    }

    #[test]
    fn unregistered_source_writes_security_event() {
        let s = store();
        let mut m = membrane();
        let _ = m.evaluate(&Signal::new("rogue", "cpu", 0.5), &s);
        let events = s.security_events_for_entity("rogue", 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "unregistered_source");
    }

    // ── Rate limiting ─────────────────────────────────────────────────────────

    #[test]
    fn quarantines_on_rate_limit_exceeded() {
        let s = store();
        let mut m = tight_membrane();
        m.register_entity("e1");
        let sig = Signal::new("e1", "cpu", 0.5);
        assert_eq!(m.evaluate(&sig, &s), MembraneVerdict::Admit);
        assert_eq!(m.evaluate(&sig, &s), MembraneVerdict::Admit);
        // Bucket now empty — third call quarantines.
        assert!(matches!(
            m.evaluate(&sig, &s),
            MembraneVerdict::Quarantine(_)
        ));
    }

    #[test]
    fn quarantine_window_index_starts_at_zero() {
        let s = store();
        let mut m = tight_membrane();
        m.register_entity("e1");
        let sig = Signal::new("e1", "cpu", 0.5);
        m.evaluate(&sig, &s); // consumes token 1
        m.evaluate(&sig, &s); // consumes token 2
        let v = m.evaluate(&sig, &s); // rate-limited → quarantine
        let idx = match v {
            MembraneVerdict::Quarantine(e) => e.window_index,
            _ => panic!("expected quarantine"),
        };
        assert_eq!(idx, 0); // first quarantine → shortest window
    }

    #[test]
    fn quarantine_advances_window_on_repeated_violations() {
        let s = store();
        let mut m = Membrane::new(MembraneConfig {
            rate_capacity: 1,
            rate_refill_per_ms: 0.0,
            plausible_range: None,
        });
        m.register_entity("e1");
        let sig = Signal::new("e1", "cpu", 0.5);
        m.evaluate(&sig, &s); // consumes only token
        m.evaluate(&sig, &s); // quarantine window 0
                              // Manually expire the quarantine so the next violation advances the window.
        m.quarantine.get_mut("e1").unwrap().until_ms = 0;
        m.evaluate(&sig, &s); // probe passes (expired window cleared)
                              // Next rate-limit violation should advance to window 1.
                              // Re-deplete the bucket first (it was refilled by time 0 trick, set tokens = 0).
        m.rate_limiters.get_mut("e1").unwrap().tokens = 0.0;
        let v = m.evaluate(&sig, &s);
        let idx = match v {
            MembraneVerdict::Quarantine(e) => e.window_index,
            _ => panic!("expected quarantine"),
        };
        assert_eq!(idx, 1);
    }

    #[test]
    fn quarantine_clears_after_window_expires() {
        let s = store();
        let mut m = membrane();
        m.register_entity("e1");
        // Inject an already-expired quarantine.
        m.quarantine.insert(
            "e1".to_string(),
            QuarantineState {
                window_index: 1,
                until_ms: 0, // expired
                category: SecurityCategory::RateLimitExceeded,
            },
        );
        assert_eq!(
            m.evaluate(&Signal::new("e1", "cpu", 0.5), &s),
            MembraneVerdict::Admit
        );
        assert!(!m.is_quarantined("e1"));
    }

    // ── Plausibility ──────────────────────────────────────────────────────────

    #[test]
    fn implausible_value_admits_but_writes_security_event() {
        let s = store();
        let mut m = Membrane::new(MembraneConfig {
            rate_capacity: 200,
            rate_refill_per_ms: 0.01,
            plausible_range: Some((0.0, 100.0)),
        });
        m.register_entity("e1");
        // Value 9999 is outside [0, 100] — admitted (hardware fault, not attack).
        assert_eq!(
            m.evaluate(&Signal::new("e1", "temperature", 9999.0), &s),
            MembraneVerdict::Admit
        );
        let events = s.security_events_for_entity("e1", 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "implausible_value");
    }

    // ── Genome hash ───────────────────────────────────────────────────────────

    #[test]
    fn genome_hash_match_admits() {
        let s = store();
        let mut m = membrane();
        m.register_genome("e1", r#"{"target":1.5}"#);
        assert_eq!(
            m.evaluate_genome("e1", r#"{"target":1.5}"#, &s),
            MembraneVerdict::Admit
        );
    }

    #[test]
    fn genome_hash_mismatch_rejects() {
        let s = store();
        let mut m = membrane();
        m.register_genome("e1", r#"{"target":1.5}"#);
        assert!(matches!(
            m.evaluate_genome("e1", r#"{"target":9.9}"#, &s),
            MembraneVerdict::Reject(RejectReason::GenomeHashMismatch { .. })
        ));
    }

    #[test]
    fn genome_hash_mismatch_writes_security_event() {
        let s = store();
        let mut m = membrane();
        m.register_genome("e1", r#"{"target":1.5}"#);
        let _ = m.evaluate_genome("e1", "tampered", &s);
        let events = s.security_events_for_entity("e1", 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "genome_hash_mismatch");
    }

    #[test]
    fn no_genome_registered_admits_any_source() {
        let s = store();
        let mut m = membrane();
        assert_eq!(
            m.evaluate_genome("e1", "anything", &s),
            MembraneVerdict::Admit
        );
    }

    // ── SHA-256 ───────────────────────────────────────────────────────────────

    #[test]
    fn sha256_hex_is_deterministic_and_distinguishes_inputs() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
        assert_eq!(sha256_hex("hello").len(), 64); // 256 bits = 64 hex chars
    }
}
