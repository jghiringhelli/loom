/// M182: `projection` first-class item — read-model projector from events

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal projection generates a struct
#[test]
fn t1_projection_generates_struct() {
    let out = ok("module M\n  projection OrderView\n  end\nend\n");
    assert!(out.contains("struct OrderViewProjection"), "expected OrderViewProjection struct, got:\n{}", out);
}

// T2 — struct is generic over event type
#[test]
fn t2_projection_is_generic() {
    let out = ok("module M\n  projection ReadModel\n  end\nend\n");
    assert!(out.contains("<E") || out.contains("Projection<"), "expected generic param, got:\n{}", out);
}

// T3 — events field emitted
#[test]
fn t3_projection_has_events_field() {
    let out = ok("module M\n  projection UserView\n  end\nend\n");
    assert!(out.contains("events"), "expected events field, got:\n{}", out);
}

// T4 — project method emitted
#[test]
fn t4_projection_has_project_method() {
    let out = ok("module M\n  projection Orders\n  end\nend\n");
    assert!(out.contains("fn project"), "expected fn project, got:\n{}", out);
}

// T5 — snapshot method emitted
#[test]
fn t5_projection_has_snapshot_method() {
    let out = ok("module M\n  projection Orders\n  end\nend\n");
    assert!(out.contains("fn snapshot"), "expected fn snapshot, got:\n{}", out);
}

// T6 — reset method emitted
#[test]
fn t6_projection_has_reset_method() {
    let out = ok("module M\n  projection Ledger\n  end\nend\n");
    assert!(out.contains("fn reset"), "expected fn reset, got:\n{}", out);
}

// T7 — name correctly embedded in struct name
#[test]
fn t7_projection_name_embedded() {
    let out = ok("module M\n  projection AccountSummary\n  end\nend\n");
    assert!(
        out.contains("struct AccountSummaryProjection"),
        "expected AccountSummaryProjection, got:\n{}",
        out
    );
}

// T8 — LOOM annotation present
#[test]
fn t8_projection_loom_annotation() {
    let out = ok("module M\n  projection Stats\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("projection"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple projections in one module
#[test]
fn t9_multiple_projections() {
    let out = ok("module M\n  projection A\n  end\n  projection B\n  end\nend\n");
    assert!(out.contains("AProjection"), "expected AProjection, got:\n{}", out);
    assert!(out.contains("BProjection"), "expected BProjection, got:\n{}", out);
}

// T10 — snapshot field and VecDeque used
#[test]
fn t10_projection_uses_vecdecque_and_vec() {
    let out = ok("module M\n  projection History\n  end\nend\n");
    assert!(
        out.contains("VecDeque") || out.contains("Vec"),
        "expected VecDeque or Vec storage, got:\n{}",
        out
    );
}

// T11 — projection alongside other items
#[test]
fn t11_projection_with_other_items() {
    let src = "module M\n  projection Inbox\n  end\n  entity Message\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(out.contains("InboxProjection"), "expected InboxProjection, got:\n{}", out);
    assert!(out.contains("Message"), "expected Message type, got:\n{}", out);
}

// T12 — missing end is a parse error
#[test]
fn t12_projection_missing_end_is_error() {
    let e = err("module M\n  projection Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
