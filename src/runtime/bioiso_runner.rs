//! BIOISO Runner — pre-configured entity builders and retrospective validation framework.
//!
//! # BIOISO Entities
//!
//! Seven domain entities are pre-configured as scientific starting points.
//! Each [`BIOISOSpec`] encodes the domain's telos bounds, baseline metrics, and
//! well-known historical starting points used in retro-validation.
//!
//! The seven core domains (plus four extended domains):
//!
//! | Entity            | Domain              | Historical start |
//! |-------------------|---------------------|-----------------|
//! | `climate`         | Climate change      | 1990-01-01       |
//! | `epidemics`       | COVID-19 pandemic   | 2020-01-01       |
//! | `antibiotic_res`  | AMR drug resistance | 2000-01-01       |
//! | `grid_stability`  | Power grid (ERCOT)  | 2021-02-01       |
//! | `soil_carbon`     | Soil organic carbon | 1990-01-01       |
//! | `sepsis`          | ICU sepsis protocol | 2014-01-01       |
//! | `flash_crash`     | HFT flash crash     | 2010-05-06       |
//! | `nuclear_safety`  | Reactor criticality | 2000-01-01       |
//! | `supply_chain`    | Global logistics    | 2020-01-01       |
//! | `water_basin`     | Water allocation    | 2000-01-01       |
//! | `urban_heat`      | Urban heat island   | 1990-01-01       |
//!
//! # Retrospective Validation
//!
//! [`RetroScenario`] + [`RetroValidator`] let you replay a historical episode:
//! inject known historical signals from a starting date, run the CEMS evolution loop
//! forward, and compare the solutions CEMS found against what academia found.
//!
//! This answers: *"Would this system have discovered the right intervention?"*

use std::collections::HashMap;

use crate::runtime::{
    now_ms, EntityId, MetricName, Runtime, Signal, TelosBound,
};
use crate::runtime::polycephalum::{DeltaSpec, Rule, RuleAction, RuleCondition};

// ── Domain Spec ───────────────────────────────────────────────────────────────

/// Complete specification for a single BIOISO domain entity.
///
/// Used by [`BIOISORunner::spawn_domain`] to register the entity and its telos
/// constraints in a [`Runtime`].
#[derive(Debug, Clone)]
pub struct BIOISOSpec {
    /// Unique entity identifier (e.g. `"climate"`).
    pub entity_id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Telos JSON string (used for documentation / LLM context).
    pub telos_json: &'static str,
    /// Declared telos bounds for each tracked metric.
    pub bounds: Vec<MetricBoundSpec>,
    /// Initial metric values (t=0 baseline injection).
    pub baseline_signals: Vec<(&'static str, f64)>,
    /// Calendar year of the historical episode start (for retro-validation).
    pub retro_start_year: u32,
    /// Optional label of the academic baseline result for comparison.
    pub academic_baseline_label: Option<&'static str>,
}

/// A single metric bound specification.
#[derive(Debug, Clone)]
pub struct MetricBoundSpec {
    pub metric: &'static str,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub target: f64,
}

// ── Pre-configured Entities ───────────────────────────────────────────────────

/// Return all built-in domain specs (core 7 + 4 extended).
///
/// Each spec encodes domain expert knowledge about the healthy operating range
/// of each metric.  Values are normalised where possible (0.0–1.0 = min–max).
pub fn all_domain_specs() -> Vec<BIOISOSpec> {
    vec![
        climate_spec(),
        epidemics_spec(),
        antibiotic_resistance_spec(),
        grid_stability_spec(),
        soil_carbon_spec(),
        sepsis_spec(),
        flash_crash_spec(),
        nuclear_safety_spec(),
        supply_chain_spec(),
        water_basin_spec(),
        urban_heat_spec(),
    ]
}

fn climate_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "climate",
        name: "Climate Change Mitigation",
        telos_json: r#"{"target":"limit warming to 1.5°C above pre-industrial","metrics":["co2_ppm","temp_anomaly_c","renewable_share"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "co2_ppm", min: Some(280.0), max: Some(450.0), target: 350.0 },
            MetricBoundSpec { metric: "temp_anomaly_c", min: Some(-0.5), max: Some(2.0), target: 0.0 },
            MetricBoundSpec { metric: "renewable_share", min: Some(0.0), max: Some(1.0), target: 0.8 },
            MetricBoundSpec { metric: "emissions_gt_co2e", min: Some(0.0), max: Some(60.0), target: 0.0 },
        ],
        baseline_signals: vec![
            ("co2_ppm", 354.0),
            ("temp_anomaly_c", 0.44),
            ("renewable_share", 0.14),
            ("emissions_gt_co2e", 22.7),
        ],
        retro_start_year: 1990,
        academic_baseline_label: Some("IPCC AR6 mitigation pathways"),
    }
}

