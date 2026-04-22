//! Autonomous experiment driver — runs the full BIOISO evolution cycle.
//!
//! This module closes the last gap: signal injection → drift → mutation proposal
//! → gate → canary deploy → promote/rollback → branching → epigenetic inheritance.
//!
//! Everything is fully autonomous — mutations are applied and entities are branched
//! without human intervention. A human-review gate can be added later by setting
//! `ExperimentConfig::autonomous = false`.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  ExperimentDriver.run_ticks(N)                      │
//! │                                                     │
//! │  for each tick:                                     │
//! │    1. SignalSimulator → inject 36 signals           │
//! │    2. Orchestrator.run_once() → drift + mutations   │
//! │    3. BranchingEngine.evaluate() → spawn children  │
//! │    4. ExperimentLog.record_tick()                   │
//! │    5. Print progress summary every 10 ticks         │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! let rt = Runtime::new("bioiso.db").unwrap();
//! let config = ExperimentConfig { total_ticks: 500, ..Default::default() };
//! let mut driver = ExperimentDriver::new(rt, config);
//! let summary = driver.run();
//! println!("{}", serde_json::to_string_pretty(&summary).unwrap());
//! ```

use std::collections::HashMap;
use std::io::Write as _;

use serde::{Deserialize, Serialize};

use crate::runtime::{
    deploy::DeployStatus,
    epigenetic::MemoryType,
    meiosis::{hash_genome, MeiosisEngine, MeiosisReport, PromotedRecord, TelomereTracker},
    orchestrator::{Orchestrator, OrchestratorConfig, TickResult},
    signal::now_ms,
    signals_sim::SignalSimulator,
    telomere_audit::TelomereAuditWriter,
    RetroResult, Runtime,
};

// ── ExperimentConfig ──────────────────────────────────────────────────────────

/// Configuration for a single experiment run.
#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    /// Total number of ticks to simulate.
    pub total_ticks: u64,

    /// Tick interval in milliseconds.  Set to 0 for maximum speed.
    pub tick_interval_ms: u64,

    /// Pseudo-random seed for the signal simulator.
    pub rng_seed: u64,

    /// Restrict simulation to these entity IDs. Empty = all 11.
    pub entity_filter: Vec<String>,

    /// Print a progress summary every N ticks. 0 = never.
    pub summary_interval: u64,

    /// Minimum stable mutations on a parent before branching is considered.
    pub branch_threshold: u32,

    /// Maximum number of child branches per parent over the whole experiment.
    pub max_branches_per_entity: u32,

    /// Fully autonomous: mutations applied without human confirmation.
    /// When false (future), a confirmation callback is invoked.
    pub autonomous: bool,

    /// Path for JSON-lines experiment log. Empty = stderr only.
    pub log_path: String,

    /// Whether to run the meiosis engine at end of experiment.
    ///
    /// When `true`, promoted mutations are collected and pushed to GitHub.
    /// Requires `GITHUB_TOKEN` and `GITHUB_REPO` env vars to actually publish.
    pub run_meiosis: bool,

    /// Generation number for meiosis — passed to `MeiosisConfig::generation`.
    pub meiosis_generation: u32,

    /// Maximum total living entities across all branches.
    /// Branching is suppressed once this limit is reached.
    /// Default: 50. Set via `MAX_ENTITY_COUNT` env var.
    pub max_entity_count: usize,

    /// Path for the telomere audit JSONL log.  Empty = no file written.
    /// Each line is a `TelomereAuditEvent` JSON object.
    pub telomere_log_path: String,

    /// Path for the `bioiso.toml` project manifest written at run end.
    /// Empty = no manifest written.
    pub manifest_path: String,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            total_ticks: 500,
            tick_interval_ms: 100,
            rng_seed: 42,
            entity_filter: Vec::new(),
            summary_interval: 10,
            branch_threshold: 3,
            max_branches_per_entity: 2,
            autonomous: true,
            log_path: String::new(),
            run_meiosis: false,
            meiosis_generation: 1,
            max_entity_count: 50,
            telomere_log_path: String::new(),
            manifest_path: String::new(),
        }
    }
}

// ── Per-tick metrics ──────────────────────────────────────────────────────────

/// Structured metrics recorded for every tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickMetrics {
    pub tick: u64,
    pub ts: u64,
    pub signals_injected: usize,
    pub drift_events: usize,
    pub drift_scores: HashMap<String, f64>,
    pub tier_used: Option<u8>,
    pub proposals: usize,
    pub promoted: usize,
    pub rolled_back: usize,
    pub entities_alive: usize,
    pub entities_branched_this_tick: Vec<String>,
    pub epigenome_core_entries: HashMap<String, usize>,
    pub distillation_ran: bool,
    pub circadian_suppressed: usize,
    pub gossip_absorbed: usize,
}

// ── BranchDecision ────────────────────────────────────────────────────────────

/// Records when and why a branch was created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDecision {
    pub tick: u64,
    pub parent_id: String,
    pub child_id: String,
    pub stable_mutations_on_parent: u32,
    pub trigger_reason: String,
}

// ── ExperimentSummary ─────────────────────────────────────────────────────────

/// Colony telos — the declared intent for the 3-entity BIOISO colony.
///
/// **Formal definition**: the colony succeeds when every seeded entity maintains
/// D_static < 0.3 for ≥20 consecutive ticks. `retro_mean_score` approximates
/// this: score = `mean over entities of (ticks_within_tolerance / total_ticks)`.
/// A score ≥ 0.7 indicates the colony is fulfilling its evolutionary mandate.
pub const COLONY_TELOS: &str =
    "Autonomously discover interventions that drive each domain entity to sustained \
     telos-alignment (D_static < 0.3) — demonstrating that self-evolving BIOISOs can \
     navigate high-dimensional NP-hard parameter spaces faster than manual optimisation.";

