//! M53 tests — NeuroML 2 XML emitter for neural beings with plasticity: blocks.

use loom::codegen::NeuroMLEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── Loom source fixtures ──────────────────────────────────────────────────────

const PLASTIC_BEING_SRC: &str = r#"module NeuralNet
being Neuron
  describe: "a spiking neuron"
  telos: "learn from experience"
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
end
"#;

const NON_PLASTIC_BEING_SRC: &str = r#"module NeuralNet
being Glial
  telos: "support neuron metabolism"
  end
end
end
"#;

const BOLTZMANN_SRC: &str = r#"module NeuralNet
being BoltzNeuron
  telos: "equilibrate energy"
  end
  plasticity:
    trigger: EnergySignal
    modifies: NetworkWeight
    rule: boltzmann
  end
end
end
"#;

const REGULATE_SRC: &str = r#"module NeuralNet
being Neuron
  telos: "maintain homeostasis"
  end
  regulate voltage
    target: resting
    bounds: (min_v, max_v)
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
end
"#;

const MORPHOGEN_SRC: &str = r#"module NeuralNet
being Neuron
  telos: "differentiate spatially"
  end
  morphogen:
    signal: Wnt
    threshold: 0.8
    produces: [DendriticCell]
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
end
"#;

const ECOSYSTEM_SRC: &str = r#"module NeuralNet
being Neuron
  telos: "fire toward signal propagation"
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
ecosystem Brain
  describe: "neural computation network"
  members: [Neuron]
  telos: "emergent cognition"
end
end
"#;

const ECOSYSTEM_SIGNAL_SRC: &str = r#"module NeuralNet
being Neuron
  telos: "fire toward signal propagation"
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
ecosystem Brain
  describe: "neural computation network"
  members: [Neuron]
  signal Spike from Neuron to Neuron
    payload: Float
  end
  telos: "emergent cognition"
end
end
"#;

// ── 1. being_with_plasticity_emits_neuroml_cell ───────────────────────────────

#[test]
fn being_with_plasticity_emits_neuroml_cell() {
    let module = parse(PLASTIC_BEING_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("<cell id=\"Neuron\""),
        "expected <cell id=\"Neuron\"> in:\n{out}"
    );
}

// ── 2. being_without_plasticity_not_emitted ───────────────────────────────────

#[test]
fn being_without_plasticity_not_emitted() {
    let module = parse(NON_PLASTIC_BEING_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        !out.contains("<cell"),
        "expected no <cell> element for non-plastic being in:\n{out}"
    );
}

// ── 3. hebbian_rule_emits_synapse ─────────────────────────────────────────────

#[test]
fn hebbian_rule_emits_synapse() {
    let module = parse(PLASTIC_BEING_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("rule=\"hebbian\""),
        "expected rule=\"hebbian\" in:\n{out}"
    );
    assert!(
        out.contains("<synapse"),
        "expected <synapse element in:\n{out}"
    );
}

// ── 4. boltzmann_rule_emits_synapse ───────────────────────────────────────────

#[test]
fn boltzmann_rule_emits_synapse() {
    let module = parse(BOLTZMANN_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("rule=\"boltzmann\""),
        "expected rule=\"boltzmann\" in:\n{out}"
    );
    assert!(
        out.contains("<synapse"),
        "expected <synapse element in:\n{out}"
    );
}

// ── 5. regulate_block_emits_biophysical_properties ───────────────────────────

#[test]
fn regulate_block_emits_biophysical_properties() {
    let module = parse(REGULATE_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("<biophysicalProperties"),
        "expected <biophysicalProperties in:\n{out}"
    );
    assert!(
        out.contains("variable=\"voltage\""),
        "expected variable=\"voltage\" in:\n{out}"
    );
    assert!(
        out.contains("min=\"min_v\" max=\"max_v\""),
        "expected bounds in:\n{out}"
    );
}

// ── 6. morphogen_block_emits_morphology ───────────────────────────────────────

#[test]
fn morphogen_block_emits_morphology() {
    let module = parse(MORPHOGEN_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("<morphology"),
        "expected <morphology in:\n{out}"
    );
    assert!(
        out.contains("threshold=\"0.8\""),
        "expected threshold=\"0.8\" in:\n{out}"
    );
}

// ── 7. ecosystem_with_beings_emits_network ────────────────────────────────────

#[test]
fn ecosystem_with_beings_emits_network() {
    let module = parse(ECOSYSTEM_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("<network id=\"Brain\""),
        "expected <network id=\"Brain\"> in:\n{out}"
    );
    assert!(
        out.contains("<population"),
        "expected <population element in:\n{out}"
    );
    assert!(
        out.contains("component=\"Neuron\""),
        "expected component=\"Neuron\" in:\n{out}"
    );
}

// ── 8. signal_emits_projection ────────────────────────────────────────────────

#[test]
fn signal_emits_projection() {
    let module = parse(ECOSYSTEM_SIGNAL_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("<projection"),
        "expected <projection element in:\n{out}"
    );
    assert!(
        out.contains("id=\"Spike\""),
        "expected id=\"Spike\" in:\n{out}"
    );
    assert!(
        out.contains("presynapticPopulation=\"Neuron_population\""),
        "expected presynapticPopulation in:\n{out}"
    );
}

// ── 9. neuroml_has_correct_xmlns ──────────────────────────────────────────────

#[test]
fn neuroml_has_correct_xmlns() {
    let module = parse(PLASTIC_BEING_SRC);
    let out = NeuroMLEmitter::emit(&module);
    assert!(
        out.contains("xmlns=\"http://www.neuroml.org/schema/neuroml2\""),
        "expected NeuroML2 namespace in:\n{out}"
    );
    assert!(
        out.contains("<neuroml"),
        "expected <neuroml root element in:\n{out}"
    );
}

// ── 10. compile_neuroml_returns_xml_string ────────────────────────────────────

#[test]
fn compile_neuroml_returns_xml_string() {
    let result = loom::compile_neuroml(PLASTIC_BEING_SRC);
    assert!(
        result.is_ok(),
        "expected compile_neuroml to return Ok: {:?}",
        result
    );
    let out = result.unwrap();
    assert!(
        out.contains("<?xml"),
        "expected XML declaration in:\n{out}"
    );
    assert!(
        out.contains("<neuroml"),
        "expected <neuroml root in:\n{out}"
    );
    assert!(
        out.contains("<cell id=\"Neuron\""),
        "expected <cell id=\"Neuron\"> in:\n{out}"
    );
}
