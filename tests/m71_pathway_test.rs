use loom::compile;

fn ok(src: &str) -> String {
    match compile(src) {
        Ok(out) => out,
        Err(e) => panic!("compile error: {:?}", e),
    }
}

// ── Parse tests ──────────────────────────────────────────────────────────────

#[test]
fn pathway_minimal_parses() {
    let out = ok("module M\n\
         pathway KrebsCycle\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(out.contains("KrebsCycle"), "output:\n{}", out);
}

#[test]
fn pathway_multi_step_parses() {
    let out = ok("module M\n\
         pathway Glycolysis\n\
           step glucose -[hexokinase]-> glucose6p\n\
           step glucose6p -[phosphoglucose_isomerase]-> fructose6p\n\
           step fructose6p -[phosphofructokinase]-> fructose16bp\n\
         end\n\
         end\n");
    assert!(out.contains("Glycolysis"), "output:\n{}", out);
}

#[test]
fn pathway_with_compensate_parses() {
    let out = ok("module M\n\
         fn rollback_metabolism :: Unit\n\
         end\n\
         pathway MetabolicSaga\n\
           step substrate -[enzyme]-> product\n\
           compensate: rollback_metabolism\n\
         end\n\
         end\n");
    assert!(out.contains("MetabolicSaga"), "output:\n{}", out);
}

// ── Codegen tests ─────────────────────────────────────────────────────────────

#[test]
fn pathway_emits_loom_annotation() {
    let out = ok("module M\n\
         pathway KrebsCycle\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("// LOOM[pathway:KrebsCycle]"),
        "expected LOOM annotation:\n{}",
        out
    );
}

#[test]
fn pathway_emits_step_loom_comments() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("// LOOM[pathway:step]"),
        "expected step annotation:\n{}",
        out
    );
    assert!(out.contains("citrate_synthase"), "output:\n{}", out);
}

#[test]
fn pathway_emits_step_enum() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub enum KrebsStep"),
        "expected KrebsStep enum:\n{}",
        out
    );
}

#[test]
fn pathway_step_enum_has_from_variant() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("AcetylCoa"),
        "expected AcetylCoa variant:\n{}",
        out
    );
}

#[test]
fn pathway_step_enum_has_to_variant() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("Citrate"),
        "expected Citrate variant:\n{}",
        out
    );
}

#[test]
fn pathway_emits_struct() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub struct Krebs"),
        "expected Krebs struct:\n{}",
        out
    );
}

#[test]
fn pathway_emits_execute_method() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub fn execute"),
        "expected execute method:\n{}",
        out
    );
}

#[test]
fn pathway_emits_new_constructor() {
    let out = ok("module M\n\
         pathway Krebs\n\
           step acetyl_coa -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub fn new()"),
        "expected new constructor:\n{}",
        out
    );
}

#[test]
fn pathway_with_compensate_emits_compensate_method() {
    let out = ok("module M\n\
         fn rollback_metabolism :: Unit\n\
         end\n\
         pathway MetabolicSaga\n\
           step substrate -[enzyme]-> product\n\
           compensate: rollback_metabolism\n\
         end\n\
         end\n");
    assert!(
        out.contains("pub fn compensate"),
        "expected compensate method:\n{}",
        out
    );
    assert!(
        out.contains("rollback_metabolism"),
        "expected rollback fn in compensate:\n{}",
        out
    );
}

#[test]
fn pathway_compensate_annotation_emitted() {
    let out = ok("module M\n\
         fn do_rollback :: Unit\n\
         end\n\
         pathway Saga\n\
           step a -[op]-> b\n\
           compensate: do_rollback\n\
         end\n\
         end\n");
    assert!(
        out.contains("// LOOM[pathway:compensate]"),
        "expected compensate annotation:\n{}",
        out
    );
}

#[test]
fn pathway_coexists_with_fn_in_module() {
    let out = ok("module Metabolism\n\
         fn catalyst :: Unit\n\
         end\n\
         pathway TcaCycle\n\
           step oxaloacetate -[citrate_synthase]-> citrate\n\
         end\n\
         end\n");
    assert!(out.contains("TcaCycle"), "output:\n{}", out);
    assert!(out.contains("catalyst"), "output:\n{}", out);
}
