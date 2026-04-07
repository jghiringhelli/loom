# Status.md

## Last Updated: 2026-04-11
## Session Summary
M100/M101: SMT contract verification bridge + manifest: documentation liveness. Also completed M106, M107, M108, M109, M110. 724 tests passing (up from 678).

## Test Count
- **Total tests:** 724 passing ✅
- **M100 SMT Bridge:** 8 new tests — checker exists, skipped without Z3, precondition/postcondition/arithmetic translation, contradiction detection, fn without contracts, lineage comment
- **M101 Manifest:** 6 new tests — manifest parses, missing file error, reflects unknown symbol warning, empty manifest valid, multiple artifacts, being without manifest valid
- **M106 Migration:** 6 new tests — migration block parses, non-breaking without adapter error, duplicate name error, autopoietic info hint, breaking with adapter, no-migration no-hint
- **M107 Minimal:** 6 new tests — unused sense warning, regulate on nonexistent field error, no being no check, no matter no Rule 2, empty sense no warning, no autopoietic-hint
- **M108 Journal+Scenario:** journal checker (keep 0 error, missing evolve/telos warnings, autopoietic warning) + scenario checker (empty given/when/then, within 0, autopoietic warning)
- **M109 Property:** 6 new tests — parses, zero samples error, shrink default, samples default, multiple properties, emits test stub
- **M110 UseCase:** 6 new tests — full parse, acceptance criteria, empty acceptance warning, duplicate criterion error, postcondition=precondition warning, tautological precondition warning

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
| M95: Specialized Stores | ✅ Done | Graph (provenance/weight annotations, edge referential integrity), TimeSeries (retention/resolution validation), Vector (HNSW/IVFFlat/LSH/BruteForce index validation) |
| M96: Local Stores | ✅ Done | InMemory (LRU/LFU/ARC eviction per Megiddo 2003, capacity validation), FlatFile (Parquet/Arrow/HDF5/CSV/JsonLines/MsgPack + compression) |
| M97: Distributed Stores | ✅ Done | Distributed MapReduce (Dean & Ghemawat 2004): map/reduce/combine pipeline; DistributedLog (Kreps 2011): partitioned append-only log with consumer offset declarations |
| M98: Session Types | ✅ Done | session/role/send/recv/duality, SessionChecker (Honda 1993) |
| M99: Effect Handlers | ✅ Done | effect/operation/handle/with, EffectHandlerChecker (Plotkin & Pretnar 2009) |
| M100: SMT Bridge | ✅ Done | SmtBridgeChecker, SMT-LIB2 translation, Z3-feature-gated (Hoare 1969 → Dijkstra 1975 → Z3) |
| M101: Manifest Liveness | ✅ Done | ManifestChecker, artifact existence + symbol reflects checking |
| M104: Journal | ✅ Done | JournalBlock, JournalChecker (episodic memory — Tulving 1972) |
| M105: Scenario | ✅ Done | ScenarioBlock, ScenarioChecker (BDD Given/When/Then — Beck 2002) |
| M106: Migration | ✅ Done | MigrationBlock, MigrationChecker (interface evolution contract) |
| M107: Minimal | ✅ Done | MinimalChecker (dead declaration detection — unused sense + regulate field) |
| M109: Property Tests | ✅ Done | PropertyBlock, PropertyChecker (QuickCheck 2000 → fast-check) |
| M110: UseCase | ✅ Done | UseCaseBlock, UseCaseChecker (Jacobson 1992 triple-derivation) |
| ALX-1 through ALX-4 | ✅ Done | Self-fix loop experiments all green |

## Current Context
- Branch: `docs/lineage-collapsed-loop`
- **678 tests passing** (634 baseline + 12 M98/M99 + additional from M85/M88/M84)
- Session types: `session` keyword, role blocks, duality checking (send/recv correspondence)
- Effect handlers: `effect` keyword, `operation` declarations, `handle...with` blocks, exhaustiveness checking
- Parser: `describe:` now valid anywhere in fn body (not just before `::`)
- Token: `implements` added to `token_as_ident()` enabling `@implements(...)` annotations

## Next Steps
1. **publish-merge**: Merge `docs/lineage-collapsed-loop` → `main`, then `cargo publish loom-lang v0.1.0`
2. arXiv submission: BIOISO convergence paper (`docs/bioiso-loom-convergence.md`)
3. Fix GitHub Actions (they were cluttering inbox — review `.github/workflows/`)

## Architecture Decision Log
| Date | Decision | Rationale | Status |
| 2025-07-18 | Loom uses single `=` for equality | Language design: simpler syntax | Active |
| 2025-07-18 | Refined types resolve to base in inference | Enables arithmetic on refined params | Active |
| 2025-07-18 | Feature-gated expensive passes | Cargo features for optional Z3/SMT | Active |
| 2025-07-18 | Bare predicates accepted without `self` | Backward compat with `valid_email` pattern | Active |
| 2026-04-06 | Annotation payload collects all tokens between () | Supports @foreign_key(Table.field) syntax | Active |
