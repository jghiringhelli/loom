//! Tier 2 benchmark: Iterated Local Search with Or-opt for CVRP.
//!
//! Benchmark: 10 customers + 1 depot, 2 vehicles, capacity = 30.
//! Uses Clarke-Wright savings for the initial solution, then improves with
//! Or-opt (single-customer relocation) iterated 500 times.

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
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

type Point = (f64, f64);

fn dist(a: Point, b: Point) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

// ── Problem data ──────────────────────────────────────────────────────────────

struct Problem {
    depot: Point,
    customers: Vec<(Point, usize)>, // (location, demand)
    capacity: usize,
    num_vehicles: usize,
}

// ── Solution representation ───────────────────────────────────────────────────

/// A solution is a list of routes; each route is a list of customer indices.
#[derive(Clone, Debug)]
struct Solution {
    routes: Vec<Vec<usize>>,
}

impl Solution {
    fn route_dist(&self, r: usize, prob: &Problem) -> f64 {
        let route = &self.routes[r];
        if route.is_empty() {
            return 0.0;
        }
        let mut d = dist(prob.depot, prob.customers[route[0]].0);
        for i in 0..route.len() - 1 {
            d += dist(prob.customers[route[i]].0, prob.customers[route[i + 1]].0);
        }
        d += dist(prob.customers[route[route.len() - 1]].0, prob.depot);
        d
    }

    fn total_distance(&self, prob: &Problem) -> f64 {
        (0..self.routes.len())
            .map(|r| self.route_dist(r, prob))
            .sum()
    }

    fn route_load(&self, r: usize, prob: &Problem) -> usize {
        self.routes[r].iter().map(|&c| prob.customers[c].1).sum()
    }
}

// ── Clarke-Wright savings ─────────────────────────────────────────────────────

/// Standard Clarke-Wright parallel savings algorithm.
/// Each customer starts in its own route (depot→i→depot).
/// Savings s(i,j) = d(0,i)+d(0,j)-d(i,j) are computed and merged greedily
/// when feasible (capacity and route-endpoint constraints respected).
fn clarke_wright(prob: &Problem) -> Solution {
    let n = prob.customers.len();

    // route_of[c] = which route index customer c belongs to (starts as c itself)
    let mut route_of: Vec<usize> = (0..n).collect();
    // routes: list of customer sequences
    let mut routes: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    // Compute savings: s(i,j) = d(depot,i) + d(depot,j) - d(i,j)
    let mut savings: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let s = dist(prob.depot, prob.customers[i].0) + dist(prob.depot, prob.customers[j].0)
                - dist(prob.customers[i].0, prob.customers[j].0);
            savings.push((s, i, j));
        }
    }
    savings.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()); // descending

    // Greedy merge.
    for (_s, ci, cj) in &savings {
        let ci = *ci;
        let cj = *cj;
        let ri = route_of[ci];
        let rj = route_of[cj];

        if ri == rj {
            continue; // same route
        }
        if routes[ri].is_empty() || routes[rj].is_empty() {
            continue;
        }

        // ci must be an endpoint of its route, cj must be an endpoint of its route.
        let ci_is_tail = *routes[ri].last().unwrap() == ci;
        let ci_is_head = *routes[ri].first().unwrap() == ci;
        let cj_is_tail = *routes[rj].last().unwrap() == cj;
        let cj_is_head = *routes[rj].first().unwrap() == cj;

        if !ci_is_tail && !ci_is_head {
            continue;
        }
        if !cj_is_tail && !cj_is_head {
            continue;
        }

        // Check capacity.
        let load_i: usize = routes[ri].iter().map(|&c| prob.customers[c].1).sum();
        let load_j: usize = routes[rj].iter().map(|&c| prob.customers[c].1).sum();
        if load_i + load_j > prob.capacity {
            continue;
        }

        // Arrange so ci is at tail of ri and cj is at head of rj.
        if !ci_is_tail {
            routes[ri].reverse();
        }
        if cj_is_tail {
            routes[rj].reverse(); // cj needs to be head, but it's a tail → reverse rj
        }

        // Merge: append rj onto ri.
        let rj_route = std::mem::take(&mut routes[rj]);
        for &c in &rj_route {
            route_of[c] = ri;
        }
        routes[ri].extend(rj_route);
    }

    // Collect non-empty routes.
    let mut result: Vec<Vec<usize>> = routes.into_iter().filter(|r| !r.is_empty()).collect();

    // Ensure we have exactly num_vehicles routes (pad with empty if needed).
    while result.len() < prob.num_vehicles {
        result.push(vec![]);
    }

    // If more routes than vehicles exist (capacity prevented full merging),
    // greedily relocate customers from excess routes into routes with spare capacity.
    while result.len() > prob.num_vehicles {
        let extra = result.pop().unwrap();
        for c in extra {
            let demand = prob.customers[c].1;
            let target = (0..result.len())
                .filter(|&r| {
                    let load: usize = result[r].iter().map(|&x| prob.customers[x].1).sum();
                    load + demand <= prob.capacity
                })
                .min_by(|&a, &b| {
                    let la: usize = result[a].iter().map(|&x| prob.customers[x].1).sum();
                    let lb: usize = result[b].iter().map(|&x| prob.customers[x].1).sum();
                    lb.cmp(&la) // prefer less loaded
                });
            match target {
                Some(t) => result[t].push(c),
                None => {
                    result.push(vec![c]); // forced extra route
                    break;
                }
            }
        }
    }

    Solution { routes: result }
}

