#![allow(unused)]
use std::convert::TryFrom;
// == LOOM AUDIT: ContractExperiment ==
// Functions  : 2
// Contracts  : 2 fn(s) → debug_assert!(debug only) + #[cfg(kani)] proof harness
// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.

pub mod contract_experiment {
    use super::*;

    pub fn add_positives(a: i64, b: i64) -> i64 {
        // LOOM[require]: ((a > 0) && (b > 0)) — debug_assert! (runtime, debug builds only)
        debug_assert!(((a > 0) && (b > 0)), "precondition violated: ((a > 0) && (b > 0))");
        let _loom_result = (a + b);
        // LOOM[ensure]: (_loom_result > 0) — checked on return value via _loom_result (debug builds only)
        debug_assert!((_loom_result > 0), "ensure: (_loom_result > 0)");
        _loom_result
    }

    // LOOM[V2:Kani]: add_positives — SAT-bounded formal proof (Kani 2021)
    // Proves require:/ensure: hold for ALL inputs within solver bounds.
    // Install: cargo install --locked kani-verifier   Run: cargo kani
    #[cfg(kani)]
    #[kani::proof]
    fn kani_verify_add_positives() {
        let a: i64 = kani::any();
        let b: i64 = kani::any();
        // Preconditions — restrict symbolic input domain
        kani::assume(((a > 0) && (b > 0)));
        let result = add_positives(a, b);
        // Postconditions — Kani proves these for all valid inputs
        kani::assert!((result > 0), "(result > 0)");
    }


    pub fn bounded_divide(dividend: f64, divisor: f64) -> f64 {
        // LOOM[require]: (divisor != 0.0) — debug_assert! (runtime, debug builds only)
        debug_assert!((divisor != 0.0), "precondition violated: (divisor != 0.0)");
        let _loom_result = (dividend / divisor);
        // LOOM[ensure]: ((_loom_result > (0 - 1000000.0)) && (_loom_result < 1000000.0)) — checked on return value via _loom_result (debug builds only)
        debug_assert!(((_loom_result > (0 - 1000000.0)) && (_loom_result < 1000000.0)), "ensure: ((_loom_result > (0 - 1000000.0)) && (_loom_result < 1000000.0))");
        _loom_result
    }

    // LOOM[V2:Kani]: bounded_divide — SAT-bounded formal proof (Kani 2021)
    // Proves require:/ensure: hold for ALL inputs within solver bounds.
    // Install: cargo install --locked kani-verifier   Run: cargo kani
    #[cfg(kani)]
    #[kani::proof]
    fn kani_verify_bounded_divide() {
        let divisor: f64 = kani::any();
        let arg1: f64 = kani::any();
        // Preconditions — restrict symbolic input domain
        kani::assume((divisor != 0.0));
        let result = bounded_divide(divisor, arg1);
        // Postconditions — Kani proves these for all valid inputs
        kani::assert!(((result > (0 - 1000000.0)) && (result < 1000000.0)), "((result > (0 - 1000000.0)) && (result < 1000000.0))");
    }

}