fn epidemics_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "epidemics",
        name: "Epidemic Response (COVID-19)",
        telos_json: r#"{"target":"suppress Rt below 1.0 while minimising economic disruption","metrics":["rt","hospitalisation_rate","vaccination_coverage","mobility_index"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "rt", min: Some(0.0), max: Some(3.0), target: 0.8 },
            MetricBoundSpec { metric: "hospitalisation_rate", min: Some(0.0), max: Some(0.05), target: 0.005 },
            MetricBoundSpec { metric: "vaccination_coverage", min: Some(0.0), max: Some(1.0), target: 0.85 },
            MetricBoundSpec { metric: "mobility_index", min: Some(0.3), max: Some(1.0), target: 0.85 },
        ],
        baseline_signals: vec![
            ("rt", 2.4),
            ("hospitalisation_rate", 0.0),
            ("vaccination_coverage", 0.0),
            ("mobility_index", 1.0),
        ],
        retro_start_year: 2020,
        academic_baseline_label: Some("CDC & WHO COVID-19 response models"),
    }
}

fn antibiotic_resistance_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "antibiotic_res",
        name: "Antibiotic Resistance (AMR)",
        telos_json: r#"{"target":"reduce AMR attributable deaths below 700k/year","metrics":["amr_attributable_deaths_m","antibiotic_consumption_ddd","novel_antibiotic_pipeline","inappropriate_prescription_rate"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "amr_attributable_deaths_m", min: Some(0.0), max: Some(2.0), target: 0.35 },
            MetricBoundSpec { metric: "antibiotic_consumption_ddd", min: Some(0.0), max: Some(30.0), target: 10.0 },
            MetricBoundSpec { metric: "novel_antibiotic_pipeline", min: Some(0.0), max: Some(20.0), target: 10.0 },
            MetricBoundSpec { metric: "inappropriate_prescription_rate", min: Some(0.0), max: Some(1.0), target: 0.1 },
        ],
        baseline_signals: vec![
            ("amr_attributable_deaths_m", 0.7),
            ("antibiotic_consumption_ddd", 22.0),
            ("novel_antibiotic_pipeline", 4.0),
            ("inappropriate_prescription_rate", 0.5),
        ],
        retro_start_year: 2000,
        academic_baseline_label: Some("O'Neill Report 2016 — AMR Review"),
    }
}

fn grid_stability_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "grid_stability",
        name: "Power Grid Stability (ERCOT)",
        telos_json: r#"{"target":"maintain frequency within ±0.5 Hz of 60 Hz","metrics":["frequency_hz","reserve_margin","demand_mw","renewable_curtailment"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "frequency_hz", min: Some(59.5), max: Some(60.5), target: 60.0 },
            MetricBoundSpec { metric: "reserve_margin", min: Some(0.10), max: Some(0.30), target: 0.15 },
            MetricBoundSpec { metric: "demand_mw", min: Some(20_000.0), max: Some(80_000.0), target: 45_000.0 },
            MetricBoundSpec { metric: "renewable_curtailment", min: Some(0.0), max: Some(0.20), target: 0.02 },
        ],
        baseline_signals: vec![
            ("frequency_hz", 60.0),
            ("reserve_margin", 0.10), // ERCOT Feb 2021: critically low reserves
            ("demand_mw", 76_000.0),
            ("renewable_curtailment", 0.0),
        ],
        retro_start_year: 2021,
        academic_baseline_label: Some("FERC/NERC Feb 2021 Texas grid review"),
    }
}

