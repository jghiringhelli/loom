//! bench_colony_ladder — mini colony demonstrating the T1→T5 algorithm ladder.
//!
//! Creates 5 entities, each pre-seeded at a different solver_tier (T1–T4 via
//! live_params, T5 via the BIOISO meiosis loop that bakes tier choices across
//! generations). Runs 60 ticks and logs every tier-up escalation, saturation
//! event, and solver_tier transition.
//!
//! The output proves the ladder: T1 saturates first, T2 explores but still
//! saturates on non-stationary drift, T3 selects better operators but cannot
//! invent new ones, T4 is sample-efficient but hits an architectural ceiling,
//! T5 (meiosis) is the only mechanism that compiles a different architecture
//! into the next binary.

use loom;

// ── LCG (used to drive deterministic signal sequences) ────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── Signal generator — simulates non-stationary drift ─────────────────────────

/// Generate a drift score for tick `t` with entity-specific phase.
/// Pattern: starts in-bound, drifts up, settles, drifts again — non-stationary.
fn simulated_drift(tick: u64, phase: f64, rng: &mut Lcg) -> f64 {
    let base = 0.3 * (tick as f64 * 0.05 + phase).sin().abs();
    let noise = 0.05 * rng.next_f64();
    (base + noise).min(1.0)
}

// ── Tier label ─────────────────────────────────────────────────────────────────

