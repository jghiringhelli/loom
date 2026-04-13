//! Simulation — Stage 5 of the CEMS pipeline.
//!
//! The Simulation stage is the digital-twin layer. Before a mutation is promoted
//! to Soft Release (Stage 6), it runs in isolation against a synthetic signal
//! stream derived from the entity's own historical telemetry. This catches regressions
//! that the Gate (Stage 4) cannot detect: the Gate validates syntax and semantics;
//! the Simulation validates *behaviour*.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │  Meiotic Pool                                        │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
//! │  │ Proposal │  │ Proposal │  │ Proposal │  …        │
//! │  │    A     │  │    B     │  │    C     │           │
//! │  └────┬─────┘  └────┬─────┘  └────┬─────┘           │
//! │       │  isolation test           │                  │
//! │       ▼             ▼             ▼                  │
//! │   effect_A      effect_B      effect_C               │
//! │       └─────────────┴─────────────┘                  │
//! │                     │ SVD / eigendecomposition        │
//! │                     ▼                                 │
//! │         Independence matrix → recombination plan      │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! # Meiotic Pool
//!
//! Mutations are held in the pool until enough candidates accumulate for an
//! independence analysis. Each candidate is tested individually against the
//! digital twin and produces an *effect vector* — a per-telos-dimension delta.
//!
//! # SVD mutation independence
//!
//! The effect matrix `E[mutation × telos_dim]` is decomposed using SVD
//! (or principal-component-style cosine similarity for the pure-Rust implementation).
//! Two mutations are orthogonal when their effect vectors are perpendicular
//! (cosine similarity ≈ 0) and anti-parallel when similarity ≈ −1.
//!
//! - **Orthogonal** → combine in one offspring (their effects are independent)
//! - **Anti-parallel** → separate lineages (combining masks both effects)
//! - **Parallel** → redundant; keep only the stronger one
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md) §Meiotic-Pool.

use std::collections::HashMap;

use crate::runtime::signal::Timestamp;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Cosine similarity threshold for "orthogonal" classification.
pub const ORTHOGONAL_THRESHOLD: f64 = 0.3;

/// Cosine similarity threshold for "anti-parallel" classification (negative end).
pub const ANTI_PARALLEL_THRESHOLD: f64 = -0.3;

/// Default number of synthetic signal ticks in a digital twin run.
pub const DEFAULT_SIMULATION_TICKS: usize = 100;

// ── Effect vector ─────────────────────────────────────────────────────────────

/// The per-telos-dimension effect produced by running a mutation through the
/// digital twin.
///
/// Each element corresponds to one telos dimension (metric); positive = improvement
/// toward target, negative = regression away from target, zero = no effect.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectVector {
    /// Mutation proposal id this vector belongs to.
    pub proposal_id: String,
    /// Per-dimension delta (index matches `telos_dimensions`).
    pub deltas: Vec<f64>,
    /// Survival score: fraction of simulation ticks within telos bounds.
    pub survival_score: f64,
    /// Timestamp when this effect was computed.
    pub computed_at: Timestamp,
}

impl EffectVector {
    /// Cosine similarity with another effect vector.
    ///
    /// Returns `None` when either vector has zero magnitude.
    pub fn cosine_similarity(&self, other: &EffectVector) -> Option<f64> {
        if self.deltas.len() != other.deltas.len() {
            return None;
        }
        let dot: f64 = self.deltas.iter().zip(&other.deltas).map(|(a, b)| a * b).sum();
        let mag_a: f64 = self.deltas.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mag_b: f64 = other.deltas.iter().map(|x| x * x).sum::<f64>().sqrt();
        if mag_a < 1e-12 || mag_b < 1e-12 {
            return None;
        }
        Some(dot / (mag_a * mag_b))
    }

    /// L2 magnitude of the effect vector.
    pub fn magnitude(&self) -> f64 {
        self.deltas.iter().map(|x| x * x).sum::<f64>().sqrt()
    }
}

