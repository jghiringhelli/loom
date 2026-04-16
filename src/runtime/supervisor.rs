//! Entity supervisor — lifecycle state machine for running Loom entities.
//!
//! The supervisor maintains in-memory records of every spawned entity and
//! coordinates with the signal store to persist lifecycle transitions.

use std::collections::HashMap;

use crate::runtime::signal::{now_ms, EntityId, Timestamp};
use crate::runtime::store::SignalStore;

/// The lifecycle state of a running entity.
///
/// Matches the Hayflick-derived states in the Loom type system:
/// Active → Warning → Diverging → Senescent → Dead.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityState {
    /// Healthy and operating within telos bounds.
    Active,
    /// Telos score below the `warning` threshold. Repair is triggered.
    Warning,
    /// Telos score below the `divergence` threshold. Apoptosis is imminent.
    Diverging,
    /// Telomere exhausted — entity is in terminal phase, no further divisions.
    Senescent,
    /// Entity has terminated.
    Dead,
}

impl EntityState {
    /// String representation matching the `entities.state` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityState::Active => "active",
            EntityState::Warning => "warning",
            EntityState::Diverging => "diverging",
            EntityState::Senescent => "senescent",
            EntityState::Dead => "dead",
        }
    }

    /// Parse from the store string representation.
    pub fn from_str(s: &str) -> Self {
        match s {
            "warning" => EntityState::Warning,
            "diverging" => EntityState::Diverging,
            "senescent" => EntityState::Senescent,
            "dead" => EntityState::Dead,
            _ => EntityState::Active,
        }
    }
}

/// Runtime record for a single supervised entity instance.
#[derive(Debug, Clone)]
pub struct EntityInstance {
    /// Unique runtime identifier.
    pub id: EntityId,
    /// Human-readable name from the `being:` declaration.
    pub name: String,
    /// Current lifecycle state.
    pub state: EntityState,
    /// Number of division/evolution events recorded so far.
    pub division_count: u32,
    /// Maximum allowed divisions before senescence (`telomere: limit:`).
    pub telomere_limit: Option<u32>,
    /// What to do on exhaustion (`telomere: on_exhaustion:`).
    pub on_exhaustion: Option<String>,
    /// Wall-clock time this entity was spawned (unix ms).
    pub spawned_at: Timestamp,
}

impl EntityInstance {
    /// Record a division event (evolve / clone).
    ///
    /// Returns `Err` with the on-exhaustion message if the telomere is exhausted.
    pub fn divide(&mut self) -> Result<(), String> {
        self.division_count += 1;
        if let Some(limit) = self.telomere_limit {
            if self.division_count >= limit {
                self.state = EntityState::Senescent;
                return Err(format!(
                    "entity '{}' telomere exhausted after {} divisions: {}",
                    self.id,
                    self.division_count,
                    self.on_exhaustion.as_deref().unwrap_or("senescence")
                ));
            }
        }
        Ok(())
    }

    /// Transition the entity to the Dead state.
    pub fn kill(&mut self) {
        self.state = EntityState::Dead;
    }

    /// Whether this entity can still participate in the evolution loop.
    pub fn is_alive(&self) -> bool {
        self.state != EntityState::Dead
    }
}

/// Supervises all running entity instances, coordinating with the signal store
/// for lifecycle persistence.
pub struct EntitySupervisor {
    entities: HashMap<EntityId, EntityInstance>,
}

impl EntitySupervisor {
    /// Create an empty supervisor.
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    /// Spawn a new entity and begin supervising it.
    ///
    /// Returns a reference to the newly created instance.
    pub fn spawn(
        &mut self,
        id: impl Into<EntityId>,
        name: impl Into<String>,
        telomere_limit: Option<u32>,
        on_exhaustion: Option<String>,
    ) -> &EntityInstance {
        let entity_id: EntityId = id.into();
        let instance = EntityInstance {
            id: entity_id.clone(),
            name: name.into(),
            state: EntityState::Active,
            division_count: 0,
            telomere_limit,
            on_exhaustion,
            spawned_at: now_ms(),
        };
        self.entities.insert(entity_id.clone(), instance);
        &self.entities[&entity_id]
    }

    /// Return a reference to an entity by id.
    pub fn get(&self, id: &str) -> Option<&EntityInstance> {
        self.entities.get(id)
    }

