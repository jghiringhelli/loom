//! M131–M135: TelosFunction → Rust codegen.
//!
//! Generates four interlocking types per `telos_function` declaration:
//!
//! 1. **`{Name}Metric` trait** (M131) — the core typed contract: `score()`, `converged()`,
//!    `degraded()`.  Implementing this trait is the only thing a user must do to wire
//!    their domain model into Loom's convergence tracking.
//!
//! 2. **`{Name}Evaluation` struct** (M131) — a value-object snapshot: score, convergence
//!    flag, degradation flag, timestamp.  Immutable.  Serialisable.
//!
//! 3. **`{Name}ConvergenceTracker` struct** (M132–M133) — maintains a rolling history of
//!    `Evaluation` values, applies `TelosThresholds`, and exposes `is_converging()`.
//!    Emits populated threshold constants when `thresholds:` is declared; falls back to
//!    `todo!()` stubs with inline guidance when absent.
//!
//! 4. **`{Name}SignalAttention` struct** (M134) — a per-telos attention-weight map that
//!    amplifies or attenuates decision axes.  Each entry in `guides:` becomes a named
//!    method constant comment so the user knows which axes are live.
//!
//! 5. **`TelosMetricFn` type alias** (M132) — derives from `measured_by:` if present.
//!
//! 6. **Guided-axis integration hints** (M134–M135) — one `// LOOM[telos:guide]:` comment
//!    per entry in `guides:`.

use crate::ast::TelosFunctionDef;

/// Emit all Rust constructs for a `telos_function` declaration.
///
/// # Parameters
/// - `tf` — the parsed `TelosFunctionDef` AST node.
///
/// # Returns
/// A `String` containing ready-to-compile Rust code (plus doc comments and
/// `// LOOM[...]` annotation markers for tooling).
pub fn emit_telos_function(tf: &TelosFunctionDef) -> String {
    let mut out = String::new();
    let name = pascal(&tf.name);

    emit_header(tf, &name, &mut out);
    emit_metric_fn_alias(tf, &name, &mut out);
    emit_metric_trait(tf, &name, &mut out);
    emit_evaluation_struct(&name, &mut out);
    emit_convergence_tracker(tf, &name, &mut out);
    emit_signal_attention(tf, &name, &mut out);
    emit_guide_hints(tf, &name, &mut out);

    out
}

// ── Section helpers ────────────────────────────────────────────────────────────

fn emit_header(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    out.push_str(&format!(
        "// ── telos_function: {raw} ──────────────────────────────────────────────\n",
        raw = tf.name
    ));
    out.push_str("// LOOM[telos_fn]: Peirce interpretant as typed function (M131–M135)\n");

    if let Some(stmt) = &tf.statement {
        out.push_str(&format!("// Statement: {}\n", stmt));
    }
    if let Some(bounded) = &tf.bounded_by {
        out.push_str(&format!("// Bounded by: {}\n", bounded));
    }
    out.push_str(&format!(
        "// Generates: {name}Metric, {name}Evaluation, {name}ConvergenceTracker, \
         {name}SignalAttention\n\n"
    ));
}

/// M132: emit the `TelosMetricFn` type alias when `measured_by` provides a signature.
fn emit_metric_fn_alias(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    let alias_type = match &tf.measured_by {
        Some(sig) => metric_fn_type(sig),
        None => return, // alias only emitted when signature is declared
    };

    out.push_str(&format!(
        "/// Typed metric function for the `{raw}` telos.\n\
         /// Signature declared in Loom: `measured_by: \"{sig}\"`\n\
         pub type {name}MetricFn = {alias_type};\n\n",
        raw = tf.name,
        sig = tf.measured_by.as_deref().unwrap_or(""),
        alias_type = alias_type,
    ));
}

/// M131: core `{Name}Metric` trait.
fn emit_metric_trait(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    let measured_hint = match &tf.measured_by {
        Some(sig) => format!(" Signature: `{}`.", sig),
        None => String::from(
            " Implement `score()` to return a value in `[0.0, 1.0]` \
             (higher = more aligned with telos).",
        ),
    };

    out.push_str(&format!(
        "/// Typed metric contract for the `{raw}` telos.\n\
         ///{measured_hint}\n\
         pub trait {name}Metric {{\n\
             /// Compute the current telos alignment score.\n\
             fn score(&self) -> f64;\n\n\
             /// Returns `true` when `score()` is at or above the convergence threshold.\n\
             fn converged(&self) -> bool;\n\n\
             /// Returns `true` when `score()` has fallen at or below the divergence threshold.\n\
             fn degraded(&self) -> bool;\n\
         }}\n\n",
        raw = tf.name,
        measured_hint = measured_hint,
    ));
}

