/// M161 — Top-level `event` item: parser + codegen tests.
///
/// `event Name field: Type ... end`
/// must parse into an `EventDef` and emit:
///  - `{Name}Event` struct with `#[derive(Debug, Clone, PartialEq)]`
///  - Typed `pub` fields (Loom types mapped to Rust)
///  - `{Name}EventHandler` trait with `fn handle(&self, event: &{Name}Event);`
///  - `LOOM[event:domain]` audit comment

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M161.1: basic event parses without error ─────────────────────────────────

#[test]
fn m161_simple_event_parses() {
    let src = r#"
module orders
event OrderPlaced
  order_id: Int
  amount: Float
end
end
"#;
    let out = compile(src);
    assert!(out.contains("OrderPlacedEvent"), "expected OrderPlacedEvent in output\n{out}");
}

// ─── M161.2: struct derive attrs emitted ──────────────────────────────────────

#[test]
fn m161_derive_attrs_emitted() {
    let src = r#"
module auth
event UserRegistered
  user_id: Int
  email: String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[derive(Debug, Clone, PartialEq)]"),
        "missing derive attrs\n{out}"
    );
}

// ─── M161.3: typed fields emitted ─────────────────────────────────────────────

#[test]
fn m161_typed_fields_emitted() {
    let src = r#"
module auth
event UserRegistered
  user_id: Int
  email: String
  verified: Bool
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub user_id: i64"), "missing user_id field\n{out}");
    assert!(out.contains("pub email: String"), "missing email field\n{out}");
    assert!(out.contains("pub verified: bool"), "missing verified field\n{out}");
}

// ─── M161.4: Float type maps to f64 ───────────────────────────────────────────

#[test]
fn m161_float_maps_to_f64() {
    let src = r#"
module payments
event PaymentReceived
  amount: Float
  currency: String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub amount: f64"), "Float should map to f64\n{out}");
}

// ─── M161.5: handler trait emitted ────────────────────────────────────────────

#[test]
fn m161_handler_trait_emitted() {
    let src = r#"
module shipping
event OrderShipped
  shipment_id: Int
  destination: String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub trait OrderShippedEventHandler"),
        "missing handler trait\n{out}"
    );
}

// ─── M161.6: handler fn handle signature ──────────────────────────────────────

#[test]
fn m161_handler_fn_signature() {
    let src = r#"
module shipping
event OrderShipped
  shipment_id: Int
  destination: String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("fn handle(&self, event: &OrderShippedEvent)"),
        "missing handle signature\n{out}"
    );
}

// ─── M161.7: audit comment emitted ────────────────────────────────────────────

#[test]
fn m161_audit_comment_emitted() {
    let src = r#"
module catalog
event ProductCreated
  product_id: Int
  name: String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[event:domain]"),
        "missing LOOM audit comment\n{out}"
    );
}

// ─── M161.8: event with no fields parses ──────────────────────────────────────

#[test]
fn m161_empty_event_parses() {
    let out = compile_check(
        r#"
module system
event SystemStarted
end
end
"#,
    );
    assert!(out.is_ok(), "empty event should parse: {:?}", out.err());
}

// ─── M161.9: multiple events in one module ────────────────────────────────────

#[test]
fn m161_multiple_events_in_module() {
    let src = r#"
module life_events
event UserCreated
  user_id: Int
end
event UserDeleted
  user_id: Int
end
end
"#;
    let out = compile(src);
    assert!(out.contains("UserCreatedEvent"), "missing UserCreatedEvent\n{out}");
    assert!(out.contains("UserDeletedEvent"), "missing UserDeletedEvent\n{out}");
    assert!(out.contains("UserCreatedEventHandler"), "missing UserCreatedEventHandler\n{out}");
    assert!(out.contains("UserDeletedEventHandler"), "missing UserDeletedEventHandler\n{out}");
}

// ─── M161.10: event mixed with other items ────────────────────────────────────

#[test]
fn m161_event_mixed_with_const() {
    let src = r#"
module notifications
const MaxRetries: Int = 3
event NotificationSent
  recipient: String
  channel: String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("MAX_RETRIES"), "missing const\n{out}");
    assert!(out.contains("NotificationSentEvent"), "missing event\n{out}");
}

// ─── M161.11: M161 reference in output ────────────────────────────────────────

#[test]
fn m161_m161_reference_in_output() {
    let src = r#"
module audit
event AuditLogged
  actor: String
  action: String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("M161"), "missing M161 reference\n{out}");
}

// ─── M161.12: event struct name has Event suffix ──────────────────────────────

#[test]
fn m161_struct_name_has_event_suffix() {
    let src = r#"
module billing
event InvoiceGenerated
  invoice_id: Int
  total: Float
end
end
"#;
    let out = compile(src);
    // Must be InvoiceGeneratedEvent, NOT InvoiceGenerated (no suffix)
    assert!(
        out.contains("pub struct InvoiceGeneratedEvent"),
        "event struct must end with 'Event'\n{out}"
    );
    // Handler must also use the Event suffix in its handle param
    assert!(
        out.contains("&InvoiceGeneratedEvent"),
        "handler handle() param must reference InvoiceGeneratedEvent\n{out}"
    );
}
