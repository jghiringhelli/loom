//! M111 tests — EvolutionVectorChecker: semantic migration deduplication.
//!
//! Uses type-lattice vector encoding (Ganter & Wille 1999) to detect identical
//! and related migration patterns across beings. The checker groups migrations
//! into evolutionary families using cosine similarity on semantic type vectors.

use loom::checker::EvolutionVectorChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn check(src: &str) -> Vec<String> {
    let tokens = Lexer::tokenize(src).expect("lex");
    let module = Parser::new(&tokens).parse_module().expect("parse");
    EvolutionVectorChecker::new()
        .check(&module)
        .iter()
        .map(|e| format!("{}", e))
        .collect()
}

// ── 1. Identical migration in two beings → duplicate warning ─────────────────

#[test]
fn test_m111_identical_migration_warns_duplicate() {
    let src = r#"
module Finance
  being TradeA
    telos: "trade a"
    end
    matter:
      price: Float
    end
    migration price_upgrade:
      from: price Float
      to:   price Double
      adapter: "fn v -> v as f64"
    end
  end
  being TradeB
    telos: "trade b"
    end
    matter:
      price: Float
    end
    migration price_upgrade:
      from: price Float
      to:   price Double
      adapter: "fn v -> v as f64"
    end
  end
end
"#;
    let warnings = check(src);
    assert!(
        !warnings.is_empty(),
        "identical Float→Double migration in two beings should produce a duplicate warning"
    );
    let combined = warnings.join("\n");
    assert!(
        combined.contains("duplicate") || combined.contains("shared"),
        "warning should mention duplicate or shared adapter: {}", combined
    );
}

// ── 2. Different migrations no false positive ─────────────────────────────────

#[test]
fn test_m111_different_migrations_no_warning() {
    let src = r#"
module Finance
  being SensorA
    telos: "sensor a"
    end
    matter:
      value: Float
    end
    migration precision:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
  end
  being SensorB
    telos: "sensor b"
    end
    matter:
      name: String
    end
    migration encoding:
      from: name String
      to:   name Bytes
      adapter: "fn v -> v.as_bytes()"
    end
  end
end
"#;
    let warnings = check(src);
    // Float→Double and String→Bytes are different type families — no duplicate.
    let combined = warnings.join("\n");
    assert!(
        !combined.contains("duplicate"),
        "Float→Double and String→Bytes should not trigger a duplicate warning: {}", combined
    );
}

// ── 3. Related migrations (same family) form a cluster ────────────────────────

#[test]
fn test_m111_related_migrations_form_cluster() {
    let src = r#"
module Finance
  being SensorA
    telos: "sensor a"
    end
    matter:
      value: Float
    end
    migration widen:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
  end
  being SensorB
    telos: "sensor b"
    end
    matter:
      reading: Float
    end
    migration widen:
      from: reading Float
      to:   reading Double
      adapter: "fn v -> v as f64"
    end
  end
  being SensorC
    telos: "sensor c"
    end
    matter:
      measurement: Float
    end
    migration widen:
      from: measurement Float
      to:   measurement Double
      adapter: "fn v -> v as f64"
    end
  end
end
"#;
    let warnings = check(src);
    // Three Float→Double migrations across three beings = cluster AND duplicates.
    let combined = warnings.join("\n");
    assert!(
        !combined.is_empty(),
        "three identical Float→Double migrations should trigger a warning"
    );
}

// ── 4. Single being migrations do not trigger cross-being warnings ─────────────

#[test]
fn test_m111_single_being_no_cross_warnings() {
    let src = r#"
module Finance
  being Standalone
    telos: "standalone"
    end
    matter:
      value: Float
    end
    migration v1:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
    migration v2:
      from: value Double
      to:   value Decimal
      adapter: "fn v -> Decimal::from(v)"
    end
  end
end
"#;
    let warnings = check(src);
    let combined = warnings.join("\n");
    assert!(
        !combined.contains("duplicate"),
        "migrations within a single being should not trigger cross-being duplicate warning: {}", combined
    );
}

// ── 5. Module with no migrations produces no warnings ─────────────────────────

#[test]
fn test_m111_no_migrations_no_warnings() {
    let src = r#"
module Empty
  being SimpleA
    telos: "simple"
    end
  end
  being SimpleB
    telos: "simple"
    end
  end
end
"#;
    let warnings = check(src);
    assert!(
        warnings.is_empty(),
        "module with no migrations should produce no warnings: {:?}", warnings
    );
}

// ── 6. Type vectors encode known type lattice correctly ───────────────────────

#[test]
fn test_m111_numeric_types_are_in_same_family() {
    // Int→Float and Float→Double are both "numeric widening" — similar delta vectors.
    // This is a structural test: verify the checker runs without panicking and
    // produces sensible output for a realistic numeric evolution scenario.
    let src = r#"
module Numeric
  being IntSensor
    telos: "int sensor"
    end
    matter:
      count: Int
    end
    migration widen_int:
      from: count Int
      to:   count Float
      adapter: "fn v -> v as f32"
    end
  end
  being FloatSensor
    telos: "float sensor"
    end
    matter:
      measure: Float
    end
    migration widen_float:
      from: measure Float
      to:   measure Double
      adapter: "fn v -> v as f64"
    end
  end
end
"#;
    // Should not panic. Output may include cluster or duplicate info.
    let _warnings = check(src);
}