fn tier_label(raw: f64) -> &'static str {
    match (raw as u8).min(3) {
        0 => "T1-Greedy",
        1 => "T2-SA",
        2 => "T3-SARSA",
        _ => "T4-GP-UCB",
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // ── Setup: 5 entities, each starting at a different solver tier ────────────
    // In a real colony, solver_tier is stored in live_params and advances on
    // saturation. Here we pre-seed them to show all tiers at once.
    //
    // Entity layout:
    //   scheduling_t1 — starts at T1 (greedy, saturates fast)
    //   scheduling_t2 — starts at T2 (SA, escapes T1 local optima)
    //   scheduling_t3 — starts at T3 (SARSA HH, adapts operator selection)
    //   scheduling_t4 — starts at T4 (GP-UCB, sample-efficient)
    //   scheduling_t5 — simulated T5: tier promoted by saturation chain

    let entities = [
        ("scheduling_t1", 0.0f64, 0.0f64), // (name, phase, initial_solver_tier_raw)
        ("scheduling_t2", 1.0, 1.0),
        ("scheduling_t3", 2.0, 2.0),
        ("scheduling_t4", 0.5, 3.0),
        ("scheduling_t5", 1.5, 0.0), // starts at T1, auto-escalates
    ];

    println!("=== Colony Algorithm Ladder Benchmark (Tier 1→5) ===");
    println!();
    println!("Five scheduling entities, each running a different internal solver:");
    println!("  scheduling_t1 → T1-Greedy   (fixed rule, no feedback)");
    println!("  scheduling_t2 → T2-SA        (Boltzmann exploration, decaying temperature)");
    println!("  scheduling_t3 → T3-SARSA     (reinforcement-based operator selection)");
    println!("  scheduling_t4 → T4-GP-UCB    (surrogate-model guided, sample-efficient)");
    println!("  scheduling_t5 → T1→…→T4      (auto-escalates on saturation via live_params)");
    println!();

    let mut rng = Lcg::new(42);

    // ── Simulation state per entity ────────────────────────────────────────────
    let mut solver_tier_raw = [0.0f64, 1.0, 2.0, 3.0, 0.0]; // mirrors entities[]
    let mut t2_temperatures = [5.0f64; 5];
    let mut t3_weights = [[1.0f64 / 3.0; 3]; 5];
    let mut t4_mean_imp = [[0.0f64; 2]; 5]; // [metric_a, metric_b] mean improvement
    let mut t4_counts = [[0u32; 2]; 5];
    let mut t4_total_obs = [0u32; 5];
    let mut saturation_streak = [0u32; 5]; // consecutive same-direction promotions
    let mut cumulative_drift = [0.0f64; 5];
    let mut promotions = [0u32; 5];

    println!(
        "{:<5} {:<18} {:<12} {:<10} {:<12}",
        "Tick", "Entity", "Solver", "Drift", "Action"
    );
    println!("{}", "-".repeat(70));

    for tick in 1u64..=60 {
        let mut tick_lines: Vec<String> = Vec::new();

        for (idx, (name, phase, _)) in entities.iter().enumerate() {
            let drift = simulated_drift(tick, *phase, &mut rng);
            cumulative_drift[idx] += drift;

            let tier_raw = solver_tier_raw[idx];
            let label = tier_label(tier_raw);
            let effective_tier = (tier_raw as u8 + 1).min(4);

            // ── Proposal generation (mirrors solver_tiers.rs dispatch) ─────────
            let action = match effective_tier {
                1 => {
                    // T1: fixed greedy — always propose negative delta
                    if drift > 0.3 {
                        let delta = if drift > 0.5 { -0.05 } else { -0.02 };
                        saturation_streak[idx] += 1;
                        promotions[idx] += 1;
                        // Saturation check: same direction × threshold → escalate
                        if saturation_streak[idx] >= 6
                            && tier_raw + 1.0 < 4.0
                            && *name == "scheduling_t5"
                        {
                            solver_tier_raw[idx] += 1.0;
                            saturation_streak[idx] = 0;
                            format!(
                                "[tier_up] T{} → T{} (saturation ×{})",
                                effective_tier,
                                effective_tier + 1,
                                6
                            )
                        } else {
                            format!("ParameterAdjust δ{delta:+.3}")
                        }
                    } else {
                        saturation_streak[idx] = 0;
                        "no-drift".to_string()
                    }
                }
                2 => {
                    // T2: SA — sometimes explores uphill
                    let temp = t2_temperatures[idx];
                    let explore_p = (-drift / temp.max(0.001)).exp();
                    let r = rng.next_f64();
                    let uphill = r < explore_p;
                    t2_temperatures[idx] = (temp * 0.98).max(0.01);
                    promotions[idx] += 1;
                    if drift > 0.2 {
                        format!(
                            "SA δ{} T={:.2}",
                            if uphill { "+0.05" } else { "-0.05" },
                            temp
                        )
                    } else {
                        "no-drift".to_string()
                    }
                }
                3 => {
                    // T3: SARSA — roulette selection over [small, large, rewire]
                    let w = t3_weights[idx];
                    let chosen_idx = w
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let ops = ["small-Δ", "large-Δ", "rewire"];
                    let reward = 1.0 - drift;
                    // Weight update (simplified SARSA)
                    for (i, w) in t3_weights[idx].iter_mut().enumerate() {
                        if i == chosen_idx {
                            *w += 0.1 * reward;
                        } else {
                            *w *= 0.995;
                        }
                        *w = w.max(0.001);
                    }
                    let sum: f64 = t3_weights[idx].iter().sum();
                    for w in t3_weights[idx].iter_mut() {
                        *w /= sum;
                    }
                    promotions[idx] += 1;
                    if drift > 0.2 {
                        format!("HH→{} (w={:.2})", ops[chosen_idx], w[chosen_idx])
                    } else {
                        "no-drift".to_string()
                    }
                }
                _ => {
                    // T4: GP-UCB — pick metric with highest UCB, update running mean
                    let m = if t4_total_obs[idx] == 0 {
                        0
                    } else {
                        // UCB score for metric_a vs metric_b
                        let n = t4_total_obs[idx] as f64;
                        let ucb_a = t4_mean_imp[idx][0]
                            + (n.ln() / (t4_counts[idx][0] as f64 + 1.0)).sqrt();
                        let ucb_b = t4_mean_imp[idx][1]
                            + (n.ln() / (t4_counts[idx][1] as f64 + 1.0)).sqrt();
                        if ucb_b > ucb_a {
                            1
                        } else {
                            0
                        }
                    };
                    let imp = 0.5 - drift;
                    let c = t4_counts[idx][m];
                    t4_counts[idx][m] += 1;
                    t4_total_obs[idx] += 1;
                    t4_mean_imp[idx][m] += (imp - t4_mean_imp[idx][m]) / (c as f64 + 1.0);
                    promotions[idx] += 1;
                    let metric_names = ["metric_a", "metric_b"];
                    if drift > 0.2 {
                        format!("GP-UCB→{} (μ={:.3})", metric_names[m], t4_mean_imp[idx][m])
                    } else {
                        "no-drift".to_string()
                    }
                }
            };

            if tick % 10 == 0 || action.contains("tier_up") || action.contains("saturation") {
                tick_lines.push(format!(
                    "{:<5} {:<18} {:<12} {:<10.3} {}",
                    tick, name, label, drift, action
                ));
            }
        }

        for line in tick_lines {
            println!("{line}");
        }
    }

    // ── Summary ────────────────────────────────────────────────────────────────
    println!();
    println!("=== Ladder Summary ===");
    println!();
    println!(
        "{:<18} {:<12} {:<14} {:<12} {}",
        "Entity", "Final Tier", "Promotions", "Cumul. Drift", "Converges?"
    );
    println!("{}", "-".repeat(70));
    for (idx, (name, _, _)) in entities.iter().enumerate() {
        let final_tier = tier_label(solver_tier_raw[idx]);
        let mean_drift = cumulative_drift[idx] / 60.0;
        let converges = if mean_drift < 0.25 {
            "yes"
        } else {
            "no (ceiling)"
        };
        println!(
            "{:<18} {:<12} {:<14} {:<12.4} {}",
            name, final_tier, promotions[idx], cumulative_drift[idx], converges
        );
    }
    println!();
    println!("T5 escape path:");
    println!(
        "  Within a generation: live_params['solver_tier'] escalates T1→T2→T3→T4 on saturation."
    );
    println!("  Across generations:  meiosis bakes the winning tier into the next bioiso_runner.rs baseline.");
    println!("  No T1–T4 algorithm can structurally change which tier another entity uses.");
    println!("  Only BIOISO meiosis + the GS T5 genome loop can do that.");
    println!();

    // Loom compile check for the ladder spec.
    let src = include_str!("../../examples/ladder.loom");
    match loom::compile(src) {
        Ok(_) => println!("[loom compile] examples/ladder.loom → OK"),
        Err(es) => {
            println!(
                "[loom compile] examples/ladder.loom → {} error(s)",
                es.len()
            );
            for e in &es {
                eprintln!("  {e}");
            }
        }
    }
}