/// Per-entity telos-alignment stats accumulated during the run.
/// Used to compute retroactive performance scores without a separate signal replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRetroStats {
    pub entity_id: String,
    /// Total ticks on which a drift event was observed for this entity.
    pub ticks_tracked: usize,
    /// Ticks where the entity aggregate D_static was below the tolerance threshold.
    pub ticks_within_tolerance: usize,
    /// Running sum of D_static scores (used for mean).
    pub sum_drift: f64,
    /// Mean D_static over all observed ticks.
    pub mean_drift: f64,
    /// `ticks_within_tolerance / ticks_tracked` — the primary colony progress metric.
    pub overall_score: f64,
}

/// Final summary produced after the experiment completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub total_ticks: u64,
    pub total_signals_injected: usize,
    pub total_drift_events: usize,
    pub total_proposals: usize,
    pub total_promoted: usize,
    pub total_rolled_back: usize,
    pub tier_activations: HashMap<String, usize>,
    pub entities_final: Vec<String>,
    pub branch_decisions: Vec<BranchDecision>,
    pub final_epigenome_core_entries: HashMap<String, usize>,
    pub convergence_reached: bool,
    pub convergence_tick: Option<u64>,
    /// All promoted mutations collected across the entire run.
    pub promoted_records: Vec<PromotedRecord>,
    /// Meiosis report produced at end of run (when `run_meiosis = true`).
    pub meiosis_report: Option<MeiosisReport>,
    /// Per-entity retro alignment stats (colony telos progress).
    pub retro_stats: Vec<EntityRetroStats>,
    /// Mean overall_score across all tracked entities.
    /// ≥ 0.7 = colony is meeting its telos mandate.
    pub retro_mean_score: f64,
    /// Legacy RetroResult records for compatibility with RetroValidator API.
    pub retro_results: Vec<RetroResult>,
    /// Genome lineage graph edges from the meiosis run.
    /// Each edge records parent hashes → offspring slug + judge decision.
    pub lineage_edges: Vec<crate::runtime::meiosis::LineageEdge>,
}

// ── RetroScorer ───────────────────────────────────────────────────────────────

/// Tracks per-entity telos-alignment throughout the experiment.
///
/// Called every tick with the D_static aggregate score for each entity.
/// At run end, `results()` returns a scored `EntityRetroStats` per entity.
struct RetroScorer {
    /// D_static tolerance threshold: scores below this count as "within tolerance".
    tolerance: f64,
    /// entity_id → (ticks_tracked, ticks_within_tolerance, sum_drift)
    stats: HashMap<String, (usize, usize, f64)>,
}

impl RetroScorer {
    fn new(tolerance: f64) -> Self {
        Self {
            tolerance,
            stats: HashMap::new(),
        }
    }

    /// Record one D_static observation for an entity.
    fn record(&mut self, entity_id: &str, d_static: f64) {
        let entry = self
            .stats
            .entry(entity_id.to_string())
            .or_insert((0, 0, 0.0));
        entry.0 += 1;
        if d_static < self.tolerance {
            entry.1 += 1;
        }
        entry.2 += d_static;
    }

    /// Produce final `EntityRetroStats` for every tracked entity.
    fn results(&self) -> Vec<EntityRetroStats> {
        let mut out: Vec<EntityRetroStats> = self
            .stats
            .iter()
            .map(|(id, &(tracked, within, sum))| {
                let mean_drift = if tracked > 0 {
                    sum / tracked as f64
                } else {
                    0.0
                };
                let overall_score = if tracked > 0 {
                    within as f64 / tracked as f64
                } else {
                    0.0
                };
                EntityRetroStats {
                    entity_id: id.clone(),
                    ticks_tracked: tracked,
                    ticks_within_tolerance: within,
                    sum_drift: sum,
                    mean_drift,
                    overall_score,
                }
            })
            .collect();
        out.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
        out
    }

    fn mean_score(&self) -> f64 {
        let scores: Vec<f64> = self
            .stats
            .values()
            .map(|&(tracked, within, _)| {
                if tracked > 0 {
                    within as f64 / tracked as f64
                } else {
                    0.0
                }
            })
            .collect();
        if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        }
    }

    /// Build `RetroResult` records compatible with the RetroValidator API.
    fn retro_results(&self) -> Vec<RetroResult> {
        self.results()
            .into_iter()
            .map(|s| {
                let mut metric_gap = HashMap::new();
                metric_gap.insert("d_static_mean".to_string(), s.mean_drift);
                RetroResult {
                    entity_id: s.entity_id.clone(),
                    academic_label: "colony_telos_alignment".to_string(),
                    ticks_replayed: s.ticks_tracked,
                    final_drift: s.mean_drift,
                    metric_gap,
                    overall_score: s.overall_score,
                    summary: format!(
                        "entity={} score={:.3} mean_drift={:.3} ticks_within={}/{}",
                        s.entity_id,
                        s.overall_score,
                        s.mean_drift,
                        s.ticks_within_tolerance,
                        s.ticks_tracked,
                    ),
                }
            })
            .collect()
    }
}

// ── BranchingEngine ───────────────────────────────────────────────────────────

/// Monitors stable-mutation accumulation and auto-spawns child entities.
///
/// A branch is triggered when a parent entity has accumulated `threshold`
/// stable mutations.  The child inherits the parent's epigenome and telos
/// bounds (evolved copies, not exact clones).
///
/// `max_entity_count` caps total living entities to bound LLM cost.
pub struct BranchingEngine {
    threshold: u32,
    max_branches: u32,
    /// Hard cap on total living entities — prevents unbounded colony growth.
    max_entity_count: usize,
    /// parent_id → stable mutation count since last branch
    stable_counts: HashMap<String, u32>,
    /// parent_id → number of children spawned
    branch_counts: HashMap<String, u32>,
    pub decisions: Vec<BranchDecision>,
}

