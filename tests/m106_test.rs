// M106: Migration — Interface Evolution Contract
// Tests for migration: block parsing and semantic validation.
// Adversarial tests for chain consistency and cycle detection (Rules 5 + 6) are at the bottom.

// 1. migration block with from:/to:/adapter: parses.
#[test]
fn test_m106_migration_parses() {
    let src = r#"
module Sensor
  being TemperatureSensor
    telos: "measure temperature"
    end
    matter:
      sense_interval: Float
    end
    migration v1_to_v2:
      from: sense_interval Float
      to:   sense_interval Duration
      adapter: "fn v1 -> Duration::from_seconds(v1)"
    end
  end
end
"#;
    let result = loom::parse(src);
    assert!(result.is_ok(), "migration block should parse: {:?}", result.err());
    let module = result.unwrap();
    let being = module.being_defs.iter().find(|b| b.name == "TemperatureSensor");
    assert!(being.is_some(), "should find TemperatureSensor");
    let b = being.unwrap();
    assert_eq!(b.migrations.len(), 1);
    let m = &b.migrations[0];
    assert_eq!(m.name, "v1_to_v2");
    assert!(m.adapter.is_some());
    assert!(m.breaking); // default is true
}

// 2. breaking: false without adapter: → error from MigrationChecker.
#[test]
fn test_m106_nonbreaking_without_adapter_is_error() {
    let src = r#"
module Sensor
  being TemperatureSensor
    telos: "measure temperature"
    end
    matter:
      threshold: Float
    end
    migration v2_to_v3:
      from: threshold Float
      to:   threshold Float
      breaking: false
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "non-breaking migration without adapter should be an error"
    );
    let errs = result.unwrap_err();
    let combined = errs.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("[error]"),
        "expected [error] in output, got: {}", combined
    );
}

// 3. two migrations with same name → error from MigrationChecker.
#[test]
fn test_m106_duplicate_migration_name_is_error() {
    let src = r#"
module Sensor
  being PressureSensor
    telos: "measure pressure"
    end
    matter:
      reading: Float
    end
    migration v1_to_v2:
      from: reading Float
      to:   reading Double
      adapter: "fn v -> v as f64"
    end
    migration v1_to_v2:
      from: reading Float
      to:   reading String
      adapter: "fn v -> v.to_string()"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "duplicate migration name should be an error"
    );
    let errs = result.unwrap_err();
    let combined = errs.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("duplicate migration"),
        "expected duplicate migration error, got: {}", combined
    );
}

// 4. migration without breaking: field → breaking defaults to true.
#[test]
fn test_m106_breaking_flag_defaults_true() {
    let src = r#"
module Sensor
  being HumiditySensor
    telos: "measure humidity"
    end
    matter:
      level: Float
    end
    migration v1_to_v2:
      from: level Float
      to:   level Percentage
      adapter: "fn v -> Percentage::new(v)"
    end
  end
end
"#;
    let result = loom::parse(src);
    assert!(result.is_ok(), "migration without breaking: should parse: {:?}", result.err());
    let module = result.unwrap();
    let being = module.being_defs.iter().find(|b| b.name == "HumiditySensor").unwrap();
    assert_eq!(being.migrations.len(), 1);
    assert!(
        being.migrations[0].breaking,
        "breaking should default to true when not specified"
    );
}

// 5. two migration blocks in one being parse without error.
#[test]
fn test_m106_multiple_migrations_parse() {
    let src = r#"
module Sensor
  being MultiVersionSensor
    telos: "multi-version sensor"
    end
    matter:
      value: Float
    end
    migration v1_to_v2:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
    migration v2_to_v3:
      from: value Double
      to:   value Decimal
      adapter: "fn v -> Decimal::from(v)"
    end
  end
end
"#;
    let result = loom::parse(src);
    assert!(result.is_ok(), "multiple migrations should parse: {:?}", result.err());
    let module = result.unwrap();
    let being = module.being_defs.iter().find(|b| b.name == "MultiVersionSensor").unwrap();
    assert_eq!(being.migrations.len(), 2);
    assert_eq!(being.migrations[0].name, "v1_to_v2");
    assert_eq!(being.migrations[1].name, "v2_to_v3");
}

// 6. migration: is optional — being without migrations compiles cleanly.
#[test]
fn test_m106_being_without_migration_is_valid() {
    let src = r#"
module Sensor
  being SimpleSensor
    telos: "simple sensor"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "being without migration: should compile cleanly: {:?}", result.err()
    );
}

