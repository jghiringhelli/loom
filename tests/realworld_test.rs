//! Real-world integration tests — compile complete Loom programs and validate
//! the exact content of the emitted Rust source.
//!
//! Each test:
//!  1. Defines a non-trivial Loom program covering multiple language features.
//!  2. Compiles it with `loom::compile`.
//!  3. Asserts on the *content* of the emitted Rust — not just that it succeeded.
//!
//! Features covered:
//!  - Product types (structs)
//!  - Sum types (enums with payloads)
//!  - `match` expressions with pattern binding
//!  - `let` bindings chained in a function body
//!  - Arithmetic and comparison `BinOp`
//!  - Pipe operator `|>`
//!  - Field access `object.field`
//!  - `require:` contracts → `debug_assert!`
//!  - `ensure:` contracts → comment
//!  - Effect types → `Result<T, Box<dyn Error>>`
//!  - Refined types → newtype + `TryFrom`
//!  - `todo` placeholder → `todo!()`
//!  - Dependency injection: `requires` + `with`
//!  - Generic type parameters `<T>`
//!  - `List<T>` / `Map<K,V>` stdlib mappings
//!  - `provides` interface → Rust trait

fn compile_ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|errors| {
        panic!(
            "compilation failed:\n{}",
            errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        )
    })
}

// ── 1. Structs + field access + arithmetic in function body ───────────────────

#[test]
fn struct_field_arithmetic_emits_correctly() {
    let src = r#"
module Pricing
type Item =
  quantity: Int,
  price: Float
end

fn total_cost :: Item -> Float
  item.quantity * item.price
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct Item"), "missing struct");
    assert!(out.contains("pub quantity: i64"), "missing field quantity");
    assert!(out.contains("pub price: f64"), "missing field price");
    assert!(out.contains("pub fn total_cost("), "missing function");
    // Field access + arithmetic in the body
    assert!(
        out.contains("item.quantity") && out.contains("item.price"),
        "expected field access in body:\n{}", out
    );
    assert!(out.contains('*'), "expected multiplication in body:\n{}", out);
}

// ── 2. Enum + match with payload binding ─────────────────────────────────────

#[test]
fn enum_match_with_payload_emits_rust_match() {
    let src = r#"
module Shapes
enum Shape =
  | Circle of Float
  | Rectangle of Int
  | Triangle
end

fn describe :: Shape -> Int
  match shp
  | Circle(r) -> 1
  | Rectangle(w) -> 2
  | Triangle -> 3
  end
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub enum Shape"), "missing enum");
    assert!(out.contains("Circle(f64)"), "missing Circle variant");
    assert!(out.contains("Rectangle(i64)"), "missing Rectangle variant");
    assert!(out.contains("Triangle,"), "missing Triangle variant");
    // Match expression in body
    assert!(out.contains("match shp"), "missing match subject");
    assert!(out.contains("Circle(r) =>"), "missing Circle arm");
    assert!(out.contains("Rectangle(w) =>"), "missing Rectangle arm");
    assert!(out.contains("Triangle =>"), "missing Triangle arm");
}

// ── 3. Chained let bindings in function body ──────────────────────────────────

#[test]
fn chained_let_bindings_emit_as_statements() {
    let src = r#"
module Calc
fn compute :: Int -> Int
  let base = 100
  let rate = 15
  let tax = base * rate
  tax
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("let base = 100"), "missing let base");
    assert!(out.contains("let rate = 15"), "missing let rate");
    assert!(out.contains("let tax ="), "missing let tax");
    // `tax` should be the final expression (no semicolon)
    let fn_body: String = out
        .lines()
        .skip_while(|l| !l.contains("pub fn compute"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(fn_body.contains("tax\n"), "expected tax as final expression:\n{}", fn_body);
}

// ── 4. Pipe operator ─────────────────────────────────────────────────────────

#[test]
fn pipe_operator_emits_intermediate_let() {
    let src = r#"
module Pipeline
fn double :: Int -> Int
  0
end

fn add_ten :: Int -> Int
  0
end

fn process :: Int -> Int
  42 |> double |> add_ten
end
end
"#;
    let out = compile_ok(src);
    // Pipe chains expand to `{ let _pipe = ...; f(_pipe) }`
    assert!(out.contains("_pipe"), "expected pipe expansion:\n{}", out);
    assert!(out.contains("double(_pipe)") || out.contains("double("), "expected double call");
    assert!(out.contains("add_ten("), "expected add_ten call");
}

// ── 5. require/ensure contracts ───────────────────────────────────────────────

#[test]
fn contracts_emit_debug_assert_and_comment() {
    let src = r#"
module Safe
fn divide :: Int -> Int -> Int
  require: divisor > 0
  ensure: result > 0
  42
end
end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("debug_assert!("),
        "expected debug_assert! for require:\n{}", out
    );
    assert!(
        out.contains("debug_assert!") && out.contains("ensure:"),
        "expected debug_assert! with ensure: label for ensure:\n{}", out
    );
    assert!(
        out.contains("divisor > 0"),
        "expected divisor > 0 in assert:\n{}", out
    );
}

