// M106: Migration — Interface Evolution Contract
// Tests for migration: block parsing and semantic validation.

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
