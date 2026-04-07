// Tests for M93 (operational stores: Relational, KeyValue, Document) and
// M94 (analytical stores: Columnar, Snowflake, Hypercube).

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// ── M93: Operational Stores ───────────────────────────────────────────────────

#[test]
fn test_m93_relational_multi_table() {
    let src = r#"
module Blog
  store BlogDb :: Relational
    table Posts
      id:      UUID   @primary_key
      title:   String @indexed
      content: String
      author:  UUID   @foreign_key(Users.id)
    end
    table Users
      id:    UUID   @primary_key
      email: String @unique
    end
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m93_keyvalue_with_ttl() {
    let src = r#"
module Session
  store Cache :: KeyValue
    key:   Token   @hashed
    value: Session
    ttl:   30days
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m93_document_dynamic_schema() {
    let src = r#"
module Config
  store ConfigStore :: Document
    schema AppConfig :: { name: String, value: Json, version: Int }
    schema FeatureFlag :: { key: String, enabled: Bool, rollout: Float }
  end
end
"#;
    assert!(parse(src).is_ok());
}

// ── M94: Analytical Stores ────────────────────────────────────────────────────

#[test]
fn test_m94_columnar_store() {
    let src = r#"
module Analytics
  store Metrics :: Columnar
    schema EventMetric :: { ts: DateTime, user_id: UUID, value: Float, region: String }
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m94_snowflake_store() {
    let src = r#"
module Warehouse
  store SalesWarehouse :: Snowflake
    fact SalesFact :: { amount: Float, date: Date, product_id: UUID, region_code: String }
    dimension Product :: { id: UUID, name: String, category: String }
    dimension Region :: { code: String, name: String, country: String }
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m94_hypercube_store() {
    let src = r#"
module OLAP
  store SalesCube :: Hypercube
    dimension Time :: { year: Int, quarter: Int, month: Int, day: Int }
    dimension Geography :: { country: String, region: String, city: String }
    dimension Product :: { category: String, subcategory: String, name: String }
    fact Revenue :: { amount: Float, units_sold: Int, discount: Float }
  end
end
"#;
    assert!(parse(src).is_ok());
}
