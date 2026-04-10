//! M52 tests — Mesa ABM simulation emitter.

use loom::codegen::SimulationEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

const SINGLE_BEING_SRC: &str = r#"module Bio
being Neuron
  describe: "a spiking neuron"
  matter:
    potential: Float
    threshold: Float
  end
  telos: "integrate and fire toward signal propagation"
  end
  regulate voltage
    target: resting
    bounds: (min_v, max_v)
  end
  evolve
    toward: telos
    search:
    | gradient_descent when gradient_available
    constraint: "E[distance_to_telos] decreasing"
  end
end
end
"#;

const TWO_BEING_SRC: &str = r#"module Bio
being Neuron
  telos: "fire toward signal propagation"
  end
end
being Astrocyte
  telos: "support neuron metabolism"
  end
end
end
"#;

const ECOSYSTEM_SRC: &str = r#"module Bio
being Neuron
  telos: "fire toward signal propagation"
  end
end
ecosystem Brain
  describe: "neural computation ecosystem"
  members: [Neuron]
  signal Spike from Neuron to Neuron
    payload: Float
  end
  telos: "emergent cognition"
end
end
"#;

// 1. simulation_emits_mesa_imports
#[test]
fn simulation_emits_mesa_imports() {
    let module = parse(SINGLE_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("from mesa import Agent"),
        "expected 'from mesa import Agent' in:\n{out}"
    );
}

// 2. simulation_being_becomes_agent_class
#[test]
fn simulation_being_becomes_agent_class() {
    let module = parse(SINGLE_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("class Neuron(Agent)"),
        "expected 'class Neuron(Agent)' in:\n{out}"
    );
}

// 3. simulation_agent_has_step_method
#[test]
fn simulation_agent_has_step_method() {
    let module = parse(SINGLE_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("def step(self)"),
        "expected 'def step(self)' in:\n{out}"
    );
}

// 4. simulation_agent_has_fitness_method
#[test]
fn simulation_agent_has_fitness_method() {
    let module = parse(SINGLE_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("_compute_fitness"),
        "expected '_compute_fitness' in:\n{out}"
    );
}

// 5. simulation_agent_has_telos_comment
#[test]
fn simulation_agent_has_telos_comment() {
    let module = parse(SINGLE_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("integrate and fire toward signal propagation"),
        "expected telos description in:\n{out}"
    );
}

// 6. simulation_ecosystem_becomes_model
#[test]
fn simulation_ecosystem_becomes_model() {
    let module = parse(ECOSYSTEM_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("class Brain(Model)"),
        "expected 'class Brain(Model)' in:\n{out}"
    );
}

// 7. simulation_model_has_schedule
#[test]
fn simulation_model_has_schedule() {
    let module = parse(ECOSYSTEM_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("RandomActivation"),
        "expected 'RandomActivation' in:\n{out}"
    );
}

// 8. simulation_model_has_step_method
#[test]
fn simulation_model_has_step_method() {
    let module = parse(ECOSYSTEM_SRC);
    let out = SimulationEmitter::new().emit(&module);
    // Both being and model define step; ensure it's present
    let count = out.matches("def step(self)").count();
    assert!(
        count >= 2,
        "expected at least 2 'def step(self)' (agent + model) in:\n{out}"
    );
}

// 9. simulation_multiple_beings_emit_multiple_agents
#[test]
fn simulation_multiple_beings_emit_multiple_agents() {
    let module = parse(TWO_BEING_SRC);
    let out = SimulationEmitter::new().emit(&module);
    assert!(
        out.contains("class Neuron(Agent)"),
        "expected 'class Neuron(Agent)' in:\n{out}"
    );
    assert!(
        out.contains("class Astrocyte(Agent)"),
        "expected 'class Astrocyte(Agent)' in:\n{out}"
    );
}

// 10. compile_simulation_entry_point_works
#[test]
fn compile_simulation_entry_point_works() {
    let result = loom::compile_simulation(SINGLE_BEING_SRC);
    assert!(
        result.is_ok(),
        "expected compile_simulation to return Ok: {:?}",
        result
    );
    let out = result.unwrap();
    assert!(
        out.contains("from mesa import Agent"),
        "expected mesa import in output"
    );
}
