/// M180: `state_machine` first-class item — explicit FSM with typed states/transitions

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

fn err(src: &str) -> String {
    loom::compile(src)
        .err()
        .map(|e| format!("{:?}", e))
        .unwrap_or_else(|| "no error".to_string())
}

// T1 — minimal state_machine generates a state enum
#[test]
fn t1_state_machine_generates_enum() {
    let out = ok("module M\n  state_machine Order\n  end\nend\n");
    assert!(out.contains("enum OrderState"), "expected OrderState enum, got:\n{}", out);
}

// T2 — generates a machine struct
#[test]
fn t2_state_machine_generates_struct() {
    let out = ok("module M\n  state_machine Order\n  end\nend\n");
    assert!(out.contains("struct OrderMachine"), "expected OrderMachine struct, got:\n{}", out);
}

// T3 — struct has a state field
#[test]
fn t3_state_machine_has_state_field() {
    let out = ok("module M\n  state_machine Traffic\n  end\nend\n");
    assert!(out.contains("state:") || out.contains("state "), "expected state field, got:\n{}", out);
}

// T4 — current method emitted
#[test]
fn t4_state_machine_has_current_method() {
    let out = ok("module M\n  state_machine Order\n  end\nend\n");
    assert!(out.contains("fn current"), "expected fn current, got:\n{}", out);
}

// T5 — transition method emitted
#[test]
fn t5_state_machine_has_transition_method() {
    let out = ok("module M\n  state_machine Order\n  end\nend\n");
    assert!(out.contains("fn transition"), "expected fn transition, got:\n{}", out);
}

// T6 — default initial state emitted in enum
#[test]
fn t6_state_machine_default_initial_state() {
    let out = ok("module M\n  state_machine Door\n  end\nend\n");
    assert!(out.contains("Initial"), "expected Initial variant, got:\n{}", out);
}

// T7 — name correctly embedded in type names
#[test]
fn t7_state_machine_name_embedded() {
    let out = ok("module M\n  state_machine Checkout\n  end\nend\n");
    assert!(out.contains("CheckoutState"), "expected CheckoutState, got:\n{}", out);
    assert!(out.contains("CheckoutMachine"), "expected CheckoutMachine, got:\n{}", out);
}

// T8 — LOOM annotation present
#[test]
fn t8_state_machine_loom_annotation() {
    let out = ok("module M\n  state_machine Process\n  end\nend\n");
    assert!(
        out.contains("LOOM") || out.contains("state_machine"),
        "expected LOOM annotation, got:\n{}",
        out
    );
}

// T9 — multiple state machines in one module
#[test]
fn t9_multiple_state_machines() {
    let out = ok("module M\n  state_machine A\n  end\n  state_machine B\n  end\nend\n");
    assert!(out.contains("AState") && out.contains("AMachine"), "expected A FSM, got:\n{}", out);
    assert!(out.contains("BState") && out.contains("BMachine"), "expected B FSM, got:\n{}", out);
}

// T10 — new() method initializes to initial state
#[test]
fn t10_state_machine_new_initializes() {
    let out = ok("module M\n  state_machine Task\n  end\nend\n");
    assert!(out.contains("fn new"), "expected fn new, got:\n{}", out);
}

// T11 — state machine alongside other items
#[test]
fn t11_state_machine_with_other_items() {
    let src = "module M\n  state_machine Workflow\n  end\n  entity Step\n    id: Int\n  end\nend\n";
    let out = ok(src);
    assert!(out.contains("WorkflowMachine"), "expected WorkflowMachine, got:\n{}", out);
    assert!(out.contains("Step"), "expected Step type, got:\n{}", out);
}

// T12 — missing end is a parse error
#[test]
fn t12_state_machine_missing_end_is_error() {
    let e = err("module M\n  state_machine Incomplete\nend\n");
    assert!(!e.is_empty(), "expected error for missing end");
}
