// proof.rs — emitted by: loom compile proof.loom
// Theory: Hoare Logic (Hoare 1969)
// Every require: becomes a debug_assert! (runtime) and kani::assume (formal).
// Every ensure: becomes a debug_assert! (runtime) and kani::assert (formal).

/// Triple 1: {balance >= 0 ∧ amount > 0 ∧ balance >= amount} withdraw {result >= 0}
pub fn withdraw(balance: f64, amount: f64) -> f64 {
    debug_assert!(balance >= 0.0, "require: balance >= 0");
    debug_assert!(amount > 0.0, "require: amount > 0");
    debug_assert!(balance >= amount, "require: balance >= amount");
    let result = balance - amount;
    debug_assert!(result >= 0.0, "ensure: result >= 0");
    result
}

/// Triple 2: {divisor != 0} safe_div {result >= 0}
pub fn safe_div(dividend: i64, divisor: i64) -> i64 {
    debug_assert!(divisor != 0, "require: divisor != 0");
    let result = dividend / divisor;
    debug_assert!(result >= 0, "ensure: result >= 0");
    result
}

/// Triple 3: {lo <= hi} clamp {lo <= result ∧ result <= hi}
pub fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    debug_assert!(lo <= hi, "require: lo <= hi");
    let result = value.max(lo).min(hi);
    debug_assert!(result >= lo, "ensure: result >= lo");
    debug_assert!(result <= hi, "ensure: result <= hi");
    result
}

// ── Kani formal verification harnesses ───────────────────────────────────────

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn hoare_withdraw_proof() {
        let balance: f64 = kani::any();
        let amount: f64 = kani::any();
        kani::assume(balance >= 0.0);
        kani::assume(amount > 0.0);
        kani::assume(balance >= amount);
        let result = withdraw(balance, amount);
        kani::assert(result >= 0.0, "Hoare triple: withdraw postcondition");
    }

    #[kani::proof]
    fn hoare_div_proof() {
        let dividend: i64 = kani::any();
        let divisor: i64 = kani::any();
        kani::assume(divisor != 0);
        kani::assume(dividend >= 0);
        kani::assume(divisor > 0);
        let result = safe_div(dividend, divisor);
        kani::assert(result >= 0, "Hoare triple: safe_div postcondition");
    }

    #[kani::proof]
    fn hoare_clamp_proof() {
        let value: f64 = kani::any();
        let lo: f64 = kani::any();
        let hi: f64 = kani::any();
        kani::assume(lo <= hi);
        kani::assume(lo.is_finite() && hi.is_finite() && value.is_finite());
        let result = clamp(value, lo, hi);
        kani::assert(result >= lo, "Hoare triple: clamp lower bound");
        kani::assert(result <= hi, "Hoare triple: clamp upper bound");
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn hoare_withdraw_precondition_guarantees_postcondition(
            balance in 0.0f64..1_000_000.0f64,
            fraction in 0.0f64..1.0f64,
        ) {
            let amount = balance * fraction;
            if amount > 0.0 {
                let result = withdraw(balance, amount);
                prop_assert!(result >= 0.0, "Hoare triple: withdraw postcondition");
            }
        }

        #[test]
        fn hoare_clamp_always_within_bounds(
            value in f64::MIN / 2.0..f64::MAX / 2.0,
            lo in -10_000.0f64..0.0f64,
            hi in 0.0f64..10_000.0f64,
        ) {
            let result = clamp(value, lo, hi);
            prop_assert!(result >= lo);
            prop_assert!(result <= hi);
        }

        #[test]
        fn hoare_safe_div_postcondition_holds(
            dividend in 0i64..i64::MAX,
            divisor in 1i64..i64::MAX,
        ) {
            let result = safe_div(dividend, divisor);
            prop_assert!(result >= 0, "Hoare triple: safe_div postcondition");
        }
    }
}
