//! V3 Proptest codegen tests.
//!
//! Gate: `property:` blocks emit (1) a deterministic edge-case `#[test]` and
//! (2) a `#[cfg(loom_proptest)]` proptest block with `prop_assert!` and correct strategies.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── Invariant translation ─────────────────────────────────────────────────

/// `and` keyword is translated to `&&` in emitted Rust.
#[test]
fn v3_and_becomes_and_and() {
    let src = r#"
module M
  property NonNegAndSmall:
    forall x: Int
    invariant: x >= 0 and x < 1000
    samples: 10
    shrink: true
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("x >= 0 && x < 1000"),
        "expected '&&', got:\n{}",
        out
    );
}

/// `or` keyword is translated to `||` in emitted Rust.
#[test]
fn v3_or_becomes_or_or() {
    let src = r#"
module M
  property EitherEnd:
    forall x: Bool
    invariant: x = true or x = false
    samples: 2
    shrink: false
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("||"), "expected '||', got:\n{}", out);
}

/// Standalone `=` becomes `==`; compound operators (`<=`, `>=`, `!=`) are preserved.
#[test]
fn v3_bare_equals_becomes_double_equals() {
    let src = r#"
module M
  property ExactlyOne:
    forall x: Int
    invariant: x = 1
    samples: 1
    shrink: false
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("== 1"), "expected '== 1', got:\n{}", out);
    assert!(!out.contains("=== 1"), "unexpected '==='");
}

/// `not ` keyword is translated to `!`.
#[test]
fn v3_not_becomes_bang() {
    let src = r#"
module M
  property NotNegative:
    forall x: Int
    invariant: not x < 0
    samples: 5
    shrink: false
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("!x < 0") || out.contains("! x < 0"),
        "expected '!', got:\n{}",
        out
    );
}

// ── Edge-case deterministic #[test] ──────────────────────────────────────

/// Int property emits Int edge cases with i64 type.
#[test]
fn v3_int_property_emits_edge_cases() {
    let src = r#"
module M
  property Positive:
    forall x: Int
    invariant: x > -1000000
    samples: 10
    shrink: true
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("#[test]"), "expected #[test]");
    assert!(
        out.contains("fn property_positive_edge_cases"),
        "expected edge_cases fn"
    );
    assert!(out.contains("i64"), "expected i64 type");
    assert!(
        out.contains("i64::MIN") || out.contains("i64::MAX"),
        "expected extremes"
    );
}

/// Float property emits f64 edge cases.
#[test]
fn v3_float_property_emits_f64_edge_cases() {
    let src = r#"
module M
  property SmallFloat:
    forall x: Float
    invariant: x >= -1000.0 and x <= 1000.0
    samples: 10
    shrink: false
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("f64"), "expected f64 type");
    assert!(
        out.contains("0.0") || out.contains("-1.0"),
        "expected float edge cases"
    );
}

/// Bool property emits `false, true` as edge cases.
#[test]
fn v3_bool_property_emits_bool_edge_cases() {
    let src = r#"
module M
  property BothBools:
    forall x: Bool
    invariant: x = true or x = false
    samples: 2
    shrink: false
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("bool"), "expected bool type");
    assert!(
        out.contains("false") && out.contains("true"),
        "expected both bool values"
    );
}

// ── Proptest random block ─────────────────────────────────────────────────

/// A `#[cfg(loom_proptest)]` block is emitted alongside the edge-case test.
#[test]
fn v3_proptest_cfg_block_emitted() {
    let src = r#"
module M
  property AboveZero:
    forall x: Int
    invariant: x != -999
    samples: 100
    shrink: true
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[cfg(loom_proptest)]"),
        "expected proptest cfg block"
    );
    assert!(out.contains("proptest!"), "expected proptest! macro");
    assert!(out.contains("prop_assert!"), "expected prop_assert!");
}

/// The proptest block uses the correct strategy for each type.
#[test]
fn v3_proptest_strategy_matches_type() {
    let src = r#"
module M
  property FloatRange:
    forall v: Float
    invariant: v < 1e10
    samples: 100
    shrink: true
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("f64::NORMAL") || out.contains("proptest::num::f64"),
        "expected f64 strategy"
    );
}

/// The generated `property_*_random` test embeds the translated invariant.
#[test]
fn v3_proptest_random_fn_embeds_invariant() {
    let src = r#"
module M
  property EvenPair:
    forall n: Int
    invariant: n >= 0 and n < 10000
    samples: 512
    shrink: true
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("fn property_even_pair_random"),
        "expected random fn"
    );
    assert!(
        out.contains("n >= 0 && n < 10000"),
        "expected translated invariant"
    );
}
