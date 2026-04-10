// M108 tests — Mermaid diagram emission.
//
// Verifies that the four Mermaid compile targets produce correct diagram
// output from parsed Loom AST. Diagrams cannot drift from code because
// they ARE derived from the code (GS Diagram-emitting property).

use loom::{
    compile_mermaid_c4, compile_mermaid_flow, compile_mermaid_sequence, compile_mermaid_state,
};

// ── 1. C4 emits a valid Mermaid block ────────────────────────────────────────

#[test]
fn test_m108_c4_emits_mermaid_block() {
    let src = r#"
module Minimal
  fn hello :: String -> String
  end
end
"#;
    let result = compile_mermaid_c4(src);
    assert!(
        result.is_ok(),
        "compile_mermaid_c4 should succeed: {:?}",
        result.err()
    );
    let diagram = result.unwrap();
    assert!(
        diagram.contains("```mermaid"),
        "output must contain ```mermaid fenced block marker"
    );
    assert!(
        diagram.contains("C4Container"),
        "output must contain C4Container declaration"
    );
}

// ── 2. Sequence diagram from session type ────────────────────────────────────

#[test]
fn test_m108_sequence_from_session_type() {
    let src = r#"
module Shop
  session Purchase
    buyer:
      send: OrderRequest
      recv: Invoice
    end
    seller:
      recv: OrderRequest
      send: Invoice
    end
    duality: buyer <-> seller
  end
end
"#;
    let result = compile_mermaid_sequence(src);
    assert!(
        result.is_ok(),
        "compile_mermaid_sequence should succeed: {:?}",
        result.err()
    );
    let diagram = result.unwrap();
    assert!(
        diagram.contains("participant buyer"),
        "sequence diagram must list buyer as participant"
    );
    assert!(
        diagram.contains("participant seller"),
        "sequence diagram must list seller as participant"
    );
}

// ── 3. State diagram from lifecycle declaration ───────────────────────────────

#[test]
fn test_m108_state_from_lifecycle() {
    let src = r#"
module Connection
  lifecycle Conn :: Disconnected -> Connected -> Authenticated
end
"#;
    let result = compile_mermaid_state(src);
    assert!(
        result.is_ok(),
        "compile_mermaid_state should succeed: {:?}",
        result.err()
    );
    let diagram = result.unwrap();
    assert!(
        diagram.contains("stateDiagram-v2"),
        "output must contain stateDiagram-v2 declaration"
    );
    assert!(
        diagram.contains("Disconnected --> Connected"),
        "must emit first transition"
    );
    assert!(
        diagram.contains("Connected --> Authenticated"),
        "must emit second transition"
    );
}

// ── 4. Flow diagram from fn declarations ─────────────────────────────────────

#[test]
fn test_m108_flow_from_fns() {
    let src = r#"
module Pipeline
  fn validate :: String -> Bool
  end
  fn transform :: String -> String
  end
  fn persist :: String -> String
  end
end
"#;
    let result = compile_mermaid_flow(src);
    assert!(
        result.is_ok(),
        "compile_mermaid_flow should succeed: {:?}",
        result.err()
    );
    let diagram = result.unwrap();
    assert!(
        diagram.contains("flowchart TD"),
        "output must contain flowchart TD declaration"
    );
    assert!(
        diagram.contains("validate"),
        "must include validate fn node"
    );
    assert!(
        diagram.contains("transform"),
        "must include transform fn node"
    );
    assert!(diagram.contains("persist"), "must include persist fn node");
    assert!(diagram.contains("Start"), "must have Start node");
    assert!(diagram.contains("End"), "must have End node");
}

// ── 5. Empty program returns valid empty diagrams ────────────────────────────

#[test]
fn test_m108_empty_program_returns_empty_diagram() {
    let src = "module Empty end\n";
    let c4 = compile_mermaid_c4(src).expect("c4 empty module should succeed");
    let seq = compile_mermaid_sequence(src).expect("sequence empty module should succeed");
    let state = compile_mermaid_state(src).expect("state empty module should succeed");
    let flow = compile_mermaid_flow(src).expect("flow empty module should succeed");

    // All should produce valid (but minimal) Mermaid blocks
    assert!(
        c4.contains("```mermaid"),
        "empty c4 must be a valid mermaid block"
    );
    assert!(
        seq.contains("```mermaid"),
        "empty sequence must be a valid mermaid block"
    );
    assert!(
        state.contains("```mermaid"),
        "empty state must be a valid mermaid block"
    );
    assert!(
        flow.contains("```mermaid"),
        "empty flow must be a valid mermaid block"
    );
}

// ── 6. C4 includes being names ───────────────────────────────────────────────

#[test]
fn test_m108_c4_includes_being_names() {
    let src = r#"
module Eco
being PaymentProcessor
  telos: "process payments reliably"
  end
end
end
"#;
    let result = compile_mermaid_c4(src);
    assert!(
        result.is_ok(),
        "compile_mermaid_c4 with being should succeed: {:?}",
        result.err()
    );
    let diagram = result.unwrap();
    assert!(
        diagram.contains("PaymentProcessor"),
        "C4 diagram must include the being name 'PaymentProcessor'"
    );
}
