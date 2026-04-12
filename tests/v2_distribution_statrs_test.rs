//! V2 distribution → statrs tests.
//!
//! Gate: Gaussian distribution codegen emits:
//! 1. `{N}GaussianSampler` struct with `sample_box_muller` (always present)
//! 2. `#[cfg(feature = "loom_statrs")] fn sample_statrs` that calls `statrs::distribution::Normal::new()`

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── struct + box_muller always present ───────────────────────────────────────

#[test]
fn gaussian_emits_sampler_struct() {
    let src = r#"
module Stats
  fn temperature_model @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 20.0, std_dev: 3.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("GaussianSampler"),
        "expected GaussianSampler struct, got:\n{}",
        out
    );
    assert!(out.contains("mean: f64"), "expected mean field");
    assert!(out.contains("std_dev: f64"), "expected std_dev field");
}

#[test]
fn gaussian_emits_box_muller_method() {
    let src = r#"
module Stats
  fn noise_model @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 0.0, std_dev: 1.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("sample_box_muller"),
        "expected sample_box_muller method, got:\n{}",
        out
    );
    assert!(
        out.contains("z1: f64, z2: f64"),
        "expected z1/z2 params in Box-Muller"
    );
}

#[test]
fn gaussian_default_mean_and_std_in_new() {
    let src = r#"
module Stats
  fn sensor_reading @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 100.0, std_dev: 5.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("100.0"), "expected mean 100.0");
    assert!(out.contains("5.0"), "expected std_dev 5.0");
}

// ── statrs block emitted under cfg ───────────────────────────────────────────

#[test]
fn gaussian_emits_statrs_cfg_block() {
    let src = r#"
module Stats
  fn price_model @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 50.0, std_dev: 2.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains(r#"#[cfg(feature = "loom_statrs")]"#),
        "expected #[cfg(feature = \"loom_statrs\")] block, got:\n{}",
        out
    );
    assert!(
        out.contains("sample_statrs"),
        "expected sample_statrs method, got:\n{}",
        out
    );
}

#[test]
fn gaussian_statrs_block_calls_normal_new() {
    let src = r#"
module Stats
  fn weather @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 15.0, std_dev: 4.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("Normal::new"),
        "statrs block must call Normal::new(self.mean, self.std_dev), got:\n{}",
        out
    );
}

#[test]
fn gaussian_statrs_block_uses_rng_generic() {
    let src = r#"
module Stats
  fn signal_flow @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 0.0, std_dev: 1.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("impl rand::Rng") || out.contains("R: rand::Rng"),
        "statrs method must accept generic Rng, got:\n{}",
        out
    );
}

// ── ecosystem comment ─────────────────────────────────────────────────────────

#[test]
fn gaussian_audit_comment_mentions_statrs() {
    let src = r#"
module Stats
  fn dist @probabilistic :: Float -> Float
    distribution:
      family: Gaussian(mean: 1.0, std_dev: 1.0)
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("statrs"),
        "audit comment should mention statrs ecosystem, got:\n{}",
        out
    );
}
