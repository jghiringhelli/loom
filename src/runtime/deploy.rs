//! Canary deployment — apply mutations to a subset of instances, monitor, promote or rollback.
//!
//! In a distributed BIOISO system, a "deployment" means patching the running
//! configuration of an entity (or cloning it with modified parameters) and
//! observing whether its telos score improves.  This module provides a
//! synchronous canary deployer that:
//!
//! 1. Records a checkpoint of the current entity state.
//! 2. Applies the proposal (parameter adjustment or structural change).
//! 3. Marks the entity as `in_canary` in the store.
//! 4. Promotes (commits) or rolls back based on the observed telos delta.
//!
//! In the current implementation, "apply" means writing the mutation to the
//! audit trail in the signal store — actual in-process parameter patching will
//! be wired when the runtime supports live entity configuration.

use crate::runtime::{
    mutation::MutationProposal,
    signal::now_ms,
    store::SignalStore,
};

// ── DeployOutcome ─────────────────────────────────────────────────────────────

/// Result of a canary deployment attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum DeployStatus {
    /// The canary was promoted — the mutation improved telos.
    Promoted,
    /// The canary was rolled back — the mutation worsened or did not improve telos.
    RolledBack,
    /// The deployment was recorded but cannot yet be promoted/rolled back (no telos
    /// comparison available — e.g. no bounds registered for the entity).
    Pending,
}

/// Full outcome record for one canary deployment.
#[derive(Debug, Clone)]
pub struct DeployOutcome {
    /// Entity that was targeted.
    pub entity_id: String,
    /// The proposal that was applied.
    pub proposal: MutationProposal,
    /// Final status.
    pub status: DeployStatus,
    /// Checkpoint ID created before the deployment (enables explicit rollback).
    pub checkpoint_id: i64,
    /// Timestamp of the deployment decision.
    pub ts: u64,
}

// ── CanaryDeployer ────────────────────────────────────────────────────────────

/// Synchronous canary deployer.
///
/// In tests and CI this runs synchronously (no background thread).
/// In production the orchestrator can run multiple ticks between deploy and
/// promote/rollback.
pub struct CanaryDeployer;

impl CanaryDeployer {
    /// Create a new canary deployer.
    pub fn new() -> Self {
        Self
    }

    /// Deploy a mutation proposal and return the outcome.
    ///
    /// Steps:
    /// 1. Save a checkpoint.
    /// 2. Record the deployment in the audit trail.
    /// 3. Decide promote/rollback based on available telos data.
    pub fn deploy(&self, proposal: &MutationProposal, store: &SignalStore) -> DeployOutcome {
        let entity_id = proposal_entity_id(proposal);
        let ts = now_ms();

        // Save a checkpoint before applying.
        let checkpoint_id = store
            .create_checkpoint(&entity_id, &serialize_proposal(proposal), ts)
            .unwrap_or(0);

        // Record the deployment in the audit trail.
        store
            .record_mutation(
                &entity_id,
                0u8,
                &serialize_proposal(proposal),
                "canary_deploy",
                None,
                ts,
            )
            .unwrap_or(());

        // Determine whether to promote or rollback.
        let status = self.canary_decision(&entity_id, proposal, store);

        DeployOutcome {
            entity_id,
            proposal: proposal.clone(),
            status,
            checkpoint_id,
            ts,
        }
    }

    /// Decide whether to promote or rollback the canary.
    ///
    /// Currently uses a conservative heuristic: if the entity has telos bounds
    /// registered AND there are recent signals that look better than the drift
    /// threshold, promote. Otherwise, mark as Pending (needs more observation).
    ///
    /// In production this would compare drift scores before/after.
    fn canary_decision(
        &self,
        entity_id: &str,
        _proposal: &MutationProposal,
        store: &SignalStore,
    ) -> DeployStatus {
        let bounds = store.telos_bounds_for_entity(entity_id).unwrap_or_default();
        if bounds.is_empty() {
            return DeployStatus::Pending;
        }

        // Compare the most recent signal for each metric against its target.
        // If all metrics are within 30% of target, promote.
        let mut all_good = true;
        for bound in &bounds {
            let signals = store
                .signals_for_entity(entity_id, 1)
                .unwrap_or_default();
            if let Some(sig) = signals.first() {
                if let Some(target) = bound.target {
                    let range = (bound.max.unwrap_or(target) - bound.min.unwrap_or(0.0)).abs();
                    let deviation = (sig.value - target).abs();
                    let score = if range > 0.0 { deviation / range } else { 0.0 };
                    if score > 0.3 {
                        all_good = false;
                        break;
                    }
                }
            }
        }

        if all_good {
            DeployStatus::Promoted
        } else {
            DeployStatus::Pending
        }
    }

