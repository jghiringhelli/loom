# Status.md

## Last Updated: 2026-04-10
## Branch: launch/v0.2-public-release

## Completed (this session)
- **feat(bioiso): M117-M119 BIOISO complete — trigger/action regulate, telomere aliases, 4 demos green** (commit 6f8aa0a)
  - `regulate:` now supports trigger/action syntax: `regulate: trigger: expr action: fn_name end`
  - `telomere:` now accepts `max_generations:` and `senescence_trigger:` aliases  
  - Ecosystem signals checker relaxed for `coevolution: true` (Darwinian selection model)
  - `RegulateBlock` AST gains `trigger: Option<String>` + `action: Option<String>` fields
  - 4 BIOISO demo files all compile cleanly:
    - `minimal_life.loom` — ProtoCell satisfying Schrödinger + Maturana/Varela + Barbieri ✅
    - `peircean_organism.loom` — E. coli as Peircean biosemiotic organism ✅
    - `natural_selection.loom` — Darwinian selection emergent from propagate: + ecosystem ✅
    - `intent_vivo.loom` — ScalpingAgent + IntentCoordinator with human governance ✅
  - All 95 test suites pass (12 bioiso_constructs, 12 being_test, 15 autopoiesis, 12 safety, ...)

## Current State
- All M1–M116 milestones complete
- Branch: `launch/v0.2-public-release`
- Binary verify: all 5 examples compile end-to-end (loom → rustc) ✅
- 27+ test suites passing (37 unit + integration tests)

## Next
- **launch-website** — write landing page for `website/` Astro site
- **launch-readme** — update README (still says "311 tests / 23 milestones")
- **launch-cargo-meta** — add `homepage`/`repository`/`documentation`/`keywords` to Cargo.toml
- **merge to main** — after website + README are ready
- **V2 Kani** — add Kani harnesses for require:/ensure: contracts (next verification tier)


- Consider: UI stdlib (M92 stores → HTML/CSS sense channels), games stdlib (M84 distributions + M87 tensors)
- `publish-merge`: merge to main, cargo publish, arXiv

## Decisions made
- Dafny scaffolds embedded as `r##"..."##` Rust const strings — developer extracts to .dfy and runs `dafny verify`
- TLA+ spec embedded similarly as const string — developer runs TLC model checker

## Blockers
- None — all work is greenfield



## Test Count
- **Total tests:** 800+ passing ✅
- **ALX gate:** ✅ ALX-1 through ALX-6 all pass
- **ALX-6:** S_realized = 44/45 = 0.9778 — all 5 convergence tests green

