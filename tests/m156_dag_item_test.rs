/// M156 — Top-level `dag` item: parser + codegen tests.
///
/// `dag Name nodes: [A, B, C] edges: [A -> B, B -> C] end`
/// must parse into a `DagDef` and emit:
///  - `{Name}Node` enum with variants from `dag.nodes`
///  - `{Name}DagItem` struct with `new()`, `add_typed_edge()`, `successors()`, `topological_sort()`
///  - Pre-initialized edges in `new()` from the `dag` declaration
///  - LOOM[dag:item] audit comment
///  - M156 reference
///  - Kahn's algorithm in `topological_sort()`

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M156.1: basic dag parses without error ───────────────────────────────────

#[test]
fn m156_simple_dag_parses() {
    let src = r#"
module etl
dag Pipeline
  nodes: [Ingest, Transform, Load]
  edges: [Ingest -> Transform, Transform -> Load]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("PipelineDagItem"),
        "expected PipelineDagItem in output\n{out}"
    );
}

// ─── M156.2: Node enum emitted with correct variants ──────────────────────────

#[test]
fn m156_node_enum_emitted() {
    let src = r#"
module etl
dag Pipeline
  nodes: [Ingest, Transform, Validate, Load]
  edges: [Ingest -> Transform]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub enum PipelineNode"),
        "missing PipelineNode enum\n{out}"
    );
    assert!(out.contains("Ingest,"), "missing Ingest variant\n{out}");
    assert!(
        out.contains("Transform,"),
        "missing Transform variant\n{out}"
    );
    assert!(out.contains("Validate,"), "missing Validate variant\n{out}");
    assert!(out.contains("Load,"), "missing Load variant\n{out}");
}

// ─── M156.3: DagItem struct emitted ──────────────────────────────────────────

#[test]
fn m156_dag_item_struct_emitted() {
    let src = r#"
module workflow
dag Workflow
  nodes: [Start, Process, End]
  edges: [Start -> Process, Process -> End]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct WorkflowDagItem"),
        "missing struct\n{out}"
    );
}

// ─── M156.4: new() constructor emitted ───────────────────────────────────────

#[test]
fn m156_new_constructor_emitted() {
    let src = r#"
module workflow
dag Workflow
  nodes: [Start, End]
  edges: [Start -> End]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn new()"),
        "missing new() constructor\n{out}"
    );
}

// ─── M156.5: edges pre-initialized in new() ──────────────────────────────────

#[test]
fn m156_edges_pre_initialized() {
    let src = r#"
module workflow
dag Workflow
  nodes: [Start, Middle, End]
  edges: [Start -> Middle, Middle -> End]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("WorkflowNode::Start, WorkflowNode::Middle"),
        "missing Start->Middle edge in new()\n{out}"
    );
    assert!(
        out.contains("WorkflowNode::Middle, WorkflowNode::End"),
        "missing Middle->End edge in new()\n{out}"
    );
}

// ─── M156.6: add_typed_edge() method emitted ─────────────────────────────────

#[test]
fn m156_add_typed_edge_method_emitted() {
    let src = r#"
module workflow
dag Workflow
  nodes: [A, B]
  edges: [A -> B]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn add_typed_edge("),
        "missing add_typed_edge()\n{out}"
    );
}

// ─── M156.7: topological_sort() method emitted ───────────────────────────────

#[test]
fn m156_topological_sort_emitted() {
    let src = r#"
module workflow
dag Workflow
  nodes: [A, B, C]
  edges: [A -> B, B -> C]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn topological_sort("),
        "missing topological_sort()\n{out}"
    );
    assert!(
        out.contains("Kahn"),
        "missing Kahn reference in comment\n{out}"
    );
}

// ─── M156.8: successors() method emitted ─────────────────────────────────────

#[test]
fn m156_successors_method_emitted() {
    let src = r#"
module workflow
dag Workflow
  nodes: [A, B]
  edges: [A -> B]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub fn successors("),
        "missing successors()\n{out}"
    );
}

// ─── M156.9: LOOM audit comment and M156 reference ───────────────────────────

#[test]
fn m156_audit_comment_present() {
    let src = r#"
module workflow
dag Workflow
  nodes: [A, B]
  edges: [A -> B]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[dag:item]"),
        "missing LOOM audit comment\n{out}"
    );
    assert!(out.contains("M156"), "missing M156 reference\n{out}");
}

// ─── M156.10: multiple dag items in one module ────────────────────────────────

#[test]
fn m156_multiple_dags_in_module() {
    let src = r#"
module multi
dag Pipeline
  nodes: [A, B]
  edges: [A -> B]
end
dag Workflow
  nodes: [X, Y]
  edges: [X -> Y]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub enum PipelineNode"),
        "missing PipelineNode\n{out}"
    );
    assert!(
        out.contains("pub enum WorkflowNode"),
        "missing WorkflowNode\n{out}"
    );
    assert!(
        out.contains("pub struct PipelineDagItem"),
        "missing PipelineDagItem\n{out}"
    );
    assert!(
        out.contains("pub struct WorkflowDagItem"),
        "missing WorkflowDagItem\n{out}"
    );
}

// ─── M156.11: dag without edges parses (edges optional) ──────────────────────

#[test]
fn m156_dag_without_edges_parses() {
    let src = r#"
module standalone
dag Minimal
  nodes: [A, B, C]
end
end
"#;
    let result = compile_check(src);
    assert!(
        result.is_ok(),
        "dag without edges should compile: {:?}",
        result
    );
}

// ─── M156.12: dag without nodes or edges parses (both optional) ──────────────

#[test]
fn m156_empty_dag_parses() {
    let src = r#"
module standalone
dag Empty
end
end
"#;
    let result = compile_check(src);
    assert!(result.is_ok(), "empty dag should compile: {:?}", result);
}

// ─── M156.13: Node enum has derive macros ────────────────────────────────────

#[test]
fn m156_node_enum_has_derive_macros() {
    let src = r#"
module workflow
dag Workflow
  nodes: [A, B]
  edges: [A -> B]
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"),
        "missing derive macros on node enum\n{out}"
    );
}
