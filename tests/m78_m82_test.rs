// M78–M82 integration tests: flow roles, telos sign, umwelt, sense, resonance.

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

#[test]
fn test_m78_flow_role_eraser_annotation() {
    let src = r#"
module Auth
  flow secret :: Password
  flow public :: HashedPassword

  fn hash_password @eraser :: Password -> HashedPassword
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "eraser annotation should parse: {:?}", result.err());
}

#[test]
fn test_m79_telos_sign_field() {
    let src = r#"
module Optimizer
  being GradientDescent
    telos: "minimize prediction error"
      sign: PredictionError
      modifiable_by: Supervisor
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "telos sign field should parse: {:?}", result.err());
}

#[test]
fn test_m80_umwelt_block_detects() {
    let src = r#"
module SensorAgent
  being ThermalAgent
    telos: "maintain thermal equilibrium"
    end
    umwelt:
      detects: [Temperature, HumidityReading]
      blind_to: [LightLevel]
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "umwelt block should parse: {:?}", result.err());
}

#[test]
fn test_m80_omni_sensory_default() {
    // A being with no umwelt block is omnisensory — no parse error
    let src = r#"
module OmniAgent
  being MantisShrimp
    telos: "detect all measurable signals"
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "being without umwelt should parse (omnisensory default): {:?}", result.err());
}

#[test]
fn test_m81_sense_declaration() {
    let src = r#"
module SignalOntology
  sense ElectromagneticSpectrum
    channels: [Gamma, XRay, UV, Visible, Infrared, Microwave, Radio]
    range: "1e-12m to 1e3m"
    unit: "nm"
  end

  sense AcousticRange
    channels: [Infrasound, Audible, Ultrasound]
    unit: "Hz"
  end

  sense ChemicalGradient
    channels: [pH, OxygenPartialPressure, GlucoseConcentration]
    unit: "mol/L"
  end

  sense QuantumState
    channels: [SpinUp, SpinDown, Superposition, Entangled]
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "sense declarations should parse: {:?}", result.err());
}

#[test]
fn test_m82_resonance_block() {
    let src = r#"
module SynapticAgent
  being ResonantBeing
    telos: "discover cross-signal correlations"
    end
    resonance:
      correlate: Temperature with Pressure via thermal_pressure_fn
      correlate: SpinState with ChemicalGradient
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "resonance block should parse: {:?}", result.err());
}

#[test]
fn test_m82_resonance_without_via() {
    let src = r#"
module CorrelationAgent
  being Observer
    telos: "find hidden patterns"
    end
    resonance:
      correlate: SignalA with SignalB
      correlate: SignalC with SignalD via cross_correlate
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "resonance without via should parse: {:?}", result.err());
}
