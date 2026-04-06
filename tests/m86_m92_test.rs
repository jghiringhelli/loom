// Tests for M86 (conservation annotations) and M92 (store declarations).

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

#[test]
fn test_m86_conserved_annotation_parses() {
    let src = r#"
module Chemistry
  fn react @conserved(Mass) :: Float -> Float
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "conserved annotation should parse: {:?}", result.err());
}

#[test]
fn test_m86_conserved_energy() {
    let src = r#"
module Physics
  fn collide @conserved(Energy) @conserved(Momentum) :: Float -> Float -> Float
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "multiple conserved annotations should parse: {:?}", result.err());
}

#[test]
fn test_m86_conserved_value_finance() {
    let src = r#"
module Finance
  fn arbitrage_free @conserved(Value) :: Float -> Float -> Float
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "conserved(Value) annotation should parse: {:?}", result.err());
}

#[test]
fn test_m92_relational_store() {
    let src = r#"
module App
  store UserStore :: Relational
    table Users
      id:    UUID   @primary_key
      email: String @unique
      name:  String
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "relational store should parse: {:?}", result.err());
}

#[test]
fn test_m92_keyvalue_store() {
    let src = r#"
module Cache
  store SessionCache :: KeyValue
    key:   SessionToken
    value: SessionData
    ttl:   Duration
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "keyvalue store should parse: {:?}", result.err());
}

#[test]
fn test_m92_graph_store() {
    let src = r#"
module Knowledge
  store KnowledgeGraph :: Graph
    node Person :: { name: String, dob: Date }
    node Compound :: { smiles: String, formula: String }
    edge BondsWith :: Compound -> Compound { bond_type: String, strength: Float }
    edge KnownBy :: Person -> Person { since: Date }
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "graph store should parse: {:?}", result.err());
}

#[test]
fn test_m92_timeseries_store() {
    let src = r#"
module Sensors
  store EventLog :: TimeSeries
    event SensorReading :: { sensor_id: String, value: Float, quality: Float }
    retention: 90days
    resolution: 1second
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "timeseries store should parse: {:?}", result.err());
}

#[test]
fn test_m92_vector_store() {
    let src = r#"
module Search
  store EmbeddingIndex :: Vector
    embedding :: { id: String, label: String }
    index: HNSW
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "vector store should parse: {:?}", result.err());
}

#[test]
fn test_m92_flatfile_store() {
    let src = r#"
module Science
  store Results :: FlatFile
    event ResultRow :: { timestamp: String, value: Float, label: String }
    format: Parquet
    compression: Zstd
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "flatfile store should parse: {:?}", result.err());
}

#[test]
fn test_m92_snowflake_store() {
    let src = r#"
module Analytics
  store SalesWarehouse :: Snowflake
    fact SalesFact :: { amount: Float, date: String, product_id: String }
    dimension Product :: { id: String, name: String, category: String }
    dimension Region :: { code: String, name: String }
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "snowflake store should parse: {:?}", result.err());
}

#[test]
fn test_m92_multiple_stores_in_module() {
    let src = r#"
module Polyglot
  store Users :: Relational
    table Person
      id: String @primary_key
    end
  end

  store Cache :: KeyValue
    key:   String
    value: String
  end

  store Events :: TimeSeries
    event Click :: { user_id: String, ts: String }
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "multiple stores should parse: {:?}", result.err());
}