// ── Independence classification ───────────────────────────────────────────────

/// Relationship between two mutation proposals based on their effect vectors.
#[derive(Debug, Clone, PartialEq)]
pub enum MutationRelationship {
    /// Effects are approximately perpendicular — combine safely in one offspring.
    Orthogonal { cosine: f64 },
    /// Effects reinforce each other — redundant, keep the stronger one.
    Parallel { cosine: f64 },
    /// Effects oppose each other — combining masks both; separate lineages.
    AntiParallel { cosine: f64 },
    /// One or both vectors have zero magnitude — no information to compare.
    Indeterminate,
}

impl MutationRelationship {
    /// Classify the cosine similarity into a relationship category.
    pub fn from_cosine(cosine: f64) -> Self {
        if cosine > ORTHOGONAL_THRESHOLD {
            MutationRelationship::Parallel { cosine }
        } else if cosine < ANTI_PARALLEL_THRESHOLD {
            MutationRelationship::AntiParallel { cosine }
        } else {
            MutationRelationship::Orthogonal { cosine }
        }
    }
}

// ── Digital twin ──────────────────────────────────────────────────────────────

/// Configuration for a digital twin simulation run.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Telos bounds: (metric_name → (min, max, target)).
    pub telos: HashMap<String, (f64, f64, f64)>,
    /// Historical signal baseline: (metric_name → mean value).
    pub baseline: HashMap<String, f64>,
    /// Noise amplitude as a fraction of the baseline mean.
    pub noise_fraction: f64,
    /// Number of signal ticks to simulate.
    pub ticks: usize,
}

impl SimulationConfig {
    /// Create a config from telos bounds and historical baseline.
    pub fn new(
        telos: HashMap<String, (f64, f64, f64)>,
        baseline: HashMap<String, f64>,
    ) -> Self {
        Self { telos, baseline, noise_fraction: 0.05, ticks: DEFAULT_SIMULATION_TICKS }
    }
}

/// Result of one digital twin simulation.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Proposal id that was tested.
    pub proposal_id: String,
    /// Effect vector produced by the run.
    pub effect: EffectVector,
    /// Whether the mutation passed the simulation (survival_score >= threshold).
    pub passed: bool,
    /// Human-readable summary of the run.
    pub summary: String,
}

/// Digital twin — simulates mutation outcomes against synthetic signal streams.
pub struct DigitalTwin {
    /// Survival threshold: minimum fraction of ticks within bounds to pass.
    pub survival_threshold: f64,
}

impl DigitalTwin {
    /// Create a new digital twin with a default 80% survival threshold.
    pub fn new() -> Self {
        Self { survival_threshold: 0.80 }
    }

    /// Simulate a mutation proposal against the given config.
    ///
    /// In production this would apply the mutation's parameter deltas to the
    /// baseline and measure telos satisfaction. This implementation computes
    /// a deterministic effect based on the proposal's parameter adjustments
    /// relative to the baseline.
    ///
    /// `param_deltas` maps metric names to proposed changes (positive = increase,
    /// negative = decrease).
    pub fn simulate(
        &self,
        proposal_id: &str,
        config: &SimulationConfig,
        param_deltas: &HashMap<String, f64>,
        now: Timestamp,
    ) -> SimulationResult {
        let metrics: Vec<&String> = config.telos.keys().collect();
        let ticks_within = self.count_surviving_ticks(config, param_deltas);
        let survival_score = ticks_within as f64 / config.ticks as f64;

        // Compute effect vector: for each telos dimension, how much does the
        // proposed mutation move the baseline toward the target?
        let deltas: Vec<f64> = metrics
            .iter()
            .map(|metric| {
                let (min, max, target) = config.telos[*metric];
                let baseline = config.baseline.get(*metric).copied().unwrap_or(0.0);
                let delta = param_deltas.get(*metric).copied().unwrap_or(0.0);
                let proposed = baseline + delta;
                // Effect: distance from target reduced (positive) or increased (negative)
                let before = (baseline - target).abs() / (max - min + f64::EPSILON);
                let after = (proposed - target).abs() / (max - min + f64::EPSILON);
                before - after // positive = improvement
            })
            .collect();

        let passed = survival_score >= self.survival_threshold;
        let summary = format!(
            "proposal '{proposal_id}': survival={:.1}% {}",
            survival_score * 100.0,
            if passed { "PASS" } else { "FAIL" }
        );

        SimulationResult {
            proposal_id: proposal_id.to_string(),
            effect: EffectVector {
                proposal_id: proposal_id.to_string(),
                deltas,
                survival_score,
                computed_at: now,
            },
            passed,
            summary,
        }
    }