## Feature Tracker
| Feature | Status | Notes |
|---------|--------|-------|
| M1–M8 (Phases 1–3) | ✅ Done | Core language |
| M9–M12 (Phase 4) | ✅ Done | Inline, coerce, iter, algebraic types |
| M13–M15 (Phase 5) | ✅ Done | OpenAPI, JSON Schema, TypeScript |
| M16–M18 | ✅ Done | Contracts, typestate, privacy |
| M19–M23 | ✅ Done | Being, teleos, safety, info-flow, units |
| M41–M52 (Phase 8) | ✅ Done | Biological autopoiesis layer |
| M56: Refinement Types | ✅ Done | RefinementChecker, TryFrom codegen |
| M57: Separation Logic | ✅ Done | owns/disjoint/frame/proof, SeparationChecker |
| M58: Temporal Logic | ✅ Done | precedes/always/never, TemporalChecker |
| M59: Gradual Typing | ✅ Done | gradual block, blame tracking |
| M60: Probabilistic Types | ✅ Done | distribution/prior/confidence |
| M61: Dependent Types | ✅ Done | proposition/termination |
| M62: Side-Channel Safety | ✅ Done | timing_safety block, constant-time check |
| M63: Category Theory | ✅ Done | functor/monad with law verification |
| M64: Curry-Howard | ✅ Done | proof annotations, certificate codegen |
| M65: Self-Certifying | ✅ Done | certificate block |
| M66: AOP Aspects | ✅ Done | aspect/pointcut/before/after/around/order |
| M66b: Annotation Algebra | ✅ Done | annotation declarations with meta |
| M67: Correctness Report | ✅ Done | correctness_report block with proved/unverified |
| M68: Degeneracy | ✅ Done | degenerate block on fn (Edelman) |
| M69: Cell Cycle Checkpoints | ✅ Done | checkpoint in lifecycle (Hartwell) |
| M70: Canalization | ✅ Done | canalize block on being (Waddington) |
| M71: Metabolic Pathways | ✅ Done | pathway items (Krebs) |
| M72: Symbiosis Typing | ✅ Done | symbiotic import mutualistic/commensal/parasitic |
| M73: Error Correction | ✅ Done | on_violation/repair_fn on refined types |
| M74: Senescence | ✅ Done | senescence block on being (Campisi) |
| M75: HGT (Lateral Adoption) | ✅ Done | adopt declaration |
| M76: Criticality Bounds | ✅ Done | criticality block on being (Langton) |
| M77: Niche Construction | ✅ Done | niche_construction item (Odling-Smee) |
| M83: Sense Stdlib | ✅ Done | 22 SI derived units, beyond-human senses, embedded SENSE_STDLIB |
| M86: Conservation Annotations | ✅ Done | @conserved(Mass/Energy/Value) |
| M87: Tensor Types | ✅ Done | Tensor<rank, shape, unit>, TensorChecker |
| M92: Store Declarations | ✅ Done | 11 store kinds, polyglot persistence |
| M93: Operational Stores | ✅ Done | Relational/KeyValue/Document checkers + Rust codegen stubs |
| M94: Analytical Stores | ✅ Done | Columnar/Snowflake/Hypercube checkers (Gray 1996 citation) |
| M95: Specialized Stores | ✅ Done | Graph/TimeSeries/Vector store checkers |
| M96: Local Stores | ✅ Done | InMemory (LRU/LFU/ARC), FlatFile (Parquet/Arrow/HDF5/CSV) |
| M97: Distributed Stores | ✅ Done | MapReduce, DistributedLog (Kreps 2011) |
| M98: Session Types | ✅ Done | session/role/send/recv/duality, SessionChecker (Honda 1993) |
| M99: Effect Handlers | ✅ Done | effect/operation/handle/with, EffectHandlerChecker |
| M100: SMT Bridge | ✅ Done | SmtBridgeChecker, SMT-LIB2 translation |
| M101: Manifest Liveness | ✅ Done | ManifestChecker, artifact existence + symbol reflects |
| M102: Provenance | ✅ Done | ProvenanceChecker, @provenance annotations |
| M103: Boundary | ✅ Done | BoundaryBlock, BoundaryChecker |
| M104: Journal | ✅ Done | JournalBlock, JournalChecker |
| M105: Scenario | ✅ Done | ScenarioBlock, ScenarioChecker |
| M106: Migration | ✅ Done | MigrationBlock, MigrationChecker |
| M107: Minimal | ✅ Done | MinimalChecker (dead declaration detection) |
| M108: Diagram Emit | ✅ Done | compile_mermaid_c4/sequence/state/flow |
| M109: Property Tests | ✅ Done | PropertyBlock, PropertyChecker |
| M110: UseCase | ✅ Done | UseCaseBlock, UseCaseChecker |
| M111: Evolution Vectors | ✅ Done | EvolutionVectorChecker, cosine similarity |
| M112: TelosDef upgrade | ✅ Done | measured_by/thresholds/guides on telos block |
| M113: TelosImmutability | ✅ Done | modifiable_by without @corrigible → error |
| M114: telos_contribution | ✅ Done | [0.0,1.0] contribution on regulate blocks |
| M115: signal_attention | ✅ Done | SignalAttentionChecker, prioritize/attenuate thresholds |
| M116: messaging_primitive | ✅ Done | MessagingChecker, 5 patterns, guarantees |
| M117: telos_function | ✅ Done | TelosFunctionDef, guides + thresholds parser |
| M118: entity | ✅ Done | EntityDef, alias_of generic type |
| M119: intent_coordinator | ✅ Done | IntentCoordinatorDef, governance + telomere |
| BIOISO: propagate: | ✅ Done | PropagateBlock, inherits/mutates/offspring_type |
| BIOISO: epigenetic: | ✅ Done | EpigeneticBlock with duration: (plasticity) |
| BIOISO: signal_attention named lists | ✅ Done | prioritize/attenuate as named signals |
| BIOISO: ecosystem tipping_points | ✅ Done | TippingPoint, condition/on_crossing |
| BIOISO: trigger/action regulate | ✅ Done | regulate: trigger:/action: syntax |
| ALX-1 through ALX-5 | ✅ Done | All ALX gates pass, S_realized ≥ 0.85 |
| ALX-6 | ✅ Done | S_realized = 44/45 = 0.9778 |
| 4 BIOISO demos | ✅ Done | minimal_life, natural_selection, peircean_organism, intent_vivo |

## Current Context
- Branch: `launch/v0.2-public-release`
- **800+ tests passing** (all 95 test suites, 0 failures)
- All M1-M119 milestones + BIOISO features complete
- All ALX gates pass (ALX-1 through ALX-5)
- ADR-0007: four-layer ganglionic monitoring architecture
- ADR-0008: spec deferrals (Parts I-II live intent, entity generics, interface_layer, distributed quorum)

## Next Steps
1. **V1 Rust emit fix** — fix `and`/`or`/`not`/`=` in require:/ensure: to emit valid Rust (`&&`/`||`/`!`/`==`)
2. **V2 Kani integration** — emit `#[cfg(kani)] #[kani::proof]` harnesses for contracted fns
3. **Scalper persistence** — add in-memory store (TimeSeries/InMemory) + serde JSON persistence to scalper.loom so the emitted Rust loads/saves market data on start/exit
4. **V5 Store struct translation** — emit real Rust structs for store declarations (not just comments)
5. **Merge to main** + `cargo publish`

## Architecture Decision Log
| Date | Decision | Rationale | Status |
| 2025-07-18 | Loom uses single `=` for equality | Language design: simpler syntax | Active |
| 2025-07-18 | Refined types resolve to base in inference | Enables arithmetic on refined params | Active |
| 2025-07-18 | Feature-gated expensive passes | Cargo features for optional Z3/SMT | Active |
| 2025-07-18 | Bare predicates accepted without `self` | Backward compat with `valid_email` pattern | Active |
| 2026-04-06 | Annotation payload collects all tokens between () | Supports @foreign_key(Table.field) syntax | Active |
