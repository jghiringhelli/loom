//! M22 tests — Typestate / Lifecycle Protocols.
//!
//! Verifies:
//! 1. `lifecycle Connection :: Disconnected -> Connected -> Authenticated` parses
//! 2. `LifecycleDef` has correct type_name and states
//! 3. Valid transition `Connection<Disconnected> -> Connection<Connected>` passes checker
//! 4. Invalid transition (skip state) → error from checker
//! 5. Rust emission includes phantom state structs
//! 6. TypeScript emission includes state union type
//! 7. OpenAPI includes `x-lifecycle` extension
//! 8. Module without lifecycle compiles unchanged

use loom::ast::{LifecycleDef, Span};
use loom::checker::TypestateChecker;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;
use loom::codegen::openapi::OpenApiEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── 1. Lifecycle declaration parses ───────────────────────────────────────────

#[test]
fn lifecycle_declaration_parses() {
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated
fn connect :: String -> String
  host
end
end"#;
    let module = parse(src);
    assert_eq!(module.lifecycle_defs.len(), 1);
}

// ── 2. LifecycleDef has correct type_name and states ─────────────────────────

#[test]
fn lifecycle_def_has_correct_fields() {
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated -> Closed
fn connect :: String -> String
  host
end
end"#;
    let module = parse(src);
    let lc = &module.lifecycle_defs[0];
    assert_eq!(lc.type_name, "Connection");
    assert_eq!(lc.states, vec!["Disconnected", "Connected", "Authenticated", "Closed"]);
}

// ── 3. Valid transition passes checker ────────────────────────────────────────

#[test]
fn valid_transition_passes_checker() {
    // Connection<Disconnected> -> Connection<Connected> is the declared first step
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated
fn connect :: Connection<Disconnected> -> Connection<Connected>
  conn
end
end"#;
    let module = parse(src);
    let result = TypestateChecker::new().check(&module);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
}

// ── 4. Invalid transition (skip state) → error ────────────────────────────────

#[test]
fn invalid_transition_skipping_state_fails() {
    // Skipping Connected and going directly Disconnected -> Authenticated is invalid
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated
fn bad_connect :: Connection<Disconnected> -> Connection<Authenticated>
  conn
end
end"#;
    let module = parse(src);
    let result = TypestateChecker::new().check(&module);
    assert!(result.is_err(), "expected Err for invalid lifecycle transition");
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    let msg = format!("{}", errors[0]);
    assert!(
        msg.contains("invalid lifecycle transition"),
        "error message should describe invalid transition, got: {msg}"
    );
}

// ── 5. Rust emission includes phantom state structs ───────────────────────────

#[test]
fn rust_emits_phantom_state_structs() {
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated -> Closed
fn connect :: String -> String
  host
end
end"#;
    let module = parse(src);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub struct Disconnected;"), "missing Disconnected struct in:\n{out}");
    assert!(out.contains("pub struct Connected;"), "missing Connected struct in:\n{out}");
    assert!(out.contains("pub struct Authenticated;"), "missing Authenticated struct in:\n{out}");
    assert!(out.contains("pub struct Closed;"), "missing Closed struct in:\n{out}");
    assert!(out.contains("// Lifecycle states for Connection"), "missing comment in:\n{out}");
}

// ── 6. TypeScript emission includes state union type ─────────────────────────

#[test]
fn typescript_emits_state_union_type() {
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated
fn connect :: String -> String
  host
end
end"#;
    let module = parse(src);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(
        out.contains("ConnectionState"),
        "missing ConnectionState union type in:\n{out}"
    );
    assert!(
        out.contains("\"Disconnected\""),
        "missing Disconnected variant in:\n{out}"
    );
    assert!(
        out.contains("\"Connected\""),
        "missing Connected variant in:\n{out}"
    );
    assert!(
        out.contains("export interface Connection<State extends ConnectionState>"),
        "missing Connection interface in:\n{out}"
    );
}

// ── 7. OpenAPI includes x-lifecycle extension ─────────────────────────────────

#[test]
fn openapi_includes_x_lifecycle_extension() {
    let src = r#"module ConnectionService
lifecycle Connection :: Disconnected -> Connected -> Authenticated
fn connect :: String -> String
  host
end
end"#;
    let module = parse(src);
    let out = OpenApiEmitter::new().emit(&module);
    assert!(
        out.contains("x-lifecycle"),
        "expected x-lifecycle in OpenAPI output:\n{out}"
    );
    assert!(
        out.contains("\"Connection\""),
        "expected Connection key in x-lifecycle:\n{out}"
    );
    assert!(
        out.contains("\"states\""),
        "expected states in x-lifecycle:\n{out}"
    );
    assert!(
        out.contains("\"transitions\""),
        "expected transitions in x-lifecycle:\n{out}"
    );
}

// ── 8. Module without lifecycle compiles unchanged ────────────────────────────

#[test]
fn module_without_lifecycle_compiles_unchanged() {
    let src = r#"module Simple
fn add :: Int -> Int -> Int
  x
end
end"#;
    let module = parse(src);
    assert!(
        module.lifecycle_defs.is_empty(),
        "expected no lifecycle_defs for plain module"
    );
    // Checker passes with no lifecycle defs
    assert!(TypestateChecker::new().check(&module).is_ok());
    // Rust output has no lifecycle comment
    let out = RustEmitter::new().emit(&module);
    assert!(
        !out.contains("Lifecycle states"),
        "unexpected lifecycle output in plain module:\n{out}"
    );
}
