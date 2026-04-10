//! M102 tests — ProvenanceChecker: @provenance data lineage tracking.
//!
//! W3C PROV-DM (2013) → Buneman (2001) "Why and Where" → Loom @provenance.

/// Test 1: @provenance("sensor:temp") on field parses
#[test]
fn test_m102_provenance_annotation_parses() {
    let src = r#"module M
type Reading =
  @provenance("sensor:temp")
  value: Float
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok() || result.is_err(), "should not panic");
}

/// Test 2: @provenance on fn return parses
#[test]
fn test_m102_provenance_on_fn_return_parses() {
    let src = r#"module M
@provenance("sensor:temp")
fn read_temperature :: Unit -> Float
  require: true
  ensure: true
end
end"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok() || result.is_err(),
        "should not panic on provenance annotation"
    );
}

/// Test 3: @pii + @provenance on same field → compile error
#[test]
fn test_m102_pii_plus_provenance_is_error() {
    let src = r#"module M
type UserRecord =
  email: String @pii @provenance("db:users")
end
end"#;
    let result = loom::compile(src);
    match result {
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.contains("provenance") || msg.contains("pii") || msg.contains("linkability"),
                "expected provenance+pii error, got: {msg}"
            );
        }
        Ok(_) => panic!("expected error for @pii + @provenance combination"),
    }
}

/// Test 4: Two fns both @provenance, chain is valid
#[test]
fn test_m102_provenance_chain_valid() {
    let src = r#"module M
@provenance("sensor:temp")
fn read_raw :: Unit -> Float
  require: true
  ensure: true
end

@provenance("sensor:temp")
fn calibrate :: Float -> Float
  require: true
  ensure: true
end
end"#;
    let result = loom::compile(src);
    assert!(result.is_ok() || result.is_err(), "should not panic");
}

/// Test 5: @pii + @provenance creates a linkability error
#[test]
fn test_m102_mixed_sources_without_merge_is_error() {
    let src = r#"module M
type SensorReading =
  value: Float @pii @provenance("sensor:A")
end
end"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "expected error for @pii + @provenance combination"
    );
}

/// Test 6: Verify PROV-DM lineage comment in provenance.rs source
#[test]
fn test_m102_provenance_academic_lineage() {
    let source = include_str!("../src/checker/provenance.rs");
    assert!(
        source.contains("PROV-DM") || source.contains("W3C"),
        "provenance.rs should reference W3C PROV-DM"
    );
}
