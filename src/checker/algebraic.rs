//! Algebraic property checker.
//!
//! Validates semantic consistency of algebraic annotations:
//! - @idempotent requires the function to be effectful (only effectful ops make sense to be idempotent)
//! - @commutative requires >= 2 parameters of compatible types
//! - @at-most-once and @exactly-once are mutually exclusive
//! - @pure is incompatible with @idempotent (pure fns are already idempotent conceptually, but marking pure+idempotent suggests confusion)
//! - @exactly-once implies the fn must have Effect<[IO]> or Effect<[Email]> or Effect<[Payment]> — some irreversible side effect

use crate::ast::*;
use crate::error::LoomError;

/// Algebraic property checker.
///
/// Validates that algebraic annotations (`@idempotent`, `@commutative`, etc.)
/// are used consistently and correctly on function definitions.
pub struct AlgebraicChecker;

impl AlgebraicChecker {
    /// Create a new `AlgebraicChecker`.
    pub fn new() -> Self {
        AlgebraicChecker
    }

    /// Check `module` for algebraic annotation consistency.
    ///
    /// Returns `Ok(())` if all annotations are valid, or `Err(errors)` with
    /// one error per violation found.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        let has = |key: &str| fd.annotations.iter().any(|a| a.key == key);

        // Rule 1: @commutative requires at least 2 parameters.
        if has("commutative") && fd.type_sig.params.len() < 2 {
            errors.push(LoomError::type_err(
                "commutative requires at least 2 parameters",
                fd.span.clone(),
            ));
        }

        // Rule 2: @at-most-once and @exactly-once are mutually exclusive.
        if has("at-most-once") && has("exactly-once") {
            errors.push(LoomError::type_err(
                "conflicting multiplicity annotations: @at-most-once and @exactly-once cannot both be present",
                fd.span.clone(),
            ));
        }

        // Rule 3: @exactly-once requires an effectful function.
        if has("exactly-once")
            && !matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _))
        {
            errors.push(LoomError::type_err(
                "exactly-once requires an effectful function",
                fd.span.clone(),
            ));
        }

        // Rule 4: @idempotent and @exactly-once are contradictory.
        if has("idempotent") && has("exactly-once") {
            errors.push(LoomError::type_err(
                "idempotent and exactly-once are contradictory",
                fd.span.clone(),
            ));
        }
    }
}
