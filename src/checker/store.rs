//! Store declaration checker. M92–M94.
//! Validates persistence store schemas: required fields, key declarations,
//! referential integrity, and store-kind constraints for operational (M93)
//! and analytical (M94) store kinds.

use std::collections::HashSet;

use crate::ast::*;
use crate::error::LoomError;

/// Store declaration checker.
///
/// Validates that each store's schema satisfies the requirements of its
/// declared store kind. Extended in M93 (operational stores) and M94
/// (analytical stores) with deeper per-kind validation.
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
            StoreKind::KeyValue        => self.check_keyvalue(store, errors),
            StoreKind::Relational      => self.check_relational(store, errors),
            StoreKind::Document        => self.check_document(store, errors),
            StoreKind::Columnar        => self.check_columnar(store, errors),
            StoreKind::Graph           => self.check_graph(store, errors),
            StoreKind::TimeSeries      => self.check_timeseries(store, errors),
            StoreKind::Vector          => self.check_vector(store, errors),
            StoreKind::Snowflake       => self.check_snowflake(store, errors),
            StoreKind::Hypercube       => self.check_hypercube(store, errors),
            StoreKind::FlatFile        => self.check_flatfile(store, errors),
            StoreKind::InMemory(_)     => self.check_inmemory(store, errors),
            StoreKind::Distributed     => self.check_distributed(store, errors),
            StoreKind::DistributedLog  => self.check_distributed_log(store, errors),
        }
    }

    // ── M93: Operational stores ───────────────────────────────────────────────

    /// Validate a Relational store.
    ///
    /// Rules:
    /// - Each table must have exactly one `@primary_key` field.
    /// - Field names must be unique within a table.
    /// - `@foreign_key(Table.field)` references must resolve to a declared
    ///   table in this store (warning when unresolvable — cross-store FKs
    ///   are allowed but unverifiable at compile time).
    fn check_relational(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let table_names: HashSet<String> = store.schema.iter()
            .filter_map(|e| {
                if let StoreSchemaEntry::Table { name, .. } = e { Some(name.clone()) } else { None }
            })
            .collect();

        for entry in &store.schema {
            if let StoreSchemaEntry::Table { name, fields, span } = entry {
                let pk_count = fields.iter()
                    .filter(|f| f.annotations.iter().any(|a| a.key == "primary_key"))
                    .count();

                if pk_count == 0 {
                    errors.push(LoomError::type_err(
                        format!(
                            "store table '{}': Relational table must have exactly one \
                             field annotated @primary_key (found 0)",
                            name
                        ),
                        span.clone(),
                    ));
                } else if pk_count > 1 {
                    errors.push(LoomError::type_err(
                        format!(
                            "store table '{}': Relational table must have exactly one \
                             @primary_key field (found {}); use a composite key type instead",
                            name, pk_count
                        ),
                        span.clone(),
                    ));
                }

                // Field name uniqueness within this table.
                let mut seen_fields: HashSet<&str> = HashSet::new();
                for field in fields {
                    if !seen_fields.insert(field.name.as_str()) {
                        errors.push(LoomError::type_err(
                            format!(
                                "store table '{}': duplicate field name '{}' — \
                                 field names must be unique within a table",
                                name, field.name
                            ),
                            field.span.clone(),
                        ));
                    }
                }

                // Foreign key cross-reference check.
                for field in fields {
                    for ann in &field.annotations {
                        if ann.key == "foreign_key" {
                            let target_table = ann.value
                                .split('.')
                                .next()
                                .unwrap_or("")
                                .trim_start_matches('(');
                            if !target_table.is_empty() && !table_names.contains(target_table) {
                                // Cross-store FK: warn but do not block compilation.
                                errors.push(LoomError::type_err(
                                    format!(
                                        "[hint] store table '{}', field '{}': \
                                         @foreign_key target table '{}' is not declared \
                                         in this store — cross-store foreign keys are \
                                         allowed but cannot be verified at compile time",
                                        name, field.name, target_table
                                    ),
                                    field.span.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate a KeyValue store.
    ///
    /// Rules:
    /// - Must have exactly one `key:` and one `value:` declaration.
    /// - `ttl:` config value must be a duration string ("30days", "1hour",
    ///   "forever", etc.) or a plain identifier (user-defined Duration type).
    /// - Hint emitted when key type is `String` — raw string keys bypass
    ///   Bloom filter optimisation.
    fn check_keyvalue(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let key_count = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::KeyType { .. }))
            .count();
        let value_count = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::ValueType { .. }))
            .count();

        if key_count == 0 {
            errors.push(LoomError::type_err(
                format!("store '{}': KeyValue store must declare 'key: Type'", store.name),
                store.span.clone(),
            ));
        } else if key_count > 1 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': KeyValue store must have exactly one 'key:' declaration \
                     (found {})",
                    store.name, key_count
                ),
                store.span.clone(),
            ));
        }

        if value_count == 0 {
            errors.push(LoomError::type_err(
                format!("store '{}': KeyValue store must declare 'value: Type'", store.name),
                store.span.clone(),
            ));
        } else if value_count > 1 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': KeyValue store must have exactly one 'value:' declaration \
                     (found {})",
                    store.name, value_count
                ),
                store.span.clone(),
            ));
        }

        // Hint: String key type without @hashed annotation misses Bloom filter.
        for entry in &store.schema {
            if let StoreSchemaEntry::KeyType { ty, span } = entry {
                if let TypeExpr::Base(type_name) = ty {
                    if type_name == "String" || type_name == "Str" {
                        errors.push(LoomError::type_err(
                            format!(
                                "[hint] store '{}': key type 'String' without @hashed \
                                 annotation — raw string keys miss Bloom filter \
                                 optimisation; add @hashed to the key declaration",
                                store.name
                            ),
                            span.clone(),
                        ));
                    }
                }
            }
        }

        // TTL format: must be "forever" or a number followed by a time unit suffix.
        for cfg in &store.config {
            if cfg.key == "ttl" && !is_valid_ttl(&cfg.value) {
                errors.push(LoomError::type_err(
                    format!(
                        "store '{}': ttl value '{}' is not a valid duration; \
                         expected a value like '30days', '1hour', '10minutes', \
                         '60seconds', or 'forever'",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
        }
    }

    /// Validate a Document store.
    ///
    /// Document stores are schema-flexible by design: no field annotations
    /// are required and `Json` fields are first-class. A hint is emitted
    /// for collections with zero fields (completely dynamic — may be
    /// intentional, but is worth flagging).
    fn check_document(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        for entry in &store.schema {
            if let StoreSchemaEntry::Collection { name, fields, span } = entry {
                if fields.is_empty() {
                    errors.push(LoomError::type_err(
                        format!(
                            "[hint] store '{}', collection '{}': zero fields declared — \
                             the collection is fully dynamic; add at least one field \
                             to improve type safety",
                            store.name, name
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
            return;
        }

        // Validate retention / resolution config formats.
        for cfg in &store.config {
            if cfg.key == "retention" && !is_valid_retention(&cfg.value) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': retention '{}' has unrecognized format — \
                         expected e.g. '90days', '1year', '24hours', 'forever'",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
            if cfg.key == "resolution" && !is_valid_resolution(&cfg.value) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': resolution '{}' has unrecognized format — \
                         expected e.g. '1second', '1minute', '1hour', '1day'",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
        }

        // Hint if no event has a timestamp field.
        const TIMESTAMP_NAMES: &[&str] = &["ts", "timestamp", "time", "datetime"];
        const TIMESTAMP_TYPES: &[&str] = &["DateTime", "Timestamp"];
        let has_timestamp = store.schema.iter().any(|e| {
            if let StoreSchemaEntry::Event { fields, .. } = e {
                fields.iter().any(|f| {
                    TIMESTAMP_NAMES.contains(&f.name.as_str())
                        || if let TypeExpr::Base(t) = &f.ty {
                            TIMESTAMP_TYPES.contains(&t.as_str())
                        } else {
                            false
                        }
                })
            } else {
                false
            }
        });
        if !has_timestamp {
            errors.push(LoomError::type_err(
                format!(
                    "[hint] store '{}': no timestamp field found in any event — \
                     consider adding a field named 'ts', 'timestamp', or of type DateTime",
                    store.name
                ),
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
            return;
        }

        // Validate index config (HNSW, IVFFlat, LSH, BruteForce).
        const VALID_INDEXES: &[&str] = &["HNSW", "IVFFlat", "LSH", "BruteForce"];
        for cfg in &store.config {
            if cfg.key == "index" && !VALID_INDEXES.contains(&cfg.value.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': index '{}' is unrecognized — \
                         valid values: HNSW, IVFFlat, LSH, BruteForce",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
        }

        // Hint if no embedding has a vector-shaped field.
        let has_vector_field = store.schema.iter().any(|e| {
            if let StoreSchemaEntry::EmbeddingEntry { fields, .. } = e {
                fields.iter().any(|f| {
                    if let TypeExpr::Base(t) = &f.ty {
                        t.contains("Tensor") || t.contains("Vec") || t.contains("Float")
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        });
        if !has_vector_field {
            errors.push(LoomError::type_err(
                format!(
                    "[hint] store '{}': no vector field found in embedding — \
                     consider a field of type Tensor, Vec, or Float array",
                    store.name
                ),
                store.span.clone(),
            ));
        }
    }

    // ── M94: Analytical stores ────────────────────────────────────────────────

    /// Validate a Columnar store (M94).
    ///
    /// Rules:
    /// - Must have at least one `schema` entry.
    /// - All field types should be scalar (no container generics `<>`); a hint
    ///   is emitted for any field whose type appears to be a container.
    /// - `@partition_key` is a valid field annotation.
    /// - Hint emitted when no numeric fields are declared — a columnar store
    ///   without aggregatable fields is unusual.
    fn check_columnar(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let schema_entries: Vec<&StoreSchemaEntry> = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::Collection { .. }))
            .collect();

        if schema_entries.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Columnar store must declare at least one 'schema' entry",
                    store.name
                ),
                store.span.clone(),
            ));
            return;
        }

        for entry in &store.schema {
            if let StoreSchemaEntry::Collection { name, fields, span } = entry {
                let has_numeric = fields.iter().any(|f| is_numeric_type(&f.ty));
                if !has_numeric {
                    errors.push(LoomError::type_err(
                        format!(
                            "[hint] store '{}', schema '{}': no numeric fields declared — \
                             columnar stores without aggregatable (Float/Int) fields are \
                             unusual; consider adding measure fields",
                            store.name, name
                        ),
                        span.clone(),
                    ));
                }

                for field in fields {
                    if is_container_type(&field.ty) {
                        errors.push(LoomError::type_err(
                            format!(
                                "[hint] store '{}', schema '{}', field '{}': container \
                                 type detected — columnar stores require scalar field \
                                 types for efficient vectorised execution",
                                store.name, name, field.name
                            ),
                            field.span.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Validate a Snowflake store (M94).
    ///
    /// Rules (reinforced from M92 base):
    /// - Must have exactly one `fact` entry and at least one `dimension`.
    /// - Fact fields with an `_id` suffix should have a corresponding
    ///   dimension (hint when unresolvable).
    /// - `@measure` and `@degenerate_dimension` are valid fact field annotations.
    fn check_snowflake(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let fact_count = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::Fact { .. }))
            .count();
        let dimension_count = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::DimensionEntry { .. }))
            .count();

        if fact_count == 0 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Snowflake store must declare exactly one 'fact' entry (found 0)",
                    store.name
                ),
                store.span.clone(),
            ));
        } else if fact_count > 1 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Snowflake store must have exactly one 'fact' entry \
                     (found {}); use a single composite fact table",
                    store.name, fact_count
                ),
                store.span.clone(),
            ));
        }

        if dimension_count == 0 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Snowflake store must declare at least one 'dimension' entry",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        // Check that _id-suffixed fact fields reference a declared dimension.
        let dimension_names: HashSet<String> = store.schema.iter()
            .filter_map(|e| {
                if let StoreSchemaEntry::DimensionEntry { name, .. } = e {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for entry in &store.schema {
            if let StoreSchemaEntry::Fact { name: fact_name, fields, span: _ } = entry {
                for field in fields {
                    if field.name.ends_with("_id") {
                        let dim_hint = field.name
                            .trim_end_matches("_id")
                            .split('_')
                            .map(|w| {
                                let mut c = w.chars();
                                match c.next() {
                                    None => String::new(),
                                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                                }
                            })
                            .collect::<String>();
                        if !dim_hint.is_empty() && !dimension_names.contains(&dim_hint) {
                            errors.push(LoomError::type_err(
                                format!(
                                    "[hint] store '{}', fact '{}', field '{}': \
                                     _id-suffixed field suggests a dimension reference \
                                     but no '{}' dimension is declared in this store",
                                    store.name, fact_name, field.name, dim_hint
                                ),
                                field.span.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Validate a Hypercube store (M94).
    ///
    /// Rules:
    /// - Must have at least one `dimension` entry (axes of the cube).
    /// - Must have at least one `fact` entry (the measure).
    /// - More than 12 dimensions triggers a combinatorial-explosion hint
    ///   (Gray 1996: OLAP cuboid count grows as 2^N — 12 dimensions = 4096
    ///   cuboids; materialising all is generally impractical).
    /// - `@sparse` via a config entry acknowledges a sparse cube.
    fn check_hypercube(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_fact = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::Fact { .. }));
        let dimension_count = store.schema.iter()
            .filter(|e| matches!(e, StoreSchemaEntry::DimensionEntry { .. }))
            .count();

        if !has_fact {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Hypercube store must declare at least one 'fact' entry \
                     (the measure of the cube)",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        if dimension_count == 0 {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Hypercube store must declare at least one 'dimension' \
                     entry (the axes of the cube)",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        // Gray (1996): >12 dimensions leads to combinatorial explosion of cuboids.
        if dimension_count > 12 {
            let is_sparse = store.config.iter().any(|c| c.key == "sparse");
            if !is_sparse {
                errors.push(LoomError::type_err(
                    format!(
                        "[hint] store '{}': {} dimensions declared — with >12 dimensions \
                         the cuboid count (2^N = {}) makes full pre-materialisation \
                         impractical (Gray 1996); add 'sparse: true' config to \
                         acknowledge a sparse cube",
                        store.name,
                        dimension_count,
                        1usize << dimension_count.min(63)
                    ),
                    store.span.clone(),
                ));
            }
        }
    }

    fn check_flatfile(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        // FlatFile is the scientist's escape hatch — no external system required.
        if store.schema.is_empty() {
            errors.push(LoomError::type_err(
                format!(
                    "[info] store '{}': FlatFile store has no schema entries \
                     (FlatFile is the scientist's escape hatch — no external system \
                     required, but consider adding an 'event' entry)",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        const VALID_FORMATS: &[&str] = &["Parquet", "Arrow", "HDF5", "CSV", "JsonLines", "MsgPack"];
        const VALID_COMPRESSIONS: &[&str] = &["Zstd", "LZ4", "Snappy", "Gzip", "None"];
        for cfg in &store.config {
            if cfg.key == "format" && !VALID_FORMATS.contains(&cfg.value.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': format '{}' is unrecognized — \
                         valid values: Parquet, Arrow, HDF5, CSV, JsonLines, MsgPack",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
            if cfg.key == "compression" && !VALID_COMPRESSIONS.contains(&cfg.value.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': compression '{}' is unrecognized — \
                         valid values: Zstd, LZ4, Snappy, Gzip, None",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
        }
    }

    /// Validate an InMemory store (M96).
    ///
    /// Rules:
    /// - `capacity` config if present must be a positive integer.
    /// - `eviction` config: valid values are LRU, LFU, ARC (Megiddo 2003), FIFO, None.
    fn check_inmemory(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        const VALID_EVICTIONS: &[&str] = &["LRU", "LFU", "ARC", "FIFO", "None"];
        for cfg in &store.config {
            if cfg.key == "capacity" && cfg.value.parse::<u64>().is_err() {
                errors.push(LoomError::type_err(
                    format!(
                        "store '{}': capacity '{}' must be a positive integer",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
            if cfg.key == "eviction" && !VALID_EVICTIONS.contains(&cfg.value.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] store '{}': eviction '{}' is unrecognized — \
                         valid values: LRU, LFU, ARC (Megiddo 2003), FIFO, None",
                        store.name, cfg.value
                    ),
                    cfg.span.clone(),
                ));
            }
        }
    }

    /// Validate a Distributed MapReduce store (M97, Dean & Ghemawat 2004).
    ///
    /// Rules:
    /// - Must have at least one `schema`/collection entry (input record type).
    /// - Must have at least one `mapreduce` job.
    /// - Each job's map signature should contain `->` and a list `[` indicating
    ///   key-value output.
    /// - Warn if `replication` config is absent.
    fn check_distributed(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_schema = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::Collection { .. }));
        let has_mapreduce = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::MapReduceJob(_)));

        if !has_schema {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Distributed store must declare at least one 'schema' entry",
                    store.name
                ),
                store.span.clone(),
            ));
        }
        if !has_mapreduce {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': Distributed store must declare at least one 'mapreduce' job",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        for entry in &store.schema {
            if let StoreSchemaEntry::MapReduceJob(mr) = entry {
                if !mr.map_sig.contains("->") {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] mapreduce '{}': map signature should contain '->' \
                             indicating key-value pair output",
                            mr.name
                        ),
                        mr.span.clone(),
                    ));
                }
            }
        }

        let has_replication = store.config.iter().any(|c| c.key == "replication");
        if !has_replication {
            errors.push(LoomError::type_err(
                format!(
                    "[warn] store '{}': replication not set — \
                     fault tolerance requires replication >= 2 in production",
                    store.name
                ),
                store.span.clone(),
            ));
        }
    }

    /// Validate a DistributedLog store (M97, Kafka-style, Kreps 2011).
    ///
    /// Rules:
    /// - Must have at least one `event` entry.
    /// - Consumer offsets must be `earliest`, `latest`, or a timestamp string.
    /// - Warn if no consumers declared.
    /// - Warn if `partitions` config is absent.
    fn check_distributed_log(&self, store: &StoreDef, errors: &mut Vec<LoomError>) {
        let has_event = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::Event { .. }));
        if !has_event {
            errors.push(LoomError::type_err(
                format!(
                    "store '{}': DistributedLog store must declare at least one 'event' entry",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        for entry in &store.schema {
            if let StoreSchemaEntry::LogConsumer(c) = entry {
                if c.offset != "earliest" && c.offset != "latest" && !looks_like_timestamp(&c.offset) {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] consumer '{}': offset '{}' is not 'earliest', 'latest', \
                             or a recognizable timestamp",
                            c.name, c.offset
                        ),
                        c.span.clone(),
                    ));
                }
            }
        }

        let has_consumer = store.schema.iter().any(|e| matches!(e, StoreSchemaEntry::LogConsumer(_)));
        if !has_consumer {
            errors.push(LoomError::type_err(
                format!(
                    "[warn] store '{}': DistributedLog has no consumers — \
                     a log with no consumers is valid but unusual",
                    store.name
                ),
                store.span.clone(),
            ));
        }

        let has_partitions = store.config.iter().any(|c| c.key == "partitions");
        if !has_partitions {
            errors.push(LoomError::type_err(
                format!(
                    "[warn] store '{}': partitions not set — \
                     setting partitions improves throughput",
                    store.name
                ),
                store.span.clone(),
            ));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` if a TTL config value is a valid duration string.
///
/// Valid forms: `"forever"` or a positive integer followed by a recognised
/// time-unit suffix (`days`, `hours`, `minutes`, `seconds`, `weeks`, `ms`, `us`).
fn is_valid_ttl(value: &str) -> bool {
    if value == "forever" {
        return true;
    }
    let digits_end = value.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digits_end == 0 {
        // No leading digits — check if it's a plain identifier (user-defined type).
        return value.chars().all(|c| c.is_alphanumeric() || c == '_');
    }
    let suffix = &value[digits_end..];
    matches!(
        suffix,
        "days" | "hours" | "minutes" | "seconds" | "weeks" | "ms" | "us" | "ns"
    )
}

/// Returns `true` if the type expression is a container (generic) type.
///
/// Used by the Columnar checker to warn about non-scalar field types.
fn is_container_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Generic(_, _) | TypeExpr::Effect(_, _) | TypeExpr::Option(_)
            | TypeExpr::Result(_, _) | TypeExpr::Tuple(_)
    )
}

/// Returns `true` if the type expression is a numeric scalar suitable for
/// columnar aggregation (Float, Int, or their aliases).
fn is_numeric_type(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Base(name) => matches!(
            name.as_str(),
            "Float" | "Int" | "Int32" | "Int64" | "UInt" | "UInt32" | "UInt64"
                | "f64" | "f32" | "i64" | "i32" | "Double" | "Decimal" | "Number"
        ),
        _ => false,
    }
}

/// Validate a retention string (e.g. `90days`, `1year`, `forever`, `24hours`).
fn is_valid_retention(s: &str) -> bool {
    if s == "forever" { return true; }
    let suffixes = [
        "days", "day", "hours", "hour", "minutes", "minute",
        "weeks", "week", "months", "month", "years", "year",
    ];
    for suffix in &suffixes {
        if s.ends_with(suffix) {
            let prefix = &s[..s.len() - suffix.len()];
            if prefix.parse::<u64>().is_ok() { return true; }
        }
    }
    false
}

/// Validate a resolution string (e.g. `1second`, `1minute`, `1hour`, `1day`).
fn is_valid_resolution(s: &str) -> bool {
    let suffixes = ["second", "seconds", "minute", "minutes", "hour", "hours", "day", "days"];
    for suffix in &suffixes {
        if s.ends_with(suffix) {
            let prefix = &s[..s.len() - suffix.len()];
            if prefix.parse::<u64>().is_ok() { return true; }
        }
    }
    false
}

/// Heuristic: does this string look like a timestamp offset (not earliest/latest)?
fn looks_like_timestamp(s: &str) -> bool {
    s.contains('-') || s.contains('T') || s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
}