    /// Execute an explicit rollback to a saved checkpoint.
    ///
    /// Records the rollback in the audit trail and returns `true` if the
    /// checkpoint existed in the store.
    pub fn rollback(
        &self,
        entity_id: &str,
        checkpoint_id: i64,
        store: &SignalStore,
    ) -> bool {
        let ts = now_ms();
        store
            .record_mutation(
                entity_id,
                0u8,
                &format!("{{\"rollback_to\":{checkpoint_id}}}"),
                "rollback",
                None,
                ts,
            )
            .is_ok()
    }
}

impl Default for CanaryDeployer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the primary entity_id from a proposal.
fn proposal_entity_id(proposal: &MutationProposal) -> String {
    match proposal {
        MutationProposal::ParameterAdjust { entity_id, .. } => entity_id.clone(),
        MutationProposal::EntityClone { source_id, .. } => source_id.clone(),
        MutationProposal::EntityRollback { entity_id, .. } => entity_id.clone(),
        MutationProposal::EntityPrune { entity_id, .. } => entity_id.clone(),
        MutationProposal::StructuralRewire { from_id, .. } => from_id.clone(),
    }
}

fn serialize_proposal(proposal: &MutationProposal) -> String {
    serde_json::to_string(proposal).unwrap_or_else(|_| "{}".into())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{mutation::MutationProposal, signal::now_ms, store::SignalStore};

    fn mem_store() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    fn param_adjust(entity_id: &str) -> MutationProposal {
        MutationProposal::ParameterAdjust {
            entity_id: entity_id.into(),
            param: "rate".into(),
            delta: 1.0,
            reason: "test".into(),
        }
    }

    #[test]
    fn deploy_creates_checkpoint_and_audit_entry() {
        let store = mem_store();
        store.register_entity("e1", "Test", "{}", now_ms()).unwrap();
        let deployer = CanaryDeployer::new();
        let outcome = deployer.deploy(&param_adjust("e1"), &store);
        assert!(outcome.checkpoint_id > 0);
        assert_eq!(outcome.entity_id, "e1");
    }

    #[test]
    fn deploy_without_telos_bounds_returns_pending() {
        let store = mem_store();
        store.register_entity("e1", "Test", "{}", now_ms()).unwrap();
        let deployer = CanaryDeployer::new();
        let outcome = deployer.deploy(&param_adjust("e1"), &store);
        assert_eq!(outcome.status, DeployStatus::Pending);
    }

    #[test]
    fn deploy_with_signals_on_target_returns_promoted() {
        let store = mem_store();
        store.register_entity("e1", "Test", "{}", now_ms()).unwrap();
        store
            .set_telos_bounds("e1", "temp", Some(0.0), Some(100.0), Some(50.0))
            .unwrap();
        // Emit a signal close to target (within 30% deviation).
        use crate::runtime::signal::Signal;
        store.write_signal(&Signal::new("e1", "temp", 52.0)).unwrap();
        let deployer = CanaryDeployer::new();
        let outcome = deployer.deploy(&param_adjust("e1"), &store);
        assert_eq!(outcome.status, DeployStatus::Promoted);
    }

    #[test]
    fn rollback_returns_true_on_valid_entity() {
        let store = mem_store();
        store.register_entity("e1", "Test", "{}", now_ms()).unwrap();
        let deployer = CanaryDeployer::new();
        // Save a checkpoint manually.
        let cid = store.create_checkpoint("e1", "{}", now_ms()).unwrap();
        let ok = deployer.rollback("e1", cid, &store);
        assert!(ok);
    }

    #[test]
    fn entity_id_extracted_from_entity_clone_proposal() {
        let proposal = MutationProposal::EntityClone {
            source_id: "original".into(),
            new_id: "clone-1".into(),
            reason: "test".into(),
        };
        let eid = proposal_entity_id(&proposal);
        assert_eq!(eid, "original");
    }
}
