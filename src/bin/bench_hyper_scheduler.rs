//! bench_hyper_scheduler — Reinforcement-based selection hyper-heuristic
//! for dynamic job scheduling (Tier 3).
//!
//! Implements a SARSA-style weight table over 5 low-level heuristics:
//! SPT, LPT, EDD, FIFO, CR. Roulette-wheel selection; reward = -Δ(TWT).

use loom;

// ── Minimal LCG PRNG ─────────────────────────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        const A: u64 = 6364136223846793005;
        const C: u64 = 1442695040888963407;
        self.state = self.state.wrapping_mul(A).wrapping_add(C);
        (self.state >> 33) as u32
    }

    /// Uniform float in [0, 1)
    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / u32::MAX as f64
    }
}

// ── Job data ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Job {
    id: usize,
    processing_time: f64,
    due_date: f64,
    arrival_time: f64,
    weight: f64,
}

fn build_jobs() -> Vec<Job> {
    let arrivals: [f64; 30] = [
        0.0, 0.5, 1.0, 2.0, 2.0, 3.0, 4.0, 5.0, 5.0, 6.0, 7.0, 8.0, 8.0, 9.0, 10.0, 10.0, 11.0,
        12.0, 13.0, 14.0, 15.0, 16.0, 16.0, 17.0, 18.0, 19.0, 20.0, 20.0, 21.0, 22.0,
    ];
    let proc: [f64; 30] = [
        3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0, 5.0, 3.0, 5.0, 8.0, 9.0, 7.0, 9.0, 3.0, 2.0, 3.0,
        8.0, 4.0, 6.0, 2.0, 6.0, 4.0, 3.0, 3.0, 8.0, 3.0, 2.0, 7.0,
    ];
    let slack: [f64; 30] = [
        2.0, 5.0, 1.0, 8.0, 3.0, 6.0, 2.0, 4.0, 7.0, 3.0, 5.0, 1.0, 9.0, 2.0, 6.0, 4.0, 3.0, 8.0,
        2.0, 5.0, 7.0, 1.0, 4.0, 6.0, 3.0, 2.0, 8.0, 3.0, 4.0, 5.0,
    ];
    (0..30)
        .map(|i| Job {
            id: i,
            processing_time: proc[i],
            due_date: arrivals[i] + proc[i] + slack[i],
            arrival_time: arrivals[i],
            weight: 1.0,
        })
        .collect()
}

// ── Scheduling simulation ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum Heuristic {
    Spt,
    Lpt,
    Edd,
    Fifo,
    Cr,
}

const N_HEURISTICS: usize = 5;

/// Select the best job from the queue given a heuristic and current clock.
fn select_job(queue: &[Job], heuristic: Heuristic, clock: f64) -> usize {
    match heuristic {
        Heuristic::Spt => queue
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.processing_time.partial_cmp(&b.processing_time).unwrap())
            .map(|(i, _)| i)
            .unwrap(),
        Heuristic::Lpt => queue
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.processing_time.partial_cmp(&b.processing_time).unwrap())
            .map(|(i, _)| i)
            .unwrap(),
        Heuristic::Edd => queue
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.due_date.partial_cmp(&b.due_date).unwrap())
            .map(|(i, _)| i)
            .unwrap(),
        Heuristic::Fifo => {
            // earliest arrival time
            queue
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.arrival_time.partial_cmp(&b.arrival_time).unwrap())
                .map(|(i, _)| i)
                .unwrap()
        }
        Heuristic::Cr => {
            // critical ratio = (due_date - clock) / processing_time
            // select minimum CR; if CR <= 0 job is already late, prioritise most overdue
            queue
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let cr_a = if a.processing_time > 0.0 {
                        (a.due_date - clock) / a.processing_time
                    } else {
                        f64::NEG_INFINITY
                    };
                    let cr_b = if b.processing_time > 0.0 {
                        (b.due_date - clock) / b.processing_time
                    } else {
                        f64::NEG_INFINITY
                    };
                    cr_a.partial_cmp(&cr_b).unwrap()
                })
                .map(|(i, _)| i)
                .unwrap()
        }
    }
}

