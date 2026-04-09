//! Tests for M66: Aspect-Oriented Specification
//! Verifies parsing, AST construction, and AspectChecker validation.

use loom::{ast::*, compile};

// ── Parsing tests ─────────────────────────────────────────────────────────────

#[test]
fn test_aspect_parses_basic_structure() {
    let src = r#"
module Payments
  fn verify_token :: String -> Bool
    true
  end

  aspect SecurityAspect
    pointcut: fn where @requires_auth
    before: verify_token
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "basic aspect should parse: {:?}", result);
}

#[test]
fn test_aspect_parses_all_advice_types() {
    let src = r#"
module Audit
  fn log_entry :: String -> Bool
    true
  end

  fn handle_error :: String -> Bool
    false
  end

  fn measure :: String -> Bool
    true
  end

  aspect ObservabilityAspect
    pointcut: fn where @trace
    before: log_entry
    after: log_entry
    after_throwing: handle_error
    around: measure
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "all advice types should parse: {:?}", result);
}

#[test]
fn test_aspect_parses_retry_logic() {
    let src = r#"
module Network
  fn exponential_backoff :: String -> Bool
    true
  end

  aspect RetryAspect
    pointcut: fn where @idempotent
    on_failure: exponential_backoff
    max_attempts: 3
    order: 2
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "retry aspect should parse: {:?}", result);
}

#[test]
fn test_aspect_parses_compound_pointcut_and() {
    let src = r#"
module Service
  fn audit_record :: String -> Bool
    true
  end

  aspect AuditAspect
    pointcut: fn where @gdpr and effect includes DB
    after: audit_record
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "compound AND pointcut should parse: {:?}", result);
}

#[test]
fn test_aspect_parses_effect_pointcut() {
    let src = r#"
module IO
  fn log_io :: String -> Bool
    true
  end

  aspect IOAspect
    pointcut: fn where effect includes IO
    after: log_io
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "effect-based pointcut should parse: {:?}", result);
}

// ── Checker validation tests ──────────────────────────────────────────────────

#[test]
fn test_aspect_rejects_duplicate_order() {
    let src = r#"
module Payments
  fn verify :: String -> Bool
    true
  end

  fn log_err :: String -> Bool
    false
  end

  aspect AspectA
    pointcut: fn where @auth
    before: verify
    order: 1
  end

  aspect AspectB
    pointcut: fn where @audit
    after: log_err
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "duplicate aspect order should be rejected"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("duplicate aspect order")),
        "error should mention duplicate order: {:?}", errs
    );
}

#[test]
fn test_aspect_rejects_missing_advice_fn() {
    let src = r#"
module Service
  aspect SecurityAspect
    pointcut: fn where @requires_auth
    before: nonexistent_fn
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "missing advice function should be rejected"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("nonexistent_fn")),
        "error should name the missing function: {:?}", errs
    );
}

#[test]
fn test_aspect_emits_doc_comments_in_codegen() {
    let src = r#"
module Payments
  fn verify_token :: String -> Bool
    true
  end

  aspect SecurityAspect
    pointcut: fn where @requires_auth
    before: verify_token
    order: 1
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "compile should succeed: {:?}", result);
    let code = result.unwrap();
    assert!(
        code.contains("// aspect: SecurityAspect"),
        "codegen should emit aspect doc comment:\n{}", code
    );
}
