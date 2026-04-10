//! M152 — Compressed binary persistence tests.
//!
//! Gate: every store kind snapshot additionally emits:
//! 1. `CompressedBinaryPersist` trait (once per module with stores)
//!    with `save_compressed` / `load_compressed` methods via flate2 gzip
//! 2. `impl CompressedBinaryPersist for {Name}Snapshot {}`
//! 3. Dependency hint: `flate2 = "1"`
//! 4. File extension convention: `.snap.gz`

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── CompressedBinaryPersist trait presence ────────────────────────────────────

#[test]
fn m152_compressed_trait_emitted_when_store_present() {
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
        out.contains("CompressedBinaryPersist"),
        "expected CompressedBinaryPersist trait\n{out}"
    );
    assert!(
        out.contains("fn save_compressed"),
        "expected save_compressed method"
    );
    assert!(
        out.contains("fn load_compressed"),
        "expected load_compressed method"
    );
}

#[test]
fn m152_compressed_trait_uses_flate2() {
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
        out.contains("flate2"),
        "expected flate2 reference in CompressedBinaryPersist\n{out}"
    );
    assert!(
        out.contains("GzEncoder") || out.contains("GzDecoder"),
        "expected GzEncoder or GzDecoder in compressed trait\n{out}"
    );
}

#[test]
fn m152_compressed_trait_extends_binary_persist() {
    let out = compile(
        r#"
module M
  store Sessions :: KeyValue
    key: String
    value: String
  end
end
"#,
    );
    // CompressedBinaryPersist must be a supertrait of BinaryPersist
    assert!(
        out.contains("CompressedBinaryPersist: BinaryPersist"),
        "expected CompressedBinaryPersist: BinaryPersist supertrait\n{out}"
    );
}

#[test]
fn m152_compressed_trait_not_emitted_without_stores() {
    let out = compile(
        r#"
module M
  fn greet :: String -> String
    body
      todo
    end
  end
end
"#,
    );
    assert!(
        !out.contains("CompressedBinaryPersist"),
        "CompressedBinaryPersist must not appear when no stores are declared"
    );
}

// ── impl CompressedBinaryPersist per snapshot ─────────────────────────────────

#[test]
fn m152_relational_snapshot_implements_compressed_persist() {
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
        out.contains("impl CompressedBinaryPersist for OrdersSnapshot"),
        "expected impl CompressedBinaryPersist for OrdersSnapshot\n{out}"
    );
}

#[test]
fn m152_keyvalue_snapshot_implements_compressed_persist() {
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
        out.contains("impl CompressedBinaryPersist for ConfigSnapshot"),
        "expected impl CompressedBinaryPersist for ConfigSnapshot\n{out}"
    );
}

#[test]
fn m152_document_snapshot_implements_compressed_persist() {
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
        out.contains("impl CompressedBinaryPersist for PostsSnapshot"),
        "expected impl CompressedBinaryPersist for PostsSnapshot\n{out}"
    );
}

#[test]
fn m152_timeseries_snapshot_implements_compressed_persist() {
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
        out.contains("impl CompressedBinaryPersist for MetricsSnapshot"),
        "expected impl CompressedBinaryPersist for MetricsSnapshot\n{out}"
    );
}

// ── Both traits present together ──────────────────────────────────────────────

#[test]
fn m152_both_persist_traits_emitted_together() {
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
        out.contains("BinaryPersist"),
        "expected BinaryPersist trait\n{out}"
    );
    assert!(
        out.contains("CompressedBinaryPersist"),
        "expected CompressedBinaryPersist trait\n{out}"
    );
    // Both traits emitted exactly once each
    let binary_count = out.matches("pub trait BinaryPersist").count();
    let compressed_count = out.matches("pub trait CompressedBinaryPersist").count();
    assert_eq!(binary_count, 1, "BinaryPersist trait should appear exactly once");
    assert_eq!(compressed_count, 1, "CompressedBinaryPersist should appear exactly once");
}

#[test]
fn m152_multiple_stores_each_get_compressed_impl() {
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
        out.contains("impl CompressedBinaryPersist for UsersSnapshot"),
        "expected UsersSnapshot compressed impl\n{out}"
    );
    assert!(
        out.contains("impl CompressedBinaryPersist for SessionsSnapshot"),
        "expected SessionsSnapshot compressed impl"
    );
}

// ── Dep hint ──────────────────────────────────────────────────────────────────

#[test]
fn m152_flate2_dep_hint_in_compressed_trait() {
    let out = compile(
        r#"
module M
  store Ledger :: Relational
    table Entry
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    // The dep hint "flate2 = " must appear in the output so developers know what to add
    assert!(
        out.contains("flate2"),
        "expected flate2 dep hint in compressed trait block\n{out}"
    );
}

#[test]
fn m152_snap_gz_extension_mentioned() {
    let out = compile(
        r#"
module M
  store Events :: TimeSeries
    event Evt :: { ts: Int, data: String }
  end
end
"#,
    );
    // File extension convention must be documented in emitted code
    assert!(
        out.contains(".snap.gz"),
        "expected .snap.gz extension convention in emitted comments\n{out}"
    );
}