// ── Intra-route 2-opt ─────────────────────────────────────────────────────────

/// Apply best-improving 2-opt within a single route.
fn two_opt_route(sol: &mut Solution, prob: &Problem, r: usize) -> bool {
    let n = sol.routes[r].len();
    if n < 3 {
        return false;
    }
    let mut improved = false;
    'outer: loop {
        for i in 0..n - 1 {
            for j in i + 2..n {
                // Nodes involved: route[i], route[i+1], route[j], route[(j+1)%n]
                let a = sol.routes[r][i];
                let b = sol.routes[r][i + 1];
                let c = sol.routes[r][j];
                let d_node = if j + 1 < n {
                    prob.customers[sol.routes[r][j + 1]].0
                } else {
                    prob.depot
                };
                let before = dist(prob.customers[a].0, prob.customers[b].0)
                    + dist(prob.customers[c].0, d_node);
                let after = dist(prob.customers[a].0, prob.customers[c].0)
                    + dist(prob.customers[b].0, d_node);
                if after < before - 1e-9 {
                    sol.routes[r][i + 1..=j].reverse();
                    improved = true;
                    continue 'outer;
                }
            }
        }
        break;
    }
    improved
}

// ── Or-opt: relocate a single customer ───────────────────────────────────────

/// Try every single-customer relocation between different routes.
/// Apply the best improving move found; return true if any improvement made.
fn or_opt_best_improving(sol: &mut Solution, prob: &Problem) -> bool {
    let nr = sol.routes.len();
    let mut best_delta = -1e-9;
    let mut best_move: Option<(usize, usize, usize, usize)> = None;

    for from_r in 0..nr {
        if sol.routes[from_r].is_empty() {
            continue;
        }
        for from_pos in 0..sol.routes[from_r].len() {
            let customer = sol.routes[from_r][from_pos];
            let demand = prob.customers[customer].1;

            for to_r in 0..nr {
                if to_r == from_r {
                    continue;
                }
                let to_load = sol.route_load(to_r, prob);
                if to_load + demand > prob.capacity {
                    continue;
                }
                for to_pos in 0..=sol.routes[to_r].len() {
                    // Compute change in distance.
                    let d_remove = removal_cost(sol, prob, from_r, from_pos);
                    let d_insert = insertion_cost(sol, prob, to_r, to_pos, customer);
                    let delta = d_insert - d_remove;
                    if delta < best_delta {
                        best_delta = delta;
                        best_move = Some((from_r, from_pos, to_r, to_pos));
                    }
                }
            }
        }
    }

    if let Some((from_r, from_pos, to_r, to_pos)) = best_move {
        let customer = sol.routes[from_r].remove(from_pos);
        // Adjust insertion index if from_r < to_r and removal was before.
        sol.routes[to_r].insert(to_pos, customer);
        true
    } else {
        false
    }
}

