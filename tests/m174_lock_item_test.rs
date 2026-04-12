//! M174: `lock` item — named mutex-style lock first-class module item.

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile failed: {:?}", e))
}

// ── M174.1: basic struct name ─────────────────────────────────────────────────
#[test]
fn m174_lock_struct_name() {
    let out = ok(r#"
module M
  lock DbLock
  end
end
"#);
    assert!(
        out.contains("DbLockLock"),
        "expected DbLockLock struct, got:\n{}",
        out
    );
}

// ── M174.2: acquire method emitted ───────────────────────────────────────────
#[test]
fn m174_acquire_method_emitted() {
    let out = ok(r#"
module M
  lock Resource
  end
end
"#);
    assert!(
        out.contains("fn acquire"),
        "expected acquire method, got:\n{}",
        out
    );
}

// ── M174.3: release method emitted ───────────────────────────────────────────
#[test]
fn m174_release_method_emitted() {
    let out = ok(r#"
module M
  lock Resource
  end
end
"#);
    assert!(
        out.contains("fn release"),
        "expected release method, got:\n{}",
        out
    );
}

// ── M174.4: is_locked method emitted ─────────────────────────────────────────
#[test]
fn m174_is_locked_method_emitted() {
    let out = ok(r#"
module M
  lock Resource
  end
end
"#);
    assert!(
        out.contains("fn is_locked"),
        "expected is_locked method, got:\n{}",
        out
    );
}

// ── M174.5: AtomicBool backing field ─────────────────────────────────────────
#[test]
fn m174_atomic_bool_field() {
    let out = ok(r#"
module M
  lock Mutex
  end
end
"#);
    assert!(
        out.contains("AtomicBool"),
        "expected AtomicBool field, got:\n{}",
        out
    );
}

// ── M174.6: acquire returns bool ─────────────────────────────────────────────
#[test]
fn m174_acquire_returns_bool() {
    let out = ok(r#"
module M
  lock Guard
  end
end
"#);
    assert!(
        out.contains("-> bool"),
        "expected bool return type, got:\n{}",
        out
    );
}

// ── M174.7: compare_exchange for acquire ─────────────────────────────────────
#[test]
fn m174_compare_exchange_used() {
    let out = ok(r#"
module M
  lock Sync
  end
end
"#);
    assert!(
        out.contains("compare_exchange"),
        "expected compare_exchange, got:\n{}",
        out
    );
}

// ── M174.8: LOOM annotation comment ──────────────────────────────────────────
#[test]
fn m174_loom_annotation_comment() {
    let out = ok(r#"
module M
  lock Critical
  end
end
"#);
    assert!(
        out.contains("LOOM[lock:concurrency]"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// ── M174.9: new() constructor ─────────────────────────────────────────────────
#[test]
fn m174_new_constructor() {
    let out = ok(r#"
module M
  lock Init
  end
end
"#);
    assert!(
        out.contains("fn new()"),
        "expected new() constructor, got:\n{}",
        out
    );
}

// ── M174.10: Ordering imports in methods ─────────────────────────────────────
#[test]
fn m174_ordering_imports_in_methods() {
    let out = ok(r#"
module M
  lock Ordered
  end
end
"#);
    assert!(
        out.contains("Ordering"),
        "expected Ordering import, got:\n{}",
        out
    );
}

// ── M174.11: multiple locks ───────────────────────────────────────────────────
#[test]
fn m174_multiple_locks() {
    let out = ok(r#"
module M
  lock ReadLock
  end
  lock WriteLock
  end
end
"#);
    assert!(
        out.contains("ReadLockLock"),
        "expected ReadLockLock, got:\n{}",
        out
    );
    assert!(
        out.contains("WriteLockLock"),
        "expected WriteLockLock, got:\n{}",
        out
    );
}

// ── M174.12: lock alongside queue ────────────────────────────────────────────
#[test]
fn m174_lock_with_queue() {
    let out = ok(r#"
module SafeQueue
  queue Items
  end
  lock Guard
  end
end
"#);
    assert!(
        out.contains("ItemsQueue"),
        "expected ItemsQueue, got:\n{}",
        out
    );
    assert!(
        out.contains("GuardLock"),
        "expected GuardLock, got:\n{}",
        out
    );
}