/// Run a fixed-heuristic schedule, return (total_weighted_tardiness, tardy_count).
fn run_fixed(jobs: &[Job], heuristic: Heuristic) -> (f64, usize) {
    let mut remaining: Vec<Job> = jobs.to_vec();
    let mut clock = 0.0f64;
    let mut twt = 0.0f64;
    let mut tardy = 0usize;

    while !remaining.is_empty() {
        // Advance clock to earliest arrival of any remaining job if queue is empty now
        let available: Vec<usize> = remaining
            .iter()
            .enumerate()
            .filter(|(_, j)| j.arrival_time <= clock + 1e-9)
            .map(|(i, _)| i)
            .collect();

        let available_jobs: Vec<Job> = if available.is_empty() {
            // Jump clock to next arrival
            let next_arrival = remaining
                .iter()
                .map(|j| j.arrival_time)
                .fold(f64::INFINITY, f64::min);
            clock = next_arrival;
            remaining
                .iter()
                .enumerate()
                .filter(|(_, j)| j.arrival_time <= clock + 1e-9)
                .map(|(_, j)| j.clone())
                .collect()
        } else {
            available.iter().map(|&i| remaining[i].clone()).collect()
        };

        let chosen_idx = select_job(&available_jobs, heuristic, clock);
        let chosen = available_jobs[chosen_idx].clone();

        clock += chosen.processing_time;
        let tardiness = (clock - chosen.due_date).max(0.0) * chosen.weight;
        twt += tardiness;
        if tardiness > 0.0 {
            tardy += 1;
        }

        remaining.retain(|j| j.id != chosen.id);
    }

    (twt, tardy)
}

/// Roulette-wheel selection from weights (already normalised).
fn roulette(weights: &[f64], rng: &mut Lcg) -> usize {
    let r = rng.next_f64();
    let mut cumsum = 0.0;
    for (i, &w) in weights.iter().enumerate() {
        cumsum += w;
        if r <= cumsum {
            return i;
        }
    }
    weights.len() - 1
}

fn heuristic_from_idx(idx: usize) -> Heuristic {
    match idx {
        0 => Heuristic::Spt,
        1 => Heuristic::Lpt,
        2 => Heuristic::Edd,
        3 => Heuristic::Fifo,
        _ => Heuristic::Cr,
    }
}

