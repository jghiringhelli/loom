//! M48/M49/M50 tests — crispr:, quorum:, plasticity: self-modification constructs.

use loom::ast::{
    BeingDef, CrisprBlock, EcosystemDef, Module, PlasticityBlock, PlasticityRule, QuorumBlock,
    SignalDef, Span, TelosDef,
};
use loom::checker::check_teleos;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn make_module_with_being(being: BeingDef) -> Module {
    Module {
        name: "Test".to_string(),
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
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    }
}

fn make_module_with_ecosystem(eco: EcosystemDef) -> Module {
    Module {
        name: "Test".to_string(),
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
        being_defs: vec![],
        ecosystem_defs: vec![eco],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    }
}

fn base_being() -> BeingDef {
    BeingDef {
        name: "Genome".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: Some(TelosDef {
            description: "self-correct errors".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: None,
            span: Span::synthetic(),
        }),
        regulate_blocks: vec![],
        evolve_block: None,
        epigenetic_blocks: vec![],
        morphogen_blocks: vec![],
        telomere: None,
        autopoietic: false,
        crispr_blocks: vec![],
        plasticity_blocks: vec![],
        span: Span::synthetic(),
    }
}

// ── 1. crispr_parses_target_replace_guide ────────────────────────────────────

#[test]
fn crispr_parses_target_replace_guide() {
    let src = r#"module Test
being Genome
  telos: "self-correct errors"
  end
  crispr:
    target: error_sequence
    replace: corrected_sequence
    guide: CasProtein
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let b = &module.being_defs[0];
    assert_eq!(b.crispr_blocks.len(), 1);
    let crispr = &b.crispr_blocks[0];
    assert_eq!(crispr.target, "error_sequence");
    assert_eq!(crispr.replace, "corrected_sequence");
    assert_eq!(crispr.guide, "CasProtein");
}

// ── 2. crispr_empty_target_fails ──────────────────────────────────────────────

#[test]
fn crispr_empty_target_fails() {
    let mut being = base_being();
    being.crispr_blocks = vec![CrisprBlock {
        target: "".to_string(),
        replace: "CorrectType".to_string(),
        guide: "CasProtein".to_string(),
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty crispr target");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty target"), "expected 'empty target' in: {msg}");
}

// ── 3. crispr_empty_replace_fails ────────────────────────────────────────────

#[test]
fn crispr_empty_replace_fails() {
    let mut being = base_being();
    being.crispr_blocks = vec![CrisprBlock {
        target: "error_sequence".to_string(),
        replace: "".to_string(),
        guide: "CasProtein".to_string(),
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty crispr replace");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty replace"), "expected 'empty replace' in: {msg}");
}

// ── 4. crispr_empty_guide_fails ──────────────────────────────────────────────

#[test]
fn crispr_empty_guide_fails() {
    let mut being = base_being();
    being.crispr_blocks = vec![CrisprBlock {
        target: "error_sequence".to_string(),
        replace: "CorrectType".to_string(),
        guide: "".to_string(),
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty crispr guide");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty guide"), "expected 'empty guide' in: {msg}");
}

// ── 5. rust_emit_crispr_has_edit_fn ──────────────────────────────────────────