// ── 6. Effectful function → Result return type ────────────────────────────────

#[test]
fn effect_fn_emits_result_return_type() {
    let src = r#"
module IO
fn fetch_data :: Int -> Effect<[IO], String>
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(
        out.contains("Result<String, Box<dyn std::error::Error>>"),
        "expected Result return type:\n{}", out
    );
    // todo! should be emitted as macro, not bare identifier
    assert!(out.contains("todo!()"), "expected todo!() in body:\n{}", out);
    assert!(!out.contains("    todo\n"), "unexpected bare todo identifier:\n{}", out);
}

// ── 7. Refined type → newtype + TryFrom ──────────────────────────────────────

#[test]
fn refined_type_emits_newtype_and_try_from() {
    let src = r#"
module Validated
type Score = Int where score_valid
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub struct Score(i64)"), "expected newtype:\n{}", out);
    assert!(
        out.contains("impl TryFrom<i64> for Score"),
        "expected TryFrom impl:\n{}", out
    );
    assert!(
        out.contains("if !(score_valid)") || out.contains("if !score_valid"),
        "expected predicate validation check:\n{}", out
    );
}

// ── 8. Provides interface → Rust trait ───────────────────────────────────────

#[test]
fn provides_emits_rust_trait() {
    let src = r#"
module Calculator
provides {
  add :: Int -> Int -> Int,
  multiply :: Int -> Int -> Int
}

fn add :: Int -> Int -> Int
  0
end

fn multiply :: Int -> Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub trait Calculator"), "expected trait:\n{}", out);
    assert!(out.contains("fn add("), "expected fn add in trait:\n{}", out);
    assert!(out.contains("fn multiply("), "expected fn multiply in trait:\n{}", out);
}

// ── 9. DI: requires + with injects ctx parameter ─────────────────────────────

#[test]
fn di_full_roundtrip_emits_context_and_ctx_param() {
    let src = r#"
module UserRepo
requires { db: Database, cache: Cache }

type User =
  id: Int,
  name: String
end

fn find_user :: Int -> String
with db
  todo
end

fn get_cached :: Int -> String
with cache
  todo
end

fn count_users :: Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);

    // Context struct
    assert!(out.contains("UserRepoContext"), "expected context struct:\n{}", out);
    assert!(out.contains("pub db: Database"), "expected db field:\n{}", out);
    assert!(out.contains("pub cache: Cache"), "expected cache field:\n{}", out);

    // ctx param on with-deps functions
    let find_line = out.lines().find(|l| l.contains("pub fn find_user")).unwrap_or("");
    assert!(find_line.contains("ctx: &UserRepoContext"), "expected ctx in find_user: {}", find_line);

    let get_line = out.lines().find(|l| l.contains("pub fn get_cached")).unwrap_or("");
    assert!(get_line.contains("ctx: &UserRepoContext"), "expected ctx in get_cached: {}", get_line);

    // Pure function — no ctx
    let count_line = out.lines().find(|l| l.contains("pub fn count_users")).unwrap_or("");
    assert!(!count_line.contains("ctx"), "unexpected ctx in count_users: {}", count_line);
}

// ── 10. Generic function with collection return ───────────────────────────────

#[test]
fn generic_fn_with_collection_param_and_return() {
    let src = r#"
module Collections
fn identity<T> :: T -> T
  todo
end

fn wrap<T> :: T -> List<T>
  todo
end

fn first_key<K, V> :: Map<K, V> -> K
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub fn identity<T>("), "expected generic identity:\n{}", out);
    assert!(out.contains("pub fn wrap<T>("), "expected generic wrap:\n{}", out);
    assert!(out.contains("-> Vec<T>"), "expected Vec<T> return:\n{}", out);
    assert!(out.contains("pub fn first_key<K, V>("), "expected two-param generic:\n{}", out);
    assert!(out.contains("HashMap<K, V>"), "expected HashMap<K,V>:\n{}", out);
}

// ── 11. Wildcard match covers all variants ────────────────────────────────────

#[test]
fn match_with_wildcard_arm_emits_underscore() {
    let src = r#"
module Status
enum Color = | Red | Green | Blue end

fn is_red :: Color -> Bool
  match col
  | Red -> true
  | _ -> false
  end
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("Red =>"), "expected Red arm:\n{}", out);
    assert!(out.contains("_ =>"), "expected wildcard arm:\n{}", out);
}

// ── 12. Comparison chain produces Bool ────────────────────────────────────────

