//! Tests for M64: Curry-Howard Correspondence

use loom::compile;

#[test]
fn structural_recursion_with_recursive_call_ok() {
    let src = r#"
module Test
fn factorial @pure :: Int -> Int
proof: structural_recursion
factorial(1)
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn structural_recursion_without_call_errors() {
    let src = r#"
module Test
fn factorial @pure :: Int -> Int
proof: structural_recursion
1
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(
        msg.contains("structural_recursion") || msg.contains("recursive") || msg.contains("curry"),
        "expected curry-howard error in: {}",
        msg
    );
}

#[test]
fn totality_with_match_ok() {
    let src = r#"
module Test
fn describe @pure :: Bool -> String
proof: totality
match true
| true -> "yes"
| false -> "no"
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn totality_without_match_errors() {
    let src = r#"
module Test
fn describe @pure :: Bool -> String
proof: totality
"yes"
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(
        msg.contains("totality") || msg.contains("match") || msg.contains("curry"),
        "expected totality error in: {}",
        msg
    );
}

#[test]
fn unknown_proof_strategy_ok() {
    let src = r#"
module Test
fn compute @pure :: Int -> Int
proof: wellFounded
1
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn proof_emitted_as_comment() {
    let src = r#"
module Test
fn factorial @pure :: Int -> Int
proof: structural_recursion
factorial(1)
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(
        rust_src.contains("structural_recursion") || rust_src.contains("proof"),
        "Expected proof comment in codegen"
    );
}
