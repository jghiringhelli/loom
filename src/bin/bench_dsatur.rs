//! bench_dsatur — DSATUR graph coloring benchmark (Tier 1 Heuristic).
//!
//! Implements DSATUR (Brélaz 1979) and a simple greedy coloring algorithm,
//! then runs them on four canonical benchmark instances.
//! Also verifies that the corresponding .loom file compiles cleanly.

use loom;

// ── Graph representation ─────────────────────────────────────────────────────

#[derive(Clone)]
struct Graph {
    n: usize,
    adj: Vec<Vec<usize>>, // adjacency list
}

impl Graph {
    fn new(n: usize, edges: &[(usize, usize)]) -> Self {
        let mut adj = vec![vec![]; n];
        for &(u, v) in edges {
            adj[u].push(v);
            adj[v].push(u);
        }
        Graph { n, adj }
    }

    fn degree(&self, v: usize) -> usize {
        self.adj[v].len()
    }

    /// Number of distinct colors among already-colored neighbors.
    fn saturation(&self, v: usize, color: &[Option<usize>]) -> usize {
        let mut neighbor_colors: Vec<usize> =
            self.adj[v].iter().filter_map(|&u| color[u]).collect();
        neighbor_colors.sort_unstable();
        neighbor_colors.dedup();
        neighbor_colors.len()
    }

    /// Smallest color not used by any neighbor of v.
    fn smallest_valid_color(&self, v: usize, color: &[Option<usize>]) -> usize {
        let mut used = std::collections::HashSet::new();
        for &u in &self.adj[v] {
            if let Some(c) = color[u] {
                used.insert(c);
            }
        }
        (0..).find(|c| !used.contains(c)).unwrap()
    }
}

// ── DSATUR algorithm ─────────────────────────────────────────────────────────

fn color_dsatur(g: &Graph) -> (Vec<usize>, usize) {
    let n = g.n;
    let mut color: Vec<Option<usize>> = vec![None; n];
    let mut num_colors = 0usize;

    for _ in 0..n {
        // Select uncolored vertex with maximum saturation; break ties by degree.
        let v = (0..n)
            .filter(|&v| color[v].is_none())
            .max_by_key(|&v| (g.saturation(v, &color), g.degree(v)))
            .unwrap();

        let c = g.smallest_valid_color(v, &color);
        color[v] = Some(c);
        if c + 1 > num_colors {
            num_colors = c + 1;
        }
    }

    (color.into_iter().map(|c| c.unwrap()).collect(), num_colors)
}

// ── Greedy coloring (sequential order) ───────────────────────────────────────

fn color_greedy(g: &Graph) -> (Vec<usize>, usize) {
    let n = g.n;
    let mut color: Vec<Option<usize>> = vec![None; n];
    let mut num_colors = 0usize;

    for v in 0..n {
        let c = g.smallest_valid_color(v, &color);
        color[v] = Some(c);
        if c + 1 > num_colors {
            num_colors = c + 1;
        }
    }

    (color.into_iter().map(|c| c.unwrap()).collect(), num_colors)
}

// ── Validity check ────────────────────────────────────────────────────────────

fn is_valid_coloring(g: &Graph, coloring: &[usize]) -> bool {
    for u in 0..g.n {
        for &v in &g.adj[u] {
            if coloring[u] == coloring[v] {
                return false;
            }
        }
    }
    true
}

// ── Benchmark instances ───────────────────────────────────────────────────────

fn build_p6() -> Graph {
    // Path P6: 0-1-2-3-4-5 (bipartite, χ=2)
    Graph::new(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)])
}

fn build_petersen() -> Graph {
    // Petersen graph: outer 5-cycle + inner pentagram (χ=3)
    Graph::new(
        10,
        &[
            // outer cycle
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 0),
            // spokes
            (0, 5),
            (1, 6),
            (2, 7),
            (3, 8),
            (4, 9),
            // inner pentagram
            (5, 7),
            (7, 9),
            (9, 6),
            (6, 8),
            (8, 5),
        ],
    )
}

fn build_k5() -> Graph {
    // K5 complete graph (χ=5)
    let mut edges = vec![];
    for i in 0..5usize {
        for j in (i + 1)..5 {
            edges.push((i, j));
        }
    }
    Graph::new(5, &edges)
}

