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
    assert!(parse(src).is_ok(), "graph store with @provenance should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "timeseries store with retention/resolution should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "vector store with HNSW index should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "InMemory store should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "FlatFile Parquet store should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "FlatFile HDF5 store should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "distributed MapReduce store should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "DistributedLog store should parse: {:?}", parse(src).err());
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
    assert!(parse(src).is_ok(), "polyglot multi-store module should parse: {:?}", parse(src).err());
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
    assert!(!errors.is_empty(), "Vector store without embedding should be rejected");
    let msgs: String = errors.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        msgs.contains("embedding") || msgs.contains("Vector"),
        "Expected embedding/Vector error, got: {}", msgs
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
    assert!(parse(src).is_ok(), "DistributedLog store should parse: {:?}", parse(src).err());
}
