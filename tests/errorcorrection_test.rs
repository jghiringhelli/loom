// tests/errorcorrection_test.rs — M73: Error correction on refined types

use loom::checker::ErrorCorrectionChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. refined type with on_violation parses
#[test]
fn refined_type_with_on_violation_parses() {
    let src = r#"module M
type PositiveInt = Int where n > 0
  on_violation: clamp_to_one
end
end"#;
    let m = parse(src);
    let rt = match &m.items[0] {
        loom::ast::Item::RefinedType(rt) => rt,
        _ => panic!("expected RefinedType"),
    };
    assert_eq!(rt.on_violation, Some("clamp_to_one".to_string()));
}

// 2. refined type with repair_fn parses
#[test]
fn refined_type_with_repair_fn_parses() {
    let src = r#"module M
type BoundedFloat = Float where x >= 0.0
  repair_fn: normalize_float
end
end"#;
    let m = parse(src);
    let rt = match &m.items[0] {
        loom::ast::Item::RefinedType(rt) => rt,
        _ => panic!("expected RefinedType"),
    };
    assert_eq!(rt.repair_fn, Some("normalize_float".to_string()));
}

// 3. refined type without error correction has None fields
#[test]
fn refined_type_without_error_correction() {
    let src = r#"module M
type PosInt = Int where n > 0
end"#;
    let m = parse(src);
    let rt = match &m.items[0] {
        loom::ast::Item::RefinedType(rt) => rt,
        _ => panic!("expected RefinedType"),
    };
    assert!(rt.on_violation.is_none());
    assert!(rt.repair_fn.is_none());
}

// 4. checker passes valid on_violation
#[test]
fn checker_passes_valid_on_violation() {
    use loom::ast::*;
    let module = Module {
        name: "M".to_string(),
        describe: None,
        domains: vec![],
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        being_defs: vec![],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        aspect_defs: vec![],
        items: vec![Item::RefinedType(RefinedType {
            name: "Pos".to_string(),
            base_type: TypeExpr::Base("Int".to_string()),
            predicate: Expr::Ident("n".to_string()),
            on_violation: Some("handle".to_string()),
            repair_fn: None,
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    assert!(ErrorCorrectionChecker::new().check(&module).is_ok());
}

// 5. codegen emits on_violation comment
#[test]
fn codegen_emits_on_violation_comment() {
    let src = r#"module M
type PositiveInt = Int where n > 0
  on_violation: clamp_to_one
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(
        out.contains("on_violation"),
        "expected on_violation in:\n{out}"
    );
    assert!(
        out.contains("clamp_to_one"),
        "expected clamp_to_one in:\n{out}"
    );
}

// 6. codegen emits repair_fn comment
#[test]
fn codegen_emits_repair_fn_comment() {
    let src = r#"module M
type ValidEmail = String where is_valid_email
  repair_fn: normalize_email
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("repair_fn"), "expected repair_fn in:\n{out}");
    assert!(
        out.contains("normalize_email"),
        "expected normalize_email in:\n{out}"
    );
}
