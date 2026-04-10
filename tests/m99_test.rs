// M99: Algebraic Effect Handlers — Plotkin & Pretnar (2009).
// Tests for effect definition parsing and handler exhaustiveness checking.

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// 1. effect definition with single operation parses.
#[test]
fn test_m99_effect_single_operation_parses() {
    let src = r#"
module Logging
  effect Log
    operation emit :: String -> Unit
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "effect with single operation should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let effect = module.items.iter().find_map(|i| {
        if let Item::Effect(e) = i {
            Some(e)
        } else {
            None
        }
    });
    assert!(effect.is_some(), "should have an Effect item");
    let e = effect.unwrap();
    assert_eq!(e.name, "Log");
    assert_eq!(e.operations.len(), 1);
    assert_eq!(e.operations[0].name, "emit");
}

// 2. effect definition with multiple operations parses.
#[test]
fn test_m99_effect_multiple_operations_parses() {
    let src = r#"
module StateEffect
  effect State
    operation get :: Unit -> Int
    operation put :: Int -> Unit
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "effect with multiple operations should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let effect = module.items.iter().find_map(|i| {
        if let Item::Effect(e) = i {
            Some(e)
        } else {
            None
        }
    });
    assert!(effect.is_some());
    let e = effect.unwrap();
    assert_eq!(e.operations.len(), 2);
    assert_eq!(e.operations[0].name, "get");
    assert_eq!(e.operations[1].name, "put");
}

// 3. effect with type parameter parses: effect State<S>.
#[test]
fn test_m99_effect_with_type_parameter_parses() {
    let src = r#"
module Polymorphic
  effect State<S>
    operation get :: Unit -> S
    operation put :: S -> Unit
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "effect with type parameter should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let effect = module.items.iter().find_map(|i| {
        if let Item::Effect(e) = i {
            Some(e)
        } else {
            None
        }
    });
    assert!(effect.is_some());
    let e = effect.unwrap();
    assert_eq!(e.name, "State");
    assert_eq!(e.type_params, vec!["S".to_string()]);
}

// 4. handle block in function parses.
#[test]
fn test_m99_handle_block_in_function_parses() {
    let src = r#"
module Runner
  effect Log
    operation emit :: String -> Unit
  end

  fn run_with_log :: Unit -> Unit
    handle computation with
      Log.emit(msg) -> k:
        k(unit)
      end
    end
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "handle block in function should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let fn_def = module
        .items
        .iter()
        .find_map(|i| if let Item::Fn(f) = i { Some(f) } else { None });
    assert!(fn_def.is_some());
    let f = fn_def.unwrap();
    assert!(
        f.handle_block.is_some(),
        "function should have a handle_block"
    );
    let hb = f.handle_block.as_ref().unwrap();
    assert_eq!(hb.computation, "computation");
}

// 5. handle with multiple effect operations parses.
#[test]
fn test_m99_handle_multiple_operations_parses() {
    let src = r#"
module StatefulRunner
  effect State
    operation get :: Unit -> Int
    operation put :: Int -> Unit
  end

  fn run_stateful :: Unit -> Int
    handle computation with
      State.get() -> k:
        k(current)
      end
      State.put(new_val) -> k:
        k(unit)
      end
    end
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "handle with multiple operations should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let fn_def = module
        .items
        .iter()
        .find_map(|i| if let Item::Fn(f) = i { Some(f) } else { None });
    assert!(fn_def.is_some());
    let f = fn_def.unwrap();
    let hb = f
        .handle_block
        .as_ref()
        .expect("handle_block should be present");
    assert_eq!(hb.handlers.len(), 2);
}

// 6. effect name referenced in Effect<[Log]> type expression parses.
#[test]
fn test_m99_effect_in_type_expression_parses() {
    let src = r#"
module App
  effect Log
    operation emit :: String -> Unit
  end

  fn compute :: Unit -> Effect<[Log], Int>
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "effect name in Effect<[Log]> type expression should parse: {:?}",
        result.err()
    );
}
