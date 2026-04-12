/// M67 — Correctness report: compiler-generated proof certificate.
///
/// `correctness_report: proved: - property: checker ... unverified: - property: reason ... end`
/// Emits: `// LOOM[correctness_report]` header + `pub const CORRECTNESS_REPORT: &str = "..."`.

fn ok(src: &str) -> String {
    loom::compile(src).unwrap_or_else(|e| panic!("compile error: {:?}", e))
}

// ─── Parse: minimal correctness_report ───────────────────────────────────────

#[test]
fn correctness_report_minimal_parses() {
    let out = ok("module M\n\
         correctness_report:\n\
         end\n\
         end\n");
    assert!(out.contains("correctness_report"), "output:\n{}", out);
}

// ─── Parse: proved section ────────────────────────────────────────────────────

#[test]
fn correctness_report_proved_section_parses() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - separation_safety: separation_checker_verified\n\
         end\n\
         end\n");
    assert!(out.contains("separation_safety"), "output:\n{}", out);
    assert!(
        out.contains("separation_checker_verified"),
        "output:\n{}",
        out
    );
}

// ─── Parse: unverified section ───────────────────────────────────────────────

#[test]
fn correctness_report_unverified_section_parses() {
    let out = ok("module M\n\
         correctness_report:\n\
           unverified:\n\
             - canalization_convergence: requires_smt_feature\n\
         end\n\
         end\n");
    assert!(out.contains("canalization_convergence"), "output:\n{}", out);
    assert!(out.contains("requires_smt_feature"), "output:\n{}", out);
}

// ─── Parse: both sections ─────────────────────────────────────────────────────

#[test]
fn correctness_report_both_sections_parse() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - membrane_integrity: separation_logic_proved\n\
             - homeostasis: refinement_bounds_verified\n\
           unverified:\n\
             - degeneracy_equivalence: requires_smt_feature\n\
         end\n\
         end\n");
    assert!(out.contains("membrane_integrity"), "output:\n{}", out);
    assert!(out.contains("homeostasis"), "output:\n{}", out);
    assert!(out.contains("degeneracy_equivalence"), "output:\n{}", out);
}

// ─── Parse: multiple proved entries ──────────────────────────────────────────

#[test]
fn correctness_report_multiple_proved_entries() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - operational_closure: autopoietic_checker_verified\n\
             - membrane_integrity: separation_logic_proved\n\
             - homeostasis: refinement_bounds_verified\n\
             - epigenetic_stability: aspect_composition_proved\n\
         end\n\
         end\n");
    assert!(out.contains("operational_closure"), "output:\n{}", out);
    assert!(out.contains("epigenetic_stability"), "output:\n{}", out);
}

// ─── Emitter: LOOM annotation ────────────────────────────────────────────────

#[test]
fn correctness_report_emits_loom_annotation() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - type_safety: type_checker_verified\n\
         end\n\
         end\n");
    assert!(
        out.contains("LOOM[correctness_report]"),
        "expected LOOM annotation, output:\n{}",
        out
    );
}

// ─── Emitter: proved comment ─────────────────────────────────────────────────

#[test]
fn correctness_report_emits_proved_comment() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - type_safety: type_checker\n\
         end\n\
         end\n");
    assert!(
        out.contains("proved:"),
        "expected proved:, output:\n{}",
        out
    );
}

// ─── Emitter: unverified comment ─────────────────────────────────────────────

#[test]
fn correctness_report_emits_unverified_comment() {
    let out = ok("module M\n\
         correctness_report:\n\
           unverified:\n\
             - canalization: requires_smt\n\
         end\n\
         end\n");
    assert!(
        out.contains("unverified:"),
        "expected unverified:, output:\n{}",
        out
    );
}

// ─── Emitter: const is generated ─────────────────────────────────────────────

#[test]
fn correctness_report_emits_const() {
    let out = ok("module M\n\
         correctness_report:\n\
           proved:\n\
             - type_safety: type_checker_verified\n\
         end\n\
         end\n");
    assert!(
        out.contains("CORRECTNESS_REPORT"),
        "expected const, output:\n{}",
        out
    );
}

// ─── Correctness report alongside other items ────────────────────────────────

#[test]
fn correctness_report_coexists_with_fn() {
    let out = ok("module M\n\
         fn add :: Int -> Int -> Int\n\
         end\n\
         correctness_report:\n\
           proved:\n\
             - type_safety: type_checker_verified\n\
         end\n\
         end\n");
    assert!(out.contains("add"), "output:\n{}", out);
    assert!(out.contains("CORRECTNESS_REPORT"), "output:\n{}", out);
}

// ─── Full BIOISO-style correctness report ────────────────────────────────────

#[test]
fn correctness_report_full_bioiso_style() {
    let out = ok("module Cell\n\
         correctness_report:\n\
           proved:\n\
             - operational_closure: autopoietic_checker_verified\n\
             - membrane_integrity: separation_logic_proved\n\
             - homeostasis: refinement_bounds_verified\n\
             - epigenetic_stability: aspect_composition_proved\n\
             - security_advice_precedes_exec: aspect_order_verified\n\
             - audit_complete_on_gdpr: effect_checker_verified\n\
           unverified:\n\
             - canalization_convergence: requires_smt_feature\n\
             - degeneracy_equivalence: requires_smt_feature\n\
         end\n\
         end\n");
    assert!(out.contains("operational_closure"), "output:\n{}", out);
    assert!(out.contains("canalization_convergence"), "output:\n{}", out);
    assert!(out.contains("CORRECTNESS_REPORT"), "output:\n{}", out);
}

// ─── Coexistence: aspect + correctness_report in one module ──────────────────

#[test]
fn aspect_and_correctness_report_coexist() {
    let out = ok("module Secure\n\
         fn verify_token :: Unit\n\
         end\n\
         aspect SecurityAspect\n\
           before: verify_token\n\
           order: 1\n\
         end\n\
         correctness_report:\n\
           proved:\n\
             - security_advice_precedes_exec: aspect_order_verified\n\
         end\n\
         end\n");
    assert!(out.contains("SecurityAspect"), "output:\n{}", out);
    assert!(
        out.contains("security_advice_precedes_exec"),
        "output:\n{}",
        out
    );
}