fn build_c5() -> Graph {
    // C5 odd cycle (χ=3)
    Graph::new(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)])
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== DSATUR Graph Coloring Benchmark (Tier 1 Heuristic) ===");
    println!();

    // P6 path
    {
        let g = build_p6();
        let edges = 5;
        let known_chi = 2;

        let (dsatur_col, dsatur_colors) = color_dsatur(&g);
        let (greedy_col, greedy_colors) = color_greedy(&g);

        assert!(
            is_valid_coloring(&g, &dsatur_col),
            "P6 DSATUR: invalid coloring"
        );
        assert!(
            is_valid_coloring(&g, &greedy_col),
            "P6 Greedy: invalid coloring"
        );

        println!("P6 path ({} vertices, {} edges)", g.n, edges);
        println!(
            "  DSATUR:  {} colors  | known χ = {}  {}",
            dsatur_colors,
            known_chi,
            if dsatur_colors == known_chi {
                "✓"
            } else {
                "✗"
            }
        );
        println!(
            "  Greedy:  {} colors  | known χ = {}  {}",
            greedy_colors,
            known_chi,
            if greedy_colors == known_chi {
                "✓"
            } else {
                "✗"
            }
        );
        println!();
    }

    // Petersen graph
    {
        let g = build_petersen();
        let edges = 15;
        let known_chi = 3;

        let (dsatur_col, dsatur_colors) = color_dsatur(&g);
        let (greedy_col, greedy_colors) = color_greedy(&g);

        assert!(
            is_valid_coloring(&g, &dsatur_col),
            "Petersen DSATUR: invalid coloring"
        );
        assert!(
            is_valid_coloring(&g, &greedy_col),
            "Petersen Greedy: invalid coloring"
        );

        println!("Petersen graph ({} vertices, {} edges)", g.n, edges);
        println!(
            "  DSATUR:  {} colors  | known χ = {}  {}",
            dsatur_colors,
            known_chi,
            if dsatur_colors == known_chi {
                "✓"
            } else {
                "✗"
            }
        );
        if greedy_colors == known_chi {
            println!(
                "  Greedy:  {} colors  | known χ = {}  ✓",
                greedy_colors, known_chi
            );
        } else {
            println!(
                "  Greedy:  ≤{} colors | known χ = {}  (expected: greedy may use more)",
                greedy_colors, known_chi
            );
        }
        println!();
    }

    // K5 complete graph
    {
        let g = build_k5();
        let edges = 10;
        let known_chi = 5;

        let (dsatur_col, dsatur_colors) = color_dsatur(&g);

        assert!(
            is_valid_coloring(&g, &dsatur_col),
            "K5 DSATUR: invalid coloring"
        );

        println!("K5 complete graph ({} vertices, {} edges)", g.n, edges);
        println!(
            "  DSATUR:  {} colors  | known χ = {}  {}",
            dsatur_colors,
            known_chi,
            if dsatur_colors == known_chi {
                "✓"
            } else {
                "✗"
            }
        );
        println!();
    }

    // C5 odd cycle
    {
        let g = build_c5();
        let edges = 5;
        let known_chi = 3;

        let (dsatur_col, dsatur_colors) = color_dsatur(&g);

        assert!(
            is_valid_coloring(&g, &dsatur_col),
            "C5 DSATUR: invalid coloring"
        );

        println!("C5 odd cycle ({} vertices, {} edges)", g.n, edges);
        println!(
            "  DSATUR:  {} colors  | known χ = {}  {}",
            dsatur_colors,
            known_chi,
            if dsatur_colors == known_chi {
                "✓"
            } else {
                "✗"
            }
        );
        println!();
    }

    println!("Tier 1 ceiling: DSATUR finds optimal on all instances above,");
    println!("but cannot recover from a bad early choice on adversarial graphs.");
    println!("No search, no adaptation — pure saturation-ordered greedy construction.");

    // ── loom compile check ────────────────────────────────────────────────────
    let loom_src = include_str!("../../examples/tier1/dsatur_graph_coloring.loom");
    match loom::compile(loom_src) {
        Ok(_) => println!("\n[loom compile] examples/tier1/dsatur_graph_coloring.loom → OK"),
        Err(e) => {
            let msgs: Vec<String> = e.iter().map(|err| format!("{}", err)).collect();
            println!(
                "\n[loom compile] examples/tier1/dsatur_graph_coloring.loom → ERROR: {}",
                msgs.join("; ")
            );
        }
    }
}
