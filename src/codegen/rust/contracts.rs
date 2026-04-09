//! Correctness annotation codegen — proof obligations attached to declarations.
//!
//! These emit from annotations the developer explicitly attached to a function
//! to claim a correctness property.  They are **not** implicit disciplines (which
//! fire without being asked) and **not** domain structures (which model data).
//! They are verifiable guarantees: the developer made a claim; Loom emits the
//! scaffolding that makes the claim auditable or enforceable.
//!
//! ## Contract map (annotation -> generated artifact)
//!
//! | `separation:`    | Ownership audit comment — Reynolds 2002 / O'Hearn 2001 |
//! | `timing_safety:` | Constant-time audit + subtle hints — Kocher 1996       |
//! | `termination:`   | Termination metric audit + iteration guard — König 1936 |
//! | `gradual:`       | Gradual typing boundary wrapper — Siek & Taha 2006     |
//! | `degenerate:`    | Degeneracy fallback dispatcher — Edelman                |
//!
//! ## The `emit_fn_contracts` dispatcher
//! Called from `functions.rs` after emitting the function body.  It also
//! delegates to `structures` for fn-level stochastic / distribution annotations
//! because those are attached to functions even though they model structures.

use crate::ast::*;
use super::{RustEmitter, to_pascal_case};


// ═══════════════════════════════════════════════════════════════════════════
// SEPARATION LOGIC
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Separation logic ownership/disjointness audit (Reynolds 2002 / O'Hearn 2001).
    /// Rust's borrow checker enforces affine ownership; this emits the proof claim as
    /// an auditable comment block, plus a pointer to Prusti for full verification.
    pub(super) fn emit_separation_audit(&self, fn_name: &str, sb: &SeparationBlock, out: &mut String) {
        let owns = sb.owns.join(", ");
        let disjoint: Vec<String> = sb.disjoint.iter().map(|(a, b)| format!("{a} * {b}")).collect();
        out.push_str(&format!(
            "// LOOM[contract:Separation]: {fn_name} — Separation Logic (Reynolds 2002)\n\
// Claim: heap regions are disjoint. Rust borrow checker enforces affine ownership.\n\
// owns: {owns}  disjoint: {disjoint}  frame: {frame}\n\
// Ecosystem: Prusti (ETH Zurich) — #[requires(x != y)] harness for full proof.\n\n",
            disjoint = disjoint.join(", "),
            frame = sb.frame.join(", "),
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// TIMING SAFETY
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Constant-time execution audit (Kocher 1996, Bernstein 2005).
    /// Emits an audit comment and a hint to use `subtle::ConstantTimeEq` instead of `==`
    /// for secret-dependent comparisons.  Dynamic verifiers: ctgrind, dudect, BINSEC/SE.
    pub(super) fn emit_timing_safety_audit(
        &self, fn_name: &str, ts: &TimingSafetyBlock, out: &mut String,
    ) {
        let mode = if ts.constant_time { "constant_time" } else { "declared_only" };
        let leaks = ts.leaks_bits.as_deref().unwrap_or("none");
        out.push_str(&format!(
            "// LOOM[contract:TimingSafety]: {fn_name} — Constant-time audit (Kocher 1996)\n\
// mode: {mode}  leaks_bits: {leaks}. Prevents timing side-channel attacks.\n\
// Ecosystem: subtle (Dalek Cryptography) — ConstantTimeEq, ConstantTimeGreater.\n\
// Use subtle::ConstantTimeEq::ct_eq instead of == for secrets.\n\
// Dynamic verifier: ctgrind, dudect, or BINSEC/SE.\n\n"
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// TERMINATION
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Termination audit (Turing 1936 / König 1936).
    /// Rust cannot prove general termination; emits the metric claim as an audit
    /// comment with a pointer to Kani (SAT-bounded) or Dafny (`decreases` clause).
    pub(super) fn emit_termination_audit(&self, fn_name: &str, metric: &str, out: &mut String) {
        out.push_str(&format!(
            "// LOOM[contract:Termination]: {fn_name} — Termination analysis (König 1936)\n\
// Claim: function terminates. Rust cannot prove general termination.\n\
// metric: {metric} — variant must strictly decrease each iteration.\n\
// Ecosystem: Kani (SAT-bounded), Dafny (decreases clause), Coq (Acc).\n\
// For production: add a bounded iteration guard and panic on exceed.\n\n"
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// GRADUAL TYPING BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Gradual typing boundary (Siek & Taha 2006).
    /// Emits a `GradualBoundary<T,U>` enum that wraps static or dynamic dispatch;
    /// the static side is checked at compile time, the dynamic side at runtime.
    pub(super) fn emit_gradual_boundary(&self, fn_name: &str, gb: &GradualBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let input  = gb.input_type.as_deref().unwrap_or("T");
        let output = gb.output_type.as_deref().unwrap_or("U");
        out.push_str(&format!(
            "// LOOM[contract:Gradual]: {fn_name} — Gradual Typing Boundary (Siek & Taha 2006)\n\
// input: {input} -> output: {output}. Cast checks at runtime.\n\
// Ecosystem: Any trait (std), erased types, dynamic dispatch.\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug)]\npub enum {n}GradualBoundary<T, U> {{\n    Static(T),\n    Dynamic(U),\n}}\n"
        ));
        out.push_str(&format!(
            "impl<T, U: std::fmt::Debug> {n}GradualBoundary<T, U> {{\n    \
/// Unwrap static side. Panics if boundary is dynamic (deliberate fail-fast).\n    \
pub fn static_value(self) -> T {{\n        \
match self {{ Self::Static(v) => v, Self::Dynamic(d) => panic!(\"gradual boundary violation: {{:?}}\", d) }}\n    \
}}\n}}\n\n"
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// DEGENERATE FALLBACK
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Degenerate case fallback dispatcher (Edelman).
    /// When the computation degenerates (e.g. empty matrix, zero vector), use the
    /// declared fallback instead of silently returning garbage.  The wrapper makes
    /// degeneracy visible and `require_non_degenerate` fails fast on unintended use.
    pub(super) fn emit_degenerate_fallback(&self, fn_name: &str, db: &DegenerateBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        out.push_str(&format!(
            "// LOOM[contract:Degenerate]: {fn_name} — Degenerate case fallback (Edelman)\n\
// primary: {}  fallback: {}. Returns fallback value instead of failing silently.\n\n",
            db.primary, db.fallback
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}DegenerateFallback<T> {{\n    pub value: T,\n    pub is_degenerate: bool,\n}}\n"
        ));
        out.push_str(&format!(
            "impl<T: std::fmt::Debug + Clone> {n}DegenerateFallback<T> {{\n    \
pub fn normal(v: T) -> Self {{ Self {{ value: v, is_degenerate: false }} }}\n    \
pub fn fallback(v: T) -> Self {{ Self {{ value: v, is_degenerate: true }} }}\n    \
pub fn require_non_degenerate(self) -> T {{\n        \
debug_assert!(!self.is_degenerate, \"degenerate fallback activated\"); self.value\n    }}\n}}\n\n"
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// FN ANNOTATION DISPATCHER
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Master dispatcher: emit ALL annotation artifacts for a function.
    ///
    /// Coordinates across two codegen modules:
    /// - `structures` — stochastic process and distribution annotations model data structures
    /// - `contracts`  — separation, timing, termination, gradual, degenerate are proof claims
    ///
    /// Called from `emit_fn_def` in `functions.rs` after emitting the function body.
    pub(super) fn emit_fn_contracts(&self, fd: &FnDef) -> String {
        let mut out = String::new();
        // Stochastic and distribution are fn-level structure declarations.
        if let Some(sp) = &fd.stochastic_process {
            self.emit_stochastic_process(&fd.name, sp, &mut out);
        }
        if let Some(db) = &fd.distribution {
            self.emit_distribution_sampler(&fd.name, db, &mut out);
        }
        // The following are correctness contracts attached to the function.
        if let Some(sb) = &fd.separation {
            self.emit_separation_audit(&fd.name, sb, &mut out);
        }
        if let Some(ts) = &fd.timing_safety {
            self.emit_timing_safety_audit(&fd.name, ts, &mut out);
        }
        if let Some(metric) = &fd.termination {
            self.emit_termination_audit(&fd.name, metric, &mut out);
        }
        if let Some(gb) = &fd.gradual {
            self.emit_gradual_boundary(&fd.name, gb, &mut out);
        }
        if let Some(db) = &fd.degenerate {
            self.emit_degenerate_fallback(&fd.name, db, &mut out);
        }
        out
    }
}
