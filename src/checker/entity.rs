//! M120–M122: Entity annotation coherence checker — three-dimension pipeline.
//!
//! **Dimension 1 — StructuralChecker**: validates graph topology annotations.
//! **Dimension 2 — FormalConstraintChecker**: validates semantic annotation obligations.
//! **Dimension 3 — OrthogonalityChecker**: verifies dimensions 1+2 compose without contradiction.
//!
//! All three run in a single pass over each `entity<N, E>` declaration so that
//! a user receives all violations at once rather than one-per-run.

use crate::ast::{Item, Module};
use crate::checker::LoomChecker;
use crate::error::LoomError;

pub struct EntityChecker;

impl EntityChecker {
    /// Construct a new [`EntityChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `entity` declarations in the module.
    ///
    /// Runs all three dimension checkers sequentially and accumulates every
    /// error so callers get the full violation set in a single pass.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Entity(ent) = item {
                self.check_structural(ent, &mut errors);
                self.check_formal_constraints(ent, &mut errors);
                self.check_orthogonality(ent, &mut errors);
            }
        }
        errors
    }

    // ── Dimension 1: Structural ───────────────────────────────────────────────

    /// Validate structural annotation coherence (graph topology rules).
    fn check_structural(&self, ent: &crate::ast::EntityDef, errors: &mut Vec<LoomError>) {
        let ann = &ent.annotations;
        let has = |s: &str| ann.iter().any(|a| a == s);

        // @directed and @undirected are mutually exclusive
        if has("directed") && has("undirected") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @directed and @undirected are mutually exclusive — \
                     a graph cannot be both simultaneously",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @acyclic requires a directionality declaration
        if has("acyclic") && !has("directed") && !has("undirected") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @acyclic requires @directed or @undirected — \
                     acyclicity is defined relative to edge direction",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @layered requires @directed (layers imply a topological ordering)
        if has("layered") && !has("directed") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @layered requires @directed — \
                     layer ordering is only meaningful on directed graphs",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @hierarchical requires both @directed and @acyclic (trees are DAGs)
        if has("hierarchical") {
            if !has("directed") {
                errors.push(LoomError::type_err(
                    format!(
                        "entity '{}': @hierarchical requires @directed — \
                         hierarchy implies parent→child edge direction",
                        ent.name
                    ),
                    ent.span.clone(),
                ));
            }
            if !has("acyclic") {
                errors.push(LoomError::type_err(
                    format!(
                        "entity '{}': @hierarchical requires @acyclic — \
                         a cyclic hierarchy is a contradiction in terms",
                        ent.name
                    ),
                    ent.span.clone(),
                ));
            }
        }

        // @weighted requires an edge_type parameter (weights must be typed)
        if has("weighted") && ent.edge_type.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @weighted requires an edge type parameter — \
                     use entity<NodeType, WeightType> to specify weight representation",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }
    }

    // ── Dimension 2: Formal Constraint ────────────────────────────────────────

    /// Validate semantic annotation obligations (formal correctness requirements).
    fn check_formal_constraints(&self, ent: &crate::ast::EntityDef, errors: &mut Vec<LoomError>) {
        let ann = &ent.annotations;
        let has = |s: &str| ann.iter().any(|a| a == s);

        // @stochastic and @deterministic are mutually exclusive
        if has("stochastic") && has("deterministic") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @stochastic and @deterministic are mutually exclusive — \
                     a system cannot be both probabilistic and fully determined simultaneously",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @stochastic requires edge_type (probabilities need typed edges)
        if has("stochastic") && ent.edge_type.is_none() {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @stochastic requires an edge type parameter — \
                     probability distributions must have a concrete numeric type (e.g. Float)",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @learnable requires @weighted (gradient descent needs weight parameters)
        if has("learnable") && !has("weighted") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @learnable requires @weighted — \
                     learning algorithms update edge weights; weightless graphs have nothing to learn",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @causal requires @directed (causality flows in one direction)
        if has("causal") && !has("directed") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @causal requires @directed — \
                     causal relationships have a definite cause→effect direction",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @causal requires @acyclic (causal loops are logically paradoxical)
        if has("causal") && !has("acyclic") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @causal requires @acyclic — \
                     a causal cycle (A causes B causes A) is a logical paradox",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @temporal requires @directed (time flows in one direction)
        if has("temporal") && !has("directed") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @temporal requires @directed — \
                     temporal ordering imposes a strict direction on edges",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }
    }

    // ── Dimension 3: Orthogonality ────────────────────────────────────────────

    /// Validate that dimensions 1 and 2 compose without contradiction.
    ///
    /// Some annotation pairs are individually valid but produce contradictions
    /// when combined. This checker catches those cross-dimension violations.
    fn check_orthogonality(&self, ent: &crate::ast::EntityDef, errors: &mut Vec<LoomError>) {
        let ann = &ent.annotations;
        let has = |s: &str| ann.iter().any(|a| a == s);

        // @hierarchical + @undirected: tree hierarchy requires directed edges
        if has("hierarchical") && has("undirected") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @hierarchical and @undirected are orthogonally incompatible — \
                     a hierarchy defines a root and parent→child direction that undirected graphs lack",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @causal + @stochastic: fundamental contradiction across dimensions
        // (causality is deterministic at the structural level; stochasticity is
        //  about probability distributions, not causal structure)
        if has("causal") && has("stochastic") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @causal and @stochastic are orthogonally incompatible — \
                     causal graphs encode deterministic cause-effect structure; \
                     use @temporal @stochastic for Markov-like probabilistic processes instead",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @learnable + @deterministic: deterministic systems have no parameters to optimise
        if has("learnable") && has("deterministic") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @learnable and @deterministic are orthogonally incompatible — \
                     a fully deterministic graph has no free parameters to optimise",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }

        // @telos_guided + @deterministic: telos adaptation requires the system to change
        if has("telos_guided") && has("deterministic") {
            errors.push(LoomError::type_err(
                format!(
                    "entity '{}': @telos_guided and @deterministic are orthogonally incompatible — \
                     telos guidance adapts behaviour over time, which deterministic graphs forbid",
                    ent.name
                ),
                ent.span.clone(),
            ));
        }
    }
}

impl LoomChecker for EntityChecker {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        self.check(module)
    }
}

impl Default for EntityChecker {
    fn default() -> Self {
        Self::new()
    }
}
