/// M183 — `resource` item: exclusive resource with atomic acquire/release

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    format!("{:?}", loom::compile(src).unwrap_err())
}

// ── Parse ────────────────────────────────────────────────────────────────────

#[test]
fn parses_minimal_resource() {
    ok("module M\n  resource MyRes\n  end\nend\n");
}

#[test]
fn rejects_resource_without_name() {
    let e = err("module M\n  resource\n  end\nend\n");
    assert!(e.contains("expected") || e.contains("identifier") || e.contains("Unexpected"),
        "unexpected error: {e}");
}

#[test]
fn rejects_resource_without_end() {
    let e = err("module M\n  resource MyRes\nend\n");
    assert!(e.contains("end") || e.contains("End") || e.contains("expected"),
        "unexpected error: {e}");
}

// ── Codegen — struct ─────────────────────────────────────────────────────────

#[test]
fn emits_resource_struct() {
    let out = ok("module M\n  resource MyRes\n  end\nend\n");
    assert!(out.contains("struct MyResResource"), "missing struct: {out}");
}

#[test]
fn emits_atomic_bool_field() {
    let out = ok("module M\n  resource Lock\n  end\nend\n");
    assert!(out.contains("AtomicBool"), "missing AtomicBool: {out}");
}

#[test]
fn emits_acquire_method() {
    let out = ok("module M\n  resource Lock\n  end\nend\n");
    assert!(out.contains("fn acquire"), "missing acquire: {out}");
}

#[test]
fn emits_release_method() {
    let out = ok("module M\n  resource Lock\n  end\nend\n");
    assert!(out.contains("fn release"), "missing release: {out}");
}

#[test]
fn emits_is_acquired_method() {
    let out = ok("module M\n  resource Lock\n  end\nend\n");
    assert!(out.contains("fn is_acquired"), "missing is_acquired: {out}");
}

#[test]
fn emits_compare_exchange() {
    let out = ok("module M\n  resource Lock\n  end\nend\n");
    assert!(out.contains("compare_exchange"), "missing compare_exchange: {out}");
}

// ── LOOM annotation ───────────────────────────────────────────────────────────

#[test]
fn emits_loom_annotation() {
    let out = ok("module M\n  resource MyRes\n  end\nend\n");
    assert!(out.contains("LOOM[resource:"), "missing LOOM annotation: {out}");
}

// ── Name derivation ───────────────────────────────────────────────────────────

#[test]
fn pascal_case_struct_name() {
    let out = ok("module M\n  resource my_resource\n  end\nend\n");
    assert!(out.contains("struct MyResourceResource"), "missing PascalCase struct: {out}");
}

// ── Multiple items ────────────────────────────────────────────────────────────

#[test]
fn two_resources_in_module() {
    let out = ok("module M\n  resource A\n  end\n  resource B\n  end\nend\n");
    assert!(out.contains("struct AResource"), "missing AResource: {out}");
    assert!(out.contains("struct BResource"), "missing BResource: {out}");
}

#[test]
fn resource_and_lease_coexist() {
    let out = ok("module M\n  resource Lock\n  end\n  lease Token\n  end\nend\n");
    assert!(out.contains("struct LockResource"), "missing LockResource: {out}");
    assert!(out.contains("struct TokenLease"), "missing TokenLease: {out}");
}
