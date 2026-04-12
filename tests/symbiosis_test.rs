// tests/symbiosis_test.rs — M72: Symbiosis

use loom::checker::SymbiosisChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. symbiotic mutualistic import parses
#[test]
fn symbiotic_mutualistic_parses() {
    let src = r#"module M
symbiotic:
  kind: mutualistic
  module: Gut
end"#;
    let m = parse(src);
    let item = &m.items[0];
    match item {
        loom::ast::Item::SymbioticImport { module, kind, .. } => {
            assert_eq!(module, "Gut");
            assert_eq!(kind, "mutualistic");
        }
        _ => panic!("expected SymbioticImport, got {:?}", item),
    }
}

// 2. symbiotic commensal parses
#[test]
fn symbiotic_commensal_parses() {
    let src = r#"module M
symbiotic:
  kind: commensal
  module: Skin
end"#;
    let m = parse(src);
    match &m.items[0] {
        loom::ast::Item::SymbioticImport { kind, .. } => assert_eq!(kind, "commensal"),
        _ => panic!("expected SymbioticImport"),
    }
}

// 3. symbiotic parasitic parses
#[test]
fn symbiotic_parasitic_parses() {
    let src = r#"module M
symbiotic:
  kind: parasitic
  module: Pathogen
end"#;
    let m = parse(src);
    match &m.items[0] {
        loom::ast::Item::SymbioticImport { kind, .. } => assert_eq!(kind, "parasitic"),
        _ => panic!("expected SymbioticImport"),
    }
}

// 4. checker rejects invalid kind
#[test]
fn checker_rejects_invalid_kind() {
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
        items: vec![Item::SymbioticImport {
            module: "Other".to_string(),
            kind: "hostile".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let result = SymbiosisChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result
        .unwrap_err()
        .iter()
        .map(|e| format!("{e}"))
        .collect::<String>();
    assert!(
        msgs.contains("not valid"),
        "expected 'not valid' in: {msgs}"
    );
}

// 5. checker rejects empty module name
#[test]
fn checker_rejects_empty_module_name() {
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
        items: vec![Item::SymbioticImport {
            module: "".to_string(),
            kind: "mutualistic".to_string(),
            span: Span::synthetic(),
        }],
        span: Span::synthetic(),
    };
    let result = SymbiosisChecker::new().check(&module);
    assert!(result.is_err());
}

// 6. codegen emits symbiotic comment
#[test]
fn codegen_emits_symbiotic_comment() {
    let src = r#"module M
symbiotic:
  kind: mutualistic
  module: Gut
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("symbiotic"), "expected symbiotic in:\n{out}");
    assert!(
        out.contains("mutualistic"),
        "expected mutualistic in:\n{out}"
    );
}
