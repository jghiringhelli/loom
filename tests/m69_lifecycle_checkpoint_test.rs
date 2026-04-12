use loom::compile;

fn ok(src: &str) -> String {
    match compile(src) {
        Ok(out) => out,
        Err(e) => panic!("compile error: {:?}", e),
    }
}

// ── Parse tests ───────────────────────────────────────────────────────────────

#[test]
fn lifecycle_minimal_parses() {
    let out = ok("module M\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nend\n");
    assert!(out.contains("Connection"), "output:\n{}", out);
}

#[test]
fn lifecycle_with_checkpoint_parses() {
    // checkpoint: Name  requires: fn  on_fail: fn  end  end (lifecycle)  end (module)
    let out = ok(
        "module M\nlifecycle Connection :: Pending -> Active -> Closed\ncheckpoint: ReadyCheck\nrequires: is_ready\non_fail: abort_connection\nend\nend\nend\n",
    );
    assert!(out.contains("Connection"), "output:\n{}", out);
}

// ── Codegen tests ─────────────────────────────────────────────────────────────

#[test]
fn lifecycle_emits_loom_annotation() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(
        out.contains("// LOOM[lifecycle:Session]"),
        "expected LOOM annotation:\n{}",
        out
    );
}

#[test]
fn lifecycle_emits_state_marker_structs() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(out.contains("pub struct Init"), "output:\n{}", out);
    assert!(out.contains("pub struct Active"), "output:\n{}", out);
    assert!(out.contains("pub struct Done"), "output:\n{}", out);
}

#[test]
fn lifecycle_emits_state_enum() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(
        out.contains("pub enum SessionState"),
        "expected SessionState enum:\n{}",
        out
    );
}

#[test]
fn lifecycle_state_enum_contains_all_variants() {
    let out = ok("module M\nlifecycle Order :: Pending -> Processing -> Shipped -> Delivered\nend\n");
    for state in &["Pending", "Processing", "Shipped", "Delivered"] {
        assert!(out.contains(state), "expected {} in enum:\n{}", state, out);
    }
}

#[test]
fn lifecycle_emits_transition_method() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(out.contains("pub fn transition"), "output:\n{}", out);
}

#[test]
fn lifecycle_transition_returns_result() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(out.contains("Result<Self"), "output:\n{}", out);
}

#[test]
fn lifecycle_checkpoint_emits_loom_annotation() {
    let out = ok(
        "module M\nlifecycle Connection :: Pending -> Active -> Closed\ncheckpoint: ReadyCheck\nrequires: is_ready\non_fail: abort_connection\nend\nend\nend\n",
    );
    assert!(
        out.contains("// LOOM[lifecycle:checkpoint:ReadyCheck]"),
        "expected checkpoint annotation:\n{}",
        out
    );
}

#[test]
fn lifecycle_checkpoint_includes_on_fail_in_annotation() {
    let out = ok(
        "module M\nlifecycle Connection :: Pending -> Active -> Closed\ncheckpoint: ReadyCheck\nrequires: is_ready\non_fail: abort_connection\nend\nend\nend\n",
    );
    assert!(out.contains("abort_connection"), "output:\n{}", out);
}

#[test]
fn lifecycle_terminal_state_returns_err() {
    let out = ok("module M\nlifecycle Session :: Init -> Active -> Done\nend\n");
    assert!(out.contains("terminal state"), "output:\n{}", out);
}

#[test]
fn lifecycle_coexists_with_fn_in_module() {
    let out = ok(
        "module App\nfn start :: Unit\nend\nlifecycle Worker :: Idle -> Running -> Stopped\nend\n",
    );
    assert!(out.contains("Worker"), "output:\n{}", out);
    assert!(out.contains("start"), "output:\n{}", out);
}

#[test]
fn multiple_lifecycles_in_module() {
    let out = ok(
        "module Domain\nlifecycle Order :: Draft -> Placed -> Fulfilled\nlifecycle Payment :: Unpaid -> Paid -> Refunded\nend\n",
    );
    assert!(out.contains("OrderState"), "output:\n{}", out);
    assert!(out.contains("PaymentState"), "output:\n{}", out);
}
