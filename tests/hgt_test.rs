// tests/hgt_test.rs — M75: Horizontal Gene Transfer (adopt)

use loom::checker::HgtChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. adopt declaration parses
#[test]
fn adopt_decl_parses() {
    let src = r#"module M
adopt: Flyable from BirdModule
end"#;
    let m = parse(src);
    let decl = match &m.items[0] {
        loom::ast::Item::Adopt(d) => d,
        _ => panic!("expected Adopt"),
    };
    assert_eq!(decl.interface, "Flyable");
    assert_eq!(decl.from_module, "BirdModule");
}

// 2. multiple adopt declarations parse
#[test]
fn multiple_adopt_decls_parse() {
    let src = r#"module M
adopt: Swimmable from FishModule
adopt: Runnable from MammalModule
end"#;
    let m = parse(src);
    assert_eq!(m.items.len(), 2);
}

// 3. checker rejects empty interface
#[test]
fn checker_rejects_empty_interface() {
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
        items: vec![Item::Adopt(AdoptDecl {
            interface: "".to_string(),
            from_module: "SomeModule".to_string(),
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = HgtChecker::new().check(&module);
    assert!(result.is_err());
}

// 4. checker rejects empty from_module
#[test]
fn checker_rejects_empty_from_module() {
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
        items: vec![Item::Adopt(AdoptDecl {
            interface: "Flyable".to_string(),
            from_module: "".to_string(),
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    let result = HgtChecker::new().check(&module);
    assert!(result.is_err());
}

// 5. checker passes valid adopt
#[test]
fn checker_passes_valid_adopt() {
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
        items: vec![Item::Adopt(AdoptDecl {
            interface: "Flyable".to_string(),
            from_module: "BirdModule".to_string(),
            span: Span::synthetic(),
        })],
        span: Span::synthetic(),
    };
    assert!(HgtChecker::new().check(&module).is_ok());
}

// 6. codegen emits adopt as use statement
#[test]
fn codegen_emits_adopt_use() {
    let src = r#"module M
adopt: Flyable from BirdModule
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("adopt"), "expected adopt in:\n{out}");
    assert!(out.contains("Flyable"), "expected Flyable in:\n{out}");
    assert!(out.contains("BirdModule"), "expected BirdModule in:\n{out}");
}
