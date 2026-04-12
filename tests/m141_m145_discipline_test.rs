//! M141–M145: Forced disciplines — explicit `discipline` declarations.
//!
//! Covers:
//! - Parsing discipline blocks: CQRS, EventSourcing, DependencyInjection, CircuitBreaker, Saga, UnitOfWork
//! - Codegen: delegates to existing emit functions in disciplines.rs
//! - DisciplineChecker: 7 semantic rules
//!
//! Test naming convention: <rule_or_kind>_<scenario>

use loom::ast::Span;
use loom::ast::{DisciplineDecl, DisciplineKind, DisciplineParam, Item, Module};
use loom::checker::{DisciplineChecker, LoomChecker};
use loom::codegen::rust::RustEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

// ── helpers ──────────────────────────────────────────────────────────────────

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn make_module_with_discipline(dd: DisciplineDecl) -> Module {
    let tokens = Lexer::tokenize("module _Empty end").expect("lex failed");
    let mut m = Parser::new(&tokens).parse_module().expect("parse failed");
    m.items.clear();
    m.items.push(Item::Discipline(dd));
    m
}

fn make_module_with_disciplines(dds: Vec<DisciplineDecl>) -> Module {
    let tokens = Lexer::tokenize("module _Empty end").expect("lex failed");
    let mut m = Parser::new(&tokens).parse_module().expect("parse failed");
    m.items.clear();
    for dd in dds {
        m.items.push(Item::Discipline(dd));
    }
    m
}

fn disc(
    kind: DisciplineKind,
    target: &str,
    params: Vec<(String, DisciplineParam)>,
) -> DisciplineDecl {
    DisciplineDecl {
        kind,
        target: target.to_string(),
        params,
        span: Span::synthetic(),
    }
}

fn list_param(key: &str, items: &[&str]) -> (String, DisciplineParam) {
    (
        key.to_string(),
        DisciplineParam::List(items.iter().map(|s| s.to_string()).collect()),
    )
}

fn num_param(key: &str, n: i64) -> (String, DisciplineParam) {
    (key.to_string(), DisciplineParam::Number(n))
}

fn check(m: &Module) -> Vec<String> {
    DisciplineChecker::new()
        .check_module(m)
        .into_iter()
        .map(|e: loom::error::LoomError| e.to_string())
        .collect()
}

// ── parse tests ───────────────────────────────────────────────────────────────

#[test]
fn parse_discipline_cqrs_minimal() {
    let m = parse_module("module Orders\n  discipline CQRS for OrderStore end\nend");
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::Cqrs));
    assert_eq!(dd.target, "OrderStore");
}

#[test]
fn parse_discipline_event_sourcing_with_events() {
    let src = "module Orders\n  discipline EventSourcing for OrderStore\n    events: [OrderCreated, OrderShipped]\n  end\nend";
    let m = parse_module(src);
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::EventSourcing));
    assert!(dd.params.iter().any(|(k, _)| k == "events"));
}

#[test]
fn parse_discipline_circuit_breaker_with_params() {
    let src = "module Payments\n  discipline CircuitBreaker for PaymentService\n    max_attempts: 3\n  end\nend";
    let m = parse_module(src);
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::CircuitBreaker));
    assert!(dd.params.iter().any(|(k, _)| k == "max_attempts"));
}

#[test]
fn parse_discipline_saga_with_steps() {
    let src = "module Checkout\n  discipline Saga for CheckoutSaga\n    steps: [ReserveInventory, ChargePayment]\n  end\nend";
    let m = parse_module(src);
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::Saga));
    assert!(dd.params.iter().any(|(k, _)| k == "steps"));
}

#[test]
fn parse_discipline_di_with_binds() {
    let src = "module App\n  discipline DependencyInjection for AppContainer\n    binds: [IUserRepo, IEmailSender]\n  end\nend";
    let m = parse_module(src);
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::DependencyInjection));
}

