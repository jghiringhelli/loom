// M107: Minimal Checker — Dead Declaration Detection
// Tests for unused declaration detection in being blocks.

// 1. sense: channel never referenced in evolve: → warning in compile output.
#[test]
fn test_m107_unused_sense_channel_is_warning() {
    let src = r#"
module Environment
  sense temperature
    channels: [celsius]
  end

  being Thermostat
    telos: "maintain equilibrium"
    end
    matter:
      setpoint: Float
    end
    regulate setpoint
      target: setpoint
      bounds: (low, high)
    end
  end
end
"#;
    // Compile returns Err when warnings are present (minimal checker surfaces them).
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "unused sense channel should produce a diagnostic"
    );
    let errs = result.unwrap_err();
    let combined = errs
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        combined.contains("[warn]"),
        "expected [warn] for unused sense channel, got: {}",
        combined
    );
}

// 2. regulate: bound for field not in matter: → hard error.
#[test]
fn test_m107_regulate_on_nonexistent_field_is_error() {
    let src = r#"
module Controller
  being PIDController
    telos: "track reference signal"
    end
    matter:
      output: Float
    end
    regulate ghost_field
      target: ghost_field
      bounds: (low, high)
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "regulate on nonexistent field should be an error"
    );
    let errs = result.unwrap_err();
    let combined = errs
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        combined.contains("[error]"),
        "expected [error] for regulate on nonexistent field, got: {}",
        combined
    );
    assert!(
        combined.contains("ghost_field"),
        "error should name the missing field, got: {}",
        combined
    );
}

// 3. sense: channel referenced in evolve: constraint → no warning.
#[test]
fn test_m107_used_sense_is_valid() {
    let src = r#"
module Environment
  sense temperature
    channels: [celsius]
  end

  being Thermostat
    telos: "maintain thermal equilibrium"
    end
    evolve
      toward: telos
      search:
        | gradient_descent when gradient_available
      constraint: "celsius decreasing toward setpoint"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "sense channel referenced in evolve: should compile cleanly: {:?}",
        result.err()
    );
}

// 4. being with no sense: declarations → no error.
#[test]
fn test_m107_being_without_sense_is_valid() {
    let src = r#"
module Controller
  being SimpleController
    telos: "control output"
    end
    matter:
      gain: Float
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "being without any sense: should compile cleanly: {:?}",
        result.err()
    );
}

// 5. fully load-bearing being → clean compile.
#[test]
fn test_m107_all_used_declarations_valid() {
    let src = r#"
module System
  sense pressure
    channels: [pascal]
  end

  being PressureController
    telos: "maintain safe pressure"
    end
    matter:
      pascal: Float
    end
    regulate pascal
      target: pascal
      bounds: (low, high)
    end
    evolve
      toward: telos
      search:
        | gradient_descent when gradient_available
      constraint: "pascal decreasing toward target"
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(
        result.is_ok(),
        "fully load-bearing being should compile cleanly: {:?}",
        result.err()
    );
}

// 6. loom::compile() runs the minimal checker in the full pipeline.
#[test]
fn test_m107_minimal_checker_in_pipeline() {
    // A being with regulate referencing a nonexistent field — must be caught by
    // the pipeline's minimal checker, not silently ignored.
    let src = r#"
module Pipeline
  being Agent
    telos: "pipeline agent"
    end
    matter:
      real_field: Float
    end
    regulate phantom
      target: phantom
      bounds: (low, high)
    end
  end
end
"#;
    // The compile() pipeline must catch this as an error.
    let result = loom::compile(src);
    assert!(
        result.is_err(),
        "MinimalChecker must run in compile() pipeline and catch phantom regulate"
    );
    let errs = result.unwrap_err();
    let combined = errs
        .iter()
        .map(|e| format!("{}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        combined.contains("phantom"),
        "error message should identify the phantom field: {}",
        combined
    );
}
