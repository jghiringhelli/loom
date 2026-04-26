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

    /// A source-level code patch produced by the T5 Synthesis tier (Forge Ladder).
    ///
    /// Unlike parameter mutations, `CodePatch` changes the entity's *genome* — the
    /// actual Loom source code.  Before deployment the GS pipeline:
    /// 1. Applies `diff` via `git apply` to `target_file`
    /// 2. Runs `test_command` to verify compile + tests pass
    /// 3. Monitors the entity's telos signals for convergence
    /// 4. Promotes or reverts based on drift direction
    ///
    /// This is the biological analogue of CRISPR-mediated genome editing: targeted,
    /// guided by intent (telos), and validated before expression.
    CodePatch {
        /// The entity whose source code should be modified.
        entity_id: EntityId,
        /// Source file to patch, relative to the project root.
        target_file: String,
        /// Unified diff (output of `diff -u`) to apply.
        diff: String,
        /// Shell command used to verify the patch (e.g. `cargo test --lib -- runtime`).
        #[serde(default)]
        test_command: String,
        /// Structured prediction of the expected improvement, e.g.
        /// "drift_score for `carbon_stock` drops below 0.3 within 20 ticks".
        #[serde(default)]
        prediction: String,
        /// LLM-generated rationale for this code change.
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
            Self::CodePatch { entity_id, .. } => entity_id,
        }
    }

    /// The synthesis tier that generated this proposal (for audit).
    /// Returns 0 when the tier is not embedded in the proposal itself.
    pub fn tier_hint(&self) -> u8 {
        // Tier 1 (Polycephalum) only ever generates ParameterAdjust.
        // Tier 5 (Synthesis) generates CodePatch.
        // Tier 2/3 may generate any other variant.  Default to 0 (unknown).
        match self {
            Self::ParameterAdjust { .. } => 1,
            Self::CodePatch { .. } => 5,
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

    #[test]
    fn code_patch_roundtrips_json() {
        let p = MutationProposal::CodePatch {
            entity_id: "amr_coevolution".into(),
            target_file: "src/runtime/bioiso_runner.rs".into(),
            diff: "--- a/foo\n+++ b/foo\n@@ -1 +1 @@\n-old\n+new\n".into(),
            test_command: "cargo test --lib -- runtime".into(),
            prediction: "drift_score drops below 0.3 within 20 ticks".into(),
            reason: "T5: restructure resistance gene expression".into(),
        };
        let json = p.to_json().unwrap();
        assert!(json.contains("code_patch"));
        let back = MutationProposal::from_json(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn code_patch_tier_hint_is_five() {
        let p = MutationProposal::CodePatch {
            entity_id: "x".into(),
            target_file: "src/lib.rs".into(),
            diff: "".into(),
            test_command: "cargo test".into(),
            prediction: "".into(),
            reason: "".into(),
        };
        assert_eq!(p.tier_hint(), 5);
    }
}
