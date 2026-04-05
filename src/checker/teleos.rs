//! Teleological checker — validates `being:` blocks.
//!
//! Rules:
//! - Every `being` must declare a `telos:` (final cause).
//! - Every `regulate` block must declare `bounds:`.
//! - Every `evolve` block must have a non-empty `constraint:`.
//! - Every `evolve` block must have at least one `search:` case.
//! - Every `evolve` constraint must assert convergence.
//! - `gradient_descent` and `derivative_free` are mutually exclusive without `when` conditions.

use crate::ast::{BeingDef, EcosystemDef, EvolveBlock, Module, SearchStrategy};
use crate::error::LoomError;

/// Run all teleological checks on a module.
pub fn check(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();
    for being in &module.being_defs {
        check_being(being, &mut errors);
    }
    check_ecosystems(module, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_being(being: &BeingDef, errors: &mut Vec<LoomError>) {
    if being.telos.is_none() {
        errors.push(LoomError::type_err(
            format!(
                "being '{}' has no telos: — every being must declare its final cause",
                being.name
            ),
            being.span.clone(),
        ));
    }
    for reg in &being.regulate_blocks {
        if reg.bounds.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "regulate '{}' in being '{}' has no bounds:",
                    reg.variable, being.name
                ),
                reg.span.clone(),
            ));
        }
    }
    if let Some(evolve) = &being.evolve_block {
        if evolve.constraint.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("evolve block in being '{}' has no constraint:", being.name),
                evolve.span.clone(),
            ));
        }
        errors.extend(validate_evolve(evolve, &being.name));
    }
}

/// Validate an `evolve` block for convergence semantics and strategy consistency.
pub fn validate_evolve(evolve: &EvolveBlock, being_name: &str) -> Vec<LoomError> {
    let mut errors = Vec::new();

    // Rule 1: at least one search: case required
    if evolve.search_cases.is_empty() {
        errors.push(LoomError::type_err(
            format!(
                "evolve block in being '{}' has no search: cases — at least one search strategy required",
                being_name
            ),
            evolve.span.clone(),
        ));
    }

    // Rule 2: constraint must assert convergence
    if !evolve.constraint.trim().is_empty() {
        let lower = evolve.constraint.to_lowercase();
        if !lower.contains("decreasing") && !lower.contains("non-increasing") && !lower.contains("converg") {
            errors.push(LoomError::type_err(
                format!(
                    "evolve constraint in being '{}' must assert convergence \
                     (e.g. 'E[distance_to_telos] decreasing')",
                    being_name
                ),
                evolve.span.clone(),
            ));
        }
    }

    // Rule 3: gradient_descent and derivative_free are mutually exclusive without when conditions
    let gd_no_when = evolve.search_cases.iter()
        .any(|sc| sc.strategy == SearchStrategy::GradientDescent && sc.when.trim().is_empty());
    let df_no_when = evolve.search_cases.iter()
        .any(|sc| sc.strategy == SearchStrategy::DerivativeFree && sc.when.trim().is_empty());
    if gd_no_when && df_no_when {
        errors.push(LoomError::type_err(
            format!(
                "evolve block in being '{}': gradient_descent and derivative_free are mutually \
                 exclusive without 'when' conditions — use 'when gradient_available' / \
                 'when state_space_unknown'",
                being_name
            ),
            evolve.span.clone(),
        ));
    }

    errors
}

/// Validate all ecosystem definitions in a module.
pub fn check_ecosystems(module: &Module, errors: &mut Vec<LoomError>) {
    let known_beings: std::collections::HashSet<&str> =
        module.being_defs.iter().map(|b| b.name.as_str()).collect();

    for eco in &module.ecosystem_defs {
        check_ecosystem(eco, &known_beings, errors);
    }
}

fn check_ecosystem(
    eco: &EcosystemDef,
    known_beings: &std::collections::HashSet<&str>,
    errors: &mut Vec<LoomError>,
) {
    // Rule 1: no signals → error
    if eco.signals.is_empty() {
        errors.push(LoomError::type_err(
            format!(
                "ecosystem '{}' has no signals — beings cannot interact without communication channels",
                eco.name
            ),
            eco.span.clone(),
        ));
    }

    // Build the set of beings visible to this ecosystem (module-level + member list)
    let mut visible: std::collections::HashSet<&str> = known_beings.clone();
    for m in &eco.members {
        visible.insert(m.as_str());
    }

    // Rule 2: every signal's from/to must reference a known being
    for sig in &eco.signals {
        if !visible.contains(sig.from.as_str()) {
            errors.push(LoomError::type_err(
                format!(
                    "signal '{}' in ecosystem '{}': 'from' being '{}' is not defined",
                    sig.name, eco.name, sig.from
                ),
                sig.span.clone(),
            ));
        }
        if !visible.contains(sig.to.as_str()) {
            errors.push(LoomError::type_err(
                format!(
                    "signal '{}' in ecosystem '{}': 'to' being '{}' is not defined",
                    sig.name, eco.name, sig.to
                ),
                sig.span.clone(),
            ));
        }
    }

    // Rule 3: telos, if present, must be non-empty
    if let Some(telos) = &eco.telos {
        if telos.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("ecosystem '{}' has an empty telos:", eco.name),
                eco.span.clone(),
            ));
        }
    }
}
