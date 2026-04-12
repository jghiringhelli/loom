#![allow(unused)]
use std::convert::TryFrom;
// == LOOM AUDIT: NaturalNumbers ==
// Functions  : 0
// Properties : 2 block(s) → edge-case #[test] + proptest (--cfg loom_proptest)
// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.

pub mod natural_numbers {
    use super::*;

    /// Property test: non_negative_square — forall n: Int
    /// invariant: n * n >= 0
    /// samples (edge cases): 1024, shrink: true
    /// V3: QuickCheck edge cases. V3+: proptest random sampling.
    #[test]
    fn property_non_negative_square_edge_cases() {
        let edge_cases: &[i64] = &[-1000, -1, 0, 1, 1000];
        for &n in edge_cases {
            assert!(n * n >= 0, "property 'non_negative_square' failed for n={}", n);
        }
    }

    // V3+: add `proptest` to [dev-dependencies] and `loom_proptest = []` to [features]
    #[cfg(feature = "loom_proptest")]
    mod property_non_negative_square_proptest {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(proptest::test_runner::Config::with_cases(1024))]
            #[test]
            fn property_non_negative_square_random(n in (-1_000_000_i64..=1_000_000_i64)) {
                prop_assert!(n * n >= 0, "property 'non_negative_square' failed for n={}", n);
            }
        }
    }

    /// Property test: additive_identity — forall n: Int
    /// invariant: n + 0 = n
    /// samples (edge cases): 1024, shrink: true
    /// V3: QuickCheck edge cases. V3+: proptest random sampling.
    #[test]
    fn property_additive_identity_edge_cases() {
        let edge_cases: &[i64] = &[-1000, -1, 0, 1, 1000];
        for &n in edge_cases {
            assert!(n + 0 == n, "property 'additive_identity' failed for n={}", n);
        }
    }

    // V3+: add `proptest` to [dev-dependencies] and `loom_proptest = []` to [features]
    #[cfg(feature = "loom_proptest")]
    mod property_additive_identity_proptest {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(proptest::test_runner::Config::with_cases(1024))]
            #[test]
            fn property_additive_identity_random(n in (-1_000_000_i64..=1_000_000_i64)) {
                prop_assert!(n + 0 == n, "property 'additive_identity' failed for n={}", n);
            }
        }
    }
}
