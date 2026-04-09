// M90: Finance stdlib — stochastic processes, portfolio theory, risk metrics,
// Black-Scholes pricing, and fixed-income analytics written in Loom.

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

#[test]
fn test_m90_finance_stdlib_parses() {
    let stdlib = loom::stdlib::FINANCE_STDLIB;
    let result = parse(stdlib);
    assert!(result.is_ok(), "finance_stdlib must parse cleanly: {:?}", result.err());
}

#[test]
fn test_m90_stdlib_has_key_functions() {
    let stdlib = loom::stdlib::FINANCE_STDLIB;
    assert!(stdlib.contains("gbm_next_price"),         "missing gbm_next_price");
    assert!(stdlib.contains("black_scholes_call"),     "missing black_scholes_call");
    assert!(stdlib.contains("sharpe_ratio"),           "missing sharpe_ratio");
    assert!(stdlib.contains("value_at_risk"),          "missing value_at_risk");
    assert!(stdlib.contains("conditional_value_at_risk"), "missing conditional_value_at_risk");
    assert!(stdlib.contains("bond_price"),             "missing bond_price");
    assert!(stdlib.contains("PriceHistory"),           "missing PriceHistory store");
    assert!(stdlib.contains("PortfolioPositions"),     "missing PortfolioPositions store");
}

#[test]
fn test_m90_refinement_types_for_finance() {
    let src = r#"
module Finance
  type Probability = Float where x >= 0.0 and x <= 1.0
  type Volatility  = Float where x >= 0.0
  type Price       = Float where x > 0.0
  type Weight      = Float where x >= 0.0 and x <= 1.0
end
"#;
    assert!(parse(src).is_ok(), "finance refinement types must parse: {:?}", parse(src).err());
}

#[test]
fn test_m90_timeseries_store_parses() {
    let src = r#"
module Portfolio
  store Returns :: TimeSeries
    key: String
    value: Float
    timestamp: String
  end
end
"#;
    assert!(parse(src).is_ok(), "TimeSeries store must parse: {:?}", parse(src).err());
}

#[test]
fn test_m90_relational_store_parses() {
    let src = r#"
module Portfolio
  store Positions :: Relational
    primary_key: id
    fields:
      id: Int
      ticker: String
      quantity: Float
      price: Float
    end
  end
end
"#;
    assert!(parse(src).is_ok(), "Relational store must parse: {:?}", parse(src).err());
}

#[test]
fn test_m90_conserved_annotation_on_pricing_fn() {
    let src = r#"
module Pricing
  fn option_price @conserved(NoArbitrage)
      :: Float -> Float -> Float -> Float -> Float -> Float
    require: spot > 0.0 and strike > 0.0 and sigma > 0.0
    ensure: result >= 0.0
  end
end
"#;
    assert!(parse(src).is_ok(), "@conserved(NoArbitrage) must parse: {:?}", parse(src).err());
}

#[test]
fn test_m90_document_store_parses() {
    let src = r#"
module Risk
  store RiskDoc :: Document
    fields:
      risk_id: String
      var_95: Float
      cvar_95: Float
    end
  end
end
"#;
    assert!(parse(src).is_ok(), "Document store must parse: {:?}", parse(src).err());
}
