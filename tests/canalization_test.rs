// tests/canalization_test.rs — M70: Canalization (Waddington)

use loom::checker::CanalizationChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. being with canalize block parses
#[test]
fn being_with_canalize_parses() {
    let src = r#"module M
being Organism
  telos: "maintain homeostasis"
  end
  canalize:
    toward: homeostasis
    despite: [temperature_stress, nutrient_deprivation]
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let can = being.canalization.as_ref().expect("canalization");
    assert_eq!(can.toward, "homeostasis");
    assert_eq!(
        can.despite,
        vec!["temperature_stress", "nutrient_deprivation"]
    );
}

// 2. being with canalize + convergence_proof parses
#[test]
fn being_with_canalize_convergence_proof_parses() {
    let src = r#"module M
being Cell
  telos: "differentiate"
  end
  canalize:
    toward: differentiated_state
    despite: [noise]
    convergence_proof: lyapunov_argument
  end
end
end"#;
    let m = parse(src);
    let being = &m.being_defs[0];
    let can = being.canalization.as_ref().expect("canalization");
    assert_eq!(can.convergence_proof, Some("lyapunov_argument".to_string()));
}

// 3. being without canalize has None
#[test]
fn being_without_canalize_is_none() {
    let src = r#"module M
being Simple
  telos: "exist"
  end
end
end"#;
    let m = parse(src);
    assert!(m.being_defs[0].canalization.is_none());
}

// 4. checker rejects empty toward
#[test]
fn checker_rejects_empty_toward() {
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
        canalization: Some(CanalizationBlock {
            toward: "".to_string(),
            despite: vec!["stress".to_string()],
            convergence_proof: None,
            span: Span::synthetic(),
        }),
        senescence: None,
        criticality: None,
        umwelt: None,
        resonance: None,
        manifest: None,
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
        cognitive_memory: None,
        signal_attention: None,
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
    let result = CanalizationChecker::new().check(&module);
    assert!(result.is_err());
}

// 5. checker rejects empty despite list
#[test]
fn checker_rejects_empty_despite() {
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
        canalization: Some(CanalizationBlock {
            toward: "homeostasis".to_string(),
            despite: vec![],
            convergence_proof: None,
            span: Span::synthetic(),
        }),
        senescence: None,
        criticality: None,
        umwelt: None,
        resonance: None,
        manifest: None,
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
        cognitive_memory: None,
        signal_attention: None,
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
    let result = CanalizationChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result
        .unwrap_err()
        .iter()
        .map(|e| format!("{e}"))
        .collect::<String>();
    assert!(msgs.contains("despite"), "expected 'despite' in: {msgs}");
}

// 6. codegen emits canalize comments
#[test]
fn codegen_emits_canalize_comments() {
    let src = r#"module M
being Organism
  telos: "maintain homeostasis"
  end
  canalize:
    toward: homeostasis
    despite: [stress]
  end
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("canalize"), "expected canalize in:\n{out}");
    assert!(
        out.contains("homeostasis"),
        "expected homeostasis in:\n{out}"
    );
}
