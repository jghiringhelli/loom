/// M184 — `lease` item: time-bounded lease with TTL + acquire/release/is_expired/is_valid

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    format!("{:?}", loom::compile(src).unwrap_err())
}

// ── Parse ────────────────────────────────────────────────────────────────────

#[test]
fn parses_minimal_lease() {
    ok("module M\n  lease MyLease\n  end\nend\n");
}

#[test]
fn parses_lease_with_ttl() {
    ok("module M\n  lease MyLease\n    ttl: 120\n  end\nend\n");
}

#[test]
fn rejects_lease_without_name() {
    let e = err("module M\n  lease\n  end\nend\n");
    assert!(
        e.contains("expected") || e.contains("identifier") || e.contains("Unexpected"),
        "unexpected error: {e}"
    );
}

// ── Codegen — struct ─────────────────────────────────────────────────────────

#[test]
fn emits_lease_struct() {
    let out = ok("module M\n  lease MyLease\n  end\nend\n");
    assert!(out.contains("struct MyLease"), "missing struct: {out}");
}

#[test]
fn emits_ttl_secs_field() {
    let out = ok("module M\n  lease Session\n  end\nend\n");
    assert!(out.contains("ttl_secs"), "missing ttl_secs: {out}");
}

#[test]
fn emits_acquired_at_field() {
    let out = ok("module M\n  lease Session\n  end\nend\n");
    assert!(out.contains("acquired_at"), "missing acquired_at: {out}");
}

#[test]
fn default_ttl_is_60() {
    let out = ok("module M\n  lease Session\n  end\nend\n");
    assert!(
        out.contains("ttl_secs: 60"),
        "missing default ttl 60: {out}"
    );
}

#[test]
fn custom_ttl_respected() {
    let out = ok("module M\n  lease Session\n    ttl: 300\n  end\nend\n");
    assert!(
        out.contains("ttl_secs: 300"),
        "missing custom ttl 300: {out}"
    );
}

// ── Codegen — methods ─────────────────────────────────────────────────────────

#[test]
fn emits_acquire_method() {
    let out = ok("module M\n  lease Token\n  end\nend\n");
    assert!(out.contains("fn acquire"), "missing acquire: {out}");
}

#[test]
fn emits_release_method() {
    let out = ok("module M\n  lease Token\n  end\nend\n");
    assert!(out.contains("fn release"), "missing release: {out}");
}

#[test]
fn emits_is_expired_method() {
    let out = ok("module M\n  lease Token\n  end\nend\n");
    assert!(out.contains("fn is_expired"), "missing is_expired: {out}");
}

#[test]
fn emits_is_valid_method() {
    let out = ok("module M\n  lease Token\n  end\nend\n");
    assert!(out.contains("fn is_valid"), "missing is_valid: {out}");
}

// ── LOOM annotation ───────────────────────────────────────────────────────────

#[test]
fn emits_loom_annotation() {
    let out = ok("module M\n  lease MyLease\n  end\nend\n");
    assert!(
        out.contains("LOOM[lease:"),
        "missing LOOM annotation: {out}"
    );
}

// ── Multiple items ────────────────────────────────────────────────────────────

#[test]
fn two_leases_in_module() {
    let out = ok("module M\n  lease A\n  end\n  lease B\n  end\nend\n");
    assert!(out.contains("struct ALease"), "missing ALease: {out}");
    assert!(out.contains("struct BLease"), "missing BLease: {out}");
}