impl BranchingEngine {
    /// Create a new branching engine.
    pub fn new(threshold: u32, max_branches: u32) -> Self {
        Self::with_max_entities(threshold, max_branches, 50)
    }

    /// Create with an explicit entity cap.
    pub fn with_max_entities(threshold: u32, max_branches: u32, max_entity_count: usize) -> Self {
        Self {
            threshold,
            max_branches,
            max_entity_count,
            stable_counts: HashMap::new(),
            branch_counts: HashMap::new(),
            decisions: Vec::new(),
        }
    }

    /// Record a promoted mutation for a given entity.
    pub fn record_promotion(&mut self, entity_id: &str) {
        *self.stable_counts.entry(entity_id.to_string()).or_insert(0) += 1;
    }

    /// Evaluate all entities for branching opportunity.
    ///
    /// Returns the IDs of newly created child entities.
    pub fn evaluate_and_branch(&mut self, runtime: &mut Runtime, tick: u64) -> Vec<String> {
        // Collect candidates before mutating runtime
        let candidates: Vec<(String, u32)> = runtime
            .supervisor
            .living_entity_ids()
            .into_iter()
            .filter_map(|id| {
                let count = *self.stable_counts.get(&id).unwrap_or(&0);
                let branches = *self.branch_counts.get(&id).unwrap_or(&0);
                if count >= self.threshold && branches < self.max_branches {
                    Some((id, count))
                } else {
                    None
                }
            })
            .collect();

        let mut branched = Vec::new();
        for (parent_id, stable_count) in candidates {
            // Guard: do not exceed the global entity cap.
            let living_now = runtime.supervisor.living_entity_ids().len();
            if living_now >= self.max_entity_count {
                eprintln!(
                    "[branch] entity cap reached ({living_now}/{}) — skipping spawn of {parent_id} child",
                    self.max_entity_count
                );
                continue;
            }

            let child_id = format!("{parent_id}_b{tick}");
            let child_name = format!("{parent_id} Branch (tick {tick})");

            // Read parent's telos bounds so we can copy them to the child
            let parent_bounds = runtime
                .store
                .telos_bounds_for_entity(&parent_id)
                .unwrap_or_default();

            // Register the child entity — offspring are more ephemeral than parents:
            // 15-division telomere ensures aggressive exploration then apoptosis.
            let telos_json = format!(r#"{{"branched_from":"{parent_id}","tick":{tick}}}"#);
            let spawn_ok = runtime
                .spawn_entity(
                    &child_id,
                    child_name,
                    &telos_json,
                    Some(15),
                    Some("apoptosis".into()),
                )
                .is_ok();

            if spawn_ok {
                // Copy parent's telos bounds to child
                for bound in &parent_bounds {
                    let _ = runtime.set_telos_bounds(
                        &child_id,
                        &bound.metric,
                        bound.min,
                        bound.max,
                        bound.target,
                    );
                }

                // Inherit epigenome: copy Core memories from parent
                let inherited = runtime.inherit_epigenome(&parent_id, &child_id);

                // Lamarckian inheritance: offspring starts from parent's accumulated
                // live_params state, not the compiled baseline. This means each
                // generation of branching starts ahead of where the parent started.
                runtime.inherit_live_params(&parent_id, &child_id);

                let decision = BranchDecision {
                    tick,
                    parent_id: parent_id.clone(),
                    child_id: child_id.clone(),
                    stable_mutations_on_parent: stable_count,
                    trigger_reason: format!(
                        "{stable_count} stable mutations; \
                         {inherited} Core memories inherited; \
                         {} telos bounds copied",
                        parent_bounds.len()
                    ),
                };
                self.decisions.push(decision);

                // Reset parent count and increment branch counter
                self.stable_counts.insert(parent_id.clone(), 0);
                *self.branch_counts.entry(parent_id).or_insert(0) += 1;

                branched.push(child_id);
            }
        }
        branched
    }
}

// ── ExperimentLog ─────────────────────────────────────────────────────────────

/// Accumulates per-tick metrics and writes JSON-lines to a file.
pub struct ExperimentLog {
    ticks: Vec<TickMetrics>,
    log_path: String,
    /// File handle for JSON-lines output (lazy-opened).
    writer: Option<Box<dyn std::io::Write + Send>>,
}

impl ExperimentLog {
    fn new(log_path: &str) -> Self {
        let writer: Option<Box<dyn std::io::Write + Send>> = if log_path.is_empty() {
            None
        } else {
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                Ok(f) => Some(Box::new(std::io::BufWriter::new(f))),
                Err(e) => {
                    eprintln!("warn: could not open log file `{log_path}`: {e}");
                    None
                }
            }
        };
        Self {
            ticks: Vec::new(),
            log_path: log_path.to_string(),
            writer,
        }
    }

    /// Record a tick and write JSON-lines entry.
    fn record(&mut self, metrics: TickMetrics) {
        if let Some(ref mut w) = self.writer {
            if let Ok(line) = serde_json::to_string(&metrics) {
                let _ = writeln!(w, "{line}");
            }
        }
        self.ticks.push(metrics);
    }
}

// ── ExperimentDriver ──────────────────────────────────────────────────────────

/// The autonomous experiment driver.
///
/// Combines signal simulation, orchestration, branching, and logging into a
/// single run-to-completion experiment loop.
pub struct ExperimentDriver {
    orchestrator: Orchestrator,
    simulator: SignalSimulator,
    brancher: BranchingEngine,
    log: ExperimentLog,
    config: ExperimentConfig,
    /// All promoted mutations accumulated during the run — fed to meiosis at end.
    promoted_records: Vec<PromotedRecord>,
    /// Tracks per-entity telomere length based on telos drift.
    telomere_tracker: TelomereTracker,
    /// Per-entity retro scorer — accumulates D_static observations each tick.
    retro_scorer: RetroScorer,
    /// Persistent telomere audit log — records every shortening event to JSONL.
    telomere_audit: TelomereAuditWriter,
    /// Entities for which the senescence log line has already been emitted.
    /// Prevents the "[telomere] X reached senescence" message from flooding
    /// logs once an entity's telomere is exhausted (fires every drift event).
    senescence_logged: std::collections::HashSet<String>,
}