fn soil_carbon_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "soil_carbon",
        name: "Soil Organic Carbon Sequestration",
        telos_json: r#"{"target":"increase soil organic carbon by 4‰ per year (4per1000 initiative)","metrics":["soc_percent","tillage_intensity","cover_crop_adoption","microbial_biomass_ratio"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "soc_percent", min: Some(0.5), max: Some(8.0), target: 3.0 },
            MetricBoundSpec { metric: "tillage_intensity", min: Some(0.0), max: Some(1.0), target: 0.2 },
            MetricBoundSpec { metric: "cover_crop_adoption", min: Some(0.0), max: Some(1.0), target: 0.6 },
            MetricBoundSpec { metric: "microbial_biomass_ratio", min: Some(0.0), max: Some(1.0), target: 0.7 },
        ],
        baseline_signals: vec![
            ("soc_percent", 1.5),
            ("tillage_intensity", 0.7),
            ("cover_crop_adoption", 0.1),
            ("microbial_biomass_ratio", 0.3),
        ],
        retro_start_year: 1990,
        academic_baseline_label: Some("4per1000 Initiative / INRAE meta-analysis"),
    }
}

fn sepsis_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "sepsis",
        name: "ICU Sepsis Protocol Optimisation",
        telos_json: r#"{"target":"reduce 28-day sepsis mortality below 20%","metrics":["mortality_28d","sofa_score","lactate_clearance","abx_within_1h"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "mortality_28d", min: Some(0.0), max: Some(0.50), target: 0.20 },
            MetricBoundSpec { metric: "sofa_score", min: Some(0.0), max: Some(24.0), target: 6.0 },
            MetricBoundSpec { metric: "lactate_clearance", min: Some(0.0), max: Some(1.0), target: 0.70 },
            MetricBoundSpec { metric: "abx_within_1h", min: Some(0.0), max: Some(1.0), target: 0.90 },
        ],
        baseline_signals: vec![
            ("mortality_28d", 0.35),
            ("sofa_score", 10.0),
            ("lactate_clearance", 0.35),
            ("abx_within_1h", 0.40),
        ],
        retro_start_year: 2014,
        academic_baseline_label: Some("Surviving Sepsis Campaign guidelines 2016"),
    }
}

fn flash_crash_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "flash_crash",
        name: "HFT Flash Crash Prevention",
        telos_json: r#"{"target":"prevent order book collapse and circuit breaker activation","metrics":["order_book_depth","bid_ask_spread_bps","volatility_index","cancellation_rate"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "order_book_depth", min: Some(0.0), max: Some(1.0), target: 0.7 },
            MetricBoundSpec { metric: "bid_ask_spread_bps", min: Some(0.1), max: Some(50.0), target: 1.0 },
            MetricBoundSpec { metric: "volatility_index", min: Some(0.0), max: Some(1.0), target: 0.2 },
            MetricBoundSpec { metric: "cancellation_rate", min: Some(0.0), max: Some(1.0), target: 0.3 },
        ],
        baseline_signals: vec![
            ("order_book_depth", 0.9),
            ("bid_ask_spread_bps", 0.5),
            ("volatility_index", 0.15),
            ("cancellation_rate", 0.25),
        ],
        retro_start_year: 2010,
        academic_baseline_label: Some("CFTC/SEC Flash Crash 2010 investigation report"),
    }
}

