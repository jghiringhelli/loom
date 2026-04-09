//! DI integration tests for M5 — `requires` / `with` dependency injection.

fn compile_ok(src: &str) -> String {
    loom::compile(src).expect("expected compilation to succeed")
}

fn compile_err(src: &str) -> Vec<loom::LoomError> {
    loom::compile(src).expect_err("expected compilation to fail")
}

// ── Context struct emission ───────────────────────────────────────────────────

#[test]
fn module_with_requires_emits_context_struct() {
    let src = r#"
module M
requires { db: DbConn }
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("MContext"), "expected MContext struct in:\n{}", out);
    assert!(out.contains("db: DbConn"), "expected db field in:\n{}", out);
}

#[test]
fn module_with_multiple_requires_emits_all_fields() {
    let src = r#"
module Svc
requires { db: DbConn, log: Logger }
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("SvcContext"), "expected SvcContext in:\n{}", out);
    assert!(out.contains("db: DbConn"), "expected db field");
    assert!(out.contains("log: Logger"), "expected log field");
}

#[test]
fn module_without_requires_emits_no_context_struct() {
    let src = r#"
module Pure
fn add :: Int -> Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(!out.contains("Context"), "unexpected Context struct in:\n{}", out);
}

// ── ctx parameter injection ───────────────────────────────────────────────────

#[test]
fn function_with_dep_gets_ctx_parameter() {
    let src = r#"
module M
requires { db: DbConn }

fn find :: Int -> String
with db
  "found"
end
end
"#;
    let out = compile_ok(src);
    // The function should receive ctx as its first parameter
    assert!(out.contains("ctx: &MContext"), "expected ctx param in:\n{}", out);
}

#[test]
fn function_without_with_gets_no_ctx_parameter() {
    let src = r#"
module M
requires { db: DbConn }

fn pure_fn :: Int -> Int
  42
end
end
"#;
    let out = compile_ok(src);
    // Pure function should NOT get ctx
    let fn_line = out.lines().find(|l| l.contains("pub fn pure_fn")).unwrap_or("");
    assert!(!fn_line.contains("ctx"), "unexpected ctx in pure_fn: {}", fn_line);
}

// ── Validation: undeclared dependency ────────────────────────────────────────

#[test]
fn function_referencing_undeclared_dep_errors() {
    let src = r#"
module M
fn find :: Int -> String
with db
  "found"
end
end
"#;
    let errors = compile_err(src);
    let has_undeclared = errors.iter().any(|e| {
        matches!(e, loom::LoomError::UndeclaredDependency { name, .. } if name == "db")
    });
    assert!(has_undeclared, "expected UndeclaredDependency error, got: {:?}", errors);
}

#[test]
fn function_referencing_dep_not_in_requires_errors() {
    let src = r#"
module M
requires { db: DbConn }

fn fetch :: Int -> String
with cache
  "found"
end
end
"#;
    let errors = compile_err(src);
    let has_undeclared = errors.iter().any(|e| {
        matches!(e, loom::LoomError::UndeclaredDependency { name, .. } if name == "cache")
    });
    assert!(has_undeclared, "expected UndeclaredDependency for 'cache', got: {:?}", errors);
}

// ── Corpus regression ─────────────────────────────────────────────────────────

#[test]
fn pricing_engine_corpus_still_compiles() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").unwrap();
    compile_ok(&src);
}

#[test]
fn user_service_corpus_still_compiles() {
    let src = std::fs::read_to_string("corpus/user_service.loom").unwrap();
    compile_ok(&src);
}
