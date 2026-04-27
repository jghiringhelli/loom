//! Bridge: `.loom` source file → BIOISO runtime entities.
//!
//! Parses a `.loom` file, extracts `being` definitions that carry BIOISO
//! properties (`telos:`, `evolve:`, `regulate:`, etc.), and converts each
//! into a [`DynamicBIOISOSpec`] that the runtime can seed and run.
//!
//! This is the closure that makes loom a live PLN: a being written in loom
//! can be loaded directly into the BIOISO colony without any manual wiring.

use crate::ast::{BeingDef, SearchStrategy, TypeExpr};
use crate::runtime::bioiso_runner::{DynamicBIOISOSpec, DynamicMetricBound};

// ── Type helpers ─────────────────────────────────────────────────────────────

fn is_float_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Base(n) if n == "Float")
}

fn is_int_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Base(n) if n == "Int")
}

fn is_numeric_type(ty: &TypeExpr) -> bool {
    is_float_type(ty) || is_int_type(ty)
}

// ── Tier inference ────────────────────────────────────────────────────────────

/// Infer the BIOISO tier (1–5) from a being's declared features.
///
/// Tier ceiling logic (highest match wins):
/// - T5: autopoietic | crispr | morphogen | rewire
/// - T4: learn_block | epigenetic_blocks
/// - T3: plasticity_blocks
/// - T2: evolve with SA / Mcmc / StochasticGradient
/// - T1: evolve with DerivativeFree, or any being with telos
fn infer_tier(being: &BeingDef) -> u8 {
    if being.autopoietic
        || !being.crispr_blocks.is_empty()
        || !being.morphogen_blocks.is_empty()
        || being.rewire_block.is_some()
    {
        return 5;
    }
    if being.learn_block.is_some() || !being.epigenetic_blocks.is_empty() {
        return 4;
    }
    if !being.plasticity_blocks.is_empty() {
        return 3;
    }
    if let Some(ev) = &being.evolve_block {
        for sc in &ev.search_cases {
            match sc.strategy {
                SearchStrategy::SimulatedAnnealing
                | SearchStrategy::Mcmc
                | SearchStrategy::StochasticGradient
                | SearchStrategy::Genetic
                | SearchStrategy::ParticleSwarm => return 2,
                _ => {}
            }
        }
    }
    1
}

// ── Telos JSON builder ────────────────────────────────────────────────────────

fn build_telos_json(being: &BeingDef) -> String {
    let desc = being
        .telos
        .as_ref()
        .map(|t| t.description.as_str())
        .unwrap_or(being.describe.as_deref().unwrap_or(&being.name));

    let metrics: Vec<String> = being
        .matter
        .as_ref()
        .map(|m| {
            m.fields
                .iter()
                .filter(|f| is_numeric_type(&f.ty))
                .map(|f| format!("\"{}\"", f.name))
                .collect()
        })
        .unwrap_or_default();

    format!(
        r#"{{"target":"{desc}","metrics":[{metrics}]}}"#,
        desc = desc.replace('"', "'"),
        metrics = metrics.join(","),
    )
}

// ── Metric bounds from telos bounded_by ──────────────────────────────────────

/// Parse a `bounded_by:` clause like `"metric >= 0.0"` or `"metric <= 1.0"`
/// into a `DynamicMetricBound`.
///
/// Only simple two-token comparisons are parsed; anything else is skipped.
fn parse_bound(expr: &str, metric_names: &[String]) -> Option<DynamicMetricBound> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let metric = parts[0].to_string();
    // Only accept known matter field names.
    if !metric_names.iter().any(|m| m == &metric) {
        return None;
    }
    let op = parts[1];
    let val: f64 = parts[2].parse().ok()?;

    let mut bound = DynamicMetricBound {
        metric: metric.clone(),
        min: None,
        max: None,
        target: val,
    };
    match op {
        ">=" | ">" => bound.min = Some(val),
        "<=" | "<" => bound.max = Some(val),
        _ => return None,
    }
    // Default mid-range target.
    let mid = match (bound.min, bound.max) {
        (Some(lo), Some(hi)) => (lo + hi) / 2.0,
        (Some(lo), None) => (lo + 1.0) / 2.0,
        (None, Some(hi)) => hi / 2.0,
        (None, None) => 0.5,
    };
    bound.target = mid;
    Some(bound)
}

