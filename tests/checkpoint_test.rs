// tests/checkpoint_test.rs — M69: Cell Cycle Checkpoints (Hartwell)

use loom::checker::CheckpointChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. lifecycle with checkpoint block parses
#[test]
fn lifecycle_with_checkpoint_parses() {
    let src = r#"module M
lifecycle Cell :: G1 -> S -> G2 -> M
checkpoint:
  G1_S
  requires: dna_intact
  on_fail: arrest_G1
end
end
end"#;
    let m = parse(src);
    assert_eq!(m.lifecycle_defs.len(), 1);
    let lc = &m.lifecycle_defs[0];
    assert_eq!(lc.states, vec!["G1", "S", "G2", "M"]);
    assert_eq!(lc.checkpoints.len(), 1);
    let cp = &lc.checkpoints[0];
    assert_eq!(cp.name, "G1_S");
    assert_eq!(cp.requires, "dna_intact");
    assert_eq!(cp.on_fail, "arrest_G1");
}

// 2. lifecycle without checkpoints has empty vec
#[test]
fn lifecycle_without_checkpoints_is_empty() {
    let src = r#"module M
lifecycle Connection :: Closed -> Open -> Authenticated
end"#;
    let m = parse(src);
    assert!(m.lifecycle_defs[0].checkpoints.is_empty());
}

// 3. checker rejects empty requires
#[test]
fn checker_rejects_empty_requires() {
    use loom::ast::*;
    let module = Module {
        name: "M".to_string(),
        describe: None,
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![LifecycleDef {
            type_name: "Cell".to_string(),
            states: vec!["G1".to_string(), "S".to_string()],
            checkpoints: vec![CheckpointDef {
                name: "g1s".to_string(),
                requires: "".to_string(),
                on_fail: "arrest".to_string(),
                span: Span::synthetic(),
            }],
            span: Span::synthetic(),
        }],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = CheckpointChecker::new().check(&module);
    assert!(result.is_err());
}

// 4. checker rejects empty on_fail
#[test]
fn checker_rejects_empty_on_fail() {
    use loom::ast::*;
    let module = Module {
        name: "M".to_string(),
        describe: None,
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![LifecycleDef {
            type_name: "Cell".to_string(),
            states: vec!["G1".to_string(), "S".to_string()],
            checkpoints: vec![CheckpointDef {
                name: "g1s".to_string(),
                requires: "dna_intact".to_string(),
                on_fail: "".to_string(),
                span: Span::synthetic(),
            }],
            span: Span::synthetic(),
        }],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = CheckpointChecker::new().check(&module);
    assert!(result.is_err());
}

// 5. checker rejects duplicate checkpoint names
#[test]
fn checker_rejects_duplicate_checkpoint_names() {
    use loom::ast::*;
    let module = Module {
        name: "M".to_string(),
        describe: None,
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![LifecycleDef {
            type_name: "Cell".to_string(),
            states: vec!["G1".to_string(), "S".to_string()],
            checkpoints: vec![
                CheckpointDef {
                    name: "g1s".to_string(),
                    requires: "dna_intact".to_string(),
                    on_fail: "arrest".to_string(),
                    span: Span::synthetic(),
                },
                CheckpointDef {
                    name: "g1s".to_string(),
                    requires: "energy_ok".to_string(),
                    on_fail: "arrest_again".to_string(),
                    span: Span::synthetic(),
                },
            ],
            span: Span::synthetic(),
        }],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = CheckpointChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result.unwrap_err().iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msgs.contains("duplicate"), "expected 'duplicate' in: {msgs}");
}

// 6. checker passes valid checkpoints
#[test]
fn checker_passes_valid_checkpoints() {
    use loom::ast::*;
    let module = Module {
        name: "M".to_string(),
        describe: None,
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![LifecycleDef {
            type_name: "Cell".to_string(),
            states: vec!["G1".to_string(), "S".to_string()],
            checkpoints: vec![CheckpointDef {
                name: "g1s".to_string(),
                requires: "dna_intact".to_string(),
                on_fail: "arrest".to_string(),
                span: Span::synthetic(),
            }],
            span: Span::synthetic(),
        }],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    assert!(CheckpointChecker::new().check(&module).is_ok());
}
