// M105: scenario: — Being-level Executable Acceptance Criteria
//
// Validates scenario: block parsing, ScenarioChecker behavior, and Rust codegen.
// Beck (2002) TDD → Cucumber BDD (2008) Given/When/Then → GS Executable property
// → Loom `scenario:` (M105).

use loom::ast::*;
use loom::checker::ScenarioChecker;
use loom::compile;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse_ok(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ─── Test 1: scenario: with given/when/then parses ───────────────────────────

#[test]
fn test_m105_scenario_parses() {
    let src = r#"
module Trade
  being Trader
    telos: "execute profitable trades"
    end
    scenario trade_executes_on_signal:
      given: market_signal
      when: being sense detects market_signal
      then: ensure position_size
    end
  end
end
"#;
    let module = parse_ok(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    assert_eq!(being.scenarios.len(), 1);
    let sc = &being.scenarios[0];
    assert_eq!(sc.name, "trade_executes_on_signal");
    assert!(!sc.given.is_empty(), "given should not be empty");
    assert!(!sc.when.is_empty(), "when should not be empty");
    assert!(!sc.then.is_empty(), "then should not be empty");
}

// ─── Test 2: within: N unit parses ───────────────────────────────────────────

#[test]
fn test_m105_within_parses() {
    let src = r#"
module Trade
  being TimedTrader
    telos: "trade within deadline"
    end
    scenario fast_execution:
      given: signal_received
      when: being sense detects signal_received
      then: ensure position_size
      within: 3 lifecycle_ticks
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    let sc = &being.scenarios[0];
    assert!(sc.within.is_some(), "within: should be present");
    let (count, unit) = sc.within.as_ref().unwrap();
    assert_eq!(*count, 3);
    assert_eq!(unit, "lifecycle_ticks");
}

// ─── Test 3: then: "" → error ─────────────────────────────────────────────────

#[test]
fn test_m105_empty_then_is_error() {
    // We parse normally but then check with ScenarioChecker
    let src = r#"
module Trade
  being BadTrader
    telos: "trade"
    end
    scenario empty_assertion:
      given: some_condition
      when: some_trigger
      then:
    end
  end
end
"#;
    // The parse will succeed (parser is lenient about empty then)
    let tokens = Lexer::tokenize(src).expect("lex ok");
    let module = Parser::new(&tokens).parse_module().expect("parse ok");
    let checker = ScenarioChecker::new();
    let errors = checker.check(&module);
    assert!(
        errors.iter().any(|e| {
            let msg = format!("{}", e);
            msg.contains("empty") && !msg.contains("[warn]")
        }),
        "empty then: should produce an error: {:?}", errors
    );
}

// ─── Test 4: within: 0 → error ────────────────────────────────────────────────

#[test]
fn test_m105_zero_within_is_error() {
    let src = r#"
module Trade
  being ZeroTrader
    telos: "trade"
    end
    scenario instant:
      given: signal
      when: detection
      then: ensure result
      within: 0 lifecycle_ticks
    end
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "within: 0 must be a compile error"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| format!("{}", e).contains("zero")),
        "should mention zero within: {:?}", errors
    );
}

// ─── Test 5: being with two scenario: blocks ──────────────────────────────────

#[test]
fn test_m105_multiple_scenarios_parse() {
    let src = r#"
module Exchange
  being MultiTrader
    telos: "trade profitably"
    end
    scenario scenario_one:
      given: condition_one
      when: trigger_one
      then: ensure outcome_one
    end
    scenario scenario_two:
      given: condition_two
      when: trigger_two
      then: ensure outcome_two
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    assert_eq!(being.scenarios.len(), 2, "should have 2 scenario blocks");
    assert_eq!(being.scenarios[0].name, "scenario_one");
    assert_eq!(being.scenarios[1].name, "scenario_two");
}

// ─── Test 6: compile_rust emits #[test] fn for scenario ──────────────────────

#[test]
fn test_m105_scenario_emits_test_stub() {
    let src = r#"
module Exchange
  being ScenarioTrader
    telos: "trade profitably"
    end
    scenario signal_triggers_trade:
      given: market_signal
      when: detection
      then: ensure position_size
    end
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "should compile: {:?}", result.err());
    let output = result.unwrap();
    assert!(
        output.contains("#[test]"),
        "codegen should emit #[test] for scenario: {:?}", &output[..output.len().min(500)]
    );
    assert!(
        output.contains("scenario_signal_triggers_trade"),
        "codegen should emit scenario function name: {:?}", &output[..output.len().min(500)]
    );
}
