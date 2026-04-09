//! Tests for M88: Stochastic Process Type System.
//!
//! Wiener (1923): rigorous Brownian motion.
//! Itô (1944): stochastic calculus; GBM paths are log-normal.
//! Ornstein & Uhlenbeck (1930): mean-reverting process.
//! Markov (1906): discrete state, memoryless processes.

use loom::compile;
use loom::parse;

// ── M88 parsing tests ─────────────────────────────────────────────────────────

#[test]
fn test_m88_geometric_brownian_parses() {
    let src = r#"
module Finance
  fn model_stock_price @probabilistic :: Float -> Float -> StochasticPath
    distribution:
      family: GeometricBrownian(drift: 0.05, volatility: 0.2)
    end
    process:
      kind: GeometricBrownian
      always_positive: true
      martingale: false
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "GeometricBrownian process block should parse: {:?}", result.err());
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        let proc = fd.stochastic_process.as_ref().expect("stochastic_process should be Some");
        assert_eq!(proc.kind, loom::ast::StochasticKind::GeometricBrownian);
        assert_eq!(proc.always_positive, Some(true));
        assert_eq!(proc.martingale, Some(false));
    }
}

#[test]
fn test_m88_ornstein_uhlenbeck_parses() {
    let src = r#"
module Rates
  fn model_spread @probabilistic :: Float -> Float -> StochasticPath
    distribution:
      family: Gaussian(mean: 0.0, std_dev: 0.1)
    end
    process:
      kind: OrnsteinUhlenbeck
      mean_reverting: true
      long_run_mean: 0.0
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "OrnsteinUhlenbeck process block should parse: {:?}", result.err());
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        let proc = fd.stochastic_process.as_ref().expect("stochastic_process should be Some");
        assert_eq!(proc.kind, loom::ast::StochasticKind::OrnsteinUhlenbeck);
        assert_eq!(proc.mean_reverting, Some(true));
        assert_eq!(proc.long_run_mean.as_deref(), Some("0"));
    }
}

#[test]
fn test_m88_poisson_process_parses() {
    let src = r#"
module Arrivals
  fn model_arrivals @probabilistic :: Float -> StochasticCount
    distribution:
      family: Poisson(lambda: 3.5)
    end
    process:
      kind: PoissonProcess
      rate: 3.5
      integer_valued: true
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "PoissonProcess block should parse: {:?}", result.err());
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        let proc = fd.stochastic_process.as_ref().expect("stochastic_process should be Some");
        assert_eq!(proc.kind, loom::ast::StochasticKind::PoissonProcess);
        assert_eq!(proc.integer_valued, Some(true));
    }
}

#[test]
fn test_m88_markov_chain_parses() {
    let src = r#"
module Weather
  fn model_weather @probabilistic :: Unit -> StochasticPath
    process:
      kind: MarkovChain
      states: [Sunny, Cloudy, Rainy]
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "MarkovChain process block should parse: {:?}", result.err());
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        let proc = fd.stochastic_process.as_ref().expect("stochastic_process should be Some");
        assert_eq!(proc.kind, loom::ast::StochasticKind::MarkovChain);
        assert_eq!(proc.states, vec!["Sunny", "Cloudy", "Rainy"]);
    }
}

#[test]
fn test_m88_gbm_gaussian_mismatch_rejected() {
    // GBM process with Gaussian distribution family is a type error:
    // GBM paths are log-normal, not Gaussian.
    // Itô's lemma: the log-return is Gaussian, not the price.
    let src = r#"
module Finance
  fn bad_model @probabilistic :: Float -> StochasticPath
    distribution:
      family: Gaussian(mean: 0.0, std_dev: 1.0)
    end
    process:
      kind: GeometricBrownian
      always_positive: true
    end
  end
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "GBM + Gaussian should be rejected by checker");
    let err_msg = format!("{:?}", result.err());
    assert!(
        err_msg.contains("GeometricBrownian") || err_msg.contains("log-normal"),
        "error should mention GeometricBrownian or log-normal: {}",
        err_msg
    );
}

#[test]
fn test_m88_process_and_distribution_together_parse() {
    // Full function with both distribution: and process: blocks.
    let src = r#"
module Finance
  fn model_stock_price @probabilistic @conserved(Value) :: Float -> Float -> StochasticPath
    distribution:
      family: GeometricBrownian(drift: 0.05, volatility: 0.2)
    end
    process:
      kind: GeometricBrownian
      always_positive: true
      martingale: false
    end
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "process: + distribution: together should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        assert!(fd.distribution.is_some(), "distribution block should be parsed");
        assert!(fd.stochastic_process.is_some(), "stochastic_process block should be parsed");
    }
}
