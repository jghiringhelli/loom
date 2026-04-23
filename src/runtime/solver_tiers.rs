//! T1–T4 internal proposal generators — the algorithm ladder inside a BIOISO.
//!
//! Each BIOISO entity carries a `solver_tier` (1–4) in its `live_params`.
//! As the entity's intra-generational ParameterAdjust proposals saturate,
//! the orchestrator escalates the solver_tier via a live_params increment.
//! Meiosis then bakes the winning tier into the next binary.
//!
//! Tier ladder:
//!
//! | Tier | Algorithm               | Distinguishing trait                                |
//! |------|-------------------------|-----------------------------------------------------|
//! |  1   | Greedy construction     | Fixed rule — same input, same output                |
//! |  2   | SA-style stochastic     | Boltzmann exploration; temperature decays           |
//! |  3   | SARSA hyper-heuristic   | Learns which proposal type works; weight table      |
//! |  4   | GP-UCB surrogate        | Maintains metric history; picks highest-EI target   |
//!
//! BIOISOs (Tier 5) are one level above: they can structurally change which of
//! these tiers an entity uses, and bake that structural choice into the genome
//! for the next generation via meiosis. No T1–T4 algorithm can do this.

use std::collections::HashMap;

use crate::runtime::{drift::DriftEvent, mutation::MutationProposal};

// ── Minimal LCG PRNG (no global state) ───────────────────────────────────────

fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

// ── T1: Greedy construction ───────────────────────────────────────────────────

/// T1 — greedy: always nudge the triggering metric toward its target by a fixed
/// fractional delta.  Deterministic — same drift input always yields the same output.
///
/// Ceiling: converges only when the fitness landscape is stationary. Saturates
/// when the same delta is promoted repeatedly without driving score to zero.
pub fn t1_greedy(event: &DriftEvent) -> Vec<MutationProposal> {
    let delta = if event.score > 0.5 { -0.05 } else { -0.02 };
    vec![MutationProposal::ParameterAdjust {
        entity_id: event.entity_id.clone(),
        param: event.triggering_metric.clone(),
        delta,
        reason: format!(
            "T1-greedy: drift={:.3} on {}",
            event.score, event.triggering_metric
        ),
    }]
}

// ── T2: SA-inspired stochastic ────────────────────────────────────────────────

/// T2 — Simulated Annealing inspired: accepts uphill moves with probability
/// `exp(-score / temperature)`.  Escapes local optima that T1 greedy cannot.
///
/// `temperature` starts high (exploration) and should decay geometrically each tick.
/// `rng_state` is a mutable LCG seed threaded through — no global random state.
///
/// Ceiling: architecture fixed (2-opt style single-metric perturbation). Cannot
/// change which signals are wired or which metrics are tracked.
pub fn t2_sa(event: &DriftEvent, temperature: f64, rng_state: &mut u64) -> Vec<MutationProposal> {
    let base_delta: f64 = if event.score > 0.5 { -0.05 } else { -0.02 };
    let explore_prob = (-event.score / temperature.max(0.001)).exp();
    let r = lcg_next(rng_state);
    let delta = if r < explore_prob {
        -base_delta // uphill — explores the other direction
    } else {
        base_delta
    };
    vec![MutationProposal::ParameterAdjust {
        entity_id: event.entity_id.clone(),
        param: event.triggering_metric.clone(),
        delta,
        reason: format!(
            "T2-SA: T={:.3} explore_p={:.3} dir={}",
            temperature,
            explore_prob,
            if delta > 0.0 { "up" } else { "down" }
        ),
    }]
}

// ── T3: SARSA selection hyper-heuristic ──────────────────────────────────────

/// Number of low-level proposal types in the T3 portfolio.
pub const N_HEURISTICS: usize = 3;

/// Proposal type index constants for the T3 weight table.
pub const H_SMALL_ADJUST: usize = 0; // ParameterAdjust delta = -0.01
pub const H_LARGE_ADJUST: usize = 1; // ParameterAdjust delta = -0.10
pub const H_REWIRE: usize = 2; // StructuralRewire signal