    /// Return a mutable reference to an entity by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut EntityInstance> {
        self.entities.get_mut(id)
    }

    /// Record a division for `entity_id`, syncing state to the store.
    ///
    /// Returns `Err` if the telomere is exhausted (entity transitions to Senescent).
    pub fn record_division(&mut self, entity_id: &str, store: &SignalStore) -> Result<(), String> {
        let instance = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| format!("entity '{}' not found", entity_id))?;
        let result = instance.divide();
        let _ = store.set_entity_state(entity_id, instance.state.as_str());
        result
    }

    /// Transition an entity to a new lifecycle state, persisting to the store.
    pub fn transition(&mut self, entity_id: &str, new_state: EntityState, store: &SignalStore) {
        if let Some(instance) = self.entities.get_mut(entity_id) {
            instance.state = new_state;
            let _ = store.set_entity_state(entity_id, instance.state.as_str());
        }
    }

    /// Return the ids of all entities that are not Dead.
    pub fn living_entity_ids(&self) -> Vec<EntityId> {
        self.entities
            .values()
            .filter(|e| e.is_alive())
            .map(|e| e.id.clone())
            .collect()
    }

    /// Number of entities in the Active state.
    pub fn active_count(&self) -> usize {
        self.entities
            .values()
            .filter(|e| e.state == EntityState::Active)
            .count()
    }

    /// Total number of supervised entities (all states).
    pub fn total_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for EntitySupervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::store::SignalStore;

    fn mem_store() -> SignalStore {
        let s = SignalStore::new(":memory:").unwrap();
        s
    }

    #[test]
    fn spawn_creates_active_entity() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "ClimateModel", Some(50), Some("senescence".into()));
        let e = sup.get("e1").unwrap();
        assert_eq!(e.state, EntityState::Active);
        assert_eq!(e.division_count, 0);
        assert_eq!(e.telomere_limit, Some(50));
    }

    #[test]
    fn divide_below_limit_stays_active() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "Foo", Some(5), None);
        let store = mem_store();
        store.register_entity("e1", "Foo", "{}", 0).unwrap();
        for _ in 0..4 {
            sup.record_division("e1", &store).unwrap();
        }
        assert_eq!(sup.get("e1").unwrap().state, EntityState::Active);
        assert_eq!(sup.get("e1").unwrap().division_count, 4);
    }

    #[test]
    fn divide_at_limit_triggers_senescence() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "Foo", Some(3), Some("graceful_shutdown".into()));
        let store = mem_store();
        store.register_entity("e1", "Foo", "{}", 0).unwrap();
        sup.record_division("e1", &store).unwrap();
        sup.record_division("e1", &store).unwrap();
        let result = sup.record_division("e1", &store);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("telomere exhausted"));
        assert!(msg.contains("graceful_shutdown"));
        assert_eq!(sup.get("e1").unwrap().state, EntityState::Senescent);
    }

    #[test]
    fn no_telomere_limit_never_senesces() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "Foo", None, None);
        let store = mem_store();
        store.register_entity("e1", "Foo", "{}", 0).unwrap();
        for _ in 0..100 {
            sup.record_division("e1", &store).unwrap();
        }
        assert_eq!(sup.get("e1").unwrap().state, EntityState::Active);
    }

    #[test]
    fn transition_updates_state_and_store() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "Foo", None, None);
        let store = mem_store();
        store.register_entity("e1", "Foo", "{}", 0).unwrap();
        sup.transition("e1", EntityState::Warning, &store);
        assert_eq!(sup.get("e1").unwrap().state, EntityState::Warning);
        let entities = store.all_entities().unwrap();
        assert_eq!(entities[0].state, "warning");
    }

    #[test]
    fn living_ids_excludes_dead() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "Foo", None, None);
        sup.spawn("e2", "Bar", None, None);
        sup.get_mut("e2").unwrap().kill();
        let living = sup.living_entity_ids();
        assert_eq!(living, vec!["e1"]);
    }

    #[test]
    fn active_count_reflects_state() {
        let mut sup = EntitySupervisor::new();
        sup.spawn("e1", "A", None, None);
        sup.spawn("e2", "B", None, None);
        sup.spawn("e3", "C", None, None);
        let store = mem_store();
        store.register_entity("e2", "B", "{}", 0).unwrap();
        sup.transition("e2", EntityState::Warning, &store);
        assert_eq!(sup.active_count(), 2);
        assert_eq!(sup.total_count(), 3);
    }
}
