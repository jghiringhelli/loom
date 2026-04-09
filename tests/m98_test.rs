// M98: Session Types — Honda (1993).
// Tests for session type parsing and duality checking.

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// 1. Simple two-role session parses.
#[test]
fn test_m98_simple_two_role_session_parses() {
    let src = r#"
module Auth
  session AuthProtocol
    client:
      send: Credentials
      recv: Token
    end
    server:
      recv: Credentials
      send: Token
    end
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "simple two-role session should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let session = module.items.iter().find_map(|i| {
        if let Item::Session(s) = i {
            Some(s)
        } else {
            None
        }
    });
    assert!(session.is_some(), "should have a Session item");
    let s = session.unwrap();
    assert_eq!(s.name, "AuthProtocol");
    assert_eq!(s.roles.len(), 2);
    assert_eq!(s.roles[0].name, "client");
    assert_eq!(s.roles[1].name, "server");
}

// 2. Multi-step session (4 steps per role) parses.
#[test]
fn test_m98_multi_step_session_parses() {
    let src = r#"
module Order
  session OrderProtocol
    buyer:
      send: OrderRequest
      recv: PriceQuote
      send: PaymentDetails
      recv: OrderConfirmation
    end
    seller:
      recv: OrderRequest
      send: PriceQuote
      recv: PaymentDetails
      send: OrderConfirmation
    end
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "multi-step session should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let session = module.items.iter().find_map(|i| {
        if let Item::Session(s) = i {
            Some(s)
        } else {
            None
        }
    });
    assert!(session.is_some());
    let s = session.unwrap();
    assert_eq!(s.roles[0].steps.len(), 4);
    assert_eq!(s.roles[1].steps.len(), 4);
}

// 3. Session with duality declaration parses.
#[test]
fn test_m98_session_with_duality_parses() {
    let src = r#"
module Protocol
  session PingPong
    pinger:
      send: Ping
      recv: Pong
    end
    ponger:
      recv: Ping
      send: Pong
    end
    duality: pinger <-> ponger
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "session with duality should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    let session = module.items.iter().find_map(|i| {
        if let Item::Session(s) = i {
            Some(s)
        } else {
            None
        }
    });
    assert!(session.is_some());
    let s = session.unwrap();
    assert_eq!(
        s.duality,
        Some(("pinger".to_string(), "ponger".to_string()))
    );
}

// 4. Duality violation (send Int but partner recv String) → checker error.
#[test]
fn test_m98_duality_violation_checker_error() {
    use loom::checker::SessionChecker;
    let src = r#"
module BadProtocol
  session BrokenPair
    roleA:
      send: Int
    end
    roleB:
      recv: String
    end
    duality: roleA <-> roleB
  end
end
"#;
    let module = parse(src).expect("should parse");
    let errors = SessionChecker::new().check(&module);
    assert!(
        !errors.is_empty(),
        "duality violation should produce an error"
    );
    let msgs: String = errors.iter().map(|e| format!("{}", e)).collect();
    assert!(
        msgs.contains("duality violation"),
        "error should mention 'duality violation', got: {msgs}"
    );
}

// 5. type with Channel<Protocol.role> in function signature parses.
#[test]
fn test_m98_channel_type_in_fn_signature_parses() {
    let src = r#"
module Server
  session AuthProtocol
    client:
      send: Credentials
      recv: Token
    end
    server:
      recv: Credentials
      send: Token
    end
    duality: client <-> server
  end

  fn handle_auth :: Channel -> Result
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "fn with channel-like sig should parse: {:?}",
        result.err()
    );
}

// 6. session with @implements annotation parses.
#[test]
fn test_m98_implements_annotation_parses() {
    let src = r#"
module Impl
  session EchoProtocol
    sender:
      send: String
      recv: String
    end
    receiver:
      recv: String
      send: String
    end
    duality: sender <-> receiver
  end

  fn echo_server @implements(EchoProtocol) :: Channel -> Result
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "fn with @implements annotation should parse: {:?}",
        result.err()
    );
}