// ── Adversarial suite: chain consistency and cycle detection (Rules 5 + 6) ──

// 7. Consistent two-step chain compiles cleanly.
//    v1→v2 produces Float, v2→v3 consumes Float — chain is valid.
#[test]
fn test_m106_consistent_chain_compiles() {
    let src = r#"
module Sensor
  being ChainedSensor
    telos: "test consistent chain"
    end
    matter:
      reading: Float
    end
    migration v1_to_v2:
      from: reading Float
      to:   reading Double
      adapter: "fn v -> v as f64"
    end
    migration v2_to_v3:
      from: reading Double
      to:   reading Decimal
      adapter: "fn v -> Decimal::from(v)"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "consistent two-step chain should compile cleanly: {:?}", result.err()
    );
}

// 8. Broken chain: v1→v2 produces Double, v2→v3 expects String — should error.
#[test]
fn test_m106_broken_chain_is_error() {
    let src = r#"
module Sensor
  being BrokenChainSensor
    telos: "test broken chain"
    end
    matter:
      reading: Float
    end
    migration v1_to_v2:
      from: reading Float
      to:   reading Double
      adapter: "fn v -> v as f64"
    end
    migration v2_to_v3:
      from: reading String
      to:   reading Decimal
      adapter: "fn v -> Decimal::parse(v)"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_err(), "broken chain should produce an error");
    let errs = result.unwrap_err();
    let combined = errs.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("chain broken") || combined.contains("[error]"),
        "expected chain broken error, got: {}", combined
    );
}

// 9. Cycle detection: Float → Double → Float is a type cycle — should error.
#[test]
fn test_m106_type_cycle_is_error() {
    let src = r#"
module Sensor
  being CyclicSensor
    telos: "test type cycle"
    end
    matter:
      value: Float
    end
    migration v1_to_v2:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
    migration v2_to_v1:
      from: value Double
      to:   value Float
      adapter: "fn v -> v as f32"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_err(), "type cycle should produce an error");
    let errs = result.unwrap_err();
    let combined = errs.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("cycle") || combined.contains("[error]"),
        "expected cycle error, got: {}", combined
    );
}

// 10. Three-step consistent chain compiles cleanly (String → Bytes → Utf8 → Text).
#[test]
fn test_m106_three_step_consistent_chain_compiles() {
    let src = r#"
module Codec
  being EncodingSensor
    telos: "test three-step chain"
    end
    matter:
      payload: String
    end
    migration v1_to_v2:
      from: payload String
      to:   payload Bytes
      adapter: "fn v -> v.as_bytes()"
    end
    migration v2_to_v3:
      from: payload Bytes
      to:   payload Utf8
      adapter: "fn v -> Utf8::from_bytes(v)"
    end
    migration v3_to_v4:
      from: payload Utf8
      to:   payload Text
      adapter: "fn v -> Text::from_utf8(v)"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "three-step consistent chain should compile cleanly: {:?}", result.err()
    );
}

// 11. Adapter function name recorded — adapter field stores the identifier.
#[test]
fn test_m106_adapter_ident_is_recorded() {
    let src = r#"
module Sensor
  being AdapterSensor
    telos: "test adapter ident"
    end
    matter:
      reading: Float
    end
    migration v1_to_v2:
      from: reading Float
      to:   reading Double
      adapter: convert_float_to_double
    end
  end
end
"#;
    let result = loom::parse(src);
    assert!(result.is_ok(), "migration with ident adapter should parse: {:?}", result.err());
    let module = result.unwrap();
    let being = module.being_defs.iter().find(|b| b.name == "AdapterSensor").unwrap();
    let migration = &being.migrations[0];
    assert_eq!(
        migration.adapter.as_deref(),
        Some("convert_float_to_double"),
        "adapter ident should be recorded verbatim"
    );
}

// 12. Version-number migration (integer from:/to:) compiles without chain errors.
//     This is the ALX-1 corpus form — version numbers, not field types.
#[test]
fn test_m106_version_number_migration_compiles() {
    let src = r#"
module Service
  being VersionedService
    telos: "test version number migration"
    end
    migration v1_to_v2:
      from: 1
      to: 2
      breaking: false
      adapter: migrate_v1_v2
    end
    migration v2_to_v3:
      from: 2
      to: 3
      breaking: false
      adapter: migrate_v2_v3
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "version-number migrations should compile cleanly: {:?}", result.err()
    );
}
