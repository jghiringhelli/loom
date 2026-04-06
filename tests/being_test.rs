//! M41 tests — Being block: Aristotle's four causes + telos + regulate + evolve.

use loom::ast::{BeingDef, EvolveBlock, RegulateBlock, Span, TelosDef};
use loom::checker::check_teleos;
use loom::codegen::openapi::OpenApiEmitter;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// 1. being_parses_with_telos
#[test]
fn being_parses_with_telos() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    assert_eq!(module.being_defs[0].name, "Organism");
    assert!(module.being_defs[0].telos.is_some());
}

// 2. being_without_telos_fails_checker
#[test]
fn being_without_telos_fails_checker() {
    let src = r#"module Test
being Organism
end
end
"#;
    let module = parse(src);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for being without telos");
}

// 3. regulate_without_bounds_fails_checker
#[test]
fn regulate_without_bounds_fails_checker() {
    use loom::ast::*;
    let module = loom::ast::Module {
        name: "Test".to_string(),
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
            aspect_defs: vec![],
        being_defs: vec![BeingDef {
            name: "Org".to_string(),
            describe: None,
            annotations: vec![],
            matter: None,
            form: None,
            function: None,
            telos: Some(TelosDef {
                description: "test".to_string(),
                fitness_fn: None,
                modifiable_by: None,
                bounded_by: None,
                span: Span::synthetic(),
            }),
            regulate_blocks: vec![RegulateBlock {
                variable: "temperature".to_string(),
                target: "37".to_string(),
                bounds: None,
                response: vec![],
                span: Span::synthetic(),
            }],
            evolve_block: None,
            epigenetic_blocks: vec![],
            morphogen_blocks: vec![],
            telomere: None,
            autopoietic: false,
            crispr_blocks: vec![],
            plasticity_blocks: vec![],
            canalization: None,
            senescence: None,
            criticality: None,
            span: Span::synthetic(),
        }],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for regulate without bounds");
}

// 4. evolve_without_constraint_fails_checker
#[test]
fn evolve_without_constraint_fails_checker() {
    use loom::ast::*;
    let module = loom::ast::Module {
        name: "Test".to_string(),
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
            aspect_defs: vec![],
        being_defs: vec![BeingDef {
            name: "Org".to_string(),
            describe: None,
            annotations: vec![],
            matter: None,
            form: None,
            function: None,
            telos: Some(TelosDef {
                description: "test".to_string(),
                fitness_fn: None,
                modifiable_by: None,
                bounded_by: None,
                span: Span::synthetic(),
            }),
            regulate_blocks: vec![],
            evolve_block: Some(EvolveBlock {
                search_cases: vec![],
                constraint: "".to_string(),
                span: Span::synthetic(),
            }),
            epigenetic_blocks: vec![],
            morphogen_blocks: vec![],
            telomere: None,
            autopoietic: false,
            crispr_blocks: vec![],
            plasticity_blocks: vec![],
            canalization: None,
            senescence: None,
            criticality: None,
            span: Span::synthetic(),
        }],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for evolve without constraint");
}

// 5. being_with_matter_parses
#[test]
fn being_with_matter_parses() {
    let src = r#"module Test
being Organism
  matter:
    energy: Float
    mass: Float
  end
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    let b = &module.being_defs[0];
    assert!(b.matter.is_some());
    assert!(b.telos.is_some());
}

// 6. rust_emit_being_has_struct
#[test]
fn rust_emit_being_has_struct() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub struct Organism"), "expected pub struct Organism in:\n{out}");
}

// 7. rust_emit_being_has_fitness_fn
#[test]
fn rust_emit_being_has_fitness_fn() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn fitness"), "expected pub fn fitness in:\n{out}");
}

// 8. rust_emit_being_has_regulate_fn
#[test]
fn rust_emit_being_has_regulate_fn() {
    use loom::ast::*;
    let module = loom::ast::Module {
        name: "Test".to_string(),
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
            aspect_defs: vec![],
        being_defs: vec![BeingDef {
            name: "Organism".to_string(),
            describe: None,
            annotations: vec![],
            matter: None,
            form: None,
            function: None,
            telos: Some(TelosDef {
                description: "test".to_string(),
                fitness_fn: None,
                modifiable_by: None,
                bounded_by: None,
                span: Span::synthetic(),
            }),
            regulate_blocks: vec![RegulateBlock {
                variable: "temperature".to_string(),
                target: "37".to_string(),
                bounds: Some(("35".to_string(), "39".to_string())),
                response: vec![],
                span: Span::synthetic(),
            }],
            evolve_block: None,
            epigenetic_blocks: vec![],
            morphogen_blocks: vec![],
            telomere: None,
            autopoietic: false,
            crispr_blocks: vec![],
            plasticity_blocks: vec![],
            canalization: None,
            senescence: None,
            criticality: None,
            span: Span::synthetic(),
        }],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn regulate_"), "expected pub fn regulate_ in:\n{out}");
}

// 9. typescript_emit_being_has_class
#[test]
fn typescript_emit_being_has_class() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("export class Organism"), "expected export class Organism in:\n{out}");
}

// 10. typescript_emit_being_has_fitness
#[test]
fn typescript_emit_being_has_fitness() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("fitness()"), "expected fitness() in:\n{out}");
}

// 11. typescript_emit_being_has_telos_jsdoc
#[test]
fn typescript_emit_being_has_telos_jsdoc() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("@telos"), "expected @telos in:\n{out}");
}

// 12. openapi_emit_being_has_x_telos
#[test]
fn openapi_emit_being_has_x_telos() {
    let src = r#"module Test
being Organism
  telos: "converge to full potential"
  end
end
end
"#;
    let module = parse(src);
    let out = OpenApiEmitter::new().emit(&module);
    assert!(out.contains("x-telos"), "expected x-telos in:\n{out}");
}
