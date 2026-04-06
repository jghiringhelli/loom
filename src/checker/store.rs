//! Store declaration checker. M92.
//! Validates persistence store schemas: required fields, key declarations,
//! referential integrity, and store-kind constraints.

use std::collections::HashSet;

use crate::ast::*;
use crate::error::LoomError;

/// Store declaration checker.
///
/// Validates that each store's schema satisfies the requirements of its
/// declared store kind.
pub struct StoreChecker;

impl StoreChecker {
    /// Create a new store checker.
    pub fn new() -> Self {
        StoreChecker
    }

    /// Check all stores in `module` for schema validity.
    ///
    /// Returns errors for invalid store declarations.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Store(sd) = item {
                self.check_store(sd, &mut errors);
            }
        }
        errors
    }

    fn check_store(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        match &store.kind {
            StoreKind::KeyValue => self.check_keyvalue(store, errors),
            StoreKind::Relational => self.check_relational(store, errors),
            StoreKind::Graph => self.check_graph(store, errors),
            StoreKind::TimeSeries => self.check_timeseries(store, errors),
            StoreKind::Vector => self.check_vector(store, errors),
            StoreKind::Snowflake | StoreKind::Hypercube => self.check_olap(store, errors),
            StoreKind::FlatFile | StoreKind::InMemory(_) => self.check_flatfile(store, errors),
            _ => {}
        }
    }

    fn check_keyvalue(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_key = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::KeyType { .. }));
        let has_value = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::ValueType { .. }));
        if !has_key {
            errors.push(LoomError::type_err(
                format!("store '{}': KeyValue store must declare 'key: Type'", store.name),
                store.span.clone(),
            ));
        }
        if !has_value {
            errors.push(LoomError::type_err(
                format!("store '{}': KeyValue store must declare 'value: Type'", store.name),
                store.span.clone(),
            ));
        }
    }

    fn check_relational(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        for entry in &store.schema {
            if let StoreSchemaEntry::Table { name, fields, span } = entry {
                let has_pk = fields.iter().any(|f| {
                    f.annotations.iter().any(|a| a.key == "primary_key")
                });
                if !has_pk {
                    errors.push(LoomError::type_err(
                        format!(
                            "store table '{}': Relational table must have at least one field \
                             annotated @primary_key",
                            name
                        ),
                        span.clone(),
                    ));
                }
            }
        }
    }

    fn check_graph(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let node_names: HashSet<String> = store.schema.iter()
            .filter_map(|e| {
                if let StoreSchemaEntry::Node { name, .. } = e { Some(name.clone()) } else { None }
            })
            .collect();

        if node_names.is_empty() {
            errors.push(LoomError::type_err(
                format!("store '{}': Graph store must declare at least one 'node' entry", store.name),
                store.span.clone(),
            ));
        }

        for entry in &store.schema {
            if let StoreSchemaEntry::Edge { name, source, target, span, .. } = entry {
                if !node_names.contains(source) {
                    errors.push(LoomError::type_err(
                        format!(
                            "store edge '{}': source node '{}' is not declared in this graph store",
                            name, source
                        ),
                        span.clone(),
                    ));
                }
                if !node_names.contains(target) {
                    errors.push(LoomError::type_err(
                        format!(
                            "store edge '{}': target node '{}' is not declared in this graph store",
                            name, target
                        ),
                        span.clone(),
                    ));
                }
            }
        }
    }

    fn check_timeseries(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_event = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::Event { .. }));
        if !has_event {
            errors.push(LoomError::type_err(
                format!("store '{}': TimeSeries store must declare at least one 'event' entry", store.name),
                store.span.clone(),
            ));
        }
    }

    fn check_vector(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_embedding = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::EmbeddingEntry { .. }));
        if !has_embedding {
            errors.push(LoomError::type_err(
                format!("store '{}': Vector store must declare at least one 'embedding' entry", store.name),
                store.span.clone(),
            ));
        }
    }

    fn check_olap(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_fact = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::Fact { .. }));
        let has_dimension = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::DimensionEntry { .. }));
        if !has_fact {
            errors.push(LoomError::type_err(
                format!("store '{}': Snowflake/Hypercube store must declare at least one 'fact' entry", store.name),
                store.span.clone(),
            ));
        }
        if !has_dimension {
            errors.push(LoomError::type_err(
                format!("store '{}': Snowflake/Hypercube store must declare at least one 'dimension' entry", store.name),
                store.span.clone(),
            ));
        }
    }

    fn check_flatfile(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        if store.schema.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': FlatFile/InMemory store has no schema entries — \
                     schema may be dynamic but consider adding an 'event' or other entry",
                    store.name
                ),
                store.span.clone(),
            ));
        }
    }
}
