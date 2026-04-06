//! Units of Measure tests.
//!
//! Verifies the unit-checker, Rust/TypeScript/JSON-Schema codegen, and the
//! full `compile()` pipeline for unit-annotated primitive types.

use loom::checker::UnitsChecker;
use loom::LoomError;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_module(src: &str) -> loom::ast::Module {
    let tokens = loom::lexer::Lexer::tokenize(src).expect("lex failed");
    loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("parse failed")
}

fn check_units(src: &str) -> Result<(), Vec<LoomError>> {
    let module = parse_module(src);
    UnitsChecker::new().check(&module)
}

fn has_type_error(src: &str) -> bool {
    check_units(src)
        .err()
        .map(|errs| errs.iter().any(|e| e.kind() == "TypeError"))
        .unwrap_or(false)
}

// ── 1. Parsing ────────────────────────────────────────────────────────────────

#[test]
fn float_with_unit_parses_as_generic() {
    use loom::ast::{Item, TypeExpr};
    let src = r#"
module M
fn foo :: Float<usd> -> Float<usd>
  price
end
end
"#;
    let module = parse_module(src);
    if let Item::Fn(fd) = &module.items[0] {
        assert_eq!(
            fd.type_sig.params[0],
            TypeExpr::Generic(
                "Float".to_string(),
                vec![TypeExpr::Base("usd".to_string())]
            )
        );
    } else {
        panic!("expected fn item");
    }
}

#[test]
fn int_with_unit_parses_as_generic() {
    use loom::ast::{Item, TypeExpr};
    let src = r#"
module M
fn distance :: Int<meters> -> Int<meters>
  d
end
end
"#;
    let module = parse_module(src);
    if let Item::Fn(fd) = &module.items[0] {
        assert_eq!(
            fd.type_sig.params[0],
            TypeExpr::Generic(
                "Int".to_string(),
                vec![TypeExpr::Base("meters".to_string())]
            )
        );
    } else {
        panic!("expected fn item");
    }
}

// ── 2. Checker: same-unit addition is OK ─────────────────────────────────────

#[test]
fn add_same_unit_ok() {
    let src = r#"
module M
fn add_prices :: Float<usd> -> Float<usd> -> Float<usd>
  price1 + price2
end
end
"#;
    assert!(check_units(src).is_ok(), "same-unit addition should be allowed");
}

#[test]
fn sub_same_unit_ok() {
    let src = r#"
module M
fn diff :: Float<eur> -> Float<eur> -> Float<eur>
  a - b
end
end
"#;
    assert!(check_units(src).is_ok(), "same-unit subtraction should be allowed");
}

// ── 3. Checker: different-unit addition errors ────────────────────────────────

#[test]
fn add_different_units_errors() {
    let src = r#"
module M
fn bad_add :: Float<usd> -> Float<eur> -> Float<usd>
  a + b
end
end
"#;
    assert!(has_type_error(src), "cross-unit addition should produce a TypeError");
}

#[test]
fn sub_different_units_errors() {
    let src = r#"
module M
fn bad_sub :: Float<celsius> -> Float<fahrenheit> -> Float<celsius>
  a - b
end
end
"#;
    assert!(has_type_error(src), "cross-unit subtraction should produce a TypeError");
}

// ── 4. Rust codegen: unit newtype struct emitted ─────────────────────────────

#[test]
fn rust_codegen_emits_usd_struct() {
    let src = r#"
module M
fn foo :: Float<usd> -> Float<usd>
  price
end
end
"#;
    let rust = loom::compile(src).expect("compile failed");
    assert!(rust.contains("pub struct Usd"), "Rust output should contain `pub struct Usd`");
    assert!(rust.contains("impl std::ops::Add for Usd"), "should have Add impl");
}

#[test]
fn rust_codegen_emits_usd_in_fn_signature() {
    let src = r#"
module M
fn foo :: Float<usd> -> Float<usd>
  price
end
end
"#;
    let rust = loom::compile(src).expect("compile failed");
    // The function signature should use `Usd` not `f64`
    assert!(rust.contains("Usd"), "Rust output should use Usd in fn signature");
}

// ── 5. TypeScript codegen: branded type emitted ──────────────────────────────

#[test]
fn typescript_emits_branded_type_alias() {
    let src = r#"
module M
fn foo :: Float<usd> -> Float<usd>
  price
end
end
"#;
    let ts = loom::compile_typescript(src).expect("ts compile failed");
    assert!(ts.contains("_unit"), "TS output should contain `_unit` brand");
    assert!(ts.contains("Usd"), "TS output should contain `Usd` type alias");
}

// ── 6. JSON Schema: x-unit extension emitted ─────────────────────────────────

#[test]
fn json_schema_emits_x_unit() {
    let src = r#"
module M
type Money = amount: Float<usd> end
end
"#;
    let schema = loom::compile_json_schema(src).expect("schema compile failed");
    assert!(schema.contains("x-unit"), "JSON Schema should contain `x-unit`");
    assert!(schema.contains("usd"), "JSON Schema should contain `usd`");
}

// ── 7. Dimensionless Float addition is OK ────────────────────────────────────

#[test]
fn dimensionless_float_add_ok() {
    let src = r#"
module M
fn add :: Float -> Float -> Float
  x + y
end
end
"#;
    assert!(check_units(src).is_ok(), "dimensionless Float addition should be OK");
}

// ── 8. Unit in struct field emits correctly ──────────────────────────────────

#[test]
fn struct_field_unit_emits_in_rust() {
    let src = r#"
module M
type Money = amount: Float<usd> end
end
"#;
    let rust = loom::compile(src).expect("compile failed");
    assert!(rust.contains("pub struct Usd"), "should emit Usd newtype");
    assert!(rust.contains("amount: Usd"), "struct field should use Usd type");
}

// ── 9. Multiplication of different units is allowed ──────────────────────────

#[test]
fn multiply_different_units_allowed() {
    let src = r#"
module M
fn apply_rate :: Float<usd> -> Float<rate> -> Float<usd>
  amount * rate
end
end
"#;
    assert!(
        check_units(src).is_ok(),
        "unit multiplication should not trigger a unit-mismatch error"
    );
}

// ── 10. Full pipeline with unit types succeeds ───────────────────────────────

#[test]
fn full_pipeline_with_units_ok() {
    let src = r#"
module Pricing
fn total :: Float<usd> -> Float<usd> -> Float<usd>
  base + tax
end
end
"#;
    assert!(
        loom::compile(src).is_ok(),
        "full compile() pipeline should succeed with unit types"
    );
}

// ── 11. Multi-unit module emits multiple newtypes ────────────────────────────

#[test]
fn multi_unit_module_emits_multiple_structs() {
    let src = r#"
module M
fn convert :: Float<usd> -> Float<eur>
  amount
end
end
"#;
    let rust = loom::compile(src).expect("compile failed");
    assert!(rust.contains("pub struct Usd"), "should emit Usd");
    assert!(rust.contains("pub struct Eur"), "should emit Eur");
}

// ── 12. Let-bound unit propagation doesn't false-positive ────────────────────

#[test]
fn let_bound_same_unit_no_error() {
    let src = r#"
module M
fn sum :: Float<usd> -> Float<usd> -> Float<usd>
  let a = x
  a + y
end
end
"#;
    assert!(
        check_units(src).is_ok(),
        "let-bound same-unit addition should not error"
    );
}
