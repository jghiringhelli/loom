/// M155 — Top-level `chain` item: parser + codegen tests.
///
/// `chain Name states: [A, B, C] transitions: A -> B: 0.7 A -> C: 0.3 end end`
/// must parse into a `ChainDef` and emit:
///  - `{Name}State` enum
///  - `{Name}TransitionMatrix` struct with `new()`, `set()`, `next_states()`, `validate()`
///  - LOOM[chain:Markov] audit comment
///  - M155 reference

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M155.1: basic chain parses without error ─────────────────────────────────

#[test]
fn m155_simple_chain_parses() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy, Cloudy]
  transitions:
    Sunny -> Rainy: 0.2
    Sunny -> Cloudy: 0.3
    Sunny -> Sunny: 0.5
  end
end
end
"#;
    // compile succeeds and emits a TransitionMatrix — proof the chain was parsed
    let out = compile(src);
    assert!(out.contains("WeatherTransitionMatrix"), "expected WeatherTransitionMatrix in output\n{out}");
}

// ─── M155.2: State enum emitted with correct variants ─────────────────────────

#[test]
fn m155_state_enum_emitted() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy, Cloudy]
  transitions:
    Sunny -> Rainy: 0.2
    Sunny -> Cloudy: 0.3
    Sunny -> Sunny: 0.5
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub enum WeatherState"), "missing WeatherState enum\n{out}");
    assert!(out.contains("Sunny,"), "missing Sunny variant\n{out}");
    assert!(out.contains("Rainy,"), "missing Rainy variant\n{out}");
    assert!(out.contains("Cloudy,"), "missing Cloudy variant\n{out}");
}

// ─── M155.3: TransitionMatrix struct emitted ─────────────────────────────────

#[test]
fn m155_transition_matrix_struct_emitted() {
    let src = r#"
module traffic
chain Traffic
  states: [Red, Green, Yellow]
  transitions:
    Red -> Green: 0.9
    Red -> Red: 0.1
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub struct TrafficTransitionMatrix"), "missing struct\n{out}");
}

// ─── M155.4: new() constructor emitted ───────────────────────────────────────

#[test]
fn m155_new_constructor_emitted() {
    let src = r#"
module traffic
chain Traffic
  states: [Red, Green, Yellow]
  transitions:
    Red -> Green: 0.9
    Red -> Red: 0.1
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn new()"), "missing new() constructor\n{out}");
}

// ─── M155.5: transitions pre-initialized in new() ────────────────────────────

#[test]
fn m155_transitions_pre_initialized() {
    let src = r#"
module traffic
chain Traffic
  states: [Red, Green, Yellow]
  transitions:
    Red -> Green: 0.9
    Red -> Red: 0.1
  end
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("TrafficState::Red, TrafficState::Green"),
        "missing Red->Green transition in new()\n{out}"
    );
    assert!(
        out.contains("TrafficState::Red, TrafficState::Red"),
        "missing Red->Red transition in new()\n{out}"
    );
}

// ─── M155.6: validate() method emitted ───────────────────────────────────────

#[test]
fn m155_validate_method_emitted() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 0.4
    Sunny -> Sunny: 0.6
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn validate"), "missing validate() method\n{out}");
}

// ─── M155.7: next_states() method emitted ────────────────────────────────────

#[test]
fn m155_next_states_method_emitted() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 0.4
    Sunny -> Sunny: 0.6
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn next_states"), "missing next_states() method\n{out}");
}

// ─── M155.8: LOOM audit comment present ──────────────────────────────────────

#[test]
fn m155_audit_comment_present() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 0.4
    Sunny -> Sunny: 0.6
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[chain:Markov]"), "missing LOOM audit comment\n{out}");
    assert!(out.contains("M155"), "missing M155 reference\n{out}");
}

// ─── M155.9: multiple chain items in one module ───────────────────────────────

#[test]
fn m155_multiple_chains_in_module() {
    let src = r#"
module multi
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 1.0
  end
end
chain Traffic
  states: [Red, Green]
  transitions:
    Red -> Green: 1.0
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub enum WeatherState"), "missing WeatherState\n{out}");
    assert!(out.contains("pub enum TrafficState"), "missing TrafficState\n{out}");
    assert!(out.contains("pub struct WeatherTransitionMatrix"), "missing WeatherTransitionMatrix\n{out}");
    assert!(out.contains("pub struct TrafficTransitionMatrix"), "missing TrafficTransitionMatrix\n{out}");
}

// ─── M155.10: chain with no transitions section parses (transitions optional) ──

#[test]
fn m155_chain_without_transitions_parses() {
    let src = r#"
module empty_chain
chain Minimal
  states: [A, B]
end
end
"#;
    let result = compile_check(src);
    assert!(result.is_ok(), "chain without transitions should compile: {:?}", result);
}

// ─── M155.11: set() method emitted ───────────────────────────────────────────

#[test]
fn m155_set_method_emitted() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 1.0
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn set("), "missing set() method\n{out}");
}

// ─── M155.12: derive macros on State enum ────────────────────────────────────

#[test]
fn m155_state_enum_has_derive_macros() {
    let src = r#"
module weather
chain Weather
  states: [Sunny, Rainy]
  transitions:
    Sunny -> Rainy: 1.0
  end
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"),
        "missing derive macros on state enum\n{out}"
    );
}
