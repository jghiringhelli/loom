/// M169 — `fallback` item: parser + codegen tests.
///
/// `fallback Name value: "literal" end`
/// generates a `{Name}Fallback<T>` struct with `get() -> &String` method.
/// Uses the existing Token::Fallback (reused from circuit_breaker fallback key).

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M169.1: fallback item parses without error ───────────────────────────────

#[test]
fn m169_fallback_parses() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "service unavailable"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DefaultMessageFallback"), "expected DefaultMessageFallback\n{out}");
}

// ─── M169.2: struct value field emitted ───────────────────────────────────────

#[test]
fn m169_struct_value_field() {
    let src = r#"
module resilience
fallback DefaultResponse
  value: "error"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub value: T"), "missing value field\n{out}");
}

// ─── M169.3: new() contains configured value ──────────────────────────────────

#[test]
fn m169_new_contains_value() {
    let src = r#"
module resilience
fallback NotAvailable
  value: "not available"
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("\"not available\""),
        "value not in output\n{out}"
    );
}

// ─── M169.4: empty fallback (no value key) parses ─────────────────────────────

#[test]
fn m169_empty_fallback_parses() {
    let out = compile_check(
        r#"
module resilience
fallback EmptyFallback
end
end
"#,
    );
    assert!(out.is_ok(), "empty fallback should parse: {:?}", out.err());
}

// ─── M169.5: get() method emitted ─────────────────────────────────────────────

#[test]
fn m169_get_method_emitted() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn get(&self) -> &String"), "missing get() method\n{out}");
}

// ─── M169.6: get() returns &self.value ────────────────────────────────────────

#[test]
fn m169_get_returns_self_value() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("&self.value"), "get() must return &self.value\n{out}");
}

// ─── M169.7: audit comment emitted ────────────────────────────────────────────

#[test]
fn m169_audit_comment_emitted() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[fallback:resilience]"), "missing audit comment\n{out}");
    assert!(out.contains("M169"), "missing M169 reference\n{out}");
}

// ─── M169.8: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m169_struct_derive_attrs() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

// ─── M169.9: new() function emitted ───────────────────────────────────────────

#[test]
fn m169_new_fn_emitted() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn new() -> Self"), "missing new() fn\n{out}");
}

// ─── M169.10: multiple fallbacks in one module ────────────────────────────────

#[test]
fn m169_multiple_fallbacks() {
    let src = r#"
module defaults
fallback ErrorMessage
  value: "error occurred"
end
fallback EmptyResponse
  value: ""
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ErrorMessageFallback"), "missing ErrorMessage\n{out}");
    assert!(out.contains("EmptyResponseFallback"), "missing EmptyResponse\n{out}");
}

// ─── M169.11: mixed with timeout and bulkhead ─────────────────────────────────

#[test]
fn m169_mixed_resilience_items() {
    let src = r#"
module resilience
timeout ApiTimeout
  duration: 30
  unit: s
end
bulkhead ApiBulkhead
  max_concurrent: 10
end
fallback ApiDefault
  value: "unavailable"
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ApiTimeoutTimeout"), "missing timeout\n{out}");
    assert!(out.contains("ApiBulkheadBulkhead"), "missing bulkhead\n{out}");
    assert!(out.contains("ApiDefaultFallback"), "missing fallback\n{out}");
}

// ─── M169.12: generic default type annotation ─────────────────────────────────

#[test]
fn m169_generic_default_type() {
    let src = r#"
module resilience
fallback DefaultMessage
  value: "ok"
end
end
"#;
    let out = compile(src);
    // struct should have generic type param with default = String
    assert!(
        out.contains("{name}Fallback<T = String>") || out.contains("Fallback<T = String>"),
        "missing generic default type\n{out}"
    );
}