#[test]
fn parse_discipline_unit_of_work() {
    let m = parse_module("module Billing\n  discipline UnitOfWork for BillingContext end\nend");
    let dd = match &m.items[0] {
        Item::Discipline(d) => d,
        _ => panic!("expected discipline"),
    };
    assert!(matches!(dd.kind, DisciplineKind::UnitOfWork));
}

// ── codegen tests ─────────────────────────────────────────────────────────────

#[test]
fn codegen_discipline_cqrs_emits_command_query() {
    let m = make_module_with_discipline(disc(DisciplineKind::Cqrs, "OrderStore", vec![]));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("Command") || code.contains("command"),
        "no Command in CQRS output:\n{code}"
    );
    assert!(
        code.contains("Query") || code.contains("query"),
        "no Query in CQRS output:\n{code}"
    );
}

#[test]
fn codegen_discipline_event_sourcing_emits_event() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::EventSourcing,
        "OrderStore",
        vec![list_param("events", &["OrderCreated", "OrderShipped"])],
    ));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("Event") || code.contains("event"),
        "no event type emitted:\n{code}"
    );
}

#[test]
fn codegen_discipline_circuit_breaker_emits_state() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::CircuitBreaker,
        "PaymentService",
        vec![num_param("max_attempts", 3)],
    ));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("Closed") || code.contains("circuit") || code.contains("CircuitBreaker"),
        "no circuit breaker state machine:\n{code}"
    );
}

#[test]
fn codegen_discipline_saga_emits_step() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::Saga,
        "CheckoutSaga",
        vec![list_param("steps", &["ReserveInventory", "ChargePayment"])],
    ));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("SagaStep") || code.contains("Saga") || code.contains("execute"),
        "no saga traits:\n{code}"
    );
}

#[test]
fn codegen_discipline_di_emits_container() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::DependencyInjection,
        "AppContainer",
        vec![list_param("binds", &["IUserRepo", "IEmailSender"])],
    ));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("Container") || code.contains("container") || code.contains("resolve"),
        "no DI container:\n{code}"
    );
}

#[test]
fn codegen_discipline_unit_of_work_emits_uow() {
    let m = make_module_with_discipline(disc(DisciplineKind::UnitOfWork, "BillingContext", vec![]));
    let code = RustEmitter::new().emit(&m);
    assert!(
        code.contains("commit") || code.contains("rollback") || code.contains("UnitOfWork"),
        "no UoW pattern:\n{code}"
    );
}

// ── checker: duplicates ───────────────────────────────────────────────────────

#[test]
fn checker_duplicate_kind_target_fails() {
    let m = make_module_with_disciplines(vec![
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
    ]);
    let errors = check(&m);
    assert!(!errors.is_empty(), "duplicate should be rejected");
    assert!(errors[0].contains("more than once"));
}

#[test]
fn checker_different_targets_no_duplicate_error() {
    let m = make_module_with_disciplines(vec![
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
        disc(DisciplineKind::Cqrs, "UserStore", vec![]),
    ]);
    let dup: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("more than once"))
        .collect();
    assert!(
        dup.is_empty(),
        "different targets should not trigger duplicate: {dup:?}"
    );
}

// ── checker: CircuitBreaker bounds ────────────────────────────────────────────

#[test]
fn checker_circuit_breaker_zero_attempts_fails() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::CircuitBreaker,
        "S",
        vec![num_param("max_attempts", 0)],
    ));
    let errors = check(&m);
    assert!(!errors.is_empty(), "max_attempts 0 should fail");
}

#[test]
fn checker_circuit_breaker_high_attempts_warns() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::CircuitBreaker,
        "S",
        vec![num_param("max_attempts", 999)],
    ));
    let errors = check(&m);
    assert!(!errors.is_empty(), "max_attempts 999 should warn");
    assert!(errors[0].contains("unreasonably") || errors[0].contains("high"));
}

#[test]
fn checker_circuit_breaker_valid_passes() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::CircuitBreaker,
        "S",
        vec![num_param("max_attempts", 3)],
    ));
    let cb: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("max_attempts"))
        .collect();
    assert!(cb.is_empty(), "valid circuit breaker should pass: {cb:?}");
}

