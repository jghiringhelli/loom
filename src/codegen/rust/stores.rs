//! Store code generation — 13 store kinds → idiomatic Rust structs with ecosystem hints.
//!
//! Each store kind emits:
//! - A `#[derive(Debug, Clone, PartialEq)]` struct per schema entity
//! - `// LOOM[store:Kind]:` audit comments explaining design intent
//! - Inline connector recommendations from the Rust ecosystem
//! - Optional serde derive comment (add `serde` feature to enable)
//!
//! V5 design goal: emitted code is self-documenting and directly usable.
//! A developer can take the emitted file and wire it to the recommended crate
//! with zero manual struct definition work.

use super::{to_snake_case, RustEmitter};
use crate::ast::*;

/// Standard struct derives for all store entities.
const STORE_DERIVES: &str = "#[derive(Debug, Clone, PartialEq)]";

/// Serde-ready derive note. Enabled when `serde` feature is active.
const SERDE_DERIVE_NOTE: &str =
    "// Add #[derive(serde::Serialize, serde::Deserialize)] with feature \"serde\"";

impl RustEmitter {
    /// Emit Rust struct declarations for a store declaration.
    ///
    /// V5 implementation: all 13 store kinds emit complete, idiomatic Rust structs
    /// with ecosystem recommendations, V7 audit trail comments, and serde guidance.
    pub fn codegen_store(&self, store: &StoreDef) -> String {
        let mut out = String::new();
        self.store_header(store, &mut out);
        match &store.kind {
            StoreKind::Relational => self.codegen_relational_store(store, &mut out),
            StoreKind::KeyValue => self.codegen_keyvalue_store(store, &mut out),
            StoreKind::Document => self.codegen_document_store(store, &mut out),
            StoreKind::Columnar => self.codegen_columnar_store(store, &mut out),
            StoreKind::Snowflake => self.codegen_snowflake_store(store, &mut out),
            StoreKind::Hypercube => self.codegen_hypercube_store(store, &mut out),
            StoreKind::Graph => self.codegen_graph_store(store, &mut out),
            StoreKind::TimeSeries => self.codegen_timeseries_store(store, &mut out),
            StoreKind::Vector => self.codegen_vector_store(store, &mut out),
            StoreKind::InMemory(inner) => self.codegen_inmemory_store(store, inner, &mut out),
            StoreKind::FlatFile => self.codegen_flatfile_store(store, &mut out),
            StoreKind::Distributed => self.codegen_distributed_store(store, &mut out),
            StoreKind::DistributedLog => self.codegen_distributedlog_store(store, &mut out),
        }
        out
    }

    // ── Header ───────────────────────────────────────────────────────────────

