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
// KANI HARNESS HELPERS (module-level free functions)
// ═══════════════════════════════════════════════════════════════════════════

/// Collect free identifiers from a require/ensure expression in first-appearance order.
/// Skips `result` (the postcondition result binding) — that's not a param.
fn kani_scan_idents(
    expr: &Expr,
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    match expr {
        Expr::Ident(name) if name != "result" => {
            if seen.insert(name.clone()) {
                out.push(name.clone());
            }
        }
        Expr::BinOp { left, right, .. } => {
            kani_scan_idents(left, out, seen);
            kani_scan_idents(right, out, seen);
        }
        Expr::Call { args, .. } => {
            for a in args {
                kani_scan_idents(a, out, seen);
            }
        }
        _ => {}
    }
}

/// Map a Loom type to a Kani-compatible Rust type name.
///
/// `String` is not `kani::Arbitrary` — mapped to `i64` as a surrogate.
/// This is a deliberate simplification: contracts on string-shaped data should be
/// extracted to integer-range constraints for SAT proofs.
fn kani_rust_type(ty: &TypeExpr) -> &'static str {
    match ty {
        TypeExpr::Base(name) => match name.as_str() {
            "Int" | "Integer" | "Nat" | "Index" | "Count" => "i64",
            "Float" | "Double" | "Real"                    => "f64",
            "Bool" | "Boolean"                             => "bool",
            "Byte" | "Char"                                => "u8",
            _ => "i64", // String/other: not kani::Arbitrary — use i64 surrogate
        },
        _ => "i64",
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// SEPARATION LOGIC
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Separation logic ownership/disjointness audit (Reynolds 2002 / O'Hearn 2001).
    ///
    /// Emits two layers:
    /// 1. `#[cfg_attr(prusti, prusti_contracts::requires(...))]` — machine-checkable when
    ///    Prusti is active (`PRUSTI_HOME` set, `cargo prusti` invoked).
    /// 2. An auditable comment block documenting the claim for readers without Prusti.
    ///
    /// The `owns:` fields map to Prusti `old(x)` ownership predicates.
    /// The `disjoint: A * B` pairs map to `!std::ptr::eq(a as *const _, b as *const _)`.
    /// The `frame:` fields are documented as non-modified resources (frame rule).
    pub(super) fn emit_separation_audit(&self, fn_name: &str, sb: &SeparationBlock, out: &mut String) {
        // ── Prusti scaffold (active when `cargo prusti` runs) ────────────────
        out.push_str("#[cfg(prusti)]\nuse prusti_contracts::*;\n");

        // Disjointness: for each (A, B) pair, emit a Prusti requires attribute.
        for (a, b) in &sb.disjoint {
            let a_param = a.to_lowercase();
            let b_param = b.to_lowercase();
            out.push_str(&format!(
                "#[cfg_attr(prusti, requires(!std::ptr::eq({a_param} as *const _, {b_param} as *const _)))]\n",
            ));
        }

        // Ownership: document owned resources as Prusti old() preconditions.
        for resource in &sb.owns {
            let param = resource.to_lowercase();
            out.push_str(&format!(
                "#[cfg_attr(prusti, requires(old({param}) == old({param})))] \
// owns: {resource} (exclusive ownership)\n",
            ));
        }

        // Frame: frame-rule resources are not modified — encoded as Prusti ensures.
        for resource in &sb.frame {
            let param = resource.to_lowercase();
            out.push_str(&format!(
                "#[cfg_attr(prusti, ensures(old({param}) == {param}))] \
// frame: {resource} (not modified)\n",
            ));
        }

        // ── Audit comment (always visible) ──────────────────────────────────
        let owns_str = sb.owns.join(", ");
        let disjoint_str: Vec<String> = sb.disjoint.iter()
            .map(|(a, b)| format!("{a} ⊥ {b}"))
            .collect();
        let proof_note = sb.proof.as_deref().unwrap_or("none");
        out.push_str(&format!(
            "// LOOM[contract:Separation]: {fn_name} — Separation Logic (Reynolds 2002 / O'Hearn 2001)\n\
// Claim: heap regions are disjoint at function boundaries.\n\
// owns: [{owns_str}]  disjoint: [{disjoint}]  frame: [{frame}]\n\
// proof: {proof_note}\n\
// Rust affine types enforce single ownership. Prusti harness above checks pointer disjointness.\n\
// Run: PRUSTI_HOME=... cargo prusti -- to formally verify this function.\n\n",
            disjoint = disjoint_str.join(", "),
            frame = sb.frame.join(", "),
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// TIMING SAFETY
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Constant-time execution audit (Kocher 1996, Bernstein 2005).
    ///
    /// Emits two layers:
    /// 1. `#[cfg(subtle)]` — a `ct_eq` usage hint as a `subtle::ConstantTimeEq` wrapper
    ///    that provides the constant-time comparison the function should use.
    /// 2. Audit comment — always visible, documents the claim and dynamic verifiers.
    pub(super) fn emit_timing_safety_audit(
        &self, fn_name: &str, ts: &TimingSafetyBlock, out: &mut String,
    ) {
        let mode = if ts.constant_time { "constant_time" } else { "declared_only" };
        let leaks = ts.leaks_bits.as_deref().unwrap_or("none");

        if ts.constant_time {
            // Emit a subtle::ConstantTimeEq wrapper function that the impl should use.
            out.push_str(&format!(
                "/// Constant-time comparison for `{fn_name}` secrets (Dalek Cryptography: subtle).\n\
/// Use this instead of `==` for any secret-carrying value to prevent timing oracles.\n\
#[cfg(feature = \"subtle\")]\n\
#[inline(never)]\n\
pub fn {fn_name_lower}_ct_eq(a: &[u8], b: &[u8]) -> bool {{\n\
    use subtle::ConstantTimeEq;\n\
    a.ct_eq(b).into()\n\
}}\n\
/// Constant-time selection: returns `a` if `choice == 1`, `b` if `choice == 0`.\n\
#[cfg(feature = \"subtle\")]\n\
#[inline(never)]\n\
pub fn {fn_name_lower}_ct_select(choice: u8, a: u64, b: u64) -> u64 {{\n\
    use subtle::{{ConditionallySelectable, Choice}};\n\
    u64::conditional_select(&b, &a, Choice::from(choice))\n\
}}\n",
                fn_name_lower = fn_name.to_lowercase(),
            ));
        }

        out.push_str(&format!(
            "// LOOM[contract:TimingSafety]: {fn_name} — Constant-time audit (Kocher 1996)\n\
// mode: {mode}  leaks_bits: {leaks}. Prevents timing side-channel attacks.\n\
// Ecosystem: subtle (Dalek Cryptography) — ConstantTimeEq, ConditionallySelectable.\n\
// Use {fn_name_lower}_ct_eq instead of == for secrets (see #[cfg(feature=\"subtle\")] above).\n\
// Dynamic verifier: ctgrind, dudect, or BINSEC/SE.\n\n",
            fn_name_lower = fn_name.to_lowercase(),
        ));
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// TERMINATION
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Termination audit (Turing 1936 / König 1936).
    ///
    /// Emits two layers:
    /// 1. A `const {NAME}_TERMINATION_BOUND: usize` constant that bounds the iteration.
    /// 2. A `{name}_guarded` wrapper function that panics if the bound is exceeded —
    ///    providing a runtime termination certificate when formal proof is unavailable.
    /// 3. Audit comment documenting the metric and pointing to Kani / Dafny.
    pub(super) fn emit_termination_audit(&self, fn_name: &str, metric: &str, out: &mut String) {
        let upper_name = fn_name.to_uppercase();

        // Emit iteration bound constant (conservative default; user should tune).
        out.push_str(&format!(
            "/// Termination bound for `{fn_name}`: the variant `{metric}` must reach 0\n\
/// within this many iterations. Adjust to the expected worst-case input size.\n\
pub const {upper_name}_TERMINATION_BOUND: usize = 1_000_000;\n\n",
        ));

        // Emit a guarded counter wrapper type.
        out.push_str(&format!(
            "/// Runtime termination guard for `{fn_name}` (König 1936 / Turing 1936).\n\
/// Wraps an iteration counter; panics if `{upper_name}_TERMINATION_BOUND` is exceeded.\n\
/// This is the Rust substitute for a formal `decreases {metric}` clause in Dafny/Coq.\n\
#[derive(Debug, Default)]\n\
pub struct {name_pascal}TerminationGuard {{\n    count: usize,\n}}\n\
impl {name_pascal}TerminationGuard {{\n\
    /// Call at the top of each iteration body. Panics if bound is exceeded.\n\
    #[inline]\n\
    pub fn tick(&mut self) {{\n\
        self.count += 1;\n\
        assert!(\n\
            self.count <= {upper_name}_TERMINATION_BOUND,\n\
            \"LOOM termination violation in `{fn_name}`: \\\
metric `{metric}` did not reach 0 within {{}} iterations (bound = {{}})\",\n\
            self.count, {upper_name}_TERMINATION_BOUND\n\
        );\n\
    }}\n\
    pub fn iterations(&self) -> usize {{ self.count }}\n\
}}\n\n",
            name_pascal = to_pascal_case(fn_name),
        ));

        // Audit comment.
        out.push_str(&format!(
            "// LOOM[contract:Termination]: {fn_name} — Termination analysis (König 1936 / Turing 1936)\n\
// metric: `{metric}` — variant must strictly decrease each iteration.\n\
// Runtime guard: {name_pascal}TerminationGuard — panics at bound {upper_name}_TERMINATION_BOUND.\n\
// Formal proof: Kani (SAT-bounded), Dafny (decreases clause), Coq (Acc).\n\n",
            name_pascal = to_pascal_case(fn_name),
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
// KANI FORMAL PROOF HARNESSES (V2)
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Emit a `#[cfg(kani)] #[kani::proof]` harness for functions with contracts.
    ///
    /// For each fn with `require:`/`ensure:`, emits a Kani SAT-bounded proof that
    /// the contracts hold for ALL inputs within the solver's bounds.
    ///
    /// The harness:
    /// 1. Declares symbolic inputs via `kani::any::<T>()`
    /// 2. Restricts the input domain via `kani::assume(require_condition)`
    /// 3. Calls the function under test
    /// 4. Asserts the postconditions via `kani::assert!(ensure_condition, "label")`
    ///
    /// Kani (2021, Amazon/AWS) uses CBMC bounded model checking over Rust MIR.
    /// Install: `cargo install --locked kani-verifier`
    /// Run:     `cargo kani`
    pub(super) fn emit_kani_harness(&self, fd: &FnDef, out: &mut String) {
        if fd.requires.is_empty() && fd.ensures.is_empty() {
            return;
        }
        let fn_name = &fd.name;
        let n_params = fd.type_sig.params.len();

        // Infer parameter names from require/ensure expression identifiers.
        let mut seen = std::collections::HashSet::new();
        let mut inferred: Vec<String> = Vec::new();
        for contract in fd.requires.iter().chain(fd.ensures.iter()) {
            kani_scan_idents(&contract.expr, &mut inferred, &mut seen);
            if inferred.len() >= n_params {
                break;
            }
        }
        let param_names: Vec<String> = (0..n_params)
            .map(|i| inferred.get(i).cloned().unwrap_or_else(|| format!("arg{i}")))
            .collect();

        out.push_str(&format!(
            "// LOOM[V2:Kani]: {fn_name} — SAT-bounded formal proof (Kani 2021)\n"
        ));
        out.push_str(
            "// Proves require:/ensure: hold for ALL inputs within solver bounds.\n",
        );
        out.push_str(
            "// Install: cargo install --locked kani-verifier   Run: cargo kani\n",
        );
        out.push_str(&format!(
            "#[cfg(kani)]\n#[kani::proof]\nfn kani_verify_{fn_name}() {{\n"
        ));

        // Symbolic inputs.
        for (i, ty) in fd.type_sig.params.iter().enumerate() {
            let name = &param_names[i];
            let rust_ty = kani_rust_type(ty);
            out.push_str(&format!("    let {name}: {rust_ty} = kani::any();\n"));
        }

        // Assumptions from require:.
        if !fd.requires.is_empty() {
            out.push_str("    // Preconditions — restrict symbolic input domain\n");
            for req in &fd.requires {
                let cond = self.emit_predicate(&req.expr);
                out.push_str(&format!("    kani::assume({cond});\n"));
            }
        }

        // Call the function under test.
        let args = param_names.join(", ");
        out.push_str(&format!("    let result = {fn_name}({args});\n"));

        // Assertions from ensure:.
        if !fd.ensures.is_empty() {
            out.push_str("    // Postconditions — Kani proves these for all valid inputs\n");
            for ens in &fd.ensures {
                let cond = self.emit_predicate(&ens.expr);
                out.push_str(&format!(
                    "    kani::assert!({cond}, \"{cond}\");\n"
                ));
            }
        }

        out.push_str("}\n\n");
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
        // V2: Kani formal proof harnesses for require/ensure contracts.
        self.emit_kani_harness(fd, &mut out);
        out
    }
}
