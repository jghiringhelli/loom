//! M131–M135: TelosFunction codegen + checker tests.
//!
//! Covers:
//!   M131 — {Name}Metric trait + {Name}Evaluation struct
//!   M132 — TelosMetricFn type alias from measured_by
//!   M133 — {Name}ConvergenceTracker with populated thresholds
//!   M134 — {Name}SignalAttention with guide-axis weights
//!   M135 — TelosFunctionChecker: completeness + threshold coherence

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn compile_telos_emit(src: &str) -> String {
    use loom::codegen::rust::RustEmitter;
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

fn check_telos(src: &str) -> Vec<String> {
    use loom::checker::{LoomChecker, TelosFunctionChecker};
    let module = parse_module(src);
    TelosFunctionChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect()
}

// ── Minimal valid telos_function ──────────────────────────────────────────────

#[test]
fn minimal_valid_telos_function_emits() {
    let out = compile_telos_emit(
        r#"
module T
telos_function pnl_alignment
  statement: "converge risk-adjusted PnL toward equilibrium"
  measured_by: "BeingState -> Float"
  guides: [signal_attention, experiment_selection]
end
end
"#,
    );
    assert!(
        out.contains("PnlAlignmentMetric"),
        "should emit trait: {}",
        out
    );
    assert!(
        out.contains("PnlAlignmentEvaluation"),
        "should emit evaluation struct: {}",
        out
    );
    assert!(
        out.contains("PnlAlignmentConvergenceTracker"),
        "should emit tracker: {}",
        out
    );
    assert!(
        out.contains("PnlAlignmentSignalAttention"),
        "should emit signal attention: {}",
        out
    );
}

// ── M131: Metric trait ────────────────────────────────────────────────────────

#[test]
fn telos_metric_trait_has_score_converged_degraded() {
    let out = compile_telos_emit(
        r#"
module T
telos_function risk_balance
  measured_by: "State -> Float"
  guides: [allocation]
end
end
"#,
    );
    assert!(
        out.contains("fn score(&self) -> f64"),
        "missing score method: {}",
        out
    );
    assert!(
        out.contains("fn converged(&self) -> bool"),
        "missing converged method: {}",
        out
    );
    assert!(
        out.contains("fn degraded(&self) -> bool"),
        "missing degraded method: {}",
        out
    );
}

#[test]
fn telos_evaluation_struct_has_required_fields() {
    let out = compile_telos_emit(
        r#"
module T
telos_function health_score
  measured_by: "Vitals -> Float"
  guides: [repair_selection]
end
end
"#,
    );
    assert!(
        out.contains("pub score: f64"),
        "missing score field: {}",
        out
    );
    assert!(
        out.contains("pub converged: bool"),
        "missing converged field: {}",
        out
    );
    assert!(
        out.contains("pub degraded: bool"),
        "missing degraded field: {}",
        out
    );
    assert!(
        out.contains("pub timestamp: u64"),
        "missing timestamp field: {}",
        out
    );
}

// ── M132: TelosMetricFn type alias ────────────────────────────────────────────

#[test]
fn metric_fn_alias_emitted_when_measured_by_declared() {
    let out = compile_telos_emit(
        r#"
module T
telos_function coherence_drive
  measured_by: "BeingState -> SignalSet -> Float"
  guides: [signal_attention]
end
end
"#,
    );
    assert!(
        out.contains("CoherenceDriveMetricFn"),
        "should emit type alias: {}",
        out
    );
    assert!(
        out.contains("Box<dyn Fn("),
        "should be a fn pointer type: {}",
        out
    );
    assert!(out.contains("-> f64>"), "Float should map to f64: {}", out);
}

#[test]
fn metric_fn_alias_maps_primitive_types() {
    let out = compile_telos_emit(
        r#"
module T
telos_function simple_score
  measured_by: "Int -> Bool -> Float"
  guides: [selection]
end
end
"#,
    );
    // Int -> i64, Bool -> bool, Float -> f64
    assert!(out.contains("i64"), "Int should map to i64: {}", out);
    assert!(out.contains("bool"), "Bool should map to bool: {}", out);
    assert!(out.contains("f64"), "Float should map to f64: {}", out);
}

#[test]
fn metric_fn_alias_not_emitted_when_measured_by_absent() {
    let out = compile_telos_emit(
        r#"
module T
telos_function bare_telos
  guides: [something]
end
end
"#,
    );
    assert!(
        !out.contains("MetricFn"),
        "no alias without measured_by: {}",
        out
    );
}

// ── M133: ConvergenceTracker with thresholds ──────────────────────────────────

#[test]
fn convergence_tracker_uses_declared_thresholds() {
    let out = compile_telos_emit(
        r#"
module T
telos_function growth_telos
  measured_by: "SystemState -> Float"
  guides: [resource_allocation]
  thresholds:
    convergence: 0.85
    divergence: 0.30
    warning: 0.55
    propagation: 0.90
  end
end
end
"#,
    );
    assert!(
        out.contains("0.8500_f64"),
        "should embed convergence constant: {}",
        out
    );
    assert!(
        out.contains("0.3000_f64"),
        "should embed divergence constant: {}",
        out
    );
    assert!(
        out.contains("Some(0.5500_f64)"),
        "should embed warning: {}",
        out
    );
    assert!(
        out.contains("Some(0.9000_f64)"),
        "should embed propagation: {}",
        out
    );
}

#[test]
fn convergence_tracker_has_is_converging_method() {
    let out = compile_telos_emit(
        r#"
module T
telos_function coherence
  measured_by: "S -> Float"
  guides: [signal_attention]
end
end
"#,
    );
    assert!(
        out.contains("fn is_converging(&self"),
        "missing is_converging: {}",
        out
    );
    assert!(
        out.contains("fn is_degraded(&self"),
        "missing is_degraded: {}",
        out
    );
    assert!(
        out.contains("fn eligible_for_propagation(&self)"),
        "missing propagation check: {}",
        out
    );
}

#[test]
fn convergence_tracker_without_thresholds_emits_todo_stubs() {
    let out = compile_telos_emit(
        r#"
module T
telos_function incomplete_telos
  measured_by: "X -> Float"
  guides: [axis_one]
end
end
"#,
    );
    assert!(
        out.contains("TODO"),
        "stubs should emit TODO hints: {}",
        out
    );
}

// ── M134: SignalAttention ─────────────────────────────────────────────────────

#[test]
fn signal_attention_has_amplify_attenuate_weight() {
    let out = compile_telos_emit(
        r#"
module T
telos_function market_telos
  measured_by: "Market -> Float"
  guides: [signal_attention, experiment_selection, resource_allocation]
end
end
"#,
    );
    assert!(out.contains("fn amplify("), "missing amplify: {}", out);
    assert!(out.contains("fn attenuate("), "missing attenuate: {}", out);
    assert!(out.contains("fn weight("), "missing weight: {}", out);
}

#[test]
fn signal_attention_inserts_declared_guide_axes() {
    let out = compile_telos_emit(
        r#"
module T
telos_function pnl_telos
  measured_by: "Portfolio -> Float"
  guides: [signal_attention, risk_selection]
end
end
"#,
    );
    assert!(
        out.contains("\"signal_attention\""),
        "should insert signal_attention axis: {}",
        out
    );
    assert!(
        out.contains("\"risk_selection\""),
        "should insert risk_selection axis: {}",
        out
    );
}

// ── M134–M135: Guide-axis integration hints ───────────────────────────────────

#[test]
fn guide_axis_hints_emitted_per_axis() {
    let out = compile_telos_emit(
        r#"
module T
telos_function learning_telos
  measured_by: "Experience -> Float"
  guides: [experiment_selection, signal_attention]
end
end
"#,
    );
    assert!(
        out.contains("LOOM[telos:guide]"),
        "missing guide annotation: {}",
        out
    );
    assert!(
        out.contains("experiment_selection"),
        "missing experiment_selection axis: {}",
        out
    );
    assert!(
        out.contains("signal_attention"),
        "missing signal_attention axis: {}",
        out
    );
}

// ── M135: TelosFunctionChecker — completeness ─────────────────────────────────

#[test]
fn checker_rejects_telos_without_measured_by() {
    let errors = check_telos(
        r#"
module T
telos_function incomplete
  statement: "some goal"
  guides: [signal_attention]
end
end
"#,
    );
    assert!(!errors.is_empty(), "should error on missing measured_by");
    assert!(
        errors.iter().any(|e| e.contains("measured_by")),
        "error should mention measured_by: {:?}",
        errors
    );
}

#[test]
fn checker_rejects_telos_without_guides() {
    let errors = check_telos(
        r#"
module T
telos_function guideless
  measured_by: "S -> Float"
end
end
"#,
    );
    assert!(!errors.is_empty(), "should error on empty guides");
    assert!(
        errors.iter().any(|e| e.contains("guides")),
        "error should mention guides: {:?}",
        errors
    );
}

#[test]
fn checker_accepts_fully_specified_telos() {
    let errors = check_telos(
        r#"
module T
telos_function complete_telos
  measured_by: "State -> Float"
  guides: [signal_attention]
end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "fully specified telos should pass: {:?}",
        errors
    );
}

// ── M135: TelosFunctionChecker — threshold coherence ─────────────────────────

#[test]
fn checker_rejects_convergence_below_divergence() {
    let errors = check_telos(
        r#"
module T
telos_function inverted
  measured_by: "S -> Float"
  guides: [signal_attention]
  thresholds:
    convergence: 0.20
    divergence: 0.80
  end
end
end
"#,
    );
    assert!(!errors.is_empty(), "inverted thresholds should be rejected");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("convergence") && e.contains("divergence")),
        "error should mention convergence and divergence: {:?}",
        errors
    );
}

