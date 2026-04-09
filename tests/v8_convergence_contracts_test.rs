//! V8: Convergence + Contract scaffold tests.
//!
//! Verifies that Loom's correctness contract emitters produce real, runnable
//! Rust scaffolding — not merely audit comments:
//!
//! - Termination: const bound + TerminationGuard struct + tick() method
//! - Timing safety: subtle::ConstantTimeEq wrapper fns when constant_time: true
//! - Telos: threshold consts + ConvergenceState enum + convergence_state() method
//! - Telos: embedded TLA+ spec string as a const (for external TLC verification)

// ── helpers ──────────────────────────────────────────────────────────────────

fn compile(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile failed: {:?}", e))
}

// ════════════════════════════════════════════════════════════════════════════
// TERMINATION
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn termination_emits_bound_constant() {
    let src = r#"
module M
fn findItem @pure :: List -> Option
  termination: list_length
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("FINDITEM_TERMINATION_BOUND"),
        "Expected termination bound const, got:\n{}",
        rust
    );
}

#[test]
fn termination_emits_guard_struct() {
    let src = r#"
module M
fn gcd @pure :: Int -> Int -> Int
  termination: smaller_arg
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("GcdTerminationGuard"),
        "Expected TerminationGuard struct, got:\n{}",
        rust
    );
}

#[test]
fn termination_guard_has_tick_method() {
    let src = r#"
module M
fn reduce @pure :: List -> List
  termination: length
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("pub fn tick(&mut self)"),
        "Expected tick() method on guard, got:\n{}",
        rust
    );
}

#[test]
fn termination_guard_panics_on_exceed() {
    let src = r#"
module M
fn solve @pure :: Problem -> Solution
  termination: problem_size
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("LOOM termination violation"),
        "Expected panic message in guard, got:\n{}",
        rust
    );
}

#[test]
fn termination_guard_tracks_iterations() {
    let src = r#"
module M
fn converge @pure :: Float -> Float
  termination: distance_to_zero
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("pub fn iterations(&self) -> usize"),
        "Expected iterations() method, got:\n{}",
        rust
    );
}

#[test]
fn termination_emits_audit_comment() {
    let src = r#"
module M
fn sort @pure :: List -> List
  termination: unsorted_pairs
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("LOOM[contract:Termination]"),
        "Expected termination audit comment, got:\n{}",
        rust
    );
    assert!(
        rust.contains("unsorted_pairs"),
        "Expected metric name in audit, got:\n{}",
        rust
    );
}

// ════════════════════════════════════════════════════════════════════════════
// TIMING SAFETY
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn timing_safety_constant_time_emits_ct_eq_wrapper() {
    let src = r#"
module M
fn compareTokens :: Token -> Token -> Bool
  timing_safety:
    constant_time: true
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("ct_eq") && rust.contains("subtle"),
        "Expected subtle ConstantTimeEq wrapper, got:\n{}",
        rust
    );
}

#[test]
fn timing_safety_emits_cfg_feature_subtle() {
    let src = r#"
module M
fn verifyMac :: Mac -> Mac -> Bool
  timing_safety:
    constant_time: true
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("#[cfg(feature = \"subtle\")]"),
        "Expected #[cfg(feature = \"subtle\")] guard, got:\n{}",
        rust
    );
}

#[test]
fn timing_safety_declared_only_emits_audit_comment() {
    let src = r#"
module M
fn decrypt :: Bytes -> Bytes
  timing_safety:
    constant_time: false
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("LOOM[contract:TimingSafety]") && rust.contains("declared_only"),
        "Expected timing safety audit comment for declared_only, got:\n{}",
        rust
    );
}

// ════════════════════════════════════════════════════════════════════════════
// TELOS CONVERGENCE
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn telos_thresholds_emit_constants() {
    let src = r#"
module Optimizer
being Agent
  telos: "maximize_reward"
    thresholds:
      convergence: 0.85
      warning: 0.50
      divergence: 0.20
    end
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("AGENT_CONVERGENCE_THRESHOLD"),
        "Expected convergence threshold const, got:\n{}",
        rust
    );
    assert!(
        rust.contains("AGENT_WARNING_THRESHOLD"),
        "Expected warning threshold const, got:\n{}",
        rust
    );
    assert!(
        rust.contains("AGENT_DIVERGENCE_THRESHOLD"),
        "Expected divergence threshold const, got:\n{}",
        rust
    );
}

#[test]
fn telos_thresholds_emit_convergence_state_enum() {
    let src = r#"
module Sys
being Robot
  telos: "reach_goal"
    thresholds:
      convergence: 0.90
      divergence: 0.10
    end
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("enum RobotConvergenceState"),
        "Expected ConvergenceState enum, got:\n{}",
        rust
    );
    assert!(
        rust.contains("Converging") && rust.contains("Warning") && rust.contains("Diverging"),
        "Expected all convergence state variants, got:\n{}",
        rust
    );
}

#[test]
fn telos_thresholds_emit_convergence_state_method() {
    let src = r#"
module Sys
being Controller
  telos: "stabilize"
    thresholds:
      convergence: 0.80
      divergence: 0.20
    end
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("pub fn convergence_state"),
        "Expected convergence_state() method, got:\n{}",
        rust
    );
}

#[test]
fn telos_emits_tla_plus_spec_string() {
    let src = r#"
module Sys
being Learner
  telos: "minimize_loss"
    thresholds:
      convergence: 0.95
      divergence: 0.05
    end
  end
end
end
"#;
    let rust = compile(src);
    assert!(
        rust.contains("LEARNER_TLA_SPEC"),
        "Expected TLA+ spec const, got:\n{}",
        rust
    );
    assert!(
        rust.contains("ConvergenceProperty"),
        "Expected TLA+ ConvergenceProperty liveness property, got:\n{}",
        rust
    );
    assert!(
        rust.contains("TelosConverged"),
        "Expected TLA+ TelosConverged predicate, got:\n{}",
        rust
    );
}

#[test]
fn telos_without_thresholds_still_compiles() {
    let src = r#"
module M
being SimpleAgent
  telos: "do_good"
  end
end
end
"#;
    let rust = compile(src);
    // Should compile fine without threshold constants
    assert!(rust.contains("Being: SimpleAgent") || rust.contains("SimpleAgent"));
}