#[test]
fn rust_emit_crispr_has_edit_fn() {
    let mut being = base_being();
    being.crispr_blocks = vec![CrisprBlock {
        target: "error_sequence".to_string(),
        replace: "CorrectType".to_string(),
        guide: "CasProtein".to_string(),
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn edit_cas_protein"), "expected edit_cas_protein in:\n{out}");
    assert!(out.contains("CRISPR"), "expected CRISPR comment in:\n{out}");
}

// ── 6. plasticity_parses_trigger_modifies_rule ───────────────────────────────

#[test]
fn plasticity_parses_trigger_modifies_rule() {
    let src = r#"module Test
being Neuron
  telos: "learn from experience"
  end
  plasticity:
    trigger: FireSignal
    modifies: SynapticWeight
    rule: hebbian
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let b = &module.being_defs[0];
    assert_eq!(b.plasticity_blocks.len(), 1);
    let p = &b.plasticity_blocks[0];
    assert_eq!(p.trigger, "FireSignal");
    assert_eq!(p.modifies, "SynapticWeight");
    assert_eq!(p.rule, PlasticityRule::Hebbian);
}

// ── 7. plasticity_boltzmann_rule_parses ───────────────────────────────────────

#[test]
fn plasticity_boltzmann_rule_parses() {
    let src = r#"module Test
being Neuron
  telos: "equilibrate energy"
  end
  plasticity:
    trigger: EnergySignal
    modifies: NetworkWeight
    rule: boltzmann
  end
end
end
"#;
    let module = parse(src);
    let p = &module.being_defs[0].plasticity_blocks[0];
    assert_eq!(p.rule, PlasticityRule::Boltzmann);
}

// ── 8. plasticity_empty_trigger_fails ────────────────────────────────────────

#[test]
fn plasticity_empty_trigger_fails() {
    let mut being = base_being();
    being.plasticity_blocks = vec![PlasticityBlock {
        trigger: "".to_string(),
        modifies: "SynapticWeight".to_string(),
        rule: PlasticityRule::Hebbian,
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty plasticity trigger");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty trigger"), "expected 'empty trigger' in: {msg}");
}

// ── 9. plasticity_empty_modifies_fails ───────────────────────────────────────

#[test]
fn plasticity_empty_modifies_fails() {
    let mut being = base_being();
    being.plasticity_blocks = vec![PlasticityBlock {
        trigger: "FireSignal".to_string(),
        modifies: "".to_string(),
        rule: PlasticityRule::Hebbian,
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty plasticity modifies");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty modifies"), "expected 'empty modifies' in: {msg}");
}

// ── 10. rust_emit_plasticity_has_update_fn ────────────────────────────────────

#[test]
fn rust_emit_plasticity_has_update_fn() {
    let mut being = base_being();
    being.plasticity_blocks = vec![PlasticityBlock {
        trigger: "FireSignal".to_string(),
        modifies: "SynapticWeight".to_string(),
        rule: PlasticityRule::Hebbian,
        span: Span::synthetic(),
    }];
    let module = make_module_with_being(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn update_synaptic_weight"), "expected update_synaptic_weight in:\n{out}");
    assert!(out.contains("Hebbian"), "expected Hebbian in:\n{out}");
}

// ── 11. quorum_parses_signal_threshold_action ────────────────────────────────

#[test]
fn quorum_parses_signal_threshold_action() {
    let src = r#"module Test
being Bacterium
  telos: "survive"
  end
end
ecosystem Colony
  members: [Bacterium]
  signal AHL from Bacterium to Bacterium
    payload: Float
  end
  quorum:
    signal: AHL
    threshold: 0.6
    action: biofilm_formation
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.ecosystem_defs.len(), 1);
    let eco = &module.ecosystem_defs[0];
    assert_eq!(eco.quorum_blocks.len(), 1);
    let q = &eco.quorum_blocks[0];
    assert_eq!(q.signal, "AHL");
    assert_eq!(q.threshold, "0.6");
    assert_eq!(q.action, "biofilm_formation");
}

// ── 12. quorum_empty_signal_fails ────────────────────────────────────────────

#[test]
fn quorum_empty_signal_fails() {
    let eco = EcosystemDef {
        name: "Colony".to_string(),
        describe: None,
        members: vec!["Bacterium".to_string()],
        signals: vec![SignalDef {
            name: "AHL".to_string(),
            from: "Bacterium".to_string(),
            to: "Bacterium".to_string(),
            payload: "Float".to_string(),
            span: Span::synthetic(),
        }],
        telos: None,
        quorum_blocks: vec![QuorumBlock {
            signal: "".to_string(),
            threshold: "0.6".to_string(),
            action: "biofilm_formation".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let module = make_module_with_ecosystem(eco);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty quorum signal");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty signal"), "expected 'empty signal' in: {msg}");
}

// ── 13. quorum_empty_action_fails ────────────────────────────────────────────

#[test]
fn quorum_empty_action_fails() {
    let eco = EcosystemDef {
        name: "Colony".to_string(),
        describe: None,
        members: vec!["Bacterium".to_string()],
        signals: vec![SignalDef {
            name: "AHL".to_string(),
            from: "Bacterium".to_string(),
            to: "Bacterium".to_string(),
            payload: "Float".to_string(),
            span: Span::synthetic(),
        }],
        telos: None,
        quorum_blocks: vec![QuorumBlock {
            signal: "AHL".to_string(),
            threshold: "0.6".to_string(),
            action: "".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let module = make_module_with_ecosystem(eco);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty quorum action");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty action"), "expected 'empty action' in: {msg}");
}

// ── 14. quorum_invalid_threshold_fails ───────────────────────────────────────

#[test]
fn quorum_invalid_threshold_fails() {
    let eco = EcosystemDef {
        name: "Colony".to_string(),
        describe: None,
        members: vec!["Bacterium".to_string()],
        signals: vec![SignalDef {
            name: "AHL".to_string(),
            from: "Bacterium".to_string(),
            to: "Bacterium".to_string(),
            payload: "Float".to_string(),
            span: Span::synthetic(),
        }],
        telos: None,
        quorum_blocks: vec![QuorumBlock {
            signal: "AHL".to_string(),
            threshold: "1.5".to_string(), // > 1.0, invalid
            action: "biofilm_formation".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let module = make_module_with_ecosystem(eco);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for threshold > 1.0");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("threshold"), "expected 'threshold' in: {msg}");
}

// ── 15. rust_emit_quorum_has_check_fn ────────────────────────────────────────

#[test]
fn rust_emit_quorum_has_check_fn() {
    let eco = EcosystemDef {
        name: "Colony".to_string(),
        describe: None,
        members: vec!["Bacterium".to_string()],
        signals: vec![SignalDef {
            name: "AHL".to_string(),
            from: "Bacterium".to_string(),
            to: "Bacterium".to_string(),
            payload: "Float".to_string(),
            span: Span::synthetic(),
        }],
        telos: Some("collective biofilm formation".to_string()),
        quorum_blocks: vec![QuorumBlock {
            signal: "AHL".to_string(),
            threshold: "0.6".to_string(),
            action: "biofilm_formation".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let module = make_module_with_ecosystem(eco);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn check_quorum_a_h_l"), "expected check_quorum_a_h_l in:\n{out}");
    assert!(out.contains("Bassler"), "expected Bassler citation in:\n{out}");
}
