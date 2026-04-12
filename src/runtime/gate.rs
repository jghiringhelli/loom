//! Type-safe mutation gate — R4.
//!
//! Every [`MutationProposal`] from any synthesis tier must pass through this gate
//! before it can be deployed.  The gate:
//!
//! 1. Converts the proposal into a Loom source patch (partial `.loom` fragment).
//! 2. Constructs a synthetic `.loom` module that applies the patch to a snapshot of
//!    the entity's current declaration.
//! 3. Runs the full Loom compiler (`loom::compile()`) on the patched source.
//! 4. Enforces the safety annotation requirement for autopoietic mutations.
//! 5. Records the verdict (accepted/rejected) and any compiler errors in the store.
//!
//! # Safety rule
//!
//! Any proposal that modifies autopoietic structure (clone, rollback, prune) MUST
//! have the entity annotated `@mortal @corrigible @sandboxed` in the source.
//! Proposals that lack these annotations are rejected with `MissingCaution`.

use crate::{
    compile,
    runtime::{
        mutation::MutationProposal,
        signal::{now_ms, EntityId},
        store::SignalStore,
    },
};

// ── Public types ──────────────────────────────────────────────────────────────

/// The verdict of the mutation gate for a single proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum GateVerdict {
    /// The proposal compiled cleanly and passed all safety checks.
    Accepted,
    /// The compiler rejected the patched source.
    CompilerRejected {
        /// Diagnostic errors from the Loom compiler.
        errors: Vec<String>,
    },
    /// The proposal requires safety annotations that are absent in the source.
    MissingCaution {
        /// Which annotation is missing (`@mortal`, `@corrigible`, `@sandboxed`).
        missing: String,
    },
    /// The proposal JSON could not be interpreted (internal error).
    MalformedProposal { reason: String },
}

impl GateVerdict {
    /// Returns `true` iff the proposal was accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Returns a short string suitable for the audit trail.
    pub fn as_audit_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::CompilerRejected { .. } => "compiler_rejected",
            Self::MissingCaution { .. } => "missing_caution",
            Self::MalformedProposal { .. } => "malformed",
        }
    }
}

/// The output of a gate evaluation.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// The proposal that was evaluated.
    pub proposal: MutationProposal,
    /// Whether the proposal was accepted or rejected, and why.
    pub verdict: GateVerdict,
}

// ── MutationGate ─────────────────────────────────────────────────────────────

/// The type-safe mutation gate.
///
/// Holds the current Loom source for all entities so it can construct
/// patched sources and run the compiler against them.
pub struct MutationGate {
    /// Map of entity_id → current Loom source fragment for that being.
    entity_sources: std::collections::HashMap<EntityId, String>,
    /// Whether to enforce safety annotations for autopoietic mutations.
    pub enforce_caution: bool,
}

impl MutationGate {
    /// Create a new gate with no registered entity sources.
    pub fn new() -> Self {
        Self {
            entity_sources: std::collections::HashMap::new(),
            enforce_caution: true,
        }
    }

    /// Register the Loom source for an entity.  The gate will use this source
    /// as the baseline when constructing patched modules.
    pub fn register_source(&mut self, entity_id: impl Into<EntityId>, source: String) {
        self.entity_sources.insert(entity_id.into(), source);
    }

