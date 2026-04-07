//! M112: CognitiveMemoryChecker — lightweight hippocampal layer validation.
//!
//! Validates that a being's declared `memory:` types are structurally consistent
//! with the blocks the being actually declares. This enforces the contract between
//! the cognitive memory declaration and the underlying episodic/procedural/etc. sources.
//!
//! Inspired by Chronicle (PragmaWorks/mcp/chronicle) but self-contained.
//! No runtime system — pure compile-time structural enforcement.
//!
//! Rules:
//! 1. `episodic` memory type requires a `journal:` block
//! 2. `procedural` memory type requires at least one `migration:` block
//! 3. `architectural` memory type requires a `manifest:` block
//! 4. `insight` memory type requires the being to be `autopoietic`
//!    (M111 clusters are only computed for autopoietic beings in evolution)
//! 5. `decay_rate` must be in [0.0, 1.0] if specified
//! 6. A being with no matching sources for any declared type gets a warning

use crate::ast::{CognitiveMemoryType, Module};
use crate::error::LoomError;

/// Validates cognitive memory declarations against structural being contracts.
pub struct CognitiveMemoryChecker;

impl CognitiveMemoryChecker {
    /// Create a new CognitiveMemoryChecker.
    pub fn new() -> Self {
        Self
    }

    /// Check all beings in the module for cognitive memory consistency.
    ///
    /// Returns errors (hard) and warnings (soft, prefixed `[warn]`).
    ///
    /// # Arguments
    /// * `module` — Parsed Loom module to check
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            let Some(ref mem) = being.cognitive_memory else { continue };

            // Rule 5: decay_rate range check.
            if let Some(dr) = mem.decay_rate {
                if !(0.0..=1.0).contains(&dr) {
                    errors.push(LoomError::type_err(
                        format!(
                            "being '{}': memory: decay_rate {} is out of range [0.0, 1.0]",
                            being.name, dr
                        ),
                        mem.span.clone(),
                    ));
                }
            }

            let mut unmatched = Vec::new();

            for memory_type in &mem.memory_types {
                match memory_type {
                    // Rule 1: episodic → journal: block required.
                    CognitiveMemoryType::Episodic => {
                        if being.journal.is_none() {
                            errors.push(LoomError::type_err(
                                format!(
                                    "being '{}': memory: type episodic requires a journal: block \
                                     (episodic memories need an event source — Tulving 1972)",
                                    being.name
                                ),
                                mem.span.clone(),
                            ));
                        }
                    }
                    // Rule 2: procedural → at least one migration: block required.
                    CognitiveMemoryType::Procedural => {
                        if being.migrations.is_empty() {
                            errors.push(LoomError::type_err(
                                format!(
                                    "being '{}': memory: type procedural requires at least one \
                                     migration: block (procedural memory encodes how to evolve)",
                                    being.name
                                ),
                                mem.span.clone(),
                            ));
                        }
                    }
                    // Rule 3: architectural → manifest: block required.
                    CognitiveMemoryType::Architectural => {
                        if being.manifest.is_none() {
                            errors.push(LoomError::type_err(
                                format!(
                                    "being '{}': memory: type architectural requires a manifest: \
                                     block (architectural memory records documentation liveness)",
                                    being.name
                                ),
                                mem.span.clone(),
                            ));
                        }
                    }
                    // Rule 4: insight → autopoietic required.
                    CognitiveMemoryType::Insight => {
                        if !being.autopoietic {
                            errors.push(LoomError::type_err(
                                format!(
                                    "being '{}': memory: type insight requires @autopoietic \
                                     (insight memories emerge from self-organizational evolution — \
                                     M111 clusters are only computed for autopoietic beings)",
                                    being.name
                                ),
                                mem.span.clone(),
                            ));
                        }
                    }
                    // Semantic memories are fed by regulate: violations — no hard requirement.
                    CognitiveMemoryType::Semantic => {
                        if being.regulate_blocks.is_empty() {
                            unmatched.push("semantic");
                        }
                    }
                }
            }

            // Rule 6: warn if semantic has no regulate: source.
            for kind in unmatched {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] being '{}': memory: type {} declared but no matching source block found \
                         (semantic memories benefit from regulate: blocks)",
                        being.name, kind
                    ),
                    mem.span.clone(),
                ));
            }

            // Warn if memory: block is empty.
            if mem.memory_types.is_empty() {
                errors.push(LoomError::type_err(
                    format!(
                        "[warn] being '{}': memory: block declares no types — \
                         add at least one of: episodic semantic procedural architectural insight",
                        being.name
                    ),
                    mem.span.clone(),
                ));
            }
        }
        errors
    }
}