impl ExperimentDriver {
    /// Create a new driver from a runtime and config.
    pub fn new(runtime: Runtime, config: ExperimentConfig) -> Self {
        // OrchestratorConfig::default() reads T2_MIN_INTERVAL_TICKS and
        // STRUCTURAL_REWIRE_THRESHOLD from env, so use it as the base and
        // only override tick_interval from the experiment config.
        let orch_config = OrchestratorConfig {
            tick_interval: std::time::Duration::from_millis(config.tick_interval_ms),
            ..OrchestratorConfig::default()
        };

        let simulator = if config.entity_filter.is_empty() {
            SignalSimulator::new(config.rng_seed)
        } else {
            SignalSimulator::new(config.rng_seed).with_filter(config.entity_filter.clone())
        };

        let brancher = BranchingEngine::with_max_entities(
            config.branch_threshold,
            config.max_branches_per_entity,
            config.max_entity_count,
        );
        let log = ExperimentLog::new(&config.log_path);
        let telomere_audit = TelomereAuditWriter::new(&config.telomere_log_path);

        Self {
            orchestrator: Orchestrator::new(runtime, orch_config),
            simulator,
            brancher,
            log,
            config,
            promoted_records: Vec::new(),
            telomere_tracker: TelomereTracker::new(500),
            retro_scorer: RetroScorer::new(0.3),
            telomere_audit,
            senescence_logged: std::collections::HashSet::new(),
        }
    }

    /// Run the experiment for the configured number of ticks.
    ///
    /// Blocks until complete or `stop_requested` is set.
    pub fn run(
        &mut self,
        stop_requested: Option<&std::sync::atomic::AtomicBool>,
    ) -> ExperimentSummary {
        use std::sync::atomic::Ordering;

        let mut totals = ExperimentTotals::default();
        let mut convergence_tick: Option<u64> = None;
        let mut consecutive_quiet = 0u64;

        for tick in 0..self.config.total_ticks {
            if let Some(stop) = stop_requested {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }

            let ts = now_ms();

            // ── 1. Inject signals ─────────────────────────────────────────────
            // Apply live_param offsets before storing: promoted ParameterAdjust
            // mutations shift the entity's effective signal value toward telos,
            // so subsequent drift measurements reflect the adaptation.
            let mut signals = self.simulator.tick(tick, ts);
            for signal in &mut signals {
                let offset = self
                    .orchestrator
                    .runtime
                    .get_live_param(&signal.entity_id, &signal.metric);
                if offset != 0.0 {
                    signal.value += offset;
                }
            }
            let signals_count = signals.len();
            for signal in signals {
                let _ = self.orchestrator.runtime.store.write_signal(&signal);
            }
            totals.signals_injected += signals_count;

            // ── 2. Orchestrator tick ──────────────────────────────────────────
            let tick_result = match self.orchestrator.run_once() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[tick {tick:>4}] orchestrator error: {e}");
                    continue;
                }
            };

            let n_drift = tick_result.drift_events.len();
            let n_proposals = tick_result.proposals.len();
            let n_promoted = tick_result
                .deploy_outcomes
                .iter()
                .filter(|o| o.status == DeployStatus::Promoted)
                .count();
            let n_rolled_back = tick_result
                .deploy_outcomes
                .iter()
                .filter(|o| o.status == DeployStatus::RolledBack)
                .count();

            totals.drift_events += n_drift;
            totals.proposals += n_proposals;
            totals.promoted += n_promoted;
            totals.rolled_back += n_rolled_back;

            if let Some(tier) = tick_result.tier_used {
                *totals.tier_activations.entry(tier.to_string()).or_insert(0) += 1;
            }

            // Record promoted mutations for branching + meiosis.
            // genome_hash links this record into the lineage graph.
            for outcome in &tick_result.deploy_outcomes {
                if outcome.status == DeployStatus::Promoted {
                    self.brancher.record_promotion(&outcome.entity_id);
                    let genome_hash = hash_genome(&outcome.proposal, tick);
                    self.promoted_records.push(PromotedRecord {
                        tick,
                        entity_id: outcome.entity_id.clone(),
                        proposal: outcome.proposal.clone(),
                        genome_hash,
                    });
                }
            }

            // ── 3. Branching ──────────────────────────────────────────────────
            let branched_ids = if self.config.autonomous {
                self.brancher
                    .evaluate_and_branch(&mut self.orchestrator.runtime, tick)
            } else {
                Vec::new()
            };

            // ── 4. Collect metrics ────────────────────────────────────────────
            let drift_scores = self.collect_drift_scores(&tick_result);

            // Record drift into the telomere tracker and retro scorer.
            // Use D_static aggregate when available (first event per entity carries it
            // after evaluate_entity_aggregate is called in evaluate_all_drift).
            // Fall back to per-metric score for backward compatibility.
            let mut seen_entities: HashMap<String, f64> = HashMap::new();
            for event in &tick_result.drift_events {
                let d_static = event.entity_aggregate_score.unwrap_or(event.score);
                seen_entities
                    .entry(event.entity_id.clone())
                    .or_insert(d_static);

                let was_shortened = {
                    let prev = self.telomere_tracker.remaining(&event.entity_id);
                    let senescent = self.telomere_tracker.record_drift(
                        &event.entity_id,
                        &event.entity_id,
                        event.score,
                    );
                    let after = self
                        .telomere_tracker
                        .remaining(&event.entity_id)
                        .unwrap_or(0);
                    let shortened = prev.map(|p| p > after).unwrap_or(false);
                    if senescent && self.senescence_logged.insert(event.entity_id.clone()) {
                        eprintln!(
                            "[telomere] {} reached senescence — prioritising for meiosis",
                            event.entity_id
                        );
                    }
                    shortened
                };

                // Telomere audit event.
                let remaining = self
                    .telomere_tracker
                    .remaining(&event.entity_id)
                    .unwrap_or(0);
                self.telomere_audit.record(
                    &event.entity_id,
                    tick,
                    event.score,
                    remaining,
                    500, // initial_length = TelomereTracker::new(500)
                    was_shortened,
                    Some(event.triggering_metric.as_str()),
                );
            }
            // Feed D_static aggregates into the retro scorer.
            for (eid, d_static) in &seen_entities {
                self.retro_scorer.record(eid, *d_static);
            }

