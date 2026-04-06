//! Tests for M65: Self-Certifying Compilation

use loom::compile;

#[test]
fn certificate_valid_fields_ok() {
    let src = r#"
module Test
certificate:
  type_safety = proven
  memory_safety = verified
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn certificate_invalid_value_errors() {
    let src = r#"
module Test
certificate:
  type_safety = maybe
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("type_safety") || msg.contains("certificate") || msg.contains("value"), "expected cert error in: {}", msg);
}

#[test]
fn certificate_all_standard_fields_ok() {
    let src = r#"
module Test
certificate:
  type_safety = proven
  memory_safety = verified
  purity = checked
  privacy = passed
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn certificate_custom_field_ok() {
    let src = r#"
module Test
certificate:
  reviewed_by = alice
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn certificate_emitted_as_comments() {
    let src = r#"
module Test
certificate:
  type_safety = proven
  memory_safety = verified
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("type_safety") || rust_src.contains("certificate"), "Expected certificate in codegen: {}", &rust_src[..rust_src.len().min(500)]);
}

#[test]
fn certificate_only_module_ok() {
    let src = r#"
module Test
certificate:
  timing_safety = proven
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}
