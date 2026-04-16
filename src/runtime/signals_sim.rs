//! Signal simulator — generates realistic domain-specific telemetry for all 7
//! curated BIOISO-class entities.
//!
//! Each domain expert spec defines four metrics with:
//! - **Baseline**: starting value at tick 0
//! - **Trend**: additive drift per tick
//! - **Noise amplitude**: Gaussian-style noise (approximated via LCG)
//! - **Crisis windows**: tick ranges where the signal spikes dramatically
//!
//! The simulator is deterministic given a seed — so experiments are reproducible.
//!
//! # Usage
//! ```rust,ignore
//! let mut sim = SignalSimulator::new(42);
//! for tick in 0..500 {
//!     let signals = sim.tick(tick);
//!     for s in signals { runtime.store.write_signal(&s).unwrap(); }
//! }
//! ```

use crate::runtime::signal::{Signal, Timestamp};

// ── LCG pseudo-random number generator ───────────────────────────────────────

/// Minimal LCG PRNG — no external dependency required.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x6c62272e07bb0142)
    }

    /// Advance and return next u64.
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    /// Return a float in [-1.0, 1.0].
    fn next_f64(&mut self) -> f64 {
        let u = self.next_u64();
        // Map to [0, 1] then center
        (u as f64 / u64::MAX as f64) * 2.0 - 1.0
    }

    /// Return a float with approximate Gaussian shape (sum of 4 uniform → CLT).
    fn gaussian(&mut self) -> f64 {
        (self.next_f64() + self.next_f64() + self.next_f64() + self.next_f64()) / 4.0
    }
}

// ── Crisis window ─────────────────────────────────────────────────────────────

/// A planned crisis event for a metric.
struct Crisis {
    /// Tick range [start, end) during which the crisis is active.
    start: u64,
    end: u64,
    /// Additive spike applied on top of normal trend+noise during the crisis.
    amplitude: f64,
    /// Rate at which the spike builds/decays within the window (fraction/tick).
    buildup: f64,
}

impl Crisis {
    fn new(start: u64, end: u64, amplitude: f64) -> Self {
        Self {
            start,
            end,
            amplitude,
            buildup: 0.1,
        }
    }

    /// Contribution at this tick (0.0 if outside window).
    fn contribution(&self, tick: u64) -> f64 {
        if tick < self.start || tick >= self.end {
            return 0.0;
        }
        let mid = (self.start + self.end) / 2;
        let half = (self.end - self.start) as f64 / 2.0;
        let dist = (tick as f64 - mid as f64).abs();
        // Bell-shaped: 1.0 at peak, tapers toward edges
        let shape = 1.0 - (dist / half).min(1.0).powi(2);
        self.amplitude * shape * self.buildup.min(1.0)
    }
}

// ── MetricSpec ────────────────────────────────────────────────────────────────

/// Configuration for one metric in a domain entity.
struct MetricSpec {
    name: &'static str,
    baseline: f64,
    trend_per_tick: f64,
    noise_amplitude: f64,
    crises: Vec<Crisis>,
}

impl MetricSpec {
    /// Compute the signal value at a given tick.
    fn value_at(&self, tick: u64, rng: &mut Lcg) -> f64 {
        let trend = self.baseline + self.trend_per_tick * tick as f64;
        let noise = rng.gaussian() * self.noise_amplitude;
        let crisis_spike: f64 = self.crises.iter().map(|c| c.contribution(tick)).sum();
        trend + noise + crisis_spike
    }
}

// ── DomainSpec ────────────────────────────────────────────────────────────────

struct DomainSpec {
    entity_id: &'static str,
    metrics: Vec<MetricSpec>,
}

// ── All domain specs ──────────────────────────────────────────────────────────

