/// M167 — `bulkhead` item: parser + codegen tests.
///
/// `bulkhead Name max_concurrent: N queue_size: N end`
/// implements concurrency isolation (Nygard 2007 "Release It!").

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M167.1: bulkhead parses without error ────────────────────────────────────

#[test]
fn m167_bulkhead_parses() {
    let src = r#"
module resilience
bulkhead DatabaseBulkhead
  max_concurrent: 10
  queue_size: 5
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DatabaseBulkheadBulkhead"), "expected DatabaseBulkheadBulkhead\n{out}");
}

// ─── M167.2: struct fields emitted ────────────────────────────────────────────

#[test]
fn m167_struct_fields_emitted() {
    let src = r#"
module resilience
bulkhead ServiceBulkhead
  max_concurrent: 20
  queue_size: 10
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub max_concurrent: u64"), "missing max_concurrent\n{out}");
    assert!(out.contains("pub queue_size: u64"), "missing queue_size\n{out}");
}

// ─── M167.3: new() uses configured values ─────────────────────────────────────

#[test]
fn m167_new_uses_configured_values() {
    let src = r#"
module resilience
bulkhead HttpBulkhead
  max_concurrent: 15
  queue_size: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("max_concurrent: 15"), "max_concurrent not 15\n{out}");
    assert!(out.contains("queue_size: 30"), "queue_size not 30\n{out}");
}

// ─── M167.4: default values when omitted ──────────────────────────────────────

#[test]
fn m167_default_values_when_omitted() {
    let src = r#"
module resilience
bulkhead SimpleBulkhead
end
end
"#;
    let out = compile(src);
    assert!(out.contains("max_concurrent: 10"), "default max_concurrent should be 10\n{out}");
    assert!(out.contains("queue_size: 0"), "default queue_size should be 0\n{out}");
}

// ─── M167.5: execute<F,T,E>() method emitted ──────────────────────────────────

#[test]
fn m167_execute_method_emitted() {
    let src = r#"
module resilience
bulkhead ApiBulkhead
  max_concurrent: 5
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn execute<F, T, E>(&self, f: F) -> Result<T, E>"),
        "missing execute method\n{out}"
    );
}

// ─── M167.6: execute uses FnOnce bound ────────────────────────────────────────

#[test]
fn m167_execute_fnonce_bound() {
    let src = r#"
module resilience
bulkhead ApiBulkhead
  max_concurrent: 5
end
end
"#;
    let out = compile(src);
    assert!(out.contains("F: FnOnce() -> Result<T, E>"), "missing FnOnce bound\n{out}");
}

// ─── M167.7: available() method emitted ───────────────────────────────────────

#[test]
fn m167_available_method_emitted() {
    let src = r#"
module resilience
bulkhead ApiBulkhead
  max_concurrent: 5
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn available(&self) -> bool"), "missing available() method\n{out}");
}

// ─── M167.8: audit comment emitted ────────────────────────────────────────────

#[test]
fn m167_audit_comment_emitted() {
    let src = r#"
module resilience
bulkhead ApiBulkhead
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[bulkhead:resilience]"), "missing audit comment\n{out}");
    assert!(out.contains("M167"), "missing M167 reference\n{out}");
    assert!(out.contains("Nygard"), "missing Nygard attribution\n{out}");
}

// ─── M167.9: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m167_struct_derive_attrs() {
    let src = r#"
module resilience
bulkhead ApiBulkhead
  max_concurrent: 5
end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

// ─── M167.10: multiple bulkheads in one module ────────────────────────────────

#[test]
fn m167_multiple_bulkheads() {
    let src = r#"
module isolation
bulkhead DatabaseBulkhead
  max_concurrent: 5
end
bulkhead HttpBulkhead
  max_concurrent: 20
  queue_size: 10
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DatabaseBulkheadBulkhead"), "missing DatabaseBulkhead\n{out}");
    assert!(out.contains("HttpBulkheadBulkhead"), "missing HttpBulkhead\n{out}");
}

// ─── M167.11: mixed with circuit_breaker and retry ────────────────────────────

#[test]
fn m167_mixed_resilience_items() {
    let src = r#"
module resilience
bulkhead ServiceBulkhead
  max_concurrent: 10
end
circuit_breaker ExternalApi
  threshold: 5
  timeout: 30
end
retry NetworkRetry
  max_attempts: 3
  base_delay: 100
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ServiceBulkheadBulkhead"), "missing bulkhead\n{out}");
    assert!(out.contains("ExternalApiCircuitBreaker"), "missing circuit breaker\n{out}");
    assert!(out.contains("NetworkRetryPolicy"), "missing retry\n{out}");
}

// ─── M167.12: zero queue_size (no queue) parses correctly ─────────────────────

#[test]
fn m167_zero_queue_parses() {
    let out = compile_check(
        r#"
module strict
bulkhead TightBulkhead
  max_concurrent: 3
  queue_size: 0
end
end
"#,
    );
    assert!(out.is_ok(), "queue_size: 0 should parse: {:?}", out.err());
    let code = out.unwrap();
    assert!(code.contains("queue_size: 0"), "queue_size should be 0");
}
