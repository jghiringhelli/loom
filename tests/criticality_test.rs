// tests/criticality_test.rs — M76: Criticality Bounds (Langton)

use loom::checker::CriticalityChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. being with criticality block parses
#[test]
fn being_with_criticality_parses() {
    let src = r#"module M
being Network
  telos: "compute"
  end
  criticality:
    lower: 0.2
    upper: 0.8
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let crit = being.criticality.as_ref().expect("criticality");
    assert!((crit.lower - 0.2).abs() < 1e-9);
    assert!((crit.upper - 0.8).abs() < 1e-9);
}

// 2. being with criticality + probe_fn parses
#[test]
fn being_with_criticality_probe_fn_parses() {
    let src = r#"module M
being CA
  telos: "evolve"
  end
  criticality:
    lower: 0.3
    upper: 0.7
    probe_fn: measure_entropy
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let crit = being.criticality.as_ref().expect("criticality");
    assert_eq!(crit.probe_fn, Some("measure_entropy".to_string()));
}

// 3. being without criticality has None
#[test]
fn being_without_criticality_is_none() {
    let src = r#"module M
being Simple
  telos: "exist"
  end
end
end"#;
    let m = parse(src);
    assert!(m.being_defs[0].criticality.is_none());
}

// 4. checker rejects upper <= lower
#[test]
fn checker_rejects_inverted_bounds() {
    use loom::ast::*;
    let being = BeingDef {
        name: "Net".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: None,
        regulate_blocks: vec![],
        evolve_block: None,
        epigenetic_blocks: vec![],
        morphogen_blocks: vec![],
        telomere: None,
        autopoietic: false,
        crispr_blocks: vec![],
        plasticity_blocks: vec![],
        canalization: None,
        senescence: None,
        criticality: Some(CriticalityBlock {
            lower: 0.8,
            upper: 0.2,
            probe_fn: None,
            span: Span::synthetic(),
        }),
        umwelt: None,
        resonance: None,
            manifest: None,
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
            cognitive_memory: None,
        span: Span::synthetic(),
    };
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
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = CriticalityChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result.unwrap_err().iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msgs.contains("upper") && msgs.contains("lower"), "expected bound error in: {msgs}");
}

// 5. checker passes valid criticality
#[test]
fn checker_passes_valid_criticality() {
    use loom::ast::*;
    let being = BeingDef {
        name: "Net".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: None,
        regulate_blocks: vec![],
        evolve_block: None,
        epigenetic_blocks: vec![],
        morphogen_blocks: vec![],
        telomere: None,
        autopoietic: false,
        crispr_blocks: vec![],
        plasticity_blocks: vec![],
        canalization: None,
        senescence: None,
        criticality: Some(CriticalityBlock {
            lower: 0.3,
            upper: 0.7,
            probe_fn: None,
            span: Span::synthetic(),
        }),
        umwelt: None,
        resonance: None,
            manifest: None,
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
            cognitive_memory: None,
        span: Span::synthetic(),
    };
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
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    assert!(CriticalityChecker::new().check(&module).is_ok());
}

// 6. codegen emits criticality comment
#[test]
fn codegen_emits_criticality_comment() {
    let src = r#"module M
being Network
  telos: "compute"
  end
  criticality:
    lower: 0.2
    upper: 0.8
  end
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("criticality"), "expected criticality in:\n{out}");
}
