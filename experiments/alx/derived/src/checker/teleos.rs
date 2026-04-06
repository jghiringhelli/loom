// ALX: derived from loom.loom §"check_teleos"
// Every being: must have a telos:. Missing telos is a compile error.
// regulate: blocks must have bounds:.
// evolve: blocks must assert convergence (contain "decreasing", "non-increasing", "converg").

use crate::ast::Module;
use crate::error::LoomError;

pub fn check_teleos(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    for being in &module.being_defs {
        // Every being must have telos:
        if being.telos.is_none() {
            errors.push(LoomError::new(
                format!(
                    "being '{}': missing telos: — every being must have a final cause",
                    being.name
                ),
                being.span,
            ));
        }

        // autopoietic: true requires regulate:, evolve:, and matter:
        if being.autopoietic {
            if being.regulate_blocks.is_empty() {
                errors.push(LoomError::new(
                    format!(
                        "being '{}': autopoietic being requires at least one regulate: block",
                        being.name
                    ),
                    being.span,
                ));
            }
            if being.evolve_block.is_none() {
                errors.push(LoomError::new(
                    format!(
                        "being '{}': autopoietic being requires evolve: block",
                        being.name
                    ),
                    being.span,
                ));
            }
            if being.matter.is_none() {
                errors.push(LoomError::new(
                    format!(
                        "being '{}': autopoietic being requires matter: block",
                        being.name
                    ),
                    being.span,
                ));
            }
        }

        // regulate: blocks must have bounds:
        for reg in &being.regulate_blocks {
            if reg.bounds.is_none() {
                errors.push(LoomError::new(
                    format!(
                        "being '{}', regulate '{}': missing bounds: — homeostasis requires bounds",
                        being.name, reg.variable
                    ),
                    reg.span,
                ));
            }
        }

        // evolve: must assert convergence AND have at least one search: case
        if let Some(evolve) = &being.evolve_block {
            if evolve.search_cases.is_empty() {
                errors.push(LoomError::new(
                    format!(
                        "being '{}', evolve: must have at least one search: case; \
                         no search: cases declared",
                        being.name
                    ),
                    evolve.span,
                ));
            }
            let convergent = evolve.constraint.contains("decreasing")
                || evolve.constraint.contains("non-increasing")
                || evolve.constraint.contains("converg");
            if !convergent {
                errors.push(LoomError::new(
                    format!(
                        "being '{}', evolve: constraint '{}' must assert convergence \
                         (contain \"decreasing\", \"non-increasing\", or \"converg\")",
                        being.name, evolve.constraint
                    ),
                    evolve.span,
                ));
            }
        }

        // epigenetic: signal and modifies must be non-empty
        for epi in &being.epigenetic_blocks {
            if epi.signal.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', epigenetic: empty signal — must specify trigger signal", being.name),
                    epi.span,
                ));
            }
            if epi.modifies.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', epigenetic: empty modifies — must specify what is modulated", being.name),
                    epi.span,
                ));
            }
        }

        // morphogen: produces must be non-empty; threshold must be valid
        for morph in &being.morphogen_blocks {
            if morph.produces.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', morphogen: inert — produces: must contain at least one target", being.name),
                    morph.span,
                ));
            }
            // threshold out-of-range check: parse as f64, must be in (0.0, 1.0]
            if let Ok(t) = morph.threshold.parse::<f64>() {
                if t <= 0.0 || t > 1.0 {
                    errors.push(LoomError::new(
                        format!("being '{}', morphogen: threshold {} out of range (0.0, 1.0]", being.name, morph.threshold),
                        morph.span,
                    ));
                }
            }
        }

        // crispr: target, replace, guide must be non-empty
        for crispr in &being.crispr_blocks {
            if crispr.target.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', crispr: empty target — must specify what to modify", being.name),
                    crispr.span,
                ));
            }
            if crispr.replace.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', crispr: empty replace — must specify replacement", being.name),
                    crispr.span,
                ));
            }
            if crispr.guide.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', crispr: empty guide — must specify guide sequence", being.name),
                    crispr.span,
                ));
            }
        }

        // plasticity: trigger and modifies must be non-empty
        for plasticity in &being.plasticity_blocks {
            if plasticity.trigger.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', plasticity: empty trigger — must specify what triggers plasticity", being.name),
                    plasticity.span,
                ));
            }
            if plasticity.modifies.is_empty() {
                errors.push(LoomError::new(
                    format!("being '{}', plasticity: empty modifies — must specify what is modified", being.name),
                    plasticity.span,
                ));
            }
        }

        // telomere: must have limit > 0
        if let Some(tel) = &being.telomere {
            if tel.limit <= 0 {
                errors.push(LoomError::new(
                    format!(
                        "being '{}', telomere: limit must be positive, got {}",
                        being.name, tel.limit
                    ),
                    tel.span,
                ));
            }
        }
    }

    // Ecosystems must also have telos:, and quorum blocks must have non-empty signal
    for eco in &module.ecosystem_defs {
        if eco.telos.is_none() {
            errors.push(LoomError::new(
                format!(
                    "ecosystem '{}': missing telos: — every ecosystem must have a final cause",
                    eco.name
                ),
                eco.span,
            ));
        }
        for quorum in &eco.quorum_blocks {
            if quorum.signal.is_empty() {
                errors.push(LoomError::new(
                    format!("ecosystem '{}', quorum: empty signal — must specify autoinducer signal", eco.name),
                    quorum.span,
                ));
            }
            if quorum.action.is_empty() {
                errors.push(LoomError::new(
                    format!("ecosystem '{}', quorum: empty action — must specify collective action", eco.name),
                    quorum.span,
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
