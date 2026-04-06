//! M45/M46/M47 tests — epigenetic/morphogen/telomere blocks.
//! Waddington (1957), Turing (1952), Hayflick (1961).

use loom::ast::{BeingDef, EpigeneticBlock, MorphogenBlock, TelomereBlock, Span, TelosDef};
use loom::checker::check_teleos;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn make_module(being: BeingDef) -> loom::ast::Module {
    loom::ast::Module {
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
            temporal_defs: vec![],
            aspect_defs: vec![],
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    }
}

fn base_being() -> BeingDef {
    BeingDef {
        name: "Cell".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: Some(TelosDef {
            description: "survive and replicate".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: None,
            sign: None,
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
        canalization: None,
        senescence: None,
        criticality: None,
        umwelt: None,
        resonance: None,
        span: Span::synthetic(),
    }
}

// ── Epigenetic tests ──────────────────────────────────────────────────────────

#[test]
fn epigenetic_parses_with_signal_and_modifies() {
    let src = r#"module Test
being Cell
  telos: "survive and replicate"
  end
  epigenetic:
    signal:    EnvironmentalStress
    modifies:  metabolism
    reverts_when: stress_absent
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    assert_eq!(being.epigenetic_blocks.len(), 1);
    let epi = &being.epigenetic_blocks[0];
    assert_eq!(epi.signal, "EnvironmentalStress");
    assert_eq!(epi.modifies, "metabolism");
    assert_eq!(epi.reverts_when, Some("stress_absent".to_string()));
}

#[test]
fn epigenetic_empty_signal_fails_checker() {
    let mut being = base_being();
    being.epigenetic_blocks.push(EpigeneticBlock {
        signal: "".to_string(),
        modifies: "metabolism.rate".to_string(),
        reverts_when: None,
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty epigenetic signal");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty signal"), "expected 'empty signal' in: {msg}");
}

#[test]
fn epigenetic_empty_modifies_fails_checker() {
    let mut being = base_being();
    being.epigenetic_blocks.push(EpigeneticBlock {
        signal: "Stress".to_string(),
        modifies: "".to_string(),
        reverts_when: None,
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty epigenetic modifies");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("empty modifies"), "expected 'empty modifies' in: {msg}");
}

#[test]
fn rust_emit_being_has_epigenetic_fn() {
    let mut being = base_being();
    being.epigenetic_blocks.push(EpigeneticBlock {
        signal: "EnvironmentalStress".to_string(),
        modifies: "metabolism.rate".to_string(),
        reverts_when: Some("stress_absent".to_string()),
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("apply_epigenetic_environmental_stress"), "expected apply_epigenetic fn in: {out}");
    assert!(out.contains("signal_strength: f64"), "expected signal_strength param in: {out}");
}

#[test]
fn typescript_emit_being_has_epigenetic_method() {
    let mut being = base_being();
    being.epigenetic_blocks.push(EpigeneticBlock {
        signal: "EnvironmentalStress".to_string(),
        modifies: "metabolism.rate".to_string(),
        reverts_when: None,
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("applyEpigeneticEnvironmentalStress"), "expected TS epigenetic method in: {out}");
    assert!(out.contains("signalStrength: number"), "expected signalStrength param in: {out}");
}

// ── Morphogen tests ───────────────────────────────────────────────────────────

#[test]
fn morphogen_parses_with_threshold_and_produces() {
    let src = r#"module Test
being Cell
  telos: "survive and replicate"
  end
  morphogen:
    signal:    GrowthFactor
    threshold: 0.8
    produces:  [Ribosome, Membrane]
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    assert_eq!(being.morphogen_blocks.len(), 1);
    let morph = &being.morphogen_blocks[0];
    assert_eq!(morph.signal, "GrowthFactor");
    assert!(morph.threshold.starts_with("0.8"), "threshold should be 0.8, got: {}", morph.threshold);
    assert_eq!(morph.produces, vec!["Ribosome".to_string(), "Membrane".to_string()]);
}

#[test]
fn morphogen_empty_produces_fails_checker() {
    let mut being = base_being();
    being.morphogen_blocks.push(MorphogenBlock {
        signal: "GrowthFactor".to_string(),
        threshold: "0.8".to_string(),
        produces: vec![],
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for empty morphogen produces");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("inert"), "expected 'inert' in: {msg}");
}

#[test]
fn morphogen_threshold_out_of_range_fails_checker() {
    let mut being = base_being();
    being.morphogen_blocks.push(MorphogenBlock {
        signal: "GrowthFactor".to_string(),
        threshold: "1.5".to_string(),
        produces: vec!["Ribosome".to_string()],
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for threshold out of range");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("out of range") || msg.contains("0.0") || msg.contains("1.0"), "expected range error in: {msg}");
}

#[test]
fn rust_emit_being_has_differentiate_fn() {
    let mut being = base_being();
    being.morphogen_blocks.push(MorphogenBlock {
        signal: "GrowthFactor".to_string(),
        threshold: "0.8".to_string(),
        produces: vec!["Ribosome".to_string()],
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("differentiate_growth_factor"), "expected differentiate fn in: {out}");
    assert!(out.contains("signal_level: f64"), "expected signal_level param in: {out}");
}

#[test]
fn typescript_emit_being_has_differentiate_method() {
    let mut being = base_being();
    being.morphogen_blocks.push(MorphogenBlock {
        signal: "GrowthFactor".to_string(),
        threshold: "0.8".to_string(),
        produces: vec!["Ribosome".to_string()],
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("differentiateGrowthFactor"), "expected TS differentiate method in: {out}");
    assert!(out.contains("signalLevel: number"), "expected signalLevel param in: {out}");
}

// ── Telomere tests ────────────────────────────────────────────────────────────

#[test]
fn telomere_parses_with_limit_and_exhaustion() {
    let src = r#"module Test
being Cell
  telos: "survive and replicate"
  end
  telomere:
    limit:        50
    on_exhaustion: senescence
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    assert!(being.telomere.is_some());
    let tel = being.telomere.as_ref().unwrap();
    assert_eq!(tel.limit, 50);
    assert_eq!(tel.on_exhaustion, "senescence");
}

#[test]
fn telomere_zero_limit_fails_checker() {
    let mut being = base_being();
    being.telomere = Some(TelomereBlock {
        limit: 0,
        on_exhaustion: "senescence".to_string(),
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for zero telomere limit");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("positive"), "expected 'positive' in: {msg}");
}

#[test]
fn telomere_rust_emit_has_telomere_count_field() {
    let mut being = base_being();
    being.telomere = Some(TelomereBlock {
        limit: 50,
        on_exhaustion: "senescence".to_string(),
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("telomere_count: u64"), "expected telomere_count field in: {out}");
}

#[test]
fn telomere_rust_emit_has_replicate_fn() {
    let mut being = base_being();
    being.telomere = Some(TelomereBlock {
        limit: 50,
        on_exhaustion: "senescence".to_string(),
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("fn replicate"), "expected replicate fn in: {out}");
    assert!(out.contains("telomere exhausted"), "expected 'telomere exhausted' in: {out}");
}

#[test]
fn typescript_emit_telomere_has_limit_field() {
    let mut being = base_being();
    being.telomere = Some(TelomereBlock {
        limit: 50,
        on_exhaustion: "senescence".to_string(),
        span: Span::synthetic(),
    });
    let module = make_module(being);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("telomereLimit"), "expected telomereLimit in: {out}");
    assert!(out.contains("50"), "expected limit value 50 in: {out}");
    assert!(out.contains("replicate"), "expected replicate method in: {out}");
}