/// Run the adaptive hyper-heuristic.
/// Returns (twt, tardy, usage_counts).
fn run_hyper(jobs: &[Job], learning_rate: f64) -> (f64, usize, [usize; N_HEURISTICS]) {
    let mut rng = Lcg::new(12345);
    let mut weights = [0.2f64; N_HEURISTICS]; // uniform initial weights
    let mut usage = [0usize; N_HEURISTICS];

    let mut remaining: Vec<Job> = jobs.to_vec();
    let mut clock = 0.0f64;
    let mut twt = 0.0f64;
    let mut tardy = 0usize;

    while !remaining.is_empty() {
        // Build available queue
        let mut available: Vec<Job> = remaining
            .iter()
            .filter(|j| j.arrival_time <= clock + 1e-9)
            .cloned()
            .collect();

        if available.is_empty() {
            let next_arrival = remaining
                .iter()
                .map(|j| j.arrival_time)
                .fold(f64::INFINITY, f64::min);
            clock = next_arrival;
            available = remaining
                .iter()
                .filter(|j| j.arrival_time <= clock + 1e-9)
                .cloned()
                .collect();
        }

        // Compute current TWT of remaining available jobs (pre-dispatch)
        // Use a simple lookahead: what would happen if we dispatched best SPT order?
        // Actually: reward = -(change in tardiness of the dispatched job vs average)
        // We use the simpler formulation: reward = -(completion_tardiness_contribution)
        // and compare across heuristics implicitly via weight updates.

        // Select heuristic via roulette wheel
        let h_idx = roulette(&weights, &mut rng);
        let heuristic = heuristic_from_idx(h_idx);
        usage[h_idx] += 1;

        let chosen_idx = select_job(&available, heuristic, clock);
        let chosen = available[chosen_idx].clone();

        let completion = clock + chosen.processing_time;
        let job_tardiness = (completion - chosen.due_date).max(0.0) * chosen.weight;

        // Compute what SPT (best greedy) would have given for this slot
        let spt_idx = select_job(&available, Heuristic::Spt, clock);
        let spt_job = &available[spt_idx];
        let spt_completion = clock + spt_job.processing_time;
        let spt_tardiness = (spt_completion - spt_job.due_date).max(0.0) * spt_job.weight;

        // Reward: positive if chosen heuristic does at least as well as SPT
        let reward = spt_tardiness - job_tardiness; // >0 means we beat SPT

        // Weight update: increase if reward > 0, decrease otherwise
        if reward > 0.0 {
            weights[h_idx] = (weights[h_idx] + learning_rate).min(1.0);
        } else {
            weights[h_idx] = (weights[h_idx] - learning_rate).max(0.01);
        }

        // Normalise
        let sum: f64 = weights.iter().sum();
        for w in weights.iter_mut() {
            *w /= sum;
        }

        clock = completion;
        twt += job_tardiness;
        if job_tardiness > 0.0 {
            tardy += 1;
        }

        remaining.retain(|j| j.id != chosen.id);
    }

    (twt, tardy, usage)
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== Hyper-heuristic Job Scheduler Benchmark (Tier 3) ===");
    println!();
    println!("30-job dynamic scheduling instance (mixed arrival times)");
    println!();

    let jobs = build_jobs();

    let (spt_twt, spt_tardy) = run_fixed(&jobs, Heuristic::Spt);
    let (edd_twt, edd_tardy) = run_fixed(&jobs, Heuristic::Edd);
    let (fifo_twt, fifo_tardy) = run_fixed(&jobs, Heuristic::Fifo);

    let learning_rate = 0.05;
    let (hh_twt, hh_tardy, usage) = run_hyper(&jobs, learning_rate);

    let total_dispatches: usize = usage.iter().sum();
    let pct = |n: usize| -> f64 {
        if total_dispatches == 0 {
            0.0
        } else {
            100.0 * n as f64 / total_dispatches as f64
        }
    };

    println!(
        "  Fixed SPT:    total tardiness = {:.1}  | tardy jobs = {}",
        spt_twt, spt_tardy
    );
    println!(
        "  Fixed EDD:    total tardiness = {:.1}  | tardy jobs = {}",
        edd_twt, edd_tardy
    );
    println!(
        "  Fixed FIFO:   total tardiness = {:.1}  | tardy jobs = {}",
        fifo_twt, fifo_tardy
    );
    println!(
        "  Hyper-HH:     total tardiness = {:.1}  | tardy jobs = {}",
        hh_twt, hh_tardy
    );
    println!(
        "  Heuristic usage: SPT={:.0}% LPT={:.0}% EDD={:.0}% FIFO={:.0}% CR={:.0}%",
        pct(usage[0]),
        pct(usage[1]),
        pct(usage[2]),
        pct(usage[3]),
        pct(usage[4]),
    );
    println!();
    println!("Tier 3 boundary: the hyper-heuristic selects WHICH heuristic to apply");
    println!("at each step — optimising the selection strategy, not the solution");
    println!("directly. Fixed architecture: SARSA-style weight table, roulette selection.");
    println!();

    // ── loom compile check ────────────────────────────────────────────────────
    let loom_src = include_str!("../../examples/tier3/hyper_heuristic_scheduler.loom");
    match loom::compile(loom_src) {
        Ok(_) => println!("[loom compile] examples/tier3/hyper_heuristic_scheduler.loom → OK"),
        Err(e) => {
            let msgs: Vec<String> = e.iter().map(|err| format!("{}", err)).collect();
            println!(
                "[loom compile] examples/tier3/hyper_heuristic_scheduler.loom → ERROR: {}",
                msgs.join("; ")
            );
        }
    }
}
