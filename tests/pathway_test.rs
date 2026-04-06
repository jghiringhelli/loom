// tests/pathway_test.rs — M71: Metabolic Pathways (Krebs)

use loom::checker::PathwayChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. pathway parses with steps
#[test]
fn pathway_with_steps_parses() {
    let src = r#"module M
pathway Krebs
  Oxaloacetate -[Citrate_Synthase]-> Citrate
  Citrate -[Aconitase]-> Isocitrate
end
end"#;
    let m = parse(src);
    let pw = match &m.items[0] {
        loom::ast::Item::Pathway(p) => p,
        _ => panic!("expected Pathway"),
    };
    assert_eq!(pw.name, "Krebs");
    assert_eq!(pw.steps.len(), 2);
    assert_eq!(pw.steps[0].from, "Oxaloacetate");
    assert_eq!(pw.steps[0].via, "Citrate_Synthase");
    assert_eq!(pw.steps[0].to, "Citrate");
}

// 2. pathway with compensate parses
#[test]
fn pathway_with_compensate_parses() {
    let src = r#"module M
pathway Glycolysis
  Glucose -[Hexokinase]-> G6P
  compensate: gluconeogenesis
end
end"#;
    let m = parse(src);
    let pw = match &m.items[0] {
        loom::ast::Item::Pathway(p) => p,
        _ => panic!("expected Pathway"),
    };
    assert_eq!(pw.compensate, Some("gluconeogenesis".to_string()));
}

// 3. checker rejects empty pathway
#[test]
fn checker_rejects_empty_pathway() {
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
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![Item::Pathway(PathwayDef {
            name: "Empty".to_string(),
            steps: vec![],
            compensate: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = PathwayChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result.unwrap_err().iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msgs.contains("at least one step"), "expected 'at least one step' in: {msgs}");
}

// 4. checker rejects step with identical from/to
#[test]
fn checker_rejects_trivial_step() {
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
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![Item::Pathway(PathwayDef {
            name: "Bad".to_string(),
            steps: vec![PathwayStep {
                from: "A".to_string(),
                via: "enzyme".to_string(),
                to: "A".to_string(),
                span: Span::synthetic(),
            }],
            compensate: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = PathwayChecker::new().check(&module);
    assert!(result.is_err());
}

// 5. checker passes valid pathway
#[test]
fn checker_passes_valid_pathway() {
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
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![Item::Pathway(PathwayDef {
            name: "Krebs".to_string(),
            steps: vec![PathwayStep {
                from: "OAA".to_string(),
                via: "CS".to_string(),
                to: "Citrate".to_string(),
                span: Span::synthetic(),
            }],
            compensate: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    assert!(PathwayChecker::new().check(&module).is_ok());
}

// 6. codegen emits pathway comment
#[test]
fn codegen_emits_pathway_comment() {
    let src = r#"module M
pathway Krebs
  OAA -[CS]-> Citrate
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("pathway"), "expected pathway in:\n{out}");
    assert!(out.contains("Krebs"), "expected Krebs in:\n{out}");
}
