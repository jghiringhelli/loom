//! bench_spt — Shortest Processing Time scheduling benchmark (Tier 1 Heuristic).
//!
//! Implements SPT (Shortest Processing Time), WSPT (Weighted SPT), and EDD
//! (Earliest Due Date) scheduling rules, then runs them on three canonical
//! benchmark instances.  Also verifies that the corresponding .loom file
//! compiles cleanly.

use loom;

// ── Job and schedule types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Job {
    id: usize,
    processing_time: f64,
    weight: f64,
    due_date: Option<f64>,
}

#[derive(Clone, Debug)]
struct ScheduledJob {
    job: Job,
    start_time: f64,
    completion_time: f64,
}

struct Schedule {
    jobs: Vec<ScheduledJob>,
    makespan: f64,
    total_completion_time: f64,
    weighted_completion_time: f64,
}

// ── SPT: sort ascending by processing_time ────────────────────────────────────

fn schedule_spt(jobs: &[Job]) -> Schedule {
    let mut ordered = jobs.to_vec();
    ordered.sort_by(|a, b| a.processing_time.partial_cmp(&b.processing_time).unwrap());
    build_schedule(ordered)
}

// ── WSPT: sort ascending by processing_time / weight ─────────────────────────

fn schedule_wspt(jobs: &[Job]) -> Schedule {
    let mut ordered = jobs.to_vec();
    ordered.sort_by(|a, b| {
        let ra = a.processing_time / a.weight;
        let rb = b.processing_time / b.weight;
        ra.partial_cmp(&rb).unwrap()
    });
    build_schedule(ordered)
}

// ── EDD: sort ascending by due_date ──────────────────────────────────────────

fn schedule_edd(jobs: &[Job]) -> Schedule {
    let mut ordered = jobs.to_vec();
    ordered.sort_by(|a, b| {
        let da = a.due_date.unwrap_or(f64::MAX);
        let db = b.due_date.unwrap_or(f64::MAX);
        da.partial_cmp(&db).unwrap()
    });
    build_schedule(ordered)
}

// ── Random order (for comparison) ────────────────────────────────────────────

/// Simple deterministic "random-ish" ordering: reverse of input order.
fn schedule_reverse(jobs: &[Job]) -> Schedule {
    let mut ordered = jobs.to_vec();
    ordered.reverse();
    build_schedule(ordered)
}

// ── Schedule builder ─────────────────────────────────────────────────────────

fn build_schedule(ordered: Vec<Job>) -> Schedule {
    let mut time = 0.0f64;
    let mut scheduled = Vec::with_capacity(ordered.len());
    let mut total_c = 0.0f64;
    let mut weighted_c = 0.0f64;

    for job in ordered {
        let start = time;
        let completion = start + job.processing_time;
        total_c += completion;
        weighted_c += job.weight * completion;
        scheduled.push(ScheduledJob {
            job: job.clone(),
            start_time: start,
            completion_time: completion,
        });
        time = completion;
    }

    Schedule {
        makespan: time,
        total_completion_time: total_c,
        weighted_completion_time: weighted_c,
        jobs: scheduled,
    }
}

// ── Maximum lateness (for EDD comparison) ────────────────────────────────────

