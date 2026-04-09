//! ALX convergence tests — per-step S_realized measurement.
//!
//! Each ALX experiment gets a convergence trace showing S_realized at every
//! pipeline stage. Tests assert:
//! - Monotonicity: S_realized never decreases (a stage cannot un-prove a claim)
//! - Final S_realized meets the experiment's gate threshold
//! - Contributing stages are non-empty (at least one stage did useful work)

use loom::alx::{compute_convergence_trace, ConvergenceTraceMode};
use std::fs;

const ALX_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/experiments/alx");

fn read_alx(filename: &str) -> String {
    let path = format!("{}/{}", ALX_DIR, filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read ALX file {}: {}", path, e))
}

// ── ALX-1: feature matrix convergence ────────────────────────────────────────

#[test]
fn alx1_convergence_is_monotonic() {
    let source = read_alx("ALX-1-feature-matrix.loom");
    let trace = compute_convergence_trace(&source);
    trace.report("ALX-1 feature matrix");
    assert!(
        trace.is_monotonic,
        "ALX-1 convergence must be monotonically non-decreasing.\nSparkline: {}",
        trace.sparkline()
    );
}

#[test]
fn alx1_convergence_meets_gate_threshold() {
    let source = read_alx("ALX-1-feature-matrix.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.final_s >= 0.70,
        "ALX-1 final S_realized {:.4} is below 0.70 gate. Proved: {}/{}\nSparkline: {}",
        trace.final_s,
        trace.total_proved,
        trace.total_claims,
        trace.sparkline()
    );
}

// ── ALX-2: cross-feature coherence convergence ───────────────────────────────

#[test]
fn alx2_convergence_is_monotonic() {
    let source = read_alx("ALX-2-cross-feature.loom");
    let trace = compute_convergence_trace(&source);
    trace.report("ALX-2 cross-feature");
    assert!(
        trace.is_monotonic,
        "ALX-2 convergence must be monotonically non-decreasing.\nSparkline: {}",
        trace.sparkline()
    );
}

// ── ALX-3: self-description convergence (claim-level) ─────────────────────────
//
// ALX-3 contains a correctness_report block → claim-level mode.
// S_realized is the fraction of declared proved claims whose checker has fired.

#[test]
fn alx3_convergence_is_claim_level() {
    let source = read_alx("ALX-3-self-description.loom");
    let trace = compute_convergence_trace(&source);
    trace.report("ALX-3 self-description");
    assert_eq!(
        trace.mode,
        ConvergenceTraceMode::ClaimLevel,
        "ALX-3 must produce claim-level convergence (requires correctness_report block)"
    );
}

#[test]
fn alx3_convergence_is_monotonic() {
    let source = read_alx("ALX-3-self-description.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.is_monotonic,
        "ALX-3 claim-level convergence must be monotonically non-decreasing.\nSparkline: {}",
        trace.sparkline()
    );
}

#[test]
fn alx3_convergence_meets_gate_threshold() {
    let source = read_alx("ALX-3-self-description.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.final_s >= 0.70,
        "ALX-3 final S_realized {:.4} is below 0.70 gate. Proved: {}/{}\nSparkline: {}",
        trace.final_s,
        trace.total_proved,
        trace.total_claims,
        trace.sparkline()
    );
}

#[test]
fn alx3_convergence_has_contributing_stages() {
    let source = read_alx("ALX-3-self-description.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        !trace.contributing_stages.is_empty(),
        "ALX-3 convergence trace must have at least one contributing stage"
    );
}

// ── ALX-5: evolvable stress convergence ──────────────────────────────────────

#[test]
fn alx5_convergence_is_monotonic() {
    let source = read_alx("ALX-5-evolvable-stress.loom");
    let trace = compute_convergence_trace(&source);
    trace.report("ALX-5 evolvable stress");
    assert!(
        trace.is_monotonic,
        "ALX-5 convergence must be monotonically non-decreasing.\nSparkline: {}",
        trace.sparkline()
    );
}

#[test]
fn alx5_migration_stage_contributes() {
    let source = read_alx("ALX-5-evolvable-stress.loom");
    let trace = compute_convergence_trace(&source);
    // ALX-5 has migrations — the migration stage must be contributing.
    assert!(
        trace.contributing_stages.contains(&"migration"),
        "ALX-5 has migrations: the 'migration' stage must appear in contributing_stages.\nContributing: {:?}",
        trace.contributing_stages
    );
}

// ── ALX-6: biotrader maximum-coverage convergence ────────────────────────────

#[test]
fn alx6_convergence_is_claim_level() {
    let source = read_alx("ALX-6-biotrader.loom");
    let trace = compute_convergence_trace(&source);
    trace.report("ALX-6 biotrader");
    assert_eq!(
        trace.mode,
        ConvergenceTraceMode::ClaimLevel,
        "ALX-6 has a correctness_report: it must use claim-level convergence"
    );
}

#[test]
fn alx6_convergence_is_monotonic() {
    let source = read_alx("ALX-6-biotrader.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.is_monotonic,
        "ALX-6 convergence must be monotonically non-decreasing.\nSparkline: {}",
        trace.sparkline()
    );
}

#[test]
fn alx6_convergence_meets_0_90_gate() {
    let source = read_alx("ALX-6-biotrader.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.final_s >= 0.90,
        "ALX-6 final S_realized {:.4} < 0.90 gate. Proved: {}/{}\nSparkline: {}",
        trace.final_s, trace.total_proved, trace.total_claims, trace.sparkline()
    );
}

#[test]
fn alx6_signal_attention_stage_contributes() {
    let source = read_alx("ALX-6-biotrader.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.contributing_stages.contains(&"signal_attention"),
        "ALX-6 has signal_attention: that stage must appear in contributing_stages.\nContributing: {:?}",
        trace.contributing_stages
    );
}

#[test]
fn alx6_messaging_stage_contributes() {
    let source = read_alx("ALX-6-biotrader.loom");
    let trace = compute_convergence_trace(&source);
    assert!(
        trace.contributing_stages.contains(&"messaging"),
        "ALX-6 has messaging_primitive: that stage must appear in contributing_stages.\nContributing: {:?}",
        trace.contributing_stages
    );
}

// ── Cross-experiment property: all ALX convergence curves are monotonic ───────

#[test]
fn all_alx_convergence_curves_are_monotonic() {
    let experiments = [
        "ALX-1-feature-matrix.loom",
        "ALX-2-cross-feature.loom",
        "ALX-3-self-description.loom",
        "ALX-5-evolvable-stress.loom",
        "ALX-6-biotrader.loom",
    ];
    for filename in experiments {
        let source = read_alx(filename);
        let trace = compute_convergence_trace(&source);
        assert!(
            trace.is_monotonic,
            "{} convergence must be monotonically non-decreasing.\nSparkline: {}",
            filename,
            trace.sparkline()
        );
    }
}