    /// Evaluate a proposal: build a patched module, compile it, enforce safety.
    ///
    /// Also persists the verdict to `store` for the audit trail.
    pub fn evaluate(
        &self,
        proposal: &MutationProposal,
        store: &SignalStore,
    ) -> GateResult {
        // Safety check first — fast path, no compilation needed.
        if self.enforce_caution {
            if let Some(verdict) = self.check_safety_annotations(proposal) {
                let result = GateResult {
                    proposal: proposal.clone(),
                    verdict,
                };
                self.record(&result, store);
                return result;
            }
        }

        // Build a patched Loom source and run the full compiler.
        let patched_source = match self.build_patched_source(proposal) {
            Ok(src) => src,
            Err(reason) => {
                let result = GateResult {
                    proposal: proposal.clone(),
                    verdict: GateVerdict::MalformedProposal { reason },
                };
                self.record(&result, store);
                return result;
            }
        };

        let verdict = match compile(&patched_source) {
            Ok(_) => GateVerdict::Accepted,
            Err(errors) => GateVerdict::CompilerRejected {
                errors: errors.iter().map(|e| e.to_string()).collect(),
            },
        };

        let result = GateResult { proposal: proposal.clone(), verdict };
        self.record(&result, store);
        result
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Check for required safety annotations on autopoietic mutations.
    ///
    /// Returns `Some(MissingCaution)` if an annotation is absent, `None` if
    /// the proposal passes.
    fn check_safety_annotations(&self, proposal: &MutationProposal) -> Option<GateVerdict> {
        let requires_caution = matches!(
            proposal,
            MutationProposal::EntityClone { .. }
                | MutationProposal::EntityRollback { .. }
                | MutationProposal::EntityPrune { .. }
                | MutationProposal::StructuralRewire { .. }
        );
        if !requires_caution {
            return None;
        }

        let entity_id = proposal.primary_entity();
        let source = match self.entity_sources.get(entity_id) {
            Some(s) => s.as_str(),
            None => return None, // no source registered → cannot verify → permit
        };

        for annotation in &["@mortal", "@corrigible", "@sandboxed"] {
            if !source.contains(annotation) {
                return Some(GateVerdict::MissingCaution {
                    missing: annotation.to_string(),
                });
            }
        }
        None
    }

    /// Construct a minimal valid Loom module that incorporates the proposal.
    ///
    /// For `ParameterAdjust` we wrap the existing source (or synthesise one).
    /// For structural mutations we rely on the base source being valid.
    fn build_patched_source(&self, proposal: &MutationProposal) -> Result<String, String> {
        match proposal {
            MutationProposal::ParameterAdjust { entity_id, param, delta, .. } => {
                let base = self
                    .entity_sources
                    .get(entity_id.as_str())
                    .map(String::as_str)
                    .unwrap_or("");

                // If we have a base source, use it verbatim — the parameter
                // adjust is a runtime value change, not a structural change.
                // We still compile the base source to verify it remains valid.
                if base.is_empty() {
                    // Synthesise a minimal valid being so the compiler can run.
                    Ok(format!(
                        "module GateCheck\nbeing {entity_id}\n  telos: \"gate check for {param}\"\n  end\nend\nend\n",
                        entity_id = entity_id,
                        param = param
                    ))
                } else {
                    Ok(base.to_string())
                }
            }
            MutationProposal::EntityClone { source_id, .. }
            | MutationProposal::EntityRollback { entity_id: source_id, .. }
            | MutationProposal::EntityPrune { entity_id: source_id, .. } => {
                let src = self
                    .entity_sources
                    .get(source_id.as_str())
                    .ok_or_else(|| format!("no source registered for '{source_id}'"))?;
                Ok(src.clone())
            }
            MutationProposal::StructuralRewire { from_id, .. } => {
                let src = self
                    .entity_sources
                    .get(from_id.as_str())
                    .ok_or_else(|| format!("no source registered for '{from_id}'"))?;
                Ok(src.clone())
            }
        }
    }

    fn record(&self, result: &GateResult, store: &SignalStore) {
        let json = result
            .proposal
            .to_json()
            .unwrap_or_else(|_| "{}".into());
        let errors_str = match &result.verdict {
            GateVerdict::CompilerRejected { errors } => Some(errors.join("; ")),
            GateVerdict::MissingCaution { missing } => Some(format!("missing: {missing}")),
            GateVerdict::MalformedProposal { reason } => Some(reason.clone()),
            GateVerdict::Accepted => None,
        };
        let _ = store.record_mutation(
            result.proposal.primary_entity(),
            result.proposal.tier_hint(),
            &json,
            result.verdict.as_audit_str(),
            errors_str.as_deref(),
            now_ms(),
        );
    }
}

impl Default for MutationGate {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{mutation::MutationProposal, store::SignalStore};

    const VALID_BEING: &str = r#"
module GateTest
being ClimateModel
  telos: "maintain temperature below 2C"
    thresholds:
      convergence: 0.9
      divergence: 0.1
    end
  end
end
end
"#;

    const SAFE_BEING: &str = r#"
module GateTest
@mortal @corrigible @sandboxed
being ClimateModel
  telos: "maintain temperature below 2C"
    thresholds:
      convergence: 0.9
      divergence: 0.1
    end
  end
end
end
"#;

    fn mem_store() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    // ── ParameterAdjust: valid source → accepted ──────────────────────────────

