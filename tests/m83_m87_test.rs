// M83: Dimensional sense stdlib — full SI + extended ontology.
// M87: Tensor types — Tensor<rank, shape, unit>.

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// ── M83: Sense stdlib tests ───────────────────────────────────────────────────

#[test]
fn test_m83_sense_stdlib_parses() {
    let stdlib = loom::stdlib::SENSE_STDLIB;
    let result = parse(stdlib);
    assert!(result.is_ok(), "sense_stdlib must parse: {:?}", result.err());
}

#[test]
fn test_m83_sense_stdlib_has_seven_si_bases() {
    let stdlib = loom::stdlib::SENSE_STDLIB;
    assert!(stdlib.contains("sense Length"));
    assert!(stdlib.contains("sense Mass"));
    assert!(stdlib.contains("sense Time"));
    assert!(stdlib.contains("sense ElectricCurrent"));
    assert!(stdlib.contains("sense Temperature"));
    assert!(stdlib.contains("sense AmountOfSubstance"));
    assert!(stdlib.contains("sense LuminousIntensity"));
}

#[test]
fn test_m83_sense_stdlib_has_extended_dimensions() {
    let stdlib = loom::stdlib::SENSE_STDLIB;
    assert!(stdlib.contains("sense PlaneAngle"));
    assert!(stdlib.contains("sense Information"));
    assert!(stdlib.contains("sense QuantumSpin"));
    assert!(stdlib.contains("sense GravitationalWaveStrain"));
    assert!(stdlib.contains("sense EntanglementCorrelation"));
}

#[test]
fn test_m83_sense_dimension_field_parses() {
    let src = r#"
module SIDimensions
  sense Length
    channels: [Meter, Kilometer]
    unit: "m"
    dimension: L
  end

  sense Mass
    channels: [Kilogram, Gram]
    unit: "kg"
    dimension: M
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "sense with dimension field must parse: {:?}", result.err());
}

#[test]
fn test_m83_sense_derived_field_parses() {
    let src = r#"
module DerivedUnits
  sense Force
    channels: [Newton, Kilonewton]
    unit: "N"
    derived: M_L_T_neg2
  end

  sense Pressure
    channels: [Pascal, Bar]
    unit: "Pa"
    derived: M_L_neg1_T_neg2
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "sense with derived field must parse: {:?}", result.err());
}

#[test]
fn test_m83_user_sense_uses_stdlib_names() {
    let src = r#"
module PhysicsAgent
  sense CustomPressure
    channels: [LowPressure, HighPressure, CriticalPressure]
    unit: "Pa"
  end

  being PressureMonitor
    telos: "maintain safe pressure"
    end
    umwelt:
      detects: [LowPressure, HighPressure]
    end
  end
end
"#;
    assert!(parse(src).is_ok());
}

// ── M87: Tensor type tests ────────────────────────────────────────────────────

#[test]
fn test_m87_tensor_type_vector() {
    let src = r#"
module Physics
  type VelocityVector = Tensor<1, [3], Float>
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_type_matrix() {
    let src = r#"
module LinearAlgebra
  type CovarianceMatrix = Tensor<2, [N, N], Float>
  type StressTensor = Tensor<2, [3, 3], Float>
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_type_scalar() {
    let src = r#"
module Fields
  type ScalarField = Tensor<0, [], Float>
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_in_function_signature() {
    let src = r#"
module ML
  fn matrix_multiply :: Tensor<2, [M, N], Float> -> Tensor<2, [N, K], Float> -> Tensor<2, [M, K], Float>
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_in_vector_store() {
    let src = r#"
module Embeddings
  store VecDb :: Vector
    embedding :: { id: String, vector: Tensor<1, [1536], Float> }
    index: HNSW
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_wavefunction() {
    let src = r#"
module Quantum
  type Ket = Tensor<1, [N], Float>
  type DensityMatrix = Tensor<2, [N, N], Float>

  fn evolve :: Ket -> Tensor<2, [N, N], Float> -> Ket
  end
end
"#;
    assert!(parse(src).is_ok());
}

#[test]
fn test_m87_tensor_rank_shape_mismatch_rejected() {
    // Tensor<2, [3, 3, 3], Float> — rank 2 but shape has 3 elements → checker error
    let src = r#"
module Physics
  fn bad_matrix :: Int -> Tensor<2, [3, 3, 3], Float>
  end
end
"#;
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    let result = loom::checker::TensorChecker::new().check(&module);
    assert!(result.is_err(), "Tensor rank/shape mismatch should be rejected by TensorChecker");
    let msgs = result.unwrap_err().iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        msgs.contains("rank") || msgs.contains("shape"),
        "Expected rank/shape error, got: {}", msgs
    );
}

#[test]
fn test_m83_sense_stdlib_dimensions_are_accessible() {
    let stdlib = loom::stdlib::SENSE_STDLIB;
    assert!(stdlib.contains("Length"));
    assert!(stdlib.contains("Mass"));
    assert!(stdlib.contains("Temperature"));
    assert!(stdlib.contains("GravitationalWaveStrain"));
}