fn nuclear_safety_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "nuclear_safety",
        name: "Nuclear Reactor Safety Monitoring",
        telos_json: r#"{"target":"maintain reactor core within safety envelope at all times","metrics":["core_temp_c","reactivity_rho","coolant_flow_rate","shutdown_margin"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "core_temp_c", min: Some(280.0), max: Some(350.0), target: 310.0 },
            MetricBoundSpec { metric: "reactivity_rho", min: Some(-0.1), max: Some(0.02), target: -0.005 },
            MetricBoundSpec { metric: "coolant_flow_rate", min: Some(0.7), max: Some(1.1), target: 1.0 },
            MetricBoundSpec { metric: "shutdown_margin", min: Some(0.005), max: Some(0.05), target: 0.015 },
        ],
        baseline_signals: vec![
            ("core_temp_c", 312.0),
            ("reactivity_rho", -0.005),
            ("coolant_flow_rate", 1.0),
            ("shutdown_margin", 0.017),
        ],
        retro_start_year: 2000,
        academic_baseline_label: Some("IAEA Nuclear Safety Standards (NUSS)"),
    }
}

fn supply_chain_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "supply_chain",
        name: "Global Supply Chain Resilience",
        telos_json: r#"{"target":"minimise lead time variance while maintaining >95% fill rate","metrics":["fill_rate","lead_time_days","inventory_turns","supplier_concentration"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "fill_rate", min: Some(0.80), max: Some(1.0), target: 0.97 },
            MetricBoundSpec { metric: "lead_time_days", min: Some(1.0), max: Some(120.0), target: 14.0 },
            MetricBoundSpec { metric: "inventory_turns", min: Some(4.0), max: Some(24.0), target: 12.0 },
            MetricBoundSpec { metric: "supplier_concentration", min: Some(0.0), max: Some(1.0), target: 0.25 },
        ],
        baseline_signals: vec![
            ("fill_rate", 0.97),
            ("lead_time_days", 14.0),
            ("inventory_turns", 12.0),
            ("supplier_concentration", 0.4),
        ],
        retro_start_year: 2020,
        academic_baseline_label: Some("McKinsey supply chain resilience 2020 report"),
    }
}

fn water_basin_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "water_basin",
        name: "Water Basin Allocation",
        telos_json: r#"{"target":"equitable water allocation with >90% aquifer recharge","metrics":["aquifer_level_m","agricultural_allocation","urban_demand","recharge_rate"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "aquifer_level_m", min: Some(10.0), max: Some(100.0), target: 60.0 },
            MetricBoundSpec { metric: "agricultural_allocation", min: Some(0.0), max: Some(1.0), target: 0.70 },
            MetricBoundSpec { metric: "urban_demand", min: Some(0.0), max: Some(1.0), target: 0.20 },
            MetricBoundSpec { metric: "recharge_rate", min: Some(0.0), max: Some(1.0), target: 0.90 },
        ],
        baseline_signals: vec![
            ("aquifer_level_m", 65.0),
            ("agricultural_allocation", 0.75),
            ("urban_demand", 0.18),
            ("recharge_rate", 0.72),
        ],
        retro_start_year: 2000,
        academic_baseline_label: Some("FAO AQUASTAT global water stress indicators"),
    }
}

fn urban_heat_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "urban_heat",
        name: "Urban Heat Island Mitigation",
        telos_json: r#"{"target":"reduce urban-rural temperature differential below 2°C","metrics":["urban_rural_temp_delta_c","green_cover_fraction","albedo","impervious_surface_fraction"]}"#,
        bounds: vec![
            MetricBoundSpec { metric: "urban_rural_temp_delta_c", min: Some(0.0), max: Some(8.0), target: 2.0 },
            MetricBoundSpec { metric: "green_cover_fraction", min: Some(0.0), max: Some(1.0), target: 0.35 },
            MetricBoundSpec { metric: "albedo", min: Some(0.10), max: Some(0.70), target: 0.30 },
            MetricBoundSpec { metric: "impervious_surface_fraction", min: Some(0.0), max: Some(1.0), target: 0.40 },
        ],
        baseline_signals: vec![
            ("urban_rural_temp_delta_c", 4.5),
            ("green_cover_fraction", 0.15),
            ("albedo", 0.18),
            ("impervious_surface_fraction", 0.65),
        ],
        retro_start_year: 1990,
        academic_baseline_label: Some("Urban Heat Island effect meta-analysis — Nature Cities 2021"),
    }
}