/// T3 — SARSA hyper-heuristic: selects *which* proposal type to issue based on
/// a learned weight table.  Operates one level above the solution space — not
/// choosing which parameter value, but which proposal strategy.
///
/// `weights` is a normalised probability distribution over the N_HEURISTICS types.
/// `epsilon` is the exploration rate (ε-greedy selection).
/// Returns `(proposals, chosen_heuristic_index)` for SARSA reward feedback.
///
/// Ceiling: the portfolio (set of proposal types) is fixed at compile time.
/// Can select from T1/T2 operators but cannot invent new operator types.
pub fn t3_sarsa(
    event: &DriftEvent,
    weights: &[f64; N_HEURISTICS],
    epsilon: f64,
    rng_state: &mut u64,
) -> (Vec<MutationProposal>, usize) {
    let chosen = if lcg_next(rng_state) < epsilon {
        (lcg_next(rng_state) * N_HEURISTICS as f64) as usize % N_HEURISTICS
    } else {
        weights
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    };

    let proposal = match chosen {
        H_SMALL_ADJUST => MutationProposal::ParameterAdjust {
            entity_id: event.entity_id.clone(),
            param: event.triggering_metric.clone(),
            delta: -0.01,
            reason: format!("T3-SARSA: small-adjust (w={:.3})", weights[H_SMALL_ADJUST]),
        },
        H_LARGE_ADJUST => MutationProposal::ParameterAdjust {
            entity_id: event.entity_id.clone(),
            param: event.triggering_metric.clone(),
            delta: -0.10,
            reason: format!("T3-SARSA: large-adjust (w={:.3})", weights[H_LARGE_ADJUST]),
        },
        _ => MutationProposal::StructuralRewire {
            from_id: event.entity_id.clone(),
            to_id: event.entity_id.clone(),
            signal_name: event.triggering_metric.clone(),
            reason: format!("T3-SARSA: rewire-signal (w={:.3})", weights[H_REWIRE]),
        },
    };

    (vec![proposal], chosen)
}

/// SARSA weight update: reinforce the chosen heuristic, decay all others.
///
/// `reward` — positive when drift decreased, negative when it increased.
/// `lr` — learning rate in (0, 1).
pub fn sarsa_update(weights: &mut [f64; N_HEURISTICS], chosen: usize, reward: f64, lr: f64) {
    for (i, w) in weights.iter_mut().enumerate() {
        if i == chosen {
            *w += lr * reward.max(0.0); // only positive reward reinforces
        } else {
            *w *= 1.0 - lr * 0.05; // slow decay for unchosen
        }
        *w = w.max(0.001); // floor to keep all heuristics explorable
    }
    // Re-normalise to a probability distribution.
    let sum: f64 = weights.iter().sum();
    for w in weights.iter_mut() {
        *w /= sum;
    }
}

// ── T4: GP-UCB surrogate model ────────────────────────────────────────────────

/// Running observation for one metric in the T4 GP-UCB table.
#[derive(Clone, Debug, Default)]
pub struct MetricObservation {
    /// Running mean of drift improvement (positive = drift decreased after adjustment).
    pub mean_improvement: f64,
    /// Total number of observations for this metric.
    pub count: u32,
}