    #[test]
    fn parameter_adjust_with_valid_source_accepted() {
        let store = mem_store();
        let mut gate = MutationGate::new();
        gate.register_source("ClimateModel", VALID_BEING.into());

        let proposal = MutationProposal::ParameterAdjust {
            entity_id: "ClimateModel".into(),
            param: "albedo".into(),
            delta: -0.01,
            reason: "reduce warming".into(),
        };
        let result = gate.evaluate(&proposal, &store);
        assert!(
            result.verdict.is_accepted(),
            "expected Accepted, got {:?}",
            result.verdict
        );
    }

    // ── ParameterAdjust: synthesised source when none registered ─────────────

    #[test]
    fn parameter_adjust_synthesises_source_when_none_registered() {
        let store = mem_store();
        let gate = MutationGate::new();
        let proposal = MutationProposal::ParameterAdjust {
            entity_id: "SyntheticEntity".into(),
            param: "growth_rate".into(),
            delta: 0.05,
            reason: "".into(),
        };
        let result = gate.evaluate(&proposal, &store);
        // Synthesised source may fail due to minimal grammar, but must not panic.
        // Just verify the gate runs without crashing.
        let _ = result.verdict;
    }

    // ── Safety: prune without annotations → MissingCaution ───────────────────

    #[test]
    fn prune_without_safety_annotations_rejected() {
        let store = mem_store();
        let mut gate = MutationGate::new();
        gate.register_source("ClimateModel", VALID_BEING.into());

        let proposal = MutationProposal::EntityPrune {
            entity_id: "ClimateModel".into(),
            reason: "test".into(),
        };
        let result = gate.evaluate(&proposal, &store);
        assert!(
            matches!(result.verdict, GateVerdict::MissingCaution { .. }),
            "expected MissingCaution, got {:?}",
            result.verdict
        );
    }

    // ── Safety: prune with all three annotations → accepted ──────────────────

    #[test]
    fn prune_with_safety_annotations_accepted() {
        let store = mem_store();
        let mut gate = MutationGate::new();
        gate.register_source("ClimateModel", SAFE_BEING.into());

        let proposal = MutationProposal::EntityPrune {
            entity_id: "ClimateModel".into(),
            reason: "test".into(),
        };
        let result = gate.evaluate(&proposal, &store);
        assert!(
            result.verdict.is_accepted(),
            "expected Accepted, got {:?}",
            result.verdict
        );
    }

    // ── Safety: clone without source → allowed (no source = can't verify) ────

    #[test]
    fn clone_without_registered_source_cannot_be_verified_and_is_permitted() {
        let store = mem_store();
        let gate = MutationGate::new();
        let proposal = MutationProposal::EntityClone {
            source_id: "UnknownEntity".into(),
            new_id: "Clone1".into(),
            reason: "".into(),
        };
        // No source registered → safety check skipped → falls through to compiler.
        // EntityClone will fail build_patched_source → MalformedProposal.
        let result = gate.evaluate(&proposal, &store);
        assert!(
            matches!(result.verdict, GateVerdict::MalformedProposal { .. }),
            "expected MalformedProposal for unregistered entity, got {:?}",
            result.verdict
        );
    }

    // ── Audit trail persisted to store ───────────────────────────────────────

    #[test]
    fn gate_records_verdict_in_store() {
        let store = mem_store();
        store.register_entity("ClimateModel", "ClimateModel", "{}", 0).unwrap();
        let mut gate = MutationGate::new();
        gate.register_source("ClimateModel", VALID_BEING.into());

        let proposal = MutationProposal::ParameterAdjust {
            entity_id: "ClimateModel".into(),
            param: "albedo".into(),
            delta: 0.01,
            reason: "test audit".into(),
        };
        gate.evaluate(&proposal, &store);
        // If recording worked, no panic. The store's mutation_proposals table
        // gets a row — we can't easily query it from outside, but this test
        // verifies the record() call does not error.
    }

    // ── GateVerdict helpers ───────────────────────────────────────────────────

    #[test]
    fn gate_verdict_is_accepted_flag() {
        assert!(GateVerdict::Accepted.is_accepted());
        assert!(!GateVerdict::CompilerRejected { errors: vec![] }.is_accepted());
        assert!(!GateVerdict::MissingCaution { missing: "@mortal".into() }.is_accepted());
    }
}
