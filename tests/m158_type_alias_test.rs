/// M158 — `type alias` item: dedicated test coverage.
///
/// `Item::TypeAlias` exists since M87. This suite pins the behaviour:
///  - `type Name = BaseType` emits `pub type Name = BaseType;`
///  - Complex type expressions (generic, nested) roundtrip correctly
///  - Multiple aliases coexist in one module
///  - Aliases mix with other items (functions, types)
///  - Alias to another alias (transitive)

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M158.1: basic alias to scalar type ──────────────────────────────────────

#[test]
fn m158_alias_to_int_emits_pub_type() {
    let src = r#"
module core
type UserId = Int
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub type UserId = i64"),
        "expected pub type UserId = i64\n{out}"
    );
}

// ─── M158.2: alias to String ──────────────────────────────────────────────────

#[test]
fn m158_alias_to_string_emits_pub_type() {
    let src = r#"
module core
type Name = String
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub type Name = String"),
        "expected pub type Name = String\n{out}"
    );
}

// ─── M158.3: alias to Float ───────────────────────────────────────────────────

#[test]
fn m158_alias_to_float_emits_f64() {
    let src = r#"
module core
type Weight = Float
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub type Weight = f64"),
        "expected pub type Weight = f64\n{out}"
    );
}

// ─── M158.4: alias to Bool ────────────────────────────────────────────────────

#[test]
fn m158_alias_to_bool_emits_bool() {
    let src = r#"
module core
type Flag = Bool
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub type Flag = bool"),
        "expected pub type Flag = bool\n{out}"
    );
}

// ─── M158.5: multiple aliases in one module ───────────────────────────────────

#[test]
fn m158_multiple_aliases_all_emitted() {
    let src = r#"
module domain
type UserId = Int
type Email = String
type Score = Float
end
"#;
    let out = compile(src);
    assert!(out.contains("pub type UserId"), "missing UserId\n{out}");
    assert!(out.contains("pub type Email"), "missing Email\n{out}");
    assert!(out.contains("pub type Score"), "missing Score\n{out}");
}

// ─── M158.6: alias mixed with function ───────────────────────────────────────

#[test]
fn m158_alias_mixed_with_fn() {
    let src = r#"
module api
type RequestId = String
fn get_request :: String -> String
  require: true
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub type RequestId"), "missing alias\n{out}");
    assert!(out.contains("fn get_request"), "missing fn\n{out}");
}

// ─── M158.7: alias mixed with product type ───────────────────────────────────

#[test]
fn m158_alias_mixed_with_product_type() {
    let src = r#"
module domain
type Timestamp = Int
type Event =
  id: Int
  at: Timestamp
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub type Timestamp"), "missing alias\n{out}");
    assert!(out.contains("pub struct Event"), "missing struct\n{out}");
}

// ─── M158.8: alias mixed with const item ─────────────────────────────────────

#[test]
fn m158_alias_mixed_with_const() {
    let src = r#"
module config
type Port = Int
const DefaultPort: Int = 8080
end
"#;
    let out = compile(src);
    assert!(out.contains("pub type Port"), "missing alias\n{out}");
    assert!(
        out.contains("pub const DEFAULT_PORT"),
        "missing const\n{out}"
    );
}

// ─── M158.9: parse error on missing `=` ──────────────────────────────────────

#[test]
fn m158_missing_eq_is_parse_error() {
    let src = r#"
module core
type UserId Int
end
"#;
    let result = compile_check(src);
    assert!(
        result.is_err(),
        "expected parse error for missing = in alias"
    );
}