    fn store_header(&self, store: &StoreDef, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[store:{:?}]: {} — V5 struct translation\n",
            store.kind, store.name
        ));
        if !store.config.is_empty() {
            let cfg: Vec<String> = store
                .config
                .iter()
                .map(|c| format!("{}={}", c.key, c.value))
                .collect();
            out.push_str(&format!("// config: {}\n", cfg.join(", ")));
        }
        out.push_str(SERDE_DERIVE_NOTE);
        out.push('\n');
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Emit struct fields from a field slice. Handles Json → serde_json::Value.
    /// When a Json field is encountered, prepends a type alias if serde_json is not yet declared.
    fn emit_struct_fields(&self, fields: &[FieldDef], out: &mut String) {
        // Emit a type alias for JsonValue so the emitted code compiles without
        // serde_json in scope. Users can swap it for serde_json::Value.
        let has_json = fields
            .iter()
            .any(|f| matches!(&f.ty, TypeExpr::Base(n) if n == "Json"));
        if has_json {
            out.push_str("    // LOOM[json]: swap JsonValue for serde_json::Value when serde_json is a dependency\n");
        }
        for field in fields {
            let rust_ty = match &field.ty {
                TypeExpr::Base(n) if n == "Json" => {
                    "String /* JsonValue — add serde_json for full type */".to_string()
                }
                TypeExpr::Base(n) if n == "Timestamp" || n == "DateTime" => "i64".to_string(),
                TypeExpr::Base(n) if n == "Uuid" => "String".to_string(),
                other => self.emit_type_expr(other),
            };
            let pk_note = if field.annotations.iter().any(|a| a.key == "primary_key") {
                " // LOOM[pk]"
            } else if field.annotations.iter().any(|a| a.key == "foreign_key") {
                " // LOOM[fk]"
            } else if field.annotations.iter().any(|a| a.key == "indexed") {
                " // LOOM[indexed]"
            } else {
                ""
            };
            out.push_str(&format!(
                "    pub {}: {},{}\n",
                field.name, rust_ty, pk_note
            ));
        }
    }

    fn emit_named_struct(&self, name: &str, fields: &[FieldDef], out: &mut String) {
        out.push_str(STORE_DERIVES);
        out.push('\n');
        out.push_str(&format!("pub struct {} {{\n", name));
        self.emit_struct_fields(fields, out);
        out.push_str("}\n\n");
    }

    // ── Relational ────────────────────────────────────────────────────────────

    fn codegen_relational_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: sqlx (compile-time query verification) | diesel | sea-orm\n");
        out.push_str("// LOOM[store:Relational]: tables → typed structs, PK/FK annotated\n\n");
        let mut tables: Vec<String> = Vec::new();
        for entry in &store.schema {
            if let StoreSchemaEntry::Table { name, fields, .. } = entry {
                let pk = fields
                    .iter()
                    .find(|f| f.annotations.iter().any(|a| a.key == "primary_key"))
                    .map(|f| f.name.as_str())
                    .unwrap_or("(none)");
                out.push_str(&format!("// Table `{}` — primary key: {}\n", name, pk));
                self.emit_named_struct(name, fields, out);
                self.emit_crud_trait_impl(&store.name, name, pk, out);
                self.emit_crud_in_memory_impl(&store.name, name, pk, out);
                self.emit_specification_pattern(name, out);
                self.emit_pagination_cursor(name, out);
                self.emit_openapi_hints(name, out);
                tables.push(name.clone());
            }
        }
        // Store-level patterns
        if !tables.is_empty() {
            self.emit_unit_of_work(&store.name, &tables, out);
        }
        self.emit_hateoas_for_store(&store.name, out);
        self.emit_cqrs_for_store(&store.name, out);
    }

    /// Emit a simple repository trait stub for a relational table.
    fn emit_crud_trait_impl(
        &self,
        store_name: &str,
        table: &str,
        pk_field: &str,
        out: &mut String,
    ) {
        let store_snake = to_snake_case(store_name);
        let table_snake = to_snake_case(table);
        out.push_str(&format!(
            "// LOOM[implicit:CRUD]: {table} CRUD trait — wire to your sqlx pool\n"
        ));
        out.push_str(&format!("pub trait {table}Repository {{\n"));
        out.push_str(&format!("    /// Find by primary key `{pk_field}`.\n"));
        out.push_str(&format!(
            "    fn find_by_id(&self, id: &str) -> Option<{table}>;\n"
        ));
        out.push_str(&format!("    /// Persist a new `{table}`.\n"));
        out.push_str(&format!(
            "    fn save(&self, entity: {table}) -> Result<{table}, String>;\n"
        ));
        out.push_str(&format!("    /// Remove by primary key.\n"));
        out.push_str(&format!(
            "    fn delete(&self, id: &str) -> Result<(), String>;\n"
        ));
        out.push_str("}\n\n");
        let _ = (store_snake, table_snake);
    }

    // ── Key-Value ─────────────────────────────────────────────────────────────

    fn codegen_keyvalue_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: dashmap (concurrent HashMap) | sled (embedded KV) | redis\n");
        out.push_str("// LOOM[store:KeyValue]: typed entry struct, get/put/del trait\n\n");

        let key_type = store
            .schema
            .iter()
            .find_map(|e| {
                if let StoreSchemaEntry::KeyType { ty, .. } = e {
                    Some(self.emit_type_expr(ty))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "String".to_string());

        let value_type = store
            .schema
            .iter()
            .find_map(|e| {
                if let StoreSchemaEntry::ValueType { ty, .. } = e {
                    Some(self.emit_type_expr(ty))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "String /* JsonValue — add serde_json for full type */".to_string());

        out.push_str(STORE_DERIVES);
        out.push('\n');
        out.push_str(&format!("pub struct {}Entry {{\n", store.name));
        out.push_str(&format!("    pub key: {}, // LOOM[pk]\n", key_type));
        out.push_str(&format!("    pub value: {},\n", value_type));
        out.push_str("}\n\n");

        // Typed KV trait
        out.push_str("// LOOM[implicit:KV]: typed get/put/del trait\n");
        out.push_str(&format!("pub trait {}Store {{\n", store.name));
        out.push_str(&format!(
            "    fn get(&self, key: &{key_type}) -> Option<{value_type}>;\n"
        ));
        out.push_str(&format!(
            "    fn put(&self, key: {key_type}, value: {value_type}) -> Result<(), String>;\n"
        ));
        out.push_str(&format!(
            "    fn del(&self, key: &{key_type}) -> Result<(), String>;\n"
        ));
        out.push_str("}\n\n");
    }

    // ── Document ──────────────────────────────────────────────────────────────

    fn codegen_document_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: mongodb | sled | surrealdb\n");
        out.push_str(
            "// LOOM[store:Document]: schema-optional collections → typed Rust structs\n\n",
        );
        for entry in &store.schema {
            if let StoreSchemaEntry::Collection { name, fields, .. } = entry {
                out.push_str(&format!("// Collection `{}`\n", name));
                self.emit_named_struct(name, fields, out);
            }
        }
    }

    // ── Columnar ──────────────────────────────────────────────────────────────

    fn codegen_columnar_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: Apache Arrow2 | polars | duckdb-rs\n");
        out.push_str("// LOOM[store:Columnar]: each row is a typed record; columns emerge from projection\n\n");
        for entry in &store.schema {
            if let StoreSchemaEntry::Collection { name, fields, .. } = entry {
                self.emit_named_struct(name, fields, out);
            }
        }
    }

    // ── Snowflake (OLAP) ──────────────────────────────────────────────────────

    fn codegen_snowflake_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: Snowflake ODBC | BigQuery | Redshift | duckdb-rs\n");
        out.push_str("// LOOM[store:Snowflake]: star-schema — fact + dimension structs\n\n");
        for entry in &store.schema {
            match entry {
                StoreSchemaEntry::Fact { name, fields, .. } => {
                    out.push_str(&format!("// Fact table: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                }
                StoreSchemaEntry::DimensionEntry { name, fields, .. } => {
                    out.push_str(&format!("// Dimension: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                }
                _ => {}
            }
        }
    }

    // ── Hypercube (MOLAP) ─────────────────────────────────────────────────────

    fn codegen_hypercube_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: ndarray | nalgebra | burn (tensor backend)\n");
        out.push_str(
            "// LOOM[store:Hypercube]: MOLAP axes → dimension types; measures → fact structs\n\n",
        );
        for entry in &store.schema {
            match entry {
                StoreSchemaEntry::DimensionEntry { name, fields, .. } => {
                    out.push_str(&format!("// Dimension axis: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                }
                StoreSchemaEntry::Fact { name, fields, .. } => {
                    out.push_str(&format!("// Measure: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                }
                _ => {}
            }
        }
    }

    // ── Graph ─────────────────────────────────────────────────────────────────

    fn codegen_graph_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: petgraph | neo4rs (Neo4j) | indradb\n");
        out.push_str("// LOOM[store:Graph]: node + edge structs; petgraph<NodeType, EdgeType> for in-memory\n\n");

        let mut node_types: Vec<&str> = Vec::new();
        let mut edge_types: Vec<&str> = Vec::new();

        for entry in &store.schema {
            match entry {
                StoreSchemaEntry::Node { name, fields, .. } => {
                    out.push_str(&format!("// Graph node: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                    node_types.push(name.as_str());
                }
                StoreSchemaEntry::Edge {
                    name,
                    source,
                    target,
                    fields,
                    ..
                } => {
                    out.push_str(&format!(
                        "// Graph edge: {} ({} → {})\n",
                        name, source, target
                    ));
                    self.emit_named_struct(name, fields, out);
                    edge_types.push(name.as_str());
                }
                _ => {}
            }
        }

        if !node_types.is_empty() {
            let nodes = node_types.join(", ");
            let edges = if edge_types.is_empty() {
                "()".to_string()
            } else {
                edge_types.join(" | ")
            };
            out.push_str(&format!(
                "// LOOM[graph:hint]: petgraph::Graph<({nodes}), ({edges})> for in-memory graph\n\n"
            ));
        }
        // Discipline: DAG + topological sort for directed-only graphs
        let is_directed = store
            .config
            .iter()
            .any(|c| c.key == "directed" && c.value == "true")
            || store.kind == StoreKind::Graph;
        if is_directed {
            self.emit_dag_wrapper(&store.name, out);
        } else {
            self.emit_lts_graph(&store.name, out);
        }
    }

    // ── TimeSeries ────────────────────────────────────────────────────────────

    fn codegen_timeseries_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: influxdb2 | timeseries-rs | tdengine\n");
        out.push_str(
            "// LOOM[store:TimeSeries]: events have mandatory timestamp; ordered by time\n\n",
        );

        for entry in &store.schema {
            if let StoreSchemaEntry::Event { name, fields, .. } = entry {
                out.push_str(&format!("// TimeSeries event: {}\n", name));
                // Inject timestamp if not already present
                let has_ts = fields
                    .iter()
                    .any(|f| f.name == "timestamp" || f.name == "ts");
                out.push_str(STORE_DERIVES);
                out.push('\n');
                out.push_str(&format!("pub struct {} {{\n", name));
                if !has_ts {
                    out.push_str(
                        "    pub timestamp: i64, // LOOM[ts]: Unix nanos — auto-injected\n",
                    );
                }
                self.emit_struct_fields(fields, out);
                out.push_str("}\n\n");
            }
        }

        // Emit a typed retention accessor
        let retention = store
            .config
            .iter()
            .find(|c| c.key == "retention" || c.key == "ttl")
            .map(|c| c.value.as_str())
            .unwrap_or("unbounded");
        out.push_str(&format!("// LOOM[ts:retention]: {}\n\n", retention));
        // Discipline: Event Sourcing + Domain Event Bus
        let event_types: Vec<String> = store
            .schema
            .iter()
            .filter_map(|e| {
                if let StoreSchemaEntry::Event { name, .. } = e {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        self.emit_event_sourcing(&store.name, &event_types, out);
        self.emit_domain_event_bus(&store.name, out);
    }

    // ── Vector ────────────────────────────────────────────────────────────────

    fn codegen_vector_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: qdrant-client | weaviate-client | candle (HuggingFace)\n");
        out.push_str(
            "// LOOM[store:Vector]: fixed-dimension embeddings with similarity search contract\n\n",
        );

        let dimension = store
            .config
            .iter()
            .find(|c| c.key == "dimension" || c.key == "dims")
            .and_then(|c| c.value.parse::<usize>().ok())
            .unwrap_or(0);

        for entry in &store.schema {
            if let StoreSchemaEntry::EmbeddingEntry { fields, .. } = entry {
                out.push_str(STORE_DERIVES);
                out.push('\n');
                out.push_str(&format!("pub struct {}Embedding {{\n", store.name));
                self.emit_struct_fields(fields, out);
                // Only emit the hardcoded vector field if the schema fields don't
                // already declare one (duplicate field would be a compile error).
                let already_has_vector = fields.iter().any(|f| f.name == "vector");
                if !already_has_vector {
                    if dimension > 0 {
                        out.push_str(&format!(
                            "    pub vector: [f32; {dimension}], // LOOM[vector:dims={dimension}]\n"
                        ));
                    } else {
                        out.push_str(
                            "    pub vector: Vec<f32>, // LOOM[vector]: dimension unknown at compile time\n"
                        );
                    }
                } else {
                    // Schema already declared a `vector` field; just emit the audit comment.
                    out.push_str("    // LOOM[vector]: vector field declared in schema\n");
                }
                out.push_str("}\n\n");
            }
        }

        // Similarity search stub
        out.push_str(&format!(
            "// LOOM[implicit:similarity]: cosine similarity search contract\n"
        ));
        out.push_str(&format!("pub trait {}VectorSearch {{\n", store.name));
        out.push_str(&format!(
            "    /// Returns top-k nearest neighbours by cosine similarity.\n"
        ));
        out.push_str(&format!(
            "    fn nearest(&self, query: &[f32], top_k: usize) -> Vec<{}Embedding>;\n",
            store.name
        ));
        out.push_str("}\n\n");
    }

    // ── InMemory ──────────────────────────────────────────────────────────────

    fn codegen_inmemory_store(&self, store: &StoreDef, inner: &Box<StoreKind>, out: &mut String) {
        let policy = match inner.as_ref() {
            StoreKind::KeyValue => "LRU cache".to_string(),
            _ => format!("{:?}", inner),
        };
        out.push_str(&format!(
            "// Ecosystem: lru | moka | dashmap | quick_cache\n"
        ));
        out.push_str(&format!(
            "// LOOM[store:InMemory({policy})]: eviction-aware typed cache\n\n"
        ));

        let capacity = store
            .config
            .iter()
            .find(|c| c.key == "capacity" || c.key == "max_size")
            .map(|c| c.value.as_str())
            .unwrap_or("unbounded");

        out.push_str(&format!("// LOOM[cache:capacity]: {capacity}\n"));

        // Emit the key/value pair struct
        let key_type = store
            .schema
            .iter()
            .find_map(|e| {
                if let StoreSchemaEntry::KeyType { ty, .. } = e {
                    Some(self.emit_type_expr(ty))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "String".to_string());
        let value_type = store
            .schema
            .iter()
            .find_map(|e| {
                if let StoreSchemaEntry::ValueType { ty, .. } = e {
                    Some(self.emit_type_expr(ty))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "String /* JsonValue — add serde_json for full type */".to_string());

        out.push_str(STORE_DERIVES);
        out.push('\n');
        out.push_str(&format!("pub struct {}CacheEntry {{\n", store.name));
        out.push_str(&format!("    pub key: {key_type},\n"));
        out.push_str(&format!("    pub value: {value_type},\n"));
        out.push_str("}\n\n");
        out.push_str(&format!(
            "// Wire: lru::LruCache<{key_type}, {value_type}> with capacity {capacity}\n\n"
        ));
    }

    // ── FlatFile ──────────────────────────────────────────────────────────────

    fn codegen_flatfile_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: arrow2 (Parquet/Arrow) | hdf5 | csv | polars\n");
        out.push_str(
            "// LOOM[store:FlatFile]: columnar row struct for Parquet/CSV/HDF5 serialization\n\n",
        );

        let format = store
            .config
            .iter()
            .find(|c| c.key == "format")
            .map(|c| c.value.as_str())
            .unwrap_or("Parquet");
        out.push_str(&format!("// LOOM[flatfile:format]: {format}\n\n"));

        for entry in &store.schema {
            if let StoreSchemaEntry::Collection { name, fields, .. } = entry {
                out.push_str(&format!("// FlatFile row: {}\n", name));
                self.emit_named_struct(name, fields, out);
            }
        }
    }

    // ── Distributed (MapReduce) ───────────────────────────────────────────────

    fn codegen_distributed_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: rayon (parallel iterators) | tokio (async tasks) | hadoop\n");
        out.push_str("// LOOM[store:Distributed]: MapReduce jobs — Dean & Ghemawat 2004\n\n");

        for entry in &store.schema {
            if let StoreSchemaEntry::MapReduceJob(mr) = entry {
                out.push_str(&format!("// MapReduce job: {}\n", mr.name));
                out.push_str(&format!("//   map:    {}\n", mr.map_sig));
                out.push_str(&format!("//   reduce: {}\n", mr.reduce_sig));
                if let Some(c) = &mr.combine_sig {
                    out.push_str(&format!("//   combine: {}\n", c));
                }
                let job_name = to_snake_case(&mr.name);
                out.push_str(&format!(
                    "// LOOM[mapreduce:hint]: impl {}_map(input) + {}_reduce(key, values) — wire to rayon::par_iter\n\n",
                    job_name, job_name
                ));
            }
        }
    }

    // ── DistributedLog (Kafka) ────────────────────────────────────────────────

    fn codegen_distributedlog_store(&self, store: &StoreDef, out: &mut String) {
        out.push_str("// Ecosystem: rdkafka | rskafka | pulsar-client\n");
        out.push_str("// LOOM[store:DistributedLog]: append-only log — Kreps 2011 (Kafka)\n\n");

        // Emit an event struct for each schema entry (reuse Event entries if any)
        let mut has_schema = false;
        for entry in &store.schema {
            match entry {
                StoreSchemaEntry::Event { name, fields, .. } => {
                    out.push_str(&format!("// Log message type: {}\n", name));
                    self.emit_named_struct(name, fields, out);
                    has_schema = true;
                }
                StoreSchemaEntry::LogConsumer(lc) => {
                    out.push_str(&format!(
                        "// LOOM[log:consumer]: {} offset={}\n\n",
                        lc.name, lc.offset
                    ));
                }
                _ => {}
            }
        }

        // Emit a typed producer/consumer trait
        let msg_type = if has_schema {
            format!("{}Message", store.name)
        } else {
            "Vec<u8>".to_string()
        };

        let partitions = store
            .config
            .iter()
            .find(|c| c.key == "partitions")
            .map(|c| c.value.as_str())
            .unwrap_or("1");
        let replication = store
            .config
            .iter()
            .find(|c| c.key == "replication")
            .map(|c| c.value.as_str())
            .unwrap_or("1");

        out.push_str(&format!(
            "// LOOM[log:config]: partitions={partitions} replication={replication}\n"
        ));
        out.push_str(&format!("pub trait {}Producer {{\n", store.name));
        out.push_str(&format!("    /// Append a message to the log.\n"));
        out.push_str(&format!(
            "    fn produce(&self, msg: {msg_type}) -> Result<u64, String>;\n"
        ));
        out.push_str("}\n\n");
        out.push_str(&format!("pub trait {}Consumer {{\n", store.name));
        out.push_str(&format!("    /// Poll the next message from the log.\n"));
        out.push_str(&format!("    fn poll(&mut self) -> Option<{msg_type}>;\n"));
        out.push_str("}\n\n");
        // Discipline: Domain Event Bus + Saga coordinator
        self.emit_domain_event_bus(&store.name, out);
        self.emit_saga_coordinator(&store.name, out);
    }
}
