/// M66 — Aspect-Oriented cross-cutting specification.
///
/// `aspect Name pointcut: fn where @annotation before: fn after: fn order: N end`
/// Emits: `// LOOM[aspect:Name]` + trait + struct + impl block.

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

// ─── Parse: minimal aspect ────────────────────────────────────────────────────

#[test]
fn aspect_minimal_parses() {
    let out = ok("module M\n\
         aspect SecurityAspect\n\
         end\n\
         end\n");
    assert!(out.contains("SecurityAspect"), "output:\n{}", out);
}

// ─── Parse: aspect with before advice ────────────────────────────────────────

#[test]
fn aspect_with_before_emits_before_method() {
    let out = ok("module M\n\
         fn verify_token :: Unit\n\
         end\n\
         aspect AuthAspect\n\
           before: verify_token\n\
         end\n\
         end\n");
    assert!(out.contains("verify_token"), "output:\n{}", out);
    assert!(out.contains("AuthAspect"), "output:\n{}", out);
}

// ─── Parse: aspect with after advice ─────────────────────────────────────────

#[test]
fn aspect_with_after_emits_after_method() {
    let out = ok("module M\n\
         fn emit_audit_record :: Unit\n\
         end\n\
         aspect AuditAspect\n\
           after: emit_audit_record\n\
         end\n\
         end\n");
    assert!(out.contains("emit_audit_record"), "output:\n{}", out);
    assert!(out.contains("after"), "output:\n{}", out);
}

// ─── Parse: aspect with after_throwing advice ─────────────────────────────────

#[test]
fn aspect_with_after_throwing_emits_method() {
    let out = ok("module M\n\
         fn log_security_event :: Unit\n\
         end\n\
         aspect ErrorAspect\n\
           after_throwing: log_security_event\n\
         end\n\
         end\n");
    assert!(out.contains("log_security_event"), "output:\n{}", out);
    assert!(out.contains("after_throwing"), "output:\n{}", out);
}

// ─── Parse: aspect with around advice ────────────────────────────────────────

#[test]
fn aspect_with_around_parses() {
    let out = ok("module M\n\
         fn time_execution :: Unit\n\
         end\n\
         aspect TimingAspect\n\
           around: time_execution\n\
         end\n\
         end\n");
    assert!(out.contains("TimingAspect"), "output:\n{}", out);
}

// ─── Parse: aspect with order ─────────────────────────────────────────────────

#[test]
fn aspect_with_order_emits_order_annotation() {
    let out = ok("module M\n\
         fn do_first :: Unit\n\
         end\n\
         aspect OrderedAspect\n\
           before: do_first\n\
           order: 2\n\
         end\n\
         end\n");
    assert!(out.contains("order: 2"), "output:\n{}", out);
}

// ─── Parse: aspect with on_failure ───────────────────────────────────────────

#[test]
fn aspect_with_on_failure_parses() {
    let out = ok("module M\n\
         fn handle_failure :: Unit\n\
         end\n\
         aspect RetryAspect\n\
           on_failure: handle_failure\n\
           max_attempts: 3\n\
         end\n\
         end\n");
    assert!(out.contains("RetryAspect"), "output:\n{}", out);
}

// ─── Parse: aspect with pointcut using fn where ───────────────────────────────

#[test]
fn aspect_with_pointcut_fn_where_annotation_parses() {
    let out = ok("module M\n\
         fn verify_token :: Unit\n\
         end\n\
         aspect SecurityAspect\n\
           pointcut: fn where @requires_auth\n\
           before: verify_token\n\
           order: 1\n\
         end\n\
         end\n");
    assert!(out.contains("SecurityAspect"), "output:\n{}", out);
    assert!(out.contains("verify_token"), "output:\n{}", out);
}

// ─── Emitter: trait is emitted ────────────────────────────────────────────────

#[test]
fn aspect_emits_trait_definition() {
    let out = ok("module M\n\
         fn log_entry :: Unit\n\
         end\n\
         fn log_exit :: Unit\n\
         end\n\
         aspect LoggingAspect\n\
           before: log_entry\n\
           after: log_exit\n\
         end\n\
         end\n");
    assert!(
        out.contains("LoggingAspectTrait"),
        "expected trait, output:\n{}",
        out
    );
}

// ─── Emitter: struct is emitted ───────────────────────────────────────────────

#[test]
fn aspect_emits_struct() {
    let out = ok("module M\n\
         fn populate_cache :: Unit\n\
         end\n\
         aspect CacheAspect\n\
           before: populate_cache\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub struct CacheAspect"),
        "expected struct, output:\n{}",
        out
    );
}

// ─── Emitter: impl block is emitted ──────────────────────────────────────────

#[test]
fn aspect_emits_impl_block() {
    let out = ok("module M\n\
         fn begin_tx :: Unit\n\
         end\n\
         fn commit_tx :: Unit\n\
         end\n\
         fn rollback_tx :: Unit\n\
         end\n\
         aspect TransactionAspect\n\
           before: begin_tx\n\
           after: commit_tx\n\
           after_throwing: rollback_tx\n\
         end\n\
         end\n");
    assert!(
        out.contains("impl TransactionAspectTrait for TransactionAspect"),
        "expected impl, output:\n{}",
        out
    );
    assert!(out.contains("begin_tx"), "output:\n{}", out);
    assert!(out.contains("commit_tx"), "output:\n{}", out);
    assert!(out.contains("rollback_tx"), "output:\n{}", out);
}

// ─── Multiple aspects in one module ──────────────────────────────────────────

#[test]
fn multiple_aspects_in_one_module() {
    let out = ok("module M\n\
         fn verify_token :: Unit\n\
         end\n\
         fn emit_audit :: Unit\n\
         end\n\
         aspect AuthAspect\n\
           before: verify_token\n\
           order: 1\n\
         end\n\
         aspect AuditAspect\n\
           after: emit_audit\n\
           order: 2\n\
         end\n\
         end\n");
    assert!(out.contains("AuthAspect"), "output:\n{}", out);
    assert!(out.contains("AuditAspect"), "output:\n{}", out);
}

// ─── LOOM annotation is present ───────────────────────────────────────────────

#[test]
fn aspect_emits_loom_annotation_comment() {
    let out = ok("module M\n\
         fn check_auth :: Unit\n\
         end\n\
         aspect SecurityAspect\n\
           before: check_auth\n\
           order: 1\n\
         end\n\
         end\n");
    assert!(
        out.contains("LOOM[aspect:SecurityAspect]"),
        "expected LOOM annotation, output:\n{}",
        out
    );
}
