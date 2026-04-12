use loom::compile;

fn ok(src: &str) -> String {
    match compile(src) {
        Ok(out) => out,
        Err(e) => panic!("compile error: {:?}", e),
    }
}

fn degenerate_src() -> &'static str {
    "module M\nfn compute :: Int -> Int\ndegenerate:\n  primary: fast_path\n  fallback: slow_path\nend\n  0\nend\nend\n"
}

// ── Parse tests ───────────────────────────────────────────────────────────────

#[test]
fn degenerate_minimal_parses() {
    let out = ok(degenerate_src());
    assert!(out.contains("compute"), "output:\n{}", out);
}

#[test]
fn degenerate_with_equivalence_proof_parses() {
    let out = ok(
        "module M\nfn solve :: Int -> Int\ndegenerate:\n  primary: algebraic\n  fallback: numeric\n  equivalence_proof: bisimulation\nend\n  0\nend\nend\n",
    );
    assert!(out.contains("solve"), "output:\n{}", out);
}

// ── Codegen tests ─────────────────────────────────────────────────────────────

#[test]
fn degenerate_emits_loom_annotation() {
    let out = ok(degenerate_src());
    assert!(
        out.contains("// LOOM[contract:Degenerate]"),
        "expected LOOM annotation:\n{}",
        out
    );
}

#[test]
fn degenerate_annotation_includes_fn_name() {
    let out = ok(degenerate_src());
    assert!(out.contains("compute"), "output:\n{}", out);
}

#[test]
fn degenerate_annotation_includes_primary() {
    let out = ok(degenerate_src());
    assert!(out.contains("fast_path"), "output:\n{}", out);
}

#[test]
fn degenerate_annotation_includes_fallback() {
    let out = ok(degenerate_src());
    assert!(out.contains("slow_path"), "output:\n{}", out);
}

#[test]
fn degenerate_emits_fallback_struct() {
    let out = ok(degenerate_src());
    assert!(
        out.contains("DegenerateFallback"),
        "expected DegenerateFallback struct:\n{}",
        out
    );
}

#[test]
fn degenerate_emits_normal_constructor() {
    let out = ok(degenerate_src());
    assert!(out.contains("pub fn normal"), "output:\n{}", out);
}

#[test]
fn degenerate_emits_fallback_constructor() {
    let out = ok(degenerate_src());
    assert!(out.contains("pub fn fallback"), "output:\n{}", out);
}

#[test]
fn degenerate_emits_require_non_degenerate() {
    let out = ok(degenerate_src());
    assert!(
        out.contains("require_non_degenerate"),
        "expected require_non_degenerate method:\n{}",
        out
    );
}

#[test]
fn degenerate_struct_is_generic() {
    let out = ok(degenerate_src());
    assert!(out.contains("<T>") || out.contains("<T:"), "expected generic T:\n{}", out);
}

#[test]
fn degenerate_coexists_with_normal_fn() {
    let out = ok(
        "module M\nfn helper :: Unit\nend\nfn compute :: Int -> Int\ndegenerate:\n  primary: fast_path\n  fallback: slow_path\nend\n  0\nend\nend\n",
    );
    assert!(out.contains("helper"), "output:\n{}", out);
    assert!(out.contains("DegenerateFallback"), "output:\n{}", out);
}
