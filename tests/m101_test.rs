// M101: manifest: — Documentation Liveness Primitive.
//
// Tests for the manifest: block parser and ManifestChecker.

use loom::ast::*;
use loom::checker::ManifestChecker;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

fn parse_ok(src: &str) -> Module {
    parse(src).expect("expected successful parse")
}

// 1. manifest: block with a single artifact parses correctly.
#[test]
fn test_m101_manifest_parses() {
    let src = r#"
module Documented
  being Agent
    telos: "serve users"
    end
    manifest:
      artifact "README.md" reflects: [Agent] end
    end
  end
end
"#;
    let module = parse_ok(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    let manifest = being
        .manifest
        .as_ref()
        .expect("manifest block should be present");
    assert_eq!(manifest.artifacts.len(), 1);
    assert_eq!(manifest.artifacts[0].path, "README.md");
    assert_eq!(manifest.artifacts[0].reflects, vec!["Agent".to_string()]);
}

// 2. artifact pointing to a nonexistent file is a compile error.
#[test]
fn test_m101_missing_file_is_error() {
    // Build a module with a being that has a manifest pointing to a nonexistent file.
    let being = BeingDef {
        name: "Documented".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: None,
        regulate_blocks: vec![],
        evolve_block: None,
        epigenetic_blocks: vec![],
        morphogen_blocks: vec![],
        telomere: None,
        autopoietic: false,
        crispr_blocks: vec![],
        plasticity_blocks: vec![],
        canalization: None,
        senescence: None,
        criticality: None,
        umwelt: None,
        resonance: None,
        manifest: Some(ManifestBlock {
            artifacts: vec![ManifestArtifact {
                path: "/nonexistent/path/that/does/not/exist.md".to_string(),
                reflects: vec![],
                freshness: None,
                required_when: None,
            }],
            span: Span::synthetic(),
        }),
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
        cognitive_memory: None,
        signal_attention: None,
        role: None,
        relates_to: vec![],
        propagate_block: None,
        rewire_block: None,
        span: Span::synthetic(),
    };

    let module = Module {
        name: "Test".to_string(),
        describe: None,
        domains: vec![],
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        aspect_defs: vec![],
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };

    let errors = ManifestChecker::new().check(&module);
    assert!(
        !errors.is_empty(),
        "missing artifact file should produce an error"
    );
    let msgs: String = errors.iter().map(|e| format!("{}", e)).collect();
    assert!(
        msgs.contains("does not exist on disk"),
        "error should mention 'does not exist on disk', got: {msgs}"
    );
}

// 3. symbol in reflects: that doesn't exist in the module → warning, not error.
#[test]
fn test_m101_reflects_unknown_symbol_is_warning() {
    let being = BeingDef {
        name: "Documented".to_string(),
        describe: None,
        annotations: vec![],
        matter: None,
        form: None,
        function: None,
        telos: None,
        regulate_blocks: vec![],
        evolve_block: None,
        epigenetic_blocks: vec![],
        morphogen_blocks: vec![],
        telomere: None,
        autopoietic: false,
        crispr_blocks: vec![],
        plasticity_blocks: vec![],
        canalization: None,
        senescence: None,
        criticality: None,
        umwelt: None,
        resonance: None,
        manifest: Some(ManifestBlock {
            artifacts: vec![ManifestArtifact {
                // Use a file that actually exists so the file check passes.
                path: "README.md".to_string(),
                reflects: vec!["NonExistentSymbol".to_string()],
                freshness: None,
                required_when: None,
            }],
            span: Span::synthetic(),
        }),
        migrations: vec![],
        journal: None,
        scenarios: vec![],
        boundary: None,
        cognitive_memory: None,
        signal_attention: None,
        role: None,
        relates_to: vec![],
        propagate_block: None,
        rewire_block: None,
        span: Span::synthetic(),
    };

    let module = Module {
        name: "Test".to_string(),
        describe: None,
        domains: vec![],
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        aspect_defs: vec![],
        being_defs: vec![being],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };

    let errors = ManifestChecker::new().check(&module);
    // All errors should be warnings (prefixed with [warn]) — none are hard errors.
    let hard_errors: Vec<_> = errors
        .iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    assert!(
        hard_errors.is_empty(),
        "unknown symbol in reflects: should only produce warnings, got hard errors: {:?}",
        hard_errors
    );
    // There should be at least one warning.
    let warnings: Vec<_> = errors
        .iter()
        .filter(|e| format!("{}", e).contains("[warn]"))
        .collect();
    assert!(
        !warnings.is_empty(),
        "should have at least one [warn] for unknown symbol"
    );
}

// 4. Empty manifest: block (no artifacts) is valid.
#[test]
fn test_m101_empty_manifest_is_valid() {
    let src = r#"
module Empty
  being Agent
    telos: "serve"
    end
    manifest:
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    let manifest = being
        .manifest
        .as_ref()
        .expect("manifest block should be present");
    assert!(
        manifest.artifacts.is_empty(),
        "empty manifest: should have no artifacts"
    );
}

// 5. Two artifacts in one manifest: block parse correctly.
#[test]
fn test_m101_multiple_artifacts_parse() {
    let src = r#"
module Multi
  being Service
    telos: "process requests"
    end
    manifest:
      artifact "README.md" reflects: [Service] end
      artifact "docs/api.md" reflects: [Service] end
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    let manifest = being
        .manifest
        .as_ref()
        .expect("manifest block should be present");
    assert_eq!(
        manifest.artifacts.len(),
        2,
        "should have exactly 2 artifacts, got {}",
        manifest.artifacts.len()
    );
    assert_eq!(manifest.artifacts[0].path, "README.md");
    assert_eq!(manifest.artifacts[1].path, "docs/api.md");
}

// 6. A being: without manifest: is valid — manifest: is optional.
#[test]
fn test_m101_being_without_manifest_is_valid() {
    let src = r#"
module Plain
  being Organism
    telos: "survive"
    end
  end
end
"#;
    let module = parse_ok(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    assert!(
        being.manifest.is_none(),
        "being without manifest: should have None manifest"
    );

    // ManifestChecker should produce no errors for a being without a manifest block.
    let module_struct = Module {
        name: "Plain".to_string(),
        describe: None,
        domains: vec![],
        annotations: vec![],
        imports: vec![],
        spec: None,
        interface_defs: vec![],
        implements: vec![],
        provides: None,
        requires: None,
        invariants: vec![],
        test_defs: vec![],
        lifecycle_defs: vec![],
        temporal_defs: vec![],
        aspect_defs: vec![],
        being_defs: vec![module.being_defs[0].clone()],
        ecosystem_defs: vec![],
        flow_labels: vec![],
        items: vec![],
        span: Span::synthetic(),
    };
    let errors = ManifestChecker::new().check(&module_struct);
    assert!(
        errors.is_empty(),
        "being without manifest: should produce no errors"
    );
}
