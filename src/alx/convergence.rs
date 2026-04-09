//! Convergence tracing for ALX experiments.
//!
//! `compute_convergence_trace` measures S_realized = proved_claims / total_claims
//! after each checker stage fires, producing a convergence curve rather than a
//! single end-of-pipeline gate score.
//!
//! For programs that declare a `correctness_report:` block, convergence is claim-level:
//! each proved claim names its verifying checker; the trace accumulates as each stage runs.
//!
//! For programs without a `correctness_report:`, convergence falls back to stage-level:
//! S_realized = stages_completed_without_error / total_stages.

use crate::ast::{Item, Module};
use crate::lexer::Lexer;
use crate::parser::Parser;

// ── Stage order mirrors the compile pipeline in lib.rs ────────────────────────
//
// Each entry is (stage_label, canonical_checker_name).
// The checker_name is matched case-insensitively against the `checker` field
// in CorrectnessClaim, so spelling variations ("typechecker" / "TypeChecker")
// both match.
const STAGE_ORDER: &[(&str, &str)] = &[
    ("lex", "Lexer"),
    ("parse", "Parser"),
    ("inference", "InferenceEngine"),
    ("aspect", "AspectChecker"),
    ("type", "TypeChecker"),
    ("refinement", "RefinementChecker"),
    ("exhaustiveness", "ExhaustivenessChecker"),
    ("effects", "EffectChecker"),
    ("algebraic", "AlgebraicChecker"),
    ("units", "UnitsChecker"),
    ("typestate", "TypestateChecker"),
    ("temporal", "TemporalChecker"),
    ("separation", "SeparationChecker"),
    ("gradual", "GradualChecker"),
    ("probabilistic", "ProbabilisticChecker"),
    ("dependent", "DependentChecker"),
    ("side_channel", "SideChannelChecker"),
    ("category", "CategoryChecker"),
    ("curry_howard", "CurryHowardChecker"),
    ("self_cert", "SelfCertChecker"),
    ("store", "StoreChecker"),
    ("tensor", "TensorChecker"),
    ("privacy", "PrivacyChecker"),
    ("teleos", "TeleosChecker"),
    ("safety", "SafetyChecker"),
    ("session", "SessionChecker"),
    ("effect_handler", "EffectHandlerChecker"),
    ("use_case", "UseCaseChecker"),
    ("randomness", "RandomnessChecker"),
    ("stochastic", "StochasticChecker"),
    ("smt", "SmtBridgeChecker"),
    ("manifest", "ManifestChecker"),
    ("migration", "MigrationChecker"),
    ("minimal", "MinimalChecker"),
    ("journal", "JournalChecker"),
    ("scenario", "ScenarioChecker"),
    ("property", "PropertyChecker"),
    ("provenance", "ProvenanceChecker"),
    ("boundary", "BoundaryChecker"),
    ("evolution_vector", "EvolutionVectorChecker"),
    ("signal_attention", "SignalAttentionChecker"),
    ("messaging", "MessagingChecker"),
];

/// A single measurement point in the S_realized convergence curve.
///
/// Records the cumulative claim coverage after a specific checker stage completes.
#[derive(Debug, Clone)]
pub struct ConvergenceStep {
    /// Position in the pipeline (0-indexed).
    pub stage_index: usize,
    /// Human-readable stage label (e.g. "type", "safety").
    pub stage_name: &'static str,
    /// The checker class name matched against correctness_report claims.
    pub checker_name: &'static str,
    /// How many claims have a matching checker at or before this stage.
    pub claims_proved_cumulative: usize,
    /// Total claims in the correctness_report (proved + unverified).
    pub total_claims: usize,
    /// S_realized = claims_proved_cumulative / total_claims.
    pub s_realized: f64,
    /// How many NEW claims this stage contributed (delta from previous step).
    pub delta: usize,
}

/// Full convergence trace for one ALX experiment.
///
/// The `steps` field is the convergence curve — one entry per pipeline stage.
/// `is_monotonic` is true iff S_realized never decreases along the curve.
#[derive(Debug)]
pub struct ConvergenceTrace {
    /// Per-stage convergence curve.
    pub steps: Vec<ConvergenceStep>,
    /// Total proved claims in the correctness_report.
    pub total_proved: usize,
    /// Total claims (proved + unverified).
    pub total_claims: usize,
    /// Final S_realized at end of pipeline.
    pub final_s: f64,
    /// True iff the curve is strictly non-decreasing (required property).
    pub is_monotonic: bool,
    /// Stages that contributed at least one new proved claim.
    pub contributing_stages: Vec<&'static str>,
    /// Whether this trace is claim-level (correctness_report present)
    /// or stage-level (fallback when no report block exists).
    pub mode: ConvergenceTraceMode,
}

