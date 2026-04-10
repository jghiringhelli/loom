// Tests for M95 (specialized stores), M96 (local stores), and M97 (distributed stores).

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// ── M95: Specialized stores ───────────────────────────────────────────────────

#[test]
fn test_m95_graph_store_with_provenance() {
    let src = r#"
module Chemistry
  store MoleculeGraph :: Graph
    node Atom :: { symbol: String, atomic_number: Int, charge: Int }
    node Molecule :: { formula: String, smiles: String, molar_mass: Float }
    edge Bond :: Atom -> Atom { bond_type: String, order: Float, source: String @provenance }
    edge Contains :: Molecule -> Atom { count: Int, position: Int }
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "graph store with @provenance should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m95_timeseries_with_retention() {
    let src = r#"
module Monitoring
  store Metrics :: TimeSeries
    event CpuReading :: { host: String, value: Float, timestamp: DateTime }
    event MemReading :: { host: String, used: Float, total: Float, ts: DateTime }
    retention: 90days
    resolution: 1minute
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "timeseries store with retention/resolution should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m95_vector_store_with_hnsw() {
    let src = r#"
module Embeddings
  store VectorDb :: Vector
    embedding :: { id: String, text: String, vector: Float }
    index: HNSW
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "vector store with HNSW index should parse: {:?}",
        parse(src).err()
    );
}

// ── M96: Local stores ─────────────────────────────────────────────────────────

#[test]
fn test_m96_inmemory_store() {
    let src = r#"
module FastPath
  store HotCache :: InMemory
    key:   String
    value: Bytes
    capacity: 1000
    eviction: LRU
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "InMemory store should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m96_flatfile_parquet() {
    let src = r#"
module Science
  store SimResults :: FlatFile
    event DataPoint :: { t: Float, x: Float, y: Float, label: String }
    format: Parquet
    compression: Zstd
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "FlatFile Parquet store should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m96_flatfile_hdf5_for_tensors() {
    let src = r#"
module NeuralData
  store Weights :: FlatFile
    event LayerWeight :: { layer: Int, row: Int, col: Int, value: Float }
    format: HDF5
    compression: LZ4
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "FlatFile HDF5 store should parse: {:?}",
        parse(src).err()
    );
}

// ── M97: Distributed stores ───────────────────────────────────────────────────

#[test]
fn test_m97_distributed_mapreduce() {
    let src = r#"
module BigData
  store WebCrawl :: Distributed
    schema CrawlRecord :: { url: String, content: String, timestamp: DateTime }
    mapreduce WordCount
      map:    CrawlRecord -> [(String, Int)]
      reduce: String -> [Int] -> (String, Int)
    end
    mapreduce LinkGraph
      map:    CrawlRecord -> [(String, String)]
      reduce: String -> [String] -> (String, Int)
      combine: String -> [String] -> String
    end
    partitions: 256
    replication: 3
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "distributed MapReduce store should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m97_distributed_log_kafka_style() {
    let src = r#"
module Streaming
  store EventStream :: DistributedLog
    event UserAction :: { user_id: String, event_type: String, ts: DateTime }
    event SystemAlert :: { service: String, severity: String, message: String }
    partitions: 32
    retention: 7days
    consumer AnalyticsPipeline :: offset: earliest
    consumer AlertingSystem    :: offset: latest
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "DistributedLog store should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m97_polyglot_all_store_kinds() {
    let src = r#"
module Platform
  store Users :: Relational
    table User
      id:    UUID @primary_key
      email: String @unique
    end
  end

  store Sessions :: KeyValue
    key:   String
    value: String
  end

  store Graph :: Graph
    node Entity :: { id: String, name: String }
    edge RelatesTo :: Entity -> Entity { weight: Float }
  end

  store Events :: DistributedLog
    event AppEvent :: { id: String, ts: DateTime }
    consumer Processor :: offset: earliest
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "polyglot multi-store module should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m95_vector_store_missing_embedding_rejected() {
    // Vector store without an embedding: entry must be rejected by StoreChecker
    let src = r#"
module Search
  store Index :: Vector
  end
end
"#;
    let module = parse(src).expect("parse failed");
    let errors = loom::checker::StoreChecker::new().check(&module);
    assert!(
        !errors.is_empty(),
        "Vector store without embedding should be rejected"
    );
    let msgs: String = errors
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msgs.contains("embedding") || msgs.contains("Vector"),
        "Expected embedding/Vector error, got: {}",
        msgs
    );
}

