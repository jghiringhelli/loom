//! M151 — Binary persistence tests.
//!
//! Gate: every store kind emits:
//! 1. `serde::Serialize + serde::Deserialize` on entity structs
//! 2. A `{Name}Snapshot` struct with `created_at_secs: i64` and per-entity `Vec<T>` fields
//! 3. `impl BinaryPersist for {Name}Snapshot {}`
//! 4. The `BinaryPersist` trait is emitted once per module with stores
//!    (containing `save_snapshot` and `load_snapshot` methods)

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── BinaryPersist trait presence ──────────────────────────────────────────────

#[test]
fn m151_binary_persist_trait_emitted_when_store_present() {
    let out = compile(
        r#"
module M
  store Orders :: Relational
    table Order
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("BinaryPersist"),
        "expected BinaryPersist trait\n{out}"
    );
    assert!(
        out.contains("fn save_snapshot"),
        "expected save_snapshot method"
    );
    assert!(
        out.contains("fn load_snapshot"),
        "expected load_snapshot method"
    );
}

#[test]
fn m151_binary_persist_trait_not_emitted_without_stores() {
    let out = compile(
        r#"
module M
  fn add :: Int -> Int -> Int
    body
      todo
    end
  end
end
"#,
    );
    assert!(
        !out.contains("BinaryPersist"),
        "BinaryPersist must not appear when no stores are declared"
    );
}

#[test]
fn m151_binary_persist_uses_bincode() {
    let out = compile(
        r#"
module M
  store Prices :: KeyValue
    key: String
    value: Float
  end
end
"#,
    );
    assert!(
        out.contains("bincode"),
        "expected bincode reference in BinaryPersist impl\n{out}"
    );
    assert!(
        out.contains("serde"),
        "expected serde in BinaryPersist trait bounds"
    );
}

// ── Serde derives on entity structs ───────────────────────────────────────────

#[test]
fn m151_relational_struct_has_serde_derive() {
    let out = compile(
        r#"
module M
  store Users :: Relational
    table User
      id: String @primary_key
      name: String
    end
  end
end
"#,
    );
    assert!(
        out.contains("serde::Serialize") || out.contains("Serialize"),
        "expected Serialize derive on User struct\n{out}"
    );
    assert!(
        out.contains("serde::Deserialize") || out.contains("Deserialize"),
        "expected Deserialize derive on User struct\n{out}"
    );
}

#[test]
fn m151_keyvalue_struct_has_serde_derive() {
    let out = compile(
        r#"
module M
  store Cache :: KeyValue
    key: String
    value: String
  end
end
"#,
    );
    assert!(
        out.contains("Serialize"),
        "expected Serialize derive on KV entry struct\n{out}"
    );
    assert!(
        out.contains("Deserialize"),
        "expected Deserialize derive on KV entry struct\n{out}"
    );
}

#[test]
fn m151_document_struct_has_serde_derive() {
    let out = compile(
        r#"
module M
  store Posts :: Document
    collection Post
      id: String
      body: String
    end
  end
end
"#,
    );
    assert!(
        out.contains("Serialize"),
        "expected Serialize derive on document struct\n{out}"
    );
}

// ── Snapshot struct ───────────────────────────────────────────────────────────

#[test]
fn m151_relational_snapshot_struct_emitted() {
    let out = compile(
        r#"
module M
  store Orders :: Relational
    table Order
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("OrdersSnapshot"),
        "expected OrdersSnapshot struct\n{out}"
    );
    assert!(
        out.contains("created_at_secs: i64"),
        "expected created_at_secs field"
    );
    assert!(
        out.contains("Vec<Order>"),
        "expected Vec<Order> in snapshot struct"
    );
}

#[test]
fn m151_snapshot_implements_binary_persist() {
    let out = compile(
        r#"
module M
  store Orders :: Relational
    table Order
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("impl BinaryPersist for OrdersSnapshot"),
        "expected impl BinaryPersist for OrdersSnapshot\n{out}"
    );
}

#[test]
fn m151_timeseries_snapshot_emitted() {
    let out = compile(
        r#"
module M
  store Metrics :: TimeSeries
    event Tick :: { ts: Int, value: Float }
  end
end
"#,
    );
    assert!(
        out.contains("MetricsSnapshot"),
        "expected MetricsSnapshot struct\n{out}"
    );
    assert!(
        out.contains("impl BinaryPersist for MetricsSnapshot"),
        "expected BinaryPersist impl for MetricsSnapshot"
    );
}

#[test]
fn m151_graph_store_snapshot_emitted() {
    let out = compile(
        r#"
module M
  store Net :: Graph
    node Person :: { id: String }
    edge Knows :: Person -> Person { since: Int }
  end
end
"#,
    );
    assert!(
        out.contains("NetSnapshot"),
        "expected NetSnapshot struct\n{out}"
    );
    assert!(
        out.contains("Vec<Person>"),
        "expected Vec<Person> in snapshot"
    );
}

#[test]
fn m151_keyvalue_snapshot_uses_records_when_no_entity_types() {
    let out = compile(
        r#"
module M
  store Config :: KeyValue
    key: String
    value: String
  end
end
"#,
    );
    assert!(
        out.contains("ConfigSnapshot"),
        "expected ConfigSnapshot struct\n{out}"
    );
    // KV stores have no named entity types — should use generic records field
    assert!(
        out.contains("impl BinaryPersist for ConfigSnapshot"),
        "expected BinaryPersist impl for ConfigSnapshot\n{out}"
    );
}

#[test]
fn m151_multiple_stores_each_get_snapshot() {
    let out = compile(
        r#"
module M
  store Users :: Relational
    table User
      id: String @primary_key
    end
  end
  store Sessions :: KeyValue
    key: String
    value: String
  end
end
"#,
    );
    assert!(
        out.contains("UsersSnapshot"),
        "expected UsersSnapshot\n{out}"
    );
    assert!(
        out.contains("SessionsSnapshot"),
        "expected SessionsSnapshot"
    );
    // Trait should only appear once
    let count = out.matches("pub trait BinaryPersist").count();
    assert_eq!(
        count, 1,
        "BinaryPersist trait should be emitted exactly once, got {count}"
    );
}

// ── Dep hint ──────────────────────────────────────────────────────────────────

#[test]
fn m151_dep_hint_emitted_in_store_header() {
    let out = compile(
        r#"
module M
  store Logs :: TimeSeries
    event Log :: { ts: Int, msg: String }
  end
end
"#,
    );
    assert!(
        out.contains("bincode") && out.contains("serde"),
        "expected bincode + serde dep hints near store header\n{out}"
    );
}