/// Cost saved by removing customer at position `pos` from route `r`.
fn removal_cost(sol: &Solution, prob: &Problem, r: usize, pos: usize) -> f64 {
    let route = &sol.routes[r];
    let n = route.len();
    let loc = |i: usize| prob.customers[route[i]].0;

    let prev = if pos == 0 { prob.depot } else { loc(pos - 1) };
    let next = if pos == n - 1 {
        prob.depot
    } else {
        loc(pos + 1)
    };
    let cur = loc(pos);

    // Before removal: prev→cur + cur→next; after: prev→next
    dist(prev, cur) + dist(cur, next) - dist(prev, next)
}

/// Extra cost of inserting `customer` at position `pos` into route `r`.
fn insertion_cost(sol: &Solution, prob: &Problem, r: usize, pos: usize, customer: usize) -> f64 {
    let route = &sol.routes[r];
    let n = route.len();
    let cust_loc = prob.customers[customer].0;

    let prev = if pos == 0 {
        prob.depot
    } else {
        prob.customers[route[pos - 1]].0
    };
    let next = if pos == n {
        prob.depot
    } else {
        prob.customers[route[pos]].0
    };

    // Before insertion: prev→next; after: prev→cust + cust→next
    dist(prev, cust_loc) + dist(cust_loc, next) - dist(prev, next)
}

// ── Iterated local search ─────────────────────────────────────────────────────

