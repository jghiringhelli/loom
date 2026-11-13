//! M4 — Negative-case suite (docs/maturity-experiments.md).
//!
//! One table per checker in `src/checker/`. Every `Neg` row is a program that
//! MUST be rejected with the expected `LoomError` variant (`kind`) and a message
//! substring. Every `Pos` row is a near-miss valid program that MUST be accepted
//! (guards against over-rejection).
//!
//! Rows run the checker directly on the parsed module (so an earlier pipeline
//! stage cannot mask the checker under test). Rows for checkers wired as hard
//! stages in `loom::compile` are additionally confirmed through `compile()`.
//! Some rules guard states the parser cannot produce (e.g. an empty string);
//! those rows carry a `mutate` hook that edits the parsed AST.
//!
//! Soundness gaps (program accepted although it should be rejected) are kept as
//! separate `#[ignore = "soundness gap: ..."]` tests and listed in
//! `experiments/maturity/evidence/m4/findings.md`.

#![allow(clippy::type_complexity)]
#![allow(dead_code)]

use loom::ast::*;
use loom::checker::*;
use loom::error::LoomError;
use loom::lexer::Lexer;
use loom::parser::Parser;

type Mutator = fn(&mut Module);

struct Neg {
    name: &'static str,
    src: &'static str,
    mutate: Option<Mutator>,
    kind: &'static str,
    msg: &'static str,
}

struct Pos {
    name: &'static str,
    src: &'static str,
}

fn n(name: &'static str, src: &'static str, kind: &'static str, msg: &'static str) -> Neg {
    Neg {
        name,
        src,
        mutate: None,
        kind,
        msg,
    }
}

fn nm(
    name: &'static str,
    src: &'static str,
    mutate: Mutator,
    kind: &'static str,
    msg: &'static str,
) -> Neg {
    Neg {
        name,
        src,
        mutate: Some(mutate),
        kind,
        msg,
    }
}

fn p(name: &'static str, src: &'static str) -> Pos {
    Pos { name, src }
}

fn parse(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).unwrap_or_else(|e| panic!("lex failed: {e:?}\n{src}"));
    Parser::new(&tokens)
        .parse_module()
        .unwrap_or_else(|e| panic!("parse failed: {e:?}\n{src}"))
}

type Run = dyn Fn(&Module) -> Vec<LoomError>;

fn errs_of(run: &Run, row: &Neg) -> Vec<LoomError> {
    let mut m = parse(row.src);
    if let Some(f) = row.mutate {
        f(&mut m);
    }
    run(&m)
}

fn rejected_as_expected(errs: &[LoomError], row: &Neg) -> bool {
    errs.iter()
        .any(|e| e.kind() == row.kind && e.to_string().contains(row.msg))
}

