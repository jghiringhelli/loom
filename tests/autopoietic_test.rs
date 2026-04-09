//! M51 tests — autopoietic: declaration + Maturana/Varela operational closure checker.

use loom::checker::check_teleos;
use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// 1. autopoietic_true_with_all_requirements_passes
#[test]
fn autopoietic_true_with_all_requirements_passes() {
    let src = r#"module Test
being Cell
  describe: "minimal autopoietic being"
  autopoietic: true
  matter:
    membrane: Float
  end
  telos: "maintain self-organization"
  end
  regulate glucose
    target: optimal
    bounds: (low, high)
  end
  evolve
    toward: telos
    search:
    | gradient_descent when gradient_available
    constraint: "E[distance_to_telos] decreasing"
  end
end
end
"#;
    let module = parse(src);
    assert_eq!(module.being_defs.len(), 1);
    assert!(
        module.being_defs[0].autopoietic,
        "expected autopoietic=true"
    );
    let result = check_teleos(&module);
    assert!(
        result.is_ok(),
        "expected autopoietic being with all requirements to pass: {:?}",
        result
    );
}

// 2. autopoietic_missing_regulate_fails
#[test]
fn autopoietic_missing_regulate_fails() {
    let src = r#"module Test
being Cell
  autopoietic: true
  matter:
    membrane: Float
  end
  telos: "maintain self-organization"
  end
  evolve
    toward: telos
    search:
    | gradient_descent when gradient_available
    constraint: "E[distance_to_telos] decreasing"
  end
end
end
"#;
    let module = parse(src);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for missing regulate:");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(
        msg.contains("regulate"),
        "expected 'regulate' in error message: {msg}"
    );
}

// 3. autopoietic_missing_evolve_fails
#[test]
fn autopoietic_missing_evolve_fails() {
    let src = r#"module Test
being Cell
  autopoietic: true
  matter:
    membrane: Float
  end
  telos: "maintain self-organization"
  end
  regulate glucose
    target: optimal
    bounds: (low, high)
  end
end
end
"#;
    let module = parse(src);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for missing evolve:");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(
        msg.contains("evolve"),
        "expected 'evolve' in error message: {msg}"
    );
}

// 4. autopoietic_missing_matter_fails
#[test]
fn autopoietic_missing_matter_fails() {
    let src = r#"module Test
being Cell
  autopoietic: true
  telos: "maintain self-organization"
  end
  regulate glucose
    target: optimal
    bounds: (low, high)
  end
  evolve
    toward: telos
    search:
    | gradient_descent when gradient_available
    constraint: "E[distance_to_telos] decreasing"
  end
end
end
"#;
    let module = parse(src);
    let result = check_teleos(&module);
    assert!(result.is_err(), "expected error for missing matter:");
    let errors = result.unwrap_err();
    let msg = errors.iter().map(|e| format!("{e}")).collect::<String>();
    assert!(
        msg.contains("matter"),
        "expected 'matter' in error message: {msg}"
    );
}

// 5. rust_emit_autopoietic_has_is_autopoietic_fn
#[test]
fn rust_emit_autopoietic_has_is_autopoietic_fn() {
    let src = r#"module Test
being Cell
  autopoietic: true
  matter:
    membrane: Float
  end
  telos: "maintain self-organization"
  end
  regulate glucose
    target: optimal
    bounds: (low, high)
  end
  evolve
    toward: telos
    search:
    | gradient_descent when gradient_available
    constraint: "E[distance_to_telos] decreasing"
  end
end
end
"#;
    let module = parse(src);
    let out = RustEmitter::new().emit(&module);
    assert!(
        out.contains("is_autopoietic"),
        "expected is_autopoietic fn in:\n{out}"
    );
    assert!(
        out.contains("verify_closure"),
        "expected verify_closure fn in:\n{out}"
    );
}
