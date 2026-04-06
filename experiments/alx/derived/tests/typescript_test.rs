//! M17 tests — TypeScript emission target.
//!
//! Verifies that the TypeScript emitter correctly translates Loom AST constructs
//! to valid TypeScript: types, functions, enums, interfaces, imports, async effects,
//! contracts, annotations, and E2E compilation via tsc (if available).

use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::codegen::typescript::TypeScriptEmitter;

fn compile_ts(src: &str) -> String {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    TypeScriptEmitter::new().emit(&module)
}

// ── Namespace wrapper ─────────────────────────────────────────────────────────

#[test]
fn module_emits_namespace() {
    let src = r#"module Payments
fn calc :: Int -> Int
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("export namespace Payments {"), "missing namespace:\n{out}");
}

// ── Type mappings ─────────────────────────────────────────────────────────────

#[test]
fn int_maps_to_number() {
    let src = r#"module M
fn f :: Int -> Int
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("number"), "Int should map to number:\n{out}");
}

#[test]
fn bool_maps_to_boolean() {
    let src = r#"module M
fn f :: Bool -> Bool
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("boolean"), "Bool should map to boolean:\n{out}");
}

#[test]
fn string_maps_to_string() {
    let src = r#"module M
fn f :: String -> String
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains(": string"), "String should map to string:\n{out}");
}

// ── Type definitions ───────────────────────────────────────────────────────────

#[test]
fn type_def_emits_interface() {
    let src = r#"module M
type Point =
  x: Float
  y: Float
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("export interface Point {"), "expected interface:\n{out}");
    assert!(out.contains("x: number;"), "expected x field:\n{out}");
    assert!(out.contains("y: number;"), "expected y field:\n{out}");
}

// ── Enum definitions ───────────────────────────────────────────────────────────

#[test]
fn enum_emits_union_type() {
    let src = r#"module M
enum Color =
  | Red
  | Green
  | Blue
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("export type Color ="), "expected type union:\n{out}");
    assert!(out.contains("\"Red\""), "expected Red variant:\n{out}");
    assert!(out.contains("\"Green\""), "expected Green variant:\n{out}");
}

#[test]
fn enum_with_payload_emits_tagged_union() {
    let src = r#"module M
enum Shape =
  | Circle of Float
  | Rect
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("tag: \"Circle\""), "expected tagged union:\n{out}");
    assert!(out.contains("value: number"), "expected payload type:\n{out}");
}

// ── Function definitions ───────────────────────────────────────────────────────

#[test]
fn fn_emits_export_function() {
    let src = r#"module M
fn add :: Int -> Int -> Int
  x + y
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("export function add("), "expected export function:\n{out}");
    assert!(out.contains("): number {"), "expected return type:\n{out}");
    assert!(out.contains("return"), "expected return statement:\n{out}");
}

#[test]
fn effectful_fn_emits_async_promise() {
    let src = r#"module M
fn fetch :: String -> Effect<[IO], String>
  url
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("async function fetch("), "expected async:\n{out}");
    assert!(out.contains("Promise<string>"), "expected Promise return:\n{out}");
}

// ── Contracts ─────────────────────────────────────────────────────────────────

#[test]
fn require_emits_throw_on_violation() {
    let src = r#"module M
fn div :: Int -> Int -> Int
  require: y > 0
  x / y
end
end"#;
    let out = compile_ts(src);
    assert!(
        out.contains("if (!(") && out.contains("throw new Error"),
        "expected precondition throw:\n{out}"
    );
}

#[test]
fn ensure_emits_postcondition_check() {
    let src = r#"module M
fn abs :: Int -> Int
  ensure: result >= 0
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("_loomResult"), "expected result capture:\n{out}");
    assert!(out.contains("throw new Error"), "expected postcondition throw:\n{out}");
}

// ── Annotations → JSDoc ───────────────────────────────────────────────────────

#[test]
fn annotations_emit_jsdoc() {
    let src = r#"module M
@deprecated("use g instead")
fn f :: Int -> Int
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("@deprecated"), "expected @deprecated in output:\n{out}");
}

#[test]
fn describe_emits_jsdoc_comment() {
    let src = r#"module M
describe: "Utility module"
fn f :: Int -> Int
  x
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("Utility module"), "expected describe text in JSDoc:\n{out}");
}

// ── Import declarations ────────────────────────────────────────────────────────

#[test]
fn import_emits_es_import() {
    let src = r#"module App
import MathLib
fn run :: Int
  1
end
end"#;
    let out = compile_ts(src);
    assert!(
        out.contains("import * as MathLib from \"./math-lib\""),
        "expected ES import:\n{out}"
    );
}

// ── Interface definitions ──────────────────────────────────────────────────────

#[test]
fn interface_emits_ts_interface() {
    let src = r#"module M
interface Greeter
  fn greet :: String -> String
end
fn greet :: String -> String
  name
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains("export interface Greeter {"), "expected TS interface:\n{out}");
    assert!(out.contains("greet("), "expected greet method signature:\n{out}");
}

#[test]
fn implements_emits_class() {
    let src = r#"module M
interface Greeter
  fn greet :: String -> String
end
implements Greeter
fn greet :: String -> String
  name
end
end"#;
    let out = compile_ts(src);
    assert!(
        out.contains("export class MImpl implements Greeter {"),
        "expected class implements:\n{out}"
    );
}

// ── HOF sugar ─────────────────────────────────────────────────────────────────

#[test]
fn map_call_emits_array_map() {
    let src = r#"module M
fn doubles :: Int -> Int
  map(xs, |x: Int| x * 2)
end
end"#;
    let out = compile_ts(src);
    assert!(out.contains(".map("), "expected .map() call:\n{out}");
}

// ── E2E: tsc compilation (skips gracefully if tsc unavailable) ────────────────

#[test]
fn e2e_typescript_output_compiles_with_tsc() {
    let src = r#"module Calc
fn add :: Int -> Int -> Int
  x + y
end
fn mul :: Int -> Int -> Int
  x * y
end
end"#;
    let ts_src = compile_ts(src);

    let tmp_dir = std::env::temp_dir();
    let ts_path = tmp_dir.join("loom_m17_e2e.ts");
    std::fs::write(&ts_path, &ts_src).unwrap();

    let status = std::process::Command::new("tsc")
        .args(["--noEmit", "--strict", "--target", "ES2020"])
        .arg(&ts_path)
        .status();

    match status {
        Ok(s) => assert!(s.success(), "tsc failed on:\n{ts_src}"),
        Err(_) => eprintln!("tsc not available — skipping E2E TS compile check"),
    }

    let _ = std::fs::remove_file(&ts_path);
}