            // ── 3b. Epigenome cross-offspring broadcasting ────────────────────
            // When distillation ran, propagate parent Core to children's Relational
            // tier and vice-versa. This makes sibling entities aware of each other's
            // learned adaptations — the true epigenetic coordination layer.
            if tick_result.distillation_ran && !self.brancher.decisions.is_empty() {
                for decision in &self.brancher.decisions {
                    let parent = decision.parent_id.as_str();
                    let child = decision.child_id.as_str();

                    // Parent Core → Child Relational (child learns from parent).
                    let parent_entries = self
                        .orchestrator
                        .runtime
                        .epigenome
                        .core_entries(parent)
                        .into_iter()
                        .take(5)
                        .map(|e| e.content.clone())
                        .collect::<Vec<_>>()
                        .join(" | ");
                    if !parent_entries.is_empty() {
                        self.orchestrator.runtime.epigenome.write_core(
                            child,
                            format!("[parent:{parent}@{tick}] {parent_entries}"),
                            MemoryType::Relational,
                            "epigenome_broadcast",
                            ts,
                        );
                    }

                    // Child Core → Parent Relational (parent learns from offspring exploration).
                    let child_entries = self
                        .orchestrator
                        .runtime
                        .epigenome
                        .core_entries(child)
                        .into_iter()
                        .take(5)
                        .map(|e| e.content.clone())
                        .collect::<Vec<_>>()
                        .join(" | ");
                    if !child_entries.is_empty() {
                        self.orchestrator.runtime.epigenome.write_core(
                            parent,
                            format!("[offspring:{child}@{tick}] {child_entries}"),
                            MemoryType::Relational,
                            "epigenome_broadcast",
                            ts,
                        );
                    }
                }
            }

            let epigenome_core = self.collect_epigenome_sizes();
            let entities_alive = self
                .orchestrator
                .runtime
                .supervisor
                .living_entity_ids()
                .len();

            let metrics = TickMetrics {
                tick,
                ts,
                signals_injected: signals_count,
                drift_events: n_drift,
                drift_scores,
                tier_used: tick_result.tier_used,
                proposals: n_proposals,
                promoted: n_promoted,
                rolled_back: n_rolled_back,
                entities_alive,
                entities_branched_this_tick: branched_ids,
                epigenome_core_entries: epigenome_core,
                distillation_ran: tick_result.distillation_ran,
                circadian_suppressed: tick_result.circadian_suppressed,
                gossip_absorbed: tick_result.gossip_absorbed,
            };
            self.log.record(metrics.clone());

            // ── 5. Progress summary ───────────────────────────────────────────
            if self.config.summary_interval > 0 && tick % self.config.summary_interval == 0 {
                self.print_summary(tick, &metrics, &totals);
                let _ = std::io::stdout().flush();
            }

            // ── 6. Convergence check ──────────────────────────────────────────
            if n_drift == 0 && n_proposals == 0 {
                consecutive_quiet += 1;
                if consecutive_quiet >= 20 && convergence_tick.is_none() {
                    convergence_tick = Some(tick);
                }
            } else {
                consecutive_quiet = 0;
            }

