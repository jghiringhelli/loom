//! Tests for M61: Dependent Types

use loom::compile;

#[test]
fn proposition_parses_ok() {
    let src = r#"
module Test
proposition NonNegative = Int
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn proposition_with_where_ok() {
    let src = r#"
module Test
proposition Positive = Int where 1 > 0
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn termination_on_pure_fn_ok() {
    let src = r#"
module Test
fn factorial @pure :: Int -> Int
termination: structural_recursion
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn termination_on_impure_fn_errors() {
    let src = r#"
module Test
fn factorial :: Int -> Int
termination: structural_recursion
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("termination") || msg.contains("pure"), "expected termination error in: {}", msg);
}

#[test]
fn proposition_emitted_as_type_alias() {
    let src = r#"
module Test
proposition NonNegative = Int
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("NonNegative") || rust_src.contains("proposition"), "Expected proposition in rust output: {}", &rust_src[..rust_src.len().min(500)]);
}

#[test]
fn multiple_propositions_ok() {
    let src = r#"
module Test
proposition PositiveInt = Int
proposition NonEmptyString = String
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}
