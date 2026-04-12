#![allow(unused)]
use std::convert::TryFrom;
// == LOOM AUDIT: Reflexivity ==
// Functions  : 0
// Properties : 1 block(s) → edge-case #[test] + proptest (--cfg loom_proptest)
// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.

pub mod reflexivity {
    use super::*;

    /// Property test: int_reflexive — forall n: Int
    /// invariant: n = n
    /// samples (edge cases): 100, shrink: true
    /// V3: QuickCheck edge cases. V3+: proptest random sampling.
    #[test]
    fn property_int_reflexive_edge_cases() {
        let edge_cases: &[i64] = &[-1000, -1, 0, 1, 1000];
        for &n in edge_cases {
            assert!(n == n, "property 'int_reflexive' failed for n={}", n);
        }
    }

    // V3+: add `proptest` to [dev-dependencies] and `loom_proptest = []` to [features]
    #[cfg(all(test, feature = "loom_proptest"))]
    mod property_int_reflexive_proptest {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(proptest::test_runner::Config::with_cases(1024))]
            #[test]
            fn property_int_reflexive_random(n in (-1_000_000_i64..=1_000_000_i64)) {
                prop_assert!(n == n, "property 'int_reflexive' failed for n={}", n);
            }
        }
    }
}
