/// M157 — Top-level `const` item: parser + codegen tests.
///
/// `const Name: Type = value` must parse into a `ConstDef` and emit:
///  - `pub const UPPER_SNAKE: RustType = value;`
///  - LOOM[const:item] audit comment
///  - M157 reference

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M157.1: integer const parses and emits ──────────────────────────────────

#[test]
fn m157_int_const_emits() {
    let src = r#"
module config
const MaxRetries: Int = 3
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const MAX_RETRIES: i64 = 3"), "missing MAX_RETRIES\n{out}");
}

// ─── M157.2: float const emits ───────────────────────────────────────────────

#[test]
fn m157_float_const_emits() {
    let src = r#"
module config
const Timeout: Float = 30.5
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const TIMEOUT: f64 = 30.5"), "missing TIMEOUT\n{out}");
}

// ─── M157.3: bool const emits ────────────────────────────────────────────────

#[test]
fn m157_bool_const_emits() {
    let src = r#"
module config
const DebugMode: Bool = false
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const DEBUG_MODE: bool = false"), "missing DEBUG_MODE\n{out}");
}

// ─── M157.4: string const emits ──────────────────────────────────────────────

#[test]
fn m157_string_const_emits() {
    let src = r#"
module config
const ServiceName: String = "api-gateway"
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const SERVICE_NAME: &str"), "missing SERVICE_NAME\n{out}");
    assert!(out.contains("api-gateway"), "missing string value\n{out}");
}

// ─── M157.5: PascalCase name → UPPER_SNAKE_CASE ──────────────────────────────

#[test]
fn m157_pascal_case_to_upper_snake() {
    let src = r#"
module config
const MyConfigValue: Int = 42
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const MY_CONFIG_VALUE: i64 = 42"), "wrong snake case\n{out}");
}

// ─── M157.6: multiple consts in one module ────────────────────────────────────

#[test]
fn m157_multiple_consts() {
    let src = r#"
module config
const MaxRetries: Int = 3
const Timeout: Float = 30.0
const ServiceName: String = "api"
end
"#;
    let out = compile(src);
    assert!(out.contains("MAX_RETRIES"), "missing MAX_RETRIES\n{out}");
    assert!(out.contains("TIMEOUT"), "missing TIMEOUT\n{out}");
    assert!(out.contains("SERVICE_NAME"), "missing SERVICE_NAME\n{out}");
}

// ─── M157.7: LOOM audit comment present ──────────────────────────────────────

#[test]
fn m157_audit_comment_present() {
    let src = r#"
module config
const MaxRetries: Int = 3
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[const:item]"), "missing LOOM audit comment\n{out}");
    assert!(out.contains("M157"), "missing M157 reference\n{out}");
}

// ─── M157.8: const mixed with other items ────────────────────────────────────

#[test]
fn m157_const_mixed_with_store() {
    let src = r#"
module service
const MaxConnections: Int = 100
store Users :: Relational
  table User
    id: Int @primary_key
    name: String
  end
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const MAX_CONNECTIONS"), "missing const\n{out}");
    assert!(out.contains("struct User"), "missing store struct\n{out}");
}

// ─── M157.9: const type inference from int value ─────────────────────────────

#[test]
fn m157_type_inference_int() {
    let src = r#"
module config
const Version: Int = 2
end
"#;
    let result = compile_check(src);
    assert!(result.is_ok(), "const with Int type should compile: {:?}", result);
}

// ─── M157.10: const type inference from float value ──────────────────────────

#[test]
fn m157_type_inference_float() {
    let src = r#"
module config
const Pi: Float = 3.14159
end
"#;
    let out = compile(src);
    assert!(out.contains("PI"), "missing PI const\n{out}");
    assert!(out.contains("f64"), "missing f64 type\n{out}");
}

// ─── M157.11: single-word name stays uppercase ───────────────────────────────

#[test]
fn m157_single_word_name_uppercase() {
    let src = r#"
module config
const Port: Int = 8080
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const PORT: i64 = 8080"), "missing PORT\n{out}");
}

// ─── M157.12: const mixed with dag and chain ─────────────────────────────────

#[test]
fn m157_const_with_chain_and_dag() {
    let src = r#"
module system
const MaxNodes: Int = 16
chain Traffic
  states: [Low, Medium, High]
  transitions: Low -> Medium: 0.3 Medium -> High: 0.2
end
dag Pipeline
  nodes: [Ingest, Process, Emit]
  edges: [Ingest -> Process, Process -> Emit]
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const MAX_NODES"), "missing MAX_NODES\n{out}");
    assert!(out.contains("TrafficTransitionMatrix"), "missing chain\n{out}");
    assert!(out.contains("PipelineDagItem"), "missing dag\n{out}");
}
