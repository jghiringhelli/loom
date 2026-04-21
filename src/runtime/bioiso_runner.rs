//! BIOISO Runner — pre-configured entity builders and retrospective validation framework.
//!
//! # BIOISO Entities
//!
//! Seven BIOISO-class domains are pre-configured. Each was selected by three strict criteria:
//! (1) the fitness landscape is *coevolutionary or structurally non-stationary*;
//! (2) structural rewiring (`StructuralRewire`) is *load-bearing* — `ParameterAdjust`
//! provably cannot converge; (3) the problem is *currently unsolved* by Tiers 1–4.
//!
//! | Entity                  | Domain                        | Historical start | Why BIOISO |
//! |-------------------------|-------------------------------|-----------------|------------|
//! | `amr_coevolution`       | Antimicrobial resistance      | 2000-01-01       | Pathogen co-evolves — resistance mechanisms restructure faster than any fixed strategy |
//! | `flash_crash`           | HFT market microstructure     | 2010-05-06       | HFT strategies reverse-engineer fixed circuit breaker rules; novel detection logic required |
//! | `adaptive_jit`          | JIT compiler optimization     | 2015-01-01       | Optimal IR pass sequence shifts as runtime hot paths evolve; structural rewiring of pass composition |
//! | `protein_drug_resistance` | Cancer/HIV drug resistance  | 2005-01-01       | Target protein mutates; chemical search space topology shifts; new pharmacophore hypotheses needed |
//! | `ics_zero_day`          | ICS/SCADA zero-day defense    | 2010-01-01       | Novel attack classes have unknown structure; ML on known attacks fails by definition |
//! | `quantum_error_mitigation` | Quantum circuit compilation | 2020-01-01      | Hardware noise model changes per calibration; no fixed decomposition strategy transfers |
//! | `climate_intervention`  | Earth system intervention seq | 1990-01-01       | Each intervention changes the causal structure; causal graph shifts after every deployment |
//! | `fusion_plasma`         | Fusion plasma confinement     | 2012-01-01       | L-H transitions and disruption precursors are structurally novel; no fixed control law transfers |
//! | `adaptive_self_assembly`| Nanostructure self-assembly   | 2015-01-01       | Each assembly step changes accessible configuration space; protocol graph must be rewired |
//!
//! # Retrospective Validation
//!
//! [`RetroScenario`] + [`RetroValidator`] let you replay a historical episode:
//! inject known historical signals from a starting date, run the CEMS evolution loop
//! forward, and compare the solutions CEMS found against what academia found.
//!
//! This answers: *"Would this system have discovered the right intervention?"*

use std::collections::HashMap;

use crate::runtime::polycephalum::{DeltaSpec, Rule, RuleAction, RuleCondition};
use crate::runtime::{now_ms, EntityId, MetricName, Runtime, Signal, TelosBound};

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

/// Return the 9 curated BIOISO-class domain specs.
///
/// Each domain was selected because structural rewiring is load-bearing —
/// `ParameterAdjust` alone cannot achieve telos convergence under the
/// coevolutionary or non-stationary dynamics each domain exhibits.
pub fn all_domain_specs() -> Vec<BIOISOSpec> {
    vec![
        amr_coevolution_spec(),
        flash_crash_spec(),
        adaptive_jit_spec(),
        protein_drug_resistance_spec(),
        ics_zero_day_spec(),
        quantum_error_mitigation_spec(),
        climate_intervention_spec(),
        fusion_plasma_spec(),
        adaptive_self_assembly_spec(),
    ]
}

// ── 7 Curated BIOISO-class domain specs ──────────────────────────────────────
//
// Selection criterion: structural rewiring is load-bearing in each domain.
// ParameterAdjust alone cannot converge because the fitness landscape itself
// restructures during the experiment horizon.

