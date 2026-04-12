# Changelog

All notable changes to Loom are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

### Added — Biological Layer Codegen (M66–M77)
- **M66 aspect:** Real Rust emitter — `pub struct {Name}Advice` + before/after/around dispatch + `// LOOM[aspect:Name]` annotation
- **M67 correctness_report:** `{Name}CorrectnessReport` struct + `verify()` method + audit annotation
- **M68 degenerate:** `{Name}DegenerateFallback<T>` struct + `normal()`/`fallback()`/`require_non_degenerate()` methods; consolidated into `contracts.rs`
- **M69 lifecycle+checkpoint:** `// LOOM[lifecycle:Name]` + zero-sized state markers + `{Name}State` enum + `transition()` method
- **M70 canalize:** `{Name}Canalization` struct + `TOWARD`/`DESPITE` consts + `is_canalized(perturbation)` method (Waddington 1957)
- **M71 pathway:** `{Name}Step` enum + `{Name}` struct + `execute()` method + compensate stub
- **M75 HGT adopt:** `pub use FromModule::Interface` + `InterfaceAdopter` struct + `impl Interface for InterfaceAdopter {}` + `// LOOM[hgt:Name]`
- **M76 criticality:** `NAME_CRITICALITY_LOWER`/`NAME_CRITICALITY_UPPER` consts + probe stub
- **M77 niche_construction:** `{Name}NicheConstruction` struct + `MODIFIES`/`AFFECTS` consts + `apply_niche_pressure()` + probe stub (Odling-Smee 1996)

### Added — BIOISO Domain Programs
- `experiments/alx/bioiso-climate.loom` — AtmosphericCarbon CO2 homeostasis (Keeling curve, Lyapunov proof)
- `experiments/alx/bioiso-energy.loom` — GridBalancer renewable energy transition
- `experiments/alx/bioiso-epidemics.loom` — PathogenController R0 suppression
- `experiments/alx/bioiso-antibiotics.loom` — StewardshipController + HGT adoption from EpidemiologyModule
- `experiments/alx/bioiso-materials.loom` — AdaptiveMaterial self-healing scaffold
- All 5 compile cleanly ✅

### Added — PLN Experiments
- `experiments/lx/LX-1-semantic-density/measure.py` — token + property density script (no external deps)
- `experiments/lx/LX-1-semantic-density/results.md` — 2.66x L/TS average; 3.3–3.8x for BIOISO beings
- `experiments/lx/LX-2-kani-harness/` — Kani `#[cfg(kani)] #[kani::proof]` harness structure verified; CBMC proof deferred (Linux-only)
- `experiments/lx/LX-4-stateless-derivability/` — Protocol + 5 feature prompts for cold-start reproducibility test

### Added — Claim Coverage
- `experiments/verification/claim_coverage.md` updated: 163 → 196 claims tracked, PROVED 138 → 170 (87%)
- 32 new PROVED claims for M66–M77 biological layer codegen
- 1 new PROVED: 5 BIOISO ALX programs compile

### Added — docs/pln.md
- Updated: drift resistance now ✅ (M66/M67), ALX score 44/45, LX-4 marked "Testable now"

## [0.2.0] — M1–M65 (prior)

See git log for full M1–M65 milestone history.
All 800+ tests pass. Real compiler with parse → check → codegen pipeline.