    /// Count ticks where the proposed parameters keep all metrics within telos bounds.
    fn count_surviving_ticks(
        &self,
        config: &SimulationConfig,
        param_deltas: &HashMap<String, f64>,
    ) -> usize {
        let mut within = 0;
        for _ in 0..config.ticks {
            let all_ok = config.telos.iter().all(|(metric, (min, max, _target))| {
                let baseline = config.baseline.get(metric).copied().unwrap_or(0.0);
                let delta = param_deltas.get(metric).copied().unwrap_or(0.0);
                let proposed = baseline + delta;
                proposed >= *min && proposed <= *max
            });
            if all_ok {
                within += 1;
            }
        }
        within
    }
}

impl Default for DigitalTwin {
    fn default() -> Self {
        Self::new()
    }
}

// ── Meiotic Pool ──────────────────────────────────────────────────────────────

/// A candidate mutation in the pool.
#[derive(Debug, Clone)]
pub struct PooledMutation {
    pub proposal_id: String,
    pub param_deltas: HashMap<String, f64>,
    pub effect: Option<EffectVector>,
    pub entered_at: Timestamp,
}

/// Recombination plan: which proposals to combine into which offspring.
#[derive(Debug, Clone)]
pub struct RecombinationPlan {
    /// Groups of proposal_ids to combine into single offspring.
    /// Each group becomes one mutation proposal to Stage 6.
    pub offspring_groups: Vec<Vec<String>>,
    /// Proposals to promote as separate lineages (anti-parallel pairs).
    pub separate_lineages: Vec<String>,
    /// Proposals that are redundant (parallel; dominated by stronger effect).
    pub redundant: Vec<String>,
}

/// Meiotic Pool — staging area for mutation independence analysis.
///
/// Accumulates mutations, runs each through the digital twin, then uses
/// SVD-inspired cosine analysis to determine which can be safely combined.
pub struct MeioticPool {
    candidates: HashMap<String, PooledMutation>,
    twin: DigitalTwin,
    /// Minimum pool size before recombination analysis runs.
    pub min_pool_size: usize,
}

impl MeioticPool {
    /// Create a new pool.
    pub fn new() -> Self {
        Self {
            candidates: HashMap::new(),
            twin: DigitalTwin::new(),
            min_pool_size: 2,
        }
    }

    /// Add a mutation candidate to the pool.
    pub fn add_candidate(
        &mut self,
        proposal_id: impl Into<String>,
        param_deltas: HashMap<String, f64>,
        now: Timestamp,
    ) {
        let id = proposal_id.into();
        self.candidates.insert(
            id.clone(),
            PooledMutation { proposal_id: id, param_deltas, effect: None, entered_at: now },
        );
    }

