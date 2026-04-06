//! Tests for M62: Side-Channel Information Flow

use loom::compile;

#[test]
fn timing_safe_with_block_ok() {
    let src = r#"
module Test
fn compare @timing-safe @pure :: Int -> Int -> Bool
timing_safety:
  constant_time: true
  leaks_bits: "0.0 bits"
  method: data_oblivious
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn timing_safe_without_block_errors() {
    let src = r#"
module Test
fn compare @timing-safe @pure :: Int -> Int -> Bool
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("timing") || msg.contains("side-channel"), "expected timing error in: {}", msg);
}

#[test]
fn constant_time_with_leaks_errors() {
    let src = r#"
module Test
fn compare @timing-safe @pure :: Int -> Int -> Bool
timing_safety:
  constant_time: true
  leaks_bits: "3.2 bits"
end
end
end"#;
    let result = compile(src);
    assert!(result.is_err());
    let msg = result.unwrap_err()[0].to_string();
    assert!(msg.contains("bits") || msg.contains("constant_time") || msg.contains("zero"), "expected leaks_bits error in: {}", msg);
}

#[test]
fn constant_time_false_with_leaks_ok() {
    let src = r#"
module Test
fn compare @pure :: Int -> Int -> Bool
timing_safety:
  constant_time: false
  leaks_bits: "3.2 bits"
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}

#[test]
fn timing_safety_in_codegen() {
    let src = r#"
module Test
fn compare @timing-safe @pure :: Int -> Int -> Bool
timing_safety:
  constant_time: true
  leaks_bits: "0.0 bits"
end
end
end"#;
    let result = compile(src);
    assert!(result.is_ok());
    let rust_src = result.unwrap();
    assert!(rust_src.contains("timing_safety") || rust_src.contains("constant_time"), "Expected timing_safety in codegen");
}

#[test]
fn constant_time_with_zero_leaks_ok() {
    let src = r#"
module Test
fn compare @timing-safe @pure :: Int -> Int -> Bool
timing_safety:
  constant_time: true
  leaks_bits: "0"
end
end
end"#;
    assert!(compile(src).is_ok(), "expected OK: {:?}", compile(src));
}
