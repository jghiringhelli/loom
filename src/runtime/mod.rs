//! CEMS runtime — the living execution layer for Loom programs.
//!
//! Converts a compiled Loom program from a static type-checked artifact into a
//! running system that emits telemetry, measures telos drift, proposes mutations,
//! and evolves toward its declared intent.
//!
//! # CEMS architecture
//!
//! Two axes:
//! - **Stages 0–8** (linear pipeline): Membrane → Reflex → Ganglion → Cortex →
//!   Gate → Simulation → Soft Release → Acclimatization → Propagation
//! - **Cross-cutting** (always-on): C (Circadian) · E (Epigenome) · M (Mycelium)
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md).

pub mod brain;
pub mod circadian;
pub mod colony;
pub mod deploy;
pub mod drift;
pub mod epigenetic;
pub mod ganglion;
pub mod gate;
pub mod immune;
pub mod mutation;
pub mod orchestrator;
pub mod polycephalum;
pub mod signal;
pub mod store;
pub mod supervisor;

pub use brain::{CostGuard, MammalBrain};
pub use circadian::{Circadian, CircadianAction, CircadianVerdict, KalmanFilter, Schedule, WallTime};
pub use colony::{ColonyPeer, GossipMessage, Mycelium, PeerStatus, PheromoneTrail};
pub use deploy::{CanaryDeployer, DeployOutcome, DeployStatus};
pub use drift::{DriftEngine, DriftEvent, DriftSeverity};
pub use epigenetic::{BufferEntry, CoreEntry, Epigenome, MemoryType, WorkingSummary};
pub use ganglion::{Ganglion, GanglionConfig};
pub use gate::{GateResult, GateVerdict, MutationGate};
pub use immune::{Membrane, MembraneConfig, MembraneVerdict, RejectReason, SecurityCategory};
pub use mutation::MutationProposal;
pub use polycephalum::{DeltaSpec, Polycephalum, Rule, RuleAction, RuleCondition, RuleRegistry};
pub use signal::{now_ms, EntityId, MetricName, Signal, Timestamp};
pub use store::{EntityRecord, SecurityEvent, SignalStore, TelosBound};
pub use supervisor::{EntityInstance, EntityState, EntitySupervisor};

// ── Runtime context ───────────────────────────────────────────────────────────

/// A fully initialised BIOISO runtime context.
///
/// Holds the signal store and entity supervisor. Created by
/// `compile_runtime()`-generated code or directly in tests.
///
/// # Example
///
/// ```rust,ignore
/// let mut rt = Runtime::new(":memory:").unwrap();
/// rt.spawn_entity("climate", "ClimateModel", r#"{"target":1.5}"#, Some(50), None).unwrap();
/// rt.emit_metric("climate", "co2_ppm", 421.3).unwrap();
/// ```
pub struct Runtime {
    /// The append-only SQLite signal store.
    pub store: SignalStore,
    /// The in-memory entity lifecycle supervisor.
    pub supervisor: EntitySupervisor,
    /// The telos drift engine — evaluates signals against declared bounds.
    pub drift_engine: DriftEngine,
    /// Stage 0 — Membrane / Immune layer.
    pub membrane: Membrane,
    /// Cross-cutting E axis — Epigenome (Buffer / Working / Core / Security tiers).
    pub epigenome: Epigenome,
    /// Cross-cutting C axis — Circadian (temporal gating + Kalman SNR pre-filter).
    pub circadian: Circadian,
    /// Cross-cutting M axis — Mycelium (colony gossip, stigmergy, offline resilience).
    pub mycelium: Mycelium,
    /// Tier 1 Polycephalum rule engine — proposes mutations from drift events.
    pub polycephalum: Polycephalum,
    /// Type-safe mutation gate — validates proposals through the full compiler.
    pub gate: MutationGate,
    /// Tier 2 Ganglion engine — local micro-LLM synthesis via Ollama.
    pub ganglion: Ganglion,
    /// Tier 3 Mammal Brain — Claude API, full genome synthesis. Optional; skipped when
    /// `CLAUDE_API_KEY` is not configured or manually set to `None`.
    pub brain: Option<MammalBrain>,
}

