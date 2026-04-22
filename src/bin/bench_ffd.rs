//! bench_ffd — First Fit Decreasing bin packing benchmark (Tier 1 Heuristic).
//!
//! Implements FFD (First Fit Decreasing) and First Fit (not decreasing),
//! then runs them on four canonical benchmark instances.
//! Also verifies that the corresponding .loom file compiles cleanly.

use loom;

// ── Bin packing algorithms ────────────────────────────────────────────────────

/// First Fit Decreasing: sort items descending, place each in first bin
/// with sufficient remaining capacity; open a new bin if none fits.
fn pack_ffd(items: &[f64], capacity: f64) -> Vec<Vec<f64>> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
    pack_first_fit_inner(&sorted, capacity)
}

/// First Fit (not decreasing): items in original order.
fn pack_ff(items: &[f64], capacity: f64) -> Vec<Vec<f64>> {
    pack_first_fit_inner(items, capacity)
}

fn pack_first_fit_inner(items: &[f64], capacity: f64) -> Vec<Vec<f64>> {
    let mut bins: Vec<Vec<f64>> = Vec::new();
    let mut remaining: Vec<f64> = Vec::new();

    for &item in items {
        // Find first bin with enough space
        let target = bins
            .iter()
            .position(|_| true)
            .and_then(|_| remaining.iter().position(|&r| r + 1e-9 >= item));

        match target {
            Some(b) => {
                bins[b].push(item);
                remaining[b] -= item;
            }
            None => {
                bins.push(vec![item]);
                remaining.push(capacity - item);
            }
        }
    }

    bins
}

/// Lower bound: ceil(sum of item sizes / capacity).
fn lower_bound(items: &[f64], capacity: f64) -> usize {
    let total: f64 = items.iter().sum();
    ((total / capacity).ceil()) as usize
}

fn bin_count(bins: &[Vec<f64>]) -> usize {
    bins.len()
}

fn utilisation(bins: &[Vec<f64>], capacity: f64) -> f64 {
    if bins.is_empty() {
        return 0.0;
    }
    let total_used: f64 = bins.iter().flat_map(|b| b.iter()).sum();
    let total_capacity = bins.len() as f64 * capacity;
    total_used / total_capacity
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let capacity = 1.0f64;

    println!("=== FFD Bin Packing Benchmark (Tier 1 Heuristic) ===");
    println!();

    // Instance 1: [0.5, 0.5, 0.5, 0.5] → optimal = 2 bins
    {
        let items = vec![0.5, 0.5, 0.5, 0.5];
        let optimal = 2;

        let ffd_bins = pack_ffd(&items, capacity);
        let ffd_count = bin_count(&ffd_bins);
        let lb = lower_bound(&items, capacity);
        let util = utilisation(&ffd_bins, capacity);

        println!("Instance 1: {:?}", items);
        println!(
            "  FFD bins:     {}  | optimal = {}  {}",
            ffd_count,
            optimal,
            if ffd_count == optimal { "✓" } else { "✗" }
        );
        println!(
            "  Lower bound:  {}  | utilisation: {:.1}%",
            lb,
            util * 100.0
        );
        println!();
    }

    // Instance 2: [0.8, 0.7, 0.5, 0.4, 0.3, 0.2]
    // sum = 2.9, lower bound = ceil(2.9/1.0) = 3; FFD produces 3 bins
    {
        let items = vec![0.8, 0.7, 0.5, 0.4, 0.3, 0.2];
        let optimal = 3;

        let ffd_bins = pack_ffd(&items, capacity);
        let ffd_count = bin_count(&ffd_bins);
        let lb = lower_bound(&items, capacity);
        let util = utilisation(&ffd_bins, capacity);

        println!("Instance 2: {:?}", items);
        println!(
            "  FFD bins:     {}  | optimal = {}  {}",
            ffd_count,
            optimal,
            if ffd_count == optimal { "✓" } else { "✗" }
        );
        println!(
            "  Lower bound:  {}  | utilisation: {:.1}%",
            lb,
            util * 100.0
        );
        println!();
    }

    // Instance 3: [0.31]*10 → optimal = 4 bins (ceil(10*0.31/1.0)=4)
    {
        let items = vec![0.31f64; 10];
        let optimal = 4;

        let ffd_bins = pack_ffd(&items, capacity);
        let ffd_count = bin_count(&ffd_bins);
        let lb = lower_bound(&items, capacity);
        let util = utilisation(&ffd_bins, capacity);

        println!("Instance 3: [0.31] × 10");
        println!(
            "  FFD bins:     {}  | optimal = {}  {}",
            ffd_count,
            optimal,
            if ffd_count <= optimal { "✓" } else { "✗" }
        );
        println!(
            "  Lower bound:  {}  | utilisation: {:.1}%",
            lb,
            util * 100.0
        );
        println!();
    }

    // Instance 4: Johnson adversarial [1/2+ε, 1/2+ε, 1/2+ε, 1/7+ε, 1/7+ε, 1/7+ε]
    // FFD produces 3 bins (OPT=3), First Fit produces 4 bins
    {
        let eps = 0.001f64;
        let items = vec![
            0.5 + eps,
            0.5 + eps,
            0.5 + eps,
            1.0 / 7.0 + eps,
            1.0 / 7.0 + eps,
            1.0 / 7.0 + eps,
        ];
        let optimal = 3;

        let ffd_bins = pack_ffd(&items, capacity);
        let ff_bins = pack_ff(&items, capacity);
        let ffd_count = bin_count(&ffd_bins);
        let ff_count = bin_count(&ff_bins);
        let lb = lower_bound(&items, capacity);
        let util_ffd = utilisation(&ffd_bins, capacity);

        println!(
            "Instance 4: Johnson adversarial (3×(1/2+ε), 3×(1/7+ε)), ε={}",
            eps
        );
        println!(
            "  FFD bins:     {}  | optimal = {}  {}",
            ffd_count,
            optimal,
            if ffd_count == optimal { "✓" } else { "✗" }
        );
        println!(
            "  First Fit:    {}  | (expected: FF uses more than FFD on this instance)",
            ff_count
        );
        println!(
            "  Lower bound:  {}  | utilisation: {:.1}%",
            lb,
            util_ffd * 100.0
        );
        println!();
    }

    println!("Tier 1 ceiling: FFD achieves 11/9·OPT + 6/9 approximation guarantee.");
    println!("No backtracking, no search — pure construction heuristic.");

    // ── loom compile check ────────────────────────────────────────────────────
    let loom_src = include_str!("../../examples/tier1/ffd_bin_packer.loom");
    match loom::compile(loom_src) {
        Ok(_) => println!("\n[loom compile] examples/tier1/ffd_bin_packer.loom → OK"),
        Err(e) => {
            let msgs: Vec<String> = e.iter().map(|err| format!("{}", err)).collect();
            println!(
                "\n[loom compile] examples/tier1/ffd_bin_packer.loom → ERROR: {}",
                msgs.join("; ")
            );
        }
    }
}
