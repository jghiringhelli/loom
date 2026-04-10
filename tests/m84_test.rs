//! Tests for M84: Full parametric distribution type system.

use loom::parse;

#[test]
fn test_m84_gaussian_family_parses() {
    let src = r#"
module Stats
  fn sample_height @probabilistic :: Unit -> Float
    distribution:
      family: Gaussian(mean: 170.0, std_dev: 10.0)
      bounds: [100.0, 250.0]
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "Gaussian family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_poisson_family_parses() {
    let src = r#"
module Events
  fn arrival_count @probabilistic :: TimeWindow -> Int
    distribution:
      family: Poisson(lambda: 3.5)
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "Poisson family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_beta_family_parses() {
    let src = r#"
module Bayesian
  fn click_rate @probabilistic :: Campaign -> Float
    distribution:
      family: Beta(alpha: 2.0, beta: 5.0)
      bounds: [0.0, 1.0]
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "Beta family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_cauchy_no_clt() {
    // Cauchy + CLT convergence claim should produce a type error
    let src = r#"
module Finance
  fn fat_tail @probabilistic :: Unit -> Float
    distribution:
      family: Cauchy(location: 0.0, scale: 1.0)
      convergence: central_limit_theorem
    end
  end
end
"#;
    // Should parse but fail the checker
    let result = parse(src);
    assert!(result.is_ok(), "Cauchy should parse: {:?}", result.err());
    // The check will produce an error — verify via compile function if available
    // For now just verify parsing succeeds
}

#[test]
fn test_m84_lognormal_family_parses() {
    let src = r#"
module Finance
  fn asset_price @probabilistic :: Unit -> Float
    distribution:
      family: LogNormal(mean: 0.05, std_dev: 0.2)
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "LogNormal family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_uniform_family_parses() {
    let src = r#"
module Simulation
  fn random_position @probabilistic :: Unit -> Float
    distribution:
      family: Uniform(low: 0.0, high: 100.0)
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "Uniform family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_backward_compat_model_string() {
    // Old M60 syntax with model: string must still work
    let src = r#"
module Legacy
  fn old_style @probabilistic :: Unit -> Float
    distribution:
      model: gaussian
      mean: 0.0
      variance: 1.0
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "Old model: string syntax must still parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_geometric_brownian_motion() {
    let src = r#"
module Finance
  fn price_path @probabilistic :: Float -> Float
    distribution:
      family: GeometricBrownian(drift: 0.05, volatility: 0.2)
    end
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "GeometricBrownian family should parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m84_cauchy_clt_claim_rejected_by_checker() {
    // Cauchy + CLT convergence: checker must reject (Cauchy has no finite mean/variance)
    let src = r#"
module Finance
  fn fat_tail @probabilistic :: Int -> Float
    distribution:
      family: Cauchy(location: 0.0, scale: 1.0)
      convergence: central_limit_theorem
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "Cauchy + CLT convergence should be rejected by checker"
    );
    let msgs = result
        .unwrap_err()
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msgs.contains("Cauchy")
            || msgs.contains("central_limit")
            || msgs.contains("mean")
            || msgs.contains("variance"),
        "Expected Cauchy/CLT error, got: {}",
        msgs
    );
}

#[test]
fn test_m84_gaussian_negative_std_dev_rejected() {
    // std_dev = 0.0 violates the must-be-positive constraint (negative literals
    // don't parse in distribution params — use zero as the boundary-value test)
    let src = r#"
module Stats
  fn bad_dist @probabilistic :: Int -> Float
    distribution:
      family: Gaussian(mean: 0.0, std_dev: 0.0)
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "std_dev=0.0 should be rejected (must be > 0)"
    );
    let msgs = result
        .unwrap_err()
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msgs.contains("std_dev") || msgs.contains("Gaussian"),
        "Expected std_dev/Gaussian error, got: {}",
        msgs
    );
}
