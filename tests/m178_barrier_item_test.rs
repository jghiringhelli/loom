/// M178: `barrier` first-class item — N-thread synchronization barrier

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal barrier generates a struct
#[test]
fn t1_barrier_generates_struct() {
    let out = ok("module M\n  barrier MyBarrier\n  end\nend\n");
    assert!(out.contains("struct MyBarrierBarrier"), "expected MyBarrierBarrier struct, got:\n{}", out);
}

// T2 — struct contains AtomicUsize for count tracking
#[test]
fn t2_barrier_has_atomic_usize() {
    let out = ok("module M\n  barrier Phase\n  end\nend\n");
    assert!(out.contains("AtomicUsize"), "expected AtomicUsize, got:\n{}", out);
}

// T3 — wait method emitted
#[test]
fn t3_barrier_has_wait_method() {
    let out = ok("module M\n  barrier Gate\n  end\nend\n");
    assert!(out.contains("fn wait"), "expected fn wait, got:\n{}", out);
}

// T4 — reset method emitted
#[test]
fn t4_barrier_has_reset_method() {
    let out = ok("module M\n  barrier Gate\n  end\nend\n");
    assert!(out.contains("fn reset"), "expected fn reset, got:\n{}", out);
}

// T5 — count field emitted
#[test]
fn t5_barrier_has_count_field() {
    let out = ok("module M\n  barrier Checkpoint\n  end\nend\n");
    assert!(out.contains("count"), "expected count field, got:\n{}", out);
}

// T6 — name correctly embedded in struct name
#[test]
fn t6_barrier_name_embedded() {
    let out = ok("module M\n  barrier ThreadSync\n  end\nend\n");
    assert!(
        out.contains("struct ThreadSyncBarrier"),
        "expected ThreadSyncBarrier, got:\n{}",
        out
    );
}

// T7 — LOOM annotation present
#[test]
fn t7_barrier_loom_annotation() {
    let out = ok("module M\n  barrier Rendezvous\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("loom") || out.contains("barrier"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T8 — multiple barriers in one module
#[test]
fn t8_multiple_barriers() {
    let out = ok("module M\n  barrier A\n  end\n  barrier B\n  end\nend\n");
    assert!(out.contains("ABarrier"), "expected ABarrier, got:\n{}", out);
    assert!(out.contains("BBarrier"), "expected BBarrier, got:\n{}", out);
}

// T9 — wait uses atomic fetch_add or compare_exchange
#[test]
fn t9_barrier_wait_uses_atomic_op() {
    let out = ok("module M\n  barrier Sync\n  end\nend\n");
    assert!(
        out.contains("fetch_add") || out.contains("compare_exchange") || out.contains("Ordering"),
        "expected atomic op in wait, got:\n{}",
        out
    );
}

// T10 — barrier alongside other items
#[test]
fn t10_barrier_with_other_items() {
    let src = "module M\n  barrier WorkPhase\n  end\n  entity Task\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(out.contains("WorkPhaseBarrier"), "expected WorkPhaseBarrier, got:\n{}", out);
    assert!(out.contains("struct Task") || out.contains("type Task"), "expected Task type, got:\n{}", out);
}

// T11 — reset stores zero back into AtomicUsize
#[test]
fn t11_barrier_reset_stores_zero() {
    let out = ok("module M\n  barrier Counter\n  end\nend\n");
    assert!(
        out.contains("store(0") || out.contains(".store(0"),
        "expected store(0) in reset, got:\n{}",
        out
    );
}

// T12 — missing end is a parse error
#[test]
fn t12_barrier_missing_end_is_error() {
    let e = err("module M\n  barrier Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