// ── checker: EventSourcing completeness ───────────────────────────────────────

#[test]
fn checker_event_sourcing_no_events_warns() {
    let m = make_module_with_disciplines(vec![
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
        disc(DisciplineKind::EventSourcing, "OrderStore", vec![]),
    ]);
    let es: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("events"))
        .collect();
    assert!(
        !es.is_empty(),
        "event sourcing without events list should warn"
    );
}

#[test]
fn checker_event_sourcing_with_events_no_event_warn() {
    let m = make_module_with_disciplines(vec![
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
        disc(
            DisciplineKind::EventSourcing,
            "OrderStore",
            vec![list_param("events", &["OrderCreated"])],
        ),
    ]);
    let no_ev: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("no 'events:'"))
        .collect();
    assert!(
        no_ev.is_empty(),
        "event sourcing with events should not warn: {no_ev:?}"
    );
}

#[test]
fn checker_event_sourcing_without_cqrs_warns() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::EventSourcing,
        "OrderStore",
        vec![list_param("events", &["OrderCreated"])],
    ));
    let cqrs: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("CQRS"))
        .collect();
    assert!(!cqrs.is_empty(), "event sourcing without CQRS should warn");
}

#[test]
fn checker_event_sourcing_with_cqrs_no_cqrs_warn() {
    let m = make_module_with_disciplines(vec![
        disc(DisciplineKind::Cqrs, "OrderStore", vec![]),
        disc(
            DisciplineKind::EventSourcing,
            "OrderStore",
            vec![list_param("events", &["OrderCreated"])],
        ),
    ]);
    let cqrs: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("implies") && e.contains("CQRS"))
        .collect();
    assert!(
        cqrs.is_empty(),
        "event sourcing with CQRS should not warn: {cqrs:?}"
    );
}

// ── checker: DI completeness ──────────────────────────────────────────────────

#[test]
fn checker_di_no_binds_warns() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::DependencyInjection,
        "AppContainer",
        vec![],
    ));
    let di: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("binds"))
        .collect();
    assert!(!di.is_empty(), "DI without binds should warn");
}

#[test]
fn checker_di_with_binds_passes() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::DependencyInjection,
        "AppContainer",
        vec![list_param("binds", &["IUserRepo"])],
    ));
    let no_binds: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("no 'binds:'"))
        .collect();
    assert!(
        no_binds.is_empty(),
        "DI with binds should not warn: {no_binds:?}"
    );
}

// ── checker: Saga completeness ────────────────────────────────────────────────

#[test]
fn checker_saga_no_steps_warns() {
    let m = make_module_with_discipline(disc(DisciplineKind::Saga, "CheckoutSaga", vec![]));
    let saga: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("steps"))
        .collect();
    assert!(!saga.is_empty(), "saga without steps should warn");
}

#[test]
fn checker_saga_with_steps_passes() {
    let m = make_module_with_discipline(disc(
        DisciplineKind::Saga,
        "CheckoutSaga",
        vec![list_param("steps", &["StepA", "StepB"])],
    ));
    let no_steps: Vec<_> = check(&m)
        .into_iter()
        .filter(|e| e.contains("no 'steps:'"))
        .collect();
    assert!(
        no_steps.is_empty(),
        "saga with steps should not warn: {no_steps:?}"
    );
}

// ── checker: UnitOfWork + CQRS ────────────────────────────────────────────────

#[test]
fn checker_unit_of_work_no_errors() {
    let m = make_module_with_discipline(disc(DisciplineKind::UnitOfWork, "BillingContext", vec![]));
    let errors = check(&m);
    assert!(
        errors.is_empty(),
        "valid UnitOfWork should have no errors: {errors:?}"
    );
}

#[test]
fn checker_cqrs_no_errors() {
    let m = make_module_with_discipline(disc(DisciplineKind::Cqrs, "OrderStore", vec![]));
    let errors = check(&m);
    assert!(
        errors.is_empty(),
        "valid CQRS should have no errors: {errors:?}"
    );
}
