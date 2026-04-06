// tests/senescence_test.rs — M74: Senescence (Campisi)

use loom::checker::SenescenceChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. being with senescence block parses
#[test]
fn being_with_senescence_parses() {
    let src = r#"module M
being Cell
  telos: "maintain function"
  end
  senescence:
    onset: replication_limit_reached
    degradation: proteostasis_failure
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let sen = being.senescence.as_ref().expect("senescence");
    assert_eq!(sen.onset, "replication_limit_reached");
    assert_eq!(sen.degradation, "proteostasis_failure");
}

// 2. being with senescence + sasp parses
#[test]
fn being_with_senescence_sasp_parses() {
    let src = r#"module M
being Fibroblast
  telos: "produce extracellular matrix"
  end
  senescence:
    onset: oncogene_activation
    degradation: mitochondrial_dysfunction
    sasp: inflammatory_cytokines
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let sen = being.senescence.as_ref().expect("senescence");
    assert_eq!(sen.sasp, Some("inflammatory_cytokines".to_string()));
}

// 3. being without senescence has None
#[test]
fn being_without_senescence_is_none() {
    let src = r#"module M
being Immortal
  telos: "live forever"
  end
end
end"#;
    let m = parse(src);
    assert!(m.being_defs[0].senescence.is_none());
}

// 4. checker rejects empty onset
#[test]
fn checker_rejects_empty_onset() {
    use loom::ast::*;
    let being = BeingDef {
        name: "Cell".to_string(),
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
        senescence: Some(SenescenceBlock {
            onset: "".to_string(),
            degradation: "proteostasis".to_string(),
            sasp: None,
            span: Span::synthetic(),
        }),
        criticality: None,
        umwelt: None,
        resonance: None,
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
    let result = SenescenceChecker::new().check(&module);
    assert!(result.is_err());
}

// 5. checker passes valid senescence
#[test]
fn checker_passes_valid_senescence() {
    use loom::ast::*;
    let being = BeingDef {
        name: "Cell".to_string(),
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
        senescence: Some(SenescenceBlock {
            onset: "telomere_shortening".to_string(),
            degradation: "proteostasis_failure".to_string(),
            sasp: None,
            span: Span::synthetic(),
        }),
        criticality: None,
        umwelt: None,
        resonance: None,
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
    assert!(SenescenceChecker::new().check(&module).is_ok());
}

// 6. codegen emits senescence comment
#[test]
fn codegen_emits_senescence_comment() {
    let src = r#"module M
being Cell
  telos: "function"
  end
  senescence:
    onset: limit_reached
    degradation: proteostasis
  end
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("senescence"), "expected senescence in:\n{out}");
    assert!(out.contains("limit_reached"), "expected limit_reached in:\n{out}");
}
