//! M42 tests — evolve: block convergence checker + strategy-specific codegen.

use loom::ast::{BeingDef, EvolveBlock, SearchCase, SearchStrategy, Span, TelosDef};
use loom::checker::check_teleos;
use loom::codegen::rust::RustEmitter;
use loom::codegen::typescript::TypeScriptEmitter;

fn make_module(being: BeingDef) -> loom::ast::Module {
    loom::ast::Module {
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
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    }
}

fn make_being_with_evolve(evolve: EvolveBlock) -> BeingDef {
    BeingDef {
        name: "Organism".to_string(),
        describe: None,
        matter: None,
        form: None,
        function: None,
        telos: Some(TelosDef {
            description: "converge to full potential".to_string(),
            fitness_fn: None,
            span: Span::synthetic(),
        }),
        regulate_blocks: vec![],
        evolve_block: Some(evolve),
        span: Span::synthetic(),
    }
}

// 1. evolve block with no search cases → checker error
#[test]
fn evolve_empty_search_fails() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for evolve with no search cases");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("no search: cases"), "expected 'no search: cases' in: {msg}");
}

// 2. constraint with no convergence keyword → checker error
#[test]
fn evolve_constraint_no_convergence_word_fails() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: "E[d] going somewhere".to_string(),
        span: Span::synthetic(),
    }));
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for non-convergence constraint");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("convergence"), "expected 'convergence' in: {msg}");
}

// 3. gradient_descent and derivative_free both without when → checker error
#[test]
fn evolve_gradient_and_derivative_free_no_when_fails() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![
            SearchCase { strategy: SearchStrategy::GradientDescent, when: "".to_string() },
            SearchCase { strategy: SearchStrategy::DerivativeFree,  when: "".to_string() },
        ],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for gradient_descent + derivative_free without when");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(msg.contains("mutually exclusive"), "expected 'mutually exclusive' in: {msg}");
}

// 4. valid evolve with gradient_descent → no errors
#[test]
fn evolve_with_gradient_descent_passes() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let result = check_teleos(&module);
    assert!(result.is_ok(), "expected no errors, got: {:?}", result.unwrap_err());
}

// 5. all 5 strategies with when conditions → no errors
#[test]
fn evolve_with_all_strategies_passes() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![
            SearchCase { strategy: SearchStrategy::GradientDescent,    when: "gradient_available".to_string() },
            SearchCase { strategy: SearchStrategy::StochasticGradient, when: "noisy_landscape".to_string() },
            SearchCase { strategy: SearchStrategy::SimulatedAnnealing, when: "local_minima_risk".to_string() },
            SearchCase { strategy: SearchStrategy::DerivativeFree,     when: "state_space_unknown".to_string() },
            SearchCase { strategy: SearchStrategy::Mcmc,               when: "posterior_sampling".to_string() },
        ],
        constraint: "E[distance_to_telos] non-increasing".to_string(),
        span: Span::synthetic(),
    }));
    let result = check_teleos(&module);
    assert!(result.is_ok(), "expected no errors, got: {:?}", result.unwrap_err());
}

// 6. Rust output contains evolve_gradient_descent method
#[test]
fn rust_emit_evolve_has_gradient_method() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("evolve_gradient_descent"), "expected evolve_gradient_descent in:\n{out}");
}

// 7. Rust output contains evolve_step dispatcher
#[test]
fn rust_emit_evolve_has_dispatcher() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::SimulatedAnnealing,
            when: "local_minima_risk".to_string(),
        }],
        constraint: "E[distance_to_telos] converging".to_string(),
        span: Span::synthetic(),
    }));
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains("pub fn evolve_step"), "expected pub fn evolve_step in:\n{out}");
    assert!(out.contains("self.evolve_simulated_annealing()"), "expected dispatcher call in:\n{out}");
}

// 8. Rust output contains constraint string in a comment
#[test]
fn rust_emit_evolve_has_constraint_comment() {
    let constraint = "E[distance_to_telos] non-increasing";
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: constraint.to_string(),
        span: Span::synthetic(),
    }));
    let out = RustEmitter::new().emit(&module);
    assert!(out.contains(constraint), "expected constraint string in:\n{out}");
}

// 9. TypeScript output contains camelCase evolveGradientDescent method
#[test]
fn typescript_emit_evolve_has_camel_methods() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("evolveGradientDescent"), "expected evolveGradientDescent in:\n{out}");
}

// 10. TypeScript dispatcher contains convergence loop
#[test]
fn typescript_emit_evolve_dispatcher_has_loop() {
    let module = make_module(make_being_with_evolve(EvolveBlock {
        search_cases: vec![SearchCase {
            strategy: SearchStrategy::GradientDescent,
            when: "gradient_available".to_string(),
        }],
        constraint: "E[distance_to_telos] decreasing".to_string(),
        span: Span::synthetic(),
    }));
    let out = TypeScriptEmitter::new().emit(&module);
    assert!(out.contains("while (distance"), "expected 'while (distance' in:\n{out}");
}
