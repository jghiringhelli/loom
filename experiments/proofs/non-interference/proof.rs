// proof.rs — emitted by: loom compile proof.loom
// Theory: Non-interference / Information Flow (Goguen & Meseguer 1982)
// Sensitivity levels are phantom type parameters.
// Secret<T> cannot flow into Public<T> without an explicit declassify call.

use std::marker::PhantomData;

// ── Sensitivity lattice ───────────────────────────────────────────────────────

pub struct Public;
pub struct Secret;
pub struct TopSecret;

/// A value tagged with its sensitivity level at the type level.
pub struct Sensitive<Level, T> {
    value: T,
    _level: PhantomData<Level>,
}

impl<L, T> Sensitive<L, T> {
    fn new(value: T) -> Self {
        Self { value, _level: PhantomData }
    }
}

// ── Public functions: accept only Public-tagged inputs ────────────────────────

pub fn greet(name: Sensitive<Public, String>) -> Sensitive<Public, String> {
    Sensitive::new(format!("Hello, {}", name.value))
}

pub fn display(message: Sensitive<Public, String>) {
    println!("{}", message.value);
}

// ── Secret functions: operate on Secret-tagged values ────────────────────────

pub fn compute_bonus(salary: Sensitive<Secret, f64>) -> Sensitive<Secret, f64> {
    Sensitive::new(salary.value * 0.1)
}

// ── Declassify: explicit, audited downgrade from Secret to Public ─────────────

/// Only this function may convert Secret -> Public.
/// Every call site is an explicit, audited disclosure decision.
pub fn redact_salary(_salary: Sensitive<Secret, f64>) -> Sensitive<Public, String> {
    Sensitive::new("***REDACTED***".to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_data_flows_to_public_output() {
        let name = Sensitive::<Public, _>::new("Alice".to_string());
        let greeting = greet(name);
        assert_eq!(greeting.value, "Hello, Alice");
    }

    #[test]
    fn secret_stays_in_secret_context() {
        let salary = Sensitive::<Secret, _>::new(100_000.0_f64);
        let bonus = compute_bonus(salary);
        assert_eq!(bonus.value, 10_000.0);
        // bonus is still Secret<f64> — cannot call display(bonus) — type error
    }

    #[test]
    fn declassify_is_the_only_path_from_secret_to_public() {
        let salary = Sensitive::<Secret, _>::new(100_000.0_f64);
        let redacted = redact_salary(salary);
        // Now it's Public<String> — safe to display
        assert_eq!(redacted.value, "***REDACTED***");
        display(redacted);
    }

    // ── Violation (type error — uncomment to see): ───────────────────────────
    // #[test]
    // fn leak_secret_to_public_context() {
    //     let salary = Sensitive::<Secret, _>::new(100_000.0_f64);
    //     let bonus = compute_bonus(salary);
    //     display(bonus); // ERROR: expected Public, found Secret
    // }
}
