//! M186 — `role:` annotation on beings.
//! M187 — `relates_to:` structural relationship declarations.
//!
//! M186: `role: sensor|effector|regulator|integrator|memory|classifier`
//! emits `// LOOM[role:X]` comment in the generated Rust.
//!
//! M187: `relates_to: TargetName kind: mutualistic|commensal|parasitic`
//! emits `// LOOM[relates_to:Target:kind]` comment in the generated Rust.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── M186: role: ──────────────────────────────────────────────────────────────

const SENSOR_BEING: &str = r#"
module Env
  being TemperatureSensor
    role: sensor
    telos: "measure temperature accurately"
    matter:
      current_temp: Float
    end
  end
end
"#;

#[test]
fn m186_role_sensor_emitted() {
    let out = compile(SENSOR_BEING);
    assert!(
        out.contains("// LOOM[role:sensor]"),
        "expected LOOM[role:sensor], got:\n{}",
        out
    );
}

#[test]
fn m186_role_effector_emitted() {
    let src = r#"
module Ctrl
  being Actuator
    role: effector
    telos: "actuate physical system"
    matter:
      state: Float
    end
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("// LOOM[role:effector]"), "expected role:effector");
}

#[test]
fn m186_role_regulator_emitted() {
    let src = r#"
module Ctrl
  being HomeostasisController
    role: regulator
    telos: "maintain homeostasis"
    matter:
      setpoint: Float
    end
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("// LOOM[role:regulator]"), "expected role:regulator");
}

/// A being without `role:` produces no LOOM[role:] comment.
#[test]
fn m186_no_role_no_comment() {
    let src = r#"
module Simple
  being Worker
    telos: "do work"
    matter:
      id: Int
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        !out.contains("LOOM[role:"),
        "no LOOM[role:] expected when role: is absent"
    );
}

// ── M187: relates_to: ────────────────────────────────────────────────────────

#[test]
fn m187_relates_to_mutualistic_emitted() {
    let src = r#"
module Ecosystem
  being PlantSensor
    role: sensor
    relates_to: NutrientEffector kind: mutualistic
    telos: "sense plant health"
    matter:
      chlorophyll: Float
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("// LOOM[relates_to:NutrientEffector:mutualistic]"),
        "expected relates_to comment, got:\n{}",
        out
    );
}

#[test]
fn m187_relates_to_default_kind_mutualistic() {
    let src = r#"
module Eco
  being SensorA
    role: sensor
    relates_to: SensorB
    telos: "measure"
    matter:
      val: Float
    end
  end
end
"#;
    let out = compile(src);
    // When kind: is omitted, default is mutualistic
    assert!(
        out.contains("// LOOM[relates_to:SensorB:mutualistic]"),
        "default kind should be mutualistic, got:\n{}",
        out
    );
}

#[test]
fn m187_multiple_relates_to_all_emitted() {
    let src = r#"
module Env
  being ClimateNode
    role: integrator
    relates_to: TempSensor kind: mutualistic
    relates_to: HumiditySensor kind: commensal
    telos: "integrate climate signals"
    matter:
      state: Float
    end
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("// LOOM[relates_to:TempSensor:mutualistic]"),
        "first relates_to missing"
    );
    assert!(
        out.contains("// LOOM[relates_to:HumiditySensor:commensal]"),
        "second relates_to missing"
    );
}

/// Role and relates_to are emitted before the struct definition.
#[test]
fn m187_role_and_relates_to_precede_struct() {
    let src = r#"
module Env
  being ClimateAdapter
    role: sensor
    relates_to: EnergyGrid kind: mutualistic
    telos: "adapt climate readings"
    matter:
      temp: Float
    end
  end
end
"#;
    let out = compile(src);
    let role_pos = out.find("LOOM[role:sensor]").expect("LOOM[role:sensor] expected");
    let relates_pos = out
        .find("LOOM[relates_to:EnergyGrid:mutualistic]")
        .expect("relates_to expected");
    let struct_pos = out.find("struct ClimateAdapter").or_else(|| out.find("ClimateAdapter"));
    if let Some(sp) = struct_pos {
        assert!(role_pos < sp, "LOOM[role:] must precede struct");
        assert!(relates_pos < sp, "LOOM[relates_to:] must precede struct");
    }
}
