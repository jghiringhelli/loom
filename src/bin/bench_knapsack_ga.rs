//! Tier 2 benchmark: Genetic Algorithm for 0/1 Knapsack.
//!
//! Benchmark instance: 20 items, capacity = 30.
//! Compares GA result to DP-optimal and prints a structured report.

// ── LCG pseudo-random number generator ───────────────────────────────────────

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

    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_f64() * n as f64) as usize
    }

    fn next_bool(&mut self) -> bool {
        self.next_f64() < 0.5
    }
}

// ── Knapsack data ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Item {
    weight: usize,
    value: usize,
}

// ── Fitness / repair ──────────────────────────────────────────────────────────

fn total_weight(bits: &[bool], items: &[Item]) -> usize {
    bits.iter()
        .enumerate()
        .filter(|(_, &b)| b)
        .map(|(i, _)| items[i].weight)
        .sum()
}

fn total_value(bits: &[bool], items: &[Item]) -> usize {
    bits.iter()
        .enumerate()
        .filter(|(_, &b)| b)
        .map(|(i, _)| items[i].value)
        .sum()
}

fn fitness(bits: &[bool], items: &[Item], capacity: usize) -> i64 {
    let w = total_weight(bits, items);
    let v = total_value(bits, items) as i64;
    if w <= capacity {
        v
    } else {
        let excess = (w - capacity) as i64;
        v - excess * 10 // penalty proportional to excess weight
    }
}

/// Greedily drop items (heaviest first) until feasible.
fn repair(bits: &mut Vec<bool>, items: &[Item], capacity: usize) {
    while total_weight(bits, items) > capacity {
        // find selected item with worst value/weight ratio
        let worst = bits
            .iter()
            .enumerate()
            .filter(|(_, &b)| b)
            .min_by(|&(a, _), &(b_idx, _)| {
                let ra = items[a].value as f64 / items[a].weight as f64;
                let rb = items[b_idx].value as f64 / items[b_idx].weight as f64;
                ra.partial_cmp(&rb).unwrap()
            })
            .map(|(i, _)| i);
        if let Some(i) = worst {
            bits[i] = false;
        } else {
            break;
        }
    }
}

// ── Tournament selection ──────────────────────────────────────────────────────

fn tournament_select<'a>(
    population: &'a [Vec<bool>],
    fitnesses: &[i64],
    k: usize,
    rng: &mut Lcg,
) -> &'a Vec<bool> {
    let n = population.len();
    let mut best_idx = rng.next_usize(n);
    for _ in 1..k {
        let idx = rng.next_usize(n);
        if fitnesses[idx] > fitnesses[best_idx] {
            best_idx = idx;
        }
    }
    &population[best_idx]
}

// ── Single-point crossover ────────────────────────────────────────────────────

fn crossover(a: &[bool], b: &[bool], rng: &mut Lcg) -> (Vec<bool>, Vec<bool>) {
    let n = a.len();
    let point = 1 + rng.next_usize(n - 1); // crossover point in [1, n-1]
    let mut child1 = a.to_vec();
    let mut child2 = b.to_vec();
    child1[point..].clone_from_slice(&b[point..]);
    child2[point..].clone_from_slice(&a[point..]);
    (child1, child2)
}

// ── Bit-flip mutation ─────────────────────────────────────────────────────────

fn mutate(bits: &mut Vec<bool>, mutation_rate: f64, rng: &mut Lcg) {
    for b in bits.iter_mut() {
        if rng.next_f64() < mutation_rate {
            *b = !*b;
        }
    }
}

// ── DP optimal ────────────────────────────────────────────────────────────────

fn dp_optimal(items: &[Item], capacity: usize) -> usize {
    let n = items.len();
    // dp[w] = max value with weight budget w
    let mut dp = vec![0usize; capacity + 1];
    for item in items {
        for w in (item.weight..=capacity).rev() {
            dp[w] = dp[w].max(dp[w - item.weight] + item.value);
        }
    }
    dp[capacity]
}

// ── Genetic Algorithm ─────────────────────────────────────────────────────────

