//! M185 — `domain:` annotation on modules.
//!
//! `domain: label [label ...]` in a module header classifies the module into
//! one or more domain taxonomy buckets.  The domains are emitted as a
//! `/// @domain: ...` doc comment in the generated Rust.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── Parser — domain: field is populated ──────────────────────────────────────

/// A single domain label is parsed and emitted.
#[test]
fn m185_single_domain_emitted() {
    let src = r#"
module ClimateMonitor
  domain: climate

  fn observe :: () -> Float
    0.0
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("@domain: climate"),
        "expected @domain: climate in output, got:\n{}",
        out
    );
}

/// Multiple domain labels are all included.
#[test]
fn m185_multiple_domains_emitted() {
    let src = r#"
module MultiDomainSensor
  domain: climate energy epidemics

  fn read :: () -> Float
    0.0
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("@domain: climate, energy, epidemics"),
        "expected all domains joined with ', '"
    );
}

/// A module without `domain:` produces no @domain comment.
#[test]
fn m185_no_domain_no_comment() {
    let src = r#"
module SimpleMath

  fn add :: Int -> Int -> Int
    0
  end
end
"#;
    let out = compile(src);
    assert!(
        !out.contains("@domain:"),
        "no @domain comment expected when domain: is absent"
    );
}

/// `domain:` can appear after `describe:`.
#[test]
fn m185_domain_after_describe() {
    let src = r#"
module EnergyGrid
  describe: "Models an energy grid node"
  domain: energy materials

  fn voltage :: () -> Float
    0.0
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("@domain: energy, materials"), "expected domains");
    assert!(out.contains("Models an energy grid node"), "expected describe");
}

/// `domain:` labels become part of the module doc, before @annotations.
#[test]
fn m185_domain_before_annotations() {
    let src = r#"
module AntibioticTracker
  domain: epidemics antibiotics
  @since("2026")

  fn resistance_level :: () -> Float
    0.0
  end
end
"#;
    let out = compile(src);
    let domain_pos = out.find("@domain:").expect("@domain: expected");
    let since_pos = out.find("@since:").expect("@since: expected");
    assert!(
        domain_pos < since_pos,
        "@domain comment must appear before @since annotation"
    );
}