    /// Test all candidates against the digital twin.
    ///
    /// Populates `PooledMutation::effect` for each candidate.
    /// Returns simulation results for all candidates.
    pub fn run_isolation_tests(
        &mut self,
        config: &SimulationConfig,
        now: Timestamp,
    ) -> Vec<SimulationResult> {
        let ids: Vec<String> = self.candidates.keys().cloned().collect();
        let mut results = Vec::new();
        for id in ids {
            let deltas = self.candidates[&id].param_deltas.clone();
            let result = self.twin.simulate(&id, config, &deltas, now);
            if let Some(candidate) = self.candidates.get_mut(&id) {
                candidate.effect = Some(result.effect.clone());
            }
            results.push(result);
        }
        results
    }

    /// Analyse independence between all tested candidates.
    ///
    /// Returns a pairwise relationship matrix: `(proposal_a, proposal_b) → relationship`.
    pub fn independence_matrix(&self) -> HashMap<(String, String), MutationRelationship> {
        let tested: Vec<&PooledMutation> = self
            .candidates
            .values()
            .filter(|c| c.effect.is_some())
            .collect();

        let mut matrix = HashMap::new();
        for i in 0..tested.len() {
            for j in (i + 1)..tested.len() {
                let a = tested[i];
                let b = tested[j];
                let effect_a = a.effect.as_ref().unwrap();
                let effect_b = b.effect.as_ref().unwrap();
                let relationship = match effect_a.cosine_similarity(effect_b) {
                    Some(cosine) => MutationRelationship::from_cosine(cosine),
                    None => MutationRelationship::Indeterminate,
                };
                let key = (a.proposal_id.clone(), b.proposal_id.clone());
                matrix.insert(key, relationship);
            }
        }
        matrix
    }

    /// Build a recombination plan from the independence matrix.
    ///
    /// Algorithm:
    /// 1. Failed simulations are dropped immediately.
    /// 2. Orthogonal pairs are grouped into one offspring.
    /// 3. Anti-parallel pairs become separate lineages.
    /// 4. Parallel (redundant) pairs: keep the one with higher survival_score.
    ///
    /// Uses a greedy grouping: build the largest orthogonal clique first.
    pub fn build_recombination_plan(&self) -> RecombinationPlan {
        let matrix = self.independence_matrix();

        // Partition by outcome: failed, anti-parallel, orthogonal, parallel.
        let passing: Vec<String> = self
            .candidates
            .values()
            .filter(|c| c.effect.as_ref().is_some_and(|e| e.survival_score >= self.twin.survival_threshold))
            .map(|c| c.proposal_id.clone())
            .collect();

        let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut separate_lineages: Vec<String> = Vec::new();
        let mut redundant: Vec<String> = Vec::new();
        let mut offspring_groups: Vec<Vec<String>> = Vec::new();

        // Greedy orthogonal clique builder.
        for proposal in &passing {
            if assigned.contains(proposal) {
                continue;
            }
            let mut group = vec![proposal.clone()];
            assigned.insert(proposal.clone());

            for other in &passing {
                if assigned.contains(other) {
                    continue;
                }
                // Check if `other` is orthogonal to all members of the current group.
                let all_orthogonal = group.iter().all(|member| {
                    let key = (member.clone(), other.clone());
                    let rev_key = (other.clone(), member.clone());
                    let rel = matrix.get(&key).or_else(|| matrix.get(&rev_key));
                    matches!(rel, Some(MutationRelationship::Orthogonal { .. }) | None)
                });

                let any_anti_parallel = group.iter().any(|member| {
                    let key = (member.clone(), other.clone());
                    let rev_key = (other.clone(), member.clone());
                    let rel = matrix.get(&key).or_else(|| matrix.get(&rev_key));
                    matches!(rel, Some(MutationRelationship::AntiParallel { .. }))
                });

                let any_parallel = group.iter().any(|member| {
                    let key = (member.clone(), other.clone());
                    let rev_key = (other.clone(), member.clone());
                    let rel = matrix.get(&key).or_else(|| matrix.get(&rev_key));
                    matches!(rel, Some(MutationRelationship::Parallel { .. }))
                });

                if any_anti_parallel {
                    if !separate_lineages.contains(other) {
                        separate_lineages.push(other.clone());
                    }
                    assigned.insert(other.clone());
                } else if any_parallel {
                    // Keep whichever has higher survival score.
                    let group_best_score = group
                        .iter()
                        .filter_map(|m| self.candidates.get(m)?.effect.as_ref().map(|e| e.survival_score))
                        .fold(f64::NEG_INFINITY, f64::max);
                    let other_score = self
                        .candidates
                        .get(other)
                        .and_then(|c| c.effect.as_ref())
                        .map_or(0.0, |e| e.survival_score);
                    if other_score > group_best_score {
                        // Replace the group with the better mutation.
                        for m in &group {
                            redundant.push(m.clone());
                        }
                        group.clear();
                        group.push(other.clone());
                    } else {
                        redundant.push(other.clone());
                    }
                    assigned.insert(other.clone());
                } else if all_orthogonal {
                    group.push(other.clone());
                    assigned.insert(other.clone());
                }
            }

            if !group.is_empty() {
                offspring_groups.push(group);
            }
        }

        RecombinationPlan { offspring_groups, separate_lineages, redundant }
    }