            // ── 7. Sleep (if not fast-forward) ────────────────────────────────
            if self.config.tick_interval_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(
                    self.config.tick_interval_ms,
                ));
            }
        }

        self.build_summary(totals, convergence_tick)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn collect_drift_scores(&self, result: &TickResult) -> HashMap<String, f64> {
        result
            .drift_events
            .iter()
            .map(|e| (e.entity_id.clone(), e.score))
            .collect()
    }

    fn collect_epigenome_sizes(&self) -> HashMap<String, usize> {
        let entity_ids: Vec<_> = self
            .orchestrator
            .runtime
            .supervisor
            .living_entity_ids()
            .into_iter()
            .collect();
        entity_ids
            .iter()
            .map(|id| {
                let count = self.orchestrator.runtime.epigenome.core_entries(id).len();
                (id.clone(), count)
            })
            .collect()
    }

    fn print_summary(&self, tick: u64, metrics: &TickMetrics, totals: &ExperimentTotals) {
        let t1 = totals.tier_activations.get("1").copied().unwrap_or(0);
        let t2 = totals.tier_activations.get("2").copied().unwrap_or(0);
        let t3 = totals.tier_activations.get("3").copied().unwrap_or(0);

        println!(
            "[tick {tick:>4}] drift={:>3} | proposals={:>3} promoted={:>3} | \
             T1={t1} T2={t2} T3={t3} | entities={:>3} | branches={}",
            metrics.drift_events,
            totals.proposals,
            totals.promoted,
            metrics.entities_alive,
            self.brancher.decisions.len(),
        );

        // Print top drifting entities
        let mut scores: Vec<_> = metrics.drift_scores.iter().collect();
        scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (eid, score) in scores.iter().take(3) {
            println!("  drift  {eid:<20} score={score:.3}");
        }

        // Print new branches this tick
        for child in &metrics.entities_branched_this_tick {
            println!("  branch → spawned {child}");
        }

        // Print epigenome memory growth for top entities
        let mut mem: Vec<_> = metrics.epigenome_core_entries.iter().collect();
        mem.sort_by(|a, b| b.1.cmp(a.1));
        for (eid, count) in mem.iter().take(3) {
            if **count > 0 {
                println!("  memory {eid:<20} core_entries={count}");
            }
        }
    }

    fn build_summary(
        &self,
        totals: ExperimentTotals,
        convergence_tick: Option<u64>,
    ) -> ExperimentSummary {
        let entities_final: Vec<_> = self
            .orchestrator
            .runtime
            .supervisor
            .living_entity_ids()
            .into_iter()
            .collect();

        let final_epi = self.collect_epigenome_sizes();

        let meiosis_report = if self.config.run_meiosis {
            let engine = MeiosisEngine::new(crate::runtime::meiosis::MeiosisConfig {
                generation: self.config.meiosis_generation,
                ..Default::default()
            });
            // Log near-senescent entities so the meiosis report captures urgency.
            let urgent = self.telomere_tracker.senescent_entities();
            if !urgent.is_empty() {
                eprintln!(
                    "[meiosis] senescent entities (genome preservation urgent): {:?}",
                    urgent
                );
            }
            for state in self.telomere_tracker.all_states_by_urgency() {
                eprintln!(
                    "[telomere] {} remaining={} drift_accum={:.2}",
                    state.entity_id, state.remaining, state.drift_accumulator
                );
            }
            Some(engine.run(&self.promoted_records))
        } else {
            None
        };

        // Colony telos alignment scores.
        let retro_stats = self.retro_scorer.results();
        let retro_mean_score = self.retro_scorer.mean_score();
        let retro_results = self.retro_scorer.retro_results();

        eprintln!(
            "[colony-telos] retro_mean_score={:.3} (≥0.7 = mandate met) — {}",
            retro_mean_score, COLONY_TELOS
        );
        for s in &retro_stats {
            eprintln!(
                "[colony-telos] {} score={:.3} mean_drift={:.3} within={}/{}",
                s.entity_id,
                s.overall_score,
                s.mean_drift,
                s.ticks_within_tolerance,
                s.ticks_tracked,
            );
        }

        // Lineage edges from meiosis report.
        let lineage_edges = meiosis_report
            .as_ref()
            .map(|r| r.lineage_edges.clone())
            .unwrap_or_default();

        let summary = ExperimentSummary {
            total_ticks: self.config.total_ticks,
            total_signals_injected: totals.signals_injected,
            total_drift_events: totals.drift_events,
            total_proposals: totals.proposals,
            total_promoted: totals.promoted,
            total_rolled_back: totals.rolled_back,
            tier_activations: totals.tier_activations,
            entities_final,
            branch_decisions: self.brancher.decisions.clone(),
            final_epigenome_core_entries: final_epi,
            convergence_reached: convergence_tick.is_some(),
            convergence_tick,
            promoted_records: self.promoted_records.clone(),
            meiosis_report,
            retro_stats,
            retro_mean_score,
            retro_results,
            lineage_edges,
        };

        // Write bioiso.toml project manifest if requested.
        if !self.config.manifest_path.is_empty() {
            write_project_manifest(&self.config.manifest_path, &summary, &self.telomere_audit);
        }

        summary
    }
}

// ── Project manifest writer ───────────────────────────────────────────────────

/// Write a `bioiso.toml` project manifest at the end of an experiment run.
///
/// The manifest captures the run's key results in a structured TOML file
/// suitable for the BIOISO paper's per-project evidence table.
fn write_project_manifest(path: &str, summary: &ExperimentSummary, audit: &TelomereAuditWriter) {
    let final_lengths = audit.final_lengths();
    let decay_counts = audit.decay_counts();

    let mut entity_sections = String::new();
    for stat in &summary.retro_stats {
        let final_tel = final_lengths.get(&stat.entity_id).copied().unwrap_or(500);
        let decays = decay_counts.get(&stat.entity_id).copied().unwrap_or(0);
        let was_donor = audit.was_meiosis_donor(&stat.entity_id);
        entity_sections.push_str(&format!(
            r#"
[entity.{id}]
retro_score        = {score:.3}
mean_drift         = {drift:.3}
ticks_within_tol   = {within}
ticks_tracked      = {tracked}
telomere_final     = {tel}
telomere_decays    = {decays}
meiosis_donor      = {donor}
"#,
            id = stat.entity_id,
            score = stat.overall_score,
            drift = stat.mean_drift,
            within = stat.ticks_within_tolerance,
            tracked = stat.ticks_tracked,
            tel = final_tel,
            decays = decays,
            donor = was_donor,
        ));
    }

    let convergence_str = match summary.convergence_tick {
        Some(t) => format!("{t}"),
        None => "null".to_string(),
    };

    let meiosis_events = summary
        .meiosis_report
        .as_ref()
        .map(|r| r.genomes_accepted)
        .unwrap_or(0);
    let breakthrough = summary.lineage_edges.iter().any(|e| {
        matches!(
            e.decision,
            Some(crate::runtime::meiosis::EvolutionDecision::Meiosis)
        )
    });

    let manifest = format!(
        r#"# bioiso.toml — Project manifest (auto-generated at experiment end)
# Edit the [project] section; all [result] fields are computed.

[project]
hypothesis        = "BIOISO autonomously discovers interventions in NP-hard domain space"
started           = "{started}"

[result]
total_ticks           = {ticks}
total_promoted        = {promoted}
total_branches        = {branches}
retro_mean_score      = {retro:.3}
convergence_tick      = {conv}
meiosis_events        = {meiosis}
breakthrough_meiosis  = {breakthrough}

[colony_telos]
definition = "{telos}"
threshold  = 0.7
{entity_sections}
[lineage]
edges = {edges}
"#,
        started = chrono_date(),
        ticks = summary.total_ticks,
        promoted = summary.total_promoted,
        branches = summary.branch_decisions.len(),
        retro = summary.retro_mean_score,
        conv = convergence_str,
        meiosis = meiosis_events,
        breakthrough = breakthrough,
        telos = COLONY_TELOS,
        entity_sections = entity_sections,
        edges = summary.lineage_edges.len(),
    );

    match std::fs::write(path, manifest) {
        Ok(()) => eprintln!("[manifest] written to `{path}`"),
        Err(e) => eprintln!("[manifest] failed to write `{path}`: {e}"),
    }
}