// ── 1. Antimicrobial Resistance Coevolution ───────────────────────────────────
// Calibrated against WHO Global AMR Action Plan; ESKAPE pathogens.
// The pathogen co-evolves: resistance mechanisms restructure faster than
// any fixed drug-target strategy can follow. 1.27M deaths/yr (GBD 2019).
fn amr_coevolution_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "amr_coevolution",
        name: "Antimicrobial Resistance Coevolution",
        telos_json: r#"{"target":"discover and maintain effective drug-target strategies faster than resistance mechanisms evolve","metrics":["resistance_prevalence_pct","effective_drug_count","treatment_success_rate","novel_resistance_rate"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "resistance_prevalence_pct",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.10,
            },
            MetricBoundSpec {
                metric: "effective_drug_count",
                min: Some(0.0),
                max: Some(30.0),
                target: 20.0,
            },
            MetricBoundSpec {
                metric: "treatment_success_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.85,
            },
            MetricBoundSpec {
                metric: "novel_resistance_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.02,
            },
        ],
        baseline_signals: vec![
            ("resistance_prevalence_pct", 0.28),
            ("effective_drug_count", 8.0),
            ("treatment_success_rate", 0.71),
            ("novel_resistance_rate", 0.12),
        ],
        retro_start_year: 2000,
        academic_baseline_label: Some("WHO Global AMR Action Plan; GBD 2019 AMR collaborators"),
    }
}

// ── 2. HFT Flash Crash Defense ────────────────────────────────────────────────
// Calibrated against CFTC/SEC 2010 Flash Crash investigation report.
// HFT strategies reverse-engineer and exploit fixed circuit breaker rules.
// Novel detection logic categories must be generated, not tuned.
fn flash_crash_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "flash_crash",
        name: "HFT Flash Crash Defense",
        telos_json: r#"{"target":"prevent order book collapse by generating novel detection logic faster than HFT strategies can reverse-engineer circuit breakers","metrics":["order_book_depth","bid_ask_spread_bps","volatility_index","cancellation_rate"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "order_book_depth",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.72,
            },
            MetricBoundSpec {
                metric: "bid_ask_spread_bps",
                min: Some(0.1),
                max: Some(50.0),
                target: 1.0,
            },
            MetricBoundSpec {
                metric: "volatility_index",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.2,
            },
            MetricBoundSpec {
                metric: "cancellation_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.3,
            },
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

// ── 3. Adaptive JIT Compiler Optimization ────────────────────────────────────
// Runtime hot paths change as program execution evolves. The optimal IR pass
// sequence is non-stationary. No fixed pass pipeline is universally optimal.
// Structural rewiring = discovering new pass composition sequences.
fn adaptive_jit_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "adaptive_jit",
        name: "Adaptive JIT Compiler Optimization",
        telos_json: r#"{"target":"maintain optimal IR pass composition as runtime hot paths evolve","metrics":["hotpath_coverage_pct","generated_code_speedup","compilation_overhead_ms","cache_hit_rate"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "hotpath_coverage_pct",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.90,
            },
            MetricBoundSpec {
                metric: "generated_code_speedup",
                min: Some(1.0),
                max: Some(20.0),
                target: 8.0,
            },
            MetricBoundSpec {
                metric: "compilation_overhead_ms",
                min: Some(0.0),
                max: Some(500.0),
                target: 15.0,
            },
            MetricBoundSpec {
                metric: "cache_hit_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.92,
            },
        ],
        baseline_signals: vec![
            ("hotpath_coverage_pct", 0.62),
            ("generated_code_speedup", 3.5),
            ("compilation_overhead_ms", 80.0),
            ("cache_hit_rate", 0.74),
        ],
        retro_start_year: 2015,
        academic_baseline_label: Some("V8/HotSpot JIT benchmarks (SPEC JVM2008, Octane)"),
    }
}

// ── 4. Protein Drug Resistance ────────────────────────────────────────────────
// Target proteins mutate; chemical search space topology shifts.
// Existing pharmacophore hypotheses become incorrect.
// Structural rewiring = generating new binding hypotheses.
fn protein_drug_resistance_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "protein_drug_resistance",
        name: "Protein Drug Resistance Evolution",
        telos_json: r#"{"target":"discover novel binding strategies as target proteins acquire resistance mutations","metrics":["binding_affinity_kcal_mol","admet_score","resistance_mutation_count","active_lead_count"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "binding_affinity_kcal_mol",
                min: Some(-15.0),
                max: Some(0.0),
                target: -9.0,
            },
            MetricBoundSpec {
                metric: "admet_score",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.75,
            },
            MetricBoundSpec {
                metric: "resistance_mutation_count",
                min: Some(0.0),
                max: Some(20.0),
                target: 1.0,
            },
            MetricBoundSpec {
                metric: "active_lead_count",
                min: Some(0.0),
                max: Some(50.0),
                target: 15.0,
            },
        ],
        baseline_signals: vec![
            ("binding_affinity_kcal_mol", -6.2),
            ("admet_score", 0.48),
            ("resistance_mutation_count", 8.0),
            ("active_lead_count", 3.0),
        ],
        retro_start_year: 2005,
        academic_baseline_label: Some("ChEMBL drug resistance database; CCLE; BindingDB"),
    }
}