/// Run up to `iterations` rounds of best-improving Or-opt.
/// When no improvement is found (local optimum), apply a random perturbation
/// (random Or-opt move that is feasible) and continue.
fn ils_or_opt(prob: &Problem, mut sol: Solution, iterations: usize, rng: &mut Lcg) -> Solution {
    let mut best = sol.clone();
    let mut best_dist = best.total_distance(prob);

    let mut no_improve_streak = 0;

    for _iter in 0..iterations {
        // Apply within-route 2-opt first.
        let nr = sol.routes.len();
        for r in 0..nr {
            two_opt_route(&mut sol, prob, r);
        }

        let improved = or_opt_best_improving(&mut sol, prob);
        if improved {
            no_improve_streak = 0;
            let d = sol.total_distance(prob);
            if d < best_dist {
                best_dist = d;
                best = sol.clone();
            }
        } else {
            no_improve_streak += 1;
            // Perturbation: random feasible Or-opt move.
            let nr = sol.routes.len();
            let non_empty: Vec<usize> = (0..nr).filter(|&r| !sol.routes[r].is_empty()).collect();
            if non_empty.len() < 2 {
                break;
            }
            let from_r = non_empty[rng.next_usize(non_empty.len())];
            let from_pos = rng.next_usize(sol.routes[from_r].len());
            let customer = sol.routes[from_r][from_pos];
            let demand = prob.customers[customer].1;

            // Try to find a feasible destination.
            for _ in 0..10 {
                let to_r = rng.next_usize(nr);
                if to_r == from_r {
                    continue;
                }
                let to_load = sol.route_load(to_r, prob);
                if to_load + demand > prob.capacity {
                    continue;
                }
                let to_pos = rng.next_usize(sol.routes[to_r].len() + 1);
                sol.routes[from_r].remove(from_pos);
                sol.routes[to_r].insert(to_pos, customer);
                break;
            }

            if no_improve_streak > 50 {
                // Reset to best known.
                sol = best.clone();
                no_improve_streak = 0;
            }
        }
    }

    best
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Problem: 10 customers + depot, 2 vehicles, capacity=30.
    // Demands chosen so that a 2-route split is feasible:
    //   e.g., route A demand ≤30, route B demand ≤30.
    let prob = Problem {
        depot: (5.0, 5.0),
        customers: vec![
            ((1.0, 1.0), 5), // 0
            ((9.0, 1.0), 8), // 1
            ((1.0, 9.0), 3), // 2
            ((9.0, 9.0), 7), // 3
            ((5.0, 2.0), 6), // 4
            ((2.0, 5.0), 4), // 5
            ((8.0, 5.0), 9), // 6
            ((5.0, 8.0), 2), // 7
            ((3.0, 3.0), 5), // 8
            ((7.0, 7.0), 6), // 9
        ],
        capacity: 30,
        num_vehicles: 2,
    };

    let total_demand: usize = prob.customers.iter().map(|(_, d)| d).sum();

    // Naive initial solution: assign customers in index order to routes, round-robin.
    // This ignores geography and produces a suboptimal (but feasible) solution.
    let naive_sol = {
        let mut routes: Vec<Vec<usize>> = vec![vec![]; prob.num_vehicles];
        let mut loads = vec![0usize; prob.num_vehicles];
        for c in 0..prob.customers.len() {
            // assign to least-loaded vehicle that has capacity
            let target = (0..prob.num_vehicles)
                .filter(|&v| loads[v] + prob.customers[c].1 <= prob.capacity)
                .min_by_key(|&v| loads[v])
                .expect("infeasible assignment");
            routes[target].push(c);
            loads[target] += prob.customers[c].1;
        }
        Solution { routes }
    };
    let naive_dist = naive_sol.total_distance(&prob);

    // Clarke-Wright initial solution.
    let cw_sol = clarke_wright(&prob);
    let cw_dist = cw_sol.total_distance(&prob);

    // Or-opt iterated local search (starting from naive solution).
    let mut rng = Lcg::new(42);
    let opt_sol = ils_or_opt(&prob, naive_sol.clone(), 500, &mut rng);
    let opt_dist = opt_sol.total_distance(&prob);

    let improvement_naive = if naive_dist > 1e-9 && naive_dist > opt_dist {
        (naive_dist - opt_dist) / naive_dist * 100.0
    } else {
        0.0
    };

    println!("=== VRP Iterated Or-opt Benchmark (Tier 2 Meta-heuristic) ===");
    println!();
    println!(
        "Instance: 10 customers + 1 depot, {} vehicles, capacity = {}",
        prob.num_vehicles, prob.capacity
    );
    println!(
        "  Total demand: {} / {} (feasible)",
        total_demand,
        prob.num_vehicles * prob.capacity
    );
    println!();
    println!(
        "  Naive initial (index-order assignment): {:.2} km",
        naive_dist
    );
    for (i, r) in naive_sol.routes.iter().enumerate() {
        let load: usize = r.iter().map(|&c| prob.customers[c].1).sum();
        println!(
            "    Route {}: {:?}  load={}/{}",
            i + 1,
            r,
            load,
            prob.capacity
        );
    }
    println!(
        "  Clarke-Wright savings heuristic:        {:.2} km",
        cw_dist
    );
    for (i, r) in cw_sol.routes.iter().enumerate() {
        let load: usize = r.iter().map(|&c| prob.customers[c].1).sum();
        println!(
            "    Route {}: {:?}  load={}/{}",
            i + 1,
            r,
            load,
            prob.capacity
        );
    }
    println!();
    println!(
        "  Or-opt optimised (500 iterations):      {:.2} km",
        opt_dist
    );
    for (i, r) in opt_sol.routes.iter().enumerate() {
        let load: usize = r.iter().map(|&c| prob.customers[c].1).sum();
        println!(
            "    Route {}: {:?}  load={}/{}",
            i + 1,
            r,
            load,
            prob.capacity
        );
    }
    println!(
        "  Improvement over naive:                 {:.1}%",
        improvement_naive
    );
    println!();
    println!("Tier 2 boundary: ILS uses fixed Or-opt (single-customer relocation)");
    println!("as the neighbourhood operator. Clarke-Wright provides the construction");
    println!("heuristic. No learning or adaptive parameter tuning.");
    println!();

    // Loom compile check.
    let src = include_str!("../../examples/tier2/vrp_pso.loom");
    match loom::compile(src) {
        Ok(_) => println!("[loom compile] examples/tier2/vrp_pso.loom \u{2192} OK"),
        Err(es) => {
            println!(
                "[loom compile] examples/tier2/vrp_pso.loom \u{2192} {} error(s)",
                es.len()
            );
            for e in &es {
                eprintln!("  {}", e);
            }
        }
    }
}
