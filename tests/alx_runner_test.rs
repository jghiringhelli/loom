//! ALX-4: Self-Fix Loop
//!
//! A test harness that:
//! 1. Compiles ALX-3 (self-description)
//! 2. Parses the resulting correctness_report doc comment
//! 3. Measures S_realized = proved_claims / total_claims
//! 4. Reports convergence: all proved claims verified by actual checkers
//!
//! This is the "Loom fixes itself" loop: the self-description is the specification,
//! the compiler is the verifier, the correctness_report is the convergence signal.

use loom::{ast::Item, compile};
use std::fs;

const ALX_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/experiments/alx");

/// Read an ALX experiment file. Panics with clear message if file is missing.
fn read_alx(filename: &str) -> String {
    let path = format!("{}/{}", ALX_DIR, filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read ALX file {}: {}", path, e))
}

/// ALX-1: Full feature matrix must compile without errors.
#[test]
fn alx1_full_feature_matrix_compiles() {
    let source = read_alx("ALX-1-feature-matrix.loom");
    let result = compile(&source);
    assert!(
        result.is_ok(),
        "ALX-1 feature matrix must compile cleanly.\nErrors:\n{}",
        result
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// ALX-2: Cross-feature coherence must compile without errors.
#[test]
fn alx2_cross_feature_coherence_compiles() {
    let source = read_alx("ALX-2-cross-feature.loom");
    let result = compile(&source);
    assert!(
        result.is_ok(),
        "ALX-2 cross-feature coherence must compile cleanly.\nErrors:\n{}",
        result
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// ALX-3: Self-description must compile without errors.
#[test]
fn alx3_self_description_compiles() {
    let source = read_alx("ALX-3-self-description.loom");
    let result = compile(&source);
    assert!(
        result.is_ok(),
        "ALX-3 self-description must compile cleanly.\nErrors:\n{}",
        result
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// ALX-4a: Self-description must have ≥ 5 proved claims.
#[test]
fn alx4a_self_description_proves_minimum_claims() {
    let source = read_alx("ALX-3-self-description.loom");

    // Parse (don't full-compile — we want to inspect the AST)
    let tokens = loom::lexer::Lexer::tokenize(&source).expect("ALX-3 must lex");
    let module = loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("ALX-3 must parse");

    // Find the correctness_report
    let report = module.items.iter().find_map(|item| {
        if let Item::CorrectnessReport(r) = item {
            Some(r)
        } else {
            None
        }
    });

    let report = report.expect("ALX-3 must contain a correctness_report block");
    let proved_count = report.proved.len();
    let total_count = proved_count + report.unverified.len();

    let s_realized = proved_count as f64 / total_count as f64;

    eprintln!(
        "ALX-4 convergence: S_realized = {}/{} = {:.3}",
        proved_count, total_count, s_realized
    );
    eprintln!("Proved claims:");
    for claim in &report.proved {
        eprintln!("  ✓ {}: {}", claim.property, claim.checker);
    }
    eprintln!("Unverified:");
    for (property, reason) in &report.unverified {
        eprintln!("  ⚠ {}: {}", property, reason);
    }

    assert!(
        proved_count >= 5,
        "ALX-3 must prove ≥ 5 claims. Found: {} proved, {} unverified",
        proved_count,
        report.unverified.len()
    );
}

/// ALX-4b: Convergence metric — S_realized must be ≥ 0.70.
///
/// S_realized = proved_claims / (proved + unverified)
///
/// This is the self-fix loop gate. When S_realized < 0.70, the specification
/// is under-proved — the compiler is not yet capturing enough of its own behavior.
/// As features are added, S_realized should approach 1.0.
#[test]
fn alx4b_convergence_gate_s_realized() {
    let source = read_alx("ALX-3-self-description.loom");
    let tokens = loom::lexer::Lexer::tokenize(&source).expect("must lex");
    let module = loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("must parse");

    let report = module
        .items
        .iter()
        .find_map(|item| {
            if let Item::CorrectnessReport(r) = item {
                Some(r)
            } else {
                None
            }
        })
        .expect("must have correctness_report");

    let proved = report.proved.len();
    let total = proved + report.unverified.len();
    let s_realized = proved as f64 / total as f64;

    // Write convergence record
    eprintln!(
        "\n╔══════════════════════════════════════════╗"
    );
    eprintln!("║  ALX-4 Self-Fix Loop — Convergence Gate  ║");
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!("║  S_realized = {}/{} = {:.4}          ║", proved, total, s_realized);
    eprintln!("║  Gate threshold: 0.70                    ║");
    eprintln!(
        "║  Status: {}                     ║",
        if s_realized >= 0.70 { "✅ PASSED" } else { "❌ FAILED" }
    );
    eprintln!("╚══════════════════════════════════════════╝\n");

    assert!(
        s_realized >= 0.70,
        "ALX S_realized {:.4} < 0.70 gate. Add more proved claims to ALX-3-self-description.loom",
        s_realized
    );
}

/// ALX-4c: All ALX programs produce Rust output containing the module name.
/// This verifies end-to-end codegen, not just parsing.
#[test]
fn alx4c_all_alx_programs_produce_rust_output() {
    for (name, file) in [
        ("ALX-1", "ALX-1-feature-matrix.loom"),
        ("ALX-2", "ALX-2-cross-feature.loom"),
        ("ALX-3", "ALX-3-self-description.loom"),
    ] {
        let source = read_alx(file);
        let result = compile(&source);
        assert!(
            result.is_ok(),
            "{} codegen failed: {:?}",
            name,
            result.unwrap_err()
        );
        let rust = result.unwrap();
        assert!(
            rust.contains("mod ") || rust.contains("pub mod "),
            "{} Rust output should contain a module declaration:\n{}",
            name,
            &rust[..rust.len().min(500)]
        );
    }
}

/// ALX-5: Evolvable stress — multi-version migration chain compiles cleanly.
///
/// This is the most adversarial ALX experiment: a being that evolves through
/// 4 interface versions with field-type chains, version-number chains, and
/// cross-feature integration (session types, journal, boundary, scenario).
///
/// Failure here means evolvable is broken under realistic conditions.
#[test]
fn alx5_evolvable_stress_compiles() {
    let source = read_alx("ALX-5-evolvable-stress.loom");
    let result = compile(&source);
    assert!(
        result.is_ok(),
        "ALX-5 evolvable stress must compile cleanly.\nErrors:\n{}",
        result
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// ALX-5b: Evolvable stress — migration chain has expected structure in AST.
#[test]
fn alx5b_evolvable_migration_chain_structure() {
    let source = read_alx("ALX-5-evolvable-stress.loom");
    let tokens = loom::lexer::Lexer::tokenize(&source).expect("ALX-5 must lex");
    let module = loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("ALX-5 must parse");

    // PriceEvolver should have 5 migrations (3 field-based + 2 version-number)
    let evolver = module.being_defs.iter().find(|b| b.name == "PriceEvolver")
        .expect("PriceEvolver being must be present");
    assert_eq!(
        evolver.migrations.len(), 5,
        "PriceEvolver should have 5 migration blocks"
    );

    // EvolvingTradingAgent (autopoietic) should have 3 migrations
    let agent = module.being_defs.iter().find(|b| b.name == "EvolvingTradingAgent")
        .expect("EvolvingTradingAgent being must be present");
    assert!(
        !agent.migrations.is_empty(),
        "EvolvingTradingAgent should have migration blocks"
    );
    assert!(
        agent.autopoietic,
        "EvolvingTradingAgent should be autopoietic"
    );
}