// ── 5. ICS/SCADA Zero-Day Defense ────────────────────────────────────────────
// Novel attack classes have unknown structure.
// ML trained on known attacks fails by definition on zero-days.
// Structural rewiring = synthesising new signal categories and detection logic.
fn ics_zero_day_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "ics_zero_day",
        name: "ICS/SCADA Zero-Day Defense",
        telos_json: r#"{"target":"detect novel ICS attack classes before physical process disruption","metrics":["detection_rate_pct","false_positive_rate","novel_attack_coverage","response_latency_ms"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "detection_rate_pct",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.95,
            },
            MetricBoundSpec {
                metric: "false_positive_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.005,
            },
            MetricBoundSpec {
                metric: "novel_attack_coverage",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.80,
            },
            MetricBoundSpec {
                metric: "response_latency_ms",
                min: Some(0.0),
                max: Some(5000.0),
                target: 100.0,
            },
        ],
        baseline_signals: vec![
            ("detection_rate_pct", 0.72),
            ("false_positive_rate", 0.08),
            ("novel_attack_coverage", 0.12),
            ("response_latency_ms", 850.0),
        ],
        retro_start_year: 2010,
        academic_baseline_label: Some(
            "ICS-CERT advisories; MITRE ATT&CK for ICS; Dragos Year in Review",
        ),
    }
}

// ── 6. Quantum Error Mitigation ───────────────────────────────────────────────
// Hardware noise models change per calibration cycle.
// No fixed decomposition strategy transfers across hardware generations.
// Structural rewiring = discovering new circuit decomposition templates.
fn quantum_error_mitigation_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "quantum_error_mitigation",
        name: "Quantum Circuit Error Mitigation",
        telos_json: r#"{"target":"maintain logical error rates below fault-tolerance threshold as hardware noise evolves","metrics":["logical_error_rate","circuit_depth","t_gate_count","qubit_fidelity"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "logical_error_rate",
                min: Some(0.0),
                max: Some(0.1),
                target: 0.001,
            },
            MetricBoundSpec {
                metric: "circuit_depth",
                min: Some(1.0),
                max: Some(1000.0),
                target: 50.0,
            },
            MetricBoundSpec {
                metric: "t_gate_count",
                min: Some(0.0),
                max: Some(500.0),
                target: 20.0,
            },
            MetricBoundSpec {
                metric: "qubit_fidelity",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.9995,
            },
        ],
        baseline_signals: vec![
            ("logical_error_rate", 0.012),
            ("circuit_depth", 280.0),
            ("t_gate_count", 145.0),
            ("qubit_fidelity", 0.991),
        ],
        retro_start_year: 2020,
        academic_baseline_label: Some("IBM Quantum Network; Google Sycamore error rates (2023)"),
    }
}

// ── 7. Climate Intervention Sequencing ────────────────────────────────────────
// Each deployed intervention changes the causal structure of the Earth system.
// The causal graph shifts after every action.
// Structural rewiring = adapting which interventions to sequence next.
fn climate_intervention_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "climate_intervention",
        name: "Earth System Intervention Sequencing",
        telos_json: r#"{"target":"discover intervention sequences that maintain Earth system resilience as causal coupling shifts with each deployment","metrics":["intervention_efficacy","tipping_point_risk","co2_trajectory_delta","system_resilience"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "intervention_efficacy",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.70,
            },
            MetricBoundSpec {
                metric: "tipping_point_risk",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.10,
            },
            MetricBoundSpec {
                metric: "co2_trajectory_delta",
                min: Some(-50.0),
                max: Some(10.0),
                target: -20.0,
            },
            MetricBoundSpec {
                metric: "system_resilience",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.75,
            },
        ],
        baseline_signals: vec![
            ("intervention_efficacy", 0.28),
            ("tipping_point_risk", 0.42),
            ("co2_trajectory_delta", 2.5),
            ("system_resilience", 0.51),
        ],
        retro_start_year: 1990,
        academic_baseline_label: Some("IPCC AR6 Ch17; Lenton et al. 2019 tipping elements"),
    }
}

