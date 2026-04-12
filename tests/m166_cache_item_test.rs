/// M166 — `cache` item: parser + codegen tests.
///
/// `cache Name key: Type value: Type ttl: N end`
/// generates a typed `{Name}Cache<K,V>` generic struct with get/set/evict methods.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M166.1: cache parses without error ───────────────────────────────────────

#[test]
fn m166_cache_parses() {
    let src = r#"
module infra
cache SessionCache
  key: String
  value: String
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("SessionCacheCache"),
        "expected SessionCacheCache\n{out}"
    );
}

// ─── M166.2: struct has ttl_secs field ────────────────────────────────────────

#[test]
fn m166_struct_ttl_field() {
    let src = r#"
module infra
cache UserCache
  key: String
  value: String
  ttl: 600
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub ttl_secs: u64"),
        "missing ttl_secs field\n{out}"
    );
}

// ─── M166.3: new() uses configured ttl ────────────────────────────────────────

#[test]
fn m166_new_uses_configured_ttl() {
    let src = r#"
module infra
cache TokenCache
  key: String
  value: String
  ttl: 900
end
end
"#;
    let out = compile(src);
    assert!(out.contains("ttl_secs: 900"), "ttl not 900\n{out}");
}

// ─── M166.4: default ttl when omitted ─────────────────────────────────────────

#[test]
fn m166_default_ttl() {
    let src = r#"
module infra
cache SimpleCache
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("ttl_secs: 300"),
        "default ttl should be 300\n{out}"
    );
}

// ─── M166.5: get() method emitted ─────────────────────────────────────────────

#[test]
fn m166_get_method_emitted() {
    let src = r#"
module infra
cache UserCache
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn get(&self, _key: &K) -> Option<V>"),
        "missing get()\n{out}"
    );
}

// ─── M166.6: set() method emitted ─────────────────────────────────────────────

#[test]
fn m166_set_method_emitted() {
    let src = r#"
module infra
cache UserCache
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn set(&mut self, _key: K, _value: V)"),
        "missing set()\n{out}"
    );
}

// ─── M166.7: evict() method emitted ───────────────────────────────────────────

#[test]
fn m166_evict_method_emitted() {
    let src = r#"
module infra
cache UserCache
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn evict(&mut self)"),
        "missing evict()\n{out}"
    );
}

// ─── M166.8: audit comment emitted ────────────────────────────────────────────

#[test]
fn m166_audit_comment_emitted() {
    let src = r#"
module infra
cache SimpleCache
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[cache:performance]"),
        "missing audit comment\n{out}"
    );
    assert!(out.contains("M166"), "missing M166 reference\n{out}");
}

// ─── M166.9: struct has #[derive(Debug, Clone)] ───────────────────────────────

#[test]
fn m166_struct_derive_attrs() {
    let src = r#"
module infra
cache UserCache
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[derive(Debug, Clone)]"),
        "missing derive\n{out}"
    );
}

// ─── M166.10: multiple caches in one module ───────────────────────────────────

#[test]
fn m166_multiple_caches() {
    let src = r#"
module caching
cache UserCache
  ttl: 300
end
cache ProductCache
  ttl: 600
end
end
"#;
    let out = compile(src);
    assert!(out.contains("UserCacheCache"), "missing UserCache\n{out}");
    assert!(
        out.contains("ProductCacheCache"),
        "missing ProductCache\n{out}"
    );
}

// ─── M166.11: mixed with rate_limiter ─────────────────────────────────────────

#[test]
fn m166_mixed_with_rate_limiter() {
    let src = r#"
module infra
cache SessionCache
  ttl: 300
end
rate_limiter ApiLimiter
  requests: 100
  per: 60
end
end
"#;
    let out = compile(src);
    assert!(out.contains("SessionCacheCache"), "missing cache\n{out}");
    assert!(
        out.contains("ApiLimiterRateLimiter"),
        "missing rate limiter\n{out}"
    );
}

// ─── M166.12: generic struct with PhantomData ─────────────────────────────────

#[test]
fn m166_generic_struct_with_phantom() {
    let src = r#"
module infra
cache UserCache
  ttl: 300
end
end
"#;
    let out = compile(src);
    assert!(out.contains("PhantomData"), "missing PhantomData\n{out}");
    assert!(
        out.contains("K: std::hash::Hash + Eq + Clone"),
        "missing K bounds\n{out}"
    );
    assert!(out.contains("V: Clone"), "missing V bound\n{out}");
}
