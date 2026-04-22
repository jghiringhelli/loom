//! bench_neural_tsp — Attention-based TSP solver with learned temperature (Tier 4).
//!
//! Implements a softmax nearest-city policy with a learned temperature parameter β
//! trained via REINFORCE across 50 random 10-city instances. Evaluates on 20 test
//! instances and compares to nearest-neighbour and untrained β=1.

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

// ── City and tour utilities ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct City {
    x: f64,
    y: f64,
}

fn dist(a: &City, b: &City) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn tour_length(cities: &[City], perm: &[usize]) -> f64 {
    let n = perm.len();
    let mut total = 0.0;
    for i in 0..n {
        total += dist(&cities[perm[i]], &cities[perm[(i + 1) % n]]);
    }
    total
}

/// Generate a random 10-city instance in the unit square.
fn random_instance(rng: &mut Lcg) -> Vec<City> {
    (0..10)
        .map(|_| City {
            x: rng.next_f64(),
            y: rng.next_f64(),
        })
        .collect()
}

// ── Softmax helper ────────────────────────────────────────────────────────────

/// Numerically-stable softmax over a slice.
fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&l| (l - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Sample an index from a discrete distribution (probabilities must sum to ≈1).
fn sample_categorical(probs: &[f64], rng: &mut Lcg) -> usize {
    let r = rng.next_f64();
    let mut cumsum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        cumsum += p;
        if r <= cumsum {
            return i;
        }
    }
    probs.len() - 1
}

// ── Attention model: softmax(-β·d) policy ────────────────────────────────────

/// One forward pass of the attention model.
/// Returns (permutation, log_prob_sum) where the tour is built greedily or sampled.
fn attention_rollout(
    cities: &[City],
    beta: f64,
    mut rng: Option<&mut Lcg>, // Some = sample, None = greedy
) -> (Vec<usize>, f64) {
    let n = cities.len();
    let mut visited = vec![false; n];
    let mut perm = Vec::with_capacity(n);
    let mut log_prob_sum = 0.0;

    // Start from city 0
    let mut current = 0;
    visited[0] = true;
    perm.push(0);

    for _step in 1..n {
        // Compute logits = -β · d(current, city_i) for unvisited
        let logits: Vec<f64> = (0..n)
            .map(|i| {
                if visited[i] {
                    f64::NEG_INFINITY
                } else {
                    -beta * dist(&cities[current], &cities[i])
                }
            })
            .collect();

        let probs = softmax(&logits);
        let log_probs: Vec<f64> = probs
            .iter()
            .map(|&p| if p > 1e-300 { p.ln() } else { -700.0 })
            .collect();

        let next = match rng {
            Some(ref mut r) => sample_categorical(&probs, *r),
            None => {
                // Greedy: pick max prob among unvisited
                (0..n)
                    .filter(|&i| !visited[i])
                    .max_by(|&a, &b| probs[a].partial_cmp(&probs[b]).unwrap())
                    .unwrap()
            }
        };

        log_prob_sum += log_probs[next];
        visited[next] = true;
        perm.push(next);
        current = next;
    }

    (perm, log_prob_sum)
}

// ── Nearest-neighbour heuristic ───────────────────────────────────────────────

fn nearest_neighbour(cities: &[City]) -> Vec<usize> {
    let n = cities.len();
    let mut visited = vec![false; n];
    let mut perm = Vec::with_capacity(n);
    let mut current = 0;
    visited[0] = true;
    perm.push(0);

    for _ in 1..n {
        let next = (0..n)
            .filter(|&i| !visited[i])
            .min_by(|&a, &b| {
                dist(&cities[current], &cities[a])
                    .partial_cmp(&dist(&cities[current], &cities[b]))
                    .unwrap()
            })
            .unwrap();
        visited[next] = true;
        perm.push(next);
        current = next;
    }

    perm
}

// ── REINFORCE training ────────────────────────────────────────────────────────

