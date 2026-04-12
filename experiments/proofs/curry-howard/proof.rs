// proof.rs — emitted by: loom compile proof.loom
// Theory: Curry-Howard Isomorphism (Curry 1934, Howard 1969)
// Types are propositions. Programs are proofs.
// The Rust type system verifies these "proofs" compile correctly.

/// Proof of "A implies A" — the identity function IS the proof.
pub fn identity_proof<A>(x: A) -> A { x }

/// Proof of "(A, B) implies A" — fst is the proof.
pub fn fst_proof<A, B>(pair: (A, B)) -> A { pair.0 }

/// Proof of "A implies (A or B)" — Left injection.
pub enum Either<A, B> { Left(A), Right(B) }

pub fn left_proof<A, B>(a: A) -> Either<A, B> { Either::Left(a) }

/// Proof of transitivity: (A→B) ∧ (B→C) → (A→C)
pub fn transitivity_proof<A, B, C, F, G>(f: F, g: G, a: A) -> C
where
    F: Fn(A) -> B,
    G: Fn(B) -> C,
{
    g(f(a))
}

/// Proof of "A and B implies B and A" (commutativity of conjunction)
pub fn swap_proof<A, B>(pair: (A, B)) -> (B, A) { (pair.1, pair.0) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_proof_of_a_implies_a() {
        assert_eq!(identity_proof(42_i64), 42);
        assert_eq!(identity_proof("hello"), "hello");
    }

    #[test]
    fn fst_is_proof_of_conjunction_implies_left() {
        assert_eq!(fst_proof((42_i64, "ignored")), 42);
    }

    #[test]
    fn left_is_proof_of_disjunction_introduction() {
        let result: Either<i64, &str> = left_proof(42);
        match result {
            Either::Left(n) => assert_eq!(n, 42),
            Either::Right(_) => panic!("should be Left"),
        }
    }

    #[test]
    fn transitivity_is_proof_of_implication_chain() {
        let add_one = |x: i64| x + 1;
        let double = |x: i64| x * 2;
        // (A→B) and (B→C) gives us (A→C): add_one then double
        assert_eq!(transitivity_proof(add_one, double, 3_i64), 8);
    }

    #[test]
    fn swap_is_proof_of_commutativity() {
        let (b, a) = swap_proof((1_i64, "hello"));
        assert_eq!(b, "hello");
        assert_eq!(a, 1);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Curry-Howard: identity_proof IS the proof of A→A
        /// Property: identity preserves value for all inputs (the proof is total)
        #[test]
        fn identity_proof_is_total(x in any::<i64>()) {
            prop_assert_eq!(identity_proof(x), x,
                "Curry-Howard: identity function as proof of A→A must be total");
        }

        #[test]
        fn fst_proof_is_total(a in any::<i64>(), b in any::<i64>()) {
            prop_assert_eq!(fst_proof((a, b)), a,
                "Curry-Howard: fst as proof of (A,B)→A must be total");
        }

        #[test]
        fn swap_proof_is_involutive(a in any::<i64>(), b in any::<i64>()) {
            let (b2, a2) = swap_proof((a, b));
            prop_assert_eq!((a2, b2), (a, b),
                "Curry-Howard: swap proof of commutativity must be involutive");
        }

        #[test]
        fn transitivity_proof_chains_correctly(x in 0i64..i64::MAX / 4) {
            let add_one = |n: i64| n.saturating_add(1);
            let double = |n: i64| n.saturating_mul(2);
            let result = transitivity_proof(add_one, double, x);
            prop_assert_eq!(result, x.saturating_add(1).saturating_mul(2));
        }
    }
}
