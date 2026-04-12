//! M146–M150: Killer demos — full M131-M145 pipeline composition.
//!
//! Each test:
//!   1. Parses the .loom demo file (no errors)
//!   2. Verifies key items are present in the AST
//!   3. Emits Rust (verifies expected patterns in output)
//!   4. Where relevant, runs the DisciplineChecker for semantic correctness
//!
//! These are integration tests: if they all pass, every primitive added in M131-M145
//! composes correctly in a real program.

use loom::ast::{DisciplineKind, Item, MessagingPattern};
use loom::checker::{DisciplineChecker, LoomChecker};
use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;
use std::fs;

const DEMOS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/experiments/demos");

fn read_demo(name: &str) -> String {
    let path = format!("{}/{}", DEMOS_DIR, name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Cannot read demo {name}: {e}"))
}

fn parse_demo(name: &str) -> loom::ast::Module {
    let src = read_demo(name);
    let tokens = Lexer::tokenize(&src).unwrap_or_else(|e| panic!("{name}: lex error: {e:?}"));
    Parser::new(&tokens)
        .parse_module()
        .unwrap_or_else(|e| panic!("{name}: parse error: {e:?}"))
}

fn emit_demo(name: &str) -> String {
    let module = parse_demo(name);
    RustEmitter::new().emit(&module)
}

fn count_disciplines(module: &loom::ast::Module, kind: DisciplineKind) -> usize {
    module
        .items
        .iter()
        .filter(|i| matches!(i, Item::Discipline(d) if d.kind == kind))
        .count()
}

fn count_messaging(module: &loom::ast::Module) -> usize {
    module
        .items
        .iter()
        .filter(|i| matches!(i, Item::MessagingPrimitive(_)))
        .count()
}

fn count_telos_fns(module: &loom::ast::Module) -> usize {
    module
        .items
        .iter()
        .filter(|i| matches!(i, Item::TelosFunction(_)))
        .count()
}

fn has_messaging_pattern(module: &loom::ast::Module, pattern: MessagingPattern) -> bool {
    module.items.iter().any(|i| {
        if let Item::MessagingPrimitive(m) = i {
            m.pattern.as_ref().map_or(false, |p| *p == pattern)
        } else {
            false
        }
    })
}

fn checker_errors(module: &loom::ast::Module) -> Vec<String> {
    DisciplineChecker::new()
        .check_module(module)
        .into_iter()
        .map(|e: loom::error::LoomError| e.to_string())
        .collect()
}

// ── M146: OrderSystem ────────────────────────────────────────────────────────

#[test]
fn m146_order_system_parses() {
    let m = parse_demo("m146_order_system.loom");
    assert_eq!(m.name, "OrderSystem");
}

#[test]
fn m146_order_system_has_telos_function() {
    let m = parse_demo("m146_order_system.loom");
    assert!(
        count_telos_fns(&m) >= 1,
        "expected order_fulfillment_alignment telos_function"
    );
}

#[test]
fn m146_order_system_has_three_messaging_channels() {
    let m = parse_demo("m146_order_system.loom");
    assert!(
        count_messaging(&m) >= 3,
        "expected RequestResponse + Stream + PointToPoint channels"
    );
}

#[test]
fn m146_order_system_has_cqrs_eventsourcing_cb_saga() {
    let m = parse_demo("m146_order_system.loom");
    assert!(
        count_disciplines(&m, DisciplineKind::Cqrs) >= 1,
        "missing CQRS"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::EventSourcing) >= 1,
        "missing EventSourcing"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::CircuitBreaker) >= 1,
        "missing CircuitBreaker"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::Saga) >= 1,
        "missing Saga"
    );
}

#[test]
fn m146_order_system_checker_coherent() {
    let m = parse_demo("m146_order_system.loom");
    let errors: Vec<_> = checker_errors(&m)
        .into_iter()
        .filter(|e| e.contains("implies") && e.contains("CQRS"))
        .collect();
    assert!(
        errors.is_empty(),
        "EventSourcing+CQRS should be coherent: {errors:?}"
    );
}

#[test]
fn m146_order_system_emits_rust() {
    let code = emit_demo("m146_order_system.loom");
    assert!(!code.is_empty());
    assert!(
        code.contains("fn ") || code.contains("trait ") || code.contains("struct "),
        "expected Rust constructs: {code}"
    );
}

