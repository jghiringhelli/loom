// tests/nichect_test.rs — M77: Niche Construction (Odling-Smee)

use loom::checker::NicheConstructionChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. niche_construction item parses
#[test]
fn niche_construction_parses() {
    let src = r#"module M
niche_construction:
  modifies: soil_chemistry
  affects: [WormPopulation, PlantGrowth]
end
end"#;
    let m = parse(src);
    let nc = match &m.items[0] {
        loom::ast::Item::NicheConstruction(n) => n,
        _ => panic!("expected NicheConstruction"),
    };
    assert_eq!(nc.modifies, "soil_chemistry");
    assert_eq!(nc.affects, vec!["WormPopulation", "PlantGrowth"]);
}

// 2. niche_construction with probe_fn parses
#[test]
fn niche_construction_with_probe_fn_parses() {
    let src = r#"module M
niche_construction:
  modifies: habitat_structure
  affects: [Beaver, Fish]
  probe_fn: measure_habitat_change
end
end"#;
    let m = parse(src);
    let nc = match &m.items[0] {
        loom::ast::Item::NicheConstruction(n) => n,
        _ => panic!("expected NicheConstruction"),
    };
    assert_eq!(nc.probe_fn, Some("measure_habitat_change".to_string()));
}

// 3. checker rejects empty modifies
#[test]
fn checker_rejects_empty_modifies() {
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
        items: vec![Item::NicheConstruction(NicheConstructionDef {
            modifies: "".to_string(),
            affects: vec!["A".to_string()],
            probe_fn: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = NicheConstructionChecker::new().check(&module);
    assert!(result.is_err());
}

// 4. checker rejects empty affects
#[test]
fn checker_rejects_empty_affects() {
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
        items: vec![Item::NicheConstruction(NicheConstructionDef {
            modifies: "soil".to_string(),
            affects: vec![],
            probe_fn: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = NicheConstructionChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result.unwrap_err().iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msgs.contains("affects"), "expected 'affects' in: {msgs}");
}

// 5. checker passes valid niche_construction
#[test]
fn checker_passes_valid_niche_construction() {
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
        items: vec![Item::NicheConstruction(NicheConstructionDef {
            modifies: "soil".to_string(),
            affects: vec!["Worms".to_string()],
            probe_fn: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    assert!(NicheConstructionChecker::new().check(&module).is_ok());
}

// 6. codegen emits niche_construction comment
#[test]
fn codegen_emits_niche_construction_comment() {
    let src = r#"module M
niche_construction:
  modifies: soil_chemistry
  affects: [Worms]
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("niche_construction"), "expected niche_construction in:\n{out}");
    assert!(out.contains("soil_chemistry"), "expected soil_chemistry in:\n{out}");
}
