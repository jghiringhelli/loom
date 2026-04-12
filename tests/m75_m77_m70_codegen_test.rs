use loom::compile;

fn ok(src: &str) -> String {
    match compile(src) {
        Ok(out) => out,
        Err(e) => panic!("compile error: {:?}", e),
    }
}

// ── M75: adopt (HGT) ─────────────────────────────────────────────────────────

fn adopt_src() -> &'static str {
    "module M\nadopt: Flyable from BirdModule\nend\n"
}

#[test]
fn adopt_emits_loom_hgt_annotation() {
    let out = ok(adopt_src());
    assert!(
        out.contains("// LOOM[hgt:Flyable]"),
        "expected LOOM[hgt:Flyable]:\n{}",
        out
    );
}

#[test]
fn adopt_annotation_includes_from_module() {
    let out = ok(adopt_src());
    assert!(out.contains("BirdModule"), "output:\n{}", out);
}

#[test]
fn adopt_emits_use_statement() {
    let out = ok(adopt_src());
    assert!(
        out.contains("pub use BirdModule::Flyable"),
        "expected pub use:\n{}",
        out
    );
}

#[test]
fn adopt_emits_adopter_struct() {
    let out = ok(adopt_src());
    assert!(
        out.contains("FlyableAdopter"),
        "expected FlyableAdopter struct:\n{}",
        out
    );
}

#[test]
fn adopt_emits_impl_block() {
    let out = ok(adopt_src());
    assert!(
        out.contains("impl Flyable for FlyableAdopter"),
        "expected impl block:\n{}",
        out
    );
}

#[test]
fn adopt_multiple_interfaces_both_emitted() {
    let out = ok("module M\nadopt: Swimmable from FishModule\nadopt: Runnable from MammalModule\nend\n");
    assert!(out.contains("LOOM[hgt:Swimmable]"), "output:\n{}", out);
    assert!(out.contains("LOOM[hgt:Runnable]"), "output:\n{}", out);
    assert!(out.contains("SwimmableAdopter"), "output:\n{}", out);
    assert!(out.contains("RunnableAdopter"), "output:\n{}", out);
}

// ── M77: niche_construction ───────────────────────────────────────────────────

fn niche_src() -> &'static str {
    "module M\nniche_construction:\n  modifies: soil_chemistry\n  affects: [WormPopulation, PlantGrowth]\nend\nend\n"
}

#[test]
fn niche_emits_loom_annotation() {
    let out = ok(niche_src());
    assert!(
        out.contains("// LOOM[niche_construction:soil_chemistry]"),
        "expected LOOM annotation:\n{}",
        out
    );
}

#[test]
fn niche_emits_struct() {
    let out = ok(niche_src());
    assert!(
        out.contains("SoilChemistryNicheConstruction"),
        "expected SoilChemistryNicheConstruction:\n{}",
        out
    );
}

#[test]
fn niche_emits_modifies_const() {
    let out = ok(niche_src());
    assert!(out.contains("MODIFIES"), "expected MODIFIES const:\n{}", out);
}

#[test]
fn niche_emits_affects_const() {
    let out = ok(niche_src());
    assert!(
        out.contains("AFFECTS"),
        "expected AFFECTS const:\n{}",
        out
    );
    assert!(out.contains("WormPopulation"), "output:\n{}", out);
    assert!(out.contains("PlantGrowth"), "output:\n{}", out);
}

#[test]
fn niche_emits_apply_niche_pressure() {
    let out = ok(niche_src());
    assert!(
        out.contains("apply_niche_pressure"),
        "expected apply_niche_pressure fn:\n{}",
        out
    );
}

#[test]
fn niche_with_probe_fn_emits_probe_method() {
    let out = ok(
        "module M\nniche_construction:\n  modifies: habitat_structure\n  affects: [Beaver, Fish]\n  probe_fn: measure_habitat_change\nend\nend\n",
    );
    assert!(
        out.contains("measure_habitat_change"),
        "expected probe fn:\n{}",
        out
    );
}

#[test]
fn niche_without_affects_still_emits_struct() {
    let out = ok(
        "module M\nniche_construction:\n  modifies: atmosphere\n  affects: [Carbon]\nend\nend\n",
    );
    assert!(out.contains("AtmosphereNicheConstruction"), "output:\n{}", out);
}

// ── M70: canalize ─────────────────────────────────────────────────────────────

fn canalize_src() -> &'static str {
    "module M\nbeing Embryo\ntelos: \"develop\"\nend\ncanalize:\n  toward: adult_form\n  despite: [temperature_shock, nutrient_stress]\nend\nend\nend\n"
}

#[test]
fn canalize_emits_loom_annotation() {
    let out = ok(canalize_src());
    assert!(
        out.contains("// LOOM[canalize:Embryo]"),
        "expected LOOM[canalize:Embryo]:\n{}",
        out
    );
}

#[test]
fn canalize_emits_struct() {
    let out = ok(canalize_src());
    assert!(
        out.contains("EmbryoCanalization"),
        "expected EmbryoCanalization struct:\n{}",
        out
    );
}

#[test]
fn canalize_emits_toward_const() {
    let out = ok(canalize_src());
    assert!(
        out.contains("TOWARD"),
        "expected TOWARD const:\n{}",
        out
    );
    assert!(out.contains("adult_form"), "output:\n{}", out);
}

#[test]
fn canalize_emits_despite_const() {
    let out = ok(canalize_src());
    assert!(out.contains("DESPITE"), "expected DESPITE const:\n{}", out);
    assert!(out.contains("temperature_shock"), "output:\n{}", out);
    assert!(out.contains("nutrient_stress"), "output:\n{}", out);
}

#[test]
fn canalize_emits_is_canalized_fn() {
    let out = ok(canalize_src());
    assert!(
        out.contains("is_canalized"),
        "expected is_canalized fn:\n{}",
        out
    );
}

#[test]
fn canalize_with_convergence_proof_includes_it() {
    let out = ok(
        "module M\nbeing Larva\ntelos: \"grow\"\nend\ncanalize:\n  toward: metamorphosis\n  despite: [heat]\n  convergence_proof: lyapunov_stable\nend\nend\nend\n",
    );
    assert!(
        out.contains("convergence_proof: lyapunov_stable"),
        "output:\n{}",
        out
    );
}