fn all_domain_specs() -> Vec<DomainSpec> {
    vec![
        // ── 1. Antimicrobial Resistance Coevolution ───────────────────────────────────
        // Calibrated against WHO AMR report; resistance prevalence rising ~0.4%/yr
        DomainSpec {
            entity_id: "amr_coevolution",
            metrics: vec![
                MetricSpec {
                    name: "resistance_prevalence_pct",
                    baseline: 0.28,
                    trend_per_tick: 0.0004,
                    noise_amplitude: 0.01,
                    crises: vec![
                        Crisis::new(120, 155, 0.08), // Novel resistance strain emergence
                        Crisis::new(350, 380, 0.06),
                    ],
                },
                MetricSpec {
                    name: "effective_drug_count",
                    baseline: 8.0,
                    trend_per_tick: -0.004,
                    noise_amplitude: 0.3,
                    crises: vec![Crisis::new(122, 158, -2.5)],
                },
                MetricSpec {
                    name: "treatment_success_rate",
                    baseline: 0.71,
                    trend_per_tick: -0.0003,
                    noise_amplitude: 0.015,
                    crises: vec![Crisis::new(118, 160, -0.12), Crisis::new(348, 382, -0.08)],
                },
                MetricSpec {
                    name: "novel_resistance_rate",
                    baseline: 0.12,
                    trend_per_tick: 0.0002,
                    noise_amplitude: 0.01,
                    crises: vec![Crisis::new(120, 156, 0.15)],
                },
            ],
        },
        // ── 2. HFT Flash Crash Defense ────────────────────────────────────────────────
        // Calibrated against CFTC/SEC 2010 report; HFT arms race dynamics
        DomainSpec {
            entity_id: "flash_crash",
            metrics: vec![
                MetricSpec {
                    name: "order_book_depth",
                    baseline: 0.9,
                    trend_per_tick: -0.00005,
                    noise_amplitude: 0.03,
                    crises: vec![
                        Crisis::new(55, 65, -0.50), // Flash crash event
                        Crisis::new(320, 328, -0.40),
                        Crisis::new(460, 468, -0.30),
                    ],
                },
                MetricSpec {
                    name: "bid_ask_spread_bps",
                    baseline: 0.5,
                    trend_per_tick: 0.002,
                    noise_amplitude: 0.08,
                    crises: vec![Crisis::new(53, 67, 12.0), Crisis::new(318, 330, 9.0)],
                },
                MetricSpec {
                    name: "volatility_index",
                    baseline: 0.15,
                    trend_per_tick: 0.00002,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(50, 70, 0.55), Crisis::new(316, 332, 0.40)],
                },
                MetricSpec {
                    name: "cancellation_rate",
                    baseline: 0.25,
                    trend_per_tick: 0.00008,
                    noise_amplitude: 0.015,
                    crises: vec![Crisis::new(52, 68, 0.35)],
                },
            ],
        },
        // ── 3. Adaptive JIT Compiler Optimization ─────────────────────────────────────
        // Hot path distribution shifts as programs evolve at runtime
        DomainSpec {
            entity_id: "adaptive_jit",
            metrics: vec![
                MetricSpec {
                    name: "hotpath_coverage_pct",
                    baseline: 0.62,
                    trend_per_tick: 0.0004,
                    noise_amplitude: 0.025,
                    crises: vec![
                        Crisis::new(80, 110, -0.18), // Hot path restructure event
                        Crisis::new(290, 320, -0.14),
                    ],
                },
                MetricSpec {
                    name: "generated_code_speedup",
                    baseline: 3.5,
                    trend_per_tick: 0.006,
                    noise_amplitude: 0.3,
                    crises: vec![Crisis::new(82, 112, -1.2), Crisis::new(292, 322, -0.9)],
                },
                MetricSpec {
                    name: "compilation_overhead_ms",
                    baseline: 80.0,
                    trend_per_tick: -0.02,
                    noise_amplitude: 4.0,
                    crises: vec![Crisis::new(78, 114, 60.0)], // Pass recompilation storm
                },
                MetricSpec {
                    name: "cache_hit_rate",
                    baseline: 0.74,
                    trend_per_tick: 0.0003,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(80, 112, -0.22)],
                },
            ],
        },
        // ── 4. Protein Drug Resistance ────────────────────────────────────────────────
        // Target protein mutations (HIV protease, BCR-ABL, EGFR) shift binding landscape
        DomainSpec {
            entity_id: "protein_drug_resistance",
            metrics: vec![
                MetricSpec {
                    name: "binding_affinity_kcal_mol",
                    baseline: -6.2,
                    trend_per_tick: 0.006, // Affinity worsening as target mutates
                    noise_amplitude: 0.2,
                    crises: vec![
                        Crisis::new(100, 135, 1.8), // Resistance mutation acquisition
                        Crisis::new(310, 345, 1.4),
                    ],
                },
                MetricSpec {
                    name: "admet_score",
                    baseline: 0.48,
                    trend_per_tick: -0.0001,
                    noise_amplitude: 0.025,
                    crises: vec![Crisis::new(98, 138, -0.12)],
                },
                MetricSpec {
                    name: "resistance_mutation_count",
                    baseline: 8.0,
                    trend_per_tick: 0.012,
                    noise_amplitude: 0.5,
                    crises: vec![Crisis::new(102, 132, 5.0), Crisis::new(312, 342, 3.5)],
                },
                MetricSpec {
                    name: "active_lead_count",
                    baseline: 3.0,
                    trend_per_tick: -0.002,
                    noise_amplitude: 0.2,
                    crises: vec![Crisis::new(96, 140, -1.5)],
                },
            ],
        },
        // ── 5. ICS/SCADA Zero-Day Defense ─────────────────────────────────────────────
        // Novel attack classes appear suddenly; detection logic must be synthesised
        DomainSpec {
            entity_id: "ics_zero_day",
            metrics: vec![
                MetricSpec {
                    name: "detection_rate_pct",
                    baseline: 0.72,
                    trend_per_tick: 0.0003,
                    noise_amplitude: 0.02,
                    crises: vec![
                        Crisis::new(60, 95, -0.30), // Novel attack class appears
                        Crisis::new(280, 315, -0.22),
                        Crisis::new(430, 460, -0.18),
                    ],
                },
                MetricSpec {
                    name: "false_positive_rate",
                    baseline: 0.08,
                    trend_per_tick: -0.00008,
                    noise_amplitude: 0.008,
                    crises: vec![Crisis::new(58, 98, 0.12)],
                },
                MetricSpec {
                    name: "novel_attack_coverage",
                    baseline: 0.12,
                    trend_per_tick: 0.0006,
                    noise_amplitude: 0.015,
                    crises: vec![Crisis::new(62, 92, -0.08), Crisis::new(282, 312, -0.06)],
                },
                MetricSpec {
                    name: "response_latency_ms",
                    baseline: 850.0,
                    trend_per_tick: -0.3,
                    noise_amplitude: 30.0,
                    crises: vec![Crisis::new(56, 100, 1200.0)],
                },
            ],
        },
        // ── 6. Quantum Error Mitigation ────────────────────────────────────────────────
        // Hardware recalibration events shift noise models; decomposition must adapt
        DomainSpec {
            entity_id: "quantum_error_mitigation",
            metrics: vec![
                MetricSpec {
                    name: "logical_error_rate",
                    baseline: 0.012,
                    trend_per_tick: -0.000004,
                    noise_amplitude: 0.0008,
                    crises: vec![
                        Crisis::new(90, 115, 0.008), // Hardware drift / recalibration
                        Crisis::new(300, 325, 0.006),
                        Crisis::new(440, 465, 0.005),
                    ],
                },
                MetricSpec {
                    name: "circuit_depth",
                    baseline: 280.0,
                    trend_per_tick: -0.12,
                    noise_amplitude: 8.0,
                    crises: vec![Crisis::new(88, 118, 80.0)],
                },
                MetricSpec {
                    name: "t_gate_count",
                    baseline: 145.0,
                    trend_per_tick: -0.06,
                    noise_amplitude: 5.0,
                    crises: vec![Crisis::new(86, 120, 40.0)],
                },
                MetricSpec {
                    name: "qubit_fidelity",
                    baseline: 0.991,
                    trend_per_tick: 0.00002,
                    noise_amplitude: 0.001,
                    crises: vec![Crisis::new(92, 116, -0.006), Crisis::new(302, 326, -0.004)],
                },
            ],
        },
        // ── 7. Climate Intervention Sequencing ────────────────────────────────────────
        // System coupling coefficients shift after each deployed intervention
        DomainSpec {
            entity_id: "climate_intervention",
            metrics: vec![
                MetricSpec {
                    name: "intervention_efficacy",
                    baseline: 0.28,
                    trend_per_tick: 0.0004,
                    noise_amplitude: 0.025,
                    crises: vec![
                        Crisis::new(110, 150, -0.15), // Prior intervention side-effects
                        Crisis::new(330, 370, -0.10),
                    ],
                },
                MetricSpec {
                    name: "tipping_point_risk",
                    baseline: 0.42,
                    trend_per_tick: 0.0003,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(108, 154, 0.18), Crisis::new(328, 374, 0.12)],
                },
                MetricSpec {
                    name: "co2_trajectory_delta",
                    baseline: 2.5,
                    trend_per_tick: -0.004,
                    noise_amplitude: 0.3,
                    crises: vec![Crisis::new(112, 148, 4.0)],
                },
                MetricSpec {
                    name: "system_resilience",
                    baseline: 0.51,
                    trend_per_tick: 0.0002,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(106, 156, -0.12), Crisis::new(326, 376, -0.08)],
                },
            ],
        },
    ]
}