#[test]
fn comparison_operators_emit_correctly() {
    let src = r#"
module Compare
fn in_range :: Int -> Bool
  let lo = 0
  let hi = 100
  lo < hi
end
end
"#;
    let out = compile_ok(src);
    let fn_body: String = out
        .lines()
        .skip_while(|l| !l.contains("pub fn in_range"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(fn_body.contains("(lo < hi)"), "expected comparison in body:\n{}", fn_body);
}

// ── 13. Match with guard condition ────────────────────────────────────────────

#[test]
fn match_arm_with_guard_emits_if_clause() {
    let src = r#"
module Guard
enum Val = | Num of Int | Other end

fn check :: Val -> Int
  match v
  | Num(n) if n > 0 -> 1
  | Num(n) -> 0
  | Other -> 2
  end
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("if (n > 0)"), "expected guard in match arm:\n{}", out);
}

// ── 14. Stdlib HashMap import only injected when needed ──────────────────────

#[test]
fn hashmap_import_only_when_map_type_present() {
    let without_map = r#"
module Pure
fn add :: Int -> Int -> Int
  0
end
end
"#;
    let out = compile_ok(without_map);
    assert!(!out.contains("HashMap"), "unexpected HashMap in pure module:\n{}", out);

    let with_map = r#"
module Cache
fn store :: Map<String, Int> -> Bool
  true
end
end
"#;
    let out2 = compile_ok(with_map);
    assert!(out2.contains("use std::collections::HashMap"), "expected HashMap import:\n{}", out2);
    assert!(out2.contains("HashMap<String, i64>"), "expected HashMap type:\n{}", out2);
}

// ── 15. Full pricing_engine corpus with output assertions ─────────────────────

#[test]
fn pricing_engine_emits_expected_rust_constructs() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").unwrap();
    let out = compile_ok(&src);

    // Structs
    assert!(out.contains("pub struct OrderLine {"), "missing OrderLine struct");
    assert!(out.contains("pub struct OrderTotal {"), "missing OrderTotal struct");
    assert!(out.contains("pub quantity: i64"), "missing quantity field");
    assert!(out.contains("pub unit_price: f64"), "missing unit_price field");
    assert!(out.contains("pub discount: f64"), "missing discount field");

    // Function
    assert!(out.contains("pub fn compute_total("), "missing compute_total fn");

    // Contracts
    assert!(out.contains("debug_assert!("), "missing debug_assert from require:");
    assert!(out.contains("debug_assert!") && out.contains("ensure:"), "missing ensure debug_assert");

    // let bindings in body
    assert!(out.contains("let subtotal ="), "missing let subtotal");
    assert!(out.contains("let discounted ="), "missing let discounted");
    assert!(out.contains("let tax ="), "missing let tax");
    assert!(out.contains("let total ="), "missing let total");

    // Field access
    assert!(out.contains("line.quantity"), "missing line.quantity field access");
    assert!(out.contains("line.unit_price"), "missing line.unit_price field access");
}

// ── 16. user_service corpus with enums, effects, refined types ───────────────

#[test]
fn user_service_emits_enum_and_refined_type() {
    let src = std::fs::read_to_string("corpus/user_service.loom").unwrap();
    let out = compile_ok(&src);

    // Enum with payloads
    assert!(out.contains("pub enum UserError"), "missing UserError enum");
    assert!(out.contains("NotFound(String)"), "missing NotFound variant");
    assert!(out.contains("InvalidInput(String)"), "missing InvalidInput variant");
    assert!(out.contains("PermissionDenied,"), "missing PermissionDenied variant");

    // Refined type
    assert!(out.contains("pub struct Email(String)"), "missing Email newtype");
    assert!(out.contains("impl TryFrom<String> for Email"), "missing TryFrom for Email");

    // Effect function
    assert!(out.contains("pub fn find_user("), "missing find_user fn");
    assert!(
        out.contains("Result<User, Box<dyn std::error::Error>>"),
        "missing Result return type"
    );
}

// ── 17. Module snake_case naming ─────────────────────────────────────────────

#[test]
fn module_name_converted_to_snake_case_in_rust() {
    let src = r#"
module PricingEngine
fn run :: Int -> Int
  0
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("pub mod pricing_engine"), "expected snake_case module:\n{}", out);
}

// ── 18. todo in function body emits macro not identifier ─────────────────────

#[test]
fn todo_in_body_emits_macro_call() {
    let src = r#"
module M
fn stub :: Int -> String
  todo
end
end
"#;
    let out = compile_ok(src);
    assert!(out.contains("todo!()"), "expected todo!() macro:\n{}", out);
    // Must NOT appear as a bare identifier (would be a Rust compile error)
    let fn_body: String = out
        .lines()
        .skip_while(|l| !l.contains("pub fn stub"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !fn_body.contains("    todo\n"),
        "unexpected bare todo identifier in:\n{}", fn_body
    );
}
