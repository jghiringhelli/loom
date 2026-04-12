//! Tests for M67: Correctness Report
//! Verifies that `correctness_report:` blocks parse and are validated correctly.

use loom::compile;

#[test]
fn test_correctness_report_parses_basic() {
    let src = r#"
module Core
  correctness_report:
    proved:
      - membrane_integrity: separation_logic_proved
      - homeostasis: refinement_bounds_verified
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "basic correctness_report should parse: {:?}",
        result
    );
}

#[test]
fn test_correctness_report_parses_with_unverified() {
    let src = r#"
module Core
  correctness_report:
    proved:
      - type_safety: type_checker_passed
    unverified:
      - smt_integration: requires_z3_feature
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "correctness_report with unverified should parse: {:?}",
        result
    );
}

#[test]
fn test_correctness_report_emits_doc_comments() {
    let src = r#"
module Core
  correctness_report:
    proved:
      - type_safety: type_checker_passed
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "should compile: {:?}", result);
    let code = result.unwrap();
    assert!(
        code.contains("// LOOM[correctness_report]"),
        "codegen should emit LOOM annotation comment:\n{}",
        code
    );
    assert!(
        code.contains("type_safety"),
        "codegen should include proved claims:\n{}",
        code
    );
}

#[test]
fn test_correctness_report_rejects_duplicate_claims() {
    let src = r#"
module Bad
  correctness_report:
    proved:
      - type_safety: type_checker_passed
      - type_safety: refinement_checker_passed
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "duplicate proved claims should be rejected"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("duplicate proved claim")),
        "error should mention duplicate claim: {:?}",
        errs
    );
}

#[test]
fn test_correctness_report_rejects_multiple_reports() {
    let src = r#"
module Bad
  correctness_report:
    proved:
      - claim_a: checker_a
  end

  correctness_report:
    proved:
      - claim_b: checker_b
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "multiple correctness_report blocks should be rejected"
    );
    let errs = result.unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("correctness_report")),
        "error should mention the constraint: {:?}",
        errs
    );
}
