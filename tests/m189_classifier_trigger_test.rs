//! M189 — `trigger: classifier:` in `regulate` block.
//!
//! When a regulate block has `trigger: classifier: Name`, the parser must
//! normalise the trigger to the convention string `"classifier:Name"`, and
//! codegen must emit `// LOOM[trigger:classifier:Name]` rather than a raw
//! token-debug string.

use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn emit(src: &str) -> String {
    let module = parse(src);
    RustEmitter::new().emit(&module)
}

// ── Parser tests ──────────────────────────────────────────────────────────────

/// Basic `trigger: classifier: AnomalyDetector` stores normalised convention string.
#[test]
fn parse_classifier_trigger_normalises_to_convention() {
    let src = r#"module Env
being Sensor
  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let being = module.being_defs.first().expect("no being");
    let reg = being.regulate_blocks.first().expect("no regulate block");
    assert_eq!(
        reg.trigger.as_deref(),
        Some("classifier:AnomalyDetector"),
        "trigger must be stored as 'classifier:Name' convention; got {:?}",
        reg.trigger
    );
}

/// Action following the classifier trigger is parsed correctly.
#[test]
fn parse_classifier_trigger_action_preserved() {
    let src = r#"module Env
being Sensor
  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let being = module.being_defs.first().expect("no being");
    let reg = being.regulate_blocks.first().expect("no regulate block");
    assert_eq!(
        reg.action.as_deref(),
        Some("cool_down"),
        "action must survive classifier trigger parsing"
    );
}

/// PascalCase classifier name round-trips through normalisation.
#[test]
fn parse_classifier_trigger_pascal_case_name() {
    let src = r#"module Bio
being Host
  regulate: infection_risk
    target: 0.0
    bounds: 0.0 .. 0.5
    trigger: classifier: InfectionRiskClassifier
    action: alert
  end
end
end"#;
    let module = parse(src);
    let being = module.being_defs.first().expect("no being");
    let reg = being.regulate_blocks.first().expect("no regulate block");
    assert_eq!(
        reg.trigger.as_deref(),
        Some("classifier:InfectionRiskClassifier")
    );
}

/// Without `classifier:` prefix, the old token-collect path still runs.
#[test]
fn parse_non_classifier_trigger_falls_through() {
    let src = r#"module Bio
being Cell
  regulate: atp
    target: 100.0
    bounds: 50.0 .. 200.0
    trigger: energy_low
    action: produce_atp
  end
end
end"#;
    let module = parse(src);
    let being = module.being_defs.first().expect("no being");
    let reg = being.regulate_blocks.first().expect("no regulate block");
    let trig = reg.trigger.as_deref().unwrap_or("");
    assert!(
        !trig.starts_with("classifier:"),
        "non-classifier trigger must not be normalised to classifier convention; got {:?}",
        trig
    );
}

/// Regulate block without any trigger field has `trigger: None`.
#[test]
fn parse_regulate_without_trigger_is_none() {
    let src = r#"module Bio
being Pump
  regulate: flow
    target: 5.0
    bounds: 1.0 .. 10.0
    action: adjust_valve
  end
end
end"#;
    let module = parse(src);
    let being = module.being_defs.first().expect("no being");
    let reg = being.regulate_blocks.first().expect("no regulate block");
    assert!(
        reg.trigger.is_none(),
        "no trigger field → trigger must be None; got {:?}",
        reg.trigger
    );
}

// ── Codegen tests ─────────────────────────────────────────────────────────────

/// Classifier trigger emits `// LOOM[trigger:classifier:Name]`.
#[test]
fn codegen_classifier_trigger_emits_loom_annotation() {
    let src = r#"module Env
being Sensor
  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("// LOOM[trigger:classifier:AnomalyDetector]"),
        "codegen must emit LOOM trigger annotation; got:\n{}",
        out
    );
}

/// Classifier trigger does NOT emit raw ClassifierKw token-debug string.
#[test]
fn codegen_classifier_trigger_no_raw_debug_tokens() {
    let src = r#"module Env
being Sensor
  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let out = emit(src);
    assert!(
        !out.contains("ClassifierKw"),
        "ClassifierKw token debug repr must not appear in output; got:\n{}",
        out
    );
}

/// Non-classifier trigger emits `// trigger: ...` plain comment (existing path).
#[test]
fn codegen_non_classifier_trigger_emits_plain_comment() {
    let src = r#"module Bio
being Cell
  regulate: atp
    target: 100.0
    bounds: 50.0 .. 200.0
    trigger: energy_low
    action: produce_atp
  end
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("// trigger:"),
        "non-classifier trigger must emit plain '// trigger:' comment; got:\n{}",
        out
    );
}
