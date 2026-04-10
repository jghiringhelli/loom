//! M43 tests — Ecosystem block: multi-being composition with session-typed signals.

use loom::ast::{EcosystemDef, SignalDef, Span};
use loom::checker::check_teleos;
use loom::codegen::openapi::OpenApiEmitter;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── 1. ecosystem_parses_with_members_and_signals ─────────────────────────────

#[test]
fn ecosystem_parses_with_members_and_signals() {
    let src = r#"module Forest
being Tree
  telos: "photosynthesis"
  end
end
being Fungus
  telos: "decomposition"
  end
end
ecosystem ForestEcosystem
  members: [Tree, Fungus]
  signal NutrientFlow from Tree to Fungus
    payload: Float
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.ecosystem_defs.len(), 1);
    let eco = &module.ecosystem_defs[0];
    assert_eq!(eco.name, "ForestEcosystem");
    assert_eq!(eco.members, vec!["Tree", "Fungus"]);
    assert_eq!(eco.signals.len(), 1);
    assert_eq!(eco.signals[0].name, "NutrientFlow");
    assert_eq!(eco.signals[0].from, "Tree");
    assert_eq!(eco.signals[0].to, "Fungus");
}

// ── 2. ecosystem_without_signals_fails ───────────────────────────────────────

#[test]
fn ecosystem_without_signals_fails() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "EmptyEco".to_string(),
            describe: None,
            members: vec!["Alpha".to_string(), "Beta".to_string()],
            signals: vec![], // no signals!
            telos: None,
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = check_teleos(&module);
    assert!(
        result.is_err(),
        "expected error for ecosystem without signals"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("no signals")),
        "expected 'no signals' error, got: {:?}",
        errs
    );
}

// ── 3. ecosystem_signal_unknown_being_fails ───────────────────────────────────

#[test]
fn ecosystem_signal_unknown_being_fails() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "BadEco".to_string(),
            describe: None,
            members: vec![], // no members declared, so beings are unknown
            signals: vec![SignalDef {
                name: "MySignal".to_string(),
                from: "GhostA".to_string(),
                to: "GhostB".to_string(),
                payload: "Int".to_string(),
                span: Span::synthetic(),
            }],
            telos: None,
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = check_teleos(&module);
    assert!(
        result.is_err(),
        "expected error for unknown being in signal"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("GhostA") || e.to_string().contains("GhostB")),
        "expected error mentioning unknown being, got: {:?}",
        errs
    );
}

// ── 4. ecosystem_with_telos_parses ────────────────────────────────────────────

#[test]
fn ecosystem_with_telos_parses() {
    let src = r#"module Eco
being Producer
  telos: "produce energy"
  end
end
being Consumer
  telos: "consume energy"
  end
end
ecosystem EnergyCycle
  members: [Producer, Consumer]
  signal EnergyFlow from Producer to Consumer
    payload: Float
  end
  telos: "sustainable energy cycling"
end
end
"#;
    let module = parse(src);
    assert_eq!(module.ecosystem_defs.len(), 1);
    let eco = &module.ecosystem_defs[0];
    assert_eq!(eco.telos.as_deref(), Some("sustainable energy cycling"));
    let result = check_teleos(&module);
    assert!(result.is_ok(), "expected no errors: {:?}", result);
}

// ── 5. ecosystem_multiple_signals_parse ──────────────────────────────────────

#[test]
fn ecosystem_multiple_signals_parse() {
    let src = r#"module Forest
being Tree
  telos: "photosynthesize"
  end
end
being Fungus
  telos: "decompose"
  end
end
being Bacterium
  telos: "mineralize"
  end
end
ecosystem ForestEcosystem
  members: [Tree, Fungus, Bacterium]
  signal NutrientFlow from Tree to Fungus
    payload: Float
  end
  signal WasteSignal from Fungus to Bacterium
    payload: String
  end
  signal MineralSignal from Bacterium to Tree
    payload: Int
  end
  telos: "sustainable nutrient cycling"
end
end
"#;
    let module = parse(src);
    assert_eq!(module.ecosystem_defs.len(), 1);
    let eco = &module.ecosystem_defs[0];
    assert_eq!(eco.signals.len(), 3);
    assert_eq!(eco.signals[0].name, "NutrientFlow");
    assert_eq!(eco.signals[1].name, "WasteSignal");
    assert_eq!(eco.signals[2].name, "MineralSignal");
}