    /// Remove a candidate from the pool.
    pub fn remove(&mut self, proposal_id: &str) {
        self.candidates.remove(proposal_id);
    }

    /// Remove all candidates. Called after a recombination plan is executed.
    pub fn clear(&mut self) {
        self.candidates.clear();
    }

    /// Number of candidates currently in the pool.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// `true` when the pool has enough candidates for analysis.
    pub fn ready_for_analysis(&self) -> bool {
        self.candidates.len() >= self.min_pool_size
    }
}

impl Default for MeioticPool {
    fn default() -> Self {
        Self::new()
    }
}

// ── Simulation stage ──────────────────────────────────────────────────────────

/// Stage 5 — Simulation facade.
///
/// Wraps the digital twin and meiotic pool into a single entry point
/// for the orchestration loop.
pub struct SimulationStage {
    pub twin: DigitalTwin,
    pub pool: MeioticPool,
}

impl SimulationStage {
    pub fn new() -> Self {
        Self { twin: DigitalTwin::new(), pool: MeioticPool::new() }
    }
}

impl Default for SimulationStage {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn telos() -> HashMap<String, (f64, f64, f64)> {
        let mut t = HashMap::new();
        t.insert("cpu".to_string(), (0.0, 1.0, 0.3));
        t.insert("latency".to_string(), (0.0, 200.0, 50.0));
        t
    }

    fn baseline() -> HashMap<String, f64> {
        let mut b = HashMap::new();
        b.insert("cpu".to_string(), 0.5);
        b.insert("latency".to_string(), 100.0);
        b
    }

    fn config() -> SimulationConfig {
        SimulationConfig::new(telos(), baseline())
    }

    // ── Effect vector ─────────────────────────────────────────────────────────