fn extract_bounds(being: &BeingDef) -> Vec<DynamicMetricBound> {
    let matter_fields: Vec<String> = being
        .matter
        .as_ref()
        .map(|m| m.fields.iter().map(|f| f.name.clone()).collect())
        .unwrap_or_default();

    let mut bounds = Vec::new();

    // From telos.bounded_by expressions (there may be multiple; we store them as
    // a single string — split on common separators).
    if let Some(telos) = &being.telos {
        if let Some(bb) = &telos.bounded_by {
            for clause in bb.split(|c: char| c == ',' || c == '\n' || c == ';') {
                let clause = clause.trim();
                if clause.is_empty() {
                    continue;
                }
                if let Some(b) = parse_bound(clause, &matter_fields) {
                    bounds.push(b);
                }
            }
        }
    }

    // If no bounds were found, generate one per Float matter field with [0.0, 1.0].
    if bounds.is_empty() {
        if let Some(m) = &being.matter {
            for f in &m.fields {
                if is_float_type(&f.ty) {
                    bounds.push(DynamicMetricBound {
                        metric: f.name.clone(),
                        min: Some(0.0),
                        max: Some(1.0),
                        target: 0.5,
                    });
                }
            }
        }
    }

    bounds
}

// ── Baseline signals ──────────────────────────────────────────────────────────

fn extract_baselines(being: &BeingDef) -> Vec<(String, f64)> {
    being
        .matter
        .as_ref()
        .map(|m| {
            m.fields
                .iter()
                .filter_map(|f| {
                    if is_float_type(&f.ty) {
                        Some((f.name.clone(), 0.5))
                    } else if is_int_type(&f.ty) {
                        Some((f.name.clone(), 0.0))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

// ── Telomere ─────────────────────────────────────────────────────────────────

fn extract_telomere(being: &BeingDef) -> (Option<u32>, String) {
    match &being.telomere {
        Some(t) => (Some(t.limit as u32), t.on_exhaustion.clone()),
        None => (None, "senescence".to_string()),
    }
}

// ── Slug helper ──────────────────────────────────────────────────────────────

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Convert a single parsed `BeingDef` into a `DynamicBIOISOSpec`.
///
/// Returns `None` if the being has no `telos:` block (not a BIOISO entity).
pub fn being_to_spec(being: &BeingDef) -> Option<DynamicBIOISOSpec> {
    // Only beings with a declared telos qualify as BIOISO entities.
    if being.telos.is_none() {
        return None;
    }

    let entity_id = slugify(&being.name);
    let name = being.describe.clone().unwrap_or_else(|| being.name.clone());
    let telos_json = build_telos_json(being);
    let bounds = extract_bounds(being);
    let baseline_signals = extract_baselines(being);
    let (telomere_limit, on_exhaustion) = extract_telomere(being);
    let tier = infer_tier(being);

    Some(DynamicBIOISOSpec {
        entity_id,
        name,
        telos_json,
        bounds,
        baseline_signals,
        tier,
        telomere_limit,
        on_exhaustion,
        retro_start_year: 2025,
        academic_baseline_label: None,
    })
}

/// Load all BIOISO-capable beings from a `.loom` source string.
///
/// Parses the source, type-checks it, then converts every `being` with a
/// `telos:` block into a `DynamicBIOISOSpec`. Returns the specs and any
/// parse/check errors.
pub fn load_from_source(
    source: &str,
) -> Result<Vec<DynamicBIOISOSpec>, Vec<crate::error::LoomError>> {
    let module = crate::parse(source)?;
    let specs = module.being_defs.iter().filter_map(being_to_spec).collect();
    Ok(specs)
}

/// Load all BIOISO-capable beings from a `.loom` file path.
pub fn load_from_file(path: &std::path::Path) -> Result<Vec<DynamicBIOISOSpec>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {e}", path.display()))?;
    load_from_source(&source).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}