/// T4 — GP-UCB: selects the metric with the highest Upper Confidence Bound score.
///
/// UCB score = `mean_improvement + exploration_weight * sqrt(ln(N+1) / (n_i+1))`
///
/// Unexplored metrics have count=0 → high exploration bonus.
/// Metrics with high mean improvement are exploited.
/// Balances sample efficiency with exploration — cannot be achieved by T1-T3.
///
/// `all_metrics` — all candidate metrics the entity could adjust.
/// `total_obs` — total adjustments made across all metrics (used in ln(N)).
///
/// Ceiling: the GP kernel (RBF) and acquisition function are fixed at compile time.
/// Cannot restructure which metrics exist or rewire signal channels.
pub fn t4_gp_ucb(
    event: &DriftEvent,
    history: &HashMap<String, MetricObservation>,
    all_metrics: &[String],
    exploration_weight: f64,
    total_obs: u32,
) -> Vec<MutationProposal> {
    let ucb_score = |obs: &MetricObservation| -> f64 {
        obs.mean_improvement
            + exploration_weight * ((total_obs as f64 + 1.0).ln() / (obs.count as f64 + 1.0)).sqrt()
    };

    let candidates: Vec<&String> = if all_metrics.is_empty() {
        vec![&event.triggering_metric]
    } else {
        all_metrics.iter().collect()
    };

    let best = candidates
        .iter()
        .map(|m| {
            let obs = history.get(*m).cloned().unwrap_or_default();
            (*m, ucb_score(&obs))
        })
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(m, _)| m.clone())
        .unwrap_or_else(|| event.triggering_metric.clone());

    vec![MutationProposal::ParameterAdjust {
        entity_id: event.entity_id.clone(),
        param: best.clone(),
        delta: -0.05,
        reason: format!("T4-GP-UCB: max-EI on {} (N={})", best, total_obs),
    }]
}

