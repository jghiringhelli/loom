//! Signal simulator — generates realistic domain-specific telemetry for all 11
//! BIOISO entities.
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
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
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
        // ── Climate Change Mitigation ──────────────────────────────────────────
        DomainSpec {
            entity_id: "climate",
            metrics: vec![
                MetricSpec {
                    name: "co2_ppm",
                    baseline: 420.0,
                    trend_per_tick: 0.08,   // ~2.5 ppm/yr compressed to 500 ticks
                    noise_amplitude: 0.4,
                    crises: vec![
                        Crisis::new(80, 110, 6.0),   // El Niño CO₂ spike
                        Crisis::new(280, 320, 4.0),  // Industrial surge
                    ],
                },
                MetricSpec {
                    name: "temp_anomaly_c",
                    baseline: 1.22,
                    trend_per_tick: 0.003,
                    noise_amplitude: 0.05,
                    crises: vec![
                        Crisis::new(150, 180, 0.25),  // Warm year event
                        Crisis::new(400, 430, 0.35),  // Record anomaly
                    ],
                },
                MetricSpec {
                    name: "sea_level_mm",
                    baseline: 100.0,
                    trend_per_tick: 0.36,   // ~3.6 mm/yr
                    noise_amplitude: 1.5,
                    crises: vec![Crisis::new(300, 360, 12.0)], // Ice sheet calving event
                },
                MetricSpec {
                    name: "extreme_events_idx",
                    baseline: 1.0,
                    trend_per_tick: 0.006,
                    noise_amplitude: 0.08,
                    crises: vec![Crisis::new(90, 130, 0.6)],
                },
            ],
        },

        // ── Epidemic Response ──────────────────────────────────────────────────
        DomainSpec {
            entity_id: "epidemics",
            metrics: vec![
                MetricSpec {
                    name: "Rt",
                    baseline: 2.5,
                    trend_per_tick: -0.012,   // Intervention brings it down
                    noise_amplitude: 0.15,
                    crises: vec![
                        Crisis::new(40, 60, 1.8),   // Policy relaxation surge
                        Crisis::new(200, 240, 2.2), // New variant emergence
                        Crisis::new(380, 410, 1.5), // Seasonal wave
                    ],
                },
                MetricSpec {
                    name: "icu_occupancy_pct",
                    baseline: 0.82,
                    trend_per_tick: -0.003,
                    noise_amplitude: 0.04,
                    crises: vec![
                        Crisis::new(50, 75, 0.35),
                        Crisis::new(210, 250, 0.40),
                    ],
                },
                MetricSpec {
                    name: "daily_cases_per_100k",
                    baseline: 45.0,
                    trend_per_tick: -0.18,
                    noise_amplitude: 3.0,
                    crises: vec![
                        Crisis::new(45, 70, 120.0),
                        Crisis::new(205, 245, 90.0),
                        Crisis::new(375, 405, 65.0),
                    ],
                },
                MetricSpec {
                    name: "vax_coverage_pct",
                    baseline: 0.35,
                    trend_per_tick: 0.0012,
                    noise_amplitude: 0.005,
                    crises: vec![],
                },
            ],
        },

        // ── Antibiotic Resistance (AMR) ────────────────────────────────────────
        DomainSpec {
            entity_id: "antibiotic_res",
            metrics: vec![
                MetricSpec {
                    name: "amr_deaths_per_yr_k",
                    baseline: 700.0,
                    trend_per_tick: 0.5,
                    noise_amplitude: 8.0,
                    crises: vec![
                        Crisis::new(120, 150, 80.0),  // New resistance strain
                        Crisis::new(350, 380, 60.0),
                    ],
                },
                MetricSpec {
                    name: "resistance_prevalence_pct",
                    baseline: 0.28,
                    trend_per_tick: 0.0004,
                    noise_amplitude: 0.01,
                    crises: vec![Crisis::new(120, 155, 0.08)],
                },
                MetricSpec {
                    name: "new_antibiotic_pipeline",
                    baseline: 8.0,
                    trend_per_tick: -0.004,  // Pipeline drying up
                    noise_amplitude: 0.3,
                    crises: vec![],
                },
                MetricSpec {
                    name: "antibiotic_consumption_ddd",
                    baseline: 21.0,
                    trend_per_tick: 0.01,
                    noise_amplitude: 0.5,
                    crises: vec![Crisis::new(45, 65, 4.0)],
                },
            ],
        },

        // ── Power Grid Stability ───────────────────────────────────────────────
        DomainSpec {
            entity_id: "grid_stability",
            metrics: vec![
                MetricSpec {
                    name: "frequency_hz",
                    baseline: 60.0,
                    trend_per_tick: 0.0,
                    noise_amplitude: 0.04,
                    crises: vec![
                        Crisis::new(70, 85, 0.6),   // Renewable intermittency spike
                        Crisis::new(200, 210, 0.8), // Grid fault
                        Crisis::new(440, 455, 0.5),
                    ],
                },
                MetricSpec {
                    name: "load_mw",
                    baseline: 45000.0,
                    trend_per_tick: 8.0,   // Growing demand
                    noise_amplitude: 500.0,
                    crises: vec![
                        Crisis::new(65, 90, 8000.0), // Peak demand event
                        Crisis::new(195, 215, 6000.0),
                    ],
                },
                MetricSpec {
                    name: "renewable_fraction",
                    baseline: 0.28,
                    trend_per_tick: 0.0005,
                    noise_amplitude: 0.04,   // High variance from wind/solar
                    crises: vec![],
                },
                MetricSpec {
                    name: "cascading_risk_idx",
                    baseline: 0.12,
                    trend_per_tick: 0.0001,
                    noise_amplitude: 0.02,
                    crises: vec![
                        Crisis::new(68, 88, 0.45),
                        Crisis::new(197, 213, 0.55),
                    ],
                },
            ],
        },

        // ── Soil Organic Carbon ────────────────────────────────────────────────
        DomainSpec {
            entity_id: "soil_carbon",
            metrics: vec![
                MetricSpec {
                    name: "soc_change_per_mille",
                    baseline: -1.2,   // Declining
                    trend_per_tick: 0.006,  // Slowly recovering with interventions
                    noise_amplitude: 0.15,
                    crises: vec![
                        Crisis::new(100, 140, -1.8),  // Drought-driven loss
                        Crisis::new(300, 340, -1.2),
                    ],
                },
                MetricSpec {
                    name: "soil_moisture_pct",
                    baseline: 0.32,
                    trend_per_tick: -0.0002,
                    noise_amplitude: 0.03,
                    crises: vec![Crisis::new(95, 145, -0.12)],
                },
                MetricSpec {
                    name: "cover_crop_adoption_pct",
                    baseline: 0.18,
                    trend_per_tick: 0.0006,
                    noise_amplitude: 0.01,
                    crises: vec![],
                },
                MetricSpec {
                    name: "erosion_rate_t_ha_yr",
                    baseline: 12.0,
                    trend_per_tick: -0.01,
                    noise_amplitude: 0.8,
                    crises: vec![Crisis::new(98, 148, 8.0)],
                },
            ],
        },

        // ── ICU Sepsis Protocol ────────────────────────────────────────────────
        DomainSpec {
            entity_id: "sepsis",
            metrics: vec![
                MetricSpec {
                    name: "mortality_28d_pct",
                    baseline: 0.27,
                    trend_per_tick: -0.0002,
                    noise_amplitude: 0.015,
                    crises: vec![
                        Crisis::new(60, 90, 0.08),   // Outbreak + staff shortage
                        Crisis::new(280, 310, 0.06),
                    ],
                },
                MetricSpec {
                    name: "time_to_antibiotics_hr",
                    baseline: 3.2,
                    trend_per_tick: -0.002,
                    noise_amplitude: 0.2,
                    crises: vec![Crisis::new(62, 92, 2.5)],
                },
                MetricSpec {
                    name: "bundle_compliance_pct",
                    baseline: 0.72,
                    trend_per_tick: 0.0004,
                    noise_amplitude: 0.02,
                    crises: vec![],
                },
                MetricSpec {
                    name: "organ_failure_score",
                    baseline: 6.5,
                    trend_per_tick: -0.003,
                    noise_amplitude: 0.4,
                    crises: vec![Crisis::new(58, 94, 3.5)],
                },
            ],
        },

        // ── HFT Flash Crash Prevention ─────────────────────────────────────────
        DomainSpec {
            entity_id: "flash_crash",
            metrics: vec![
                MetricSpec {
                    name: "order_book_depth_m",
                    baseline: 85.0,
                    trend_per_tick: -0.02,
                    noise_amplitude: 3.0,
                    crises: vec![
                        Crisis::new(55, 65, -50.0),  // Flash crash event
                        Crisis::new(320, 328, -40.0),
                        Crisis::new(460, 468, -30.0),
                    ],
                },
                MetricSpec {
                    name: "bid_ask_spread_bps",
                    baseline: 2.5,
                    trend_per_tick: 0.002,
                    noise_amplitude: 0.3,
                    crises: vec![
                        Crisis::new(53, 67, 12.0),
                        Crisis::new(318, 330, 9.0),
                    ],
                },
                MetricSpec {
                    name: "hft_cancellation_ratio",
                    baseline: 0.92,
                    trend_per_tick: 0.0001,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(52, 68, 0.06)],
                },
                MetricSpec {
                    name: "volatility_vix",
                    baseline: 18.0,
                    trend_per_tick: 0.02,
                    noise_amplitude: 1.5,
                    crises: vec![
                        Crisis::new(50, 70, 35.0),
                        Crisis::new(316, 332, 25.0),
                    ],
                },
            ],
        },

        // ── Nuclear Reactor Safety ─────────────────────────────────────────────
        DomainSpec {
            entity_id: "nuclear_safety",
            metrics: vec![
                MetricSpec {
                    name: "safety_margin_pct",
                    baseline: 0.85,
                    trend_per_tick: -0.0003,   // Aging degradation
                    noise_amplitude: 0.008,
                    crises: vec![
                        Crisis::new(180, 210, -0.18),  // Equipment anomaly
                        Crisis::new(420, 445, -0.12),
                    ],
                },
                MetricSpec {
                    name: "coolant_temp_c",
                    baseline: 285.0,
                    trend_per_tick: 0.02,
                    noise_amplitude: 0.8,
                    crises: vec![
                        Crisis::new(182, 212, 18.0),
                        Crisis::new(422, 447, 12.0),
                    ],
                },
                MetricSpec {
                    name: "neutron_flux_normalized",
                    baseline: 1.0,
                    trend_per_tick: 0.0002,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(185, 208, 0.15)],
                },
                MetricSpec {
                    name: "maintenance_backlog_hrs",
                    baseline: 120.0,
                    trend_per_tick: 0.4,
                    noise_amplitude: 5.0,
                    crises: vec![],
                },
            ],
        },

        // ── Global Supply Chain Resilience ─────────────────────────────────────
        DomainSpec {
            entity_id: "supply_chain",
            metrics: vec![
                MetricSpec {
                    name: "fill_rate_pct",
                    baseline: 0.97,
                    trend_per_tick: -0.0002,
                    noise_amplitude: 0.015,
                    crises: vec![
                        Crisis::new(30, 80, -0.25),   // Port disruption
                        Crisis::new(240, 290, -0.18), // Geopolitical shock
                        Crisis::new(400, 430, -0.12),
                    ],
                },
                MetricSpec {
                    name: "lead_time_days",
                    baseline: 14.0,
                    trend_per_tick: 0.01,
                    noise_amplitude: 1.2,
                    crises: vec![
                        Crisis::new(32, 82, 22.0),
                        Crisis::new(242, 292, 16.0),
                    ],
                },
                MetricSpec {
                    name: "inventory_turnover",
                    baseline: 8.5,
                    trend_per_tick: -0.004,
                    noise_amplitude: 0.3,
                    crises: vec![],
                },
                MetricSpec {
                    name: "supplier_concentration_hhi",
                    baseline: 0.22,
                    trend_per_tick: 0.0001,
                    noise_amplitude: 0.01,
                    crises: vec![Crisis::new(235, 295, 0.08)],
                },
            ],
        },

        // ── Water Basin Allocation ─────────────────────────────────────────────
        DomainSpec {
            entity_id: "water_basin",
            metrics: vec![
                MetricSpec {
                    name: "aquifer_recharge_pct",
                    baseline: 0.88,
                    trend_per_tick: -0.0004,
                    noise_amplitude: 0.025,
                    crises: vec![
                        Crisis::new(110, 165, -0.20),  // Multi-year drought
                        Crisis::new(360, 400, -0.15),
                    ],
                },
                MetricSpec {
                    name: "allocation_efficiency_pct",
                    baseline: 0.71,
                    trend_per_tick: 0.0003,
                    noise_amplitude: 0.02,
                    crises: vec![],
                },
                MetricSpec {
                    name: "groundwater_depth_m",
                    baseline: 45.0,
                    trend_per_tick: 0.08,  // Sinking water table
                    noise_amplitude: 0.8,
                    crises: vec![Crisis::new(112, 168, 6.0)],
                },
                MetricSpec {
                    name: "conflict_index",
                    baseline: 0.15,
                    trend_per_tick: 0.0002,
                    noise_amplitude: 0.02,
                    crises: vec![Crisis::new(115, 170, 0.35)],
                },
            ],
        },

        // ── Urban Heat Island Mitigation ───────────────────────────────────────
        DomainSpec {
            entity_id: "urban_heat",
            metrics: vec![
                MetricSpec {
                    name: "urban_rural_delta_c",
                    baseline: 2.8,
                    trend_per_tick: 0.003,
                    noise_amplitude: 0.08,
                    crises: vec![
                        Crisis::new(160, 200, 1.2),   // Extreme heat summer
                        Crisis::new(385, 420, 0.9),
                    ],
                },
                MetricSpec {
                    name: "green_cover_pct",
                    baseline: 0.18,
                    trend_per_tick: -0.0001,  // Urban sprawl
                    noise_amplitude: 0.008,
                    crises: vec![],
                },
                MetricSpec {
                    name: "albedo_idx",
                    baseline: 0.15,
                    trend_per_tick: 0.0001,
                    noise_amplitude: 0.005,
                    crises: vec![],
                },
                MetricSpec {
                    name: "heat_mortality_per_100k",
                    baseline: 1.8,
                    trend_per_tick: 0.003,
                    noise_amplitude: 0.15,
                    crises: vec![
                        Crisis::new(162, 202, 3.5),
                        Crisis::new(387, 422, 2.8),
                    ],
                },
            ],
        },
    ]
}

