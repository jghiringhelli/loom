// tests/degeneracy_test.rs — M68: Degeneracy (Edelman)

use loom::checker::DegeneracyChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex");
    Parser::new(&tokens).parse_module().expect("parse")
}

// 1. fn with valid degenerate block parses
#[test]
fn fn_with_degenerate_block_parses() {
    let src = r#"module M
fn compute :: Int -> Int
degenerate:
  primary: fast_path
  fallback: slow_path
end
end
end"#;
    let m = parse(src);
    let fd = match &m.items[0] {
        loom::ast::Item::Fn(f) => f,
        _ => panic!("expected Fn"),
    };
    let dg = fd.degenerate.as_ref().expect("degenerate");
    assert_eq!(dg.primary, "fast_path");
    assert_eq!(dg.fallback, "slow_path");
}

// 2. fn with degenerate + equivalence_proof parses
#[test]
fn fn_with_degenerate_equivalence_proof_parses() {
    let src = r#"module M
fn solve :: Int -> Int
degenerate:
  primary: algebraic_solver
  fallback: numeric_solver
  equivalence_proof: output_identical
end
end
end"#;
    let m = parse(src);
    let fd = match &m.items[0] {
        loom::ast::Item::Fn(f) => f,
        _ => panic!("expected Fn"),
    };
    let dg = fd.degenerate.as_ref().expect("degenerate");
    assert_eq!(dg.equivalence_proof, Some("output_identical".to_string()));
}

// 3. fn without degenerate block has None
#[test]
fn fn_without_degenerate_block_is_none() {
    let src = r#"module M
fn simple :: Int -> Int
end
end"#;
    let m = parse(src);
    let fd = match &m.items[0] {
        loom::ast::Item::Fn(f) => f,
        _ => panic!("expected Fn"),
    };
    assert!(fd.degenerate.is_none());
}

// 4. checker rejects identical primary and fallback
#[test]
fn checker_rejects_identical_primary_fallback() {
    use loom::ast::*;
    let fd = FnDef {
        name: "f".to_string(),
        describe: None,
        annotations: vec![],
        type_params: vec![],
        type_sig: FnTypeSignature {
            params: vec![],
            return_type: Box::new(TypeExpr::Base("Int".to_string())),
        },
        effect_tiers: vec![],
        requires: vec![],
        ensures: vec![],
        with_deps: vec![],
        separation: None,
        gradual: None,
        distribution: None,
        timing_safety: None,
        termination: None,
        proofs: vec![],
        degenerate: Some(DegenerateBlock {
            primary: "same".to_string(),
            fallback: "same".to_string(),
            equivalence_proof: None,
            span: Span::synthetic(),
        }),
        stochastic_process: None,
        handle_block: None,
        body: vec![],
        span: Span::synthetic(),
    };
    let module = Module {
        name: "M".to_string(),
        describe: None,
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
        items: vec![Item::Fn(fd)],
        span: Span::synthetic(),
    };
    let result = DegeneracyChecker::new().check(&module);
    assert!(result.is_err());
    let msgs = result.unwrap_err().iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msgs.contains("identical"), "expected 'identical' in: {msgs}");
}

// 5. checker rejects empty primary
#[test]
fn checker_rejects_empty_primary() {
    use loom::ast::*;
    let fd = FnDef {
        name: "f".to_string(),
        describe: None,
        annotations: vec![],
        type_params: vec![],
        type_sig: FnTypeSignature {
            params: vec![],
            return_type: Box::new(TypeExpr::Base("Int".to_string())),
        },
        effect_tiers: vec![],
        requires: vec![],
        ensures: vec![],
        with_deps: vec![],
        separation: None,
        gradual: None,
        distribution: None,
        timing_safety: None,
        termination: None,
        proofs: vec![],
        degenerate: Some(DegenerateBlock {
            primary: "".to_string(),
            fallback: "backup".to_string(),
            equivalence_proof: None,
            span: Span::synthetic(),
        }),
        stochastic_process: None,
        handle_block: None,
        body: vec![],
        span: Span::synthetic(),
    };
    let module = Module {
        name: "M".to_string(),
        describe: None,
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
        items: vec![Item::Fn(fd)],
        span: Span::synthetic(),
    };
    let result = DegeneracyChecker::new().check(&module);
    assert!(result.is_err());
}

// 6. checker passes valid degenerate block
#[test]
fn checker_passes_valid_degenerate() {
    use loom::ast::*;
    let fd = FnDef {
        name: "f".to_string(),
        describe: None,
        annotations: vec![],
        type_params: vec![],
        type_sig: FnTypeSignature {
            params: vec![],
            return_type: Box::new(TypeExpr::Base("Int".to_string())),
        },
        effect_tiers: vec![],
        requires: vec![],
        ensures: vec![],
        with_deps: vec![],
        separation: None,
        gradual: None,
        distribution: None,
        timing_safety: None,
        termination: None,
        proofs: vec![],
        degenerate: Some(DegenerateBlock {
            primary: "path_a".to_string(),
            fallback: "path_b".to_string(),
            equivalence_proof: None,
            span: Span::synthetic(),
        }),
        stochastic_process: None,
        handle_block: None,
        body: vec![],
        span: Span::synthetic(),
    };
    let module = Module {
        name: "M".to_string(),
        describe: None,
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
        items: vec![Item::Fn(fd)],
        span: Span::synthetic(),
    };
    assert!(DegeneracyChecker::new().check(&module).is_ok());
}

// 7. codegen emits degenerate comments
#[test]
fn codegen_emits_degenerate_comments() {
    let src = r#"module M
fn compute :: Int -> Int
degenerate:
  primary: fast_path
  fallback: slow_path
end
end
end"#;
    let out = loom::compile(src).expect("compile");
    assert!(out.contains("degenerate"), "expected degenerate comment in:\n{out}");
    assert!(out.contains("fast_path"), "expected fast_path in:\n{out}");
    assert!(out.contains("slow_path"), "expected slow_path in:\n{out}");
}
