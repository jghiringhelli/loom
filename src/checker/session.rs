//! M98: Session type checker.
//!
//! Verifies protocol duality: send/recv correspondence between the two roles
//! declared in a session type. Honda (1993): session types for safe
//! communication protocols.
//!
//! Rules enforced:
//! 1. Every session must have exactly 2 roles.
//! 2. If `duality: A <-> B` is declared, each `send` in A must correspond to
//!    a `recv` in B at the same position, and vice versa.
//!    - Step-count mismatch → "duality violation: protocols have different lengths"
//!    - Type mismatch → "duality violation: role A sends X but role B expects Y"

use crate::ast::*;
use crate::error::LoomError;

/// Session type checker.
///
/// Validates that session types declared with a `duality:` clause satisfy the
/// Honda (1993) duality condition: send in one role corresponds to recv in the
/// other, and vice versa, with matching payload types.
pub struct SessionChecker;

impl SessionChecker {
    /// Create a new session checker.
    pub fn new() -> Self {
        SessionChecker
    }

    /// Check all session definitions in `module`.
    ///
    /// Returns accumulated errors — the pipeline receives all errors at once.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Session(sd) = item {
                self.check_session(sd, &mut errors);
            }
        }
        errors
    }

    fn check_session(&self, session: &SessionDef, errors: &mut Vec<LoomError>) {
        if session.roles.len() != 2 {
            errors.push(LoomError::parse(
                format!(
                    "session '{}': must have exactly 2 roles, found {}",
                    session.name,
                    session.roles.len()
                ),
                session.span.clone(),
            ));
            return;
        }

        let (duality_a, duality_b) = match &session.duality {
            Some(pair) => pair,
            None => return, // no duality declared, nothing to verify
        };

        // Find the two roles by the declared duality names.
        let role_a = session.roles.iter().find(|r| &r.name == duality_a);
        let role_b = session.roles.iter().find(|r| &r.name == duality_b);

        let (role_a, role_b) = match (role_a, role_b) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                errors.push(LoomError::parse(
                    format!(
                        "session '{}': duality references unknown role(s) '{}' or '{}'",
                        session.name, duality_a, duality_b
                    ),
                    session.span.clone(),
                ));
                return;
            }
        };

        if role_a.steps.len() != role_b.steps.len() {
            errors.push(LoomError::parse(
                format!(
                    "session '{}': duality violation: protocols have different lengths \
                     ({} steps in '{}' vs {} steps in '{}')",
                    session.name,
                    role_a.steps.len(),
                    role_a.name,
                    role_b.steps.len(),
                    role_b.name
                ),
                session.span.clone(),
            ));
            return;
        }

        for (i, (step_a, step_b)) in role_a.steps.iter().zip(role_b.steps.iter()).enumerate() {
            match (step_a, step_b) {
                (SessionStep::Send(ty_a), SessionStep::Recv(ty_b)) => {
                    if ty_a != ty_b {
                        errors.push(LoomError::parse(
                            format!(
                                "session '{}': duality violation at step {}: \
                                 role '{}' sends {:?} but role '{}' expects {:?}",
                                session.name,
                                i + 1,
                                role_a.name,
                                ty_a,
                                role_b.name,
                                ty_b
                            ),
                            session.span.clone(),
                        ));
                    }
                }
                (SessionStep::Recv(ty_a), SessionStep::Send(ty_b)) => {
                    if ty_a != ty_b {
                        errors.push(LoomError::parse(
                            format!(
                                "session '{}': duality violation at step {}: \
                                 role '{}' expects {:?} but role '{}' sends {:?}",
                                session.name,
                                i + 1,
                                role_a.name,
                                ty_a,
                                role_b.name,
                                ty_b
                            ),
                            session.span.clone(),
                        ));
                    }
                }
                _ => {
                    // Both send or both recv at the same position — duality violation.
                    errors.push(LoomError::parse(
                        format!(
                            "session '{}': duality violation at step {}: \
                             both roles have the same direction (both send or both recv)",
                            session.name,
                            i + 1
                        ),
                        session.span.clone(),
                    ));
                }
            }
        }
    }
}
