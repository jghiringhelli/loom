//! Tests for M66b: Annotation Algebra
//! Verifies that `annotation` declarations with typed params and meta-annotations parse correctly.

use loom::compile;

#[test]
fn test_annotation_decl_parses_basic() {
    let src = r#"
module Auth
  annotation requires_auth()
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "basic annotation decl should parse: {:?}",
        result
    );
}

#[test]
fn test_annotation_decl_parses_with_params() {
    let src = r#"
module Transfer
  annotation concurrent_transfer(a: String, b: String)
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "annotation with params should parse: {:?}",
        result
    );
}

#[test]
fn test_annotation_decl_emits_doc_comment() {
    let src = r#"
module Transfer
  annotation concurrent_transfer(a: String, b: String)
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "should compile: {:?}", result);
    let code = result.unwrap();
    assert!(
        code.contains("// annotation concurrent_transfer"),
        "codegen should emit annotation comment:\n{}",
        code
    );
}

#[test]
fn test_annotation_decl_rejects_duplicate_param_names() {
    let src = r#"
module Bad
  annotation duplicate_param(x: String, x: Int)
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "duplicate param names should be rejected");
    let errs = result.unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("duplicate parameter")),
        "error should mention duplicate param: {:?}",
        errs
    );
}

#[test]
fn test_annotation_decl_in_module_with_fns() {
    let src = r#"
module Payments
  annotation idempotent_op(op: String)

  fn process :: String -> Bool
    true
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "annotation decl with fns should compile: {:?}",
        result
    );
}
