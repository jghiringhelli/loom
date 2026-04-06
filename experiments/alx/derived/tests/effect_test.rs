//! Effect checker tests — exercises [`EffectChecker`] directly (lex → parse → check).
//!
//! We bypass the full `loom::compile` pipeline so that type-unification errors
//! from earlier passes don't shadow effect errors in our assertions.
//!
//! Loom function application uses parentheses: `f(arg)`, not `f arg`.
//!
//! The checker enforces:
//! 1. Pure functions (no `Effect<…>` return) may not call effectful functions.
//! 2. Effectful functions must declare *all* transitively-used effects.

use loom::checker::EffectChecker;
use loom::LoomError;

fn check_effects(src: &str) -> Result<(), Vec<LoomError>> {
    let tokens = loom::lexer::Lexer::tokenize(src).expect("lex failed");
    let module = loom::parser::Parser::new(&tokens)
        .parse_module()
        .expect("parse failed");
    EffectChecker::new().check(&module)
}

fn has_effect_error(src: &str) -> bool {
    check_effects(src)
        .err()
        .map(|errs| errs.iter().any(|e| e.kind() == "EffectError"))
        .unwrap_or(false)
}

fn is_ok(src: &str) -> bool {
    check_effects(src).is_ok()
}

// ── Pure modules / functions ──────────────────────────────────────────────────

#[test]
fn pure_empty_module_is_ok() {
    assert!(is_ok("module M end"));
}

#[test]
fn pure_fn_with_literal_body_is_ok() {
    let src = r#"
module M
fn answer :: Int -> Int
  42
end
end
"#;
    assert!(is_ok(src));
}

#[test]
fn pure_fn_calling_another_pure_fn_is_ok() {
    let src = r#"
module M
fn double :: Int -> Int
  0
end

fn quad :: Int -> Int
  double(0)
end
end
"#;
    assert!(is_ok(src));
}

// ── Effectful functions declared correctly ────────────────────────────────────

#[test]
fn effectful_fn_declared_with_io_is_ok() {
    let src = r#"
module M
fn fetch :: Int -> Effect<[IO], String>
  todo
end
end
"#;
    assert!(is_ok(src));
}

#[test]
fn effectful_fn_calling_effectful_fn_same_effect_is_ok() {
    let src = r#"
module M
fn read_db :: Int -> Effect<[DB], String>
  todo
end

fn process :: Int -> Effect<[DB], String>
  read_db(0)
end
end
"#;
    assert!(is_ok(src));
}

// ── Violations: pure function calling effectful ───────────────────────────────

#[test]
fn pure_fn_calling_effectful_fn_is_error() {
    let src = r#"
module M
fn fetch :: Int -> Effect<[IO], String>
  todo
end

fn orchestrate :: Int -> String
  fetch(1)
end
end
"#;
    assert!(
        has_effect_error(src),
        "expected effect error: pure fn calling effectful fn"
    );
}

#[test]
fn pure_fn_calling_transitive_effectful_fn_is_error() {
    let src = r#"
module M
fn read_db :: Int -> Effect<[DB], String>
  todo
end

fn process :: Int -> Effect<[DB], String>
  read_db(0)
end

fn run :: Int -> String
  process(1)
end
end
"#;
    // `run` is pure; transitively reaches DB via process → read_db.
    assert!(
        has_effect_error(src),
        "expected effect error: transitive effect not declared"
    );
}

// ── Violations: declared effects too narrow ───────────────────────────────────

#[test]
fn effectful_fn_missing_declared_effect_is_error() {
    let src = r#"
module M
fn send_email :: Int -> Effect<[IO], Bool>
  todo
end

fn do_both :: Int -> Effect<[DB], Bool>
  send_email(0)
end
end
"#;
    // `do_both` declares only [DB] but calls `send_email` which needs IO.
    assert!(
        has_effect_error(src),
        "expected effect error: IO effect used but not declared"
    );
}

// ── Effect set coverage ───────────────────────────────────────────────────────

#[test]
fn fn_declaring_superset_of_effects_is_ok() {
    let src = r#"
module M
fn write_db :: Int -> Effect<[DB], Bool>
  todo
end

fn do_work :: Int -> Effect<[IO, DB], Bool>
  write_db(0)
end
end
"#;
    // `do_work` declares [IO, DB]; calls `write_db` which uses only [DB]. Superset is ok.
    assert!(is_ok(src));
}

// ── Multiple functions, some pure some not ────────────────────────────────────

#[test]
fn mixed_module_pure_and_effectful_coexist() {
    let src = r#"
module M
fn pure_helper :: Int -> Int
  0
end

fn effectful_op :: Int -> Effect<[IO], String>
  todo
end

fn also_pure :: Int -> Bool
  true
end
end
"#;
    // Neither pure function calls the effectful one.
    assert!(is_ok(src));
}

// ── Corpus regression ─────────────────────────────────────────────────────────

#[test]
fn pricing_engine_corpus_no_effect_errors() {
    let src = std::fs::read_to_string("corpus/pricing_engine.loom").unwrap();
    assert!(!has_effect_error(&src), "pricing_engine.loom should have no effect errors");
}

#[test]
fn user_service_corpus_effectful_fns_accepted() {
    let src = std::fs::read_to_string("corpus/user_service.loom").unwrap();
    // user_service declares Effect<[IO], …> — the checker should accept those.
    assert!(!has_effect_error(&src), "user_service.loom should have no effect errors");
}