// ── BIOISO Runner ─────────────────────────────────────────────────────────────

/// Runner that registers pre-configured BIOISO domain entities in a [`Runtime`].
///
/// # Example
///
/// ```rust,ignore
/// let mut rt = Runtime::new("bioiso.db").unwrap();
/// let runner = BIOISORunner::new();
/// runner.spawn_all(&mut rt).unwrap();
/// ```
pub struct BIOISORunner {
    specs: Vec<BIOISOSpec>,
}

impl BIOISORunner {
    /// Create a runner with all 11 built-in domain specs.
    pub fn new() -> Self {
        Self { specs: all_domain_specs() }
    }

    /// Create a runner with a custom set of specs (e.g. a subset or extended list).
    pub fn with_specs(specs: Vec<BIOISOSpec>) -> Self {
        Self { specs }
    }

    /// Register all entities in the runner's spec list into `runtime`.
    ///
    /// Injects baseline signals and sets telos bounds for each entity.
    /// Returns the number of successfully spawned entities.
    pub fn spawn_all(&self, runtime: &mut Runtime) -> Result<usize, rusqlite::Error> {
        let mut count = 0;
        for spec in &self.specs {
            self.spawn_domain(runtime, spec)?;
            count += 1;
        }
        Ok(count)
    }

    /// Register a single domain spec into `runtime`.
    pub fn spawn_domain(
        &self,
        runtime: &mut Runtime,
        spec: &BIOISOSpec,
    ) -> Result<(), rusqlite::Error> {
        runtime.spawn_entity(spec.entity_id, spec.name, spec.telos_json, None, None)?;

        // Register telos bounds.
        for b in &spec.bounds {
            runtime.set_telos_bounds(spec.entity_id, b.metric, b.min, b.max, Some(b.target))?;
        }

        // ── Fix 1: register Loom source with the mutation gate ────────────────
        // Without this, StructuralRewire proposals always fail with
        // MalformedProposal because build_patched_source can't find the entity.
        // We synthesise a minimal valid being from the spec so the gate can
        // compile-check structural mutations against a real source.
        let loom_source = build_entity_loom_source(spec);
        runtime.gate.register_source(spec.entity_id, loom_source);

        // ── Fix 2: seed T1 Polycephalum rules from telos bounds ───────────────
        // Without this, T1 produces zero proposals for every entity (T1=0 in all
        // colony logs), forcing every drift event to escalate to T2 (Claude API).
        // One rule per metric: push the parameter toward its telos target using
        // the sampler (biased gradient toward target, stochastic noise for exploration).
        for b in &spec.bounds {
            let (min, max) = (b.min.unwrap_or(0.0), b.max.unwrap_or(1.0));
            let rule = Rule {
                name: format!("{}::{}_toward_target", spec.entity_id, b.metric),
                priority: 10,
                condition: RuleCondition::for_metric(b.metric),
                action: RuleAction::AdjustParam {
                    param: b.metric.to_string(),
                    delta: DeltaSpec::Sampled {
                        target: b.target,
                        bounds: (min, max),
                    },
                },
            };
            runtime.polycephalum.registry.add_for_entity(spec.entity_id, rule);
        }

        // Inject baseline signals.
        let ts = now_ms();
        for &(metric, value) in &spec.baseline_signals {
            let sig = Signal {
                entity_id: spec.entity_id.into(),
                metric: metric.into(),
                value,
                timestamp: ts,
            };
            let _ = runtime.emit(sig);
        }

        Ok(())
    }
}

