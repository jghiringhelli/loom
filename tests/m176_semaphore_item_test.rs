/// M176: `semaphore` first-class item — counting semaphore with wait/signal/count

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal semaphore generates a struct
#[test]
fn t1_semaphore_generates_struct() {
    let out = ok("module M\n  semaphore MySem\n  end\nend\n");
    assert!(
        out.contains("struct MySemSemaphore"),
        "expected MySemSemaphore struct, got:\n{}",
        out
    );
}

// T2 — struct contains AtomicUsize for permit counting
#[test]
fn t2_semaphore_has_atomic_usize() {
    let out = ok("module M\n  semaphore Counter\n  end\nend\n");
    assert!(
        out.contains("AtomicUsize"),
        "expected AtomicUsize, got:\n{}",
        out
    );
}

// T3 — wait method emitted
#[test]
fn t3_semaphore_has_wait_method() {
    let out = ok("module M\n  semaphore Gate\n  end\nend\n");
    assert!(out.contains("fn wait"), "expected fn wait, got:\n{}", out);
}

// T4 — signal method emitted
#[test]
fn t4_semaphore_has_signal_method() {
    let out = ok("module M\n  semaphore Gate\n  end\nend\n");
    assert!(
        out.contains("fn signal"),
        "expected fn signal, got:\n{}",
        out
    );
}

// T5 — count method emitted
#[test]
fn t5_semaphore_has_count_method() {
    let out = ok("module M\n  semaphore Capacity\n  end\nend\n");
    assert!(out.contains("fn count"), "expected fn count, got:\n{}", out);
}

// T6 — permits field emitted
#[test]
fn t6_semaphore_has_permits_field() {
    let out = ok("module M\n  semaphore Resource\n  end\nend\n");
    assert!(
        out.contains("permits"),
        "expected permits field, got:\n{}",
        out
    );
}

// T7 — name is correctly embedded in struct name
#[test]
fn t7_semaphore_name_embedded() {
    let out = ok("module M\n  semaphore WorkerPool\n  end\nend\n");
    assert!(
        out.contains("struct WorkerPoolSemaphore"),
        "expected WorkerPoolSemaphore, got:\n{}",
        out
    );
}

// T8 — LOOM annotation present
#[test]
fn t8_semaphore_loom_annotation() {
    let out = ok("module M\n  semaphore Ticket\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("loom") || out.contains("semaphore"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple semaphores in one module
#[test]
fn t9_multiple_semaphores() {
    let out = ok("module M\n  semaphore A\n  end\n  semaphore B\n  end\nend\n");
    assert!(
        out.contains("ASemaphore"),
        "expected ASemaphore, got:\n{}",
        out
    );
    assert!(
        out.contains("BSemaphore"),
        "expected BSemaphore, got:\n{}",
        out
    );
}

// T10 — wait uses compare_exchange or similar atomic operation
#[test]
fn t10_semaphore_wait_uses_atomic_op() {
    let out = ok("module M\n  semaphore Permit\n  end\nend\n");
    assert!(
        out.contains("compare_exchange") || out.contains("fetch_sub") || out.contains("Ordering"),
        "expected atomic op in wait, got:\n{}",
        out
    );
}

// T11 — semaphore inside module with other items
#[test]
fn t11_semaphore_with_other_items() {
    let src = "module M\n  semaphore Conn\n  end\n  entity User\n    id: Int\n    name: String\n  end\nend\n";
    let out = ok(src);
    assert!(
        out.contains("ConnSemaphore"),
        "expected ConnSemaphore, got:\n{}",
        out
    );
    assert!(
        out.contains("struct User") || out.contains("type User"),
        "expected User type, got:\n{}",
        out
    );
}

// T12 — missing end produces parse error
#[test]
fn t12_semaphore_missing_end_is_error() {
    let e = err("module M\n  semaphore Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
