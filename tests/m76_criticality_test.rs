use loom::compile;

fn ok(src: &str) -> String {
    match compile(src) {
        Ok(out) => out,
        Err(e) => panic!("compile error: {:?}", e),
    }
}

fn criticality_src(name: &str, lower: &str, upper: &str) -> String {
    format!(
        "module M\nbeing {}\n  telos: \"compute\"\n  end\n  criticality:\n    lower: {}\n    upper: {}\n  end\nend\nend\n",
        name, lower, upper
    )
}

// ── Parse tests ───────────────────────────────────────────────────────────────

#[test]
fn criticality_minimal_parses() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(out.contains("Network"), "output:\n{}", out);
}

#[test]
fn criticality_with_probe_fn_parses() {
    let out = ok(
        "module M\nbeing CA\n  telos: \"evolve\"\n  end\n  criticality:\n    lower: 0.3\n    upper: 0.7\n    probe_fn: measure_entropy\n  end\nend\nend\n",
    );
    assert!(out.contains("CA"), "output:\n{}", out);
}

// ── Codegen tests ─────────────────────────────────────────────────────────────

#[test]
fn criticality_emits_loom_annotation() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(
        out.contains("// LOOM[criticality:Network]"),
        "expected LOOM annotation:\n{}",
        out
    );
}

#[test]
fn criticality_annotation_includes_bounds() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(out.contains("lower=0.2"), "expected lower in annotation:\n{}", out);
    assert!(out.contains("upper=0.8"), "expected upper in annotation:\n{}", out);
}

#[test]
fn criticality_emits_lower_const() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(
        out.contains("NETWORK_CRITICALITY_LOWER"),
        "expected lower const:\n{}",
        out
    );
}

#[test]
fn criticality_emits_upper_const() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(
        out.contains("NETWORK_CRITICALITY_UPPER"),
        "expected upper const:\n{}",
        out
    );
}

#[test]
fn criticality_lower_const_value() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(out.contains("0.2"), "expected 0.2 in output:\n{}", out);
}

#[test]
fn criticality_upper_const_value() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(out.contains("0.8"), "expected 0.8 in output:\n{}", out);
}

#[test]
fn criticality_with_probe_fn_emits_loom_probe_annotation() {
    let out = ok(
        "module M\nbeing CA\n  telos: \"evolve\"\n  end\n  criticality:\n    lower: 0.3\n    upper: 0.7\n    probe_fn: measure_entropy\n  end\nend\nend\n",
    );
    assert!(
        out.contains("// LOOM[criticality:probe]"),
        "expected probe annotation:\n{}",
        out
    );
    assert!(out.contains("measure_entropy"), "output:\n{}", out);
}

#[test]
fn criticality_with_probe_fn_emits_probe_function() {
    let out = ok(
        "module M\nbeing CA\n  telos: \"evolve\"\n  end\n  criticality:\n    lower: 0.3\n    upper: 0.7\n    probe_fn: measure_entropy\n  end\nend\nend\n",
    );
    assert!(
        out.contains("ca_criticality_probe"),
        "expected probe fn:\n{}",
        out
    );
}

#[test]
fn criticality_different_bounds() {
    let out = ok(&criticality_src("Ecosystem", "0.1", "0.9"));
    assert!(out.contains("ECOSYSTEM_CRITICALITY_LOWER"), "output:\n{}", out);
    assert!(out.contains("ECOSYSTEM_CRITICALITY_UPPER"), "output:\n{}", out);
}

#[test]
fn criticality_coexists_with_telos() {
    let out = ok(&criticality_src("Network", "0.2", "0.8"));
    assert!(out.contains("Network"), "output:\n{}", out);
    assert!(out.contains("CRITICALITY"), "output:\n{}", out);
}