/// M131: immutable `{Name}Evaluation` snapshot.
fn emit_evaluation_struct(name: &str, out: &mut String) {
    out.push_str(&format!(
        "/// Immutable telos evaluation snapshot for `{name}`.\n\
         #[derive(Debug, Clone, PartialEq)]\n\
         pub struct {name}Evaluation {{\n\
             /// Raw alignment score in `[0.0, 1.0]`.\n\
             pub score: f64,\n\
             /// Whether the being has converged toward telos.\n\
             pub converged: bool,\n\
             /// Whether the being has degraded beyond the alarm threshold.\n\
             pub degraded: bool,\n\
             /// Unix-epoch timestamp (seconds) when this evaluation was taken.\n\
             pub timestamp: u64,\n\
         }}\n\n"
    ));
}

/// M132–M133: convergence tracker with populated thresholds.
fn emit_convergence_tracker(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    // Threshold constants — use actual values or f64 stubs
    let (convergence_val, warning_val, divergence_val, propagation_val) = match &tf.thresholds {
        Some(t) => {
            let w = match t.warning {
                Some(w) => format!("Some({:.4}_f64)", w),
                None => "None".to_string(),
            };
            let p = match t.propagation {
                Some(p) => format!("Some({:.4}_f64)", p),
                None => "None".to_string(),
            };
            (
                format!("{:.4}_f64", t.convergence),
                w,
                format!("{:.4}_f64", t.divergence),
                p,
            )
        }
        None => (
            "0.75_f64 // TODO: set convergence threshold".to_string(),
            "Some(0.50_f64) // TODO: optional warning threshold".to_string(),
            "0.25_f64 // TODO: set divergence/alarm threshold".to_string(),
            "None".to_string(),
        ),
    };

    out.push_str(&format!(
        "/// Rolling convergence tracker for the `{raw}` telos.\n\
         pub struct {name}ConvergenceTracker {{\n\
             history: Vec<{name}Evaluation>,\n\
             convergence_threshold: f64,\n\
             warning_threshold: Option<f64>,\n\
             divergence_threshold: f64,\n\
             propagation_threshold: Option<f64>,\n\
         }}\n\n\
         impl {name}ConvergenceTracker {{\n\
             /// Construct a tracker with the thresholds declared in the Loom spec.\n\
             pub fn new() -> Self {{\n\
                 Self {{\n\
                     history: Vec::new(),\n\
                     convergence_threshold: {convergence_val},\n\
                     warning_threshold: {warning_val},\n\
                     divergence_threshold: {divergence_val},\n\
                     propagation_threshold: {propagation_val},\n\
                 }}\n\
             }}\n\n\
             /// Record a new evaluation snapshot.\n\
             pub fn record(&mut self, eval: {name}Evaluation) {{\n\
                 self.history.push(eval);\n\
             }}\n\n\
             /// Returns `true` if the last N evaluations all show convergence.\n\
             pub fn is_converging(&self, window: usize) -> bool {{\n\
                 if self.history.len() < window {{\n\
                     return false;\n\
                 }}\n\
                 self.history\n\
                     .iter()\n\
                     .rev()\n\
                     .take(window)\n\
                     .all(|e| e.score >= self.convergence_threshold)\n\
             }}\n\n\
             /// Returns `true` if any recent evaluation triggered the alarm threshold.\n\
             pub fn is_degraded(&self, window: usize) -> bool {{\n\
                 self.history\n\
                     .iter()\n\
                     .rev()\n\
                     .take(window)\n\
                     .any(|e| e.score <= self.divergence_threshold)\n\
             }}\n\n\
             /// Returns `true` when above the propagation threshold (if declared).\n\
             pub fn eligible_for_propagation(&self) -> bool {{\n\
                 match (self.history.last(), self.propagation_threshold) {{\n\
                     (Some(e), Some(p)) => e.score >= p,\n\
                     _ => false,\n\
                 }}\n\
             }}\n\
         }}\n\n\
         impl Default for {name}ConvergenceTracker {{\n\
             fn default() -> Self {{ Self::new() }}\n\
         }}\n\n",
        raw = tf.name,
        name = name,
        convergence_val = convergence_val,
        warning_val = warning_val,
        divergence_val = divergence_val,
        propagation_val = propagation_val,
    ));
}