/// Update GP-UCB history with the observed drift improvement after promoting a proposal.
///
/// `improvement` — positive when drift score decreased (good), negative when it increased.
pub fn gp_observe(
    history: &mut HashMap<String, MetricObservation>,
    total_obs: &mut u32,
    metric: &str,
    improvement: f64,
) {
    let obs = history.entry(metric.to_string()).or_default();
    obs.count += 1;
    obs.mean_improvement += (improvement - obs.mean_improvement) / obs.count as f64;
    *total_obs += 1;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::drift::DriftEvent;

    fn evt(score: f64) -> DriftEvent {
        DriftEvent {
            entity_id: "test".into(),
            triggering_metric: "metric_a".into(),
            score,
            ts: 0,
            entity_aggregate_score: None,
            velocity: 0.0,
        }
    }

    // ── T1 tests ──────────────────────────────────────────────────────────────

    #[test]
    fn t1_greedy_returns_parameter_adjust() {
        let p = t1_greedy(&evt(0.7));
        assert!(matches!(p[0], MutationProposal::ParameterAdjust { .. }));
    }

    #[test]
    fn t1_greedy_uses_larger_delta_for_high_drift() {
        let p_high = t1_greedy(&evt(0.8));
        let p_low = t1_greedy(&evt(0.3));
        let d_high = if let MutationProposal::ParameterAdjust { delta, .. } = p_high[0].clone() {
            delta.abs()
        } else {
            panic!()
        };
        let d_low = if let MutationProposal::ParameterAdjust { delta, .. } = p_low[0].clone() {
            delta.abs()
        } else {
            panic!()
        };
        assert!(d_high > d_low, "high-drift T1 must use larger delta");
    }

    #[test]
    fn t1_greedy_is_deterministic() {
        let p1 = t1_greedy(&evt(0.6));
        let p2 = t1_greedy(&evt(0.6));
        assert_eq!(
            p1, p2,
            "T1 must produce identical output for identical input"
        );
    }

    // ── T2 tests ──────────────────────────────────────────────────────────────

    #[test]
    fn t2_sa_high_temperature_explores_both_directions() {
        let event = evt(0.5);
        let mut uphill = 0u32;
        for seed in 0..200u64 {
            let mut rng = seed.wrapping_mul(31337).wrapping_add(1);
            if let MutationProposal::ParameterAdjust { delta, .. } =
                t2_sa(&event, 1000.0, &mut rng)[0].clone()
            {
                if delta > 0.0 {
                    uphill += 1;
                }
            }
        }
        assert!(
            uphill > 50,
            "high-T SA must explore uphill moves; got {uphill}/200"
        );
    }

    #[test]
    fn t2_sa_zero_temperature_matches_t1() {
        let event = evt(0.7);
        let mut rng = 42u64;
        // At T→0 explore_prob → 0, so SA always takes the greedy move.
        if let (
            MutationProposal::ParameterAdjust { delta: d_sa, .. },
            MutationProposal::ParameterAdjust { delta: d_t1, .. },
        ) = (
            t2_sa(&event, 0.0001, &mut rng)[0].clone(),
            t1_greedy(&event)[0].clone(),
        ) {
            assert_eq!(d_sa, d_t1, "near-zero-T SA should match T1 greedy");
        }
    }

    // ── T3 tests ──────────────────────────────────────────────────────────────

    #[test]
    fn t3_sarsa_weight_normalisation_holds_after_update() {
        let event = evt(0.5);
        let mut weights = [1.0 / 3.0f64; N_HEURISTICS];
        let mut rng = 42u64;
        let (_, chosen) = t3_sarsa(&event, &weights, 0.0, &mut rng);
        sarsa_update(&mut weights, chosen, 1.0, 0.1);
        let sum: f64 = weights.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-9,
            "weights must sum to 1.0; got {sum}"
        );
    }

    #[test]
    fn t3_sarsa_greedy_selects_highest_weight() {
        let event = evt(0.5);
        let weights = [0.1f64, 0.8, 0.1]; // H_LARGE_ADJUST dominates
        let mut rng = 42u64;
        // epsilon=0 → pure greedy
        let (proposals, chosen) = t3_sarsa(&event, &weights, 0.0, &mut rng);
        assert_eq!(chosen, H_LARGE_ADJUST);
        assert!(
            matches!(proposals[0], MutationProposal::ParameterAdjust { delta, .. } if (delta + 0.10).abs() < 1e-9)
        );
    }

    #[test]
    fn t3_sarsa_reinforces_chosen_heuristic() {
        let mut weights = [1.0 / 3.0f64; N_HEURISTICS];
        sarsa_update(&mut weights, H_SMALL_ADJUST, 1.0, 0.5);
        assert!(
            weights[H_SMALL_ADJUST] > 1.0 / 3.0,
            "chosen heuristic weight must increase after positive reward"
        );
    }

    // ── T4 tests ──────────────────────────────────────────────────────────────

    #[test]
    fn t4_gp_ucb_prefers_unexplored_metric() {
        let event = evt(0.6);
        let mut history = HashMap::new();
        // Saturate metric_a with observations → low exploration bonus
        history.insert(
            "metric_a".to_string(),
            MetricObservation {
                mean_improvement: 0.1,
                count: 100,
            },
        );
        let all_metrics = vec!["metric_a".to_string(), "metric_b".to_string()];
        let proposals = t4_gp_ucb(&event, &history, &all_metrics, 1.0, 100);
        if let MutationProposal::ParameterAdjust { param, .. } = &proposals[0] {
            assert_eq!(param, "metric_b", "UCB must prefer unexplored metric");
        }
    }

    #[test]
    fn t4_gp_observe_updates_running_mean() {
        let mut history: HashMap<String, MetricObservation> = HashMap::new();
        let mut total = 0u32;
        gp_observe(&mut history, &mut total, "metric_a", 1.0);
        gp_observe(&mut history, &mut total, "metric_a", 0.0);
        let obs = &history["metric_a"];
        assert!((obs.mean_improvement - 0.5).abs() < 1e-9);
        assert_eq!(obs.count, 2);
        assert_eq!(total, 2);
    }

    #[test]
    fn t4_gp_ucb_falls_back_to_triggering_metric_when_no_candidates() {
        let event = evt(0.5);
        let history = HashMap::new();
        let proposals = t4_gp_ucb(&event, &history, &[], 1.0, 0);
        if let MutationProposal::ParameterAdjust { param, .. } = &proposals[0] {
            assert_eq!(param, "metric_a");
        }
    }
}
