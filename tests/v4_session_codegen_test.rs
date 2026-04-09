// V4 Session Type Codegen Tests — emit_session_state_machine
//
// Verifies that session: declarations emit full typestate-enforced Rust.
// The key property: calling steps in the WRONG order is a compile-time TYPE ERROR.
//
// Test strategy:
//   1. Check emitted string structure (state structs, typed channel, transitions)
//   2. Check the transition methods have the correct typestate signature
//
// These tests verify the emitted code structure.
// The compile-time enforcement claim is proved by the fact that `send(self, ...)` and
// `recv(self)` consume `self` — a moved-out value cannot be used again (affine types).

use loom::compile;

fn emit(src: &str) -> String {
    compile(src).unwrap_or_else(|errs| panic!("compile failed: {:?}", errs))
}

// 1. A session emits per-role state marker structs (one per step + Done).
#[test]
fn v4_emits_state_marker_structs_per_step() {
    let src = r#"
module Auth
  session AuthProtocol
    client:
      send: String
      recv: Int
    end
    server:
      recv: String
      send: Int
    end
  end
end
"#;
    let rust = emit(src);
    // Client: Step0, Step1, Done
    assert!(
        rust.contains("AuthProtocolClientStep0"),
        "should emit step0 state for client; got:\n{rust}"
    );
    assert!(
        rust.contains("AuthProtocolClientStep1"),
        "should emit step1 state for client; got:\n{rust}"
    );
    assert!(
        rust.contains("AuthProtocolClientDone"),
        "should emit Done state for client; got:\n{rust}"
    );
}

// 2. Each role gets its own typed channel wrapper.
#[test]
fn v4_emits_typed_channel_wrapper_per_role() {
    let src = r#"
module Order
  session OrderProtocol
    buyer:
      send: String
      recv: Int
    end
    seller:
      recv: String
      send: Int
    end
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("OrderProtocolBuyerChannel"),
        "should emit typed channel for buyer; got:\n{rust}"
    );
    assert!(
        rust.contains("OrderProtocolSellerChannel"),
        "should emit typed channel for seller; got:\n{rust}"
    );
}

// 3. Channel is PhantomData-wrapped (zero-cost typestate).
#[test]
fn v4_channel_uses_phantom_data_for_zero_cost_typestate() {
    let src = r#"
module Proto
  session PingPong
    pinger:
      send: Int
      recv: Int
    end
    ponger:
      recv: Int
      send: Int
    end
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("PhantomData"),
        "typestate channel should use PhantomData<State>; got:\n{rust}"
    );
}

// 4. Send step emits a `send(self, msg: T)` that returns the next state.
#[test]
fn v4_send_step_emits_consuming_transition() {
    let src = r#"
module Transfer
  session TransferProtocol
    sender:
      send: Int
    end
    receiver:
      recv: Int
    end
  end
end
"#;
    let rust = emit(src);
    // send consumes self (no & self) and returns the next channel state
    assert!(
        rust.contains("pub fn send(self,"),
        "send step should emit `pub fn send(self, _msg: ...)` (consumes state); got:\n{rust}"
    );
}

// 5. Recv step emits a `recv(self)` returning (next_state, T).
#[test]
fn v4_recv_step_emits_consuming_transition_with_return() {
    let src = r#"
module Transfer
  session TransferProtocol
    sender:
      send: Int
    end
    receiver:
      recv: Int
    end
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("pub fn recv(self)"),
        "recv step should emit `pub fn recv(self)` (consumes state); got:\n{rust}"
    );
}

// 6. Constructor starts in Step0 state.
#[test]
fn v4_constructor_starts_in_step0() {
    let src = r#"
module Handshake
  session HandshakeProtocol
    initiator:
      send: String
      recv: String
    end
    responder:
      recv: String
      send: String
    end
  end
end
"#;
    let rust = emit(src);
    // The `new()` constructor impl block should be for Step0
    assert!(
        rust.contains("HandshakeProtocolInitiatorStep0"),
        "constructor should start in step0 state; got:\n{rust}"
    );
    assert!(
        rust.contains("pub fn new()"),
        "should emit constructor; got:\n{rust}"
    );
}

// 7. Multi-step session emits a chain of transition impls.
#[test]
fn v4_multi_step_session_emits_full_transition_chain() {
    let src = r#"
module Trade
  session TradeProtocol
    buyer:
      send: String
      recv: Int
      send: Int
    end
    seller:
      recv: String
      send: Int
      recv: Int
    end
  end
end
"#;
    let rust = emit(src);
    // Three steps → Step0, Step1, Step2, Done
    assert!(
        rust.contains("TradeProtocolBuyerStep0"),
        "should emit Step0 for 3-step session; got:\n{rust}"
    );
    assert!(
        rust.contains("TradeProtocolBuyerStep2"),
        "should emit Step2 for 3-step session; got:\n{rust}"
    );
    assert!(
        rust.contains("TradeProtocolBuyerDone"),
        "should emit Done state after last step; got:\n{rust}"
    );
}

// 8. Duality annotation emits a comment recording the dual roles.
#[test]
fn v4_duality_emits_comment() {
    let src = r#"
module Protocol
  session PingPong
    pinger:
      send: Int
      recv: Int
    end
    ponger:
      recv: Int
      send: Int
    end
    duality: pinger <-> ponger
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("duality") && rust.contains("pinger") && rust.contains("ponger"),
        "duality annotation should emit a comment; got:\n{rust}"
    );
}

// 9. V7 audit header reflects session count.
#[test]
fn v7_audit_header_reflects_session_count() {
    let src = r#"
module WithSession
  session MyProtocol
    role_a:
      send: Int
    end
    role_b:
      recv: Int
    end
  end
end
"#;
    let rust = emit(src);
    assert!(
        rust.contains("Sessions"),
        "audit header should show Sessions count; got:\n{rust}"
    );
    assert!(
        rust.contains("typestate"),
        "audit header should mention typestate enforcement; got:\n{rust}"
    );
}
