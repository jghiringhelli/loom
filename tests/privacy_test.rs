//! Tests for M20: Privacy Labels

use loom::ast::*;
use loom::checker::PrivacyChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::{compile, compile_json_schema, compile_openapi, compile_typescript};

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── 1. Basic parsing of field annotations ────────────────────────────────────

#[test]
fn parses_field_annotations_pii_gdpr() {
    let m = parse_module("module M type User = email: String @pii @gdpr end end");
    if let Item::Type(td) = &m.items[0] {
        assert_eq!(td.fields.len(), 1);
        let f = &td.fields[0];
        assert_eq!(f.name, "email");
        let keys: Vec<&str> = f.annotations.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"pii"), "expected @pii in annotations");
        assert!(keys.contains(&"gdpr"), "expected @gdpr in annotations");
    } else {
        panic!("expected TypeDef");
    }
}

// ── 2. FieldDef.annotations contains @pii and @gdpr ──────────────────────────

#[test]
fn field_def_annotations_contain_pii_and_gdpr() {
    let m = parse_module("module M type User = email: String @pii @gdpr end end");
    if let Item::Type(td) = &m.items[0] {
        let f = &td.fields[0];
        assert_eq!(f.annotations.len(), 2);
        assert_eq!(f.annotations[0].key, "pii");
        assert_eq!(f.annotations[1].key, "gdpr");
    } else {
        panic!("expected TypeDef");
    }
}

// ── 3. Rust emission includes #[loom_pii] ─────────────────────────────────────

#[test]
fn rust_emission_includes_loom_pii_attribute() {
    let src = "module M type User = id: Int email: String @pii end end";
    let out = compile(src).expect("compile failed");
    assert!(
        out.contains("#[loom_pii]"),
        "expected #[loom_pii] in Rust output:\n{}",
        out
    );
}

// ── 4. TypeScript emission includes JSDoc @pii ────────────────────────────────

#[test]
fn typescript_emission_includes_jsdoc_pii() {
    let src = "module M type User = id: Int email: String @pii end end";
    let out = compile_typescript(src).expect("ts compile failed");
    assert!(
        out.contains("@pii"),
        "expected @pii in TypeScript output:\n{}",
        out
    );
}

// ── 5. JSON Schema emission includes x-pii: true ─────────────────────────────

#[test]
fn schema_emission_includes_x_pii() {
    let src = "module M type User = id: Int email: String @pii end end";
    let out = compile_json_schema(src).expect("schema compile failed");
    assert!(
        out.contains("\"x-pii\": true"),
        "expected x-pii in schema:\n{}",
        out
    );
}

// ── 6. OpenAPI includes x-data-protection when PII fields exist ──────────────

#[test]
fn openapi_includes_x_data_protection_for_pii() {
    let src = "module M type User = email: String @pii @gdpr end end";
    let out = compile_openapi(src).expect("openapi compile failed");
    assert!(
        out.contains("x-data-protection"),
        "expected x-data-protection in OpenAPI:\n{}",
        out
    );
    assert!(
        out.contains("User.email"),
        "expected User.email in pii-fields:\n{}",
        out
    );
}

// ── 7. @pci without @encrypt-at-rest fails privacy checker ───────────────────

#[test]
fn pci_without_encrypt_at_rest_fails() {
    let src = "module M type Payment = card: String @pci @never-log end end";
    let m = parse_module(src);
    let result = PrivacyChecker::new().check(&m);
    assert!(
        result.is_err(),
        "expected privacy error for @pci without @encrypt-at-rest"
    );
    let errs = result.unwrap_err();
    assert!(errs
        .iter()
        .any(|e| e.to_string().contains("encrypt-at-rest")));
}

// ── 8. @hipaa without @encrypt-at-rest fails privacy checker ─────────────────

#[test]
fn hipaa_without_encrypt_at_rest_fails() {
    let src = "module M type Patient = ssn: String @hipaa end end";
    let m = parse_module(src);
    let result = PrivacyChecker::new().check(&m);
    assert!(
        result.is_err(),
        "expected privacy error for @hipaa without @encrypt-at-rest"
    );
    let errs = result.unwrap_err();
    assert!(errs
        .iter()
        .any(|e| e.to_string().contains("encrypt-at-rest")));
}

// ── 9. @pci with @encrypt-at-rest and @never-log passes checker ──────────────

#[test]
fn pci_with_required_annotations_passes() {
    let src = "module M type Payment = card: String @pci @encrypt-at-rest @never-log end end";
    let m = parse_module(src);
    assert!(
        PrivacyChecker::new().check(&m).is_ok(),
        "expected @pci with @encrypt-at-rest and @never-log to pass"
    );
}

// ── 10. Existing corpus compiles without errors (no PII = no annotations) ─────

#[test]
fn corpus_pricing_engine_still_compiles() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").expect("corpus file missing");
    assert!(
        compile(&src).is_ok(),
        "pricing_engine.loom should still compile"
    );
}

// ── 11. Hyphenated annotations parse correctly ────────────────────────────────

#[test]
fn parses_hyphenated_annotations() {
    let m =
        parse_module("module M type Secret = token: String @encrypt-at-rest @never-log end end");
    if let Item::Type(td) = &m.items[0] {
        let f = &td.fields[0];
        let keys: Vec<&str> = f.annotations.iter().map(|a| a.key.as_str()).collect();
        assert!(
            keys.contains(&"encrypt-at-rest"),
            "expected @encrypt-at-rest"
        );
        assert!(keys.contains(&"never-log"), "expected @never-log");
    } else {
        panic!("expected TypeDef");
    }
}

// ── 12. Multiple privacy labels on multiple fields ────────────────────────────

#[test]
fn multiple_fields_with_multiple_labels() {
    let m = parse_module(
        "module M type User = \
            id: Int \
            email: String @pii @gdpr \
            ssn: String @pii @hipaa @encrypt-at-rest \
        end end",
    );
    if let Item::Type(td) = &m.items[0] {
        assert_eq!(td.fields.len(), 3);
        assert_eq!(td.fields[0].annotations.len(), 0);
        assert_eq!(td.fields[1].annotations.len(), 2);
        assert_eq!(td.fields[2].annotations.len(), 3);
    } else {
        panic!("expected TypeDef");
    }
}

// ── 13. Rust never-log comment is emitted ─────────────────────────────────────

#[test]
fn rust_never_log_comment_emitted() {
    let src = "module M type Sec = tok: String @never-log end end";
    let out = compile(src).expect("compile failed");
    assert!(
        out.contains("NEVER LOG"),
        "expected NEVER LOG comment in Rust output:\n{}",
        out
    );
}

// ── 14. @pci without @never-log fails (missing never-log) ────────────────────

#[test]
fn pci_without_never_log_fails() {
    let src = "module M type Payment = card: String @pci @encrypt-at-rest end end";
    let m = parse_module(src);
    let result = PrivacyChecker::new().check(&m);
    assert!(
        result.is_err(),
        "expected error for @pci without @never-log"
    );
    let errs = result.unwrap_err();
    assert!(errs.iter().any(|e| e.to_string().contains("never-log")));
}