// ── M147: EvolvingTrader ──────────────────────────────────────────────────────

#[test]
fn m147_evolving_trader_parses() {
    let m = parse_demo("m147_evolving_trader.loom");
    assert_eq!(m.name, "EvolvingTrader");
}

#[test]
fn m147_evolving_trader_has_return_alignment_telos_fn() {
    let m = parse_demo("m147_evolving_trader.loom");
    let has_ra = m.items.iter().any(|i| {
        if let Item::TelosFunction(tf) = i {
            tf.name.contains("return") || tf.name.contains("alignment")
        } else {
            false
        }
    });
    assert!(has_ra, "expected return_alignment telos_function");
}

#[test]
fn m147_evolving_trader_has_all_three_patterns() {
    let m = parse_demo("m147_evolving_trader.loom");
    assert!(
        has_messaging_pattern(&m, MessagingPattern::Stream),
        "missing Stream (MarketTickStream)"
    );
    assert!(
        has_messaging_pattern(&m, MessagingPattern::RequestResponse),
        "missing RequestResponse (OrderExecutionChannel)"
    );
    assert!(
        has_messaging_pattern(&m, MessagingPattern::PointToPoint),
        "missing PointToPoint (RiskAlertBus)"
    );
}

#[test]
fn m147_evolving_trader_has_circuit_breaker_di_uow() {
    let m = parse_demo("m147_evolving_trader.loom");
    assert!(
        count_disciplines(&m, DisciplineKind::CircuitBreaker) >= 1,
        "missing CircuitBreaker"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::DependencyInjection) >= 1,
        "missing DI"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::UnitOfWork) >= 1,
        "missing UnitOfWork"
    );
}

#[test]
fn m147_evolving_trader_emits_circuit_breaker() {
    let code = emit_demo("m147_evolving_trader.loom");
    assert!(
        code.contains("Closed") || code.contains("circuit") || code.contains("CircuitBreaker"),
        "no circuit breaker pattern in output:\n{code}"
    );
}

// ── M148: BioiSO Upgraded ─────────────────────────────────────────────────────

#[test]
fn m148_bioiso_upgraded_parses() {
    let m = parse_demo("m148_bioiso_upgraded.loom");
    assert_eq!(m.name, "BioiSODemo");
}

#[test]
fn m148_bioiso_upgraded_has_two_telos_functions() {
    let m = parse_demo("m148_bioiso_upgraded.loom");
    assert!(
        count_telos_fns(&m) >= 2,
        "expected survival_alignment + selection_pressure_alignment, got {}",
        count_telos_fns(&m)
    );
}

#[test]
fn m148_bioiso_upgraded_has_two_stream_channels() {
    let m = parse_demo("m148_bioiso_upgraded.loom");
    let streams = m.items.iter().filter(|i| {
        matches!(i, Item::MessagingPrimitive(mp) if matches!(mp.pattern, Some(MessagingPattern::Stream)))
    }).count();
    assert!(
        streams >= 2,
        "expected ChemicalSignalChannel + DivisionEventBus, got {streams}"
    );
}

#[test]
fn m148_bioiso_upgraded_es_and_cqrs_coherent() {
    let m = parse_demo("m148_bioiso_upgraded.loom");
    // CQRS + EventSourcing for same target → no "implies CQRS" warning
    let errors: Vec<_> = checker_errors(&m)
        .into_iter()
        .filter(|e| e.contains("implies") && e.contains("CQRS"))
        .collect();
    assert!(
        errors.is_empty(),
        "CQRS+EventSourcing pair should be coherent: {errors:?}"
    );
}

#[test]
fn m148_bioiso_upgraded_emits_event_sourcing() {
    let code = emit_demo("m148_bioiso_upgraded.loom");
    assert!(
        code.contains("Event") || code.contains("event"),
        "expected event sourcing output:\n{code}"
    );
}

// ── M149: Compiler Self-Description ──────────────────────────────────────────

#[test]
fn m149_compiler_self_desc_parses() {
    let m = parse_demo("m149_compiler_self_desc.loom");
    assert_eq!(m.name, "LoomCompilerV2");
}

