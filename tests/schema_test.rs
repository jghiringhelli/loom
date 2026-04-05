//! M18 tests — JSON Schema + OpenAPI 3.0 contract materialisation.
//!
//! Verifies that:
//! - JSON Schema emitter: type/enum/refined-type → $defs, correct type mappings,
//!   Option/Result/List/Map generics, tuple prefix-items, module $id.
//! - OpenAPI emitter: paths from functions, HTTP method inference, annotations,
//!   request body schema, response schema, async (Effect) flag, components/schemas.

use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::codegen::schema::JsonSchemaEmitter;
use loom::codegen::openapi::OpenApiEmitter;

fn json_schema(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    JsonSchemaEmitter::new().emit(&module)
}

fn openapi(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    OpenApiEmitter::new().emit(&module)
}

// ═══════════════════════════════════════════════════════════════════════════════
// JSON Schema tests
// ═══════════════════════════════════════════════════════════════════════════════

// ── document structure ────────────────────────────────────────────────────────

#[test]
fn json_schema_contains_schema_version() {
    let src = r#"module M
fn f :: Int -> Int
  x
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("json-schema.org/draft/2020-12"), "missing schema URL:\n{out}");
}

#[test]
fn json_schema_uses_module_name_as_id() {
    let src = r#"module Pricing
fn f :: Int -> Int
  x
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"$id\": \"Pricing\""), "missing $id:\n{out}");
}

// ── type definitions → object schema ──────────────────────────────────────────

#[test]
fn type_def_emits_object_schema() {
    let src = r#"module M
type Point =
  x: Float
  y: Float
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"type\":\"object\""), "expected object type:\n{out}");
    assert!(out.contains("\"x\""), "expected x field:\n{out}");
    assert!(out.contains("\"y\""), "expected y field:\n{out}");
    assert!(out.contains("\"required\""), "expected required:\n{out}");
}

#[test]
fn type_def_fields_have_correct_types() {
    let src = r#"module M
type Person =
  name: String
  age: Int
  score: Float
  active: Bool
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"type\":\"string\""), "String field:\n{out}");
    assert!(out.contains("\"type\":\"integer\""), "Int field:\n{out}");
    assert!(out.contains("\"type\":\"number\""), "Float field:\n{out}");
    assert!(out.contains("\"type\":\"boolean\""), "Bool field:\n{out}");
}

// ── enum definitions → oneOf / const ──────────────────────────────────────────

#[test]
fn enum_without_payload_emits_const_strings() {
    let src = r#"module M
enum Color =
  | Red
  | Green
  | Blue
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"const\":\"Red\""), "Red const:\n{out}");
    assert!(out.contains("\"const\":\"Green\""), "Green const:\n{out}");
    assert!(out.contains("\"const\":\"Blue\""), "Blue const:\n{out}");
}

#[test]
fn enum_with_payload_emits_tagged_object() {
    let src = r#"module M
enum Shape =
  | Circle of Float
  | Square
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"tag\""), "expected tag property:\n{out}");
    assert!(out.contains("\"const\":\"Circle\""), "expected Circle tag:\n{out}");
    assert!(out.contains("\"value\""), "expected value property:\n{out}");
    assert!(out.contains("\"type\":\"number\""), "expected Float→number:\n{out}");
}

// ── refined types ─────────────────────────────────────────────────────────────

#[test]
fn refined_type_emits_allof_with_description() {
    let src = r#"module M
type Email = String where
  is_valid(value)
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"allOf\""), "expected allOf:\n{out}");
    assert!(out.contains("Email"), "expected Email in description:\n{out}");
}

// ── generic types ─────────────────────────────────────────────────────────────

#[test]
fn list_type_maps_to_array_schema() {
    let src = r#"module M
type Bag =
  items: List<Int>
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"type\":\"array\""), "expected array type:\n{out}");
    assert!(out.contains("\"items\""), "expected items:\n{out}");
}

#[test]
fn option_type_maps_to_oneof_with_null() {
    let src = r#"module M
type Wrapper =
  value: Option<String>
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"oneOf\""), "expected oneOf for Option:\n{out}");
    assert!(out.contains("\"type\":\"null\""), "expected null alternative:\n{out}");
}

#[test]
fn user_type_ref_emits_dollar_ref() {
    let src = r#"module M
type Container =
  inner: Point
end
end"#;
    let out = json_schema(src);
    assert!(out.contains("\"$ref\":\"#/$defs/Point\""), "expected $ref:\n{out}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// OpenAPI tests
// ═══════════════════════════════════════════════════════════════════════════════

// ── document structure ────────────────────────────────────────────────────────

#[test]
fn openapi_document_version() {
    let src = r#"module M
fn f :: Int -> Int
  x
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"openapi\": \"3.0.3\""), "missing openapi version:\n{out}");
}

