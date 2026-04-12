//! M188 tests — `classifier` item type: parse + AST + codegen scaffold.
//!
//! Classifier sits between pure regex (cheap but limited) and a full LLM
//! (expensive). BIOISO beings can retrain it on demand via M189 regulate triggers.

use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::codegen::rust::RustEmitter;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn emit(src: &str) -> String {
    let module = parse(src);
    RustEmitter::new().emit(&module)
}

// ── 1. minimal classifier (model defaults to regex) ──────────────────────────

#[test]
fn classifier_minimal_parses() {
    let src = r#"module Test
classifier SignalRouter
  model: regex
end
end"#;
    let module = parse(src);
    let item = module.items.iter().find_map(|i| {
        if let loom::ast::Item::Classifier(c) = i { Some(c) } else { None }
    });
    assert!(item.is_some(), "expected Item::Classifier in parsed module");
    let c = item.unwrap();
    assert_eq!(c.name, "SignalRouter");
    assert_eq!(c.model, "regex");
    assert!(c.retrain_trigger.is_none());
}

// ── 2. classifier with bert-tiny model ───────────────────────────────────────

#[test]
fn classifier_bert_tiny_parses() {
    let src = r#"module Test
classifier ToxicityGate
  model: bert-tiny
end
end"#;
    let module = parse(src);
    let c = module.items.iter().find_map(|i| {
        if let loom::ast::Item::Classifier(c) = i { Some(c) } else { None }
    }).unwrap();
    assert_eq!(c.name, "ToxicityGate");
    assert_eq!(c.model, "bert-tiny");
}

// ── 3. classifier with retrain_trigger ───────────────────────────────────────

#[test]
fn classifier_retrain_trigger_parses() {
    let src = r#"module Test
classifier AnomalyDetector
  model: mlp
  retrain_trigger: "accuracy < 0.85 over 1000 samples"
end
end"#;
    let module = parse(src);
    let c = module.items.iter().find_map(|i| {
        if let loom::ast::Item::Classifier(c) = i { Some(c) } else { None }
    }).unwrap();
    assert_eq!(c.model, "mlp");
    assert_eq!(
        c.retrain_trigger.as_deref(),
        Some("accuracy < 0.85 over 1000 samples")
    );
}

// ── 4. codegen emits LOOM[classifier:Name:model] marker ──────────────────────

#[test]
fn classifier_codegen_emits_loom_marker() {
    let out = emit(r#"module Test
classifier SignalRouter
  model: regex
end
end"#);
    assert!(
        out.contains("// LOOM[classifier:SignalRouter:regex]"),
        "expected LOOM marker, got:\n{out}"
    );
}

// ── 5. codegen emits trait and struct scaffold ────────────────────────────────

#[test]
fn classifier_codegen_emits_trait_and_struct() {
    let out = emit(r#"module Test
classifier ToxicityGate
  model: bert-tiny
end
end"#);
    assert!(out.contains("pub trait ToxicityGateClassify"), "expected trait: {out}");
    assert!(out.contains("pub struct ToxicityGateClassifier"), "expected struct: {out}");
    assert!(out.contains("fn predict"), "expected predict method: {out}");
}

// ── 6. codegen emits retrain_trigger comment ─────────────────────────────────

#[test]
fn classifier_codegen_emits_retrain_trigger() {
    let out = emit(r#"module Test
classifier AnomalyDetector
  model: tfidf
  retrain_trigger: "accuracy < 0.85 over 1000 samples"
end
end"#);
    assert!(
        out.contains("// retrain_trigger: accuracy < 0.85 over 1000 samples"),
        "expected retrain comment: {out}"
    );
}

// ── 7. multiple classifiers in one module ────────────────────────────────────

#[test]
fn classifier_multiple_in_module() {
    let src = r#"module Test
classifier GateA
  model: regex
end
classifier GateB
  model: bert-tiny
end
end"#;
    let module = parse(src);
    let classifiers: Vec<_> = module.items.iter().filter_map(|i| {
        if let loom::ast::Item::Classifier(c) = i { Some(c) } else { None }
    }).collect();
    assert_eq!(classifiers.len(), 2, "expected 2 classifiers");
    assert_eq!(classifiers[0].name, "GateA");
    assert_eq!(classifiers[1].name, "GateB");
}

// ── 8. classifier name is pascal-cased in emitted code ───────────────────────

#[test]
fn classifier_name_pascal_cased_in_emit() {
    let out = emit(r#"module Test
classifier climate_risk
  model: regex
end
end"#);
    // Pascal-cased: ClimateRisk
    assert!(out.contains("ClimateRisk"), "expected PascalCase name: {out}");
}
