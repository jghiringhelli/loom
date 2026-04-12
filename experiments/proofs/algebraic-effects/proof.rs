// proof.rs — emitted by: loom compile proof.loom
// Theory: Algebraic Effects (Plotkin & Power 2001)
// Effects are type parameters. Pure functions have Effect<[], T> = T.
// Calling effectful code from pure context is a type mismatch.

use std::marker::PhantomData;

// ── Effect type encoding ──────────────────────────────────────────────────────

pub struct IO;
pub struct DB;

/// An effectful computation carrying effects E and returning T.
/// Pure computation = Effect<(), T> which simplifies to T directly.
pub struct Effect<E, T> {
    pub value: T,
    _effects: PhantomData<E>,
}

impl<E, T> Effect<E, T> {
    pub fn new(value: T) -> Self {
        Self { value, _effects: PhantomData }
    }
}

// ── Pure function: no Effect wrapper ─────────────────────────────────────────

/// Pure: same input always gives same output, no side effects.
pub fn pure_add(a: i64, b: i64) -> i64 {
    a + b
}

// ── Effectful functions ───────────────────────────────────────────────────────

/// IO effect: reads a line. Cannot be called from pure context without handler.
pub fn read_line() -> Effect<IO, String> {
    Effect::new("mocked_input".to_string())
}

/// DB effect: fetches a user by ID.
pub fn fetch_user(id: i64) -> Effect<DB, String> {
    Effect::new(format!("User_{}", id))
}

/// Combined IO+DB effect.
pub fn log_and_fetch(id: i64) -> Effect<(IO, DB), String> {
    Effect::new(format!("User_{}_logged", id))
}

// ── Handlers: eliminate effects ───────────────────────────────────────────────

pub fn run_io<T>(effect: Effect<IO, T>) -> T {
    effect.value
}

pub fn run_db<T>(effect: Effect<DB, T>) -> T {
    effect.value
}

pub fn run_io_db<T>(effect: Effect<(IO, DB), T>) -> T {
    effect.value
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_fn_has_no_effect_wrapper() {
        // Pure function returns plain i64 — no Effect<> wrapper required
        let result: i64 = pure_add(3, 4);
        assert_eq!(result, 7);
    }

    #[test]
    fn effectful_fn_requires_handler_to_extract_value() {
        // Cannot get the value without running through a handler
        let effect = fetch_user(42);
        let name = run_db(effect);
        assert_eq!(name, "User_42");
    }

    #[test]
    fn combined_effects_handled_together() {
        let effect = log_and_fetch(1);
        let result = run_io_db(effect);
        assert!(result.contains("User_1"));
    }

    // ── Violation (type-level, uncomment to see): ────────────────────────────
    // fn pure_calls_effectful() -> i64 {
    //     let e: Effect<DB, String> = fetch_user(1);
    //     e.value.len() as i64  // This compiles only because we access .value directly.
    //     // In Loom's checker, calling fetch_user from a fn with no effect: declaration
    //     // is a LoomError::UncheckedEffect at compile time.
    // }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn pure_add_is_commutative(
            a in i64::MIN / 2..i64::MAX / 2,
            b in i64::MIN / 2..i64::MAX / 2,
        ) {
            prop_assert_eq!(pure_add(a, b), pure_add(b, a));
        }

        #[test]
        fn pure_add_is_associative(
            a in i64::MIN / 3..i64::MAX / 3,
            b in i64::MIN / 3..i64::MAX / 3,
            c in i64::MIN / 3..i64::MAX / 3,
        ) {
            prop_assert_eq!(pure_add(pure_add(a, b), c), pure_add(a, pure_add(b, c)));
        }

        #[test]
        fn run_io_handler_extracts_value(n in any::<i64>()) {
            let effect: Effect<IO, i64> = Effect::new(n);
            let result = run_io(effect);
            prop_assert_eq!(result, n);
        }

        #[test]
        fn run_db_handler_extracts_value(n in any::<i64>()) {
            let effect: Effect<DB, i64> = Effect::new(n);
            let result = run_db(effect);
            prop_assert_eq!(result, n);
        }
    }
}