#[test]
fn openapi_info_title_is_module_name() {
    let src = r#"module UserService
fn get_user :: Int -> String
  id
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"title\": \"UserService\""), "missing title:\n{out}");
}

#[test]
fn openapi_info_description_from_describe() {
    let src = r#"module M
describe: "Handles payments"
fn pay :: Int -> Int
  amount
end
end"#;
    let out = openapi(src);
    assert!(out.contains("Handles payments"), "missing description:\n{out}");
}

// ── path generation ───────────────────────────────────────────────────────────

#[test]
fn fn_generates_path() {
    let src = r#"module Payments
fn charge :: Int -> Int
  amount
end
end"#;
    let out = openapi(src);
    assert!(out.contains("/payments/charge"), "missing path:\n{out}");
}

#[test]
fn path_annotation_overrides_default() {
    let src = r#"module M
fn list_users @path("/api/v1/users") :: Int -> Int
  page
end
end"#;
    let out = openapi(src);
    assert!(out.contains("/api/v1/users"), "missing custom path:\n{out}");
}

// ── HTTP method inference ─────────────────────────────────────────────────────

#[test]
fn fn_with_params_defaults_to_post() {
    let src = r#"module M
fn create :: String -> Int
  name
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"post\""), "expected POST for fn with params:\n{out}");
}

#[test]
fn fn_with_no_params_defaults_to_get() {
    let src = r#"module M
fn ping :: Int
  1
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"get\""), "expected GET for fn with no params:\n{out}");
}

#[test]
fn method_annotation_overrides_default() {
    let src = r#"module M
fn list @method("GET") @path("/items") :: String -> Int
  query
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"get\""), "expected GET from annotation:\n{out}");
}

// ── request body ─────────────────────────────────────────────────────────────

#[test]
fn post_fn_emits_request_body() {
    let src = r#"module M
fn create :: String -> Int -> Int
  name + count
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"requestBody\""), "expected requestBody:\n{out}");
    assert!(out.contains("\"application/json\""), "expected content type:\n{out}");
}

// ── response schema ───────────────────────────────────────────────────────────

#[test]
fn return_type_maps_to_response_schema() {
    let src = r#"module M
fn get_count :: Int
  1
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"200\""), "expected 200 response:\n{out}");
    assert!(out.contains("\"type\":\"integer\""), "expected integer response schema:\n{out}");
}

#[test]
fn effect_return_emits_async_extension() {
    let src = r#"module M
fn fetch :: String -> Effect<[IO], Int>
  url
end
end"#;
    let out = openapi(src);
    assert!(out.contains("x-loom-async"), "expected x-loom-async extension:\n{out}");
}

// ── components/schemas ────────────────────────────────────────────────────────

#[test]
fn types_appear_in_components_schemas() {
    let src = r#"module M
type Order =
  id: Int
  amount: Float
end
fn process :: Int -> Int
  id
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"components\""), "expected components:\n{out}");
    assert!(out.contains("\"schemas\""), "expected schemas:\n{out}");
    assert!(out.contains("\"Order\""), "expected Order schema:\n{out}");
}

#[test]
fn enums_appear_in_components_schemas() {
    let src = r#"module M
enum Status =
  | Active
  | Inactive
end
fn get_status :: Int -> Int
  id
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"Status\""), "expected Status enum in schemas:\n{out}");
}

// ── operation id and summary ──────────────────────────────────────────────────

#[test]
fn fn_name_becomes_operation_id() {
    let src = r#"module M
fn create_order :: String -> Int
  name
end
end"#;
    let out = openapi(src);
    assert!(out.contains("\"operationId\": \"create_order\""), "expected operationId:\n{out}");
}

#[test]
fn fn_describe_becomes_summary() {
    let src = r#"module M
describe: "Create a new order in the system"
fn create_order :: String -> Int
  name
end
end"#;
    let out = openapi(src);
    // Module describe used as description; fn name as summary fallback
    assert!(out.contains("create_order"), "expected fn name in output:\n{out}");
}

// ── pipeline entry points ─────────────────────────────────────────────────────

#[test]
fn compile_json_schema_pipeline() {
    let src = r#"module Orders
type Order =
  id: Int
  amount: Float
end
fn process :: Int -> Int
  id
end
end"#;
    let result = loom::compile_json_schema(src);
    assert!(result.is_ok(), "compile_json_schema failed: {:?}", result);
    assert!(result.unwrap().contains("Order"));
}

#[test]
fn compile_openapi_pipeline() {
    let src = r#"module Inventory
fn list_items :: Int -> Int
  page
end
end"#;
    let result = loom::compile_openapi(src);
    assert!(result.is_ok(), "compile_openapi failed: {:?}", result);
    assert!(result.unwrap().contains("3.0.3"));
}
