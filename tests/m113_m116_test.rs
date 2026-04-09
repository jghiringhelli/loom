//! M112–M116 tests: TelosDef upgrade, TelosImmutability, telos_contribution,
//! signal_attention, messaging_primitive.

use loom::compile;

fn ok(src: &str) {
    let r = compile(src);
    assert!(
        r.is_ok(),
        "expected ok:\n{}",
        r.unwrap_err().iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
    );
}

fn err_contains(src: &str, fragment: &str) {
    let r = compile(src);
    assert!(r.is_err(), "expected error containing '{}' but compiled ok", fragment);
    let msg = r.unwrap_err().iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains(fragment),
        "expected error containing '{}'\nGot:\n{}",
        fragment,
        msg
    );
}

// ── M112: TelosDef upgrade — telos with measured_by and thresholds ────────────

#[test]
fn test_m112_telos_with_measured_by_ok() {
    ok(r#"
module M
  being Sensor
    telos: "maintain calibration"
      measured_by: "fn :: SensorState -> Float"
    end
  end
end
"#);
}

#[test]
fn test_m112_telos_with_valid_thresholds_ok() {
    ok(r#"
module M
  being Regulator
    telos: "regulate temperature"
      thresholds:
        convergence: 0.8
        divergence: 0.3
      end
    end
  end
end
"#);
}

#[test]
fn test_m112_threshold_divergence_less_than_convergence_required() {
    err_contains(
        r#"
module M
  being Regulator
    telos: "regulate temperature"
      thresholds:
        convergence: 0.3
        divergence: 0.8
      end
    end
  end
end
"#,
        "divergence",
    );
}

#[test]
fn test_m112_threshold_out_of_range_errors() {
    err_contains(
        r#"
module M
  being Regulator
    telos: "regulate temperature"
      thresholds:
        convergence: 1.5
        divergence: 0.3
      end
    end
  end
end
"#,
        "convergence threshold",
    );
}

// ── M113: TelosImmutability — modifiable_by without @corrigible ───────────────

#[test]
fn test_m113_modifiable_by_with_corrigible_ok() {
    ok(r#"
module M
  being Agent
    @corrigible
    telos: "serve"
      modifiable_by: HumanOperator
    end
  end
end
"#);
}

#[test]
fn test_m113_modifiable_by_without_corrigible_warns() {
    // M113 TelosImmutability: emitted as [warn] so compile() succeeds,
    // but the warning must appear in the checker output.
    // compile() suppresses [warn] prefixes in SafetyCheckerAdapter (hard stage).
    // Actually SafetyCheckerAdapter uses hard(), so [warn] messages are not filtered.
    // This should produce an error about telos immutability.
    err_contains(
        r#"
module M
  being Agent
    telos: "serve"
      modifiable_by: HumanOperator
    end
  end
end
"#,
        "telos is immutable",
    );
}

// ── M114: telos_contribution in regulate block ────────────────────────────────

#[test]
fn test_m114_telos_contribution_valid_ok() {
    ok(r#"
module M
  being Regulator
    telos: "regulate temperature"
    end
    regulate temperature
      target: nominal
      bounds: (low, high)
      telos_contribution: 0.8
    end
  end
end
"#);
}

#[test]
fn test_m114_telos_contribution_out_of_range_errors() {
    err_contains(
        r#"
module M
  being Regulator
    telos: "regulate temperature"
    end
    regulate temperature
      target: nominal
      bounds: (low, high)
      telos_contribution: 1.5
    end
  end
end
"#,
        "telos_contribution",
    );
}

// ── M115: signal_attention block validation ───────────────────────────────────

#[test]
fn test_m115_signal_attention_valid_ok() {
    ok(r#"
module M
  being Sensor
    telos: "detect signals"
    end
    signal_attention
      prioritize: 0.7
      attenuate: 0.2
    end
  end
end
"#);
}

#[test]
fn test_m115_signal_attention_inverted_window_errors() {
    err_contains(
        r#"
module M
  being Sensor
    telos: "detect signals"
    end
    signal_attention
      prioritize: 0.3
      attenuate: 0.7
    end
  end
end
"#,
        "inverted window",
    );
}

#[test]
fn test_m115_signal_attention_prioritize_out_of_range_errors() {
    err_contains(
        r#"
module M
  being Sensor
    telos: "detect signals"
    end
    signal_attention
      prioritize: 1.5
      attenuate: 0.2
    end
  end
end
"#,
        "prioritize_above",
    );
}

// ── M116: messaging_primitive declaration ────────────────────────────────────

#[test]
fn test_m116_messaging_primitive_request_response_ok() {
    ok(r#"
module M
  messaging_primitive OrderChannel
    pattern: request_response
    timeout: mandatory
  end
end
"#);
}

#[test]
fn test_m116_messaging_primitive_publish_subscribe_ok() {
    ok(r#"
module M
  messaging_primitive EventBus
    pattern: publish_subscribe
  end
end
"#);
}

#[test]
fn test_m116_messaging_primitive_with_guarantees_ok() {
    ok(r#"
module M
  messaging_primitive ReliableQueue
    pattern: producer_consumer
    guarantees: at_least_once idempotent
  end
end
"#);
}