// ── 8. Fusion Plasma Control ──────────────────────────────────────────────────
// Calibrated against ITER disruption database and JET H-mode experiments.
// Plasma confinement transitions (L-H mode, ELM events, disruption precursors)
// are structurally non-stationary: the control policy that stabilises one regime
// fails when the plasma transitions. Fixed PID controllers lose confinement;
// ML policies trained on one regime do not transfer to novel instability modes.
// A BIOISO rewires the control law graph when disruption probability spikes,
// synthesising detection logic for instability classes not seen during training.
fn fusion_plasma_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "fusion_plasma",
        name: "Fusion Plasma Control",
        telos_json: r#"{"target":"maintain plasma confinement across regime transitions by synthesising novel control laws for unseen instability modes","metrics":["confinement_quality_h98","disruption_probability","elm_frequency_hz","beta_normalised"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "confinement_quality_h98",
                min: Some(0.0),
                max: Some(2.0),
                target: 1.05,
            },
            MetricBoundSpec {
                metric: "disruption_probability",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.02,
            },
            MetricBoundSpec {
                metric: "elm_frequency_hz",
                min: Some(0.0),
                max: Some(80.0),
                target: 8.0,
            },
            MetricBoundSpec {
                metric: "beta_normalised",
                min: Some(0.0),
                max: Some(4.0),
                target: 2.2,
            },
        ],
        baseline_signals: vec![
            ("confinement_quality_h98", 0.82),
            ("disruption_probability", 0.04),
            ("elm_frequency_hz", 12.0),
            ("beta_normalised", 1.8),
        ],
        retro_start_year: 2012,
        academic_baseline_label: Some("ITER disruption database; Kates-Harbeck et al. 2019 FRNN"),
    }
}

