//! Tests for M63: Category Theory Foundations

use loom::compile;

#[test]
fn functor_with_two_laws_ok() {
    let src = r#"
module Test
functor MyFunctor<F>
law: identity
law: composition
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn functor_with_one_law_errors() {
    let src = r#"
module Test
functor MyFunctor<F>
law: identity
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("functor") || msg.contains("law"), "expected functor error in: {}", msg);
}

#[test]
fn monad_with_three_laws_ok() {
    let src = r#"
module Test
monad Maybe<A>
law: left_identity
law: right_identity
law: associativity
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn monad_missing_law_errors() {
    let src = r#"
module Test
monad Maybe<A>
law: left_identity
law: right_identity
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("monad") || msg.contains("law") || msg.contains("associativity"), "expected monad error in: {}", msg);
}

#[test]
fn functor_emitted_as_trait() {
    let src = r#"
module Test
functor MyFunctor<F>
law: identity
law: composition
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("Functor") || rust_src.contains("functor"), "Expected Functor trait in codegen: {}", &rust_src[..rust_src.len().min(500)]);
}

#[test]
fn monad_emitted_as_trait() {
    let src = r#"
module Test
monad Maybe<A>
law: left_identity
law: right_identity
law: associativity
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("Monad") || rust_src.contains("monad"), "Expected Monad trait in codegen");
}