#[test]
fn test_m97_distributed_log_parses() {
    let src = r#"
module Streaming
  store EventLog :: DistributedLog
    event AppEvent :: { id: String, ts: String }
    partitions: 12
    replication: 3
    consumer Analytics :: offset: earliest
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "DistributedLog store should parse: {:?}",
        parse(src).err()
    );
}

// ── V5+V7: Store codegen emits complete Rust with audit trail ─────────────────

fn compile_to_rust(src: &str) -> Result<String, Vec<loom::LoomError>> {
    loom::compile(src)
}

#[test]
fn test_v5_graph_store_emits_node_edge_structs() {
    let src = r#"
module Chem
  store MolGraph :: Graph
    node Atom :: { symbol: String, number: Int }
    edge Bond :: Atom -> Atom { order: Float }
  end
end
"#;
    let rust = compile_to_rust(src).expect("V5 graph store should compile");
    assert!(
        rust.contains("pub struct Atom"),
        "should emit Atom node struct"
    );
    assert!(
        rust.contains("pub struct Bond"),
        "should emit Bond edge struct"
    );
    assert!(
        rust.contains("LOOM[store:Graph]"),
        "V7 audit trail for Graph"
    );
    assert!(rust.contains("petgraph"), "should recommend petgraph");
}

#[test]
fn test_v5_timeseries_store_injects_timestamp() {
    let src = r#"
module Monitor
  store Metrics :: TimeSeries
    event Cpu :: { host: String, value: Float }
  end
end
"#;
    let rust = compile_to_rust(src).expect("V5 timeseries store should compile");
    assert!(
        rust.contains("pub struct Cpu"),
        "should emit Cpu event struct"
    );
    assert!(rust.contains("timestamp"), "should inject timestamp field");
    assert!(
        rust.contains("LOOM[ts]"),
        "V7 audit: timestamp auto-injected"
    );
}

#[test]
fn test_v5_vector_store_emits_embedding_struct() {
    let src = r#"
module Search
  store EmbeddingDb :: Vector
    embedding :: { id: String, text: String, vector: Float }
    index: HNSW
  end
end
"#;
    let rust = compile_to_rust(src).expect("V5 vector store should compile");
    assert!(
        rust.contains("EmbeddingDbEmbedding"),
        "should emit typed embedding struct"
    );
    assert!(rust.contains("LOOM[vector"), "V7 audit for vector");
    assert!(
        rust.contains("nearest"),
        "should emit similarity search trait"
    );
}

#[test]
fn test_v5_distributedlog_emits_producer_consumer_traits() {
    let src = r#"
module Events
  store AppLog :: DistributedLog
    event Action :: { user: String, kind: String }
    partitions: 8
    replication: 3
  end
end
"#;
    let rust = compile_to_rust(src).expect("V5 DistributedLog store should compile");
    assert!(
        rust.contains("AppLogProducer"),
        "should emit typed Producer trait"
    );
    assert!(
        rust.contains("AppLogConsumer"),
        "should emit typed Consumer trait"
    );
    assert!(
        rust.contains("LOOM[store:DistributedLog]"),
        "V7 audit trail"
    );
    assert!(rust.contains("rdkafka"), "should recommend rdkafka");
}

#[test]
fn test_v7_audit_trail_in_contracts() {
    let src = r#"
module Contracts
fn safe_divide :: Int -> Int -> Int
  require: b != 0
  a / b
end
end
"#;
    let rust = compile_to_rust(src).expect("contract fn should compile");
    assert!(
        rust.contains("LOOM[require]"),
        "V7 audit: require contract annotated"
    );
    assert!(
        rust.contains("debug_assert!"),
        "precondition must emit debug_assert!"
    );
}

#[test]
fn test_v7_audit_header_in_all_emitted_files() {
    let src = r#"
module Anything
fn id :: Int -> Int
  x
end
end
"#;
    let rust = compile_to_rust(src).expect("trivial module should compile");
    assert!(
        rust.contains("LOOM[v7:audit]"),
        "every emitted file must have V7 audit header"
    );
}
