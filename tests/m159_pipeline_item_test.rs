/// M159 — `pipeline` item: parser + codegen tests.
///
/// `pipeline Name step a :: In -> Out ... end`
/// must parse into a `PipelineDef` and emit:
///  - `{Name}Pipeline` unit struct
///  - Per-step stub fn with `todo!()` body
///  - `process()` method chaining all steps in order
///  - `LOOM[pipeline:item]` audit comment

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M159.1: basic pipeline parses ───────────────────────────────────────────

#[test]
fn m159_simple_pipeline_parses() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
  step trim :: String -> String
end
end
"#;
    compile(src);
}

// ─── M159.2: emits struct ────────────────────────────────────────────────────

#[test]
fn m159_emits_pipeline_struct() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct DataCleanerPipeline"),
        "missing struct\n{out}"
    );
}

// ─── M159.3: emits step fn ───────────────────────────────────────────────────

#[test]
fn m159_emits_step_fn() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn normalize("),
        "missing normalize fn\n{out}"
    );
    assert!(out.contains("todo!("), "step body should be todo!\n{out}");
}

// ─── M159.4: step type mapping ───────────────────────────────────────────────

#[test]
fn m159_step_types_map_to_rust() {
    let src = r#"
module transform
pipeline Validator
  step check :: String -> Bool
  step score :: String -> Float
  step count :: String -> Int
end
end
"#;
    let out = compile(src);
    assert!(out.contains("-> bool"), "expected bool return\n{out}");
    assert!(out.contains("-> f64"), "expected f64 return\n{out}");
    assert!(out.contains("-> i64"), "expected i64 return\n{out}");
}

// ─── M159.5: emits process() method ──────────────────────────────────────────

#[test]
fn m159_emits_process_method() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
  step trim :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub fn process("), "missing process()\n{out}");
}

// ─── M159.6: audit comment ───────────────────────────────────────────────────

#[test]
fn m159_emits_audit_comment() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[pipeline:item]"),
        "missing audit comment\n{out}"
    );
}

// ─── M159.7: step audit comment ──────────────────────────────────────────────

#[test]
fn m159_step_has_audit_comment() {
    let src = r#"
module transform
pipeline DataCleaner
  step normalize :: String -> String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[pipeline:step]"),
        "missing step audit comment\n{out}"
    );
}

// ─── M159.8: multiple steps all emitted ──────────────────────────────────────

#[test]
fn m159_multiple_steps_all_emitted() {
    let src = r#"
module transform
pipeline Etl
  step extract :: String -> String
  step transform :: String -> String
  step load :: String -> Bool
end
end
"#;
    let out = compile(src);
    assert!(out.contains("fn extract("), "missing extract\n{out}");
    assert!(out.contains("fn transform("), "missing transform\n{out}");
    assert!(out.contains("fn load("), "missing load\n{out}");
}

// ─── M159.9: empty pipeline (no steps) parses ────────────────────────────────

#[test]
fn m159_empty_pipeline_parses() {
    let src = r#"
module transform
pipeline Empty
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct EmptyPipeline"),
        "missing struct for empty pipeline\n{out}"
    );
}

// ─── M159.10: pipeline mixed with const ──────────────────────────────────────

#[test]
fn m159_mixed_with_const() {
    let src = r#"
module service
const MaxSteps: Int = 3
pipeline Process
  step run :: String -> Bool
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub const MAX_STEPS"), "missing const\n{out}");
    assert!(
        out.contains("pub struct ProcessPipeline"),
        "missing pipeline\n{out}"
    );
}

// ─── M159.11: pipeline mixed with chain ──────────────────────────────────────

#[test]
fn m159_mixed_with_chain() {
    let src = r#"
module system
pipeline DataFlow
  step ingest :: String -> String
end
chain Lifecycle
  states: [Active, Inactive]
  transitions:
    Active -> Inactive: 0.1
    Inactive -> Active: 0.1
    Active -> Active: 0.9
    Inactive -> Inactive: 0.9
  end
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct DataFlowPipeline"),
        "missing pipeline\n{out}"
    );
    assert!(out.contains("LifecycleState"), "missing chain\n{out}");
}

// ─── M159.12: parse error on missing step keyword ────────────────────────────

#[test]
fn m159_missing_step_keyword_is_error() {
    let src = r#"
module transform
pipeline Bad
  normalize :: String -> String
end
end
"#;
    let result = compile_check(src);
    assert!(
        result.is_err(),
        "expected parse error for missing step keyword"
    );
}