/// M134: `{Name}SignalAttention` — per-telos attention-weight map for decision axes.
fn emit_signal_attention(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    // Build axis-weight insertion lines from `guides`
    let inserts: String = tf
        .guides
        .iter()
        .map(|axis| {
            format!(
                "            map.insert(\"{axis}\".to_string(), 1.0_f64); \
                 // LOOM[guide]: default weight for axis '{axis}'\n",
                axis = axis
            )
        })
        .collect();

    let insert_block = if inserts.is_empty() {
        "            // no guides declared — add axes here\n".to_string()
    } else {
        inserts
    };

    out.push_str(&format!(
        "/// Per-axis attention weights for the `{raw}` telos.\n\
         /// Weights > 1.0 amplify a decision axis; weights < 1.0 attenuate it.\n\
         pub struct {name}SignalAttention {{\n\
             pub attention_weights: std::collections::HashMap<String, f64>,\n\
         }}\n\n\
         impl {name}SignalAttention {{\n\
             /// Construct with default unit weights for all declared guide axes.\n\
             pub fn new() -> Self {{\n\
                 let mut map = std::collections::HashMap::new();\n\
         {insert_block}\
                 Self {{ attention_weights: map }}\n\
             }}\n\n\
             /// Amplify an axis (multiply its weight by `factor`).\n\
             pub fn amplify(&mut self, axis: &str, factor: f64) {{\n\
                 let w = self.attention_weights.entry(axis.to_string()).or_insert(1.0);\n\
                 *w *= factor;\n\
             }}\n\n\
             /// Attenuate an axis (divide its weight by `factor`; minimum `0.0`).\n\
             pub fn attenuate(&mut self, axis: &str, factor: f64) {{\n\
                 if factor == 0.0 {{ return; }}\n\
                 let w = self.attention_weights.entry(axis.to_string()).or_insert(1.0);\n\
                 *w = (*w / factor).max(0.0);\n\
             }}\n\n\
             /// Return the effective weight for `axis` (defaults to `1.0`).\n\
             pub fn weight(&self, axis: &str) -> f64 {{\n\
                 self.attention_weights.get(axis).copied().unwrap_or(1.0)\n\
             }}\n\
         }}\n\n\
         impl Default for {name}SignalAttention {{\n\
             fn default() -> Self {{ Self::new() }}\n\
         }}\n\n",
        raw = tf.name,
        name = name,
        insert_block = insert_block,
    ));
}

/// M134–M135: `// LOOM[telos:guide]:` integration hints — one per `guides` axis.
fn emit_guide_hints(tf: &TelosFunctionDef, name: &str, out: &mut String) {
    if tf.guides.is_empty() {
        return;
    }

    out.push_str(&format!(
        "// ── Guide-axis integration hints for '{raw}' ──────────────────────\n",
        raw = tf.name
    ));

    for axis in &tf.guides {
        out.push_str(&format!(
            "// LOOM[telos:guide]: {name} guides '{axis}' — \
             wire {name}SignalAttention::weight(\"{axis}\") \
             into your {axis} selection logic\n",
            name = name,
            axis = axis
        ));
    }
    out.push('\n');
}

// ── Conversion helpers ─────────────────────────────────────────────────────────

/// Convert a `measured_by` signature string to a Rust `fn` pointer type.
///
/// Loom uses arrow notation: `"BeingState -> SignalSet -> Float"`.
/// We map `Float` → `f64`, then emit `Box<dyn Fn(A, B, ...) -> Z>`.
fn metric_fn_type(sig: &str) -> String {
    let parts: Vec<&str> = sig.split("->").map(str::trim).collect();
    if parts.len() < 2 {
        return format!("Box<dyn Fn() -> f64>  // NOTE: could not parse '{}'", sig);
    }
    let inputs: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|t| loom_type_to_rust(t))
        .collect();
    let output = loom_type_to_rust(parts[parts.len() - 1]);
    format!("Box<dyn Fn({}) -> {}>", inputs.join(", "), output)
}

/// Map Loom primitive type names to Rust equivalents.
fn loom_type_to_rust(loom_ty: &str) -> String {
    match loom_ty.trim() {
        "Float" | "float" => "f64".to_string(),
        "Int" | "int" | "Integer" => "i64".to_string(),
        "Bool" | "bool" => "bool".to_string(),
        "String" | "string" | "Str" => "String".to_string(),
        other => other.to_string(),
    }
}

/// Convert `snake_case` to `PascalCase`.
fn pascal(s: &str) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}
