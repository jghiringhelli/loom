/// M163 — `circuit_breaker` item: parser + codegen tests.
///
/// `circuit_breaker Name threshold: N timeout: N fallback: name end`
/// implements the Circuit Breaker pattern (Nygard 2007, "Release It!").

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M163.1: circuit_breaker parses without error ─────────────────────────────

#[test]
fn m163_circuit_breaker_parses() {
    let src = r#"
module payments
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
  fallback: use_cache
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("PaymentGatewayCircuitBreaker"),
        "expected PaymentGatewayCircuitBreaker\n{out}"
    );
}

// ─── M163.2: state enum emitted ───────────────────────────────────────────────

#[test]
fn m163_circuit_state_enum_emitted() {
    let src = r#"
module payments
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub enum PaymentGatewayCircuitState"),
        "missing state enum\n{out}"
    );
    assert!(out.contains("Closed"), "missing Closed variant\n{out}");
    assert!(out.contains("Open"), "missing Open variant\n{out}");
    assert!(out.contains("HalfOpen"), "missing HalfOpen variant\n{out}");
}

// ─── M163.3: struct fields emitted ────────────────────────────────────────────

#[test]
fn m163_struct_fields_emitted() {
    let src = r#"
module payments
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub failure_threshold: u32"), "missing failure_threshold\n{out}");
    assert!(out.contains("pub timeout_secs: u64"), "missing timeout_secs\n{out}");
    assert!(
        out.contains("pub state: PaymentGatewayCircuitState"),
        "missing state field\n{out}"
    );
}

// ─── M163.4: new() uses configured values ─────────────────────────────────────

#[test]
fn m163_new_uses_configured_values() {
    let src = r#"
module payments
circuit_breaker SlowService
  threshold: 3
  timeout: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("failure_threshold: 3"), "threshold not 3\n{out}");
    assert!(out.contains("timeout_secs: 60"), "timeout not 60\n{out}");
}

// ─── M163.5: call<F,T> method emitted ─────────────────────────────────────────

#[test]
fn m163_call_method_emitted() {
    let src = r#"
module integrations
circuit_breaker ExternalApi
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn call<F, T>(&mut self, f: F) -> Result<T, String>"),
        "missing call method\n{out}"
    );
}

// ─── M163.6: fallback fn emitted when declared ────────────────────────────────

#[test]
fn m163_fallback_fn_emitted() {
    let src = r#"
module payments
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
  fallback: use_cache
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn fallback_use_cache"),
        "missing fallback_use_cache fn\n{out}"
    );
}

// ─── M163.7: no fallback fn when not declared ─────────────────────────────────

#[test]
fn m163_no_fallback_when_not_declared() {
    let src = r#"
module integrations
circuit_breaker ExternalApi
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(
        !out.contains("pub fn fallback_"),
        "should not emit fallback fn when not declared\n{out}"
    );
}

// ─── M163.8: audit comment emitted ────────────────────────────────────────────

#[test]
fn m163_audit_comment_emitted() {
    let src = r#"
module integrations
circuit_breaker ExternalApi
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[circuit_breaker:resilience]"),
        "missing audit comment\n{out}"
    );
    assert!(out.contains("M163"), "missing M163 reference\n{out}");
    assert!(out.contains("Nygard"), "missing Nygard reference\n{out}");
}

// ─── M163.9: default threshold when omitted ───────────────────────────────────

#[test]
fn m163_default_threshold_when_omitted() {
    let src = r#"
module ops
circuit_breaker SimpleBreaker
end
end
"#;
    let out = compile(src);
    // Default threshold = 5, default timeout = 30
    assert!(out.contains("failure_threshold: 5"), "default threshold should be 5\n{out}");
    assert!(out.contains("timeout_secs: 30"), "default timeout should be 30\n{out}");
}

// ─── M163.10: multiple circuit breakers in one module ─────────────────────────

#[test]
fn m163_multiple_circuit_breakers() {
    let src = r#"
module resilience
circuit_breaker PaymentService
  threshold: 3
  timeout: 20
end
circuit_breaker EmailService
  threshold: 5
  timeout: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("PaymentServiceCircuitBreaker"), "missing PaymentService\n{out}");
    assert!(out.contains("EmailServiceCircuitBreaker"), "missing EmailService\n{out}");
}

// ─── M163.11: circuit_breaker mixed with command ──────────────────────────────

#[test]
fn m163_mixed_with_command() {
    let src = r#"
module checkout
command PlaceOrder
  order_id: Int
end
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    assert!(out.contains("PlaceOrderCommand"), "missing command\n{out}");
    assert!(out.contains("PaymentGatewayCircuitBreaker"), "missing circuit breaker\n{out}");
}

// ─── M163.12: struct has #[derive(Debug, Clone)] ──────────────────────────────

#[test]
fn m163_struct_derive_attrs() {
    let src = r#"
module payments
circuit_breaker PaymentGateway
  threshold: 5
  timeout: 30
end
end
"#;
    let out = compile(src);
    // The struct must derive Debug + Clone (state enum gets PartialEq too)
    assert!(out.contains("#[derive(Debug, Clone)]"), "struct missing derive\n{out}");
    assert!(
        out.contains("#[derive(Debug, Clone, PartialEq)]"),
        "state enum missing derive\n{out}"
    );
}
