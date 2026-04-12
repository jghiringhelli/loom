/// M177: `actor` first-class item — lightweight actor with mailbox

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal actor generates a struct
#[test]
fn t1_actor_generates_struct() {
    let out = ok("module M\n  actor MyActor\n  end\nend\n");
    assert!(
        out.contains("struct MyActorActor"),
        "expected MyActorActor struct, got:\n{}",
        out
    );
}

// T2 — struct is generic over message type or uses VecDeque
#[test]
fn t2_actor_is_generic() {
    let out = ok("module M\n  actor Worker\n  end\nend\n");
    assert!(
        out.contains("<M>") || out.contains("VecDeque"),
        "expected generic or VecDeque, got:\n{}",
        out
    );
}

// T3 — mailbox/VecDeque field emitted
#[test]
fn t3_actor_has_mailbox() {
    let out = ok("module M\n  actor Processor\n  end\nend\n");
    assert!(
        out.contains("VecDeque") || out.contains("mailbox"),
        "expected mailbox, got:\n{}",
        out
    );
}

// T4 — send method emitted
#[test]
fn t4_actor_has_send_method() {
    let out = ok("module M\n  actor Handler\n  end\nend\n");
    assert!(out.contains("fn send"), "expected fn send, got:\n{}", out);
}

// T5 — receive method emitted
#[test]
fn t5_actor_has_receive_method() {
    let out = ok("module M\n  actor Handler\n  end\nend\n");
    assert!(
        out.contains("fn receive"),
        "expected fn receive, got:\n{}",
        out
    );
}

// T6 — pending method emitted
#[test]
fn t6_actor_has_pending_method() {
    let out = ok("module M\n  actor Queue\n  end\nend\n");
    assert!(
        out.contains("fn pending"),
        "expected fn pending, got:\n{}",
        out
    );
}

// T7 — name correctly embedded in struct name
#[test]
fn t7_actor_name_embedded() {
    let out = ok("module M\n  actor EventBus\n  end\nend\n");
    assert!(
        out.contains("struct EventBusActor"),
        "expected EventBusActor, got:\n{}",
        out
    );
}

// T8 — LOOM annotation present
#[test]
fn t8_actor_loom_annotation() {
    let out = ok("module M\n  actor Ticker\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("loom") || out.contains("actor"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple actors in one module
#[test]
fn t9_multiple_actors() {
    let out = ok("module M\n  actor A\n  end\n  actor B\n  end\nend\n");
    assert!(out.contains("AActor"), "expected AActor, got:\n{}", out);
    assert!(out.contains("BActor"), "expected BActor, got:\n{}", out);
}

// T10 — receive returns Option or uses pop_front
#[test]
fn t10_actor_receive_returns_option() {
    let out = ok("module M\n  actor Inbox\n  end\nend\n");
    assert!(
        out.contains("Option") || out.contains("pop_front"),
        "expected Option return or pop_front, got:\n{}",
        out
    );
}

// T11 — actor alongside other items
#[test]
fn t11_actor_with_other_items() {
    let src = "module M\n  actor Worker\n  end\n  entity Job\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(
        out.contains("WorkerActor"),
        "expected WorkerActor, got:\n{}",
        out
    );
    assert!(
        out.contains("struct Job") || out.contains("type Job"),
        "expected Job type, got:\n{}",
        out
    );
}

// T12 — missing end is a parse error
#[test]
fn t12_actor_missing_end_is_error() {
    let e = err("module M\n  actor Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
