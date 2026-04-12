//! M191 — TelosConsistencyChecker: classifier retrain_trigger requires telos metric.
//!
//! Rule: if a being has a regulate block with `trigger: classifier:Name` and the
//! named classifier has a `retrain_trigger`, then the being must declare
//! `telos: measured_by: <metric>`. Without a measurable metric, classifier
//! retraining cannot be proven to converge.

use loom::checker::teleos;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── Violation cases ───────────────────────────────────────────────────────────

/// Being with classifier-triggered regulate, classifier has retrain_trigger,
/// being has NO telos at all → M191 error (in addition to the existing "no telos" error).
#[test]
fn m191_no_telos_with_retrain_classifier_is_error() {
    let src = r#"module Bio
classifier AnomalyDetector
  model: mlp
  retrain_trigger: "accuracy < 0.85 over 1000 samples"
end

being Sensor
  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    assert!(
        result.is_err(),
        "expected M191 error when being has no telos but classifier has retrain_trigger"
    );
    // M191 error may be anywhere in the list (the "no telos" error also fires first)
    let errs = result.unwrap_err();
    let has_m191 = errs.iter().any(|e| e.to_string().contains("M191"));
    assert!(
        has_m191,
        "expected at least one M191 error in: {:?}",
        errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
    );
    let has_classifier = errs.iter().any(|e| e.to_string().contains("AnomalyDetector"));
    assert!(
        has_classifier,
        "error must name the classifier; errors: {:?}",
        errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
    );
}

/// Being has telos but no measured_by (metric is None) → M191 error.
#[test]
fn m191_telos_without_metric_with_retrain_classifier_is_error() {
    let src = r#"module Bio
classifier RiskClassifier
  model: mlp
  retrain_trigger: "f1 < 0.75"
end

being Host
  telos: survive the infection
  end

  regulate: infection_risk
    target: 0.0
    bounds: 0.0 .. 0.5
    trigger: classifier: RiskClassifier
    action: alert
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    assert!(
        result.is_err(),
        "expected M191 error when telos has no metric"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("M191")),
        "M191 second test: {:?}", errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
    );
}

// ── Passing cases ─────────────────────────────────────────────────────────────

/// Being with classifier-triggered regulate, classifier has retrain_trigger,
/// being declares telos with measured_by → no M191 error.
#[test]
fn m191_telos_with_metric_satisfies_check() {
    let src = r#"module Bio
classifier AnomalyDetector
  model: mlp
  retrain_trigger: "accuracy < 0.85 over 1000 samples"
end

being Sensor
  telos: stay within safe temperature range
    measured_by: temperature_deviation
  end

  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    assert!(
        result.is_ok(),
        "expected no M191 error when telos has metric; got: {:?}", result.err()
    );
}

/// Classifier WITHOUT retrain_trigger → no M191 error even if telos has no metric.
/// Being still needs a telos to pass the existing "no telos" check.
#[test]
fn m191_no_retrain_trigger_no_error_without_metric() {
    let src = r#"module Bio
classifier AnomalyDetector
  model: regex
end

being Sensor
  telos: maintain safe temperature
  end

  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: AnomalyDetector
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    // No M191 error because classifier has no retrain_trigger.
    // If there's an error it must not be M191.
    if let Err(errs) = &result {
        let m191_errs: Vec<_> = errs.iter().filter(|e| e.to_string().contains("M191")).collect();
        assert!(
            m191_errs.is_empty(),
            "no retrain_trigger means no convergence risk — M191 must not fire; got: {:?}",
            m191_errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        );
    }
}

/// Non-classifier trigger → no M191 error (M191 only applies to classifier: triggers).
/// Being has telos so the pre-existing "no telos" check does not fire.
#[test]
fn m191_non_classifier_trigger_not_affected() {
    let src = r#"module Bio
being Cell
  telos: maintain energy homeostasis
  end

  regulate: atp
    target: 100.0
    bounds: 50.0 .. 200.0
    trigger: energy_low
    action: produce_atp
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    if let Err(errs) = &result {
        let m191_errs: Vec<_> = errs.iter().filter(|e| e.to_string().contains("M191")).collect();
        assert!(
            m191_errs.is_empty(),
            "non-classifier trigger must not trigger M191; got: {:?}",
            m191_errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        );
    }
}

/// Multiple regulate blocks: only the classifier-triggered one without metric errors.
#[test]
fn m191_multiple_regulate_blocks_only_classifier_one_errors() {
    let src = r#"module Bio
classifier DriftDetector
  model: tfidf
  retrain_trigger: "drift_score > 0.3"
end

being Monitor
  telos: maintain system health
  end

  regulate: cpu
    target: 50.0
    bounds: 0.0 .. 100.0
    trigger: high_load
    action: throttle
  end

  regulate: prediction_quality
    target: 0.9
    bounds: 0.7 .. 1.0
    trigger: classifier: DriftDetector
    action: retrain_model
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    assert!(
        result.is_err(),
        "M191 must fire because the classifier regulate block has no telos metric"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("M191")),
        "expected M191; got: {:?}", errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
    );
    assert!(
        errs.iter().any(|e| e.to_string().contains("DriftDetector")),
        "error must name DriftDetector"
    );
}

/// Multiple regulate blocks: classifier-triggered one passes because telos has metric.
#[test]
fn m191_multiple_regulate_blocks_with_metric_passes() {
    let src = r#"module Bio
classifier DriftDetector
  model: tfidf
  retrain_trigger: "drift_score > 0.3"
end

being Monitor
  telos: maintain prediction accuracy
    measured_by: prediction_quality_score
  end

  regulate: cpu
    target: 50.0
    bounds: 0.0 .. 100.0
    trigger: high_load
    action: throttle
  end

  regulate: prediction_quality
    target: 0.9
    bounds: 0.7 .. 1.0
    trigger: classifier: DriftDetector
    action: retrain_model
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    assert!(
        result.is_ok(),
        "M191 must not fire when telos has metric; got: {:?}", result.err()
    );
}

/// Undefined classifier reference → no M191 error (caught by reference checker, not teleos).
/// Being has telos to avoid triggering the existing "no telos" check.
#[test]
fn m191_undefined_classifier_reference_not_m191_error() {
    let src = r#"module Bio
being Sensor
  telos: keep temperature stable
  end

  regulate: temperature
    target: 37.0
    bounds: 35.0 .. 39.0
    trigger: classifier: Nonexistent
    action: cool_down
  end
end
end"#;
    let module = parse(src);
    let result = teleos::check(&module);
    // M191 skips unknown classifiers; they are the reference checker's domain
    if let Err(errs) = &result {
        let m191_errs: Vec<_> = errs.iter().filter(|e| e.to_string().contains("M191")).collect();
        assert!(
            m191_errs.is_empty(),
            "undefined classifier should not trigger M191; got: {:?}",
            m191_errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        );
    }
}
