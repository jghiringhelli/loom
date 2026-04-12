/// M171 — `pool` item: parser + codegen tests.
///
/// `pool Name size: N end`
/// generates a `{Name}Pool<T>` struct with acquire/release.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

#[test]
fn m171_pool_parses() {
    let out = compile(
        r#"
module infra
pool ConnectionPool
  size: 20
end
end
"#,
    );
    assert!(out.contains("ConnectionPoolPool"), "expected struct\n{out}");
}

#[test]
fn m171_struct_fields_emitted() {
    let out = compile(
        r#"
module infra
pool WorkerPool
  size: 5
end
end
"#,
    );
    assert!(
        out.contains("pub capacity: usize"),
        "missing capacity\n{out}"
    );
}

#[test]
fn m171_new_uses_configured_size() {
    let out = compile(
        r#"
module infra
pool DbPool
  size: 25
end
end
"#,
    );
    assert!(out.contains("capacity: 25"), "capacity not 25\n{out}");
}

#[test]
fn m171_default_size_when_omitted() {
    let out = compile(
        r#"
module infra
pool SimplePool
end
end
"#,
    );
    assert!(
        out.contains("capacity: 10"),
        "default capacity should be 10\n{out}"
    );
}

#[test]
fn m171_acquire_method_emitted() {
    let out = compile(
        r#"
module infra
pool DbPool
  size: 10
end
end
"#,
    );
    assert!(
        out.contains("pub fn acquire(&mut self) -> Option<T>"),
        "missing acquire()\n{out}"
    );
}

#[test]
fn m171_release_method_emitted() {
    let out = compile(
        r#"
module infra
pool DbPool
  size: 10
end
end
"#,
    );
    assert!(
        out.contains("pub fn release(&mut self, _item: T)"),
        "missing release()\n{out}"
    );
}

#[test]
fn m171_audit_comment_emitted() {
    let out = compile(
        r#"
module infra
pool SimplePool
end
end
"#,
    );
    assert!(
        out.contains("LOOM[pool:performance]"),
        "missing audit comment\n{out}"
    );
    assert!(out.contains("M171"), "missing M171 reference\n{out}");
}

#[test]
fn m171_struct_derive_attrs() {
    let out = compile(
        r#"
module infra
pool SimplePool
end
end
"#,
    );
    assert!(
        out.contains("#[derive(Debug, Clone)]"),
        "missing derive\n{out}"
    );
}

#[test]
fn m171_phantom_data_emitted() {
    let out = compile(
        r#"
module infra
pool SimplePool
  size: 5
end
end
"#,
    );
    assert!(out.contains("PhantomData"), "missing PhantomData\n{out}");
}

#[test]
fn m171_multiple_pools() {
    let out = compile(
        r#"
module pools
pool ConnectionPool
  size: 20
end
pool WorkerPool
  size: 4
end
end
"#,
    );
    assert!(
        out.contains("ConnectionPoolPool"),
        "missing connection\n{out}"
    );
    assert!(out.contains("WorkerPoolPool"), "missing worker\n{out}");
}

#[test]
fn m171_acquire_has_todo_stub() {
    let out = compile(
        r#"
module infra
pool SimplePool
end
end
"#,
    );
    assert!(
        out.contains("todo!(\"implement pool acquire\")"),
        "missing acquire todo\n{out}"
    );
}

#[test]
fn m171_mixed_with_cache() {
    let out = compile(
        r#"
module infra
pool ConnectionPool
  size: 10
end
cache UserCache
  ttl: 300
end
end
"#,
    );
    assert!(out.contains("ConnectionPoolPool"), "missing pool\n{out}");
    assert!(out.contains("UserCacheCache"), "missing cache\n{out}");
}