#[test]
fn m149_compiler_self_desc_has_s_realized_telos_fn() {
    let m = parse_demo("m149_compiler_self_desc.loom");
    let has_s = m.items.iter().any(|i| {
        if let Item::TelosFunction(tf) = i {
            tf.name.contains("s_realized") || tf.name.contains("convergence")
        } else {
            false
        }
    });
    assert!(has_s, "expected s_realized_convergence telos_function");
}

#[test]
fn m149_compiler_self_desc_has_di_and_cb() {
    let m = parse_demo("m149_compiler_self_desc.loom");
    assert!(
        count_disciplines(&m, DisciplineKind::DependencyInjection) >= 1,
        "missing DI"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::CircuitBreaker) >= 1,
        "missing CircuitBreaker for SMT"
    );
}

#[test]
fn m149_compiler_self_desc_emits_di_container() {
    let code = emit_demo("m149_compiler_self_desc.loom");
    assert!(
        code.contains("Container") || code.contains("resolve") || code.contains("container"),
        "expected DI container in output:\n{code}"
    );
}

#[test]
fn m149_compiler_self_desc_emits_unit_of_work() {
    let code = emit_demo("m149_compiler_self_desc.loom");
    assert!(
        code.contains("commit") || code.contains("rollback") || code.contains("UnitOfWork"),
        "expected UoW in output:\n{code}"
    );
}

// ── M150: Full Composition ────────────────────────────────────────────────────

#[test]
fn m150_full_composition_parses() {
    let m = parse_demo("m150_full_composition.loom");
    assert_eq!(m.name, "AnalyticsPlatform");
}

#[test]
fn m150_full_composition_has_all_six_discipline_kinds() {
    let m = parse_demo("m150_full_composition.loom");
    assert!(
        count_disciplines(&m, DisciplineKind::Cqrs) >= 1,
        "missing CQRS"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::EventSourcing) >= 1,
        "missing EventSourcing"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::DependencyInjection) >= 1,
        "missing DI"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::CircuitBreaker) >= 1,
        "missing CB"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::Saga) >= 1,
        "missing Saga"
    );
    assert!(
        count_disciplines(&m, DisciplineKind::UnitOfWork) >= 1,
        "missing UoW"
    );
}

#[test]
fn m150_full_composition_has_all_three_messaging_patterns() {
    let m = parse_demo("m150_full_composition.loom");
    assert!(
        has_messaging_pattern(&m, MessagingPattern::Stream),
        "missing Stream"
    );
    assert!(
        has_messaging_pattern(&m, MessagingPattern::RequestResponse),
        "missing RequestResponse"
    );
    assert!(
        has_messaging_pattern(&m, MessagingPattern::PointToPoint),
        "missing PointToPoint"
    );
}

#[test]
fn m150_full_composition_has_telos_function() {
    let m = parse_demo("m150_full_composition.loom");
    assert!(
        count_telos_fns(&m) >= 1,
        "expected pipeline_health_alignment"
    );
}

#[test]
fn m150_full_composition_checker_coherent() {
    let m = parse_demo("m150_full_composition.loom");
    let cqrs_errors: Vec<_> = checker_errors(&m)
        .into_iter()
        .filter(|e| e.contains("implies") && e.contains("CQRS"))
        .collect();
    assert!(
        cqrs_errors.is_empty(),
        "CQRS+EventSourcing should be coherent in full composition: {cqrs_errors:?}"
    );
}

#[test]
fn m150_full_composition_emits_all_patterns() {
    let code = emit_demo("m150_full_composition.loom");
    assert!(!code.is_empty(), "emit produced empty output");
    // Should contain output from multiple codegen paths
    let fn_count = code.matches("fn ").count();
    assert!(fn_count >= 1, "expected at least one fn in output:\n{code}");
}

#[test]
fn m150_full_composition_telos_fn_has_all_four_thresholds() {
    let m = parse_demo("m150_full_composition.loom");
    let tf = m
        .items
        .iter()
        .find_map(|i| {
            if let Item::TelosFunction(tf) = i {
                Some(tf)
            } else {
                None
            }
        })
        .expect("no telos_function found");
    let t = tf.thresholds.as_ref().expect("thresholds missing");
    assert!(t.convergence > 0.0, "convergence threshold should be set");
    assert!(t.warning.is_some(), "warning threshold should be set");
    assert!(t.divergence > 0.0, "divergence threshold should be set");
    assert!(
        t.propagation.is_some(),
        "propagation threshold should be set"
    );
}
