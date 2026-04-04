//! Exhaustiveness checker tests.
//!
//! These tests verify that `match` expressions on sum types are exhaustive.
//! All tests use `loom::compile` to exercise the full pipeline.

use loom::LoomError;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compile(src: &str) -> Result<String, Vec<LoomError>> {
    loom::compile(src)
}

fn exhaustiveness_errors(src: &str) -> Vec<LoomError> {
    match compile(src) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .filter(|e| matches!(e, LoomError::NonExhaustiveMatch { .. }))
            .collect(),
    }
}

fn missing_from_error(e: &LoomError) -> Vec<String> {
    match e {
        LoomError::NonExhaustiveMatch { missing, .. } => {
            let mut m = missing.clone();
            m.sort();
            m
        }
        _ => panic!("expected NonExhaustiveMatch, got {:?}", e),
    }
}

// ── Passing cases (exhaustive) ────────────────────────────────────────────────

#[test]
fn exhaustive_match_all_variants_covered_passes() {
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | Green -> 2
  | Blue -> 3
  end
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "fully exhaustive match should compile without errors"
    );
}

#[test]
fn wildcard_arm_covers_all_passes() {
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | _ -> 0
  end
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "wildcard arm should satisfy exhaustiveness"
    );
}

#[test]
fn variable_ident_arm_covers_all_passes() {
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | other -> 0
  end
end
end
"#;
    assert!(
        compile(src).is_ok(),
        "variable-ident arm should satisfy exhaustiveness"
    );
}

#[test]
fn non_enum_match_is_not_checked() {
    // A match where patterns are only literals — no enum involved.
    // The checker should not flag this.
    let src = r#"
module M
fn classify :: Int -> Int
  match n
  | 1 -> 10
  | 2 -> 20
  end
end
end
"#;
    let errors = exhaustiveness_errors(src);
    assert!(
        errors.is_empty(),
        "literal-only match should not trigger exhaustiveness error"
    );
}

// ── Failing cases (non-exhaustive) ───────────────────────────────────────────

#[test]
fn missing_one_variant_errors() {
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | Green -> 2
  end
end
end
"#;
    let errors = exhaustiveness_errors(src);
    assert_eq!(errors.len(), 1, "expected exactly one exhaustiveness error");
    assert_eq!(
        missing_from_error(&errors[0]),
        vec!["Blue"],
        "missing variant should be Blue"
    );
}

#[test]
fn missing_multiple_variants_errors() {
    let src = r#"
module M
enum Direction = | North | South | East | West end
fn to_int :: Int -> Int
  match d
  | North -> 0
  end
end
end
"#;
    let errors = exhaustiveness_errors(src);
    assert_eq!(errors.len(), 1, "expected exactly one exhaustiveness error");
    assert_eq!(
        missing_from_error(&errors[0]),
        vec!["East", "South", "West"],
        "missing variants should be South, East, West (sorted)"
    );
}

#[test]
fn guarded_arm_does_not_count_as_covering() {
    // Blue has a guard, so it does not cover Blue unconditionally.
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | Green -> 2
  | Blue if cond -> 3
  end
end
end
"#;
    let errors = exhaustiveness_errors(src);
    assert_eq!(errors.len(), 1, "guarded arm must not count as a cover");
    assert_eq!(missing_from_error(&errors[0]), vec!["Blue"]);
}

#[test]
fn wildcard_with_guard_does_not_count_as_total_cover() {
    let src = r#"
module M
enum Color = | Red | Green | Blue end
fn describe :: Int -> Int
  match x
  | Red -> 1
  | Green -> 2
  | _ if cond -> 0
  end
end
end
"#;
    let errors = exhaustiveness_errors(src);
    assert_eq!(
        errors.len(),
        1,
        "guarded wildcard must not count as total cover"
    );
    assert_eq!(missing_from_error(&errors[0]), vec!["Blue"]);
}