/// Run every negative row; collect all failures and panic once with the list.
fn run_neg(checker: &str, hard: bool, run: &Run, rows: &[Neg]) {
    assert!(
        rows.len() >= 5,
        "{checker}: M4 requires >= 5 must-fail programs"
    );
    let mut failures = Vec::new();
    for row in rows {
        let errs = errs_of(run, row);
        if !rejected_as_expected(&errs, row) {
            failures.push(format!(
                "[{checker}/{}] expected {} containing {:?}, got {:?}",
                row.name, row.kind, row.msg, errs
            ));
            continue;
        }
        // A matching diagnostic carrying a suppressible prefix is a warning, not a
        // rejection: the pipeline filters it, so `compile` is allowed to accept.
        let only_soft = errs
            .iter()
            .filter(|e| e.kind() == row.kind && e.to_string().contains(row.msg))
            .all(|e| {
                let s = e.to_string();
                s.contains("[warn]") || s.contains("[info]") || s.contains("[hint]")
            });
        if only_soft {
            eprintln!("[{checker}/{}] diagnostic-only (warning-level) row", row.name);
        } else if hard && row.mutate.is_none() && loom::compile(row.src).is_ok() {
            failures.push(format!(
                "[{checker}/{}] checker rejects but loom::compile accepts",
                row.name
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Near-miss valid programs: no diagnostic may survive `suppress`.
fn run_pos(checker: &str, suppress: &[&str], run: &Run, rows: &[Pos]) {
    let mut failures = Vec::new();
    for row in rows {
        let m = parse(row.src);
        let errs: Vec<LoomError> = run(&m)
            .into_iter()
            .filter(|e| {
                let s = e.to_string();
                suppress.iter().all(|pre| !s.contains(pre))
            })
            .collect();
        if !errs.is_empty() {
            failures.push(format!(
                "[{checker}/{}] wrongly rejected: {:?}",
                row.name, errs
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Single-row runner used by `#[ignore]`d soundness-gap tests.
fn assert_rejected(run: &Run, row: Neg) {
    let errs = errs_of(run, &row);
    assert!(
        rejected_as_expected(&errs, &row),
        "ACCEPTED/mismatch: expected {} containing {:?}, got {:?}",
        row.kind,
        row.msg,
        errs
    );
}

macro_rules! chk {
    ($t:expr) => {
        &|m: &Module| $t.check_module(m)
    };
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. AlgebraicChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_algebraic() {
    run_neg(
        "algebraic",
        true,
        chk!(AlgebraicChecker::new()),
        &[
            n(
                "commutative_one_param",
                "module D\n fn f @commutative :: Int -> Int\n todo\n end\nend",
                "TypeError",
                "commutative requires at least 2 parameters",
            ),
            n(
                "commutative_zero_param",
                "module D\n fn f @commutative :: Int\n todo\n end\nend",
                "TypeError",
                "commutative requires at least 2 parameters",
            ),
            n(
                "at_most_once_plus_exactly_once",
                "module D\n fn f @at-most-once @exactly-once :: String -> Effect<[IO], String>\n a\n end\nend",
                "TypeError",
                "conflicting multiplicity annotations",
            ),
            n(
                "exactly_once_on_pure_fn",
                "module D\n fn f @exactly-once :: Int -> Int\n n\n end\nend",
                "TypeError",
                "exactly-once requires an effectful function",
            ),
            n(
                "idempotent_plus_exactly_once",
                "module D\n fn f @idempotent @exactly-once :: Int -> Effect<[Payment], String>\n a\n end\nend",
                "TypeError",
                "idempotent and exactly-once are contradictory",
            ),
        ],
    );
}

#[test]
fn pos_algebraic() {
    run_pos(
        "algebraic",
        &[],
        chk!(AlgebraicChecker::new()),
        &[
            p(
                "commutative_two_params",
                "module D\n fn f @commutative :: Int -> Int -> Int\n todo\n end\nend",
            ),
            p(
                "exactly_once_effectful",
                "module D\n fn f @exactly-once :: Int -> Effect<[IO], String>\n a\n end\nend",
            ),
            p(
                "idempotent_alone",
                "module D\n fn f @idempotent :: Int -> Effect<[IO], String>\n a\n end\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. AspectChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_aspect() {
    run_neg(
        "aspect",
        true,
        chk!(AspectChecker::new()),
        &[
            n(
                "duplicate_order",
                "module M\n fn v :: String -> Bool\n true\n end\n fn l :: String -> Bool\n false\n end\n aspect A\n pointcut: fn where @auth\n before: v\n order: 1\n end\n aspect B\n pointcut: fn where @audit\n after: l\n order: 1\n end\nend",
                "TypeError",
                "duplicate aspect order",
            ),
            n(
                "undeclared_before_advice",
                "module M\n aspect A\n before: ghost\n order: 1\n end\nend",
                "TypeError",
                "advice function `ghost` is not declared",
            ),
            n(
                "undeclared_around_advice",
                "module M\n aspect A\n around: ghost_wrap\n end\nend",
                "TypeError",
                "advice function `ghost_wrap` is not declared",
            ),
            n(
                "undeclared_on_failure",
                "module M\n aspect R\n on_failure: nobody\n max_attempts: 3\n end\nend",
                "TypeError",
                "on_failure handler `nobody` is not declared",
            ),
            n(
                "requires_missing_aspect",
                "module M\n fn f @requires_aspect(\"Audit\") :: Int -> Int\n n\n end\nend",
                "TypeError",
                "requires aspect `Audit` but no such aspect is declared",
            ),
            n(
                "duplicate_annotation_param",
                "module M\n annotation ct(a: String, a: String)\nend",
                "TypeError",
                "duplicate parameter name `a`",
            ),
            n(
                "two_correctness_reports",
                "module M\n correctness_report:\n end\n correctness_report:\n end\nend",
                "TypeError",
                "at most one is allowed",
            ),
            n(
                "duplicate_proved_claim",
                "module M\n correctness_report:\n proved:\n - homeostasis: a\n - homeostasis: b\n end\nend",
                "TypeError",
                "duplicate proved claim `homeostasis`",
            ),
        ],
    );
}

#[test]
fn pos_aspect() {
    run_pos(
        "aspect",
        &[],
        chk!(AspectChecker::new()),
        &[
            p(
                "distinct_orders",
                "module M\n fn v :: String -> Bool\n true\n end\n aspect A\n before: v\n order: 1\n end\n aspect B\n after: v\n order: 2\n end\nend",
            ),
            p(
                "requires_declared_aspect",
                "module M\n fn v :: String -> Bool\n true\n end\n aspect Audit\n before: v\n end\n fn f @requires_aspect(\"Audit\") :: Int -> Int\n n\n end\nend",
            ),
            p(
                "one_report_distinct_claims",
                "module M\n correctness_report:\n proved:\n - a: x\n - b: y\n end\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. TypeChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_types() {
    run_neg(
        "types",
        true,
        chk!(TypeChecker::new()),
        &[
            n(
                "undeclared_dependency_no_requires",
                "module M\nfn find :: Int -> String\nwith db\n \"found\"\nend\nend",
                "UndeclaredDependency",
                "db",
            ),
            n(
                "dependency_not_in_requires",
                "module M\nrequires { db: DbConn }\nfn fetch :: Int -> String\nwith cache\n \"x\"\nend\nend",
                "UndeclaredDependency",
                "cache",
            ),
            n(
                "unknown_pattern_variant",
                "module M\nenum Color = | Red | Green end\nfn f :: Int -> Int\n match x\n | Purple -> 1\n | Red -> 2\n end\nend\nend",
                "TypeError",
                "unknown variant or type in pattern: `Purple`",
            ),
            n(
                "implements_missing_method",
                "module M\ninterface Greeter\n fn greet :: String -> String\nend\nimplements Greeter\nfn unrelated :: Int\n 1\nend\nend",
                "TypeError",
                "missing method 'greet'",
            ),
            n(
                "implements_signature_mismatch",
                "module M\ninterface Greeter\n fn greet :: String -> String\nend\nimplements Greeter\nfn greet :: Int -> Int\n 1\nend\nend",
                "TypeError",
                "signature mismatch for interface 'Greeter'",
            ),
            n(
                "unknown_nested_pattern_variant",
                "module M\nenum Opt = | Some | Nothing end\nfn f :: Int -> Int\n match x\n | Some(Bogus) -> 1\n | Nothing -> 0\n end\nend\nend",
                "TypeError",
                "unknown variant or type in pattern: `Bogus`",
            ),
        ],
    );
}

#[test]
fn pos_types() {
    run_pos(
        "types",
        &[],
        chk!(TypeChecker::new()),
        &[
            p(
                "declared_dependency",
                "module M\nrequires { db: DbConn }\nfn fetch :: Int -> String\nwith db\n \"x\"\nend\nend",
            ),
            p(
                "known_variants",
                "module M\nenum Color = | Red | Green end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | Green -> 2\n end\nend\nend",
            ),
            p(
                "conforming_implements",
                "module M\ninterface Greeter\n fn greet :: String -> String\nend\nimplements Greeter\nfn greet :: String -> String\n name\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. InferenceEngine
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_infer() {
    run_neg(
        "infer",
        true,
        chk!(InferenceEngine::new()),
        &[
            n(
                "int_body_bool_return",
                "module M\nfn bad :: Int -> Bool\n n + 1\nend\nend",
                "UnificationError",
                "type mismatch",
            ),
            n(
                "int_plus_bool",
                "module M\nfn bad :: Int -> Int\n n + true\nend\nend",
                "UnificationError",
                "type mismatch",
            ),
            n(
                "int_literal_for_bool",
                "module M\nfn bad :: Int -> Bool\n 42\nend\nend",
                "UnificationError",
                "type mismatch",
            ),
            n(
                "bool_literal_for_int",
                "module M\nfn bad :: Int -> Int\n true\nend\nend",
                "UnificationError",
                "type mismatch",
            ),
            n(
                "string_literal_for_int",
                "module M\nfn bad :: Int -> Int\n \"hi\"\nend\nend",
                "UnificationError",
                "type mismatch",
            ),
        ],
    );
}

#[test]
fn pos_infer() {
    run_pos(
        "infer",
        &[],
        chk!(InferenceEngine::new()),
        &[
            p("int_body", "module M\nfn d :: Int -> Int\n n + 1\nend\nend"),
            p("comparison_is_bool", "module M\nfn d :: Int -> Bool\n n > 0\nend\nend"),
            p("bool_literal", "module M\nfn d :: Int -> Bool\n true\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. ExhaustivenessChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_exhaustiveness() {
    run_neg(
        "exhaustiveness",
        true,
        chk!(ExhaustivenessChecker::new()),
        &[
            n(
                "missing_one_variant",
                "module M\nenum Color = | Red | Green | Blue end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | Green -> 2\n end\nend\nend",
                "NonExhaustiveMatch",
                "Blue",
            ),
            n(
                "missing_many_variants",
                "module M\nenum Dir = | North | South | East | West end\nfn f :: Int -> Int\n match d\n | North -> 0\n end\nend\nend",
                "NonExhaustiveMatch",
                "West",
            ),
            n(
                "guarded_arm_not_a_cover",
                "module M\nenum Color = | Red | Green | Blue end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | Green -> 2\n | Blue if cond -> 3\n end\nend\nend",
                "NonExhaustiveMatch",
                "Blue",
            ),
            n(
                "guarded_wildcard_not_total",
                "module M\nenum Color = | Red | Green | Blue end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | Green -> 2\n | _ if cond -> 0\n end\nend\nend",
                "NonExhaustiveMatch",
                "Blue",
            ),
            n(
                "single_arm_two_variant_enum",
                "module M\nenum Flag = | On | Off end\nfn f :: Int -> Int\n match x\n | Off -> 0\n end\nend\nend",
                "NonExhaustiveMatch",
                "On",
            ),
        ],
    );
}

#[test]
fn pos_exhaustiveness() {
    run_pos(
        "exhaustiveness",
        &[],
        chk!(ExhaustivenessChecker::new()),
        &[
            p(
                "all_variants",
                "module M\nenum Color = | Red | Green | Blue end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | Green -> 2\n | Blue -> 3\n end\nend\nend",
            ),
            p(
                "variable_arm_total",
                "module M\nenum Color = | Red | Green | Blue end\nfn f :: Int -> Int\n match x\n | Red -> 1\n | other -> 0\n end\nend\nend",
            ),
            p(
                "literal_match_not_checked",
                "module M\nfn f :: Int -> Int\n match n\n | 1 -> 10\n | 2 -> 20\n end\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. EffectChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_effects() {
    run_neg(
        "effects",
        true,
        chk!(EffectChecker::new()),
        &[
            n(
                "pure_calls_effectful",
                "module M\nfn fetch :: Int -> Effect<[IO], String>\n todo\nend\nfn orch :: Int -> String\n fetch(1)\nend\nend",
                "EffectError",
                "pure function `orch` calls effectful",
            ),
            n(
                "pure_calls_transitive_effectful",
                "module M\nfn read_db :: Int -> Effect<[DB], String>\n todo\nend\nfn transform :: Int -> Effect<[DB], String>\n read_db(0)\nend\nfn run :: Int -> String\n transform(1)\nend\nend",
                "EffectError",
                "pure function `run` calls effectful",
            ),
            n(
                "effect_not_declared",
                "module M\nfn send_mail :: Int -> Effect<[IO], Bool>\n todo\nend\nfn both :: Int -> Effect<[DB], Bool>\n send_mail(0)\nend\nend",
                "EffectError",
                "uses effect `IO` but does not declare it",
            ),
            n(
                "at_pure_annotation_calls_effectful",
                "module M\nfn save :: Effect<[IO], Int>\n 42\nend\nfn compute\n @pure\n :: Int\n save()\nend\nend",
                "EffectError",
                "@pure function `compute`",
            ),
            n(
                "irreversible_from_reversible",
                "module M\nfn send_email :: Effect<[IO@irreversible], Int>\n 42\nend\nfn save_draft :: Effect<[IO@reversible], Int>\n send_email()\nend\nend",
                "EffectError",
                "more severe consequence tier",
            ),
        ],
    );
}

#[test]
fn pos_effects() {
    run_pos(
        "effects",
        &[],
        chk!(EffectChecker::new()),
        &[
            p(
                "superset_of_effects",
                "module M\nfn w :: Int -> Effect<[DB], Bool>\n todo\nend\nfn d :: Int -> Effect<[IO, DB], Bool>\n w(0)\nend\nend",
            ),
            p(
                "same_tier",
                "module M\nfn a :: Effect<[IO@irreversible], Int>\n 42\nend\nfn b :: Effect<[IO@irreversible], Int>\n a()\nend\nend",
            ),
            p(
                "pure_calls_pure",
                "module M\nfn a :: Int -> Int\n 0\nend\nfn b :: Int -> Int\n a(0)\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. CanalizationChecker
// ═════════════════════════════════════════════════════════════════════════════

const CANAL_OK: &str = "module M\nbeing Cell\n telos: \"differentiate\"\n end\n canalize:\n toward: differentiated_state\n despite: [noise, heat]\n end\nend\nend";
const CANAL_TWO: &str = "module M\nbeing A\n telos: \"a\"\n end\n canalize:\n toward: x\n despite: [n]\n end\nend\nbeing B\n telos: \"b\"\n end\n canalize:\n toward: y\n despite: [n]\n end\nend\nend";

#[test]
fn neg_canalization() {
    run_neg(
        "canalization",
        true,
        chk!(CanalizationChecker::new()),
        &[
            nm(
                "empty_toward",
                CANAL_OK,
                |m| m.being_defs[0].canalization.as_mut().unwrap().toward.clear(),
                "ParseError",
                "canalize block has empty toward",
            ),
            n(
                "empty_despite_list_literal",
                "module M\nbeing Cell\n telos: \"t\"\n end\n canalize:\n toward: x\n despite: []\n end\nend\nend",
                "ParseError",
                "must list at least one despite perturbation",
            ),
            nm(
                "empty_despite_mutated",
                CANAL_OK,
                |m| m.being_defs[0].canalization.as_mut().unwrap().despite.clear(),
                "ParseError",
                "must list at least one despite",
            ),
            nm(
                "second_being_empty_toward",
                CANAL_TWO,
                |m| m.being_defs[1].canalization.as_mut().unwrap().toward.clear(),
                "ParseError",
                "being 'B': canalize block has empty toward",
            ),
            nm(
                "second_being_empty_despite",
                CANAL_TWO,
                |m| m.being_defs[1].canalization.as_mut().unwrap().despite.clear(),
                "ParseError",
                "being 'B'",
            ),
        ],
    );
}

#[test]
fn pos_canalization() {
    run_pos(
        "canalization",
        &[],
        chk!(CanalizationChecker::new()),
        &[
            p("single_perturbation", "module M\nbeing C\n telos: \"t\"\n end\n canalize:\n toward: x\n despite: [noise]\n end\nend\nend"),
            p("with_convergence_proof", "module M\nbeing C\n telos: \"t\"\n end\n canalize:\n toward: x\n despite: [noise]\n convergence_proof: lyapunov\n end\nend\nend"),
            p("no_canalize_block", "module M\nbeing C\n telos: \"t\"\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. CategoryChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_category() {
    run_neg(
        "category",
        true,
        chk!(CategoryChecker::new()),
        &[
            n(
                "functor_one_law",
                "module T\nfunctor F<A>\nlaw: identity\nend\nend",
                "TypeError",
                "functor `F` must declare at least 2 laws",
            ),
            n(
                "functor_zero_laws",
                "module T\nfunctor F<A>\nend\nend",
                "TypeError",
                "but declares 0",
            ),
            n(
                "monad_two_laws",
                "module T\nmonad Maybe<A>\nlaw: left_identity\nlaw: right_identity\nend\nend",
                "TypeError",
                "missing required law `associativity`",
            ),
            n(
                "monad_zero_laws",
                "module T\nmonad M<A>\nend\nend",
                "TypeError",
                "monad `M` must declare 3 laws",
            ),
            n(
                "monad_three_laws_wrong_names",
                "module T\nmonad M<A>\nlaw: left_identity\nlaw: identity\nlaw: composition\nend\nend",
                "TypeError",
                "missing required law `right_identity`",
            ),
        ],
    );
}

#[test]
fn pos_category() {
    run_pos(
        "category",
        &[],
        chk!(CategoryChecker::new()),
        &[
            p("functor_two_laws", "module T\nfunctor F<A>\nlaw: identity\nlaw: composition\nend\nend"),
            p("monad_three_laws", "module T\nmonad M<A>\nlaw: left_identity\nlaw: right_identity\nlaw: associativity\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 9. CheckpointChecker
// ═════════════════════════════════════════════════════════════════════════════

const CKPT_OK: &str = "module M\nlifecycle Conn :: Pending -> Active -> Closed\ncheckpoint: Ready\nrequires: is_ready\non_fail: abort\nend\ncheckpoint: Auth\nrequires: is_auth\non_fail: deny\nend\nend\nend";

#[test]
fn neg_checkpoint() {
    run_neg(
        "checkpoint",
        true,
        chk!(CheckpointChecker::new()),
        &[
            n(
                "duplicate_checkpoint_name",
                "module M\nlifecycle Conn :: P -> A -> C\ncheckpoint: Ready\nrequires: r1\non_fail: f1\nend\ncheckpoint: Ready\nrequires: r2\non_fail: f2\nend\nend\nend",
                "ParseError",
                "duplicate checkpoint name 'Ready'",
            ),
            nm(
                "empty_requires",
                CKPT_OK,
                |m| m.lifecycle_defs[0].checkpoints[0].requires.clear(),
                "ParseError",
                "requires must be non-empty",
            ),
            nm(
                "empty_on_fail",
                CKPT_OK,
                |m| m.lifecycle_defs[0].checkpoints[1].on_fail.clear(),
                "ParseError",
                "on_fail must be non-empty",
            ),
            nm(
                "second_checkpoint_renamed_duplicate",
                CKPT_OK,
                |m| m.lifecycle_defs[0].checkpoints[1].name = "Ready".to_string(),
                "ParseError",
                "duplicate checkpoint name",
            ),
            n(
                "non_adjacent_duplicate",
                "module M\nlifecycle Conn :: P -> A -> C\ncheckpoint: A1\nrequires: r\non_fail: f\nend\ncheckpoint: B1\nrequires: r\non_fail: f\nend\ncheckpoint: A1\nrequires: r\non_fail: f\nend\nend\nend",
                "ParseError",
                "duplicate checkpoint name 'A1'",
            ),
        ],
    );
}

#[test]
fn pos_checkpoint() {
    run_pos(
        "checkpoint",
        &[],
        chk!(CheckpointChecker::new()),
        &[
            p("two_distinct_checkpoints", CKPT_OK),
            p("lifecycle_without_checkpoints", "module M\nlifecycle S :: Init -> Active -> Done\nend\n"),
            p(
                "same_name_in_different_lifecycles",
                "module M\nlifecycle A :: P -> Q\ncheckpoint: Ready\nrequires: r\non_fail: f\nend\nend\nlifecycle B :: X -> Y\ncheckpoint: Ready\nrequires: r\non_fail: f\nend\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 10. CriticalityChecker
// ═════════════════════════════════════════════════════════════════════════════

const CRIT_OK: &str = "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.2\n upper: 0.8\n end\nend\nend";

#[test]
fn neg_criticality() {
    run_neg(
        "criticality",
        true,
        chk!(CriticalityChecker::new()),
        &[
            n(
                "inverted_bounds",
                "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.8\n upper: 0.2\n end\nend\nend",
                "ParseError",
                "criticality upper (0.2) must be > lower (0.8)",
            ),
            n(
                "equal_bounds",
                "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.5\n upper: 0.5\n end\nend\nend",
                "ParseError",
                "must be > lower",
            ),
            n(
                "upper_above_one",
                "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.2\n upper: 1.5\n end\nend\nend",
                "ParseError",
                "upper bound 1.5 must be <= 1.0",
            ),
            nm(
                "negative_lower",
                CRIT_OK,
                |m| m.being_defs[0].criticality.as_mut().unwrap().lower = -0.1,
                "ParseError",
                "lower bound -0.1 must be >= 0.0",
            ),
            nm(
                "both_out_of_range",
                CRIT_OK,
                |m| {
                    let c = m.being_defs[0].criticality.as_mut().unwrap();
                    c.lower = -1.0;
                    c.upper = 2.0;
                },
                "ParseError",
                "upper bound 2 must be <= 1.0",
            ),
        ],
    );
}

#[test]
fn pos_criticality() {
    run_pos(
        "criticality",
        &[],
        chk!(CriticalityChecker::new()),
        &[
            p("interior_band", CRIT_OK),
            p("full_unit_interval", "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.0\n upper: 1.0\n end\nend\nend"),
            p("tiny_band", "module M\nbeing N\n telos: \"c\"\n end\n criticality:\n lower: 0.49\n upper: 0.5\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 11. CurryHowardChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_curryhow() {
    run_neg(
        "curryhow",
        true,
        chk!(CurryHowardChecker::new()),
        &[
            n(
                "structural_recursion_without_call",
                "module T\nfn factorial @pure :: Int -> Int\nproof: structural_recursion\n1\nend\nend",
                "TypeError",
                "no recursive call to `factorial` found",
            ),
            n(
                "totality_without_match",
                "module T\nfn d @pure :: Bool -> String\nproof: totality\n\"yes\"\nend\nend",
                "TypeError",
                "declares `proof: totality` but no match expression",
            ),
            n(
                "structural_recursion_calls_other_fn",
                "module T\nfn other @pure :: Int -> Int\n1\nend\nfn factorial @pure :: Int -> Int\nproof: structural_recursion\nother(1)\nend\nend",
                "TypeError",
                "no recursive call to `factorial`",
            ),
            n(
                "totality_with_let_but_no_match",
                "module T\nfn d @pure :: Int -> Int\nproof: totality\nlet x = 1\nx\nend\nend",
                "TypeError",
                "no match expression found in body",
            ),
            n(
                "structural_recursion_second_fn_only",
                "module T\nfn a @pure :: Int -> Int\nproof: structural_recursion\na(1)\nend\nfn b @pure :: Int -> Int\nproof: structural_recursion\n2\nend\nend",
                "TypeError",
                "function `b`",
            ),
        ],
    );
}

#[test]
fn pos_curryhow() {
    run_pos(
        "curryhow",
        &[],
        chk!(CurryHowardChecker::new()),
        &[
            p("recursive_call_present", "module T\nfn factorial @pure :: Int -> Int\nproof: structural_recursion\nfactorial(1)\nend\nend"),
            p("match_present", "module T\nfn d @pure :: Bool -> String\nproof: totality\nmatch true\n| true -> \"y\"\n| false -> \"n\"\nend\nend\nend"),
            p("unknown_strategy_not_checked", "module T\nfn c @pure :: Int -> Int\nproof: wellFounded\n1\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 12. DegeneracyChecker
// ═════════════════════════════════════════════════════════════════════════════

const DEGEN_OK: &str = "module M\nfn a :: Int -> Int\ndegenerate:\n primary: fast\n fallback: slow\nend\nend\nfn b :: Int -> Int\ndegenerate:\n primary: x\n fallback: y\nend\nend\nend";

#[test]
fn neg_degeneracy() {
    run_neg(
        "degeneracy",
        true,
        chk!(DegeneracyChecker::new()),
        &[
            n(
                "identical_primary_fallback",
                "module M\nfn c :: Int -> Int\ndegenerate:\n primary: same\n fallback: same\nend\nend\nend",
                "ParseError",
                "primary and fallback are identical ('same')",
            ),
            nm(
                "empty_primary",
                DEGEN_OK,
                |m| {
                    if let Item::Fn(f) = &mut m.items[0] {
                        f.degenerate.as_mut().unwrap().primary.clear();
                    }
                },
                "ParseError",
                "degenerate block has empty primary",
            ),
            nm(
                "empty_fallback",
                DEGEN_OK,
                |m| {
                    if let Item::Fn(f) = &mut m.items[0] {
                        f.degenerate.as_mut().unwrap().fallback.clear();
                    }
                },
                "ParseError",
                "degenerate block has empty fallback",
            ),
            nm(
                "second_fn_identical",
                DEGEN_OK,
                |m| {
                    if let Item::Fn(f) = &mut m.items[1] {
                        let d = f.degenerate.as_mut().unwrap();
                        d.fallback = d.primary.clone();
                    }
                },
                "ParseError",
                "fn 'b': degenerate block primary and fallback are identical",
            ),
            nm(
                "both_empty",
                DEGEN_OK,
                |m| {
                    if let Item::Fn(f) = &mut m.items[1] {
                        let d = f.degenerate.as_mut().unwrap();
                        d.primary.clear();
                        d.fallback.clear();
                    }
                },
                "ParseError",
                "fn 'b': degenerate block has empty primary",
            ),
        ],
    );
}

#[test]
fn pos_degeneracy() {
    run_pos(
        "degeneracy",
        &[],
        chk!(DegeneracyChecker::new()),
        &[
            p("distinct_paths", DEGEN_OK),
            p("with_equivalence_proof", "module M\nfn s :: Int -> Int\ndegenerate:\n primary: alg\n fallback: num\n equivalence_proof: identical\nend\nend\nend"),
            p("no_degenerate_block", "module M\nfn s :: Int -> Int\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 13. HgtChecker
// ═════════════════════════════════════════════════════════════════════════════

const HGT_OK: &str = "module M\nadopt: Flyable from BirdModule\nadopt: Swimmable from FishModule\nend";

#[test]
fn neg_hgt() {
    run_neg(
        "hgt",
        true,
        chk!(HgtChecker::new()),
        &[
            nm(
                "empty_interface",
                HGT_OK,
                |m| {
                    if let Item::Adopt(d) = &mut m.items[0] {
                        d.interface.clear();
                    }
                },
                "ParseError",
                "adopt: interface name must be non-empty",
            ),
            nm(
                "empty_from_module",
                HGT_OK,
                |m| {
                    if let Item::Adopt(d) = &mut m.items[0] {
                        d.from_module.clear();
                    }
                },
                "ParseError",
                "adopt: from_module must be non-empty",
            ),
            nm(
                "second_adopt_empty_interface",
                HGT_OK,
                |m| {
                    if let Item::Adopt(d) = &mut m.items[1] {
                        d.interface.clear();
                    }
                },
                "ParseError",
                "interface name must be non-empty",
            ),
            nm(
                "second_adopt_empty_from",
                HGT_OK,
                |m| {
                    if let Item::Adopt(d) = &mut m.items[1] {
                        d.from_module.clear();
                    }
                },
                "ParseError",
                "from_module must be non-empty",
            ),
            nm(
                "both_empty_reports_both",
                HGT_OK,
                |m| {
                    if let Item::Adopt(d) = &mut m.items[0] {
                        d.interface.clear();
                        d.from_module.clear();
                    }
                },
                "ParseError",
                "from_module must be non-empty",
            ),
        ],
    );
}

#[test]
fn pos_hgt() {
    run_pos(
        "hgt",
        &[],
        chk!(HgtChecker::new()),
        &[
            p("two_adopts", HGT_OK),
            p("no_adopt", "module M\nfn f :: Int -> Int\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 14. NicheConstructionChecker
// ═════════════════════════════════════════════════════════════════════════════

const NICHE_OK: &str = "module M\nniche_construction:\n modifies: soil\n affects: [Worms, Plants]\nend\nniche_construction:\n modifies: dam\n affects: [Fish]\nend\nend";

#[test]
fn neg_nichect() {
    run_neg(
        "nichect",
        true,
        chk!(NicheConstructionChecker::new()),
        &[
            n(
                "empty_affects_literal",
                "module M\nniche_construction:\n modifies: soil\n affects: []\nend\nend",
                "ParseError",
                "affects must have at least one entry",
            ),
            nm(
                "empty_modifies",
                NICHE_OK,
                |m| {
                    if let Item::NicheConstruction(nc) = &mut m.items[0] {
                        nc.modifies.clear();
                    }
                },
                "ParseError",
                "modifies must be non-empty",
            ),
            nm(
                "empty_affects_mutated",
                NICHE_OK,
                |m| {
                    if let Item::NicheConstruction(nc) = &mut m.items[0] {
                        nc.affects.clear();
                    }
                },
                "ParseError",
                "affects must have at least one entry",
            ),
            nm(
                "second_block_empty_modifies",
                NICHE_OK,
                |m| {
                    if let Item::NicheConstruction(nc) = &mut m.items[1] {
                        nc.modifies.clear();
                    }
                },
                "ParseError",
                "modifies must be non-empty",
            ),
            nm(
                "second_block_empty_affects",
                NICHE_OK,
                |m| {
                    if let Item::NicheConstruction(nc) = &mut m.items[1] {
                        nc.affects.clear();
                    }
                },
                "ParseError",
                "affects must have at least one entry",
            ),
        ],
    );
}

#[test]
fn pos_nichect() {
    run_pos(
        "nichect",
        &[],
        chk!(NicheConstructionChecker::new()),
        &[
            p("two_blocks", NICHE_OK),
            p("with_probe_fn", "module M\nniche_construction:\n modifies: h\n affects: [B]\n probe_fn: measure\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 15. PathwayChecker
// ═════════════════════════════════════════════════════════════════════════════

const PW_OK: &str = "module M\npathway Krebs\n OAA -[CS]-> Citrate\n Citrate -[Acon]-> Isocitrate\nend\nend";

#[test]
fn neg_pathway() {
    run_neg(
        "pathway",
        true,
        chk!(PathwayChecker::new()),
        &[
            n(
                "from_equals_to",
                "module M\npathway P\n A -[X]-> A\nend\nend",
                "ParseError",
                "step from 'A' and to 'A' are identical",
            ),
            nm(
                "no_steps",
                PW_OK,
                |m| {
                    if let Item::Pathway(pw) = &mut m.items[0] {
                        pw.steps.clear();
                    }
                },
                "ParseError",
                "must have at least one step",
            ),
            nm(
                "empty_from",
                PW_OK,
                |m| {
                    if let Item::Pathway(pw) = &mut m.items[0] {
                        pw.steps[0].from.clear();
                    }
                },
                "ParseError",
                "step has empty from/via/to",
            ),
            nm(
                "empty_via",
                PW_OK,
                |m| {
                    if let Item::Pathway(pw) = &mut m.items[0] {
                        pw.steps[1].via.clear();
                    }
                },
                "ParseError",
                "step has empty from/via/to",
            ),
            nm(
                "empty_to",
                PW_OK,
                |m| {
                    if let Item::Pathway(pw) = &mut m.items[0] {
                        pw.steps[1].to.clear();
                    }
                },
                "ParseError",
                "step has empty from/via/to",
            ),
            n(
                "second_step_self_loop",
                "module M\npathway P\n A -[X]-> B\n B -[Y]-> B\nend\nend",
                "ParseError",
                "step from 'B' and to 'B' are identical",
            ),
        ],
    );
}

#[test]
fn pos_pathway() {
    run_pos(
        "pathway",
        &[],
        chk!(PathwayChecker::new()),
        &[
            p("two_steps", PW_OK),
            p("with_compensate", "module M\npathway G\n Glc -[Hex]-> G6P\n compensate: gng\nend\nend"),
            p("cycle_back_to_start_is_fine", "module M\npathway C\n A -[X]-> B\n B -[Y]-> A\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 16. PrivacyChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_privacy() {
    run_neg(
        "privacy",
        true,
        chk!(PrivacyChecker::new()),
        &[
            n(
                "pci_missing_encrypt",
                "module M\ntype Card = id: Int number: String @pci @never-log end end",
                "TypeError",
                "`Card.number` has @pci but is missing @encrypt-at-rest",
            ),
            n(
                "pci_missing_never_log",
                "module M\ntype Card = number: String @pci @encrypt-at-rest end end",
                "TypeError",
                "has @pci but is missing @never-log",
            ),
            n(
                "pci_bare",
                "module M\ntype Card = number: String @pci end end",
                "TypeError",
                "missing @never-log",
            ),
            n(
                "hipaa_missing_encrypt",
                "module M\ntype Rec = diagnosis: String @hipaa end end",
                "TypeError",
                "`Rec.diagnosis` has @hipaa but is missing @encrypt-at-rest",
            ),
            n(
                "second_field_violates",
                "module M\ntype Rec = name: String @pii ssn: String @hipaa end end",
                "TypeError",
                "`Rec.ssn` has @hipaa",
            ),
            n(
                "pci_with_gdpr_still_needs_encrypt",
                "module M\ntype Card = number: String @pci @gdpr @never-log end end",
                "TypeError",
                "missing @encrypt-at-rest",
            ),
        ],
    );
}

#[test]
fn pos_privacy() {
    run_pos(
        "privacy",
        &[],
        chk!(PrivacyChecker::new()),
        &[
            p("pci_fully_labelled", "module M\ntype Card = number: String @pci @encrypt-at-rest @never-log end end"),
            p("hipaa_encrypted", "module M\ntype Rec = d: String @hipaa @encrypt-at-rest end end"),
            p("pii_alone_ok", "module M\ntype U = email: String @pii @gdpr end end"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 17. SenescenceChecker
// ═════════════════════════════════════════════════════════════════════════════

const SEN_OK: &str = "module M\nbeing A\n telos: \"a\"\n end\n senescence:\n onset: replication_limit\n degradation: failure\n end\nend\nbeing B\n telos: \"b\"\n end\n senescence:\n onset: o2\n degradation: d2\n end\nend\nend";

#[test]
fn neg_senescence() {
    run_neg(
        "senescence",
        true,
        chk!(SenescenceChecker::new()),
        &[
            nm(
                "empty_onset",
                SEN_OK,
                |m| m.being_defs[0].senescence.as_mut().unwrap().onset.clear(),
                "ParseError",
                "being 'A': senescence block has empty onset",
            ),
            nm(
                "empty_degradation",
                SEN_OK,
                |m| m.being_defs[0].senescence.as_mut().unwrap().degradation.clear(),
                "ParseError",
                "senescence block has empty degradation",
            ),
            nm(
                "second_being_empty_onset",
                SEN_OK,
                |m| m.being_defs[1].senescence.as_mut().unwrap().onset.clear(),
                "ParseError",
                "being 'B': senescence block has empty onset",
            ),
            nm(
                "second_being_empty_degradation",
                SEN_OK,
                |m| m.being_defs[1].senescence.as_mut().unwrap().degradation.clear(),
                "ParseError",
                "being 'B': senescence block has empty degradation",
            ),
            nm(
                "both_empty",
                SEN_OK,
                |m| {
                    let s = m.being_defs[0].senescence.as_mut().unwrap();
                    s.onset.clear();
                    s.degradation.clear();
                },
                "ParseError",
                "empty degradation",
            ),
        ],
    );
}

#[test]
fn pos_senescence() {
    run_pos(
        "senescence",
        &[],
        chk!(SenescenceChecker::new()),
        &[
            p("two_beings", SEN_OK),
            p("with_sasp", "module M\nbeing F\n telos: \"t\"\n end\n senescence:\n onset: o\n degradation: d\n sasp: cytokines\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 18. SymbiosisChecker
// ═════════════════════════════════════════════════════════════════════════════

const SYM_OK: &str = "module M
symbiotic:
 kind: mutualistic
 module: Gut
end";

#[test]
fn neg_symbiosis() {
    run_neg(
        "symbiosis",
        true,
        chk!(SymbiosisChecker::new()),
        &[
            n(
                "invalid_kind_friendly",
                "module M\nsymbiotic:\n kind: friendly\n module: Gut\nend",
                "ParseError",
                "kind 'friendly' is not valid",
            ),
            n(
                "invalid_kind_wrong_case",
                "module M\nsymbiotic:\n kind: Mutualistic\n module: Gut\nend",
                "ParseError",
                "kind 'Mutualistic' is not valid",
            ),
            nm(
                "empty_module_name",
                SYM_OK,
                |m| {
                    if let Item::SymbioticImport { module, .. } = &mut m.items[0] {
                        module.clear();
                    }
                },
                "ParseError",
                "module name must be non-empty",
            ),
            nm(
                "empty_kind",
                SYM_OK,
                |m| {
                    if let Item::SymbioticImport { kind, .. } = &mut m.items[0] {
                        kind.clear();
                    }
                },
                "ParseError",
                "kind '' is not valid",
            ),
            n(
                "second_import_invalid",
                "module M
fn f :: Int -> Int
end
symbiotic:
 kind: predatory
 module: B
end",
                "ParseError",
                "kind 'predatory' is not valid",
            ),
        ],
    );
}

#[test]
fn pos_symbiosis() {
    run_pos(
        "symbiosis",
        &[],
        chk!(SymbiosisChecker::new()),
        &[
            p("mutualistic_commensal", SYM_OK),
            p("parasitic", "module M\nsymbiotic:\n kind: parasitic\n module: P\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 19. SelfCertChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_selfcert() {
    run_neg(
        "selfcert",
        true,
        chk!(SelfCertChecker::new()),
        &[
            n(
                "type_safety_maybe",
                "module T\ncertificate:\n type_safety = maybe\nend\nend",
                "TypeError",
                "`type_safety` has unknown value `maybe`",
            ),
            n(
                "purity_yes",
                "module T\ncertificate:\n purity = yes\nend\nend",
                "TypeError",
                "`purity` has unknown value `yes`",
            ),
            n(
                "termination_ok_is_not_a_value",
                "module T\ncertificate:\n termination = ok\nend\nend",
                "TypeError",
                "`termination` has unknown value `ok`",
            ),
            n(
                "second_field_bad",
                "module T\ncertificate:\n type_safety = proven\n separation = done\nend\nend",
                "TypeError",
                "`separation` has unknown value `done`",
            ),
            nm(
                "empty_field_value",
                "module T\ncertificate:\n type_safety = proven\nend\nend",
                |m| {
                    if let Item::Certificate(c) = &mut m.items[0] {
                        c.fields[0].value.clear();
                    }
                },
                "TypeError",
                "empty name or value",
            ),
        ],
    );
}

#[test]
fn pos_selfcert() {
    run_pos(
        "selfcert",
        &[],
        chk!(SelfCertChecker::new()),
        &[
            p("all_valid_values", "module T\ncertificate:\n type_safety = proven\n memory_safety = verified\n purity = checked\n privacy = passed\n termination = partial\n effect_safety = assumed\nend\nend"),
            p("custom_field_free_value", "module T\ncertificate:\n reviewed_by = alice\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 20. SideChannelChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_sidechannel() {
    run_neg(
        "sidechannel",
        true,
        chk!(SideChannelChecker::new()),
        &[
            n(
                "timing_safe_without_block",
                "module T\nfn cmp @timing-safe @pure :: Int -> Int -> Bool\nend\nend",
                "TypeError",
                "annotated `@timing-safe` but lacks a `timing_safety:` block",
            ),
            n(
                "constant_time_leaks_bits",
                "module T\nfn cmp @timing-safe @pure :: Int -> Int -> Bool\ntiming_safety:\n constant_time: true\n leaks_bits: \"3.2 bits\"\nend\nend\nend",
                "TypeError",
                "side-channel",
            ),
            n(
                "constant_time_small_leak",
                "module T\nfn cmp @pure :: Int -> Int -> Bool\ntiming_safety:\n constant_time: true\n leaks_bits: \"0.5\"\nend\nend\nend",
                "TypeError",
                "side-channel",
            ),
            n(
                "underscore_spelling_without_block",
                "module T\nfn cmp @timing_safe @pure :: Int -> Int -> Bool\nend\nend",
                "TypeError",
                "lacks a `timing_safety:` block",
            ),
            n(
                "constant_time_one_bit",
                "module T\nfn cmp @pure :: Int -> Int -> Bool\ntiming_safety:\n constant_time: true\n leaks_bits: \"1 bits\"\nend\nend\nend",
                "TypeError",
                "side-channel",
            ),
        ],
    );
}

#[test]
fn pos_sidechannel() {
    run_pos(
        "sidechannel",
        &[],
        chk!(SideChannelChecker::new()),
        &[
            p("zero_leak_constant_time", "module T\nfn c @timing-safe @pure :: Int -> Int -> Bool\ntiming_safety:\n constant_time: true\n leaks_bits: \"0.0 bits\"\nend\nend\nend"),
            p("not_constant_time_may_leak", "module T\nfn c @pure :: Int -> Int -> Bool\ntiming_safety:\n constant_time: false\n leaks_bits: \"3.2 bits\"\nend\nend\nend"),
            p("no_annotation_no_block", "module T\nfn c @pure :: Int -> Int -> Bool\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 21. GradualChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_gradual() {
    run_neg(
        "gradual",
        true,
        chk!(GradualChecker::new()),
        &[
            n(
                "dynamic_param_no_block",
                "module T\nfn f @pure :: ? -> String\nend\nend",
                "TypeError",
                "uses `?` (dynamic) type without a `gradual:` block",
            ),
            n(
                "dynamic_return_no_block",
                "module T\nfn f @pure :: Int -> ?\nend\nend",
                "TypeError",
                "uses `?` (dynamic) type without a `gradual:` block",
            ),
            n(
                "cast_failure_without_blame",
                "module T\nfn f @pure :: ? -> String\ngradual:\n input_type: ?\n on_cast_failure: panic\nend\nend\nend",
                "TypeError",
                "missing `blame:`",
            ),
            n(
                "cast_failure_without_blame_with_boundary",
                "module T\nfn f @pure :: ? -> String\ngradual:\n input_type: ?\n boundary: B\n output_type: String\n on_cast_failure: log\nend\nend\nend",
                "TypeError",
                "blame tracking is required",
            ),
            n(
                "dynamic_second_param",
                "module T\nfn f @pure :: Int -> ? -> String\nend\nend",
                "TypeError",
                "function `f` uses `?`",
            ),
        ],
    );
}

#[test]
fn pos_gradual() {
    run_pos(
        "gradual",
        &[],
        chk!(GradualChecker::new()),
        &[
            p("dynamic_with_block_and_blame", "module T\nfn f @pure :: ? -> String\ngradual:\n input_type: ?\n on_cast_failure: panic\n blame: caller\nend\nend\nend"),
            p("dynamic_with_block_no_handler", "module T\nfn f @pure :: ? -> String\ngradual:\n input_type: ?\nend\nend\nend"),
            p("fully_static", "module T\nfn f @pure :: Int -> String\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 22. DependentChecker
// ═════════════════════════════════════════════════════════════════════════════

const DEP_OK: &str = "module T\nproposition A = Int\nproposition B = String\nend";

#[test]
fn neg_dependent() {
    run_neg(
        "dependent",
        true,
        chk!(DependentChecker::new()),
        &[
            n(
                "termination_on_impure_fn",
                "module T\nfn f :: Int -> Int\ntermination: structural_recursion\nend\nend",
                "TypeError",
                "not annotated `@pure`",
            ),
            n(
                "termination_with_other_annotation_only",
                "module T\nfn f @idempotent :: Int -> Int\ntermination: well_founded\nend\nend",
                "TypeError",
                "termination proofs are only valid for pure functions",
            ),
            n(
                "second_fn_impure",
                "module T\nfn a @pure :: Int -> Int\ntermination: structural_recursion\nend\nfn b :: Int -> Int\ntermination: structural_recursion\nend\nend",
                "TypeError",
                "function `b` declares a `termination:` claim",
            ),
            nm(
                "empty_proposition_base",
                DEP_OK,
                |m| {
                    if let Item::Proposition(pr) = &mut m.items[0] {
                        pr.base_type = TypeExpr::Base(String::new());
                    }
                },
                "TypeError",
                "proposition `A` has an empty base type",
            ),
            nm(
                "second_proposition_empty_base",
                DEP_OK,
                |m| {
                    if let Item::Proposition(pr) = &mut m.items[1] {
                        pr.base_type = TypeExpr::Base(String::new());
                    }
                },
                "TypeError",
                "proposition `B` has an empty base type",
            ),
        ],
    );
}

#[test]
fn pos_dependent() {
    run_pos(
        "dependent",
        &[],
        chk!(DependentChecker::new()),
        &[
            p("termination_on_pure", "module T\nfn f @pure :: Int -> Int\ntermination: structural_recursion\nend\nend"),
            p("propositions", DEP_OK),
            p("impure_fn_without_termination", "module T\nfn f :: Int -> Int\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 23. ProbabilisticChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_probabilistic() {
    run_neg(
        "probabilistic",
        true,
        chk!(ProbabilisticChecker::new()),
        &[
            n(
                "annotated_without_distribution",
                "module T\nfn s @probabilistic @pure :: Int -> Float\nend\nend",
                "TypeError",
                "lacks a `distribution:` block",
            ),
            n(
                "convergence_on_non_numeric_return",
                "module T\nfn c @probabilistic @pure :: Int -> String\ndistribution:\n model: mcmc\n convergence: 0\nend\nend\nend",
                "TypeError",
                "convergence",
            ),
            n(
                "cauchy_with_clt_convergence",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Cauchy(location: 0.0, scale: 1.0)\n convergence: central_limit\nend\nend\nend",
                "TypeError",
                "Cauchy",
            ),
            n(
                "levy_with_law_of_large_numbers",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Levy(location: 0.0, scale: 1.0)\n convergence: law_of_large_numbers\nend\nend\nend",
                "TypeError",
                "no defined mean or variance",
            ),
            n(
                "beta_alpha_non_positive",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Beta(alpha: 0.0, beta: 1.0)\nend\nend\nend",
                "TypeError",
                "Beta alpha parameter must be > 0",
            ),
            n(
                "beta_beta_negative",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Beta(alpha: 1.0, beta: 0.0)\nend\nend\nend",
                "TypeError",
                "Beta beta parameter must be > 0",
            ),
            n(
                "binomial_p_above_one",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Binomial(n: 10, p: 1.5)\nend\nend\nend",
                "TypeError",
                "Binomial p must be in [0, 1]",
            ),
            n(
                "gaussian_zero_std_dev",
                "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Gaussian(mean: 0.0, std_dev: 0.0)\nend\nend\nend",
                "TypeError",
                "Gaussian std_dev must be > 0",
            ),
        ],
    );
}

#[test]
fn pos_probabilistic() {
    run_pos(
        "probabilistic",
        &[],
        chk!(ProbabilisticChecker::new()),
        &[
            p("gaussian_valid", "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Gaussian(mean: 0.0, std_dev: 1.0)\nend\nend\nend"),
            p("beta_valid", "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Beta(alpha: 2.0, beta: 5.0)\nend\nend\nend"),
            p("binomial_boundary_p", "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Binomial(n: 10, p: 1.0)\nend\nend\nend"),
            p("cauchy_without_convergence_claim", "module T\nfn c @probabilistic @pure :: Int -> Float\ndistribution:\n family: Cauchy(location: 0.0, scale: 1.0)\nend\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 24. ErrorCorrectionChecker
// ═════════════════════════════════════════════════════════════════════════════

const EC_OK: &str = "module M\ntype PosInt = Int where n > 0\n on_violation: clamp\n repair_fn: fixit\nend\ntype Small = Int where n < 10\n on_violation: cap\nend\nend";

#[test]
fn neg_errorcorrection() {
    run_neg(
        "errorcorrection",
        true,
        chk!(ErrorCorrectionChecker::new()),
        &[
            nm(
                "empty_on_violation",
                EC_OK,
                |m| {
                    if let Item::RefinedType(rt) = &mut m.items[0] {
                        rt.on_violation = Some(String::new());
                    }
                },
                "ParseError",
                "type 'PosInt': on_violation must be non-empty if present",
            ),
            nm(
                "empty_repair_fn",
                EC_OK,
                |m| {
                    if let Item::RefinedType(rt) = &mut m.items[0] {
                        rt.repair_fn = Some(String::new());
                    }
                },
                "ParseError",
                "type 'PosInt': repair_fn must be non-empty if present",
            ),
            nm(
                "second_type_empty_on_violation",
                EC_OK,
                |m| {
                    if let Item::RefinedType(rt) = &mut m.items[1] {
                        rt.on_violation = Some(String::new());
                    }
                },
                "ParseError",
                "type 'Small': on_violation must be non-empty",
            ),
            nm(
                "second_type_empty_repair_fn",
                EC_OK,
                |m| {
                    if let Item::RefinedType(rt) = &mut m.items[1] {
                        rt.repair_fn = Some(String::new());
                    }
                },
                "ParseError",
                "type 'Small': repair_fn must be non-empty",
            ),
            nm(
                "both_empty",
                EC_OK,
                |m| {
                    if let Item::RefinedType(rt) = &mut m.items[0] {
                        rt.on_violation = Some(String::new());
                        rt.repair_fn = Some(String::new());
                    }
                },
                "ParseError",
                "repair_fn must be non-empty",
            ),
        ],
    );
}

#[test]
fn pos_errorcorrection() {
    run_pos(
        "errorcorrection",
        &[],
        chk!(ErrorCorrectionChecker::new()),
        &[
            p("handlers_present", EC_OK),
            p("no_handlers", "module M\ntype P = Int where n > 0\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 25. TemporalChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_temporal() {
    run_neg(
        "temporal",
        true,
        chk!(TemporalChecker::new()),
        &[
            n(
                "never_backward_transition",
                "module B\nlifecycle Payment :: Pending -> Settled -> Archived\ntemporal R\n never: Settled transitions to Pending\nend\nfn regress :: Payment<Settled> -> Payment<Pending>\n todo\nend\nend",
                "TypeError",
                "transition from `Settled` to `Pending` is forbidden",
            ),
            n(
                "never_forward_skip",
                "module B\nlifecycle Payment :: Pending -> Settled -> Archived\ntemporal R\n never: Pending transitions to Archived\nend\nfn jump :: Payment<Pending> -> Payment<Archived>\n todo\nend\nend",
                "TypeError",
                "transition from `Pending` to `Archived` is forbidden",
            ),
            n(
                "precedes_reversed_order",
                "module B\nlifecycle Auth :: Anonymous -> Identified -> Authorized\ntemporal R\n precedes: Authorized before Identified\nend\nend",
                "TypeError",
                "`Authorized` does not precede `Identified`",
            ),
            n(
                "precedes_skips_required_predecessor",
                "module B\nlifecycle Auth :: Anonymous -> Identified -> Authorized\ntemporal R\n precedes: Identified before Authorized\nend\nfn skip :: Auth<Anonymous> -> Auth<Authorized>\n todo\nend\nend",
                "TypeError",
                "skipping required predecessor `Identified`",
            ),
            n(
                "precedes_same_state",
                "module B\nlifecycle Auth :: Anonymous -> Identified -> Authorized\ntemporal R\n precedes: Identified before Identified\nend\nend",
                "TypeError",
                "does not precede",
            ),
        ],
    );
}

#[test]
fn pos_temporal() {
    run_pos(
        "temporal",
        &[],
        chk!(TemporalChecker::new()),
        &[
            p(
                "never_not_realised_by_any_fn",
                "module B\nlifecycle Payment :: Pending -> Settled -> Archived\ntemporal R\n never: Settled transitions to Pending\nend\nfn settle :: Payment<Pending> -> Payment<Settled>\n todo\nend\nend",
            ),
            p(
                "precedes_respected",
                "module B\nlifecycle Auth :: Anonymous -> Identified -> Authorized\ntemporal R\n precedes: Identified before Authorized\nend\nfn step :: Auth<Identified> -> Auth<Authorized>\n todo\nend\nend",
            ),
            p(
                "eventually_and_always_not_checked",
                "module B\nlifecycle Order :: Created -> Shipped -> Delivered\ntemporal R\n eventually: Order reaches Delivered\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 26. TypestateChecker
// ═════════════════════════════════════════════════════════════════════════════

const TS_LC: &str = "lifecycle Connection :: Disconnected -> Connected -> Authenticated";

#[test]
fn neg_typestate() {
    run_neg(
        "typestate",
        true,
        chk!(TypestateChecker::new()),
        &[
            n(
                "skip_a_state",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn bad :: Connection<Disconnected> -> Connection<Authenticated>\n conn\nend\nend",
                "TypeError",
                "invalid lifecycle transition: Connection<Disconnected> -> Connection<Authenticated>",
            ),
            n(
                "backward_transition",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn bad :: Connection<Authenticated> -> Connection<Connected>\n conn\nend\nend",
                "TypeError",
                "invalid lifecycle transition",
            ),
            n(
                "self_transition",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn bad :: Connection<Connected> -> Connection<Connected>\n conn\nend\nend",
                "TypeError",
                "Connection<Connected> -> Connection<Connected>",
            ),
            n(
                "effect_wrapped_return_still_checked",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn bad :: Connection<Disconnected> -> Effect<[IO], Connection<Authenticated>>\n conn\nend\nend",
                "TypeError",
                "invalid lifecycle transition",
            ),
            n(
                "one_of_two_inputs_invalid",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn bad :: Connection<Disconnected> -> Connection<Connected> -> Connection<Authenticated>\n conn\nend\nend",
                "TypeError",
                "Connection<Disconnected> -> Connection<Authenticated>",
            ),
        ],
    );
}

#[test]
fn pos_typestate() {
    run_pos(
        "typestate",
        &[],
        chk!(TypestateChecker::new()),
        &[
            p(
                "first_step",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn c :: Connection<Disconnected> -> Connection<Connected>\n conn\nend\nend",
            ),
            p(
                "second_step_effectful",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn c :: Connection<Connected> -> Effect<[IO], Connection<Authenticated>>\n conn\nend\nend",
            ),
            p(
                "unrelated_type_not_checked",
                "module S\nlifecycle Connection :: Disconnected -> Connected -> Authenticated\nfn c :: Session<Open> -> Session<Closed>\n s\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 27. SeparationChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_separation() {
    run_neg(
        "separation",
        true,
        chk!(SeparationChecker::new()),
        &[
            n(
                "disjoint_right_undeclared",
                "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\ndisjoint: a * b\nend\nend\nend",
                "TypeError",
                "`disjoint` references `b` but it is not declared in `owns:`",
            ),
            n(
                "disjoint_left_undeclared",
                "module T\nfn f :: Unit -> Unit\nseparation:\nowns: b\ndisjoint: a * b\nend\nend\nend",
                "TypeError",
                "`disjoint` references `a`",
            ),
            n(
                "frame_undeclared",
                "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\nframe: ghost\nend\nend\nend",
                "TypeError",
                "`frame` references `ghost` but it is not declared in `owns:`",
            ),
            n(
                "disjoint_without_any_owns",
                "module T\nfn f :: Unit -> Unit\nseparation:\ndisjoint: x * y\nend\nend\nend",
                "TypeError",
                "`disjoint` references `x`",
            ),
            n(
                "second_fn_violates",
                "module T\nfn ok :: Unit -> Unit\nseparation:\nowns: a\nowns: b\ndisjoint: a * b\nend\nend\nfn bad :: Unit -> Unit\nseparation:\nowns: a\nframe: z\nend\nend\nend",
                "TypeError",
                "separation violation in `bad`",
            ),
        ],
    );
}

#[test]
fn pos_separation() {
    run_pos(
        "separation",
        &[],
        chk!(SeparationChecker::new()),
        &[
            p("declared_disjoint", "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\nowns: b\ndisjoint: a * b\nend\nend\nend"),
            p("declared_frame", "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\nframe: a\nend\nend\nend"),
            p("no_separation_block", "module T\nfn f :: Unit -> Unit\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 28. TensorChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_tensor() {
    run_neg(
        "tensor",
        true,
        chk!(TensorChecker::new()),
        &[
            n(
                "rank_exceeds_shape_param",
                "module T\nfn f :: Tensor<2, [3], Float> -> Int\n 0\nend\nend",
                "TypeError",
                "tensor rank 2 does not match shape length 1",
            ),
            n(
                "shape_exceeds_rank_return",
                "module T\nfn f :: Int -> Tensor<1, [3, 3], Float>\n todo\nend\nend",
                "TypeError",
                "tensor rank 1 does not match shape length 2",
            ),
            n(
                "rank_above_eight",
                "module T\nfn f :: Tensor<9, [1, 1, 1, 1, 1, 1, 1, 1, 1], Float> -> Int\n 0\nend\nend",
                "TypeError",
                "unusually high (> 8)",
            ),
            n(
                "nested_in_generic",
                "module T\nfn f :: List<Tensor<1, [2, 3], Float>> -> Int\n 0\nend\nend",
                "TypeError",
                "tensor rank 1 does not match shape length 2",
            ),
            n(
                "scalar_rank_with_shape",
                "module T\nfn f :: Tensor<0, [3], Float> -> Int\n 0\nend\nend",
                "TypeError",
                "tensor rank 0 does not match shape length 1",
            ),
        ],
    );
}

#[test]
fn pos_tensor() {
    run_pos(
        "tensor",
        &[],
        chk!(TensorChecker::new()),
        &[
            p("matrix", "module T\nfn f :: Tensor<2, [3, 3], Float> -> Int\n 0\nend\nend"),
            p("scalar", "module T\nfn f :: Tensor<0, [], Float> -> Int\n 0\nend\nend"),
            p("rank_eight_boundary", "module T\nfn f :: Tensor<8, [1, 1, 1, 1, 1, 1, 1, 1], Float> -> Int\n 0\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 29. SessionChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_session() {
    run_neg(
        "session",
        false,
        chk!(SessionChecker::new()),
        &[
            n(
                "three_roles",
                "module P\nsession S\n a:\n send: Int\n end\n b:\n recv: Int\n end\n c:\n recv: Int\n end\nend\nend",
                "ParseError",
                "must have exactly 2 roles, found 3",
            ),
            n(
                "one_role",
                "module P\nsession S\n a:\n send: Int\n end\nend\nend",
                "ParseError",
                "must have exactly 2 roles, found 1",
            ),
            n(
                "duality_unknown_role",
                "module P\nsession S\n a:\n send: Int\n end\n b:\n recv: Int\n end\n duality: a <-> ghost\nend\nend",
                "ParseError",
                "duality references unknown role(s)",
            ),
            n(
                "duality_length_mismatch",
                "module P\nsession S\n a:\n send: Int\n recv: Int\n end\n b:\n recv: Int\n end\n duality: a <-> b\nend\nend",
                "ParseError",
                "protocols have different lengths (2 steps in 'a' vs 1 steps in 'b')",
            ),
            n(
                "duality_send_recv_type_mismatch",
                "module P\nsession S\n a:\n send: Int\n end\n b:\n recv: String\n end\n duality: a <-> b\nend\nend",
                "ParseError",
                "duality violation at step 1",
            ),
            n(
                "duality_both_send",
                "module P\nsession S\n a:\n send: Int\n end\n b:\n send: Int\n end\n duality: a <-> b\nend\nend",
                "ParseError",
                "both roles have the same direction",
            ),
            n(
                "duality_recv_send_type_mismatch",
                "module P\nsession S\n a:\n recv: Int\n end\n b:\n send: Bool\n end\n duality: a <-> b\nend\nend",
                "ParseError",
                "role 'a' expects",
            ),
        ],
    );
}

#[test]
fn pos_session() {
    run_pos(
        "session",
        &[],
        chk!(SessionChecker::new()),
        &[
            p("dual_pair", "module P\nsession S\n a:\n send: Int\n recv: String\n end\n b:\n recv: Int\n send: String\n end\n duality: a <-> b\nend\nend"),
            p("two_roles_no_duality_declared", "module P\nsession S\n a:\n send: Int\n end\n b:\n send: Int\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 30. EffectHandlerChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_effect_handler() {
    run_neg(
        "effect_handler",
        false,
        chk!(EffectHandlerChecker::new()),
        &[
            n(
                "missing_second_operation",
                "module R\neffect Log\n operation emit :: String -> Unit\n operation flush :: Unit -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n Log.emit(msg) -> k:\n k(unit)\n end\n end\nend\nend",
                "ParseError",
                "does not handle operation 'flush' declared in effect 'Log'",
            ),
            n(
                "missing_first_operation",
                "module R\neffect Log\n operation emit :: String -> Unit\n operation flush :: Unit -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n Log.flush(u) -> k:\n k(unit)\n end\n end\nend\nend",
                "ParseError",
                "operation 'emit'",
            ),
            n(
                "three_ops_only_one_handled",
                "module R\neffect St\n operation get :: Unit -> Int\n operation put :: Int -> Unit\n operation reset :: Unit -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n St.get(u) -> k:\n k(1)\n end\n end\nend\nend",
                "ParseError",
                "operation 'reset' declared in effect 'St'",
            ),
            n(
                "two_effects_second_incomplete",
                "module R\neffect A\n operation x :: Unit -> Unit\nend\neffect B\n operation y :: Unit -> Unit\n operation z :: Unit -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n A.x(u) -> k:\n k(unit)\n end\n B.y(u) -> k:\n k(unit)\n end\n end\nend\nend",
                "ParseError",
                "operation 'z' declared in effect 'B'",
            ),
            n(
                "second_fn_incomplete",
                "module R\neffect Log\n operation emit :: String -> Unit\n operation flush :: Unit -> Unit\nend\nfn ok :: Unit -> Unit\n handle comp with\n Log.emit(m) -> k:\n k(unit)\n end\n Log.flush(u) -> k:\n k(unit)\n end\n end\nend\nfn bad :: Unit -> Unit\n handle comp with\n Log.emit(m) -> k:\n k(unit)\n end\n end\nend\nend",
                "ParseError",
                "fn 'bad': handle block does not handle",
            ),
        ],
    );
}

#[test]
fn pos_effect_handler() {
    run_pos(
        "effect_handler",
        &[],
        chk!(EffectHandlerChecker::new()),
        &[
            p(
                "all_operations_handled",
                "module R\neffect Log\n operation emit :: String -> Unit\n operation flush :: Unit -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n Log.emit(m) -> k:\n k(unit)\n end\n Log.flush(u) -> k:\n k(unit)\n end\n end\nend\nend",
            ),
            p(
                "single_op_effect",
                "module R\neffect Log\n operation emit :: String -> Unit\nend\nfn run :: Unit -> Unit\n handle comp with\n Log.emit(m) -> k:\n k(unit)\n end\n end\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 31. SafetyChecker (via SafetyCheckerAdapter)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_safety() {
    run_neg(
        "safety",
        true,
        chk!(SafetyCheckerAdapter),
        &[
            n(
                "autopoietic_missing_mortal",
                "module M\nbeing C\n @sandboxed\n autopoietic: true\n matter:\n m: Float\n end\n telos: \"t\"\n end\nend\nend",
                "TypeError",
                "autopoietic being missing @mortal",
            ),
            n(
                "autopoietic_missing_sandboxed",
                "module M\nbeing C\n @mortal\n autopoietic: true\n matter:\n m: Float\n end\n telos: \"t\"\n end\n telomere:\n limit: 5\n on_exhaustion: apoptosis\n end\nend\nend",
                "TypeError",
                "autopoietic being missing @sandboxed",
            ),
            n(
                "mortal_without_telomere",
                "module M\nbeing C\n @mortal\n telos: \"t\"\n end\nend\nend",
                "TypeError",
                "@mortal requires telomere: block",
            ),
            n(
                "corrigible_without_modifiable_by",
                "module M\nbeing C\n @corrigible\n telos: \"serve\"\n end\nend\nend",
                "TypeError",
                "@corrigible requires telos.modifiable_by",
            ),
            n(
                "corrigible_without_telos",
                "module M\nbeing C\n @corrigible\n matter:\n m: Float\n end\nend\nend",
                "TypeError",
                "@corrigible requires telos: block",
            ),
            n(
                "bounded_telos_open_ended_term",
                "module M\nbeing C\n @bounded_telos\n telos: \"maximize profit\"\n bounded_by: ceiling\n end\nend\nend",
                "TypeError",
                "open-ended utility term 'maximize'",
            ),
            n(
                "bounded_telos_without_bounded_by",
                "module M\nbeing C\n @bounded_telos\n telos: \"serve users\"\n end\nend\nend",
                "TypeError",
                "@bounded_telos requires telos.bounded_by",
            ),
        ],
    );
}

#[test]
fn pos_safety() {
    run_pos(
        "safety",
        &[],
        chk!(SafetyCheckerAdapter),
        &[
            p(
                "fully_safe_autopoietic",
                "module M\nbeing C\n @mortal @sandboxed\n autopoietic: true\n matter:\n m: Float\n end\n telos: \"t\"\n end\n telomere:\n limit: 5\n on_exhaustion: apoptosis\n end\nend\nend",
            ),
            p(
                "corrigible_with_modifiable_by",
                "module M\nbeing C\n @corrigible\n telos: \"serve\"\n modifiable_by: operator\n end\nend\nend",
            ),
            p(
                "bounded_telos_ok",
                "module M\nbeing C\n @bounded_telos\n telos: \"serve users\"\n bounded_by: ceiling\n end\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 32. RandomnessChecker (via RandomnessCheckerAdapter)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_randomness() {
    run_neg(
        "randomness",
        true,
        chk!(RandomnessCheckerAdapter),
        &[
            n(
                "pseudo_random_requires_auth",
                "module M\nfn token @pseudo_random @requires_auth :: Int -> Int\n 0\nend\nend",
                "TypeError",
                "`token` is @pseudo_random but used in a security context",
            ),
            n(
                "pseudo_random_pii",
                "module M\nfn salt @pseudo_random @pii :: Int -> Int\n 0\nend\nend",
                "TypeError",
                "`salt` is @pseudo_random",
            ),
            n(
                "pseudo_random_both_security_marks",
                "module M\nfn key @pseudo_random @requires_auth @pii :: Int -> Int\n 0\nend\nend",
                "TypeError",
                "NIST SP 800-90A",
            ),
            n(
                "annotation_order_swapped",
                "module M\nfn nonce @requires_auth @pseudo_random :: Int -> Int\n 0\nend\nend",
                "TypeError",
                "`nonce` is @pseudo_random",
            ),
            n(
                "second_fn_violates",
                "module M\nfn good @crypto_random @requires_auth :: Int -> Int\n 0\nend\nfn bad @pseudo_random @requires_auth :: Int -> Int\n 0\nend\nend",
                "TypeError",
                "`bad` is @pseudo_random",
            ),
        ],
    );
}

#[test]
fn pos_randomness() {
    run_pos(
        "randomness",
        &[],
        chk!(RandomnessCheckerAdapter),
        &[
            p("crypto_with_auth", "module M\nfn t @crypto_random @requires_auth :: Int -> Int\n 0\nend\nend"),
            p("true_random_with_pii", "module M\nfn t @true_random @pii :: Int -> Int\n 0\nend\nend"),
            p("pseudo_in_non_security_context", "module M\nfn shuffle @pseudo_random :: Int -> Int\n 0\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 33. StochasticChecker (via StochasticCheckerAdapter)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_stochastic() {
    run_neg(
        "stochastic",
        true,
        chk!(StochasticCheckerAdapter),
        &[
            n(
                "gbm_always_positive_false",
                "module W\nfn m @probabilistic :: Unit -> Float\n process:\n kind: GeometricBrownian\n always_positive: false\n end\nend\nend",
                "TypeError",
                "GeometricBrownian with always_positive: false",
            ),
            n(
                "poisson_not_integer_valued",
                "module W\nfn m @probabilistic :: Unit -> Int\n process:\n kind: PoissonProcess\n rate: 3.5\n integer_valued: false\n end\nend\nend",
                "TypeError",
                "PoissonProcess with integer_valued: false",
            ),
            n(
                "markov_chain_without_states",
                "module W\nfn m @probabilistic :: Unit -> Int\n process:\n kind: MarkovChain\n end\nend\nend",
                "TypeError",
                "MarkovChain but provides no states",
            ),
            n(
                "gbm_with_gaussian_family",
                "module W\nfn m @probabilistic :: Unit -> Float\n distribution:\n family: Gaussian(mean: 0.0, std_dev: 0.1)\n end\n process:\n kind: GeometricBrownian\n always_positive: true\n end\nend\nend",
                "TypeError",
                "GeometricBrownian but distribution family Gaussian",
            ),
            n(
                "second_fn_markov_empty",
                "module W\nfn ok @probabilistic :: Unit -> Int\n process:\n kind: MarkovChain\n states: [A, B]\n end\nend\nfn bad @probabilistic :: Unit -> Int\n process:\n kind: MarkovChain\n end\nend\nend",
                "TypeError",
                "`bad` declares MarkovChain",
            ),
        ],
    );
}

#[test]
fn pos_stochastic() {
    run_pos(
        "stochastic",
        &[],
        chk!(StochasticCheckerAdapter),
        &[
            p("gbm_ok", "module W\nfn m @probabilistic :: Unit -> Float\n distribution:\n family: GeometricBrownian(drift: 0.05, volatility: 0.2)\n end\n process:\n kind: GeometricBrownian\n always_positive: true\n end\nend\nend"),
            p("poisson_ok", "module W\nfn m @probabilistic :: Unit -> Int\n process:\n kind: PoissonProcess\n rate: 3.5\n integer_valued: true\n end\nend\nend"),
            p("markov_with_states", "module W\nfn m @probabilistic :: Unit -> Int\n process:\n kind: MarkovChain\n states: [Sunny, Rainy]\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 34. UmweltChecker
// ═════════════════════════════════════════════════════════════════════════════

const UMW_HEAD: &str = "module S\nbeing T\n telos: \"t\"\n end\n umwelt:\n detects: [Temperature, Humidity]\n blind_to: [LightLevel]\n end\n function:\n";

#[test]
fn neg_umwelt() {
    let mk = |fns: &str| -> String { format!("{UMW_HEAD}{fns}\n end\nend\nend") };
    let blind = mk("fn see :: LightLevel -> Unit\n todo\n end");
    let blind_second = mk("fn ok :: Temperature -> Unit\n todo\n end\n fn see :: Int -> LightLevel -> Unit\n todo\n end");
    let not_detected = mk("fn odd :: Pressure -> Unit\n todo\n end");
    let blind_effect = mk("fn see :: LightLevel -> Effect<[IO], Unit>\n todo\n end");
    let both = mk("fn see :: LightLevel -> Unit\n todo\n end\n fn odd :: Pressure -> Unit\n todo\n end");
    // leak the strings so the table can hold &'static str
    let s = |x: String| -> &'static str { Box::leak(x.into_boxed_str()) };
    run_neg(
        "umwelt",
        true,
        chk!(UmweltChecker::new()),
        &[
            n(
                "blind_signal_handled",
                s(blind),
                "TypeError",
                "umwelt.blind_to",
            ),
            n(
                "blind_signal_in_second_param",
                s(blind_second),
                "TypeError",
                "signal type 'LightLevel' which is in umwelt.blind_to",
            ),
            n(
                "undetected_signal_when_detects_declared",
                s(not_detected),
                "TypeError",
                "'Pressure' not in umwelt.detects",
            ),
            n(
                "blind_through_effect_wrapper",
                s(blind_effect),
                "TypeError",
                "this being is blind to that signal",
            ),
            n(
                "blind_and_undetected_reports_blind",
                s(both),
                "TypeError",
                "function 'see'",
            ),
        ],
    );
    let ok1 = mk("fn read :: Temperature -> Unit\n todo\n end");
    let ok2 = mk("fn read :: Humidity -> Unit\n todo\n end\n fn read2 :: Temperature -> Humidity -> Unit\n todo\n end");
    run_pos(
        "umwelt",
        &[],
        chk!(UmweltChecker::new()),
        &[
            p("detected_signal", s(ok1)),
            p("two_detected_signals", s(ok2)),
            p(
                "no_umwelt_is_omnisensory",
                "module S\nbeing T\n telos: \"t\"\n end\n function:\n fn x :: LightLevel -> Unit\n todo\n end\n end\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 35. ResonanceChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_resonance() {
    run_neg(
        "resonance",
        false,
        chk!(ResonanceChecker::new()),
        &[
            n(
                "both_signals_unknown",
                "module C\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: SignalA with SignalB\n end\nend\nend",
                "TypeError",
                "signal type 'SignalA'",
            ),
            n(
                "second_signal_unknown",
                "module C\nfn f :: Temperature -> Unit\n todo\nend\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: Temperature with Pressure\n end\nend\nend",
                "TypeError",
                "signal type 'Pressure'",
            ),
            n(
                "first_signal_unknown",
                "module C\nfn f :: Temperature -> Unit\n todo\nend\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: Spin with Temperature\n end\nend\nend",
                "TypeError",
                "signal type 'Spin'",
            ),
            n(
                "via_clause_does_not_declare_signals",
                "module C\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: SignalC with SignalD via cross_correlate\n end\nend\nend",
                "TypeError",
                "signal type 'SignalD'",
            ),
            n(
                "second_correlation_unknown",
                "module C\nfn f :: A -> B -> Unit\n todo\nend\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: A with B\n correlate: A with Ghost\n end\nend\nend",
                "TypeError",
                "signal type 'Ghost'",
            ),
        ],
    );
}

#[test]
fn pos_resonance() {
    run_pos(
        "resonance",
        &[],
        chk!(ResonanceChecker::new()),
        &[
            p("signals_from_fn_params", "module C\nfn f :: Temperature -> Pressure -> Unit\n todo\nend\nbeing O\n telos: \"t\"\n end\n resonance:\n correlate: Temperature with Pressure\n end\nend\nend"),
            p("no_resonance_block", "module C\nbeing O\n telos: \"t\"\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 36. UnitsChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_units() {
    run_neg(
        "units",
        true,
        chk!(UnitsChecker::new()),
        &[
            n(
                "usd_plus_eur",
                "module M\nfn f :: Float<usd> -> Float<eur> -> Float<usd>\n a + b\nend\nend",
                "TypeError",
                "unit mismatch: cannot add/subtract Float<usd> and Float<eur>",
            ),
            n(
                "celsius_minus_fahrenheit",
                "module M\nfn f :: Float<celsius> -> Float<fahrenheit> -> Float<celsius>\n a - b\nend\nend",
                "TypeError",
                "Float<celsius> and Float<fahrenheit>",
            ),
            n(
                "mismatch_through_let_binding",
                "module M\nfn f :: Float<usd> -> Float<eur> -> Float<usd>\n let t = a\n t + b\nend\nend",
                "TypeError",
                "unit mismatch",
            ),
            n(
                "money_plus_rate",
                "module M\nfn f :: Float<usd> -> Float<rate> -> Float<usd>\n a + b\nend\nend",
                "TypeError",
                "Float<usd> and Float<rate>",
            ),
            n(
                "second_operand_chain",
                "module M\nfn f :: Float<usd> -> Float<usd> -> Float<eur> -> Float<usd>\n a + b + c\nend\nend",
                "TypeError",
                "Float<usd> and Float<eur>",
            ),
            n(
                "int_units_meters_seconds",
                "module M\nfn f :: Int<meters> -> Int<seconds> -> Int<meters>\n a + b\nend\nend",
                "TypeError",
                "Float<meters> and Float<seconds>",
            ),
        ],
    );
}

#[test]
fn pos_units() {
    run_pos(
        "units",
        &[],
        chk!(UnitsChecker::new()),
        &[
            p("same_unit_add", "module M\nfn f :: Float<usd> -> Float<usd> -> Float<usd>\n a + b\nend\nend"),
            p("same_unit_sub_via_let", "module M\nfn f :: Float<eur> -> Float<eur> -> Float<eur>\n let t = a\n t - b\nend\nend"),
            p("multiplication_of_units_not_checked", "module M\nfn f :: Float<usd> -> Float<rate> -> Float<usd>\n a * b\nend\nend"),
            p("unitless_mixed_with_unit", "module M\nfn f :: Float -> Float<usd> -> Float<usd>\n a + b\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 37. RefinementChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_refinement() {
    run_neg(
        "refinement",
        true,
        chk!(RefinementChecker::new()),
        &[
            n(
                "unknown_base_type",
                "module T\ntype X = Mystery where self > 0\nend",
                "TypeError",
                "refined type base `Mystery` is not a known type",
            ),
            n(
                "impure_call_in_predicate",
                "module T\ntype Bad = Int where foo(self)\nend",
                "TypeError",
                "function call `foo` not allowed in refinement predicate",
            ),
            n(
                "call_nested_under_and",
                "module T\ntype Bad = Int where self > 0 and foo(self)\nend",
                "TypeError",
                "`foo`",
            ),
            n(
                "field_access_unsupported",
                "module T\ntype Bad = String where self.length > 0\nend",
                "TypeError",
                "unsupported expression in refinement predicate",
            ),
            n(
                "second_type_unknown_base",
                "module T\ntype A = Int where self > 0\ntype B = Ghost where self < 9\nend",
                "TypeError",
                "base `Ghost`",
            ),
            n(
                "both_types_bad_both_reported",
                "module T\ntype A = Int where f(self)\ntype B = Int where g(self)\nend",
                "TypeError",
                "function call `g` not allowed",
            ),
        ],
    );
}

#[test]
fn pos_refinement() {
    run_pos(
        "refinement",
        &[],
        chk!(RefinementChecker::new()),
        &[
            p("simple_comparison", "module T\ntype Pos = Int where self > 0\nend"),
            p("compound_and", "module T\ntype B = Int where self >= 0 and self <= 100\nend"),
            p("refines_user_type", "module T\ntype A = Int where self > 0\ntype B = A where self < 9\nend"),
            p("bare_identifier_validator", "module T\ntype E = String where valid_email\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 38. TeleosChecker (via TeleosCheckerAdapter)
// ═════════════════════════════════════════════════════════════════════════════

const AUTO_FULL: &str = "module T\nbeing Cell\n autopoietic: true\n matter:\n membrane: Float\n end\n telos: \"self-organise\"\n end\n regulate glucose\n target: optimal\n bounds: (low, high)\n end\n evolve\n toward: telos\n search:\n | gradient_descent when gradient_available\n constraint: \"E[distance_to_telos] decreasing\"\n end\nend\nend";

#[test]
fn neg_teleos() {
    run_neg(
        "teleos",
        true,
        chk!(TeleosCheckerAdapter),
        &[
            n(
                "being_without_telos",
                "module T\nbeing B\n matter:\n x: Float\n end\nend\nend",
                "TypeError",
                "being 'B' has no telos:",
            ),
            n(
                "autopoietic_missing_regulate",
                "module T\nbeing Cell\n autopoietic: true\n matter:\n m: Float\n end\n telos: \"x\"\n end\n evolve\n toward: telos\n search:\n | gradient_descent when gradient_available\n constraint: \"E[d] decreasing\"\n end\nend\nend",
                "TypeError",
                "regulate",
            ),
            n(
                "autopoietic_missing_evolve",
                "module T\nbeing Cell\n autopoietic: true\n matter:\n m: Float\n end\n telos: \"x\"\n end\n regulate g\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "evolve",
            ),
            n(
                "autopoietic_missing_matter",
                "module T\nbeing Cell\n autopoietic: true\n telos: \"x\"\n end\n regulate g\n target: optimal\n bounds: (low, high)\n end\n evolve\n toward: telos\n search:\n | gradient_descent when gradient_available\n constraint: \"E[d] decreasing\"\n end\nend\nend",
                "TypeError",
                "matter",
            ),
            n(
                "regulate_without_bounds",
                "module T\nbeing Cell\n telos: \"x\"\n end\n regulate glucose\n target: optimal\n end\nend\nend",
                "TypeError",
                "regulate 'glucose' in being 'Cell' has no bounds:",
            ),
            n(
                "evolve_constraint_not_convergent",
                "module T\nbeing Cell\n telos: \"x\"\n end\n evolve\n toward: telos\n search:\n | gradient_descent when gradient_available\n constraint: \"just do whatever\"\n end\nend\nend",
                "TypeError",
                "must assert convergence",
            ),
            n(
                "thresholds_inverted",
                "module T\nbeing Cell\n telos: \"x\"\n thresholds:\n convergence: 0.3\n divergence: 0.8\n end\n end\nend\nend",
                "TypeError",
                "divergence",
            ),
            n(
                "threshold_out_of_range",
                "module T\nbeing Cell\n telos: \"x\"\n thresholds:\n convergence: 1.5\n divergence: 0.3\n end\n end\nend\nend",
                "TypeError",
                "convergence threshold 1.500 out of range",
            ),
            n(
                "telomere_zero_limit",
                "module T\nbeing Cell\n telos: \"x\"\n end\n telomere:\n limit: 0\n on_exhaustion: senescence\n end\nend\nend",
                "TypeError",
                "telomere limit must be positive",
            ),
            n(
                "ecosystem_signal_from_undefined_being",
                "module T\nbeing A\n telos: \"a\"\n end\nend\necosystem E\n members: [A]\n signal S from Ghost to A\n payload: Float\n end\nend\nend",
                "TypeError",
                "'from' being 'Ghost' is not defined",
            ),
        ],
    );
}

#[test]
fn pos_teleos() {
    run_pos(
        "teleos",
        &[],
        chk!(TeleosCheckerAdapter),
        &[
            p("complete_autopoietic_being", AUTO_FULL),
            p("minimal_being_with_telos", "module T\nbeing B\n telos: \"x\"\n end\nend\nend"),
            p("valid_thresholds", "module T\nbeing C\n telos: \"x\"\n thresholds:\n convergence: 0.8\n divergence: 0.3\n end\n end\nend\nend"),
            p("regulate_with_bounds", "module T\nbeing C\n telos: \"x\"\n end\n regulate g\n target: optimal\n bounds: (low, high)\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 39. TelosFunctionChecker (not wired into loom::compile — checker called directly)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_telos_fn() {
    run_neg(
        "telos_fn",
        false,
        chk!(TelosFunctionChecker::new()),
        &[
            n(
                "missing_measured_by",
                "module T\ntelos_function g\n guides: [axis]\nend\nend",
                "TypeError",
                "missing 'measured_by'",
            ),
            n(
                "empty_guides",
                "module T\ntelos_function g\n measured_by: \"S -> Float\"\nend\nend",
                "TypeError",
                "'guides' list is empty",
            ),
            n(
                "convergence_not_above_divergence",
                "module T\ntelos_function g\n measured_by: \"S -> Float\"\n guides: [a]\n thresholds:\n convergence: 0.3\n divergence: 0.8\n end\nend\nend",
                "TypeError",
                "convergence threshold (0.3) must be strictly greater than divergence",
            ),
            n(
                "warning_outside_stress_zone",
                "module T\ntelos_function g\n measured_by: \"S -> Float\"\n guides: [a]\n thresholds:\n convergence: 0.85\n divergence: 0.30\n warning: 0.95\n end\nend\nend",
                "TypeError",
                "warning threshold (0.95) must be strictly between",
            ),
            n(
                "propagation_below_convergence",
                "module T\ntelos_function g\n measured_by: \"S -> Float\"\n guides: [a]\n thresholds:\n convergence: 0.85\n divergence: 0.30\n propagation: 0.5\n end\nend\nend",
                "TypeError",
                "propagation threshold (0.5) must be",
            ),
            n(
                "both_missing_reports_both",
                "module T\ntelos_function g\nend\nend",
                "TypeError",
                "'guides' list is empty",
            ),
        ],
    );
}

#[test]
fn pos_telos_fn() {
    run_pos(
        "telos_fn",
        &[],
        chk!(TelosFunctionChecker::new()),
        &[
            p("complete", "module T\ntelos_function g\n measured_by: \"S -> Float\"\n guides: [a, b]\n thresholds:\n convergence: 0.85\n divergence: 0.30\n warning: 0.55\n propagation: 0.90\n end\nend\nend"),
            p("no_thresholds", "module T\ntelos_function g\n measured_by: \"S -> Float\"\n guides: [a]\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 40. ConservationChecker (not wired into loom::compile)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_conservation() {
    run_neg(
        "conservation",
        false,
        chk!(ConservationChecker::new()),
        &[
            n(
                "quantity_nowhere_in_signature",
                "module M\nfn f @conserved(Mass) :: Int -> Int\nend\nend",
                "TypeError",
                "does not appear in parameter or return types",
            ),
            n(
                "quantity_only_in_params",
                "module M\nfn f @conserved(Mass) :: Mass -> Int\nend\nend",
                "TypeError",
                "appears in parameters but not in return type",
            ),
            n(
                "quantity_only_in_return",
                "module M\nfn f @conserved(Mass) :: Int -> Mass\nend\nend",
                "TypeError",
                "appears in return type but not in parameters",
            ),
            n(
                "information_dropped_on_return",
                "module M
fn f @conserved(Information) :: Bit -> Int
end
end",
                "TypeError",
                "appears in parameters but not in return type",
            ),
            n(
                "second_of_two_quantities_unconserved",
                "module M\nfn f @conserved(Energy) @conserved(Momentum) :: Energy -> Energy\nend\nend",
                "TypeError",
                "quantity 'Momentum'",
            ),
        ],
    );
}

#[test]
fn pos_conservation() {
    run_pos(
        "conservation",
        &[],
        chk!(ConservationChecker::new()),
        &[
            p("mass_in_and_out", "module M\nfn f @conserved(Mass) :: Mass -> Mass\nend\nend"),
            p("two_quantities_in_and_out", "module M\nfn f @conserved(Energy) @conserved(Momentum) :: Energy -> Momentum -> (Energy, Momentum)\nend\nend"),
            p("no_conserved_annotation", "module M\nfn f :: Int -> Int\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 41. DisciplineChecker (not wired into loom::compile)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_discipline() {
    run_neg(
        "discipline",
        false,
        chk!(DisciplineChecker::new()),
        &[
            n(
                "duplicate_discipline",
                "module O\n discipline CQRS for Store end\n discipline CQRS for Store end\nend",
                "TypeError",
                "is declared more than once",
            ),
            n(
                "circuit_breaker_zero_attempts",
                "module O\n discipline CircuitBreaker for Svc\n max_attempts: 0\n end\nend",
                "TypeError",
                "max_attempts must be >= 1",
            ),
            n(
                "circuit_breaker_absurd_attempts",
                "module O\n discipline CircuitBreaker for Svc\n max_attempts: 500\n end\nend",
                "TypeError",
                "unreasonably high",
            ),
            n(
                "event_sourcing_without_events",
                "module O\n discipline CQRS for S end\n discipline EventSourcing for S end\nend",
                "TypeError",
                "no 'events:' list declared",
            ),
            n(
                "event_sourcing_without_cqrs",
                "module O\n discipline EventSourcing for S\n events: [Created]\n end\nend",
                "TypeError",
                "EventSourcing implies",
            ),
            n(
                "dependency_injection_without_binds",
                "module O\n discipline DependencyInjection for App end\nend",
                "TypeError",
                "no 'binds:' list declared",
            ),
            n(
                "saga_without_steps",
                "module O\n discipline Saga for Checkout end\nend",
                "TypeError",
                "no 'steps:' list declared",
            ),
        ],
    );
}

#[test]
fn pos_discipline() {
    run_pos(
        "discipline",
        &[],
        chk!(DisciplineChecker::new()),
        &[
            p("cqrs_plus_event_sourcing", "module O\n discipline CQRS for S end\n discipline EventSourcing for S\n events: [Created, Shipped]\n end\nend"),
            p("circuit_breaker_in_range", "module O\n discipline CircuitBreaker for Svc\n max_attempts: 5\n end\nend"),
            p("saga_with_steps", "module O\n discipline Saga for C\n steps: [Reserve, Charge]\n end\nend"),
            p("same_kind_different_targets", "module O\n discipline CQRS for A end\n discipline CQRS for B end\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 42. InfoFlowChecker (not wired into loom::compile)
// ═════════════════════════════════════════════════════════════════════════════

const FLOW_HEAD: &str = "module A\nflow secret :: Password, Token\nflow tainted :: UserInput\nflow public :: UserId, Bool\n";

#[test]
fn neg_infoflow() {
    let mk = |body: &str| -> &'static str { Box::leak(format!("{FLOW_HEAD}{body}\nend").into_boxed_str()) };
    run_neg(
        "infoflow",
        false,
        &|m: &Module| InfoFlowChecker::new().check(m).err().unwrap_or_default(),
        &[
            n(
                "secret_to_public_return",
                mk("fn leak :: Password -> UserId\n todo\nend"),
                "TypeError",
                "takes @secret 'Password' but returns @public 'UserId'",
            ),
            n(
                "secret_to_unlabelled_return",
                mk("fn leak :: Token -> Bool\n todo\nend"),
                "TypeError",
                "information flow violation",
            ),
            n(
                "tainted_into_db_operation",
                mk("fn insert_user :: UserInput -> Int\n todo\nend"),
                "TypeError",
                "@tainted input 'UserInput' flows to DB operation 'insert_user'",
            ),
            n(
                "eraser_without_labelled_param",
                mk("fn wipe @eraser :: Int -> Int\n todo\nend"),
                "TypeError",
                "@eraser function 'wipe' has no flow-labeled parameters",
            ),
            n(
                "reader_without_labelled_param",
                mk("fn peek @reader :: Int -> Int\n todo\nend"),
                "TypeError",
                "@reader function 'peek' has no flow-labeled parameters",
            ),
        ],
    );
    run_pos(
        "infoflow",
        &[],
        &|m: &Module| InfoFlowChecker::new().check(m).err().unwrap_or_default(),
        &[
            p("declassify_by_name", mk("fn declassify_password :: Password -> UserId\n todo\nend")),
            p("eraser_with_labelled_param", mk("fn wipe @eraser :: Password -> UserId\n todo\nend")),
            p("secret_to_secret", mk("fn rotate :: Token -> Token\n todo\nend")),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 43. EntityChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_entity() {
    run_neg(
        "entity",
        true,
        chk!(EntityChecker::new()),
        &[
            n(
                "directed_and_undirected",
                "module T\nentity Bad @directed @undirected end\nend",
                "TypeError",
                "@directed and @undirected are mutually exclusive",
            ),
            n(
                "acyclic_without_direction",
                "module T\nentity Bad @acyclic end\nend",
                "TypeError",
                "@acyclic requires @directed or @undirected",
            ),
            n(
                "layered_without_directed",
                "module T\nentity Bad @layered end\nend",
                "TypeError",
                "@layered requires @directed",
            ),
            n(
                "hierarchical_without_directed_and_acyclic",
                "module T\nentity Bad @hierarchical end\nend",
                "TypeError",
                "@hierarchical requires @directed",
            ),
            n(
                "stochastic_and_deterministic",
                "module T\nentity Bad<N, Float> @stochastic @deterministic end\nend",
                "TypeError",
                "@stochastic and @deterministic are mutually exclusive",
            ),
            n(
                "learnable_without_weighted",
                "module T\nentity Bad @learnable end\nend",
                "TypeError",
                "@learnable requires @weighted",
            ),
            n(
                "causal_without_directed",
                "module T\nentity Bad @causal end\nend",
                "TypeError",
                "@causal requires @directed",
            ),
            n(
                "weighted_without_edge_type",
                "module T\nentity Bad @weighted end\nend",
                "TypeError",
                "@weighted requires an edge type parameter",
            ),
        ],
    );
}

#[test]
fn pos_entity() {
    run_pos(
        "entity",
        &[],
        chk!(EntityChecker::new()),
        &[
            p("directed_acyclic", "module T\nentity G<Module, Import> @directed @acyclic end\nend"),
            p("hierarchical_complete", "module T\nentity H @directed @acyclic @hierarchical end\nend"),
            p("weighted_with_edge_type", "module T\nentity W<N, Float> @weighted end\nend"),
            p("learnable_weighted", "module T\nentity L<N, Float> @weighted @learnable end\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 44. SemiosisChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_semiosis() {
    run_neg(
        "semiosis",
        false,
        chk!(SemiosisChecker::new()),
        &[
            n(
                "sign_without_handler_no_function_block",
                "module S\nbeing B\n telos: \"t\"\n sign: Alarm\n end\nend\nend",
                "TypeError",
                "telos.sign 'Alarm' but has no function handler",
            ),
            n(
                "sign_handler_has_wrong_param_type",
                "module S\nbeing B\n telos: \"t\"\n sign: Alarm\n end\n function:\n fn on_other :: Heartbeat -> Unit\n todo\n end\n end\nend\nend",
                "TypeError",
                "being 'B'",
            ),
            n(
                "sign_handler_only_in_return_type",
                "module S\nbeing B\n telos: \"t\"\n sign: Alarm\n end\n function:\n fn make :: Int -> Alarm\n todo\n end\n end\nend\nend",
                "TypeError",
                "semiosis warning",
            ),
            n(
                "second_being_unhandled",
                "module S\nbeing A\n telos: \"t\"\n sign: Ping\n end\n function:\n fn h :: Ping -> Unit\n todo\n end\n end\nend\nbeing B\n telos: \"t\"\n sign: Pong\n end\nend\nend",
                "TypeError",
                "being 'B' declares telos.sign 'Pong'",
            ),
            n(
                "multiple_handlers_none_match",
                "module S\nbeing B\n telos: \"t\"\n sign: Alarm\n end\n function:\n fn a :: Int -> Unit\n todo\n end\n fn b :: String -> Unit\n todo\n end\n end\nend\nend",
                "TypeError",
                "no function handler with that parameter type",
            ),
        ],
    );
}

#[test]
fn pos_semiosis() {
    run_pos(
        "semiosis",
        &[],
        chk!(SemiosisChecker::new()),
        &[
            p("handler_present", "module S\nbeing B\n telos: \"t\"\n sign: Alarm\n end\n function:\n fn h :: Alarm -> Unit\n todo\n end\n end\nend\nend"),
            p("no_sign_declared", "module S\nbeing B\n telos: \"t\"\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 45. SignalAttentionChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_signal_attention() {
    let mk = |body: &str| -> &'static str {
        Box::leak(
            format!("module M\nbeing S\n telos: \"d\"\n end\n signal_attention\n{body}\n end\nend\nend")
                .into_boxed_str(),
        )
    };
    run_neg(
        "signal_attention",
        true,
        chk!(SignalAttentionChecker::new()),
        &[
            n(
                "inverted_window",
                mk(" prioritize: 0.3\n attenuate: 0.7"),
                "TypeError",
                "prioritize_above (0.300) must be strictly greater than attenuate_below (0.700)",
            ),
            n(
                "equal_window",
                mk(" prioritize: 0.5\n attenuate: 0.5"),
                "TypeError",
                "must be strictly greater",
            ),
            n(
                "prioritize_above_one",
                mk(" prioritize: 1.5\n attenuate: 0.2"),
                "TypeError",
                "prioritize_above 1.500 out of range [0.0, 1.0]",
            ),
            n(
                "attenuate_above_one",
                mk(" prioritize: 2.0\n attenuate: 1.7"),
                "TypeError",
                "attenuate_below 1.700 out of range",
            ),
            n(
                "prioritize_alone_out_of_range",
                mk(" prioritize: 3.0"),
                "TypeError",
                "out of range",
            ),
        ],
    );
    run_pos(
        "signal_attention",
        &[],
        chk!(SignalAttentionChecker::new()),
        &[
            p("valid_window", mk(" prioritize: 0.7\n attenuate: 0.2")),
            p("boundary_values", mk(" prioritize: 1.0\n attenuate: 0.0")),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 46. MinimalChecker  (suppresses only `[info]`; `[error]` and `[warn]` surface)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_minimal() {
    run_neg(
        "minimal",
        true,
        chk!(MinimalChecker::new()),
        &[
            n(
                "regulate_field_not_in_matter",
                "module M\nbeing B\n telos: \"t\"\n end\n matter:\n temp: Float\n end\n regulate glucose\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "regulated field 'glucose'",
            ),
            n(
                "second_regulate_not_in_matter",
                "module M\nbeing B\n telos: \"t\"\n end\n matter:\n temp: Float\n end\n regulate temp\n target: optimal\n bounds: (low, high)\n end\n regulate ph\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "regulate 'ph' in being 'B'",
            ),
            n(
                "regulate_typo_of_matter_field",
                "module M\nbeing B\n telos: \"t\"\n end\n matter:\n temperature: Float\n end\n regulate temprature\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "'temprature'",
            ),
            n(
                "second_being_violates",
                "module M\nbeing A\n telos: \"t\"\n end\n matter:\n x: Float\n end\n regulate x\n target: optimal\n bounds: (low, high)\n end\nend\nbeing C\n telos: \"t\"\n end\n matter:\n y: Float\n end\n regulate z\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "in being 'C'",
            ),
            n(
                "regulate_unknown_among_many_matter_fields",
                "module M\nbeing B\n telos: \"t\"\n end\n matter:\n a: Float\n b: Float\n c: Float\n end\n regulate d\n target: optimal\n bounds: (low, high)\n end\nend\nend",
                "TypeError",
                "regulated field 'd'",
            ),
            n(
                "unused_sense_channel_is_flagged",
                "module M\nsense Eyes\n channels: [light]\nend\nbeing B\n telos: \"t\"\n end\n matter:\n x: Float\n end\nend\nend",
                "TypeError",
                "sense channel 'light'",
            ),
        ],
    );
}

#[test]
fn pos_minimal() {
    run_pos(
        "minimal",
        &["[info]"],
        chk!(MinimalChecker::new()),
        &[
            p("regulate_declared_matter_field", "module M\nbeing B\n telos: \"t\"\n end\n matter:\n temp: Float\n end\n regulate temp\n target: optimal\n bounds: (low, high)\n end\nend\nend"),
            p("being_without_matter_not_checked", "module M\nbeing B\n telos: \"t\"\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 47. MigrationChecker
// ═════════════════════════════════════════════════════════════════════════════

const MIG_HEAD: &str = "module S\nbeing T\n telos: \"t\"\n end\n matter:\n a: Float\n b: Float\n end\n";

#[test]
fn neg_migration() {
    let mk = |body: &str| -> &'static str {
        Box::leak(format!("{MIG_HEAD}{body}\nend\nend").into_boxed_str())
    };
    run_neg(
        "migration",
        true,
        chk!(MigrationChecker::new()),
        &[
            n(
                "duplicate_migration_name",
                mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end\n migration v1:\n from: b Float\n to: b Int\n adapter: \"g\"\n end"),
                "TypeError",
                "duplicate migration name 'v1'",
            ),
            n(
                "non_breaking_without_adapter",
                mk(" migration v2:\n from: a Float\n to: a Float\n breaking: false\n end"),
                "TypeError",
                "breaking: false requires an adapter",
            ),
            n(
                "broken_type_chain",
                mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end\n migration v2:\n from: a Bool\n to: a String\n adapter: \"g\"\n end"),
                "TypeError",
                "migration chain broken for field 'a'",
            ),
            n(
                "migration_cycle",
                mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end\n migration v2:\n from: a Int\n to: a Float\n adapter: \"g\"\n end"),
                "TypeError",
                "migration cycle detected for field 'a'",
            ),
            n(
                "non_breaking_without_adapter_second_migration",
                mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end\n migration v3:\n from: b Float\n to: b Float\n breaking: false\n end"),
                "TypeError",
                "migration 'v3' in being 'T'",
            ),
        ],
    );
    run_pos(
        "migration",
        &["[warn]", "[info]"],
        chk!(MigrationChecker::new()),
        &[
            p("single_with_adapter", mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end")),
            p("consistent_chain", mk(" migration v1:\n from: a Float\n to: a Int\n adapter: \"f\"\n end\n migration v2:\n from: a Int\n to: a String\n adapter: \"g\"\n end")),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 48. ManifestChecker  (hard error: artifact path must exist on disk)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_manifest() {
    let mk = |paths: &str| -> &'static str {
        Box::leak(
            format!("module S\nbeing Agent\n telos: \"t\"\n end\n manifest:\n{paths}\n end\nend\nend")
                .into_boxed_str(),
        )
    };
    run_neg(
        "manifest",
        true,
        chk!(ManifestChecker::new()),
        &[
            n(
                "missing_markdown_artifact",
                mk(" artifact \"NOPE_DOES_NOT_EXIST.md\" reflects: [Agent] end"),
                "ParseError",
                "manifest artifact 'NOPE_DOES_NOT_EXIST.md' does not exist on disk",
            ),
            n(
                "missing_in_subdirectory",
                mk(" artifact \"docs/ghost/readme.md\" reflects: [Agent] end"),
                "ParseError",
                "does not exist on disk",
            ),
            n(
                "one_of_two_missing",
                mk(" artifact \"Cargo.toml\" reflects: [Agent] end\n artifact \"missing_file.txt\" reflects: [Agent] end"),
                "ParseError",
                "'missing_file.txt'",
            ),
            n(
                "missing_with_unknown_reflected_symbol",
                mk(" artifact \"absent.md\" reflects: [Agent, Ghost] end"),
                "ParseError",
                "does not exist on disk",
            ),
            n(
                "missing_directory_path",
                mk(" artifact \"no_such_dir/\" reflects: [Agent] end"),
                "ParseError",
                "being 'Agent'",
            ),
        ],
    );
    run_pos(
        "manifest",
        &["[warn]"],
        chk!(ManifestChecker::new()),
        &[
            p("existing_artifact", mk(" artifact \"Cargo.toml\" reflects: [Agent] end")),
            p("existing_directory", mk(" artifact \"src\" reflects: [Agent] end")),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 49. UseCaseChecker  (hard: duplicate criterion; the rest are [warn] diagnostics)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_usecase() {
    run_neg(
        "usecase",
        false,
        chk!(UseCaseChecker::new()),
        &[
            n(
                "duplicate_acceptance_criterion",
                "module U\n usecase Login:\n actor: User\n precondition: user_exists\n trigger: POST\n postcondition: session_created\n acceptance:\n test ok: correct credentials\n test ok: other credentials\n end\n end\nend",
                "ParseError",
                "duplicate acceptance criterion name 'ok'",
            ),
            n(
                "empty_acceptance_block",
                "module U\n usecase Login:\n actor: User\n precondition: user_exists\n trigger: POST\n postcondition: session_created\n acceptance:\n end\n end\nend",
                "ParseError",
                "acceptance: block is empty",
            ),
            n(
                "tautological_precondition",
                "module U\n usecase Login:\n actor: User\n precondition: true\n trigger: POST\n postcondition: session_created\n acceptance:\n test a: x\n end\n end\nend",
                "ParseError",
                "precondition: 'true' is a tautology",
            ),
            n(
                "postcondition_equals_precondition",
                "module U\n usecase Login:\n actor: User\n precondition: same_state\n trigger: POST\n postcondition: same_state\n acceptance:\n test a: x\n end\n end\nend",
                "ParseError",
                "identical to precondition",
            ),
            n(
                "duplicate_in_second_usecase",
                "module U\n usecase A:\n actor: U\n precondition: p\n trigger: GET\n postcondition: q\n acceptance:\n test one: x\n end\n end\n usecase B:\n actor: U\n precondition: p\n trigger: GET\n postcondition: q\n acceptance:\n test dup: x\n test dup: y\n end\n end\nend",
                "ParseError",
                "usecase 'B': duplicate acceptance criterion name 'dup'",
            ),
            n(
                "duplicate_adjacent_pair",
                "module U\n usecase A:\n actor: U\n precondition: p\n trigger: GET\n postcondition: q\n acceptance:\n test same: x\n test same: x\n end\n end\nend",
                "ParseError",
                "duplicate acceptance criterion name 'same'",
            ),
            n(
                "duplicate_with_four_criteria",
                "module U\n usecase A:\n actor: U\n precondition: p\n trigger: GET\n postcondition: q\n acceptance:\n test a: x\n test b: x\n test c: x\n test b: y\n end\n end\nend",
                "ParseError",
                "criterion name 'b'",
            ),
            n(
                "duplicate_among_three",
                "module U\n usecase Login:\n actor: User\n precondition: p\n trigger: POST\n postcondition: q\n acceptance:\n test a: x\n test b: y\n test a: z\n end\n end\nend",
                "ParseError",
                "duplicate acceptance criterion name 'a'",
            ),
        ],
    );
    run_pos(
        "usecase",
        &["[warn]"],
        chk!(UseCaseChecker::new()),
        &[
            p("well_formed", "module U\n usecase Login:\n actor: User\n precondition: user_exists\n trigger: POST\n postcondition: session_created\n acceptance:\n test a: x\n test b: y\n end\n end\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 50. JournalChecker  (hard: keep last 0; the rest are [warn] diagnostics)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_journal() {
    run_neg(
        "journal",
        false,
        chk!(JournalChecker::new()),
        &[
            n(
                "keep_last_zero",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 0\n end\nend\nend",
                "TypeError",
                "keep: last 0",
            ),
            n(
                "evolve_step_without_evolve_block",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every evolve_step\n keep: last 10\n end\nend\nend",
                "TypeError",
                "records evolve_step but there is no evolve: block",
            ),
            nm(
                "telos_progress_without_telos",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 10\n end\nend\nend",
                |m| m.being_defs[0].telos = None,
                "TypeError",
                "records telos_progress but there is no telos: block",
            ),
            n(
                "autopoietic_without_journal",
                "module M\nbeing B\n autopoietic: true\n telos: \"t\"\n end\nend\nend",
                "TypeError",
                "autopoietic being 'B' has no journal:",
            ),
            n(
                "keep_last_zero_without_records",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n keep: last 0\n end\nend\nend",
                "TypeError",
                "zero-size ring buffer is invalid",
            ),
            n(
                "keep_last_zero_two_records",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n record: every evolve_step\n keep: last 0\n end\nend\nend",
                "TypeError",
                "journal: in being 'B' has keep: last 0",
            ),
            nm(
                "keep_last_mutated_to_zero",
                "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 5\n end\nend\nend",
                |m| m.being_defs[0].journal.as_mut().unwrap().keep_last = Some(0),
                "TypeError",
                "keep: last 0",
            ),
            n(
                "second_being_keep_zero",
                "module M\nbeing A\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 5\n end\nend\nbeing C\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 0\n end\nend\nend",
                "TypeError",
                "in being 'C'",
            ),
        ],
    );
    run_pos(
        "journal",
        &["[warn]"],
        chk!(JournalChecker::new()),
        &[
            p("keep_positive", "module M\nbeing B\n telos: \"t\"\n end\n journal:\n record: every telos_progress\n keep: last 100\n end\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 51. ScenarioChecker
// ═════════════════════════════════════════════════════════════════════════════

const SCN_OK: &str = "module T\nbeing Tr\n telos: \"t\"\n end\n scenario go:\n given: sig\n when: being sense detects sig\n then: ensure pos\n within: 3 lifecycle_ticks\n end\nend\nend";

#[test]
fn neg_scenario() {
    run_neg(
        "scenario",
        true,
        chk!(ScenarioChecker::new()),
        &[
            n(
                "within_zero_ticks",
                "module T\nbeing Tr\n telos: \"t\"\n end\n scenario go:\n given: sig\n when: being sense detects sig\n then: ensure pos\n within: 0 lifecycle_ticks\n end\nend\nend",
                "TypeError",
                "within: 0",
            ),
            nm(
                "empty_given",
                SCN_OK,
                |m| m.being_defs[0].scenarios[0].given.clear(),
                "TypeError",
                "scenario 'go' in being 'Tr' has empty given:",
            ),
            nm(
                "empty_when",
                SCN_OK,
                |m| m.being_defs[0].scenarios[0].when.clear(),
                "TypeError",
                "has empty when:",
            ),
            nm(
                "empty_then",
                SCN_OK,
                |m| m.being_defs[0].scenarios[0].then.clear(),
                "TypeError",
                "has empty then:",
            ),
            n(
                "second_scenario_within_zero",
                "module T\nbeing Tr\n telos: \"t\"\n end\n scenario a:\n given: s\n when: being sense detects s\n then: ensure p\n within: 2 lifecycle_ticks\n end\n scenario b:\n given: s\n when: being sense detects s\n then: ensure p\n within: 0 lifecycle_ticks\n end\nend\nend",
                "TypeError",
                "scenario 'b' in being 'Tr' has within: 0",
            ),
            n(
                "autopoietic_without_scenarios",
                "module T\nbeing Tr\n autopoietic: true\n telos: \"t\"\n end\nend\nend",
                "TypeError",
                "autopoietic being 'Tr' has no scenario: blocks",
            ),
        ],
    );
    run_pos(
        "scenario",
        &["[warn]"],
        chk!(ScenarioChecker::new()),
        &[p("complete_scenario", SCN_OK)],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 52. PropertyChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_property() {
    run_neg(
        "property",
        true,
        chk!(PropertyChecker::new()),
        &[
            n(
                "zero_samples",
                "module P\n property broken:\n forall x: Int\n invariant: x = x\n samples: 0\n end\nend",
                "ParseError",
                "samples must be > 0, got 0",
            ),
            n(
                "zero_samples_string_property",
                "module P\n property rt:\n forall s: String\n invariant: decode(s) = s\n samples: 0\n end\nend",
                "ParseError",
                "property 'rt'",
            ),
            n(
                "second_property_zero_samples",
                "module P\n property a:\n forall x: Int\n invariant: x = x\n samples: 10\n end\n property b:\n forall y: Int\n invariant: y = y\n samples: 0\n end\nend",
                "ParseError",
                "property 'b'",
            ),
            n(
                "invariant_ignores_quantified_var",
                "module P\n property c:\n forall x: Int\n invariant: 1 = 1\n samples: 10\n end\nend",
                "ParseError",
                "does not reference 'x'",
            ),
            n(
                "zero_samples_float_property",
                "module P\n property fp:\n forall x: Float\n invariant: x = x\n samples: 0\n end\nend",
                "ParseError",
                "property 'fp': samples must be > 0",
            ),
            n(
                "zero_samples_with_shrink",
                "module P\n property sh:\n forall x: Int\n invariant: x = x\n shrink: true\n samples: 0\n end\nend",
                "ParseError",
                "samples must be > 0, got 0",
            ),
            n(
                "invariant_references_other_name",
                "module P\n property d:\n forall item: Int\n invariant: other = other\n samples: 10\n end\nend",
                "ParseError",
                "quantified variable appears unused",
            ),
        ],
    );
    run_pos(
        "property",
        &["[warn]"],
        chk!(PropertyChecker::new()),
        &[p(
            "valid_property",
            "module P\n property ok:\n forall x: Int\n invariant: x = x\n samples: 100\n end\nend",
        )],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 53. ProvenanceChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_provenance() {
    run_neg(
        "provenance",
        true,
        chk!(ProvenanceChecker::new()),
        &[
            n(
                "provenance_on_pii_field",
                "module M\ntype U = email: String @provenance @pii end end",
                "TypeError",
                "field 'email' in type 'U': @provenance on @pii field creates linkability risk",
            ),
            n(
                "pii_then_provenance_order",
                "module M\ntype U = ssn: String @pii @provenance end end",
                "TypeError",
                "field 'ssn' in type 'U'",
            ),
            n(
                "second_field_violates",
                "module M\ntype U = id: Int @provenance name: String @pii @provenance end end",
                "TypeError",
                "field 'name'",
            ),
            n(
                "provenance_fn_calls_other_fn_is_flagged",
                "module M\nfn helper :: Int -> Int\n 1\nend\nfn traced @provenance :: Int -> Int\n helper(1)\nend\nend",
                "TypeError",
                "@provenance fn calls other fns",
            ),
            n(
                "provenance_on_pii_in_third_type",
                "module M\ntype A = x: Int end\ntype B = y: Int @provenance end\ntype C = z: String @provenance @pii end end",
                "TypeError",
                "field 'z' in type 'C'",
            ),
            n(
                "second_type_violates",
                "module M\ntype A = x: Int @provenance end\ntype B = y: String @provenance @pii end end",
                "TypeError",
                "in type 'B'",
            ),
        ],
    );
    run_pos(
        "provenance",
        &["[warn]"],
        chk!(ProvenanceChecker::new()),
        &[
            p("provenance_on_non_pii", "module M\ntype U = id: Int @provenance end end"),
            p("pii_without_provenance", "module M\ntype U = email: String @pii end end"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 54. BoundaryChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_boundary() {
    run_neg(
        "boundary",
        true,
        chk!(BoundaryChecker::new()),
        &[
            n(
                "exports_undeclared_name",
                "module M\nfn real :: Int -> Int\n require: true\n ensure: true\nend\nboundary:\n export: ghost\nend\nend",
                "TypeError",
                "exports 'ghost' which is not declared in this module",
            ),
            n(
                "private_type_in_exported_fn",
                "module M\ntype InternalHelper = data: String end\nfn public_api :: InternalHelper -> Int\n require: true\n ensure: true\nend\nboundary:\n export: public_api\n private: InternalHelper\nend\nend",
                "TypeError",
                "uses private type 'InternalHelper' in its signature",
            ),
            n(
                "second_export_undeclared",
                "module M\nfn real :: Int -> Int\n require: true\n ensure: true\nend\nboundary:\n export: real nothing\nend\nend",
                "TypeError",
                "exports 'nothing'",
            ),
            n(
                "being_exports_fn_missing_from_function_block",
                "module M\nbeing B\n telos: \"t\"\n end\n function:\n fn present :: Int -> Int\n todo\n end\n end\n boundary:\n export: absent\n end\nend\nend",
                "TypeError",
                "being 'B' exports 'absent' which is not declared in its function block",
            ),
            n(
                "private_type_in_second_param_of_exported_fn",
                "module M\ntype Secret = data: String end\nfn api :: Int -> Secret -> Int\n require: true\n ensure: true\nend\nboundary:\n export: api\n private: Secret\nend\nend",
                "TypeError",
                "fn 'api' is exported but uses private type 'Secret'",
            ),
            n(
                "undeclared_visibility_is_flagged",
                "module M\ntype A = v: Int end\ntype Z = v: Int end\nboundary:\n export: A\nend\nend",
                "TypeError",
                "'Z' is not listed",
            ),
        ],
    );
    run_pos(
        "boundary",
        &["[warn]"],
        chk!(BoundaryChecker::new()),
        &[
            p("clean_boundary", "module M\ntype PublicType = value: Int end\ntype InternalState = data: String end\nfn public_fn :: Int -> Int\n require: true\n ensure: true\nend\nboundary:\n export: PublicType public_fn\n private: InternalState\nend\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 55. CognitiveMemoryChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_cognitive_memory() {
    let mk = |body: &str| -> &'static str {
        Box::leak(
            format!("module M\nbeing Agent\n telos: \"agent\"\n end\n memory:\n{body}\n end\nend\nend")
                .into_boxed_str(),
        )
    };
    run_neg(
        "cognitive_memory",
        true,
        chk!(CognitiveMemoryChecker::new()),
        &[
            n(
                "decay_rate_above_one",
                mk(" type: semantic\n decay_rate: 1.5"),
                "TypeError",
                "decay_rate 1.5 is out of range [0.0, 1.0]",
            ),
            n(
                "episodic_without_journal",
                mk(" type: episodic"),
                "TypeError",
                "type episodic requires a journal: block",
            ),
            n(
                "procedural_without_migration",
                mk(" type: procedural"),
                "TypeError",
                "type procedural requires at least one migration: block",
            ),
            n(
                "architectural_without_manifest",
                mk(" type: architectural"),
                "TypeError",
                "type architectural requires a manifest: block",
            ),
            n(
                "insight_without_autopoietic",
                mk(" type: insight"),
                "TypeError",
                "type insight requires @autopoietic",
            ),
            n(
                "semantic_without_regulate_is_flagged",
                mk(" type: semantic"),
                "TypeError",
                "memory: type semantic declared but no matching source block",
            ),
        ],
    );
    run_pos(
        "cognitive_memory",
        &["[warn]"],
        chk!(CognitiveMemoryChecker::new()),
        &[
            p(
                "episodic_with_journal",
                "module M\nbeing Agent\n telos: \"agent\"\n end\n journal:\n record: every telos_progress\n keep: last 5\n end\n memory:\n type: episodic\n end\nend\nend",
            ),
            p(
                "semantic_with_regulate",
                "module M\nbeing Agent\n telos: \"agent\"\n end\n regulate g\n target: optimal\n bounds: (low, high)\n end\n memory:\n type: semantic\n decay_rate: 0.5\n end\nend\nend",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 56. EvolutionVectorChecker  (diagnostics are [warn]/[info] only; none is a hard error)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_evolution_vector() {
    let mk = |from: &str, to: &str| -> &'static str {
        Box::leak(
            format!(
                "module M\nbeing A\n telos: \"t\"\n end\n matter:\n f: {from}\n end\n migration m1:\n from: f {from}\n to: f {to}\n adapter: \"x\"\n end\nend\nbeing B\n telos: \"t\"\n end\n matter:\n f: {from}\n end\n migration m2:\n from: f {from}\n to: f {to}\n adapter: \"y\"\n end\nend\nend"
            )
            .into_boxed_str(),
        )
    };
    run_neg(
        "evolution_vector",
        false,
        chk!(EvolutionVectorChecker::new()),
        &[
            n(
                "duplicate_float_to_int",
                mk("Float", "Int"),
                "TypeError",
                "migration vector duplicate",
            ),
            n(
                "duplicate_string_to_text",
                mk("String", "Text"),
                "TypeError",
                "migration vector duplicate",
            ),
            n(
                "duplicate_int_to_decimal",
                mk("Int", "Decimal"),
                "TypeError",
                "migration vector duplicate",
            ),
            n(
                "duplicate_float_to_money",
                mk("Float", "Money"),
                "TypeError",
                "migration vector duplicate",
            ),
            n(
                "duplicate_bool_to_int",
                mk("Bool", "Int"),
                "TypeError",
                "migration vector duplicate",
            ),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 57. MessagingChecker
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_messaging() {
    run_neg(
        "messaging",
        true,
        chk!(MessagingChecker::new()),
        &[
            // NOTE: the parser stores `guarantees:` tokens as Debug strings, so the
            // checker's label matching only fires on hand-built / mutated ASTs.
            // The source-level versions of these three rows are the ignored
            // `gap_messaging_*` tests below.
            nm(
                "exactly_once_and_at_most_once",
                "module M\n messaging_primitive Q\n pattern: producer_consumer\n end\nend",
                |m| {
                    if let Item::MessagingPrimitive(mp) = &mut m.items[0] {
                        mp.guarantees = vec!["exactly-once".into(), "at-most-once".into()];
                    }
                },
                "TypeError",
                "'exactly-once' and 'at-most-once' are mutually exclusive",
            ),
            nm(
                "exactly_once_and_at_least_once",
                "module M\n messaging_primitive Q\n pattern: producer_consumer\n end\nend",
                |m| {
                    if let Item::MessagingPrimitive(mp) = &mut m.items[0] {
                        mp.guarantees = vec!["exactly_once".into(), "at_least_once".into()];
                    }
                },
                "TypeError",
                "'exactly-once' and 'at-least-once' are mutually exclusive",
            ),
            nm(
                "stream_with_exactly_once",
                "module M\n messaging_primitive S\n pattern: stream\n end\nend",
                |m| {
                    if let Item::MessagingPrimitive(mp) = &mut m.items[0] {
                        mp.guarantees = vec!["exactly-once".into()];
                    }
                },
                "TypeError",
                "'stream' pattern with 'exactly-once'",
            ),
            n(
                "request_response_without_timeout",
                "module M\n messaging_primitive R\n pattern: request_response\n end\nend",
                "TypeError",
                "'request_response' pattern should declare",
            ),
            nm(
                "empty_name",
                "module M\n messaging_primitive Q\n pattern: stream\n end\nend",
                |m| {
                    if let Item::MessagingPrimitive(mp) = &mut m.items[0] {
                        mp.name.clear();
                    }
                },
                "TypeError",
                "messaging_primitive has empty name",
            ),
            nm(
                "empty_guarantee_entry",
                "module M\n messaging_primitive Q\n pattern: stream\n guarantees: @idempotent\n end\nend",
                |m| {
                    if let Item::MessagingPrimitive(mp) = &mut m.items[0] {
                        mp.guarantees.push(String::new());
                    }
                },
                "TypeError",
                "guarantees list contains an empty entry",
            ),
        ],
    );
    run_pos(
        "messaging",
        &["[warn]"],
        chk!(MessagingChecker::new()),
        &[
            p("request_response_with_timeout", "module M\n messaging_primitive R\n pattern: request_response\n timeout: mandatory\n end\nend"),
            p("at_least_once_idempotent", "module M\n messaging_primitive Q\n pattern: producer_consumer\n guarantees: @at-least-once @idempotent\n end\nend"),
            p("pubsub", "module M\n messaging_primitive E\n pattern: publish_subscribe\n end\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 58. StoreChecker  (hard errors are un-prefixed; [hint]/[warn]/[info] are suppressed)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn neg_store() {
    run_neg(
        "store",
        true,
        chk!(StoreChecker::new()),
        &[
            n(
                "keyvalue_missing_key",
                "module S\n store K :: KeyValue\n value: Float\n end\nend",
                "TypeError",
                "KeyValue store must declare 'key: Type'",
            ),
            n(
                "keyvalue_missing_value",
                "module S\n store K :: KeyValue\n key: Int\n end\nend",
                "TypeError",
                "KeyValue store must declare 'value: Type'",
            ),
            n(
                "graph_without_node",
                "module S\n store G :: Graph\n end\nend",
                "TypeError",
                "Graph store must declare at least one 'node' entry",
            ),
            n(
                "timeseries_without_event",
                "module S\n store T :: TimeSeries\n end\nend",
                "TypeError",
                "TimeSeries store must declare at least one 'event' entry",
            ),
            n(
                "vector_without_embedding",
                "module S\n store V :: Vector\n end\nend",
                "TypeError",
                "Vector store must declare at least one 'embedding' entry",
            ),
            n(
                "columnar_without_schema",
                "module S\n store C :: Columnar\n end\nend",
                "TypeError",
                "Columnar store must declare at least one 'schema' entry",
            ),
            n(
                "snowflake_without_fact",
                "module S\n store F :: Snowflake\n end\nend",
                "TypeError",
                "Snowflake store must declare exactly one 'fact' entry (found 0)",
            ),
            n(
                "hypercube_without_fact",
                "module S\n store H :: Hypercube\n end\nend",
                "TypeError",
                "Hypercube store must declare at least one 'fact' entry",
            ),
            n(
                "distributed_without_schema",
                "module S\n store D :: Distributed\n end\nend",
                "TypeError",
                "Distributed store must declare at least one 'schema' entry",
            ),
        ],
    );
    run_pos(
        "store",
        &["[hint]", "[warn]", "[info]"],
        chk!(StoreChecker::new()),
        &[
            p("keyvalue_complete", "module S\n store K :: KeyValue\n key: Int\n value: Float\n end\nend"),
            p("vector_with_embedding", "module S\n store VectorDb :: Vector\n embedding :: { id: String, text: String, vector: Float }\n index: HNSW\n end\nend"),
        ],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 59. SmtBridgeChecker  (default build has no Z3: contradiction detection is the
//     only rejecting path that works without a solver)
// ═════════════════════════════════════════════════════════════════════════════

fn smt_contract_strings(src: &str) -> (String, String) {
    let m = parse(src);
    let res = SmtBridgeChecker::check(&m.items);
    assert_eq!(res.len(), 1, "expected exactly one verification result");
    (res[0].precondition.clone(), res[0].postcondition.clone())
}

#[test]
fn neg_smt_bridge_contradictions_detected() {
    let cases: &[(&str, &str)] = &[
        ("x > 5", "x < 3"),
        ("x > 10", "x < 10"),
        ("x > 0", "x < 0"),
        ("x > 100", "x < 5"),
        ("x > 1", "x < 1"),
    ];
    for (pre, post) in cases {
        let src = format!("module T\n fn f :: Int -> Int\n require: {pre}\n ensure: {post}\n end\nend");
        let (p_s, q_s) = smt_contract_strings(&src);
        assert!(
            SmtBridgeChecker::detect_contradiction(&p_s, &q_s),
            "contradiction not detected for require `{pre}` / ensure `{post}` (translated `{p_s}` / `{q_s}`)"
        );
    }
}

#[test]
fn pos_smt_bridge_consistent_contracts_not_flagged() {
    let cases: &[(&str, &str)] = &[("x > 0", "x < 10"), ("x > 5", "x < 6"), ("x >= 0", "x >= 0")];
    for (pre, post) in cases {
        let src = format!("module T\n fn f :: Int -> Int\n require: {pre}\n ensure: {post}\n end\nend");
        let (p_s, q_s) = smt_contract_strings(&src);
        assert!(
            !SmtBridgeChecker::detect_contradiction(&p_s, &q_s),
            "false contradiction for require `{pre}` / ensure `{post}`"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Soundness gaps (confirmed ACCEPTED when they should be rejected).
// Each is listed in experiments/maturity/evidence/m4/findings.md.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "soundness gap: ConservationChecker accepts @conserved(Mass) on a signature with no mass-typed value (hint 'g' substring-matches 'String')"]
fn gap_conservation_mass_matches_string_by_substring() {
    assert_rejected(
        chk!(ConservationChecker::new()),
        n(
            "mass_on_string_signature",
            "module M\nfn f @conserved(Mass) :: String -> String\nend\nend",
            "TypeError",
            "does not appear in parameter or return types",
        ),
    );
}

#[test]
#[ignore = "soundness gap: ConservationChecker accepts a conserved Energy quantity that is dropped on return when the return type is the generic hint 'Float'"]
fn gap_conservation_energy_laundered_through_float() {
    assert_rejected(
        chk!(ConservationChecker::new()),
        n(
            "energy_to_float",
            "module M\nfn f @conserved(Energy) :: Energy -> Float\nend\nend",
            "TypeError",
            "appears in parameters but not in return type",
        ),
    );
}

#[test]
#[ignore = "soundness gap: contradictory require/ensure (x > 5 vs x < 3) is accepted by loom::compile; SmtBridgeChecker::detect_contradiction exists but is never called and the Z3 path never yields Counterexample"]
fn gap_compile_accepts_contradictory_contract() {
    let src = "module T\n fn f :: Int -> Int\n require: x > 5\n ensure: x < 3\n end\nend";
    assert!(
        loom::compile(src).is_err(),
        "contradictory contract was accepted"
    );
}

#[test]
#[ignore = "soundness gap: source `guarantees: @exactly-once @at-most-once` is accepted; parser stores guarantee tokens as Debug strings so MessagingChecker's label match never fires on parsed source"]
fn gap_messaging_exactly_once_and_at_most_once_from_source() {
    assert_rejected(
        chk!(MessagingChecker::new()),
        n(
            "src_exactly_once_at_most_once",
            "module M\n messaging_primitive Q\n pattern: producer_consumer\n guarantees: @exactly-once @at-most-once\n end\nend",
            "TypeError",
            "'exactly-once' and 'at-most-once' are mutually exclusive",
        ),
    );
}

#[test]
#[ignore = "soundness gap: source `guarantees: @exactly-once @at-least-once` is accepted (same root cause as the at-most-once gap)"]
fn gap_messaging_exactly_once_and_at_least_once_from_source() {
    assert_rejected(
        chk!(MessagingChecker::new()),
        n(
            "src_exactly_once_at_least_once",
            "module M\n messaging_primitive Q\n pattern: producer_consumer\n guarantees: @exactly-once @at-least-once\n end\nend",
            "TypeError",
            "'exactly-once' and 'at-least-once' are mutually exclusive",
        ),
    );
}

#[test]
#[ignore = "soundness gap: source stream pattern with `guarantees: @exactly-once` is accepted (same root cause)"]
fn gap_messaging_stream_exactly_once_from_source() {
    assert_rejected(
        chk!(MessagingChecker::new()),
        n(
            "src_stream_exactly_once",
            "module M\n messaging_primitive S\n pattern: stream\n guarantees: @exactly-once\n end\nend",
            "TypeError",
            "'stream' pattern with 'exactly-once'",
        ),
    );
}

#[test]
fn pos_evolution_vector() {
    run_pos(
        "evolution_vector",
        &["[warn]", "[info]"],
        chk!(EvolutionVectorChecker::new()),
        &[
            p(
                "distinct_vectors_in_two_beings",
                "module M\nbeing A\n telos: \"t\"\n end\n matter:\n f: Float\n end\n migration m1:\n from: f Float\n to: f Int\n adapter: \"x\"\n end\nend\nbeing B\n telos: \"t\"\n end\n matter:\n g: String\n end\n migration m2:\n from: g String\n to: g Text\n adapter: \"y\"\n end\nend\nend",
            ),
            p(
                "no_migrations",
                "module M\nbeing A\n telos: \"t\"\n end\nend\nend",
            ),
        ],
    );
}

// @@NEXT@@
