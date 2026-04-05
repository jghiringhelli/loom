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
    if being.autopoietic {
        check_autopoiesis(being, errors);
    }

    // Check epigenetic blocks
    for epi in &being.epigenetic_blocks {
        if epi.signal.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("epigenetic block in being '{}' has empty signal:", being.name),
                epi.span.clone(),
            ));
        }
        if epi.modifies.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("epigenetic block in being '{}' has empty modifies:", being.name),
                epi.span.clone(),
            ));
        }
    }

    // Check morphogen blocks
    for morph in &being.morphogen_blocks {
        if morph.signal.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("morphogen block in being '{}' has empty signal:", being.name),
                morph.span.clone(),
            ));
        }
        if morph.produces.is_empty() {
            errors.push(LoomError::type_err(
                format!("morphogen with no produces: is inert in being '{}'", being.name),
                morph.span.clone(),
            ));
        }
        if !morph.threshold.trim().is_empty() {
            match morph.threshold.parse::<f64>() {
                Ok(v) if v >= 0.0 && v <= 1.0 => {}
                Ok(_) => {
                    errors.push(LoomError::type_err(
                        format!(
                            "morphogen threshold {:?} in being '{}' is out of range [0.0, 1.0]",
                            morph.threshold, being.name
                        ),
                        morph.span.clone(),
                    ));
                }
                Err(_) => {
                    errors.push(LoomError::type_err(
                        format!(
                            "morphogen threshold {:?} in being '{}' must be a float between 0.0 and 1.0",
                            morph.threshold, being.name
                        ),
                        morph.span.clone(),
                    ));
                }
            }
        }
    }

    // Check telomere
    if let Some(tel) = &being.telomere {
        if tel.limit == 0 {
            errors.push(LoomError::type_err(
                format!("telomere limit must be positive in being '{}'", being.name),
                tel.span.clone(),
            ));
        }
        if tel.on_exhaustion.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("telomere on_exhaustion must be non-empty in being '{}'", being.name),
                tel.span.clone(),
            ));
        }
        const KNOWN_EXHAUSTION: &[&str] = &["senescence", "apoptosis", "quiescence"];
        if !KNOWN_EXHAUSTION.contains(&tel.on_exhaustion.as_str()) {
            let fn_names: Vec<&str> = being.function.as_ref()
                .map(|fb| fb.fns.iter().map(|f| f.name.as_str()).collect())
                .unwrap_or_default();
            if !fn_names.contains(&tel.on_exhaustion.as_str()) {
                errors.push(LoomError::type_err(
                    format!(
                        "telomere on_exhaustion {:?} in being '{}' is not a known keyword (senescence/apoptosis/quiescence) and not a declared function",
                        tel.on_exhaustion, being.name
                    ),
                    tel.span.clone(),
                ));
            }
        }
    }

    // CRISPR checks
    for crispr in &being.crispr_blocks {
        if crispr.target.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("crispr block in being '{}' has empty target:", being.name),
                crispr.span.clone(),
            ));
        }
        if crispr.replace.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("crispr block in being '{}' has empty replace:", being.name),
                crispr.span.clone(),
            ));
        }
        if crispr.guide.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("crispr block in being '{}' has empty guide:", being.name),
                crispr.span.clone(),
            ));
        }
    }

    // Plasticity checks
    for plasticity in &being.plasticity_blocks {
        if plasticity.trigger.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("plasticity block in being '{}' has empty trigger:", being.name),
                plasticity.span.clone(),
            ));
        }
        if plasticity.modifies.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("plasticity block in being '{}' has empty modifies:", being.name),
                plasticity.span.clone(),
            ));
        }
    }
}

/// Verify that an autopoietic being satisfies all four organisational layers.
///
/// Maturana/Varela (1972): operational closure requires telos (purpose),
/// regulate (homeostasis), evolve (self-modification), and matter (boundary
/// substrate).
fn check_autopoiesis(being: &BeingDef, errors: &mut Vec<LoomError>) {
    let mut missing: Vec<&str> = Vec::new();
    if being.regulate_blocks.is_empty() {
        missing.push("regulate: (homeostasis)");
    }
    if being.evolve_block.is_none() {
        missing.push("evolve: (self-modification)");
    }
    if being.matter.is_none() {
        missing.push("matter: (boundary substrate)");
    }
    for req in missing {
        errors.push(LoomError::type_err(
            format!(
                "autopoietic being '{}' is missing {}: autopoietic systems require \
                 telos: (purpose), regulate: (homeostasis), evolve: (self-modification), and \
                 matter: (boundary substrate). Maturana/Varela (1972): operational closure \
                 requires all four organizational layers.",
                being.name, req
            ),
            being.span.clone(),
        ));
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

    // Quorum checks
    for quorum in &eco.quorum_blocks {
        if quorum.signal.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("quorum block in ecosystem '{}' has empty signal:", eco.name),
                quorum.span.clone(),
            ));
        }
        if quorum.action.trim().is_empty() {
            errors.push(LoomError::type_err(
                format!("quorum block in ecosystem '{}' has empty action:", eco.name),
                quorum.span.clone(),
            ));
        }
        match quorum.threshold.parse::<f64>() {
            Ok(f) if f > 0.0 && f <= 1.0 => {}
            _ => errors.push(LoomError::type_err(
                format!(
                    "quorum block in ecosystem '{}': threshold '{}' must be a float in (0.0, 1.0]",
                    eco.name, quorum.threshold
                ),
                quorum.span.clone(),
            )),
        }
    }
}
