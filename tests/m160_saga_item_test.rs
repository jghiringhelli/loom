/// M160 — `saga` item: parser + codegen tests.
///
/// `saga Name step a :: In -> Out [compensate a :: In -> Out] ... end`
/// must emit:
///  - `{Name}Saga` unit struct
///  - `{Name}SagaError` enum with one variant per step
///  - Per-step stub fn returning `Result<Out, {Name}SagaError>`
///  - Per-compensate `{step}_compensate` stub fn
///  - `execute()` chaining all steps
///  - `LOOM[saga:item]` / `LOOM[saga:step]` / `LOOM[saga:compensate]` audit comments

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M160.1: basic saga parses ────────────────────────────────────────────────

#[test]
fn m160_simple_saga_parses() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
  step charge :: String -> String
end
end
"#;
    compile(src);
}

// ─── M160.2: emits struct ────────────────────────────────────────────────────

#[test]
fn m160_emits_saga_struct() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct OrderSaga"),
        "missing struct\n{out}"
    );
}

// ─── M160.3: emits error enum ────────────────────────────────────────────────

#[test]
fn m160_emits_error_enum() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
  step charge :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub enum OrderSagaError"),
        "missing error enum\n{out}"
    );
    assert!(
        out.contains("ReserveFailed"),
        "missing ReserveFailed variant\n{out}"
    );
    assert!(
        out.contains("ChargeFailed"),
        "missing ChargeFailed variant\n{out}"
    );
}

// ─── M160.4: emits step fn returning Result ───────────────────────────────────

#[test]
fn m160_step_returns_result() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("fn reserve("), "missing reserve fn\n{out}");
    assert!(out.contains("Result<"), "step should return Result\n{out}");
    assert!(
        out.contains("OrderSagaError"),
        "step should use error enum\n{out}"
    );
}

// ─── M160.5: compensate emits _compensate fn ─────────────────────────────────

#[test]
fn m160_compensate_emits_fn() {
    let src = r#"
module order
saga OrderSaga
  step charge :: String -> String
  compensate charge :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("fn charge_compensate("),
        "missing charge_compensate fn\n{out}"
    );
}

// ─── M160.6: audit comments ──────────────────────────────────────────────────

#[test]
fn m160_emits_audit_comments() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
  step charge :: String -> String
  compensate charge :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("LOOM[saga:item]"), "missing saga audit\n{out}");
    assert!(out.contains("LOOM[saga:step]"), "missing step audit\n{out}");
    assert!(
        out.contains("LOOM[saga:compensate]"),
        "missing compensate audit\n{out}"
    );
}

// ─── M160.7: emits execute() method ──────────────────────────────────────────

#[test]
fn m160_emits_execute_method() {
    let src = r#"
module order
saga OrderSaga
  step reserve :: String -> String
  step charge :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn execute("), "missing execute()\n{out}");
}

// ─── M160.8: multi-step saga ─────────────────────────────────────────────────

#[test]
fn m160_multi_step_all_emitted() {
    let src = r#"
module order
saga EcommerceSaga
  step reserve :: String -> String
  step charge :: String -> String
  compensate charge :: String -> String
  step fulfill :: String -> String
  compensate fulfill :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("fn reserve("), "missing reserve\n{out}");
    assert!(out.contains("fn charge("), "missing charge\n{out}");
    assert!(
        out.contains("fn charge_compensate("),
        "missing charge_compensate\n{out}"
    );
    assert!(out.contains("fn fulfill("), "missing fulfill\n{out}");
    assert!(
        out.contains("fn fulfill_compensate("),
        "missing fulfill_compensate\n{out}"
    );
}

// ─── M160.9: type mapping in step params ──────────────────────────────────────

#[test]
fn m160_type_mapping() {
    let src = r#"
module service
saga TypeSaga
  step parse :: String -> Int
  step validate :: Int -> Bool
end
end
"#;
    let out = compile(src);
    assert!(out.contains("i64"), "expected i64 (Int)\n{out}");
    assert!(out.contains("bool"), "expected bool (Bool)\n{out}");
}

// ─── M160.10: mixed with pipeline ─────────────────────────────────────────────

#[test]
fn m160_mixed_with_pipeline() {
    let src = r#"
module service
pipeline PreProcess
  step clean :: String -> String
end
saga ProcessSaga
  step run :: String -> Bool
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct PreProcessPipeline"),
        "missing pipeline\n{out}"
    );
    assert!(
        out.contains("pub struct ProcessSaga"),
        "missing saga\n{out}"
    );
}

// ─── M160.11: empty saga parses ───────────────────────────────────────────────

#[test]
fn m160_empty_saga_parses() {
    let src = r#"
module service
saga EmptySaga
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct EmptySaga"),
        "missing empty saga struct\n{out}"
    );
}

// ─── M160.12: saga + chain + dag mix ──────────────────────────────────────────

#[test]
fn m160_mixed_with_chain_and_dag() {
    let src = r#"
module system
saga DeploySaga
  step build :: String -> String
  step verify :: String -> Bool
end
dag BuildGraph
  nodes: [Compile, Link, Ship]
  edges: [Compile -> Link, Link -> Ship]
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub struct DeploySaga"), "missing saga\n{out}");
    assert!(out.contains("BuildGraphNode"), "missing dag\n{out}");
}
