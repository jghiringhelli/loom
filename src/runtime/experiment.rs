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
//! │    1. SignalSimulator → inject 44 signals           │
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

use serde::{Deserialize, Serialize};

use crate::runtime::{
    orchestrator::{Orchestrator, OrchestratorConfig, TickResult},
    signal::now_ms,
    signals_sim::SignalSimulator,
    Runtime,
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
}

// ── BranchingEngine ───────────────────────────────────────────────────────────

/// Monitors stable-mutation accumulation and auto-spawns child entities.
///
/// A branch is triggered when a parent entity has accumulated `threshold`
/// stable mutations.  The child inherits the parent's epigenome and telos
/// bounds (evolved copies, not exact clones).
pub struct BranchingEngine {
    threshold: u32,
    max_branches: u32,
    /// parent_id → stable mutation count since last branch
    stable_counts: HashMap<String, u32>,
    /// parent_id → number of children spawned
    branch_counts: HashMap<String, u32>,
    pub decisions: Vec<BranchDecision>,
}

impl BranchingEngine {
    /// Create a new branching engine.
    pub fn new(threshold: u32, max_branches: u32) -> Self {
        Self {
            threshold,
            max_branches,
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
    pub fn evaluate_and_branch(
        &mut self,
        runtime: &mut Runtime,
        tick: u64,
    ) -> Vec<String> {
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
            let child_id = format!("{parent_id}_b{tick}");
            let child_name = format!("{parent_id} Branch (tick {tick})");

            // Read parent's telos bounds so we can copy them to the child
            let parent_bounds = runtime
                .store
                .telos_bounds_for_entity(&parent_id)
                .unwrap_or_default();

            // Register the child entity
            let telos_json = format!(r#"{{"branched_from":"{parent_id}","tick":{tick}}}"#);
            let spawn_ok = runtime
                .spawn_entity(&child_id, child_name, &telos_json, None, None)
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
}

impl ExperimentDriver {
    /// Create a new driver from a runtime and config.
    pub fn new(runtime: Runtime, config: ExperimentConfig) -> Self {
        let orch_config = OrchestratorConfig {
            tick_interval: std::time::Duration::from_millis(config.tick_interval_ms),
            ..Default::default()
        };

        let simulator = if config.entity_filter.is_empty() {
            SignalSimulator::new(config.rng_seed)
        } else {
            SignalSimulator::new(config.rng_seed)
                .with_filter(config.entity_filter.clone())
        };

        let brancher = BranchingEngine::new(config.branch_threshold, config.max_branches_per_entity);
        let log = ExperimentLog::new(&config.log_path);

        Self {
            orchestrator: Orchestrator::new(runtime, orch_config),
            simulator,
            brancher,
            log,
            config,
        }
    }

    /// Run the experiment for the configured number of ticks.
    ///
    /// Blocks until complete or `stop_requested` is set.
    pub fn run(&mut self, stop_requested: Option<&std::sync::atomic::AtomicBool>) -> ExperimentSummary {
        use std::sync::atomic::Ordering;
        use crate::runtime::deploy::DeployStatus;

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
            let signals = self.simulator.tick(tick, ts);
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

            // Record promoted mutations for branching
            for outcome in &tick_result.deploy_outcomes {
                if outcome.status == DeployStatus::Promoted {
                    self.brancher.record_promotion(&outcome.entity_id);
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
            if self.config.summary_interval > 0
                && tick % self.config.summary_interval == 0
            {
                self.print_summary(tick, &metrics, &totals);
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

        ExperimentSummary {
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
        }
    }
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
        // 5 ticks × 44 signals (11 entities × 4 metrics)
        assert_eq!(summary.total_signals_injected, 5 * 44);
    }

    #[test]
    fn branching_engine_triggers_after_threshold() {
        let mut rt = make_seeded_runtime();
        let mut brancher = BranchingEngine::new(2, 1);

        // Record 2 promotions on climate
        brancher.record_promotion("climate");
        brancher.record_promotion("climate");

        // Should branch
        let branched = brancher.evaluate_and_branch(&mut rt, 100);
        assert_eq!(branched.len(), 1);
        assert!(branched[0].starts_with("climate_b"), "got: {}", branched[0]);
        assert_eq!(brancher.decisions.len(), 1);
        assert_eq!(brancher.decisions[0].stable_mutations_on_parent, 2);
    }

    #[test]
    fn branching_respects_max_branches() {
        let mut rt = make_seeded_runtime();
        let mut brancher = BranchingEngine::new(1, 1); // max 1 branch

        brancher.record_promotion("epidemics");
        brancher.evaluate_and_branch(&mut rt, 50);

        // Second attempt — should not branch (limit reached)
        brancher.record_promotion("epidemics");
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
        assert!(driver.log.ticks.iter().all(|t| t.signals_injected == 44));
    }
}
