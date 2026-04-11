/// M168 — `timeout` item: parser + codegen tests.
///
/// `timeout Name duration: N unit: ms|s|min end`
/// generates a `{Name}Timeout` struct with `execute<F,T>()` deadline wrapper.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M168.1: timeout parses without error ─────────────────────────────────────

#[test]
fn m168_timeout_parses() {
    let src = r#"
module resilience
timeout RequestTimeout
  duration: 30
  unit: s
end
end
"#;
    let out = compile(src);
    assert!(out.contains("RequestTimeoutTimeout"), "expected RequestTimeoutTimeout\n{out}");
}

// ─── M168.2: struct fields emitted ────────────────────────────────────────────

#[test]
fn m168_struct_fields_emitted() {
    let src = r#"
module resilience
timeout ApiTimeout
  duration: 5000
  unit: ms
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub duration: u64"), "missing duration field\n{out}");
    assert!(out.contains("pub unit: &'static str"), "missing unit field\n{out}");
}

// ─── M168.3: new() uses configured values ─────────────────────────────────────

#[test]
fn m168_new_uses_configured_values() {
    let src = r#"
module resilience
timeout DbTimeout
  duration: 10
  unit: s
end
end
"#;
    let out = compile(src);
    assert!(out.contains("duration: 10"), "duration not 10\n{out}");
    assert!(out.contains("unit: \"s\""), "unit not s\n{out}");
}

// ─── M168.4: default values when omitted ──────────────────────────────────────

#[test]
fn m168_default_values_when_omitted() {
    let src = r#"
module resilience
timeout SimpleTimeout
end
end
"#;
    let out = compile(src);
    assert!(out.contains("duration: 30"), "default duration should be 30\n{out}");
    assert!(out.contains("unit: \"s\""), "default unit should be s\n{out}");
}

// ─── M168.5: execute<F,T>() method emitted ────────────────────────────────────

#[test]
fn m168_execute_method_emitted() {
    let src = r#"
module resilience
timeout ApiTimeout
  duration: 30
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn execute<F, T>(&self, f: F) -> Result<T, String>"),
        "missing execute method\n{out}"
    );
}

// ─── M168.6: execute uses FnOnce bound ────────────────────────────────────────

#[test]
fn m168_execute_fnonce_bound() {
    let src = r#"
module resilience
timeout ApiTimeout
  duration: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("F: FnOnce() -> T"), "missing FnOnce bound\n{out}");
}

// ─── M168.7: millisecond unit parses ──────────────────────────────────────────

#[test]
fn m168_ms_unit_parses() {
    let src = r#"
module resilience
timeout FastTimeout
  duration: 500
  unit: ms
end
end
"#;
    let out = compile(src);
    assert!(out.contains("unit: \"ms\""), "unit should be ms\n{out}");
    assert!(out.contains("duration: 500"), "duration not 500\n{out}");
}

// ─── M168.8: audit comment emitted ────────────────────────────────────────────

#[test]
fn m168_audit_comment_emitted() {
    let src = r#"
module resilience
timeout SimpleTimeout
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[timeout:resilience]"), "missing audit comment\n{out}");
    assert!(out.contains("M168"), "missing M168 reference\n{out}");
}

// ─── M168.9: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m168_struct_derive_attrs() {
    let src = r#"
module resilience
timeout ApiTimeout
  duration: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

// ─── M168.10: multiple timeouts in one module ─────────────────────────────────

#[test]
fn m168_multiple_timeouts() {
    let src = r#"
module timeouts
timeout FastTimeout
  duration: 100
  unit: ms
end
timeout SlowTimeout
  duration: 5
  unit: min
end
end
"#;
    let out = compile(src);
    assert!(out.contains("FastTimeoutTimeout"), "missing FastTimeout\n{out}");
    assert!(out.contains("SlowTimeoutTimeout"), "missing SlowTimeout\n{out}");
}

// ─── M168.11: mixed with bulkhead and retry ───────────────────────────────────

#[test]
fn m168_mixed_resilience_items() {
    let src = r#"
module resilience
timeout DbTimeout
  duration: 10
  unit: s
end
bulkhead DbBulkhead
  max_concurrent: 5
end
retry DbRetry
  max_attempts: 3
  base_delay: 100
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DbTimeoutTimeout"), "missing timeout\n{out}");
    assert!(out.contains("DbBulkheadBulkhead"), "missing bulkhead\n{out}");
    assert!(out.contains("DbRetryPolicy"), "missing retry\n{out}");
}

// ─── M168.12: minute unit parses ──────────────────────────────────────────────

#[test]
fn m168_min_unit_parses() {
    let out = compile_check(
        r#"
module resilience
timeout LongTimeout
  duration: 2
  unit: min
end
end
"#,
    );
    assert!(out.is_ok(), "min unit should parse: {:?}", out.err());
    let code = out.unwrap();
    assert!(code.contains("unit: \"min\""), "unit should be min");
}
