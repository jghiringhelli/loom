#![allow(unused)]
use std::convert::TryFrom;
// == LOOM AUDIT: AddPositiveContracts ==
// Functions  : 1
// Contracts  : 1 fn(s) → debug_assert!(runtime) + #[cfg(kani)] proof harness
// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.

pub mod add_positive_contracts {
    use super::*;

    pub fn add_positive(a: i64, b: i64) -> i64 {
        // LOOM[require]: (a > 0) — debug_assert! (runtime, debug builds only)
        debug_assert!((a > 0), "precondition violated: (a > 0)");
        // LOOM[require]: (b > 0) — debug_assert! (runtime, debug builds only)
        debug_assert!((b > 0), "precondition violated: (b > 0)");
        let _loom_result = (a + b);
        // LOOM[ensure]: (_loom_result > a) — checked on return value via _loom_result
        debug_assert!((_loom_result > a), "ensure: (_loom_result > a)");
        // LOOM[ensure]: (_loom_result > b) — checked on return value via _loom_result
        debug_assert!((_loom_result > b), "ensure: (_loom_result > b)");
        _loom_result
    }

    // LOOM[V2:Kani]: add_positive — SAT-bounded formal proof (Kani 2021)
    // Proves require:/ensure: hold for ALL inputs within solver bounds.
    // Install: cargo install --locked kani-verifier   Run: cargo kani
    #[cfg(kani)]
    #[kani::proof]
    fn kani_verify_add_positive() {
        let a: i64 = kani::any();
        let b: i64 = kani::any();
        // Preconditions — restrict symbolic input domain
        kani::assume((a > 0));
        kani::assume((b > 0));
        let result = add_positive(a, b);
        // Postconditions — Kani proves these for all valid inputs
        kani::assert!((result > a), "(result > a)");
        kani::assert!((result > b), "(result > b)");
    }

}
