//! Signal store — append-only SQLite persistence for runtime telemetry.
//!
//! All tables are append-only: signals, drift events, mutation proposals, and
//! checkpoints are never updated in-place. Entity state is the only mutable row.

use rusqlite::{params, Connection, Result as SqlResult};

use crate::runtime::signal::{EntityId, Signal, Timestamp};

/// Append-only SQLite-backed store for all BIOISO runtime data.
///
/// Use `":memory:"` for tests; a file path for production.
pub struct SignalStore {
    conn: Connection,
}

impl SignalStore {
    /// Open or create a signal store at `path`.
    ///
    /// Runs `CREATE TABLE IF NOT EXISTS` for all tables on first open.
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.create_schema()?;
        Ok(store)
    }

    fn create_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS entities (
                id       TEXT PRIMARY KEY,
                name     TEXT NOT NULL,
                telos_json TEXT,
                born_at  INTEGER NOT NULL,
                state    TEXT NOT NULL DEFAULT 'active'
            );
            CREATE TABLE IF NOT EXISTS signals (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_id TEXT NOT NULL,
                metric    TEXT NOT NULL,
                value     REAL NOT NULL,
                ts        INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_signals_entity
                ON signals(entity_id, ts);
            CREATE TABLE IF NOT EXISTS telos_bounds (
                entity_id TEXT NOT NULL,
                metric    TEXT NOT NULL,
                min       REAL,
                max       REAL,
                target    REAL,
                PRIMARY KEY (entity_id, metric)
            );
            CREATE TABLE IF NOT EXISTS drift_events (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_id         TEXT NOT NULL,
                score             REAL NOT NULL,
                ts                INTEGER NOT NULL,
                triggering_signal TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_drift_entity
                ON drift_events(entity_id, ts);
            CREATE TABLE IF NOT EXISTS mutation_proposals (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_id      TEXT NOT NULL,
                tier           INTEGER NOT NULL,
                proposal_json  TEXT NOT NULL,
                verdict        TEXT NOT NULL,
                checker_errors TEXT,
                ts             INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS checkpoints (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_id  TEXT NOT NULL,
                state_json TEXT NOT NULL,
                ts         INTEGER NOT NULL
            );
            ",
        )
    }

    // ── Entity registration ───────────────────────────────────────────────────

    /// Register a new entity. No-op if the id already exists.
    pub fn register_entity(
        &self,
        id: &str,
        name: &str,
        telos_json: &str,
        born_at: Timestamp,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entities (id, name, telos_json, born_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, telos_json, born_at as i64],
        )?;
        Ok(())
    }

    /// Update the lifecycle state of an entity.
    pub fn set_entity_state(&self, entity_id: &str, state: &str) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE entities SET state = ?1 WHERE id = ?2",
            params![state, entity_id],
        )?;
        Ok(())
    }

    /// Return all registered entity records.
    pub fn all_entities(&self) -> SqlResult<Vec<EntityRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, telos_json, born_at, state FROM entities",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EntityRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                telos_json: row.get(2)?,
                born_at: row.get::<_, i64>(3)? as u64,
                state: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    // ── Signal telemetry ──────────────────────────────────────────────────────

    /// Append a telemetry signal. The store is append-only; nothing is updated.
    pub fn write_signal(&self, signal: &Signal) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO signals (entity_id, metric, value, ts) VALUES (?1, ?2, ?3, ?4)",
            params![signal.entity_id, signal.metric, signal.value, signal.timestamp as i64],
        )?;
        Ok(())
    }

    /// Return up to `limit` most-recent signals for `entity_id`, newest first.
    pub fn signals_for_entity(&self, entity_id: &str, limit: usize) -> SqlResult<Vec<Signal>> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id, metric, value, ts FROM signals
             WHERE entity_id = ?1
             ORDER BY ts DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![entity_id, limit as i64], |row| {
            Ok(Signal {
                entity_id: row.get(0)?,
                metric: row.get(1)?,
                value: row.get(2)?,
                timestamp: row.get::<_, i64>(3)? as u64,
            })
        })?;
        rows.collect()
    }

    // ── Telos bounds ──────────────────────────────────────────────────────────

    /// Set (or replace) telos bounds for a specific metric on an entity.
    pub fn set_telos_bounds(
        &self,
        entity_id: &str,
        metric: &str,
        min: Option<f64>,
        max: Option<f64>,
        target: Option<f64>,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO telos_bounds (entity_id, metric, min, max, target)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entity_id, metric, min, max, target],
        )?;
        Ok(())
    }

    /// Return all telos bounds registered for an entity.
    pub fn telos_bounds_for_entity(&self, entity_id: &str) -> SqlResult<Vec<TelosBound>> {
        let mut stmt = self.conn.prepare(
            "SELECT metric, min, max, target FROM telos_bounds WHERE entity_id = ?1",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(TelosBound {
                metric: row.get(0)?,
                min: row.get(1)?,
                max: row.get(2)?,
                target: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    // ── Drift events ──────────────────────────────────────────────────────────

    /// Append a drift event for an entity.
    pub fn record_drift_event(
        &self,
        entity_id: &str,
        score: f64,
        ts: Timestamp,
        triggering_signal: Option<&str>,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO drift_events (entity_id, score, ts, triggering_signal)
             VALUES (?1, ?2, ?3, ?4)",
            params![entity_id, score, ts as i64, triggering_signal],
        )?;
        Ok(())
    }

    /// Return the most recent drift score for an entity, if any.
    pub fn latest_drift_score(&self, entity_id: &str) -> SqlResult<Option<f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT score FROM drift_events WHERE entity_id = ?1 ORDER BY ts DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![entity_id], |row| row.get(0))?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    // ── Mutation proposals ────────────────────────────────────────────────────

    /// Record a mutation proposal outcome (accepted or rejected).
    pub fn record_mutation(
        &self,
        entity_id: &str,
        tier: u8,
        proposal_json: &str,
        verdict: &str,
        checker_errors: Option<&str>,
        ts: Timestamp,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO mutation_proposals
             (entity_id, tier, proposal_json, verdict, checker_errors, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entity_id,
                tier as i64,
                proposal_json,
                verdict,
                checker_errors,
                ts as i64
            ],
        )?;
        Ok(())
    }

    // ── Checkpoints ───────────────────────────────────────────────────────────

    /// Snapshot an entity's state. Returns the new checkpoint id.
    pub fn create_checkpoint(&self, entity_id: &str, state_json: &str, ts: Timestamp) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO checkpoints (entity_id, state_json, ts) VALUES (?1, ?2, ?3)",
            params![entity_id, state_json, ts as i64],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Retrieve a checkpoint's state JSON by id.
    pub fn get_checkpoint(&self, checkpoint_id: i64) -> SqlResult<Option<String>> {
        let mut stmt = self.conn
            .prepare("SELECT state_json FROM checkpoints WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![checkpoint_id], |row| row.get(0))?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }
}

// ── Value types ───────────────────────────────────────────────────────────────

/// Telos bounds for a single metric on an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct TelosBound {
    /// The metric name (e.g. `"temperature_delta"`).
    pub metric: String,
    /// Lower bound — signal below this is in violation.
    pub min: Option<f64>,
    /// Upper bound — signal above this is in violation.
    pub max: Option<f64>,
    /// Ideal target value.
    pub target: Option<f64>,
}

/// A row from the `entities` table.
#[derive(Debug, Clone)]
pub struct EntityRecord {
    pub id: String,
    pub name: String,
    pub telos_json: Option<String>,
    pub born_at: Timestamp,
    pub state: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::signal::Signal;

    fn mem() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    #[test]
    fn register_entity_and_list() {
        let s = mem();
        s.register_entity("e1", "ClimateModel", r#"{"target":1.5}"#, 0).unwrap();
        let entities = s.all_entities().unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "e1");
        assert_eq!(entities[0].state, "active");
    }

    #[test]
    fn register_entity_idempotent() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        s.register_entity("e1", "Foo", "{}", 0).unwrap(); // second call is no-op
        assert_eq!(s.all_entities().unwrap().len(), 1);
    }

    #[test]
    fn write_and_read_signal_roundtrip() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        let sig = Signal::with_timestamp("e1", "co2_ppm", 420.0, 1_000);
        s.write_signal(&sig).unwrap();
        let signals = s.signals_for_entity("e1", 10).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].metric, "co2_ppm");
        assert!((signals[0].value - 420.0).abs() < 1e-9);
        assert_eq!(signals[0].timestamp, 1_000);
    }

    #[test]
    fn signals_ordered_newest_first() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        s.write_signal(&Signal::with_timestamp("e1", "m", 1.0, 100)).unwrap();
        s.write_signal(&Signal::with_timestamp("e1", "m", 2.0, 200)).unwrap();
        s.write_signal(&Signal::with_timestamp("e1", "m", 3.0, 300)).unwrap();
        let sigs = s.signals_for_entity("e1", 10).unwrap();
        assert_eq!(sigs[0].timestamp, 300); // newest first
        assert_eq!(sigs[2].timestamp, 100);
    }

    #[test]
    fn telos_bounds_set_and_retrieve() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        s.set_telos_bounds("e1", "temperature", Some(0.0), Some(2.0), Some(1.5)).unwrap();
        let bounds = s.telos_bounds_for_entity("e1").unwrap();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].metric, "temperature");
        assert_eq!(bounds[0].min, Some(0.0));
        assert_eq!(bounds[0].max, Some(2.0));
        assert_eq!(bounds[0].target, Some(1.5));
    }

    #[test]
    fn drift_event_round_trip() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        s.record_drift_event("e1", 0.82, 1_000, Some("temperature")).unwrap();
        let score = s.latest_drift_score("e1").unwrap();
        assert!((score.unwrap() - 0.82).abs() < 1e-9);
    }

    #[test]
    fn no_drift_returns_none() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        assert_eq!(s.latest_drift_score("e1").unwrap(), None);
    }

    #[test]
    fn checkpoint_create_and_retrieve() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        let id = s.create_checkpoint("e1", r#"{"param":42}"#, 500).unwrap();
        let state = s.get_checkpoint(id).unwrap();
        assert_eq!(state.as_deref(), Some(r#"{"param":42}"#));
    }

    #[test]
    fn checkpoint_missing_returns_none() {
        let s = mem();
        assert_eq!(s.get_checkpoint(9999).unwrap(), None);
    }

    #[test]
    fn set_entity_state_updates_row() {
        let s = mem();
        s.register_entity("e1", "Foo", "{}", 0).unwrap();
        s.set_entity_state("e1", "senescent").unwrap();
        let entities = s.all_entities().unwrap();
        assert_eq!(entities[0].state, "senescent");
    }
}
