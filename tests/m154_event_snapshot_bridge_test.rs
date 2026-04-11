//! M154 — EventStore snapshot bridge + payload keyword fix tests.
//!
//! Gate:
//! 1. `payload` is now a valid field name (soft keyword fix)
//! 2. TimeSeries stores emit `{S}SnapshotBridge` trait (M154)
//! 3. SnapshotBridge extends both Aggregate and BinaryPersist
//! 4. `snapshot_to` and `resume_from` methods present
//! 5. Audit comment `LOOM[persist:snapshot-bridge]` and M154 reference

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ── payload soft-keyword fix ──────────────────────────────────────────────────

#[test]
fn m154_payload_field_parses_in_timeseries_event() {
    let result = compile_check(
        r#"
module M
  store Events :: TimeSeries
    event Msg :: { ts: Int, payload: String }
  end
end
"#,
    );
    assert!(
        result.is_ok(),
        "payload should be a valid field name in event structs, got: {:?}",
        result.err()
    );
}

#[test]
fn m154_payload_field_emitted_in_struct() {
    let out = compile(
        r#"
module M
  store Events :: TimeSeries
    event Msg :: { ts: Int, payload: String }
  end
end
"#,
    );
    assert!(
        out.contains("pub payload:"),
        "expected payload field in emitted struct\n{out}"
    );
}

#[test]
fn m154_payload_as_field_in_relational_table() {
    let result = compile_check(
        r#"
module M
  store Messages :: Relational
    table Message
      id: String @primary_key
      payload: String
    end
  end
end
"#,
    );
    assert!(
        result.is_ok(),
        "payload should be a valid field name in table structs, got: {:?}",
        result.err()
    );
}

// ── SnapshotBridge trait ──────────────────────────────────────────────────────

#[test]
fn m154_snapshot_bridge_emitted_for_timeseries_store() {
    let out = compile(
        r#"
module M
  store Metrics :: TimeSeries
    event Reading :: { ts: Int, value: Float }
  end
end
"#,
    );
    assert!(
        out.contains("MetricsSnapshotBridge"),
        "expected MetricsSnapshotBridge trait\n{out}"
    );
}

#[test]
fn m154_snapshot_bridge_extends_aggregate_and_binary_persist() {
    let out = compile(
        r#"
module M
  store Ledger :: TimeSeries
    event Entry :: { ts: Int, amount: Float }
  end
end
"#,
    );
    assert!(
        out.contains("LedgerAggregate") && out.contains("BinaryPersist"),
        "expected SnapshotBridge to extend Aggregate + BinaryPersist\n{out}"
    );
}

#[test]
fn m154_snapshot_bridge_has_snapshot_to_method() {
    let out = compile(
        r#"
module M
  store Events :: TimeSeries
    event Tick :: { ts: Int, value: Float }
  end
end
"#,
    );
    assert!(
        out.contains("fn snapshot_to"),
        "expected snapshot_to method in SnapshotBridge\n{out}"
    );
}

#[test]
fn m154_snapshot_bridge_has_resume_from_method() {
    let out = compile(
        r#"
module M
  store Events :: TimeSeries
    event Tick :: { ts: Int, value: Float }
  end
end
"#,
    );
    assert!(
        out.contains("fn resume_from"),
        "expected resume_from method in SnapshotBridge\n{out}"
    );
}

#[test]
fn m154_resume_from_calls_load_snapshot_and_apply() {
    let out = compile(
        r#"
module M
  store Orders :: TimeSeries
    event Placed :: { ts: Int, amount: Float }
  end
end
"#,
    );
    assert!(
        out.contains("load_snapshot"),
        "expected load_snapshot call in resume_from\n{out}"
    );
    assert!(
        out.contains("agg.apply"),
        "expected agg.apply(ev) in resume_from event replay\n{out}"
    );
}

#[test]
fn m154_snapshot_bridge_audit_comment() {
    let out = compile(
        r#"
module M
  store Telemetry :: TimeSeries
    event Sample :: { ts: Int, value: Float }
  end
end
"#,
    );
    assert!(
        out.contains("LOOM[persist:snapshot-bridge]"),
        "expected LOOM[persist:snapshot-bridge] audit comment\n{out}"
    );
    assert!(
        out.contains("M154"),
        "expected M154 reference in audit comment\n{out}"
    );
}

// ── All three pattern pieces present together ─────────────────────────────────

#[test]
fn m154_timeseries_emits_event_store_aggregate_and_bridge() {
    let out = compile(
        r#"
module M
  store Journal :: TimeSeries
    event Commit :: { ts: Int, data: String }
  end
end
"#,
    );
    // EventStore trait
    assert!(out.contains("JournalEventStore"), "expected JournalEventStore\n{out}");
    // Aggregate trait
    assert!(out.contains("JournalAggregate"), "expected JournalAggregate\n{out}");
    // Snapshot bridge (M154)
    assert!(out.contains("JournalSnapshotBridge"), "expected JournalSnapshotBridge\n{out}");
    // Binary persistence (M151)
    assert!(out.contains("BinaryPersist"), "expected BinaryPersist\n{out}");
    // Compressed persistence (M152)
    assert!(out.contains("CompressedBinaryPersist"), "expected CompressedBinaryPersist\n{out}");
}