// ── SignalSimulator ────────────────────────────────────────────────────────────

/// Generates realistic domain-specific signals for all 7 curated BIOISO-class entities.
///
/// The simulator is deterministic given a seed. Call [`tick`] once per
/// orchestrator tick to get the signals to inject into the signal store.
pub struct SignalSimulator {
    rng: Lcg,
    domains: Vec<DomainSpec>,
    /// Subset of entity IDs to simulate. Empty = all.
    filter: Vec<String>,
}

impl SignalSimulator {
    /// Create a new simulator with the given seed.
    ///
    /// Use `42` for reproducible experiments. Use `now_ms` for random runs.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Lcg::new(seed),
            domains: all_domain_specs(),
            filter: Vec::new(),
        }
    }

    /// Restrict simulation to a subset of entity IDs.
    pub fn with_filter(mut self, entity_ids: Vec<String>) -> Self {
        self.filter = entity_ids;
        self
    }

    /// Generate all signals for the given tick.
    ///
    /// Returns one `Signal` per (entity, metric) pair for all active domains.
    pub fn tick(&mut self, tick: u64, ts: Timestamp) -> Vec<Signal> {
        let mut out = Vec::new();
        for domain in &self.domains {
            if !self.filter.is_empty() && !self.filter.iter().any(|f| f == domain.entity_id) {
                continue;
            }
            for metric in &domain.metrics {
                let value = metric.value_at(tick, &mut self.rng);
                out.push(Signal::with_timestamp(
                    domain.entity_id,
                    metric.name,
                    value,
                    ts,
                ));
            }
        }
        out
    }

    /// Total number of entity-metric pairs that will be simulated.
    pub fn signal_count(&self) -> usize {
        let domains: Vec<_> = if self.filter.is_empty() {
            self.domains.iter().collect()
        } else {
            self.domains
                .iter()
                .filter(|d| self.filter.iter().any(|f| f == d.entity_id))
                .collect()
        };
        domains.iter().map(|d| d.metrics.len()).sum()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulator_produces_signals_for_all_7_entities() {
        let mut sim = SignalSimulator::new(42);
        let signals = sim.tick(0, 1_000_000);
        // 7 BIOISO-class entities × 4 metrics each
        assert_eq!(signals.len(), 28);
    }

    #[test]
    fn simulator_is_deterministic_given_same_seed() {
        let mut a = SignalSimulator::new(99);
        let mut b = SignalSimulator::new(99);
        let sa = a.tick(10, 1_000_000);
        let sb = b.tick(10, 1_000_000);
        assert_eq!(sa.len(), sb.len());
        for (x, y) in sa.iter().zip(sb.iter()) {
            assert_eq!(x.entity_id, y.entity_id);
            assert_eq!(x.metric, y.metric);
            assert!((x.value - y.value).abs() < 1e-10);
        }
    }

    #[test]
    fn crisis_windows_produce_elevated_values() {
        // AMR novel resistance crisis at ticks 120-155 — expect elevated resistance_prevalence
        let normal = SignalSimulator::new(0).tick(0, 1_000);
        let crisis = {
            let mut s = SignalSimulator::new(0);
            for t in 0..130u64 {
                s.tick(t, t * 1000);
            }
            s.tick(130, 130_000)
        };
        let normal_val = normal
            .iter()
            .find(|s| s.entity_id == "amr_coevolution" && s.metric == "resistance_prevalence_pct")
            .unwrap()
            .value;
        let crisis_val = crisis
            .iter()
            .find(|s| s.entity_id == "amr_coevolution" && s.metric == "resistance_prevalence_pct")
            .unwrap()
            .value;
        // Crisis resistance prevalence should be higher than baseline
        assert!(
            crisis_val > normal_val,
            "crisis={crisis_val:.4} should exceed normal={normal_val:.4}"
        );
    }

    #[test]
    fn filter_restricts_to_subset() {
        let mut sim = SignalSimulator::new(1).with_filter(vec![
            "amr_coevolution".to_string(),
            "flash_crash".to_string(),
        ]);
        let signals = sim.tick(0, 1_000);
        assert_eq!(signals.len(), 8); // 2 entities × 4 metrics
        assert!(signals
            .iter()
            .all(|s| s.entity_id == "amr_coevolution" || s.entity_id == "flash_crash"));
    }

    #[test]
    fn trend_increases_over_time() {
        let mut sim = SignalSimulator::new(7);
        // AMR resistance prevalence must trend up over time
        let early: f64 = (0..5u64)
            .map(|t| {
                sim.tick(t, t * 1000)
                    .into_iter()
                    .find(|s| {
                        s.entity_id == "amr_coevolution" && s.metric == "resistance_prevalence_pct"
                    })
                    .unwrap()
                    .value
            })
            .sum::<f64>()
            / 5.0;

        let mut sim2 = SignalSimulator::new(7);
        for t in 0..400u64 {
            sim2.tick(t, t * 1000);
        }
        let late: f64 = (400..405u64)
            .map(|t| {
                sim2.tick(t, t * 1000)
                    .into_iter()
                    .find(|s| {
                        s.entity_id == "amr_coevolution" && s.metric == "resistance_prevalence_pct"
                    })
                    .unwrap()
                    .value
            })
            .sum::<f64>()
            / 5.0;

        assert!(
            late > early,
            "late={late:.4} should exceed early={early:.4}"
        );
    }
}