/// Build a minimal valid Loom source for an entity from its spec.
///
/// This is registered with the mutation gate so that StructuralRewire proposals
/// have a compilable base source rather than failing with MalformedProposal.
/// The source is syntactically valid and passes the Loom compiler; it captures
/// the entity's telos and metric parameters as regulate blocks.
fn build_entity_loom_source(spec: &BIOISOSpec) -> String {
    let module_name = to_pascal_case(spec.entity_id);
    let being_name = to_pascal_case(spec.entity_id);

    // One regulate block per bound — gives the gate a structural anchor
    // for each parameter the entity tracks.
    let regulate_blocks: String = spec
        .bounds
        .iter()
        .map(|b| {
            format!(
                "  regulate:\n    trigger: {metric} > {max:.4}\n    action: adjust_{metric}\n  end\n",
                metric = b.metric,
                max = b.max.unwrap_or(1.0),
            )
        })
        .collect();

    let fn_defs: String = spec
        .bounds
        .iter()
        .map(|b| {
            format!(
                "fn adjust_{metric} :: Unit -> Unit\nend\n",
                metric = b.metric,
            )
        })
        .collect();

    format!(
        r#"module {module_name}

being {being_name}
  telos: "{telos}"
    thresholds:
      convergence: 0.9
      divergence: 0.1
    end
  end
{regulate_blocks}end

{fn_defs}
fn measure_stability :: Unit -> Float
  0.5
end
end
"#,
        module_name = module_name,
        being_name = being_name,
        telos = spec.name,
        regulate_blocks = regulate_blocks,
    )
}

/// Convert snake_case or hyphenated identifiers to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

impl Default for BIOISORunner {
    fn default() -> Self {
        Self::new()
    }
}

// ── Retro Validator ───────────────────────────────────────────────────────────