// ── SignalSimulator ────────────────────────────────────────────────────────────

/// Generates realistic domain-specific signals for all 11 BIOISO entities.
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
    fn simulator_produces_signals_for_all_11_entities() {
        let mut sim = SignalSimulator::new(42);
        let signals = sim.tick(0, 1_000_000);
        // 11 entities × 4 metrics each
        assert_eq!(signals.len(), 44);
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
        let mut sim = SignalSimulator::new(0);
        // Climate CO₂ crisis at ticks 80-110 — expect elevated co2_ppm
        let normal = sim.tick(0, 1_000);
        let mut sim2 = SignalSimulator::new(0);
        // Tick through to crisis zone (must preserve rng state — use same seed for comparison)
        let crisis = {
            let mut s = SignalSimulator::new(0);
            // advance rng to tick 90 by calling tick 0..89
            for t in 0..90u64 {
                s.tick(t, t * 1000);
            }
            s.tick(90, 90_000)
        };
        let normal_co2 = normal
            .iter()
            .find(|s| s.entity_id == "climate" && s.metric == "co2_ppm")
            .unwrap()
            .value;
        let crisis_co2 = crisis
            .iter()
            .find(|s| s.entity_id == "climate" && s.metric == "co2_ppm")
            .unwrap()
            .value;
        // Crisis co2 should be higher: baseline + trend(90) + crisis_spike >> baseline at tick 0
        assert!(crisis_co2 > normal_co2 + 5.0, "crisis={crisis_co2:.2} normal={normal_co2:.2}");
    }

    #[test]
    fn filter_restricts_to_subset() {
        let mut sim = SignalSimulator::new(1).with_filter(vec!["climate".to_string(), "sepsis".to_string()]);
        let signals = sim.tick(0, 1_000);
        assert_eq!(signals.len(), 8); // 2 entities × 4 metrics
        assert!(signals.iter().all(|s| s.entity_id == "climate" || s.entity_id == "sepsis"));
    }

    #[test]
    fn trend_increases_over_time() {
        let mut sim = SignalSimulator::new(7);
        // Co2 must trend up: tick 0 vs tick 400 (average over noise)
        let early: f64 = (0..5u64)
            .map(|t| {
                sim.tick(t, t * 1000)
                    .into_iter()
                    .find(|s| s.entity_id == "climate" && s.metric == "co2_ppm")
                    .unwrap()
                    .value
            })
            .sum::<f64>()
            / 5.0;

        let mut sim2 = SignalSimulator::new(7);
        for t in 0..400u64 { sim2.tick(t, t * 1000); } // advance
        let late: f64 = (400..405u64)
            .map(|t| {
                sim2.tick(t, t * 1000)
                    .into_iter()
                    .find(|s| s.entity_id == "climate" && s.metric == "co2_ppm")
                    .unwrap()
                    .value
            })
            .sum::<f64>()
            / 5.0;

        assert!(late > early + 20.0, "late={late:.2} early={early:.2}");
    }
}