#[test]
fn checker_rejects_warning_outside_bounds() {
    let errors = check_telos(
        r#"
module T
telos_function bad_warning
  measured_by: "S -> Float"
  guides: [signal_attention]
  thresholds:
    convergence: 0.80
    divergence: 0.20
    warning: 0.90
  end
end
end
"#,
    );
    // warning 0.90 >= convergence 0.80 — invalid
    assert!(
        !errors.is_empty(),
        "warning above convergence should be rejected"
    );
    assert!(
        errors.iter().any(|e| e.contains("warning")),
        "error should mention warning: {:?}",
        errors
    );
}

#[test]
fn checker_rejects_propagation_below_convergence() {
    let errors = check_telos(
        r#"
module T
telos_function premature_propagation
  measured_by: "S -> Float"
  guides: [signal_attention]
  thresholds:
    convergence: 0.80
    divergence: 0.20
    propagation: 0.60
  end
end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "propagation below convergence should be rejected"
    );
    assert!(
        errors.iter().any(|e| e.contains("propagation")),
        "error should mention propagation: {:?}",
        errors
    );
}

#[test]
fn checker_accepts_valid_thresholds() {
    let errors = check_telos(
        r#"
module T
telos_function valid_thresholds
  measured_by: "S -> Float"
  guides: [signal_attention]
  thresholds:
    convergence: 0.80
    divergence: 0.20
    warning: 0.50
    propagation: 0.90
  end
end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid thresholds should pass: {:?}",
        errors
    );
}

// ── Header and statement ──────────────────────────────────────────────────────

#[test]
fn telos_header_includes_statement_and_bounded_by() {
    let out = compile_telos_emit(
        r#"
module T
telos_function equilibrium_telos
  statement: "maintain systemic equilibrium across all subsystems"
  measured_by: "SystemState -> Float"
  guides: [resource_allocation]
end
end
"#,
    );
    assert!(
        out.contains("maintain systemic equilibrium"),
        "statement should appear in header: {}",
        out
    );
    assert!(
        out.contains("LOOM[telos_fn]"),
        "should include LOOM marker: {}",
        out
    );
}

#[test]
fn pascal_case_conversion_works_for_snake_name() {
    let out = compile_telos_emit(
        r#"
module T
telos_function pnl_risk_adjusted_score
  measured_by: "Portfolio -> Float"
  guides: [signal_attention]
end
end
"#,
    );
    // snake_case → PascalCase
    assert!(
        out.contains("PnlRiskAdjustedScore"),
        "should convert to PascalCase: {}",
        out
    );
}