fn genetic_algorithm(
    items: &[Item],
    capacity: usize,
    pop_size: usize,
    generations: usize,
    mutation_rate: f64,
    crossover_rate: f64,
    rng: &mut Lcg,
) -> (Vec<bool>, usize) {
    let n = items.len();

    // Random initial population.
    let mut population: Vec<Vec<bool>> = (0..pop_size)
        .map(|_| (0..n).map(|_| rng.next_bool()).collect())
        .collect();

    // Repair all individuals.
    for ind in population.iter_mut() {
        repair(ind, items, capacity);
    }

    let mut best_bits: Vec<bool> = population[0].clone();
    let mut best_val = total_value(&best_bits, items);

    for _gen in 0..generations {
        // Evaluate fitness.
        let fitnesses: Vec<i64> = population
            .iter()
            .map(|bits| fitness(bits, items, capacity))
            .collect();

        // Identify top-2 for elitism.
        let mut ranked: Vec<usize> = (0..pop_size).collect();
        ranked.sort_by(|&a, &b| fitnesses[b].cmp(&fitnesses[a]));
        let elite: Vec<Vec<bool>> = ranked[..2].iter().map(|&i| population[i].clone()).collect();

        // Build next generation.
        let mut next_gen: Vec<Vec<bool>> = elite.clone();

        while next_gen.len() < pop_size {
            let parent_a = tournament_select(&population, &fitnesses, 3, rng).clone();
            let parent_b = tournament_select(&population, &fitnesses, 3, rng).clone();

            let (mut child_a, mut child_b) = if rng.next_f64() < crossover_rate {
                crossover(&parent_a, &parent_b, rng)
            } else {
                (parent_a.clone(), parent_b.clone())
            };

            mutate(&mut child_a, mutation_rate, rng);
            mutate(&mut child_b, mutation_rate, rng);
            repair(&mut child_a, items, capacity);
            repair(&mut child_b, items, capacity);

            next_gen.push(child_a);
            if next_gen.len() < pop_size {
                next_gen.push(child_b);
            }
        }

        population = next_gen;

        // Track best.
        for ind in &population {
            let v = total_value(ind, items);
            if v > best_val && total_weight(ind, items) <= capacity {
                best_val = v;
                best_bits = ind.clone();
            }
        }
    }

    (best_bits, best_val)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let items: Vec<Item> = vec![
        Item {
            weight: 2,
            value: 5,
        },
        Item {
            weight: 3,
            value: 4,
        },
        Item {
            weight: 6,
            value: 3,
        },
        Item {
            weight: 7,
            value: 7,
        },
        Item {
            weight: 5,
            value: 2,
        },
        Item {
            weight: 9,
            value: 9,
        },
        Item {
            weight: 4,
            value: 6,
        },
        Item {
            weight: 1,
            value: 1,
        },
        Item {
            weight: 8,
            value: 8,
        },
        Item {
            weight: 3,
            value: 3,
        },
        Item {
            weight: 2,
            value: 4,
        },
        Item {
            weight: 6,
            value: 7,
        },
        Item {
            weight: 5,
            value: 5,
        },
        Item {
            weight: 4,
            value: 3,
        },
        Item {
            weight: 7,
            value: 9,
        },
        Item {
            weight: 3,
            value: 2,
        },
        Item {
            weight: 8,
            value: 6,
        },
        Item {
            weight: 9,
            value: 8,
        },
        Item {
            weight: 1,
            value: 4,
        },
        Item {
            weight: 6,
            value: 5,
        },
    ];
    let capacity = 30;

    // DP optimal (ground truth).
    let dp_opt = dp_optimal(&items, capacity);

    // GA.
    let mut rng = Lcg::new(42);
    let (ga_bits, ga_val) = genetic_algorithm(
        &items, capacity, 50,   // population
        200,  // generations
        0.02, // mutation rate
        0.8,  // crossover rate
        &mut rng,
    );
    let ga_weight = total_weight(&ga_bits, &items);
    let gap_pct = (dp_opt - ga_val) as f64 / dp_opt as f64 * 100.0;

    println!("=== Knapsack Genetic Algorithm Benchmark (Tier 2 Meta-heuristic) ===");
    println!();
    println!("Instance: 20 items, capacity = {}", capacity);
    println!("  DP optimal value:       {}", dp_opt);
    println!(
        "  GA best value:          {} (weight used: {}/{})",
        ga_val, ga_weight, capacity
    );
    println!("  Optimality gap:         {:.1}%", gap_pct);
    println!();
    println!("GA parameters: population=50, generations=200,");
    println!("  mutation_rate=0.02, crossover_rate=0.8, elitism=2");
    println!();
    println!("Tier 2 boundary: GA uses a fixed representation (bitstring),");
    println!("fixed operators (tournament selection, single-point crossover,");
    println!("bit-flip mutation) and deterministic elitism. No learning.");
    println!();

    // Loom compile check.
    let src = include_str!("../../examples/tier2/knapsack_ga.loom");
    match loom::compile(src) {
        Ok(_) => println!("[loom compile] examples/tier2/knapsack_ga.loom \u{2192} OK"),
        Err(es) => {
            println!(
                "[loom compile] examples/tier2/knapsack_ga.loom \u{2192} {} error(s)",
                es.len()
            );
            for e in &es {
                eprintln!("  {}", e);
            }
        }
    }
}