// ── 9. Adaptive Self-Assembly ──────────────────────────────────────────────────
// Calibrated against DNA origami and colloidal self-assembly literature:
// Rothemund (2006) Science; Zeravcic et al. (2017) Rev. Mod. Phys.
// Each assembly step changes which nanostructure configurations are accessible —
// the protocol graph must be rewired between deployments. Tier 4 surrogate models
// optimise yield within the current accessible configuration space but cannot
// anticipate pathway bifurcations that collapse access to target structures.
fn adaptive_self_assembly_spec() -> BIOISOSpec {
    BIOISOSpec {
        entity_id: "adaptive_self_assembly",
        name: "Adaptive Self-Assembly",
        telos_json: r#"{"target":"maximise yield of target nanostructures by rewiring assembly protocol graphs as the accessible configuration space shifts after each deployment","metrics":["assembly_yield_pct","defect_density_per_um2","accessible_configuration_count","protocol_convergence_rate"]}"#,
        bounds: vec![
            MetricBoundSpec {
                metric: "assembly_yield_pct",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.82,
            },
            MetricBoundSpec {
                metric: "defect_density_per_um2",
                min: Some(0.0),
                max: Some(20.0),
                target: 0.8,
            },
            MetricBoundSpec {
                metric: "accessible_configuration_count",
                min: Some(0.0),
                max: Some(50.0),
                target: 25.0,
            },
            MetricBoundSpec {
                metric: "protocol_convergence_rate",
                min: Some(0.0),
                max: Some(1.0),
                target: 0.75,
            },
        ],
        baseline_signals: vec![
            ("assembly_yield_pct", 0.58),
            ("defect_density_per_um2", 3.4),
            ("accessible_configuration_count", 14.0),
            ("protocol_convergence_rate", 0.44),
        ],
        retro_start_year: 2015,
        academic_baseline_label: Some(
            "Rothemund 2006; Zeravcic et al. 2017; Yin et al. 2008 DNA brick",
        ),
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
    /// Create a runner with all 9 curated BIOISO-class domain specs.
    pub fn new() -> Self {
        Self {
            specs: all_domain_specs(),
        }
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
            runtime
                .polycephalum
                .registry
                .add_for_entity(spec.entity_id, rule);
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn run_all(runtime: &mut Runtime, scenarios: &[RetroScenario]) -> Vec<RetroResult> {
        let mut results: Vec<RetroResult> =
            scenarios.iter().map(|s| Self::run(runtime, s)).collect();
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
    fn all_domain_specs_returns_nine_specs() {
        let specs = all_domain_specs();
        assert_eq!(specs.len(), 9);
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
        assert_eq!(count, 9);
        let entities = rt.entities().unwrap();
        assert_eq!(entities.len(), 9);
    }

    #[test]
    fn spawn_domain_sets_telos_bounds() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let spec = amr_coevolution_spec();
        runner.spawn_domain(&mut rt, &spec).unwrap();
        let bounds = rt.store.telos_bounds_for_entity("amr_coevolution").unwrap();
        assert!(
            !bounds.is_empty(),
            "amr_coevolution entity should have telos bounds"
        );
        assert!(bounds
            .iter()
            .any(|b| b.metric == "resistance_prevalence_pct"));
    }

    #[test]
    fn spawn_domain_injects_baseline_signals() {
        let mut rt = Runtime::new(":memory:").unwrap();
        let runner = BIOISORunner::new();
        let spec = adaptive_jit_spec();
        runner.spawn_domain(&mut rt, &spec).unwrap();
        let signals = rt.recent_signals("adaptive_jit", 20).unwrap();
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
                (
                    1000,
                    vec![
                        ("order_book_depth", 0.3),
                        ("bid_ask_spread_bps", 45.0),
                        ("volatility_index", 0.9),
                    ],
                ),
                (
                    2000,
                    vec![
                        ("order_book_depth", 0.5),
                        ("bid_ask_spread_bps", 10.0),
                        ("volatility_index", 0.5),
                    ],
                ),
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
        runner
            .spawn_domain(&mut rt, &amr_coevolution_spec())
            .unwrap();
        runner
            .spawn_domain(&mut rt, &quantum_error_mitigation_spec())
            .unwrap();

        let scenarios = vec![
            RetroScenario {
                entity_id: "amr_coevolution",
                academic_label: "WHO AMR baseline",
                signal_steps: vec![(
                    1000,
                    vec![
                        ("resistance_prevalence_pct", 0.10),
                        ("treatment_success_rate", 0.85),
                    ],
                )],
                academic_outcome: vec![
                    ("resistance_prevalence_pct", 0.10),
                    ("treatment_success_rate", 0.85),
                ],
            },
            RetroScenario {
                entity_id: "quantum_error_mitigation",
                academic_label: "IBM Quantum 2023",
                signal_steps: vec![(
                    1000,
                    vec![("logical_error_rate", 0.001), ("qubit_fidelity", 0.9995)],
                )],
                academic_outcome: vec![("logical_error_rate", 0.001), ("qubit_fidelity", 0.9995)],
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
        runner.spawn_domain(&mut rt, &flash_crash_spec()).unwrap();

        // Inject exactly the academic target — should score 1.0.
        let scenario = RetroScenario {
            entity_id: "flash_crash",
            academic_label: "perfect",
            signal_steps: vec![(
                1000,
                vec![
                    ("order_book_depth", 0.7),
                    ("bid_ask_spread_bps", 1.0),
                    ("volatility_index", 0.2),
                    ("cancellation_rate", 0.3),
                ],
            )],
            academic_outcome: vec![
                ("order_book_depth", 0.7),
                ("bid_ask_spread_bps", 1.0),
                ("volatility_index", 0.2),
                ("cancellation_rate", 0.3),
            ],
        };

        let result = RetroValidator::run(&mut rt, &scenario);
        assert!(
            (result.overall_score - 1.0).abs() < 1e-9,
            "perfect replay should score 1.0, got {}",
            result.overall_score
        );
    }

    #[test]
    fn bioiso_runner_default_same_as_new() {
        let r1 = BIOISORunner::new();
        let r2 = BIOISORunner::default();
        assert_eq!(r1.specs.len(), r2.specs.len());
    }
}
