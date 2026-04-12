//! Mutation proposal types — R3-1.
//!
//! Every mutation that any tier (Polycephalum, Ganglion, Mammal Brain) can propose
//! is expressed as a variant of this enum.  The type-safe mutation gate (R4) runs
//! the full Loom compiler against proposals before they are deployed.

use crate::runtime::signal::EntityId;
use serde::{Deserialize, Serialize};

/// A proposed change to the running system produced by any synthesis tier.
///
/// Proposals are serialised to JSON for the audit trail, LLM prompting, and the
/// mutation gate.  Each variant corresponds to one of the biological operations
/// defined in the Loom language (`epigenetic:`, `evolve:`, `crispr:`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MutationProposal {
    /// Adjust a numeric parameter on a running entity — fine-grained gradient step.
    ParameterAdjust {
        /// The entity whose parameter should change.
        entity_id: EntityId,
        /// The parameter name (must match a `regulate:` declaration in the being).
        param: String,
        /// Additive delta to apply to the current value.
        delta: f64,
        /// Human-readable rationale (used by LLM tiers; optional for Tier 1).
        reason: String,
    },

    /// Clone an entity, producing a new instance with the same telos.
    EntityClone {
        /// Source entity to clone.
        source_id: EntityId,
        /// Identifier for the new instance.
        new_id: EntityId,
        /// Rationale for the cloning (e.g. "increase redundancy for stability").
        reason: String,
    },

    /// Roll an entity back to a previously checkpointed state.
    EntityRollback {
        /// The entity to roll back.
        entity_id: EntityId,
        /// Checkpoint id from the signal store.
        checkpoint_id: i64,
        /// Rationale.
        reason: String,
    },

    /// Permanently remove a diverged entity from the ecosystem.
    EntityPrune {
        /// The entity to prune.
        entity_id: EntityId,
        /// Why this entity should be removed.
        reason: String,
    },

    /// Redirect a signal channel between two entities.
    StructuralRewire {
        /// The entity currently sending the signal.
        from_id: EntityId,
        /// The entity that should receive the signal instead.
        to_id: EntityId,
        /// The signal name (must match a `signal` declaration in the ecosystem).
        signal_name: String,
        /// Rationale.
        reason: String,
    },
}

impl MutationProposal {
    /// The entity id this proposal primarily concerns.
    pub fn primary_entity(&self) -> &str {
        match self {
            Self::ParameterAdjust { entity_id, .. } => entity_id,
            Self::EntityClone { source_id, .. } => source_id,
            Self::EntityRollback { entity_id, .. } => entity_id,
            Self::EntityPrune { entity_id, .. } => entity_id,
            Self::StructuralRewire { from_id, .. } => from_id,
        }
    }

    /// The synthesis tier that generated this proposal (for audit).
    /// Returns 0 when the tier is not embedded in the proposal itself.
    pub fn tier_hint(&self) -> u8 {
        // Tier 1 (Polycephalum) only ever generates ParameterAdjust.
        // Tier 2/3 may generate any variant.  Default to 0 (unknown).
        match self {
            Self::ParameterAdjust { .. } => 1,
            _ => 0,
        }
    }

    /// Serialise the proposal to a compact JSON string for the audit trail.
    ///
    /// # Errors
    /// Returns an error if serde_json serialisation fails (should be infallible
    /// for well-formed proposals).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialise a proposal from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_adjust_roundtrips_json() {
        let p = MutationProposal::ParameterAdjust {
            entity_id: "climate_1".into(),
            param: "albedo".into(),
            delta: -0.02,
            reason: "reduce warming trend".into(),
        };
        let json = p.to_json().unwrap();
        let back = MutationProposal::from_json(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn entity_prune_roundtrips_json() {
        let p = MutationProposal::EntityPrune {
            entity_id: "pandemic_2".into(),
            reason: "sustained divergence > 5 cycles".into(),
        };
        let json = p.to_json().unwrap();
        assert!(json.contains("entity_prune"));
        let back = MutationProposal::from_json(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn structural_rewire_roundtrips_json() {
        let p = MutationProposal::StructuralRewire {
            from_id: "a".into(),
            to_id: "b".into(),
            signal_name: "nutrient_flow".into(),
            reason: "optimise gradient".into(),
        };
        let json = p.to_json().unwrap();
        let back = MutationProposal::from_json(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn primary_entity_returns_correct_id() {
        let p = MutationProposal::EntityClone {
            source_id: "src".into(),
            new_id: "dst".into(),
            reason: "".into(),
        };
        assert_eq!(p.primary_entity(), "src");
    }

    #[test]
    fn tier_hint_is_one_for_parameter_adjust() {
        let p = MutationProposal::ParameterAdjust {
            entity_id: "x".into(),
            param: "k".into(),
            delta: 0.1,
            reason: "".into(),
        };
        assert_eq!(p.tier_hint(), 1);
    }
}
