//! BIOISO runtime — the living execution layer for Loom programs.
//!
//! Converts a compiled Loom program from a static type-checked artifact into a
//! running system that emits telemetry, measures telos drift, proposes mutations,
//! and evolves toward its declared intent.
//!
//! # Architecture
//!
//! ```text
//! Signal emission → Signal store → Telos drift engine
//!                                        ↓
//!                         Mutation proposals (three tiers)
//!                                        ↓
//!                         Type-safe mutation gate (compile())
//!                                        ↓
//!                         Canary deploy → monitor → promote/rollback
//! ```
//!
//! # Three-tier synthesis
//!
//! | Tier | Analog | Technology | Latency |
//! |---|---|---|---|
//! | Polycephalum | Slime mold | Deterministic Rust rule engine | < 50ms |
//! | Ganglion | Nerve cluster | Local micro-LLM (Ollama) | seconds |
//! | Mammal Brain | Cortex | External LLM API (Claude) | seconds + cost |
//!
//! See [`ADR-0010`](../../docs/adrs/ADR-0010-bioiso-runtime-architecture.md).

pub mod signal;
pub mod store;
pub mod supervisor;

pub use signal::{now_ms, EntityId, MetricName, Signal, Timestamp};
pub use store::{EntityRecord, SignalStore, TelosBound};
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
}

impl Runtime {
    /// Create a new runtime, opening the signal store at `db_path`.
    ///
    /// Use `":memory:"` for tests; a file path for production deployments.
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            store: SignalStore::new(db_path)?,
            supervisor: EntitySupervisor::new(),
        })
    }

    /// Spawn a new entity: registers in supervisor + signal store.
    ///
    /// `telos_json` is the serialised telos declaration (used by the drift engine).
    /// `telomere_limit` is the maximum number of evolutions before senescence.
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
        self.store
            .register_entity(&entity_id, &entity_name, telos_json, born_at)?;
        self.supervisor
            .spawn(entity_id, entity_name, telomere_limit, on_exhaustion);
        Ok(())
    }

    /// Emit a telemetry signal from a running entity.
    pub fn emit(&self, signal: Signal) -> Result<(), rusqlite::Error> {
        self.store.write_signal(&signal)
    }

    /// Convenience: emit a named metric value for an entity with the current timestamp.
    pub fn emit_metric(
        &self,
        entity_id: impl Into<EntityId>,
        metric: impl Into<MetricName>,
        value: f64,
    ) -> Result<(), rusqlite::Error> {
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

    /// Number of currently Active entities.
    pub fn active_count(&self) -> usize {
        self.supervisor.active_count()
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
