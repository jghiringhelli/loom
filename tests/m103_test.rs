//! M103 tests — BoundaryChecker: boundary: explicit public API surface.
//!
//! Parnas (1972) information hiding → Composable GS → Loom `boundary:`.

/// Test 1: boundary: with export:/private: parses
#[test]
fn test_m103_boundary_block_parses() {
    let src = r#"module M
type PublicType = value: Int end
type InternalState = data: String end
fn public_fn :: Int -> Int
  require: true
  ensure: true
end
boundary:
  export: PublicType public_fn
  private: InternalState
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "boundary block should parse, got: {:?}", result.err());
}

/// Test 2: private type in public fn signature → error
#[test]
fn test_m103_private_type_in_public_fn_is_error() {
    let src = r#"module M
type InternalHelper = data: String end
fn public_api :: InternalHelper -> String
  require: true
  ensure: true
end
boundary:
  export: public_api
  private: InternalHelper
end
end"#;
    let result = loom::compile(src);
    match result {
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.contains("InternalHelper") || msg.contains("private") || msg.contains("boundary"),
                "expected boundary leak error, got: {msg}"
            );
        }
        Ok(_) => panic!("expected error for private type in public fn signature"),
    }
}

/// Test 3: exporting nonexistent symbol → error
#[test]
fn test_m103_ghost_export_is_error() {
    let src = r#"module M
type RealType = value: Int end
boundary:
  export: RealType GhostFunction
end
end"#;
    let result = loom::compile(src);
    match result {
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.contains("GhostFunction") || msg.contains("not declared") || msg.contains("boundary"),
                "expected ghost export error, got: {msg}"
            );
        }
        Ok(_) => panic!("expected error for ghost export"),
    }
}

/// Test 4: seal: list parses correctly
#[test]
fn test_m103_seal_parses() {
    let src = r#"module M
type SealedType = value: Int end
boundary:
  export: SealedType
  seal: SealedType
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "seal: in boundary should parse, got: {:?}", result.err());
}

/// Test 5: boundary: inside being: block parses
#[test]
fn test_m103_being_with_boundary_parses() {
    let src = r#"module M
being Agent
  telos: "serve users"
  end
  boundary:
    export: process_request
    private: internal_state
  end
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "boundary: in being block should parse, got: {:?}", result.err());
}

/// Test 6: boundary: is optional (no boundary = valid)
#[test]
fn test_m103_no_boundary_is_valid() {
    let src = r#"module M
type Foo = x: Int end
fn bar :: Int -> Int
  require: true
  ensure: true
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "module without boundary: should be valid, got: {:?}", result.err());
}
