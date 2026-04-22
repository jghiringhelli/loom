//! Tier 2 benchmark: Simulated Annealing with 2-opt for TSP.
//!
//! Runs on a fixed 20-city synthetic instance and prints structured results
//! comparing nearest-neighbour (Tier 1) to SA (Tier 2).

// ── LCG pseudo-random number generator ───────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance one step and return a float in [0, 1).
    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Use top 53 bits for mantissa.
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Return a usize in [0, n).
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_f64() * n as f64) as usize
    }
}

// ── TSP distance helpers ──────────────────────────────────────────────────────

type City = (f64, f64);

fn euclidean(a: City, b: City) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

fn tour_length(tour: &[usize], cities: &[City]) -> f64 {
    let n = tour.len();
    (0..n)
        .map(|i| euclidean(cities[tour[i]], cities[tour[(i + 1) % n]]))
        .sum()
}

// ── Nearest-neighbour heuristic (Tier 1 baseline) ────────────────────────────

fn nearest_neighbour(cities: &[City]) -> Vec<usize> {
    let n = cities.len();
    let mut visited = vec![false; n];
    let mut tour = Vec::with_capacity(n);
    let mut current = 0;
    visited[current] = true;
    tour.push(current);

    for _ in 1..n {
        let next = (0..n)
            .filter(|&j| !visited[j])
            .min_by(|&a, &b| {
                euclidean(cities[current], cities[a])
                    .partial_cmp(&euclidean(cities[current], cities[b]))
                    .unwrap()
            })
            .unwrap();
        visited[next] = true;
        tour.push(next);
        current = next;
    }
    tour
}

// ── 2-opt swap ────────────────────────────────────────────────────────────────

/// Reverse the segment tour[i+1..=j] in-place (2-opt move).
fn two_opt_swap(tour: &mut Vec<usize>, i: usize, j: usize) {
    let mut lo = i + 1;
    let mut hi = j;
    while lo < hi {
        tour.swap(lo, hi);
        lo += 1;
        hi -= 1;
    }
}

/// Compute the change in tour length when the 2-opt move (i, j) is applied,
/// without actually performing the move.
fn two_opt_delta(tour: &[usize], cities: &[City], i: usize, j: usize) -> f64 {
    let n = tour.len();
    let a = tour[i];
    let b = tour[(i + 1) % n];
    let c = tour[j];
    let d = tour[(j + 1) % n];
    let before = euclidean(cities[a], cities[b]) + euclidean(cities[c], cities[d]);
    let after = euclidean(cities[a], cities[c]) + euclidean(cities[b], cities[d]);
    after - before
}

// ── Simulated Annealing ───────────────────────────────────────────────────────

fn simulated_annealing(
    cities: &[City],
    t0: f64,
    alpha: f64,
    t_min: f64,
    iters_per_temp: usize,
    rng: &mut Lcg,
) -> (Vec<usize>, usize, usize) {
    let n = cities.len();
    let mut current = nearest_neighbour(cities);
    let mut best = current.clone();
    let mut best_len = tour_length(&best, cities);

    let mut total_iters: usize = 0;
    let mut improvements: usize = 0;
    let mut temp = t0;

    while temp > t_min {
        for _ in 0..iters_per_temp {
            // Pick two distinct indices i < j.
            let i = rng.next_usize(n - 1);
            let j = i + 1 + rng.next_usize(n - 1 - i);

            let delta = two_opt_delta(&current, cities, i, j);
            let accept = if delta < 0.0 {
                true
            } else {
                rng.next_f64() < (-delta / temp).exp()
            };

            if accept {
                two_opt_swap(&mut current, i, j);
                total_iters += 1;
                let cur_len = tour_length(&current, cities);
                if cur_len < best_len {
                    best_len = cur_len;
                    best = current.clone();
                    improvements += 1;
                }
            } else {
                total_iters += 1;
            }
        }
        temp *= alpha;
    }

    (best, total_iters, improvements)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cities: &[City] = &[
        (0.0, 0.0),
        (3.0, 1.0),
        (6.0, 2.0),
        (9.0, 0.0),
        (8.0, 4.0),
        (6.0, 6.0),
        (9.0, 8.0),
        (6.0, 9.0),
        (3.0, 8.0),
        (0.0, 7.0),
        (2.0, 5.0),
        (1.0, 3.0),
        (4.0, 4.0),
        (7.0, 3.0),
        (5.0, 1.0),
        (2.0, 7.0),
        (8.0, 6.0),
        (4.0, 2.0),
        (7.0, 5.0),
        (1.0, 6.0),
    ];

    // Nearest-neighbour baseline.
    let nn_tour = nearest_neighbour(cities);
    let nn_len = tour_length(&nn_tour, cities);

    // Simulated Annealing.
    let mut rng = Lcg::new(42);
    let (sa_tour, total_iters, improvements) =
        simulated_annealing(cities, 1000.0, 0.995, 0.01, 1000, &mut rng);
    let sa_len = tour_length(&sa_tour, cities);

    let improvement_pct = (nn_len - sa_len) / nn_len * 100.0;

    println!("=== TSP Simulated Annealing Benchmark (Tier 2 Meta-heuristic) ===");
    println!();
    println!("Instance: 20-city synthetic (integer coordinates)");
    println!("  Nearest-neighbour (Tier 1 baseline): ~{:.1} km", nn_len);
    println!(
        "  Simulated Annealing (T\u{2080}=1000, \u{03b1}=0.995): ~{:.1} km",
        sa_len
    );
    println!("  Improvement over NN: {:.1}%", improvement_pct);
    println!(
        "  Iterations: {} | Improvements: {}",
        total_iters, improvements
    );
    println!();
    println!("Tier 2 boundary: SA explores an adaptive neighbourhood (temperature-");
    println!("controlled acceptance of uphill moves). The architecture is fixed:");
    println!("2-opt operator, geometric cooling, exp(-\u{0394}/T) acceptance.");
    println!();

    // Loom compile check.
    let src = include_str!("../../examples/tier2/tsp_simulated_annealing.loom");
    match loom::compile(src) {
        Ok(_) => println!("[loom compile] examples/tier2/tsp_simulated_annealing.loom \u{2192} OK"),
        Err(es) => {
            println!(
                "[loom compile] examples/tier2/tsp_simulated_annealing.loom \u{2192} {} error(s)",
                es.len()
            );
            for e in &es {
                eprintln!("  {}", e);
            }
        }
    }
}