impl Runtime {
    /// Create a new runtime, opening the signal store at `db_path`.
    ///
    /// Use `":memory:"` for tests; a file path for production deployments.
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            store: SignalStore::new(db_path)?,
            supervisor: EntitySupervisor::new(),
            drift_engine: DriftEngine::new(),
            membrane: Membrane::new(MembraneConfig::default()),
            epigenome: Epigenome::new(),
            circadian: Circadian::new(),
            mycelium: Mycelium::new(),
            polycephalum: Polycephalum::new(),
            gate: MutationGate::new(),
            ganglion: Ganglion::new(GanglionConfig::default()),
            brain: MammalBrain::from_env(),
        })
    }

    /// Spawn a new entity: registers in supervisor, signal store, and Membrane.
    ///
    /// `telos_json` is the serialised telos declaration (used by the drift engine).
    /// `telomere_limit` is the maximum number of evolutions before senescence.
    /// `genome_source` is hashed and registered for lineage verification (optional).
    pub fn spawn_entity(
        &mut self,
        id: impl Into<EntityId>,
        name: impl Into<String>,
        telos_json: &str,
        telomere_limit: Option<u32>,
        on_exhaustion: Option<String>,
    ) -> Result<(), rusqlite::Error> {
        let entity_id: EntityId = id.into();
        let entity_name: String = name.into();
        let born_at = now_ms();
        self.store.register_entity(&entity_id, &entity_name, telos_json, born_at)?;
        self.supervisor.spawn(entity_id.clone(), entity_name, telomere_limit, on_exhaustion);
        self.membrane.register_entity(&entity_id);
        self.membrane.register_genome(&entity_id, telos_json);
        Ok(())
    }

    /// Emit a telemetry signal from a running entity.
    ///
    /// Signals pass through the Membrane (Stage 0) first. Admitted signals are stored
    /// in the SQLite signal store and ingested into the Epigenome Buffer tier.
    /// Returns `Ok(true)` when admitted, `Ok(false)` when rejected or quarantined.
    pub fn emit(&mut self, signal: Signal) -> Result<bool, rusqlite::Error> {
        match self.membrane.evaluate(&signal, &self.store) {
            MembraneVerdict::Admit => {
                self.store.write_signal(&signal)?;
                // Feed Epigenome Buffer (drift score unknown at this stage — set 0.0;
                // the orchestrator overwrites after evaluate_drift).
                self.epigenome.record_signal(BufferEntry {
                    entity_id: signal.entity_id.clone(),
                    metric: signal.metric.clone(),
                    value: signal.value,
                    ts: signal.timestamp,
                    drift_score: 0.0,
                });
                Ok(true)
            }
            MembraneVerdict::Reject(_) | MembraneVerdict::Quarantine(_) => Ok(false),
        }
    }

    /// Convenience: emit a named metric value for an entity with the current timestamp.
    ///
    /// Returns `Ok(true)` if admitted, `Ok(false)` if blocked by the Membrane.
    pub fn emit_metric(
        &mut self,
        entity_id: impl Into<EntityId>,
        metric: impl Into<MetricName>,
        value: f64,
    ) -> Result<bool, rusqlite::Error> {
        self.emit(Signal::new(entity_id, metric, value))
    }

    /// Return the n most recent signals for an entity, newest first.
    pub fn recent_signals(
        &self,
        entity_id: &str,
        n: usize,
    ) -> Result<Vec<Signal>, rusqlite::Error> {
        self.store.signals_for_entity(entity_id, n)
    }

    /// Evaluate a signal for telos drift and return a [`DriftEvent`] if threshold exceeded.
    ///
    /// This is the primary integration point for R2 from the orchestration loop.
    pub fn evaluate_drift(
        &self,
        signal: &Signal,
    ) -> Result<Option<DriftEvent>, rusqlite::Error> {
        self.drift_engine.evaluate(signal, &self.store)
    }

    /// Evaluate all recent signals for every entity and return emitted drift events.
    pub fn evaluate_all_drift(
        &self,
        lookback: usize,
    ) -> Result<Vec<DriftEvent>, rusqlite::Error> {
        let entity_ids: Vec<EntityId> = self
            .entities()?
            .into_iter()
            .map(|e| e.id)
            .collect();
        self.drift_engine.evaluate_all(&entity_ids, &self.store, lookback)
    }

    /// Return all registered entity records.
    pub fn entities(&self) -> Result<Vec<EntityRecord>, rusqlite::Error> {
        self.store.all_entities()
    }

    /// Register telos bounds for a metric on a given entity.
    pub fn set_telos_bounds(
        &self,
        entity_id: &str,
        metric: &str,
        min: Option<f64>,
        max: Option<f64>,
        target: Option<f64>,
    ) -> Result<(), rusqlite::Error> {
        self.store.set_telos_bounds(entity_id, metric, min, max, target)
    }

    /// Run Tier 1 (Polycephalum) against a drift event and return proposals.
    ///
    /// The optional `checkpoint_id` enables rollback proposals. Returns an empty
    /// vec when no rule matches — caller should escalate to Tier 2.
    pub fn propose_mutations(
        &self,
        event: &DriftEvent,
        checkpoint_id: Option<i64>,
    ) -> Vec<MutationProposal> {
        let severity = self.drift_engine.severity(event.score);
        self.polycephalum.evaluate(event, severity, checkpoint_id)
    }

    /// Number of currently Active entities.
    pub fn active_count(&self) -> usize {
        self.supervisor.active_count()
    }

    /// Validate a proposal through the type-safe mutation gate and record the verdict.
    ///
    /// Returns the [`GateResult`] containing the verdict and the original proposal.
    pub fn apply_proposal(&self, proposal: &MutationProposal) -> GateResult {
        self.gate.evaluate(proposal, &self.store)
    }
    /// Run Tier 3 (Mammal Brain) against a drift event. Returns proposals, or an empty
    /// vec if the brain is not configured or the cost guard is exhausted.
    ///
    /// `genome` is the full `.loom` source of the entity being evolved — pass `None`
    /// if the genome has not been registered.
    pub fn evaluate_tier3(
        &mut self,
        event: &DriftEvent,
        genome: Option<&str>,
    ) -> Vec<MutationProposal> {
        match &mut self.brain {
            Some(brain) => brain.evaluate(event, &self.store, genome),
            None => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_new_creates_store() {
        let rt = Runtime::new(":memory:").unwrap();
        let entities = rt.entities().unwrap();
        assert!(entities.is_empty());
    }

    #[test]
    fn spawn_entity_registers_in_store_and_supervisor() {
        let mut rt = Runtime::new(":memory:").unwrap();
        rt.spawn_entity("e1", "ClimateModel", r#"{"target":1.5}"#, Some(50), None)
            .unwrap();
        assert_eq!(rt.active_count(), 1);
        let entities = rt.entities().unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "ClimateModel");
    }

    #[test]
    fn emit_metric_stored_and_readable() {
        let mut rt = Runtime::new(":memory:").unwrap();
        rt.spawn_entity("e1", "Foo", "{}", None, None).unwrap();
        rt.emit_metric("e1", "co2_ppm", 415.0).unwrap();
        rt.emit_metric("e1", "co2_ppm", 416.0).unwrap();
        let sigs = rt.recent_signals("e1", 10).unwrap();
        assert_eq!(sigs.len(), 2);
        // newest first
        assert!((sigs[0].value - 416.0).abs() < 1e-9);
    }

    #[test]
    fn set_telos_bounds_persisted() {
        let mut rt = Runtime::new(":memory:").unwrap();
        rt.spawn_entity("e1", "Foo", "{}", None, None).unwrap();
        rt.set_telos_bounds("e1", "temperature", Some(0.0), Some(2.0), Some(1.5))
            .unwrap();
        let bounds = rt.store.telos_bounds_for_entity("e1").unwrap();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].target, Some(1.5));
    }

    #[test]
    fn telomere_exhaustion_transitions_entity() {
        let mut rt = Runtime::new(":memory:").unwrap();
        rt.spawn_entity("e1", "Foo", "{}", Some(2), Some("halt".into()))
            .unwrap();
        rt.supervisor
            .record_division("e1", &rt.store)
            .unwrap();
        let result = rt.supervisor.record_division("e1", &rt.store);
        assert!(result.is_err());
        assert_eq!(
            rt.supervisor.get("e1").unwrap().state,
            EntityState::Senescent
        );
    }
}
