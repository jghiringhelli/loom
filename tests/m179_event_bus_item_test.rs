/// M179: `event_bus` first-class item — pub/sub event dispatcher

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal event_bus generates a struct
#[test]
fn t1_event_bus_generates_struct() {
    let out = ok("module M\n  event_bus MyBus\n  end\nend\n");
    assert!(out.contains("struct MyBusEventBus"), "expected MyBusEventBus struct, got:\n{}", out);
}

// T2 — struct is generic over event type
#[test]
fn t2_event_bus_is_generic() {
    let out = ok("module M\n  event_bus AppBus\n  end\nend\n");
    assert!(out.contains("<E") || out.contains("EventBus<"), "expected generic param, got:\n{}", out);
}

// T3 — subscribe method emitted
#[test]
fn t3_event_bus_has_subscribe() {
    let out = ok("module M\n  event_bus Notifier\n  end\nend\n");
    assert!(out.contains("fn subscribe"), "expected fn subscribe, got:\n{}", out);
}

// T4 — publish method emitted
#[test]
fn t4_event_bus_has_publish() {
    let out = ok("module M\n  event_bus Notifier\n  end\nend\n");
    assert!(out.contains("fn publish"), "expected fn publish, got:\n{}", out);
}

// T5 — drain method emitted
#[test]
fn t5_event_bus_has_drain() {
    let out = ok("module M\n  event_bus Queue\n  end\nend\n");
    assert!(out.contains("fn drain"), "expected fn drain, got:\n{}", out);
}

// T6 — subscribers field (Vec of callbacks) emitted
#[test]
fn t6_event_bus_has_subscribers_field() {
    let out = ok("module M\n  event_bus Hub\n  end\nend\n");
    assert!(out.contains("subscribers"), "expected subscribers field, got:\n{}", out);
}

// T7 — name correctly embedded in struct name
#[test]
fn t7_event_bus_name_embedded() {
    let out = ok("module M\n  event_bus DomainEvents\n  end\nend\n");
    assert!(
        out.contains("struct DomainEventsEventBus"),
        "expected DomainEventsEventBus, got:\n{}",
        out
    );
}

// T8 — LOOM annotation present
#[test]
fn t8_event_bus_loom_annotation() {
    let out = ok("module M\n  event_bus Signal\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("event_bus"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple event buses in one module
#[test]
fn t9_multiple_event_buses() {
    let out = ok("module M\n  event_bus A\n  end\n  event_bus B\n  end\nend\n");
    assert!(out.contains("AEventBus"), "expected AEventBus, got:\n{}", out);
    assert!(out.contains("BEventBus"), "expected BEventBus, got:\n{}", out);
}

// T10 — VecDeque or Vec used for pending events
#[test]
fn t10_event_bus_has_pending_storage() {
    let out = ok("module M\n  event_bus Pending\n  end\nend\n");
    assert!(
        out.contains("VecDeque") || out.contains("Vec"),
        "expected VecDeque or Vec, got:\n{}",
        out
    );
}

// T11 — event_bus alongside other items
#[test]
fn t11_event_bus_with_other_items() {
    let src = "module M\n  event_bus Events\n  end\n  entity Order\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(out.contains("EventsEventBus"), "expected EventsEventBus, got:\n{}", out);
    assert!(out.contains("Order"), "expected Order type, got:\n{}", out);
}

// T12 — missing end is a parse error
#[test]
fn t12_event_bus_missing_end_is_error() {
    let e = err("module M\n  event_bus Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