    #[test]
    fn cosine_similarity_of_identical_vectors_is_one() {
        let v = EffectVector {
            proposal_id: "a".into(),
            deltas: vec![1.0, 0.0, 0.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        assert!((v.cosine_similarity(&v).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_of_perpendicular_vectors_is_zero() {
        let a = EffectVector {
            proposal_id: "a".into(),
            deltas: vec![1.0, 0.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        let b = EffectVector {
            proposal_id: "b".into(),
            deltas: vec![0.0, 1.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        assert!((a.cosine_similarity(&b).unwrap()).abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_of_opposite_vectors_is_neg_one() {
        let a = EffectVector {
            proposal_id: "a".into(),
            deltas: vec![1.0, 0.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        let b = EffectVector {
            proposal_id: "b".into(),
            deltas: vec![-1.0, 0.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        assert!((a.cosine_similarity(&b).unwrap() - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_returns_none_for_zero_vector() {
        let a = EffectVector {
            proposal_id: "a".into(),
            deltas: vec![0.0, 0.0],
            survival_score: 0.0,
            computed_at: 0,
        };
        let b = EffectVector {
            proposal_id: "b".into(),
            deltas: vec![1.0, 0.0],
            survival_score: 1.0,
            computed_at: 0,
        };
        assert!(a.cosine_similarity(&b).is_none());
    }

    // ── Independence classification ───────────────────────────────────────────

    #[test]
    fn relationship_from_zero_cosine_is_orthogonal() {
        assert_eq!(
            MutationRelationship::from_cosine(0.0),
            MutationRelationship::Orthogonal { cosine: 0.0 }
        );
    }

    #[test]
    fn relationship_from_high_positive_cosine_is_parallel() {
        let r = MutationRelationship::from_cosine(0.9);
        assert!(matches!(r, MutationRelationship::Parallel { .. }));
    }

    #[test]
    fn relationship_from_high_negative_cosine_is_anti_parallel() {
        let r = MutationRelationship::from_cosine(-0.8);
        assert!(matches!(r, MutationRelationship::AntiParallel { .. }));
    }

    // ── Digital twin ──────────────────────────────────────────────────────────

    #[test]
    fn simulation_passes_when_deltas_move_toward_target() {
        let twin = DigitalTwin::new();
        let cfg = config();
        // cpu baseline=0.5, target=0.3 → delta=-0.2 moves toward target
        // latency baseline=100, target=50 → delta=-50 moves toward target
        let mut deltas = HashMap::new();
        deltas.insert("cpu".to_string(), -0.2);
        deltas.insert("latency".to_string(), -50.0);
        let result = twin.simulate("p1", &cfg, &deltas, 0);
        assert!(result.passed, "should pass: {}", result.summary);
        assert!(result.effect.survival_score > 0.8);
    }

    #[test]
    fn simulation_fails_when_deltas_breach_telos_bounds() {
        let twin = DigitalTwin::new();
        let cfg = config();
        // cpu baseline=0.5 + delta=0.8 = 1.3 > max=1.0 → always out of bounds
        let mut deltas = HashMap::new();
        deltas.insert("cpu".to_string(), 0.8);
        deltas.insert("latency".to_string(), 0.0);
        let result = twin.simulate("p1", &cfg, &deltas, 0);
        assert!(!result.passed, "should fail: {}", result.summary);
        assert_eq!(result.effect.survival_score, 0.0);
    }

    #[test]
    fn simulation_effect_vector_positive_when_delta_reduces_distance_to_target() {
        let twin = DigitalTwin::new();
        let cfg = config();
        let mut deltas = HashMap::new();
        deltas.insert("cpu".to_string(), -0.1); // moves 0.5→0.4, closer to target=0.3
        deltas.insert("latency".to_string(), 0.0);
        let result = twin.simulate("p1", &cfg, &deltas, 0);
        // cpu delta should be positive (improvement)
        let cpu_idx = cfg.telos.keys().position(|k| k == "cpu").unwrap_or(0);
        assert!(
            result.effect.deltas[cpu_idx] > 0.0,
            "cpu effect should be positive: {:?}",
            result.effect.deltas
        );
    }

    // ── Meiotic Pool ──────────────────────────────────────────────────────────

    #[test]
    fn pool_ready_for_analysis_after_min_pool_size_reached() {
        let mut pool = MeioticPool::new();
        pool.min_pool_size = 2;
        assert!(!pool.ready_for_analysis());
        pool.add_candidate("a", HashMap::new(), 0);
        assert!(!pool.ready_for_analysis());
        pool.add_candidate("b", HashMap::new(), 0);
        assert!(pool.ready_for_analysis());
    }

    #[test]
    fn isolation_tests_populate_effect_vectors() {
        let mut pool = MeioticPool::new();
        let cfg = config();
        let mut da = HashMap::new();
        da.insert("cpu".to_string(), -0.1);
        da.insert("latency".to_string(), -10.0);
        pool.add_candidate("a", da, 0);
        let results = pool.run_isolation_tests(&cfg, 100);
        assert_eq!(results.len(), 1);
        assert!(pool.candidates["a"].effect.is_some());
    }

    #[test]
    fn orthogonal_mutations_grouped_into_one_offspring() {
        let mut pool = MeioticPool::new();
        // Mutation A: only affects cpu (effect vector [1, 0])
        // Mutation B: only affects latency (effect vector [0, 1])
        // These are orthogonal → should be combined.
        pool.candidates.insert(
            "a".into(),
            PooledMutation {
                proposal_id: "a".into(),
                param_deltas: HashMap::new(),
                effect: Some(EffectVector {
                    proposal_id: "a".into(),
                    deltas: vec![1.0, 0.0],
                    survival_score: 0.9,
                    computed_at: 0,
                }),
                entered_at: 0,
            },
        );
        pool.candidates.insert(
            "b".into(),
            PooledMutation {
                proposal_id: "b".into(),
                param_deltas: HashMap::new(),
                effect: Some(EffectVector {
                    proposal_id: "b".into(),
                    deltas: vec![0.0, 1.0],
                    survival_score: 0.9,
                    computed_at: 0,
                }),
                entered_at: 0,
            },
        );
        let plan = pool.build_recombination_plan();
        assert_eq!(plan.offspring_groups.len(), 1, "should combine into one offspring");
        let group = &plan.offspring_groups[0];
        assert!(group.contains(&"a".to_string()));
        assert!(group.contains(&"b".to_string()));
        assert!(plan.redundant.is_empty());
        assert!(plan.separate_lineages.is_empty());
    }

    #[test]
    fn anti_parallel_mutations_become_separate_lineages() {
        let mut pool = MeioticPool::new();
        pool.candidates.insert(
            "a".into(),
            PooledMutation {
                proposal_id: "a".into(),
                param_deltas: HashMap::new(),
                effect: Some(EffectVector {
                    proposal_id: "a".into(),
                    deltas: vec![1.0, 0.0],
                    survival_score: 0.9,
                    computed_at: 0,
                }),
                entered_at: 0,
            },
        );
        pool.candidates.insert(
            "b".into(),
            PooledMutation {
                proposal_id: "b".into(),
                param_deltas: HashMap::new(),
                effect: Some(EffectVector {
                    proposal_id: "b".into(),
                    deltas: vec![-1.0, 0.0],
                    survival_score: 0.85,
                    computed_at: 0,
                }),
                entered_at: 0,
            },
        );
        let plan = pool.build_recombination_plan();
        // One is in an offspring group, the other in separate_lineages.
        let total_placed =
            plan.offspring_groups.iter().map(|g| g.len()).sum::<usize>()
                + plan.separate_lineages.len();
        assert_eq!(total_placed, 2, "both mutations must be placed");
    }

    #[test]
    fn failed_simulation_excluded_from_plan() {
        let mut pool = MeioticPool::new();
        pool.candidates.insert(
            "failing".into(),
            PooledMutation {
                proposal_id: "failing".into(),
                param_deltas: HashMap::new(),
                effect: Some(EffectVector {
                    proposal_id: "failing".into(),
                    deltas: vec![1.0],
                    survival_score: 0.2, // below 0.8 threshold
                    computed_at: 0,
                }),
                entered_at: 0,
            },
        );
        let plan = pool.build_recombination_plan();
        assert!(plan.offspring_groups.is_empty());
        assert!(plan.separate_lineages.is_empty());
        assert!(plan.redundant.is_empty());
    }

    #[test]
    fn pool_clear_empties_all_candidates() {
        let mut pool = MeioticPool::new();
        pool.add_candidate("a", HashMap::new(), 0);
        pool.add_candidate("b", HashMap::new(), 0);
        pool.clear();
        assert_eq!(pool.len(), 0);
    }
}