// ── 6. rust_emit_ecosystem_has_module ─────────────────────────────────────────

#[test]
fn rust_emit_ecosystem_has_module() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "ForestEcosystem".to_string(),
            describe: None,
            members: vec!["Tree".to_string(), "Fungus".to_string()],
            signals: vec![SignalDef {
                name: "NutrientFlow".to_string(),
                from: "Tree".to_string(),
                to: "Fungus".to_string(),
                payload: "Float".to_string(),
                span: Span::synthetic(),
            }],
            telos: Some("sustainable nutrient cycling".to_string()),
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = RustEmitter::new().emit(&module);
    assert!(
        out.contains("pub mod forest_ecosystem"),
        "expected pub mod in:\n{out}"
    );
}

// ── 7. rust_emit_ecosystem_has_signal_structs ─────────────────────────────────

#[test]
fn rust_emit_ecosystem_has_signal_structs() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "ForestEcosystem".to_string(),
            describe: None,
            members: vec!["Tree".to_string(), "Fungus".to_string()],
            signals: vec![
                SignalDef {
                    name: "NutrientFlow".to_string(),
                    from: "Tree".to_string(),
                    to: "Fungus".to_string(),
                    payload: "Float".to_string(),
                    span: Span::synthetic(),
                },
                SignalDef {
                    name: "WasteSignal".to_string(),
                    from: "Fungus".to_string(),
                    to: "Tree".to_string(),
                    payload: "String".to_string(),
                    span: Span::synthetic(),
                },
            ],
            telos: Some("nutrient cycling".to_string()),
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = RustEmitter::new().emit(&module);
    assert!(
        out.contains("pub struct NutrientFlow"),
        "expected NutrientFlow struct in:\n{out}"
    );
    assert!(
        out.contains("pub struct WasteSignal"),
        "expected WasteSignal struct in:\n{out}"
    );
}

// ── 8. rust_emit_ecosystem_has_coordinate_fn ─────────────────────────────────

#[test]
fn rust_emit_ecosystem_has_coordinate_fn() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "ForestEcosystem".to_string(),
            describe: None,
            members: vec!["Tree".to_string(), "Fungus".to_string()],
            signals: vec![SignalDef {
                name: "NutrientFlow".to_string(),
                from: "Tree".to_string(),
                to: "Fungus".to_string(),
                payload: "Float".to_string(),
                span: Span::synthetic(),
            }],
            telos: Some("sustainable cycling".to_string()),
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = RustEmitter::new().emit(&module);
    assert!(
        out.contains("fn coordinate"),
        "expected fn coordinate in:\n{out}"
    );
}

// ── 9. typescript_emit_ecosystem_has_namespace ────────────────────────────────

#[test]
fn typescript_emit_ecosystem_has_namespace() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "ForestEcosystem".to_string(),
            describe: None,
            members: vec!["Tree".to_string(), "Fungus".to_string()],
            signals: vec![SignalDef {
                name: "NutrientFlow".to_string(),
                from: "Tree".to_string(),
                to: "Fungus".to_string(),
                payload: "Float".to_string(),
                span: Span::synthetic(),
            }],
            telos: Some("sustainable cycling".to_string()),
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(
        out.contains("export namespace ForestEcosystem"),
        "expected export namespace in:\n{out}"
    );
}

// ── 10. openapi_emit_ecosystem_has_x_ecosystems ──────────────────────────────

#[test]
fn openapi_emit_ecosystem_has_x_ecosystems() {
    let module = loom::ast::Module {
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
        being_defs: vec![],
        ecosystem_defs: vec![EcosystemDef {
            name: "ForestEcosystem".to_string(),
            describe: None,
            members: vec!["Tree".to_string(), "Fungus".to_string()],
            signals: vec![SignalDef {
                name: "NutrientFlow".to_string(),
                from: "Tree".to_string(),
                to: "Fungus".to_string(),
                payload: "Float".to_string(),
                span: Span::synthetic(),
            }],
            telos: Some("sustainable cycling".to_string()),
            quorum_blocks: vec![],
            collective_telos_metric: None,
            tipping_points: Vec::new(),
            coevolution: false,
            coupling: None,
            span: Span::synthetic(),
        }],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = OpenApiEmitter::new().emit(&module);
    assert!(
        out.contains("x-ecosystems"),
        "expected x-ecosystems in:\n{out}"
    );
    assert!(
        out.contains("ForestEcosystem"),
        "expected ForestEcosystem in:\n{out}"
    );
}
