//! M173: `queue` item — FIFO/LIFO named queue first-class module item.

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile failed: {:?}", e))
}

// ── M173.1: basic struct name ─────────────────────────────────────────────────
#[test]
fn m173_queue_struct_name() {
    let out = ok(r#"
module M
  queue Jobs
  end
end
"#);
    assert!(out.contains("JobsQueue"), "expected JobsQueue struct, got:\n{}", out);
}

// ── M173.2: generic type parameter ───────────────────────────────────────────
#[test]
fn m173_queue_generic_param() {
    let out = ok(r#"
module M
  queue Tasks
  end
end
"#);
    assert!(out.contains("TasksQueue<T>"), "expected generic param, got:\n{}", out);
}

// ── M173.3: enqueue method emitted ───────────────────────────────────────────
#[test]
fn m173_enqueue_method_emitted() {
    let out = ok(r#"
module M
  queue Work
  end
end
"#);
    assert!(out.contains("fn enqueue"), "expected enqueue method, got:\n{}", out);
}

// ── M173.4: dequeue method emitted ───────────────────────────────────────────
#[test]
fn m173_dequeue_method_emitted() {
    let out = ok(r#"
module M
  queue Work
  end
end
"#);
    assert!(out.contains("fn dequeue"), "expected dequeue method, got:\n{}", out);
}

// ── M173.5: is_empty method emitted ──────────────────────────────────────────
#[test]
fn m173_is_empty_method_emitted() {
    let out = ok(r#"
module M
  queue Work
  end
end
"#);
    assert!(out.contains("fn is_empty"), "expected is_empty method, got:\n{}", out);
}

// ── M173.6: capacity field emitted ───────────────────────────────────────────
#[test]
fn m173_capacity_field_emitted() {
    let out = ok(r#"
module M
  queue Bounded capacity: 100
  end
end
"#);
    assert!(out.contains("capacity: 100"), "expected capacity=100, got:\n{}", out);
}

// ── M173.7: default capacity is 0 (unbounded) ────────────────────────────────
#[test]
fn m173_default_capacity_unbounded() {
    let out = ok(r#"
module M
  queue Unbounded
  end
end
"#);
    assert!(out.contains("unbounded"), "expected unbounded annotation, got:\n{}", out);
}

// ── M173.8: kind annotation in comment ───────────────────────────────────────
#[test]
fn m173_kind_in_annotation() {
    let out = ok(r#"
module M
  queue Lifo kind: lifo
  end
end
"#);
    assert!(out.contains("lifo"), "expected lifo in output, got:\n{}", out);
}

// ── M173.9: VecDeque in inner field ──────────────────────────────────────────
#[test]
fn m173_vecdeque_inner_field() {
    let out = ok(r#"
module M
  queue Items
  end
end
"#);
    assert!(out.contains("VecDeque"), "expected VecDeque inner field, got:\n{}", out);
}

// ── M173.10: LOOM annotation comment ─────────────────────────────────────────
#[test]
fn m173_loom_annotation_comment() {
    let out = ok(r#"
module M
  queue Events
  end
end
"#);
    assert!(
        out.contains("LOOM[queue:concurrency]"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// ── M173.11: multiple queues in one module ────────────────────────────────────
#[test]
fn m173_multiple_queues() {
    let out = ok(r#"
module M
  queue Inbox
  end
  queue Outbox
  end
end
"#);
    assert!(out.contains("InboxQueue"), "expected InboxQueue, got:\n{}", out);
    assert!(out.contains("OutboxQueue"), "expected OutboxQueue, got:\n{}", out);
}

// ── M173.12: queue alongside other items ─────────────────────────────────────
#[test]
fn m173_queue_with_other_items() {
    let out = ok(r#"
module Pipeline
  queue Buffer capacity: 50
  end
  fn process :: String -> String
    "done"
  end
end
"#);
    assert!(out.contains("BufferQueue"), "expected BufferQueue, got:\n{}", out);
    assert!(out.contains("fn process"), "expected fn process, got:\n{}", out);
}
