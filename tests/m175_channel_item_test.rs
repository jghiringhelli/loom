//! M175: `channel` item — typed MPSC channel first-class module item.

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile failed: {:?}", e))
}

// ── M175.1: basic struct name ─────────────────────────────────────────────────
#[test]
fn m175_channel_struct_name() {
    let out = ok(r#"
module M
  channel Events
  end
end
"#);
    assert!(
        out.contains("EventsChannel"),
        "expected EventsChannel struct, got:\n{}",
        out
    );
}

// ── M175.2: generic type parameter ───────────────────────────────────────────
#[test]
fn m175_channel_generic_param() {
    let out = ok(r#"
module M
  channel Messages
  end
end
"#);
    assert!(
        out.contains("MessagesChannel<T>"),
        "expected generic param, got:\n{}",
        out
    );
}

// ── M175.3: default type String ──────────────────────────────────────────────
#[test]
fn m175_default_type_string() {
    let out = ok(r#"
module M
  channel Signals
  end
end
"#);
    assert!(
        out.contains("T = String"),
        "expected default T=String, got:\n{}",
        out
    );
}

// ── M175.4: custom element type ──────────────────────────────────────────────
#[test]
fn m175_custom_element_type() {
    let out = ok(r#"
module M
  channel Orders type: Order
  end
end
"#);
    assert!(out.contains("T = Order"), "expected T=Order, got:\n{}", out);
}

// ── M175.5: send method emitted ───────────────────────────────────────────────
#[test]
fn m175_send_method_emitted() {
    let out = ok(r#"
module M
  channel Pipe
  end
end
"#);
    assert!(
        out.contains("fn send"),
        "expected send method, got:\n{}",
        out
    );
}

// ── M175.6: recv method emitted ───────────────────────────────────────────────
#[test]
fn m175_recv_method_emitted() {
    let out = ok(r#"
module M
  channel Pipe
  end
end
"#);
    assert!(
        out.contains("fn recv"),
        "expected recv method, got:\n{}",
        out
    );
}

// ── M175.7: send returns Result ───────────────────────────────────────────────
#[test]
fn m175_send_returns_result() {
    let out = ok(r#"
module M
  channel Wire
  end
end
"#);
    assert!(
        out.contains("Result<(), String>"),
        "expected Result return, got:\n{}",
        out
    );
}

// ── M175.8: recv returns Option ───────────────────────────────────────────────
#[test]
fn m175_recv_returns_option() {
    let out = ok(r#"
module M
  channel Wire
  end
end
"#);
    assert!(
        out.contains("Option<T>"),
        "expected Option<T> return, got:\n{}",
        out
    );
}

// ── M175.9: capacity field emitted ────────────────────────────────────────────
#[test]
fn m175_capacity_field_emitted() {
    let out = ok(r#"
module M
  channel Bounded capacity: 64
  end
end
"#);
    assert!(
        out.contains("capacity: 64"),
        "expected capacity=64, got:\n{}",
        out
    );
}

// ── M175.10: LOOM annotation comment ─────────────────────────────────────────
#[test]
fn m175_loom_annotation_comment() {
    let out = ok(r#"
module M
  channel Notify
  end
end
"#);
    assert!(
        out.contains("LOOM[channel:concurrency]"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// ── M175.11: PhantomData in struct ────────────────────────────────────────────
#[test]
fn m175_phantom_data_in_struct() {
    let out = ok(r#"
module M
  channel Typed
  end
end
"#);
    assert!(
        out.contains("PhantomData"),
        "expected PhantomData field, got:\n{}",
        out
    );
}

// ── M175.12: channel alongside lock and queue ─────────────────────────────────
#[test]
fn m175_channel_with_lock_and_queue() {
    let out = ok(r#"
module Actor
  queue Inbox
  end
  lock State
  end
  channel Output type: Event
  end
end
"#);
    assert!(
        out.contains("InboxQueue"),
        "expected InboxQueue, got:\n{}",
        out
    );
    assert!(
        out.contains("StateLock"),
        "expected StateLock, got:\n{}",
        out
    );
    assert!(
        out.contains("OutputChannel"),
        "expected OutputChannel, got:\n{}",
        out
    );
}