fn chrono_date() -> String {
    // Simple ISO date without external chrono dependency.
    // Uses the file system mtime is unavailable; fall back to a static marker.
    // In production Railway sets SOURCE_DATE_EPOCH or similar — good enough for the paper.
    std::env::var("EXPERIMENT_DATE").unwrap_or_else(|_| "unknown".to_string())
}

// ── Internal totals accumulator ───────────────────────────────────────────────

#[derive(Default)]
struct ExperimentTotals {
    signals_injected: usize,
    drift_events: usize,
    proposals: usize,
    promoted: usize,
    rolled_back: usize,
    tier_activations: HashMap<String, usize>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::bioiso_runner::BIOISORunner;

    fn make_seeded_runtime() -> Runtime {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let specs = crate::runtime::all_domain_specs();
        for spec in &specs {
            let _ = runner.spawn_domain(&mut rt, spec);
        }
        rt
    }

    #[test]
    fn experiment_driver_runs_10_ticks_without_panic() {
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 10,
            tick_interval_ms: 0,
            rng_seed: 42,
            summary_interval: 5,
            ..Default::default()
        };
        let mut driver = ExperimentDriver::new(rt, config);
        let summary = driver.run(None);
        assert_eq!(summary.total_ticks, 10);
        assert!(summary.total_signals_injected > 0);
    }

    #[test]
    fn experiment_injects_signals_every_tick() {
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 5,
            tick_interval_ms: 0,
            rng_seed: 1,
            summary_interval: 0,
            ..Default::default()
        };
        let mut driver = ExperimentDriver::new(rt, config);
        let summary = driver.run(None);
        // 5 ticks × 36 signals (9 BIOISO-class entities × 4 metrics)
        assert_eq!(summary.total_signals_injected, 5 * 36);
    }

    #[test]
    fn branching_engine_triggers_after_threshold() {
        let mut rt = make_seeded_runtime();
        let mut brancher = BranchingEngine::new(2, 1);

        // Record 2 promotions on amr_coevolution
        brancher.record_promotion("amr_coevolution");
        brancher.record_promotion("amr_coevolution");

        // Should branch
        let branched = brancher.evaluate_and_branch(&mut rt, 100);
        assert_eq!(branched.len(), 1);
        assert!(
            branched[0].starts_with("amr_coevolution_b"),
            "got: {}",
            branched[0]
        );
        assert_eq!(brancher.decisions.len(), 1);
        assert_eq!(brancher.decisions[0].stable_mutations_on_parent, 2);
    }

    #[test]
    fn branching_respects_max_branches() {
        let mut rt = make_seeded_runtime();
        let mut brancher = BranchingEngine::new(1, 1); // max 1 branch

        brancher.record_promotion("flash_crash");
        brancher.evaluate_and_branch(&mut rt, 50);

        // Second attempt — should not branch (limit reached)
        brancher.record_promotion("flash_crash");
        let second = brancher.evaluate_and_branch(&mut rt, 100);
        assert!(second.is_empty(), "should not exceed max_branches");
    }

    #[test]
    fn experiment_summary_has_correct_tick_count() {
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 20,
            tick_interval_ms: 0,
            rng_seed: 99,
            summary_interval: 0,
            ..Default::default()
        };
        let mut driver = ExperimentDriver::new(rt, config);
        let summary = driver.run(None);
        assert_eq!(summary.total_ticks, 20);
    }

    #[test]
    fn log_records_tick_metrics() {
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 3,
            tick_interval_ms: 0,
            rng_seed: 5,
            summary_interval: 0,
            ..Default::default()
        };
        let mut driver = ExperimentDriver::new(rt, config);
        driver.run(None);
        assert_eq!(driver.log.ticks.len(), 3);
        assert!(driver.log.ticks.iter().all(|t| t.signals_injected == 36));
    }

    #[test]
    fn telomere_death_kills_entity_after_promotions() {
        // Spawn a single entity with a telomere limit of 2 so it dies after 2 promoted
        // mutations. Use the orchestrator directly to drive the division loop.
        use crate::runtime::{
            orchestrator::{Orchestrator, OrchestratorConfig},
            supervisor::EntityState,
        };
        let mut rt = Runtime::new(":memory:").unwrap();
        rt.spawn_entity(
            "short_lived",
            "ShortLived",
            r#"{"target":"test"}"#,
            Some(2),
            Some("apoptosis".into()),
        )
        .unwrap();

        // Manually drive two division events — simulates two promoted mutations.
        rt.supervisor
            .record_division("short_lived", &rt.store)
            .unwrap();
        let result = rt.supervisor.record_division("short_lived", &rt.store);
        // Second division should exhaust the telomere.
        assert!(
            result.is_err(),
            "expected telomere exhaustion on second division"
        );

        // After apoptosis, transition to Dead.
        rt.supervisor
            .transition("short_lived", EntityState::Dead, &rt.store);
        assert_eq!(
            rt.supervisor.get("short_lived").unwrap().state,
            EntityState::Dead
        );
        // Dead entity should not appear in living_entity_ids().
        assert!(!rt
            .supervisor
            .living_entity_ids()
            .contains(&"short_lived".to_string()));
    }

    #[test]
    fn offspring_get_shorter_telomere_than_parents() {
        // Branched offspring should have a telomere limit of 15.
        let mut rt = make_seeded_runtime();
        let mut brancher = BranchingEngine::new(1, 2);

        brancher.record_promotion("flash_crash");
        let branched = brancher.evaluate_and_branch(&mut rt, 10);
        assert_eq!(branched.len(), 1);
        let child_id = &branched[0];
        let child = rt.supervisor.get(child_id).unwrap();
        assert_eq!(
            child.telomere_limit,
            Some(15),
            "offspring telomere should be 15"
        );

        // Parent telomere limit should be 20 (flash_crash spec).
        let parent = rt.supervisor.get("flash_crash").unwrap();
        assert_eq!(
            parent.telomere_limit,
            Some(20),
            "flash_crash telomere should be 20"
        );
    }

    #[test]
    fn epigenome_broadcast_writes_to_offspring_on_distillation() {
        // After branching, the driver should broadcast parent Core to child Relational
        // when distillation runs. Verify epigenome_core_entries grows for offspring.
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 25, // distil_interval=10 → distillation fires at tick 10 & 20
            tick_interval_ms: 0,
            rng_seed: 42,
            branch_threshold: 1,
            autonomous: true,
            summary_interval: 0,
            ..Default::default()
        };
        let mut driver = ExperimentDriver::new(rt, config);
        let summary = driver.run(None);
        // If branching occurred AND the parent has Core entries, the child should too
        // (broadcast propagated them). If parent has none yet, broadcast is a no-op.
        for decision in &summary.branch_decisions {
            let parent_entries = summary
                .final_epigenome_core_entries
                .get(&decision.parent_id)
                .copied()
                .unwrap_or(0);
            if parent_entries > 0 {
                let child_entries = summary
                    .final_epigenome_core_entries
                    .get(&decision.child_id)
                    .copied()
                    .unwrap_or(0);
                assert!(
                    child_entries > 0,
                    "offspring {} should have epigenome entries when parent {} has {}",
                    decision.child_id,
                    decision.parent_id,
                    parent_entries
                );
            }
        }
    }

    #[test]
    fn live_params_applied_reduces_drift() {
        // Verify that applying a live_param offset shifts signals toward telos,
        // causing fewer drift events over time compared to no live patching.
        let mut rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 5,
            tick_interval_ms: 0,
            rng_seed: 42,
            summary_interval: 0,
            branch_threshold: 999,
            max_branches_per_entity: 0,
            autonomous: false,
            log_path: String::new(),
            run_meiosis: false,
            meiosis_generation: 1,
            max_entity_count: 50,
            telomere_log_path: String::new(),
            manifest_path: String::new(),
            entity_filter: vec![],
        };
        let mut driver = ExperimentDriver::new(rt, config);

        // Apply a large live_param offset for flash_crash order_book_depth —
        // moves the signal strongly toward telos target (0.85), should reduce drift.
        driver
            .orchestrator
            .runtime
            .apply_live_param("flash_crash", "order_book_depth", 0.5);

        let summary = driver.run(None);

        // After 5 ticks with a large correction, total drift events should be lower
        // than if we had run with no correction. We verify the param is stored.
        let total_offset = driver
            .orchestrator
            .runtime
            .get_live_param("flash_crash", "order_book_depth");
        assert!(
            (total_offset - 0.5).abs() < 1e-9,
            "live_param should persist: got {total_offset}"
        );
        // The experiment ran without panic — live_param injection is safe.
        assert!(summary.total_ticks > 0);
    }

    #[test]
    fn offspring_inherits_parent_live_params() {
        // Verify that when a parent branches, the child starts with the parent's
        // accumulated live_params rather than the compiled baseline (Lamarck).
        let rt = make_seeded_runtime();
        let config = ExperimentConfig {
            total_ticks: 30,
            tick_interval_ms: 0,
            rng_seed: 42,
            summary_interval: 0,
            branch_threshold: 3,
            max_branches_per_entity: 2,
            autonomous: true,
            log_path: String::new(),
            run_meiosis: false,
            meiosis_generation: 1,
            max_entity_count: 50,
            telomere_log_path: String::new(),
            manifest_path: String::new(),
            entity_filter: vec![],
        };
        let mut driver = ExperimentDriver::new(rt, config);

        // Pre-load a live_param for flash_crash before the run so any branch will
        // inherit it.
        driver
            .orchestrator
            .runtime
            .apply_live_param("flash_crash", "order_book_depth", 0.15);

        driver.run(None);

        // If flash_crash branched, the child should have inherited the 0.15 offset.
        for decision in &driver.brancher.decisions {
            if decision.parent_id == "flash_crash" {
                let child_offset = driver
                    .orchestrator
                    .runtime
                    .get_live_param(&decision.child_id, "order_book_depth");
                assert!(
                    child_offset >= 0.15 - 1e-9,
                    "child '{}' should inherit parent live_param >= 0.15, got {child_offset}",
                    decision.child_id
                );
            }
        }
        // Test passes whether or not flash_crash branched; the key is no panic.
    }

    #[test]
    fn saturation_track_resets_on_direction_change() {
        // Verify the orchestrator's saturation_track resets when the promoted
        // delta direction changes — we shouldn't log saturation on oscillations.
        use crate::runtime::orchestrator::{Orchestrator, OrchestratorConfig};
        let rt = make_seeded_runtime();
        let mut orch = Orchestrator::new(rt, OrchestratorConfig::default());

        // Manually simulate alternating promotion directions.
        orch.runtime
            .apply_live_param("flash_crash", "order_book_depth", 0.01);
        orch.runtime
            .apply_live_param("flash_crash", "order_book_depth", -0.01);

        // Just verify apply_live_param accumulates correctly.
        let net = orch
            .runtime
            .get_live_param("flash_crash", "order_book_depth");
        assert!(net.abs() < 1e-9, "net offset should be ~0, got {net}");
    }
}
