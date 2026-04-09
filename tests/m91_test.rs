// M91: Quantum mechanics stdlib — state vectors, gates, measurement,
// uncertainty, Schrödinger evolution, and quantum information written in Loom.

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

#[test]
fn test_m91_quantum_stdlib_parses() {
    let stdlib = loom::stdlib::QUANTUM_STDLIB;
    let result = parse(stdlib);
    assert!(
        result.is_ok(),
        "quantum_stdlib must parse cleanly: {:?}",
        result.err()
    );
}

#[test]
fn test_m91_stdlib_has_key_functions() {
    let stdlib = loom::stdlib::QUANTUM_STDLIB;
    assert!(
        stdlib.contains("born_probability"),
        "missing born_probability"
    );
    assert!(stdlib.contains("hadamard_alpha"), "missing hadamard_alpha");
    assert!(stdlib.contains("pauli_x_alpha"), "missing pauli_x_alpha");
    assert!(stdlib.contains("phase_gate"), "missing phase_gate");
    assert!(
        stdlib.contains("cnot_target_alpha"),
        "missing cnot_target_alpha"
    );
    assert!(
        stdlib.contains("heisenberg_satisfied"),
        "missing heisenberg_satisfied"
    );
    assert!(
        stdlib.contains("von_neumann_entropy"),
        "missing von_neumann_entropy"
    );
    assert!(stdlib.contains("fidelity"), "missing fidelity");
    assert!(
        stdlib.contains("QuantumRegister"),
        "missing QuantumRegister store"
    );
}

#[test]
fn test_m91_refinement_types_for_quantum() {
    let src = r#"
module Quantum
  type BornProb  = Float where x >= 0.0 and x <= 1.0
  type Phase     = Float where x >= 0.0 and x <= 6.283185307
  type NumQubits = Int  where x > 0
end
"#;
    assert!(
        parse(src).is_ok(),
        "quantum refinement types must parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m91_keyvalue_store_for_register() {
    let src = r#"
module Quantum
  store Register :: KeyValue
    key: Int
    value: Float
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "KeyValue store must parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m91_conserved_unitarity_annotation() {
    let src = r#"
module Gates
  fn hadamard @conserved(Unitarity)
      :: Float -> Float -> Float
    require: true
    ensure: true
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "@conserved(Unitarity) must parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m91_conserved_probability_annotation() {
    let src = r#"
module Measurement
  fn collapse @conserved(Probability)
      :: Int -> Int -> Float
    require: basis_state >= 0 and num_basis_states > 0
    ensure: result >= 0.0 and result <= 1.0
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "@conserved(Probability) must parse: {:?}",
        parse(src).err()
    );
}

#[test]
fn test_m91_conserved_energy_annotation() {
    let src = r#"
module Schrodinger
  fn energy_level @conserved(Energy)
      :: Int -> Float -> Float
    require: n >= 1 and omega > 0.0
    ensure: result > 0.0
  end
end
"#;
    assert!(
        parse(src).is_ok(),
        "@conserved(Energy) must parse: {:?}",
        parse(src).err()
    );
}
