/// M164 — `retry` item: parser + codegen tests.
///
/// `retry Name max_attempts: N base_delay: N multiplier: N on: ErrorType end`
/// implements exponential backoff retry policy (Tanenbaum & Van Steen 2007).

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M164.1: retry parses without error ───────────────────────────────────────

#[test]
fn m164_retry_parses() {
    let src = r#"
module payments
retry PaymentRetry
  max_attempts: 3
  base_delay: 100
  multiplier: 2
end
end
"#;
    let out = compile(src);
    assert!(out.contains("PaymentRetryPolicy"), "expected PaymentRetryPolicy\n{out}");
}

// ─── M164.2: struct fields emitted ────────────────────────────────────────────

#[test]
fn m164_struct_fields_emitted() {
    let src = r#"
module ops
retry NetworkRetry
  max_attempts: 5
  base_delay: 200
  multiplier: 3
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub max_attempts: u32"), "missing max_attempts\n{out}");
    assert!(out.contains("pub base_delay_ms: u64"), "missing base_delay_ms\n{out}");
    assert!(out.contains("pub multiplier: u32"), "missing multiplier\n{out}");
}

// ─── M164.3: new() uses configured values ─────────────────────────────────────

#[test]
fn m164_new_uses_configured_values() {
    let src = r#"
module ops
retry DatabaseRetry
  max_attempts: 4
  base_delay: 500
  multiplier: 2
end
end
"#;
    let out = compile(src);
    assert!(out.contains("max_attempts: 4"), "max_attempts not 4\n{out}");
    assert!(out.contains("base_delay_ms: 500"), "base_delay not 500\n{out}");
    assert!(out.contains("multiplier: 2"), "multiplier not 2\n{out}");
}

// ─── M164.4: default values when omitted ──────────────────────────────────────

#[test]
fn m164_default_values_when_omitted() {
    let src = r#"
module ops
retry SimpleRetry
end
end
"#;
    let out = compile(src);
    assert!(out.contains("max_attempts: 3"), "default max_attempts should be 3\n{out}");
    assert!(out.contains("base_delay_ms: 100"), "default base_delay should be 100\n{out}");
    assert!(out.contains("multiplier: 2"), "default multiplier should be 2\n{out}");
}

// ─── M164.5: execute<F,T,E>() method emitted ──────────────────────────────────

#[test]
fn m164_execute_method_emitted() {
    let src = r#"
module ops
retry ApiRetry
  max_attempts: 3
  base_delay: 100
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn execute<F, T, E>(&self, f: F) -> Result<T, E>"),
        "missing execute method\n{out}"
    );
}

// ─── M164.6: execute generic bounds ───────────────────────────────────────────

#[test]
fn m164_execute_generic_bounds() {
    let src = r#"
module ops
retry ApiRetry
  max_attempts: 3
  base_delay: 100
end
end
"#;
    let out = compile(src);
    assert!(out.contains("F: Fn() -> Result<T, E>"), "missing Fn bound\n{out}");
    assert!(out.contains("E: std::fmt::Debug"), "missing Debug bound\n{out}");
}

// ─── M164.7: audit comment emitted ────────────────────────────────────────────

#[test]
fn m164_audit_comment_emitted() {
    let src = r#"
module ops
retry SimpleRetry
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[retry:resilience]"), "missing audit comment\n{out}");
    assert!(out.contains("M164"), "missing M164 reference\n{out}");
}

// ─── M164.8: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m164_struct_derive_attrs() {
    let src = r#"
module ops
retry ApiRetry
  max_attempts: 3
  base_delay: 100
end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

// ─── M164.9: on: error type parses ────────────────────────────────────────────

#[test]
fn m164_on_error_parses() {
    let out = compile_check(
        r#"
module ops
retry NetworkRetry
  max_attempts: 3
  base_delay: 100
  on: NetworkError
end
end
"#,
    );
    assert!(out.is_ok(), "retry with on: should parse: {:?}", out.err());
}

// ─── M164.10: multiple retry policies in one module ───────────────────────────

#[test]
fn m164_multiple_retry_policies() {
    let src = r#"
module resilience
retry DatabaseRetry
  max_attempts: 5
  base_delay: 200
end
retry HttpRetry
  max_attempts: 3
  base_delay: 50
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DatabaseRetryPolicy"), "missing DatabaseRetry\n{out}");
    assert!(out.contains("HttpRetryPolicy"), "missing HttpRetry\n{out}");
}

// ─── M164.11: retry mixed with circuit_breaker ────────────────────────────────

#[test]
fn m164_mixed_with_circuit_breaker() {
    let src = r#"
module resilience
retry DatabaseRetry
  max_attempts: 3
  base_delay: 100
end
circuit_breaker ExternalApi
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("DatabaseRetryPolicy"), "missing retry\n{out}");
    assert!(out.contains("ExternalApiCircuitBreaker"), "missing circuit breaker\n{out}");
}

// ─── M164.12: execute contains todo!() stub ───────────────────────────────────

#[test]
fn m164_execute_has_todo_stub() {
    let src = r#"
module ops
retry SimpleRetry
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("todo!(\"implement retry with exponential backoff\")"),
        "execute must have todo! stub\n{out}"
    );
}
