/// Tests for M23: Information Flow Labels.
///
/// Covers parsing, checker, and codegen output for flow label declarations.
use loom::ast::{FlowLabel, Module, Span};
use loom::checker::InfoFlowChecker;
use loom::codegen::{OpenApiEmitter, TypeScriptEmitter};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse(src: &str) -> Module {
    let tokens = loom::lexer::Lexer::tokenize(src).expect("lex error");
    loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("parse error")
}

// ── 1. Parsing ────────────────────────────────────────────────────────────────

#[test]
fn flow_label_parses_correctly() {
    let src = r#"
module Auth
flow secret :: Password, Token
end
"#;
    let module = parse(src);
    assert_eq!(module.flow_labels.len(), 1);
}

#[test]
fn flow_label_has_correct_fields() {
    let src = r#"
module Auth
flow secret :: Password, Token
end
"#;
    let module = parse(src);
    let fl = &module.flow_labels[0];
    assert_eq!(fl.label, "secret");
    assert_eq!(fl.types, vec!["Password", "Token"]);
}

#[test]
fn multiple_flow_labels_parse() {
    let src = r#"
module Auth
flow secret :: Password, Token, SessionKey
flow tainted :: UserInput, QueryParam
flow public :: UserId, Email, Bool, Int
end
"#;
    let module = parse(src);
    assert_eq!(module.flow_labels.len(), 3);
    assert_eq!(module.flow_labels[0].label, "secret");
    assert_eq!(module.flow_labels[1].label, "tainted");
    assert_eq!(module.flow_labels[2].label, "public");
}

// ── 2. Checker — valid cases ──────────────────────────────────────────────────

#[test]
fn checker_allows_secret_to_secret_flow() {
    // fn wrap :: Password -> Password
    // Both param and return are @secret — no violation.
    let src = r#"
module Auth
flow secret :: Password

fn wrap :: Password -> Password
  password
end
end
"#;
    let module = parse(src);
    let result = InfoFlowChecker::new().check(&module);
    assert!(result.is_ok(), "expected no errors but got: {:?}", result);
}

#[test]
fn checker_allows_declassification_fn() {
    // fn hash_password :: Password -> String
    // Name contains "hash" → intentional declassification, no error.
    let src = r#"
module Auth
flow secret :: Password
flow public :: String

fn hash_password :: Password -> String
  password
end
end
"#;
    let module = parse(src);
    let result = InfoFlowChecker::new().check(&module);
    assert!(
        result.is_ok(),
        "hash_password should be allowed: {:?}",
        result
    );
}

#[test]
fn checker_allows_module_without_flow_labels() {
    let src = r#"
module Simple
fn add :: Int -> Int -> Int
  x
end
end
"#;
    let module = parse(src);
    assert!(module.flow_labels.is_empty());
    let result = InfoFlowChecker::new().check(&module);
    assert!(result.is_ok());
}

// ── 3. Checker — violations ───────────────────────────────────────────────────

#[test]
fn checker_rejects_secret_to_public_without_declassification() {
    // fn get_token :: Password -> String
    // Password is @secret, String is @public (no label = public), fn name doesn't suggest declassification.
    let src = r#"
module Auth
flow secret :: Password

fn get_token :: Password -> String
  password
end
end
"#;
    let module = parse(src);
    let result = InfoFlowChecker::new().check(&module);
    assert!(result.is_err(), "expected info-flow error");
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors[0].to_string().contains("information flow violation"));
}

#[test]
fn checker_rejects_tainted_to_db_operation() {
    // fn find_user :: UserInput -> String
    // UserInput is @tainted and "find" suggests DB access.
    let src = r#"
module Users
flow tainted :: UserInput

fn find_user :: UserInput -> String
  input
end
end
"#;
    let module = parse(src);
    let result = InfoFlowChecker::new().check(&module);
    assert!(result.is_err(), "expected tainted-to-db error");
    let errors = result.unwrap_err();
    assert!(errors[0].to_string().contains("information flow violation"));
    assert!(errors[0].to_string().contains("tainted"));
}

// ── 4. TypeScript codegen ─────────────────────────────────────────────────────

#[test]
fn typescript_emits_branded_types_for_flow_labels() {
    let src = r#"
module Auth
flow secret :: Password, Token
flow tainted :: UserInput
end
"#;
    let module = parse(src);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(
        out.contains("_sensitivity"),
        "should contain _sensitivity brand"
    );
    assert!(out.contains("Password"), "should contain Password type");
    assert!(out.contains("Token"), "should contain Token type");
    assert!(out.contains("UserInput"), "should contain UserInput type");
    assert!(
        out.contains("\"secret\""),
        "should contain secret sensitivity"
    );
    assert!(
        out.contains("\"tainted\""),
        "should contain tainted sensitivity"
    );
}

// ── 5. OpenAPI codegen ────────────────────────────────────────────────────────

#[test]
fn openapi_emits_x_security_labels() {
    let src = r#"
module Auth
flow secret :: Password, Token, SessionKey
flow tainted :: UserInput, QueryParam
flow public :: UserId, Email
end
"#;
    let module = parse(src);
    let out = OpenApiEmitter::new().emit(&module);
    assert!(
        out.contains("x-security-labels"),
        "should contain x-security-labels"
    );
    assert!(out.contains("\"secret\""), "should list secret label");
    assert!(out.contains("Password"), "should list Password type");
    assert!(out.contains("\"tainted\""), "should list tainted label");
}

// ── 6. Module without flow_labels ────────────────────────────────────────────

#[test]
fn module_without_flow_labels_has_empty_vec() {
    let src = r#"
module Simple
fn add :: Int -> Int -> Int
  x
end
end
"#;
    let module = parse(src);
    assert!(
        module.flow_labels.is_empty(),
        "module without flow declarations should have empty flow_labels"
    );
}
