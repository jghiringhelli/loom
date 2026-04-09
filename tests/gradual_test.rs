//! Tests for M59: Gradual Typing

use loom::compile;

#[test]
fn gradual_fn_with_block_ok() {
    let src = r#"
module Test
fn process @pure :: ? -> String
gradual:
  input_type: ?
  boundary: SafeBoundary
  output_type: String
  on_cast_failure: panic
  blame: caller
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK but got: {:?}", compile(src));
}

#[test]
fn gradual_fn_without_block_errors() {
    let src = r#"
module Test
fn process @pure :: ? -> String
end
end"#;
    let result = compile(src);
    assert!(result.is_err(), "expected error for missing gradual block");
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("gradual") || msg.contains("dynamic"), "expected gradual error in: {}", msg);
}

#[test]
fn on_cast_failure_without_blame_errors() {
    let src = r#"
module Test
fn process @pure :: ? -> String
gradual:
  input_type: ?
  on_cast_failure: panic
end
end
end"#;
    let result = compile(src);
    assert!(result.is_err(), "expected error for missing blame");
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("blame"), "expected blame mention in: {}", msg);
}

#[test]
fn on_cast_failure_with_blame_ok() {
    let src = r#"
module Test
fn process @pure :: ? -> String
gradual:
  input_type: ?
  on_cast_failure: panic
  blame: caller
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn dynamic_in_return_type_needs_gradual() {
    let src = r#"
module Test
fn compute @pure :: Int -> ?
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("gradual") || msg.contains("dynamic"));
}

#[test]
fn gradual_block_in_codegen() {
    let src = r#"
module Test
fn process @pure :: ? -> String
gradual:
  input_type: ?
  boundary: SafeBoundary
  output_type: String
  on_cast_failure: panic
  blame: caller
end
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("gradual typing") || rust_src.contains("gradual"), "Expected gradual comment in: {}", &rust_src[..rust_src.len().min(500)]);
}

#[test]
fn dynamic_type_emits_box_dyn_any() {
    let src = r#"
module Test
fn process @pure :: ? -> String
gradual:
  input_type: ?
  blame: system
end
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("Box<dyn std::any::Any>"), "Expected Box<dyn Any> in: {}", &rust_src[..rust_src.len().min(500)]);
}

#[test]
fn gradual_block_minimal_ok() {
    let src = r#"
module Test
fn process @pure :: ? -> ?
gradual:
  boundary: GradualBoundary
  blame: system
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}
