// proof.rs — emitted by: loom compile proof.loom
// Theory: Hindley-Milner Type Inference (Milner & Damas 1978)
// Rust's type inference (itself Hindley-Milner) verifies that Loom's emitted
// generic functions have the correct principal types.

/// Polymorphic identity: type parameter A is inferred at call site.
pub fn identity<A>(x: A) -> A { x }

/// Compose: (B -> C) -> (A -> B) -> A -> C
pub fn compose<A, B, C, F, G>(f: F, g: G, x: A) -> C
where
    F: Fn(B) -> C,
    G: Fn(A) -> B,
{
    f(g(x))
}

/// Map: (A -> B) -> Vec<A> -> Vec<B>
pub fn map_list<A, B, F: Fn(A) -> B>(f: F, xs: Vec<A>) -> Vec<B> {
    xs.into_iter().map(f).collect()
}

/// Fold: (B -> A -> B) -> B -> Vec<A> -> B
pub fn fold_list<A, B, F: Fn(B, A) -> B>(f: F, acc: B, xs: Vec<A>) -> B {
    xs.into_iter().fold(acc, f)
}

/// Inferred concrete instantiation: [f64] -> f64
pub fn sum_list(xs: Vec<f64>) -> f64 {
    fold_list(|acc, x| acc + x, 0.0, xs)
}

/// Inferred concrete instantiation: Vec<i64> -> Vec<String>
pub fn int_to_string_list(xs: Vec<i64>) -> Vec<String> {
    map_list(|n| n.to_string(), xs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_polymorphic() {
        assert_eq!(identity(42_i64), 42);
        assert_eq!(identity("hello"), "hello");
        assert_eq!(identity(3.14_f64), 3.14);
    }

    #[test]
    fn compose_chains_functions() {
        let add_one = |x: i64| x + 1;
        let double = |x: i64| x * 2;
        // compose(double, add_one, 3) = double(add_one(3)) = double(4) = 8
        assert_eq!(compose(double, add_one, 3_i64), 8);
    }

    #[test]
    fn map_preserves_length_and_transforms() {
        let result = map_list(|x: i64| x * 2, vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn fold_accumulates_correctly() {
        let result = fold_list(|acc: i64, x: i64| acc + x, 0, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, 15);
    }

    #[test]
    fn sum_list_infers_f64() {
        let result = sum_list(vec![1.0, 2.0, 3.0]);
        assert!((result - 6.0).abs() < 1e-10);
    }

    #[test]
    fn int_to_string_list_infers_correctly() {
        let result = int_to_string_list(vec![1, 2, 3]);
        assert_eq!(result, vec!["1", "2", "3"]);
    }

    #[test]
    fn principal_type_is_most_general() {
        // identity works for ANY type — this is the principal type guarantee
        let _i: i64 = identity(42);
        let _s: &str = identity("test");
        let _f: f64 = identity(1.0);
        let _v: Vec<i64> = identity(vec![1, 2, 3]);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn identity_preserves_value(x in any::<i64>()) {
            prop_assert_eq!(identity(x), x);
        }

        #[test]
        fn compose_preserves_semantics(x in any::<i64>()) {
            let add_one = |n: i64| n.saturating_add(1);
            let double = |n: i64| n.saturating_mul(2);
            let result = compose(double, add_one, x);
            prop_assert_eq!(result, x.saturating_add(1).saturating_mul(2));
        }

        #[test]
        fn sum_list_equals_manual_sum(
            xs in proptest::collection::vec(
                any::<f64>().prop_filter("finite", |f| f.is_finite()),
                0..100
            )
        ) {
            let manual: f64 = xs.iter().sum();
            let result = sum_list(xs);
            prop_assert!((result - manual).abs() < 1e-6);
        }
    }
}
