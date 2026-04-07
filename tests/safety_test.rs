//! M55 tests — SafetyChecker: @mortal, @corrigible, @sandboxed, @bounded_telos.
//!
//! The Three Laws of Robotics as a type system (Asimov 1942, S→1 edition).

use loom::ast::{Annotation, BeingDef, Module, Span, TelosDef, TelomereBlock};
use loom::checker::SafetyChecker;

fn make_module(being: BeingDef) -> Module {
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
            temporal_defs: vec![],
            aspect_defs: vec![],
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    }
}

fn ann(key: &str) -> Annotation {
    Annotation { key: key.to_string(), value: String::new() }
}

fn base_being() -> BeingDef {
    BeingDef {
        name: "Bot".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: Some(TelosDef {
            description: "serve users".to_string(),
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
        manifest: None,
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
        span: Span::synthetic(),
    }
}

fn with_telomere(being: BeingDef) -> BeingDef {
    BeingDef {
        telomere: Some(TelomereBlock {
            limit: 100,
            on_exhaustion: "halt".to_string(),
            span: Span::synthetic(),
        }),
        ..being
    }
}

// 1. autopoietic_without_mortal_errors
#[test]
fn autopoietic_without_mortal_errors() {
    let being = BeingDef {
        autopoietic: true,
        annotations: vec![ann("sandboxed")],
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("missing @mortal")),
        "expected @mortal error, got: {:?}", errors
    );
}

// 2. autopoietic_without_sandboxed_errors
#[test]
fn autopoietic_without_sandboxed_errors() {
    let being = BeingDef {
        autopoietic: true,
        annotations: vec![ann("mortal")],
        telomere: Some(TelomereBlock {
            limit: 50,
            on_exhaustion: "halt".to_string(),
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("missing @sandboxed")),
        "expected @sandboxed error, got: {:?}", errors
    );
}

// 3. autopoietic_with_both_annotations_ok
#[test]
fn autopoietic_with_both_annotations_ok() {
    let being = BeingDef {
        autopoietic: true,
        annotations: vec![ann("mortal"), ann("sandboxed")],
        telomere: Some(TelomereBlock {
            limit: 50,
            on_exhaustion: "halt".to_string(),
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// 4. mortal_without_telomere_errors
#[test]
fn mortal_without_telomere_errors() {
    let being = BeingDef {
        annotations: vec![ann("mortal")],
        telomere: None,
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("@mortal requires telomere")),
        "expected telomere error, got: {:?}", errors
    );
}

// 5. mortal_with_telomere_ok
#[test]
fn mortal_with_telomere_ok() {
    let being = with_telomere(BeingDef {
        annotations: vec![ann("mortal")],
        ..base_being()
    });
    let errors = SafetyChecker::check(&make_module(being));
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// 6. corrigible_without_modifiable_by_errors
#[test]
fn corrigible_without_modifiable_by_errors() {
    let being = BeingDef {
        annotations: vec![ann("corrigible")],
        telos: Some(TelosDef {
            description: "serve users".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: None,
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("modifiable_by")),
        "expected modifiable_by error, got: {:?}", errors
    );
}

// 7. corrigible_with_modifiable_by_ok
#[test]
fn corrigible_with_modifiable_by_ok() {
    let being = BeingDef {
        annotations: vec![ann("corrigible")],
        telos: Some(TelosDef {
            description: "serve users".to_string(),
            fitness_fn: None,
            modifiable_by: Some("HumanAuthority".to_string()),
            bounded_by: None,
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// 8. bounded_telos_rejects_maximize
#[test]
fn bounded_telos_rejects_maximize() {
    let being = BeingDef {
        annotations: vec![ann("bounded_telos")],
        telos: Some(TelosDef {
            description: "maximize profit".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: Some("OperationalScope".to_string()),
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("maximize")),
        "expected 'maximize' error, got: {:?}", errors
    );
}

// 9. bounded_telos_rejects_unlimited
#[test]
fn bounded_telos_rejects_unlimited() {
    let being = BeingDef {
        annotations: vec![ann("bounded_telos")],
        telos: Some(TelosDef {
            description: "provide unlimited assistance".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: Some("OperationalScope".to_string()),
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("unlimited")),
        "expected 'unlimited' error, got: {:?}", errors
    );
}

// 10. bounded_telos_without_bounded_by_errors
#[test]
fn bounded_telos_without_bounded_by_errors() {
    let being = BeingDef {
        annotations: vec![ann("bounded_telos")],
        telos: Some(TelosDef {
            description: "serve users within scope".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: None,
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(
        errors.iter().any(|e| format!("{e}").contains("bounded_by")),
        "expected bounded_by error, got: {:?}", errors
    );
}

// 11. bounded_telos_with_bounded_by_ok
#[test]
fn bounded_telos_with_bounded_by_ok() {
    let being = BeingDef {
        annotations: vec![ann("bounded_telos")],
        telos: Some(TelosDef {
            description: "serve users within defined scope".to_string(),
            fitness_fn: None,
            modifiable_by: None,
            bounded_by: Some("OperationalScope".to_string()),
            sign: None,
            span: Span::synthetic(),
        }),
        ..base_being()
    };
    let errors = SafetyChecker::check(&make_module(being));
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// 12. non_autopoietic_being_no_safety_errors
#[test]
fn non_autopoietic_being_no_safety_errors() {
    let errors = SafetyChecker::check(&make_module(base_being()));
    assert!(errors.is_empty(), "expected no errors for plain being, got: {:?}", errors);
}
