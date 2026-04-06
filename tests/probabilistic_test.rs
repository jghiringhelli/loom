//! Tests for M60: Probabilistic Types

use loom::compile;

#[test]
fn probabilistic_with_distribution_ok() {
    let src = r#"
module Test
fn sample @probabilistic @pure :: Int -> Float
distribution:
  model: normal
  mean: 0
  variance: 1
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn probabilistic_without_distribution_errors() {
    let src = r#"
module Test
fn sample @probabilistic @pure :: Int -> Float
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("probabilistic") || msg.contains("distribution"), "expected probabilistic error in: {}", msg);
}

#[test]
fn distribution_convergence_non_numeric_errors() {
    let src = r#"
module Test
fn compute @probabilistic @pure :: Int -> String
distribution:
  model: mcmc
  convergence: 0
end
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("convergence") || msg.contains("numeric"), "expected convergence error in: {}", msg);
}

#[test]
fn distribution_convergence_float_ok() {
    let src = r#"
module Test
fn estimate @probabilistic @pure :: Int -> Float
distribution:
  model: mcmc
  convergence: 0
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn distribution_block_in_codegen() {
    let src = r#"
module Test
fn sample @probabilistic @pure :: Int -> Float
distribution:
  model: normal
  mean: 0
end
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("distribution") || rust_src.contains("normal"), "Expected distribution comment in rust output");
}

#[test]
fn distribution_block_without_annotation_ok() {
    let src = r#"
module Test
fn sample @pure :: Int -> Float
distribution:
  model: uniform
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}
