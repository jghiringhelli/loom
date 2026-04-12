// proof.rs — emitted by: loom compile proof.loom
// Theory: Model Checking (Clarke, Emerson & Sifakis 1981)
// Kani explores ALL possible inputs (within bounds) and verifies invariants hold.
// Unlike tests (which check specific inputs), model checking is exhaustive.

pub fn process_state(state: i64) -> &'static str {
    debug_assert!(state >= 0, "require: state >= 0");
    debug_assert!(state <= 3, "require: state <= 3");
    let result = match state {
        0 => "Idle",
        1 => "Processing",
        2 => "Complete",
        3 => "Error",
        _ => "Invalid",
    };
    debug_assert!(!result.is_empty(), "ensure: non-empty result");
    result
}

pub fn safe_increment(counter: i64) -> i64 {
    debug_assert!(counter >= 0 && counter <= 255);
    let result = if counter == 255 { 0 } else { counter + 1 };
    debug_assert!(result >= 0 && result <= 255);
    result
}

pub fn check_mutex(request: bool, other_in_cs: bool) -> bool {
    let result = if other_in_cs { false } else { request };
    debug_assert!(!(result && other_in_cs), "ensure: mutual exclusion");
    result
}

// ── Kani model checking harnesses — verify ALL possible inputs ────────────────

#[cfg(kani)]
mod model_checks {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)]
    fn model_check_process_state_all_valid_inputs() {
        let state: i64 = kani::any();
        kani::assume(state >= 0 && state <= 3);
        let result = process_state(state);
        kani::assert(!result.is_empty(), "all valid states produce non-empty output");
    }

    #[kani::proof]
    #[kani::unwind(10)]
    fn model_check_safe_increment_all_counters() {
        let counter: i64 = kani::any();
        kani::assume(counter >= 0 && counter <= 255);
        let result = safe_increment(counter);
        kani::assert(result >= 0, "increment always >= 0");
        kani::assert(result <= 255, "increment always <= 255");
        // Specific check: wraparound works correctly
        if counter == 255 {
            kani::assert(result == 0, "wraparound at 255");
        } else {
            kani::assert(result == counter + 1, "normal increment");
        }
    }

    #[kani::proof]
    fn model_check_mutex_all_states() {
        let request: bool = kani::any();
        let other_in_cs: bool = kani::any();
        let result = check_mutex(request, other_in_cs);
        // Key invariant: mutual exclusion — NEVER both in critical section
        kani::assert(!(result && other_in_cs), "mutual exclusion invariant");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_all_valid_states() {
        for state in 0..=3 {
            assert!(!process_state(state).is_empty());
        }
    }

    #[test]
    fn counter_wraparound() {
        assert_eq!(safe_increment(255), 0);
        assert_eq!(safe_increment(0), 1);
        assert_eq!(safe_increment(127), 128);
    }

    #[test]
    fn mutex_never_grants_when_occupied() {
        assert!(!check_mutex(true, true), "mutex must not grant when other is in CS");
        assert!(!check_mutex(false, true));
        assert!(check_mutex(true, false), "mutex must grant when CS is free");
    }
}
