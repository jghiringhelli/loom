/// M165 — `rate_limiter` item: parser + codegen tests.
///
/// `rate_limiter Name requests: N per: N burst: N end`
/// implements token-bucket rate limiting (Anderson 1990).

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M165.1: rate_limiter parses without error ────────────────────────────────

#[test]
fn m165_rate_limiter_parses() {
    let src = r#"
module api
rate_limiter ApiLimiter
  requests: 100
  per: 60
  burst: 20
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ApiLimiterRateLimiter"), "expected ApiLimiterRateLimiter\n{out}");
}

// ─── M165.2: struct fields emitted ────────────────────────────────────────────

#[test]
fn m165_struct_fields_emitted() {
    let src = r#"
module api
rate_limiter ApiLimiter
  requests: 200
  per: 30
  burst: 50
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub requests_per_window: u64"), "missing requests_per_window\n{out}");
    assert!(out.contains("pub window_secs: u64"), "missing window_secs\n{out}");
    assert!(out.contains("pub burst_capacity: u64"), "missing burst_capacity\n{out}");
}

// ─── M165.3: new() uses configured values ─────────────────────────────────────

#[test]
fn m165_new_uses_configured_values() {
    let src = r#"
module api
rate_limiter PaymentGateway
  requests: 50
  per: 10
  burst: 5
end
end
"#;
    let out = compile(src);
    assert!(out.contains("requests_per_window: 50"), "requests not 50\n{out}");
    assert!(out.contains("window_secs: 10"), "per not 10\n{out}");
    assert!(out.contains("burst_capacity: 5"), "burst not 5\n{out}");
}

// ─── M165.4: default values when all keys omitted ─────────────────────────────

#[test]
fn m165_default_values_when_omitted() {
    let src = r#"
module api
rate_limiter SimpleLimiter
end
end
"#;
    let out = compile(src);
    assert!(out.contains("requests_per_window: 100"), "default requests should be 100\n{out}");
    assert!(out.contains("window_secs: 60"), "default per should be 60\n{out}");
    assert!(out.contains("burst_capacity: 0"), "default burst should be 0\n{out}");
}

// ─── M165.5: allow() method emitted ───────────────────────────────────────────

#[test]
fn m165_allow_method_emitted() {
    let src = r#"
module api
rate_limiter ApiLimiter
  requests: 100
  per: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn allow(&self) -> bool"), "missing allow() method\n{out}");
}

// ─── M165.6: allow() contains todo!() stub ────────────────────────────────────

#[test]
fn m165_allow_has_todo_stub() {
    let src = r#"
module api
rate_limiter ApiLimiter
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("todo!(\"implement token bucket allow()\")"),
        "allow must have todo! stub\n{out}"
    );
}

// ─── M165.7: audit comment emitted ────────────────────────────────────────────

#[test]
fn m165_audit_comment_emitted() {
    let src = r#"
module api
rate_limiter ApiLimiter
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[rate_limiter:resilience]"), "missing audit comment\n{out}");
    assert!(out.contains("M165"), "missing M165 reference\n{out}");
}

// ─── M165.8: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m165_struct_derive_attrs() {
    let src = r#"
module api
rate_limiter ApiLimiter
  requests: 100
  per: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[derive(Debug, Clone)]"), "missing derive\n{out}");
}

// ─── M165.9: new() function emitted ───────────────────────────────────────────

#[test]
fn m165_new_fn_emitted() {
    let src = r#"
module api
rate_limiter ApiLimiter
  requests: 100
  per: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn new() -> Self"), "missing new() fn\n{out}");
}

// ─── M165.10: multiple limiters in one module ─────────────────────────────────

#[test]
fn m165_multiple_limiters() {
    let src = r#"
module limits
rate_limiter ApiLimiter
  requests: 100
  per: 60
end
rate_limiter DatabaseLimiter
  requests: 20
  per: 1
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ApiLimiterRateLimiter"), "missing ApiLimiter\n{out}");
    assert!(out.contains("DatabaseLimiterRateLimiter"), "missing DatabaseLimiter\n{out}");
}

// ─── M165.11: mixed with retry ────────────────────────────────────────────────

#[test]
fn m165_mixed_with_retry() {
    let src = r#"
module resilience
retry HttpRetry
  max_attempts: 3
  base_delay: 100
end
rate_limiter HttpLimiter
  requests: 50
  per: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("HttpRetryPolicy"), "missing retry\n{out}");
    assert!(out.contains("HttpLimiterRateLimiter"), "missing rate limiter\n{out}");
}

// ─── M165.12: burst=0 (no burst) parses cleanly ───────────────────────────────

#[test]
fn m165_zero_burst_parses() {
    let out = compile_check(
        r#"
module strict
rate_limiter StrictLimiter
  requests: 10
  per: 1
  burst: 0
end
end
"#,
    );
    assert!(out.is_ok(), "burst: 0 should parse: {:?}", out.err());
    let result = out.unwrap();
    assert!(result.contains("burst_capacity: 0"), "burst_capacity should be 0");
}