fn train_beta(n_episodes: usize, learning_rate: f64, train_seed: u64) -> f64 {
    let mut rng = Lcg::new(train_seed);
    let mut beta = 1.0f64;
    let mut baseline = 0.0f64; // running average tour length
    let baseline_alpha = 0.1; // EMA decay for baseline

    for ep in 0..n_episodes {
        let cities = random_instance(&mut rng);

        // Sample a tour using current beta
        // We need a second RNG for sampling — use a derived seed
        let mut sample_rng = Lcg::new(train_seed.wrapping_add(ep as u64 * 1000 + 7));
        let (perm, log_prob_sum) = attention_rollout(&cities, beta, Some(&mut sample_rng));
        let length = tour_length(&cities, &perm);

        // Update baseline
        if ep == 0 {
            baseline = length;
        } else {
            baseline = (1.0 - baseline_alpha) * baseline + baseline_alpha * length;
        }

        // REINFORCE gradient: ∇β J ≈ (length - baseline) × ∇β log π
        // ∇β log π = Σ_t ∂/∂β log softmax(-β·d)[action_t]
        // = Σ_t (-d_chosen_t + Σ_i p_i·d_i)  (softmax gradient w.r.t. scale)
        // We approximate: ∇β log π ≈ log_prob_sum derivative via finite diff
        // Simpler: use score function directly.
        // log π = Σ_t log P(a_t) where P uses β; ∂log P/∂β is the natural gradient.
        // For softmax(-β·d): ∂log P(a)/∂β = -d_a + Σ_j p_j d_j  (mean - chosen distance)
        // Compute this properly:
        let grad_log_pi = {
            let mut grad = 0.0f64;
            let mut visited = vec![false; cities.len()];
            let mut current = 0;
            visited[0] = true;
            for step in 1..perm.len() {
                let action = perm[step];
                let logits: Vec<f64> = (0..cities.len())
                    .map(|i| {
                        if visited[i] {
                            f64::NEG_INFINITY
                        } else {
                            -beta * dist(&cities[current], &cities[i])
                        }
                    })
                    .collect();
                let probs = softmax(&logits);

                // d_chosen
                let d_chosen = dist(&cities[current], &cities[action]);
                // E_p[d] = Σ_i p_i d_i  (for unvisited)
                let e_d: f64 = (0..cities.len())
                    .filter(|&i| !visited[i])
                    .map(|i| probs[i] * dist(&cities[current], &cities[i]))
                    .sum();

                grad += -d_chosen + e_d;
                visited[action] = true;
                current = action;
            }
            grad
        };

        // Policy gradient: minimise length → gradient ascent on -length
        // ∇β = -(length - baseline) × grad_log_pi
        // We want to minimise length, so:
        // β += lr × (-(length - baseline)) × grad_log_pi
        let _ = log_prob_sum; // used implicitly via grad_log_pi
        let advantage = length - baseline;
        beta -= learning_rate * advantage * grad_log_pi;

        // Keep beta positive (distances must be scaled positively for nearest-city logic)
        beta = beta.max(0.01).min(50.0);
    }

    beta
}

// ── Evaluation ────────────────────────────────────────────────────────────────

fn evaluate_model(beta: f64, n_test: usize, test_seed: u64) -> (f64, f64) {
    let mut rng = Lcg::new(test_seed);
    let mut lengths = Vec::with_capacity(n_test);
    for _ in 0..n_test {
        let cities = random_instance(&mut rng);
        let (perm, _) = attention_rollout(&cities, beta, None); // greedy at test time
        lengths.push(tour_length(&cities, &perm));
    }
    let mean = lengths.iter().sum::<f64>() / n_test as f64;
    let std = {
        let var = lengths.iter().map(|&l| (l - mean).powi(2)).sum::<f64>() / n_test as f64;
        var.sqrt()
    };
    (mean, std)
}

fn evaluate_nn(n_test: usize, test_seed: u64) -> (f64, f64) {
    let mut rng = Lcg::new(test_seed);
    let mut lengths = Vec::with_capacity(n_test);
    for _ in 0..n_test {
        let cities = random_instance(&mut rng);
        let perm = nearest_neighbour(&cities);
        lengths.push(tour_length(&cities, &perm));
    }
    let mean = lengths.iter().sum::<f64>() / n_test as f64;
    let std = {
        let var = lengths.iter().map(|&l| (l - mean).powi(2)).sum::<f64>() / n_test as f64;
        var.sqrt()
    };
    (mean, std)
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== Neural Combinatorial TSP Benchmark (Tier 4 Learning-based) ===");
    println!();
    println!("10-city TSP instances (random, unit square)");
    println!("Training: 50 episodes, REINFORCE, lr=0.01");
    println!();

    let n_train = 50;
    let n_test = 20;
    let learning_rate = 0.01;
    let train_seed = 12345u64;
    let test_seed = 99999u64;

    // Train β
    let learned_beta = train_beta(n_train, learning_rate, train_seed);

    // Evaluate three methods on the same test instances
    let (nn_mean, nn_std) = evaluate_nn(n_test, test_seed);
    let (untrained_mean, untrained_std) = evaluate_model(1.0, n_test, test_seed);
    let (trained_mean, trained_std) = evaluate_model(learned_beta, n_test, test_seed);

    let improvement_pct = 100.0 * (untrained_mean - trained_mean) / untrained_mean;

    println!(
        "  Nearest-neighbour (Tier 1):           avg length = {:.2} ± {:.2}",
        nn_mean, nn_std
    );
    println!(
        "  Attention model β=1.0 (untrained):    avg length = {:.2} ± {:.2}",
        untrained_mean, untrained_std
    );
    println!(
        "  Attention model β={:.2} (trained):  avg length = {:.2} ± {:.2}",
        learned_beta, trained_mean, trained_std
    );
    println!("  Improvement from learning: {:.1}%", improvement_pct);
    println!();
    println!("Tier 4 boundary: the parameter β is trained across instances — search cost");
    println!("is amortised via gradient updates. Each test instance requires only one");
    println!("forward pass (O(n²) attention), not full SA/GA search per instance.");
    println!();

    // ── loom compile check ────────────────────────────────────────────────────
    let loom_src = include_str!("../../examples/tier4/neural_combinatorial.loom");
    match loom::compile(loom_src) {
        Ok(_) => println!("[loom compile] examples/tier4/neural_combinatorial.loom → OK"),
        Err(e) => {
            let msgs: Vec<String> = e.iter().map(|err| format!("{}", err)).collect();
            println!(
                "[loom compile] examples/tier4/neural_combinatorial.loom → ERROR: {}",
                msgs.join("; ")
            );
        }
    }
}
