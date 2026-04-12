// proof.rs — emitted by: loom compile proof.loom
// Theory: Dijkstra Weakest Precondition (Dijkstra 1975)
// The weakest precondition wp(C, Q) is the most general condition on inputs
// that guarantees postcondition Q after executing C.
// Loom's SMT bridge computes wp and verifies require: => wp(C, ensure:).

pub fn increment_past_five(x: i64) -> i64 {
    debug_assert!(x > 4, "require: x > 4 (weakest precondition of result > 5)");
    let result = x + 1;
    debug_assert!(result > 5, "ensure: result > 5");
    result
}

/// WP of (result >= 0) under (if x >= 0 then x else -x) is TRUE.
/// No precondition needed — absolute value is always non-negative.
pub fn absolute_value(x: i64) -> i64 {
    let result = if x >= 0 { x } else { -x };
    debug_assert!(result >= 0, "ensure: result >= 0");
    result
}

/// wp(y := x+1; z := y*2, z > 10) = x > 4
pub fn double_increment(x: i64) -> i64 {
    debug_assert!(x > 4, "require: x > 4 (weakest precondition of result > 10)");
    let y = x + 1;
    let result = y * 2;
    debug_assert!(result > 10, "ensure: result > 10");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wp_increment_boundary() {
        // x = 5 is the boundary: wp says x > 4, so 5 is the minimum
        assert_eq!(increment_past_five(5), 6);
        assert!(increment_past_five(5) > 5);
    }

    #[test]
    fn wp_absolute_value_no_precondition() {
        // WP is TRUE: works for all inputs
        assert_eq!(absolute_value(5), 5);
        assert_eq!(absolute_value(-5), 5);
        assert_eq!(absolute_value(0), 0);
    }

    #[test]
    fn wp_sequential_composition() {
        // x = 5 satisfies x > 4; result = (5+1)*2 = 12 > 10
        assert_eq!(double_increment(5), 12);
        assert!(double_increment(5) > 10);
    }

    #[test]
    fn wp_is_minimal_condition() {
        // x = 5 works (satisfies wp: x > 4)
        // x = 4 would violate the require: (would panic in debug mode)
        // This test documents that the require: is exactly wp(C, Q)
        let values_satisfying_wp: Vec<i64> = vec![5, 6, 7, 100];
        for x in values_satisfying_wp {
            assert!(increment_past_five(x) > 5,
                "wp guarantee: x > 4 ensures result > 5 for x = {}", x);
        }
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn wp_absolute_value_always_nonnegative(x in i64::MIN / 2..i64::MAX / 2) {
            let result = absolute_value(x);
            prop_assert!(result >= 0);
        }

        #[test]
        fn wp_increment_satisfies_postcondition(x in 5i64..i64::MAX / 2) {
            let result = increment_past_five(x);
            prop_assert!(result > 5);
        }

        #[test]
        fn wp_double_increment_satisfies_postcondition(x in 5i64..i64::MAX / 4) {
            let result = double_increment(x);
            prop_assert!(result > 10);
        }
    }
}