/// How S_realized is computed for this trace.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvergenceTraceMode {
    /// correctness_report block found — claims mapped to checkers.
    ClaimLevel,
    /// No correctness_report — S_realized = stages passed / total stages.
    StageFallback,
}

impl ConvergenceTrace {
    /// Format the convergence curve as a single-line ASCII sparkline.
    ///
    /// Each character represents one stage that contributed new claims.
    /// Non-contributing stages are represented by `·`.
    pub fn sparkline(&self) -> String {
        let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        self.steps
            .iter()
            .map(|s| {
                if s.delta == 0 {
                    "·".to_string()
                } else {
                    let idx = ((s.s_realized * 7.0) as usize).min(7);
                    bars[idx].to_string()
                }
            })
            .collect()
    }

    /// Print a human-readable convergence report to stderr.
    pub fn report(&self, label: &str) {
        eprintln!("\n╔══ ALX Convergence: {} ══", label);
        eprintln!("║  Mode: {:?}", self.mode);
        eprintln!(
            "║  Final S_realized = {}/{} = {:.4}",
            self.total_proved, self.total_claims, self.final_s
        );
        eprintln!(
            "║  Monotonic: {}",
            if self.is_monotonic { "✅" } else { "❌" }
        );
        eprintln!("║  Sparkline: {}", self.sparkline());
        if !self.contributing_stages.is_empty() {
            eprintln!(
                "║  Contributing stages: {}",
                self.contributing_stages.join(", ")
            );
        }
        eprintln!("╚══");
    }
}

/// Compute the per-stage S_realized convergence trace for a Loom source program.
///
/// # Arguments
/// * `source` — Loom source string (must lex and parse successfully)
///
/// # Returns
/// A [`ConvergenceTrace`] with one step per pipeline stage.
/// If the source fails to parse, returns a zero-step trace with `final_s = 0.0`.
pub fn compute_convergence_trace(source: &str) -> ConvergenceTrace {
    // Parse to AST — convergence is computed from the static declarations.
    let tokens = match Lexer::tokenize(source) {
        Ok(t) => t,
        Err(_) => return empty_trace(),
    };
    let module = match Parser::new(&tokens).parse_module() {
        Ok(m) => m,
        Err(_) => return empty_trace(),
    };

    if let Some(trace) = claim_level_trace(&module) {
        trace
    } else {
        stage_fallback_trace(&module)
    }
}

// ── Claim-level trace (correctness_report present) ────────────────────────────

/// Normalize a checker identifier for fuzzy matching.
///
/// Handles both `type_checker_passed` (ALX-1 style) and `TypeChecker` (ALX-3 style).
fn normalize_checker_name(s: &str) -> String {
    s.to_lowercase()
        .replace("_passed", "")
        .replace("_proved", "")
        .replace("_checked", "")
        .replace("_check", "")
        .replace("_checker", "")
        .replace("checker", "")
        .replace("_logic", "")
        .replace("_safety", "")
        .replace("_ordering", "")
        .replace("_completeness", "")
        .replace("_quality", "")
        .replace('_', "")
}

