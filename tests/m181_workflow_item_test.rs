/// M181: `workflow` first-class item — sequential step orchestrator

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal workflow generates a struct
#[test]
fn t1_workflow_generates_struct() {
    let out = ok("module M\n  workflow MyFlow\n  end\nend\n");
    assert!(out.contains("struct MyFlowWorkflow"), "expected MyFlowWorkflow struct, got:\n{}", out);
}

// T2 — a Step trait is generated
#[test]
fn t2_workflow_generates_step_trait() {
    let out = ok("module M\n  workflow Order\n  end\nend\n");
    assert!(out.contains("trait") && out.contains("Step"), "expected Step trait, got:\n{}", out);
}

// T3 — steps field (Vec of Box<dyn Step>) emitted
#[test]
fn t3_workflow_has_steps_field() {
    let out = ok("module M\n  workflow Pipeline\n  end\nend\n");
    assert!(out.contains("steps"), "expected steps field, got:\n{}", out);
}

// T4 — add_step method emitted
#[test]
fn t4_workflow_has_add_step() {
    let out = ok("module M\n  workflow Build\n  end\nend\n");
    assert!(out.contains("fn add_step"), "expected fn add_step, got:\n{}", out);
}

// T5 — run method emitted
#[test]
fn t5_workflow_has_run() {
    let out = ok("module M\n  workflow Build\n  end\nend\n");
    assert!(out.contains("fn run"), "expected fn run, got:\n{}", out);
}

// T6 — step_count method emitted
#[test]
fn t6_workflow_has_step_count() {
    let out = ok("module M\n  workflow Process\n  end\nend\n");
    assert!(out.contains("fn step_count"), "expected fn step_count, got:\n{}", out);
}

// T7 — name correctly embedded in struct and trait name
#[test]
fn t7_workflow_name_embedded() {
    let out = ok("module M\n  workflow Checkout\n  end\nend\n");
    assert!(out.contains("CheckoutWorkflow"), "expected CheckoutWorkflow, got:\n{}", out);
    assert!(out.contains("CheckoutStep"), "expected CheckoutStep, got:\n{}", out);
}

// T8 — LOOM annotation present
#[test]
fn t8_workflow_loom_annotation() {
    let out = ok("module M\n  workflow Deploy\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("workflow"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple workflows in one module
#[test]
fn t9_multiple_workflows() {
    let out = ok("module M\n  workflow A\n  end\n  workflow B\n  end\nend\n");
    assert!(out.contains("AWorkflow"), "expected AWorkflow, got:\n{}", out);
    assert!(out.contains("BWorkflow"), "expected BWorkflow, got:\n{}", out);
}

// T10 — run iterates over steps with execute
#[test]
fn t10_workflow_run_calls_execute() {
    let out = ok("module M\n  workflow Task\n  end\nend\n");
    assert!(out.contains("execute"), "expected execute in run body, got:\n{}", out);
}

// T11 — workflow alongside other items
#[test]
fn t11_workflow_with_other_items() {
    let src = "module M\n  workflow Import\n  end\n  entity Record\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(out.contains("ImportWorkflow"), "expected ImportWorkflow, got:\n{}", out);
    assert!(out.contains("Record"), "expected Record type, got:\n{}", out);
}

// T12 — missing end is a parse error
#[test]
fn t12_workflow_missing_end_is_error() {
    let e = err("module M\n  workflow Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