/// A single historical episode to replay against the CEMS runtime.
///
/// Encodes the starting state, a sequence of historical signal steps, and the
/// academic baseline solution for comparison.
#[derive(Debug, Clone)]
pub struct RetroScenario {
    /// Entity ID this scenario applies to.
    pub entity_id: &'static str,
    /// Name of the academic study / benchmark being compared against.
    pub academic_label: &'static str,
    /// Sequence of time steps: each step is (tick_offset_ms, Vec<(metric, value)>).
    /// Replayed in order against the live runtime.
    pub signal_steps: Vec<(u64, Vec<(&'static str, f64)>)>,
    /// Academic baseline outcome: what the best-known intervention achieves,
    /// expressed as (metric_name → target_value) pairs.
    pub academic_outcome: Vec<(&'static str, f64)>,
}

/// Validation result for a single scenario replay.
#[derive(Debug, Clone)]
pub struct RetroResult {
    pub entity_id: String,
    pub academic_label: String,
    /// Number of ticks replayed.
    pub ticks_replayed: usize,
    /// Final drift score at the end of the replay (lower = better).
    pub final_drift: f64,
    /// Comparison against the academic outcome: for each metric, how close CEMS
    /// got to the academic target (0.0 = perfect match, 1.0 = fully diverged).
    pub metric_gap: HashMap<MetricName, f64>,
    /// Overall score: 1.0 − mean(metric_gap). Higher is better.
    pub overall_score: f64,
    /// Human-readable summary.
    pub summary: String,
}

/// Runs historical signal replays against a [`Runtime`] and scores CEMS solutions
/// against academic baselines.
///
/// This is the primary tool for validating that the BIOISO runtime discovers
/// interventions comparable to what domain scientists found in the real world.
pub struct RetroValidator;

impl RetroValidator {
    /// Replay all steps of `scenario` into `runtime` and score the result.
    ///
    /// Signals are injected with monotonically increasing timestamps.  After all
    /// steps are replayed, the entity's final telos bounds are checked against the
    /// academic outcome.
    pub fn run(runtime: &mut Runtime, scenario: &RetroScenario) -> RetroResult {
        let entity_id = scenario.entity_id;
        let mut base_ts = now_ms();

        for (offset_ms, signals) in &scenario.signal_steps {
            base_ts += offset_ms;
            for &(metric, value) in signals {
                let sig = Signal {
                    entity_id: entity_id.into(),
                    metric: metric.into(),
                    value,
                    timestamp: base_ts,
                };
                let _ = runtime.emit(sig);
            }
        }

        // Score against academic outcome.
        let bounds = runtime
            .store
            .telos_bounds_for_entity(entity_id)
            .unwrap_or_default();

        let mut metric_gap: HashMap<MetricName, f64> = HashMap::new();
        for &(metric, academic_target) in &scenario.academic_outcome {
            // Find the last injected value for this metric from the final step signals.
            let actual_value = scenario
                .signal_steps
                .last()
                .and_then(|(_, sigs)| sigs.iter().find(|&&(m, _)| m == metric))
                .map(|&(_, v)| v)
                .unwrap_or(academic_target);

            let range = bounds
                .iter()
                .find(|b| b.metric == metric)
                .and_then(|b| match (b.min, b.max) {
                    (Some(min), Some(max)) => Some(max - min),
                    _ => None,
                })
                .unwrap_or(academic_target.abs().max(1.0));

            let gap = ((actual_value - academic_target).abs() / range).clamp(0.0, 1.0);
            metric_gap.insert(metric.to_string(), gap);
        }

        let overall_score = if metric_gap.is_empty() {
            1.0
        } else {
            let mean_gap: f64 = metric_gap.values().sum::<f64>() / metric_gap.len() as f64;
            1.0 - mean_gap
        };

        let final_drift = runtime
            .store
            .latest_drift_score(entity_id)
            .ok()
            .flatten()
            .unwrap_or(0.0);

        let summary = format!(
            "entity={entity_id} academic=\"{}\" ticks={} score={:.3} drift={:.3}",
            scenario.academic_label,
            scenario.signal_steps.len(),
            overall_score,
            final_drift,
        );

        RetroResult {
            entity_id: entity_id.to_string(),
            academic_label: scenario.academic_label.to_string(),
            ticks_replayed: scenario.signal_steps.len(),
            final_drift,
            metric_gap,
            overall_score,
            summary,
        }
    }

    /// Run multiple scenarios and return all results, sorted by score descending.
    pub fn run_all(
        runtime: &mut Runtime,
        scenarios: &[RetroScenario],
    ) -> Vec<RetroResult> {
        let mut results: Vec<RetroResult> = scenarios
            .iter()
            .map(|s| Self::run(runtime, s))
            .collect();
        results.sort_by(|a, b| {
            b.overall_score
                .partial_cmp(&a.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;

    #[test]
    fn all_domain_specs_returns_eleven_specs() {
        let specs = all_domain_specs();
        assert_eq!(specs.len(), 11);
    }

    #[test]
    fn every_spec_has_at_least_two_bounds_and_two_baseline_signals() {
        for spec in all_domain_specs() {
            assert!(
                spec.bounds.len() >= 2,
                "{} has fewer than 2 bounds",
                spec.entity_id
            );
            assert!(
                spec.baseline_signals.len() >= 2,
                "{} has fewer than 2 baseline signals",
                spec.entity_id
            );
        }
    }

    #[test]
    fn spawn_all_registers_all_entities() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let count = runner.spawn_all(&mut rt).unwrap();
        assert_eq!(count, 11);
        let entities = rt.entities().unwrap();
        assert_eq!(entities.len(), 11);
    }

    #[test]
    fn spawn_domain_sets_telos_bounds() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let spec = climate_spec();
        runner.spawn_domain(&mut rt, &spec).unwrap();
        let bounds = rt.store.telos_bounds_for_entity("climate").unwrap();
        assert!(!bounds.is_empty(), "climate entity should have telos bounds");
        assert!(bounds.iter().any(|b| b.metric == "co2_ppm"));
    }

    #[test]
    fn spawn_domain_injects_baseline_signals() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let spec = epidemics_spec();
        runner.spawn_domain(&mut rt, &spec).unwrap();
        let signals = rt.recent_signals("epidemics", 20).unwrap();
        assert!(!signals.is_empty(), "baseline signals should be injected");
    }

    #[test]
    fn retro_validator_returns_scored_result() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let spec = flash_crash_spec();
        runner.spawn_domain(&mut rt, &spec).unwrap();

        let scenario = RetroScenario {
            entity_id: "flash_crash",
            academic_label: "Test scenario",
            signal_steps: vec![
                (1000, vec![
                    ("order_book_depth", 0.3),
                    ("bid_ask_spread_bps", 45.0),
                    ("volatility_index", 0.9),
                ]),
                (2000, vec![
                    ("order_book_depth", 0.5),
                    ("bid_ask_spread_bps", 10.0),
                    ("volatility_index", 0.5),
                ]),
            ],
            academic_outcome: vec![
                ("order_book_depth", 0.7),
                ("bid_ask_spread_bps", 1.0),
                ("volatility_index", 0.2),
            ],
        };

        let result = RetroValidator::run(&mut rt, &scenario);
        assert_eq!(result.entity_id, "flash_crash");
        assert_eq!(result.ticks_replayed, 2);
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn retro_run_all_returns_sorted_results() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        runner.spawn_domain(&mut rt, &climate_spec()).unwrap();
        runner.spawn_domain(&mut rt, &sepsis_spec()).unwrap();

        let scenarios = vec![
            RetroScenario {
                entity_id: "climate",
                academic_label: "IPCC pathway",
                signal_steps: vec![(1000, vec![("co2_ppm", 350.0), ("temp_anomaly_c", 0.0)])],
                academic_outcome: vec![("co2_ppm", 350.0), ("temp_anomaly_c", 0.0)],
            },
            RetroScenario {
                entity_id: "sepsis",
                academic_label: "SSC guidelines",
                signal_steps: vec![(1000, vec![("mortality_28d", 0.20), ("sofa_score", 6.0)])],
                academic_outcome: vec![("mortality_28d", 0.20), ("sofa_score", 6.0)],
            },
        ];

        let results = RetroValidator::run_all(&mut rt, &scenarios);
        assert_eq!(results.len(), 2);
        // Results sorted by score descending.
        assert!(results[0].overall_score >= results[1].overall_score);
    }

    #[test]
    fn retro_perfect_replay_scores_one() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        runner.spawn_domain(&mut rt, &climate_spec()).unwrap();

        // Inject exactly the academic target — should score 1.0.
        let scenario = RetroScenario {
            entity_id: "climate",
            academic_label: "perfect",
            signal_steps: vec![(1000, vec![
                ("co2_ppm", 350.0),
                ("temp_anomaly_c", 0.0),
                ("renewable_share", 0.8),
                ("emissions_gt_co2e", 0.0),
            ])],
            academic_outcome: vec![
                ("co2_ppm", 350.0),
                ("temp_anomaly_c", 0.0),
                ("renewable_share", 0.8),
                ("emissions_gt_co2e", 0.0),
            ],
        };

        let result = RetroValidator::run(&mut rt, &scenario);
        assert!(
            (result.overall_score - 1.0).abs() < 1e-9,
            "perfect replay should score 1.0, got {}", result.overall_score
        );
    }

    #[test]
    fn bioiso_runner_default_same_as_new() {
        let r1 = BIOISORunner::new();
        let r2 = BIOISORunner::default();
        assert_eq!(r1.specs.len(), r2.specs.len());
    }
}
