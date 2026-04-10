//! Tests for M58 — Temporal Logic Properties
//!
//! Temporal properties express invariants over time:
//! - `always:` — property holds in every reachable state
//! - `eventually:` — property holds in some future state
//! - `never:` — property never holds in any reachable state
//! - `precedes:` — one state always occurs before another

/// Helper: compile and expect success.
fn compile_ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|errs| {
        panic!(
            "expected compilation to succeed but got errors:\n{:#?}",
            errs
        )
    })
}

/// Helper: compile and expect failure.
fn compile_err(src: &str) -> Vec<loom::LoomError> {
    loom::compile(src).expect_err("expected compilation to fail but it succeeded")
}

// ── 1. Temporal block parses ────────────────────────────────────────────────

#[test]
fn temporal_block_with_always_parses() {
    let src = r#"
module PaymentTest

lifecycle Payment :: Pending -> Settled -> Archived

temporal PaymentRules
  always: Payment != Archived or Payment = Archived
end

end
"#;
    let _out = compile_ok(src);
}

#[test]
fn temporal_block_with_eventually_parses() {
    let src = r#"
module LifecycleTest

lifecycle Order :: Created -> Shipped -> Delivered

temporal OrderRules
  eventually: Order reaches Delivered
end

end
"#;
    let _out = compile_ok(src);
}

#[test]
fn temporal_block_with_never_parses() {
    let src = r#"
module SafetyTest

lifecycle Connection :: Open -> Authenticated -> Closed

temporal ConnectionSafety
  never: Closed transitions to Open
end

end
"#;
    let _out = compile_ok(src);
}

#[test]
fn temporal_block_with_precedes_parses() {
    let src = r#"
module OrderingTest

lifecycle Auth :: Anonymous -> Identified -> Authorized

temporal AuthOrdering
  precedes: Identified before Authorized
end

end
"#;
    let _out = compile_ok(src);
}

// ── 2. Temporal checker validates properties ────────────────────────────────

#[test]
fn never_rejects_invalid_backward_transition() {
    let src = r#"
module BadTransition

lifecycle Payment :: Pending -> Settled -> Archived

temporal PaymentRules
  never: Settled transitions to Pending
end

fn regress :: Payment<Settled> -> Payment<Pending>
  todo
end

end
"#;
    let errors = compile_err(src);
    let has_temporal = errors.iter().any(|e| {
        matches!(e, loom::LoomError::TypeError { msg, .. } if msg.contains("temporal") || msg.contains("transition"))
    });
    assert!(
        has_temporal,
        "expected temporal violation error, got: {:?}",
        errors
    );
}

#[test]
fn precedes_rejects_skipped_state() {
    let src = r#"
module SkipState

lifecycle Auth :: Anonymous -> Identified -> Authorized

temporal AuthRules
  precedes: Identified before Authorized
end

fn skip_auth :: Auth<Anonymous> -> Auth<Authorized>
  todo
end

end
"#;
    let errors = compile_err(src);
    let has_temporal = errors.iter().any(|e| {
        matches!(e, loom::LoomError::TypeError { msg, .. }
            if msg.contains("temporal") || msg.contains("precedes") || msg.contains("transition"))
    });
    assert!(
        has_temporal,
        "expected precedes violation, got: {:?}",
        errors
    );
}

// ── 3. Multiple temporal properties in one block ────────────────────────────

#[test]
fn multiple_temporal_properties_compile() {
    let src = r#"
module MultiProp

lifecycle Task :: Queued -> Running -> Done

temporal TaskRules
  always: Task != Done or Task = Done
  eventually: Task reaches Done
  never: Done transitions to Queued
end

end
"#;
    let _out = compile_ok(src);
}

// ── 4. Temporal blocks emit documentation ───────────────────────────────────

#[test]
fn temporal_block_emits_rust_comment() {
    let src = r#"
module DocTest

lifecycle Job :: Idle -> Active -> Complete

temporal JobInvariants
  always: Job != Complete or Job = Complete
end

end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("temporal") || out.contains("invariant") || out.contains("JobInvariants"),
        "temporal properties should appear in emitted code as documentation:\n{}",
        out
    );
}