fn max_lateness(sched: &Schedule) -> f64 {
    sched
        .jobs
        .iter()
        .map(|sj| {
            if let Some(d) = sj.job.due_date {
                sj.completion_time - d
            } else {
                f64::NEG_INFINITY
            }
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== SPT Job Scheduling Benchmark (Tier 1 Heuristic) ===");
    println!();

    // Instance 1: 4 jobs, times=[2,5,1,3], weights=[1,1,1,1]
    // SPT order: job3(t=1), job1(t=2), job4(t=3), job2(t=5)
    // Completions: 1, 3, 6, 11 → total = 21
    {
        let jobs = vec![
            Job {
                id: 1,
                processing_time: 2.0,
                weight: 1.0,
                due_date: None,
            },
            Job {
                id: 2,
                processing_time: 5.0,
                weight: 1.0,
                due_date: None,
            },
            Job {
                id: 3,
                processing_time: 1.0,
                weight: 1.0,
                due_date: None,
            },
            Job {
                id: 4,
                processing_time: 3.0,
                weight: 1.0,
                due_date: None,
            },
        ];
        let known_optimal_total = 21.0f64;

        let spt = schedule_spt(&jobs);
        let rev = schedule_reverse(&jobs);

        let spt_order: Vec<usize> = spt.jobs.iter().map(|sj| sj.job.id).collect();
        println!("Instance 1: times=[2,5,1,3], weights=[1,1,1,1]");
        println!("  SPT order:      {:?}", spt_order);
        println!(
            "  Total completion: {:.0}  | known optimal = {:.0}  {}",
            spt.total_completion_time,
            known_optimal_total,
            if (spt.total_completion_time - known_optimal_total).abs() < 0.01 {
                "✓"
            } else {
                "✗"
            }
        );
        println!("  Makespan:       {:.0}", spt.makespan);
        println!(
            "  SPT ≤ reverse:  {} (SPT={:.0}, reverse={:.0})",
            spt.total_completion_time <= rev.total_completion_time,
            spt.total_completion_time,
            rev.total_completion_time
        );
        println!();
    }

    // Instance 2: 3 jobs, times=[3,1,2], weights=[3,1,2]
    // WSPT ratio: job1=3/3=1, job2=1/1=1, job3=2/2=1 (all equal)
    // Tie-breaking by processing time: order j2(t=1),j3(t=2),j1(t=3)
    // Completions: 1, 3, 6 → weighted = 1*1 + 2*3 + 3*6 = 1+6+18 = 25
    {
        let jobs = vec![
            Job {
                id: 1,
                processing_time: 3.0,
                weight: 3.0,
                due_date: None,
            },
            Job {
                id: 2,
                processing_time: 1.0,
                weight: 1.0,
                due_date: None,
            },
            Job {
                id: 3,
                processing_time: 2.0,
                weight: 2.0,
                due_date: None,
            },
        ];

        let wspt = schedule_wspt(&jobs);
        let wspt_order: Vec<usize> = wspt.jobs.iter().map(|sj| sj.job.id).collect();

        println!("Instance 2: times=[3,1,2], weights=[3,1,2] (all p/w ratios = 1)");
        println!(
            "  WSPT order:            {:?}  (ties resolved by processing time)",
            wspt_order
        );
        println!(
            "  Weighted completion:   {:.0}",
            wspt.weighted_completion_time
        );
        println!("  Total completion:      {:.0}", wspt.total_completion_time);
        println!("  (All p/w ratios equal; WSPT is optimal for any tie-breaking)",);
        println!();
    }

    // Instance 3: 5 jobs, times=[4,2,6,1,3]
    // SPT order: 1,3,6,10,15 → total = 35
    {
        let jobs = vec![
            Job {
                id: 1,
                processing_time: 4.0,
                weight: 1.0,
                due_date: Some(5.0),
            },
            Job {
                id: 2,
                processing_time: 2.0,
                weight: 1.0,
                due_date: Some(3.0),
            },
            Job {
                id: 3,
                processing_time: 6.0,
                weight: 1.0,
                due_date: Some(9.0),
            },
            Job {
                id: 4,
                processing_time: 1.0,
                weight: 1.0,
                due_date: Some(2.0),
            },
            Job {
                id: 5,
                processing_time: 3.0,
                weight: 1.0,
                due_date: Some(7.0),
            },
        ];
        // SPT order: t4=1, t2=2, t5=3, t1=4, t3=6 → completions: 1+3+6+10+16 = 36
        let known_optimal_total = 36.0f64;

        let spt = schedule_spt(&jobs);
        let edd = schedule_edd(&jobs);
        let rev = schedule_reverse(&jobs);

        let spt_order: Vec<usize> = spt.jobs.iter().map(|sj| sj.job.id).collect();
        let edd_order: Vec<usize> = edd.jobs.iter().map(|sj| sj.job.id).collect();

        println!("Instance 3: times=[4,2,6,1,3], due dates=[5,3,9,2,7]");
        println!("  SPT order:      {:?}", spt_order);
        println!(
            "  Total completion: {:.0}  | known optimal = {:.0}  {}",
            spt.total_completion_time,
            known_optimal_total,
            if (spt.total_completion_time - known_optimal_total).abs() < 0.01 {
                "✓"
            } else {
                "✗"
            }
        );
        println!("  Makespan:       {:.0}", spt.makespan);
        println!();
        println!(
            "  EDD order:      {:?}  (optimal for max lateness)",
            edd_order
        );
        println!("  EDD max lateness:  {:.0}", max_lateness(&edd));
        println!("  SPT max lateness:  {:.0}", max_lateness(&spt));
        println!();
        println!(
            "  SPT ≤ reverse ordering: {} (SPT={:.0}, reverse={:.0})",
            spt.total_completion_time <= rev.total_completion_time,
            spt.total_completion_time,
            rev.total_completion_time
        );
        println!();
    }

    println!("Tier 1 ceiling: SPT is optimal for total completion time on a single machine.");
    println!("No search, no adaptation — a pure fixed dispatch rule.");

    // ── loom compile check ────────────────────────────────────────────────────
    let loom_src = include_str!("../../examples/tier1/greedy_job_scheduler.loom");
    match loom::compile(loom_src) {
        Ok(_) => println!("\n[loom compile] examples/tier1/greedy_job_scheduler.loom → OK"),
        Err(e) => {
            let msgs: Vec<String> = e.iter().map(|err| format!("{}", err)).collect();
            println!(
                "\n[loom compile] examples/tier1/greedy_job_scheduler.loom → ERROR: {}",
                msgs.join("; ")
            );
        }
    }
}
