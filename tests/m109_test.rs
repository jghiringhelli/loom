// M109 tests — property: testing primitive.
//
// Verifies parsing, defaulting, and code generation for the `property:` construct.
// QuickCheck (Claessen & Hughes 2000) → fast-check → Hypothesis → Loom M109.

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;
use loom::compile;

fn parse(src: &str) -> Result<Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// ── 1. property: block parses correctly ──────────────────────────────────────

#[test]
fn test_m109_property_parses() {
    let src = r#"
module Props
  property encode_decode_roundtrip:
    forall x: String
    invariant: decode(encode(x)) = x
    shrink: true
    samples: 1000
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "property: block should parse: {:?}", result.err());
    let module = result.unwrap();
    let prop = module.items.iter().find_map(|i| {
        if let Item::Property(pb) = i { Some(pb) } else { None }
    });
    assert!(prop.is_some(), "should have one Property item");
    let pb = prop.unwrap();
    assert_eq!(pb.name, "encode_decode_roundtrip");
    assert_eq!(pb.var_name, "x");
    assert_eq!(pb.var_type, "String");
    assert!(pb.invariant.contains("decode"), "invariant should contain 'decode'");
    assert_eq!(pb.shrink, true);
    assert_eq!(pb.samples, 1000);
}

// ── 2. samples: 0 is a compile error ─────────────────────────────────────────

#[test]
fn test_m109_zero_samples_is_error() {
    let src = r#"
module Props
  property broken_prop:
    forall x: Int
    invariant: x = x
    samples: 0
  end
end
"#;
    // compile runs the full pipeline including PropertyChecker
    let result = compile(src);
    assert!(
        result.is_err(),
        "samples: 0 must be a compile error"
    );
    let errors = result.unwrap_err();
    let has_samples_error = errors.iter().any(|e| format!("{}", e).contains("samples"));
    assert!(has_samples_error, "error must mention 'samples': {:?}", errors);
}

// ── 3. shrink defaults to true ───────────────────────────────────────────────

#[test]
fn test_m109_shrink_defaults_true() {
    let src = r#"
module Props
  property sort_idempotent:
    forall xs: List
    invariant: sort(sort(xs)) = sort(xs)
  end
end
"#;
    let module = parse(src).expect("should parse without shrink:");
    let prop = module.items.iter().find_map(|i| {
        if let Item::Property(pb) = i { Some(pb) } else { None }
    }).expect("should have a property");
    assert_eq!(prop.shrink, true, "shrink must default to true when omitted");
}

// ── 4. samples defaults to 100 ───────────────────────────────────────────────

#[test]
fn test_m109_samples_defaults_100() {
    let src = r#"
module Props
  property default_samples:
    forall n: Int
    invariant: n = n
  end
end
"#;
    let module = parse(src).expect("should parse without samples:");
    let prop = module.items.iter().find_map(|i| {
        if let Item::Property(pb) = i { Some(pb) } else { None }
    }).expect("should have a property");
    assert_eq!(prop.samples, 100, "samples must default to 100 when omitted");
}

// ── 5. Multiple property: blocks parse ───────────────────────────────────────

#[test]
fn test_m109_multiple_properties_parse() {
    let src = r#"
module Props
  property prop_one:
    forall x: String
    invariant: len(x) >= 0
    samples: 50
  end
  property prop_two:
    forall n: Int
    invariant: n + 0 = n
    samples: 200
  end
end
"#;
    let module = parse(src).expect("two property blocks should parse");
    let props: Vec<_> = module.items.iter().filter_map(|i| {
        if let Item::Property(pb) = i { Some(pb) } else { None }
    }).collect();
    assert_eq!(props.len(), 2, "should have exactly 2 property items");
    assert_eq!(props[0].name, "prop_one");
    assert_eq!(props[1].name, "prop_two");
    assert_eq!(props[0].samples, 50);
    assert_eq!(props[1].samples, 200);
}

// ── 6. property: emits #[test] fn stub in Rust codegen ───────────────────────

#[test]
fn test_m109_property_emits_test_stub() {
    let src = r#"
module Props
  property encode_decode_roundtrip:
    forall x: String
    invariant: decode(encode(x)) = x
    samples: 500
  end
end
"#;
    let result = compile(src);
    assert!(result.is_ok(), "property: block should compile to Rust: {:?}", result.err());
    let rust = result.unwrap();
    assert!(
        rust.contains("#[test]"),
        "emitted Rust must contain #[test] attribute"
    );
    assert!(
        rust.contains("fn property_encode_decode_roundtrip"),
        "emitted Rust must contain fn property_encode_decode_roundtrip"
    );
    // V3: emits edge-case loop rather than todo!()
    assert!(
        rust.contains("edge_cases") || rust.contains("proptest") || rust.contains("assert!"),
        "emitted property test must contain an assertion or edge-case loop:\n{}", rust
    );
    // V3+: also emits proptest block
    assert!(
        rust.contains("proptest"),
        "emitted property test must contain proptest block:\n{}", rust
    );
}