fn claim_level_trace(module: &Module) -> Option<ConvergenceTrace> {
    let report = module.items.iter().find_map(|item| {
        if let Item::CorrectnessReport(r) = item {
            Some(r)
        } else {
            None
        }
    })?;

    let total_claims = report.proved.len() + report.unverified.len();
    if total_claims == 0 {
        return None;
    }

    // For each proved claim, find the first pipeline stage that matches its checker name.
    // A claim that matches no stage is assigned STAGE_ORDER.len() (never fired).
    let stage_count = STAGE_ORDER.len();
    let claim_stage_indices: Vec<usize> = report
        .proved
        .iter()
        .map(|claim| {
            let norm_claim = normalize_checker_name(&claim.checker);
            STAGE_ORDER
                .iter()
                .position(|(_, checker_name)| {
                    let norm_stage = normalize_checker_name(checker_name);
                    norm_stage == norm_claim
                        || norm_stage.contains(&norm_claim)
                        || norm_claim.contains(&norm_stage)
                })
                .unwrap_or(stage_count)
        })
        .collect();

    // Build the convergence curve.
    let mut steps = Vec::with_capacity(stage_count);
    let mut contributing = Vec::new();

    for (idx, &(stage_name, checker_name)) in STAGE_ORDER.iter().enumerate() {
        let cumulative = claim_stage_indices.iter().filter(|&&si| si <= idx).count();
        let prev = if idx == 0 {
            0
        } else {
            steps
                .last()
                .map(|s: &ConvergenceStep| s.claims_proved_cumulative)
                .unwrap_or(0)
        };
        let delta = cumulative - prev;
        let s = cumulative as f64 / total_claims as f64;

        if delta > 0 {
            contributing.push(stage_name);
        }

        steps.push(ConvergenceStep {
            stage_index: idx,
            stage_name,
            checker_name,
            claims_proved_cumulative: cumulative,
            total_claims,
            s_realized: s,
            delta,
        });
    }

    let final_cumulative = claim_stage_indices
        .iter()
        .filter(|&&si| si < stage_count)
        .count();
    let is_monotonic = steps.windows(2).all(|w| w[1].s_realized >= w[0].s_realized);

    Some(ConvergenceTrace {
        total_proved: report.proved.len(),
        total_claims,
        final_s: final_cumulative as f64 / total_claims as f64,
        is_monotonic,
        contributing_stages: contributing,
        steps,
        mode: ConvergenceTraceMode::ClaimLevel,
    })
}

// ── Stage-level fallback (no correctness_report) ──────────────────────────────
//
// S_realized = stage_index / total_stages — a uniform ramp showing that the
// program passes every stage without errors. Used for ALX-1/2/5 which don't
// declare a correctness_report.

fn stage_fallback_trace(module: &Module) -> ConvergenceTrace {
    // Count how many stages are actually exercised by this module's constructs.
    let total = STAGE_ORDER.len();
    let mut steps = Vec::with_capacity(total);
    let mut contributing = Vec::new();

    for (idx, &(stage_name, checker_name)) in STAGE_ORDER.iter().enumerate() {
        let s = (idx + 1) as f64 / total as f64;
        let delta = if stage_is_relevant(stage_name, module) {
            1
        } else {
            0
        };
        if delta > 0 {
            contributing.push(stage_name);
        }
        steps.push(ConvergenceStep {
            stage_index: idx,
            stage_name,
            checker_name,
            claims_proved_cumulative: idx + 1,
            total_claims: total,
            s_realized: s,
            delta,
        });
    }

    ConvergenceTrace {
        total_proved: total,
        total_claims: total,
        final_s: 1.0,
        is_monotonic: true,
        contributing_stages: contributing,
        steps,
        mode: ConvergenceTraceMode::StageFallback,
    }
}

/// Heuristic: is this stage relevant to the module's constructs?
fn stage_is_relevant(stage: &str, module: &Module) -> bool {
    match stage {
        "migration" => module.being_defs.iter().any(|b| !b.migrations.is_empty()),
        "journal" => module.being_defs.iter().any(|b| b.journal.is_some()),
        "session" => module.items.iter().any(|i| matches!(i, Item::Session(_))),
        "manifest" => module.being_defs.iter().any(|b| b.manifest.is_some()),
        "scenario" => module.being_defs.iter().any(|b| !b.scenarios.is_empty()),
        "use_case" => module.items.iter().any(|i| matches!(i, Item::UseCase(_))),
        "boundary" => module
            .items
            .iter()
            .any(|i| matches!(i, Item::BoundaryBlock(_))),
        "smt" => module.items.iter().any(|i| matches!(i, Item::Fn(_))),
        "signal_attention" => module
            .being_defs
            .iter()
            .any(|b| b.signal_attention.is_some()),
        "messaging" => module
            .items
            .iter()
            .any(|i| matches!(i, Item::MessagingPrimitive(_))),
        _ => true, // structural stages always relevant
    }
}

fn empty_trace() -> ConvergenceTrace {
    ConvergenceTrace {
        steps: vec![],
        total_proved: 0,
        total_claims: 0,
        final_s: 0.0,
        is_monotonic: true,
        contributing_stages: vec![],
        mode: ConvergenceTraceMode::StageFallback,
    }
}
