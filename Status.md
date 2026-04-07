# Status.md

## Last Updated: 2026-04-13
## Session Summary
M102 (ProvenanceChecker) and M103 (BoundaryChecker) wired into compile pipeline.
Fixed ALX-1-feature-matrix.loom (property colon syntax, @mortal/@corrigible telomere/modifiable_by,
leading annotations in type fields). 748 tests passing, all 6 ALX gates pass.

## Test Count
- **Total tests:** 748 passing ✅, 0 failed
- **ALX gate:** ✅ ALX-1 through ALX-4 all pass (S_realized ≥ 0.85)
- **M102 Provenance:** 6 tests — @provenance parses, @pii+@provenance error, chain warning, being with provenance
- **M103 Boundary:** 6 tests — boundary: parses, ghost export error, private-type leak, seal parses

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
| M102: Provenance | ✅ Done | ProvenanceChecker, @provenance annotations, W3C PROV-DM rules (Moreau 2013) |
| M103: Boundary | ✅ Done | BoundaryBlock, BoundaryChecker, information hiding (Parnas 1972) |
| M104: Journal | ✅ Done | JournalBlock, JournalChecker (episodic memory — Tulving 1972) |
| M105: Scenario | ✅ Done | ScenarioBlock, ScenarioChecker, scenario: → #[test] stubs (BDD — Beck 2002) |
| M106: Migration | ✅ Done | MigrationBlock, MigrationChecker (interface evolution contract) |
| M107: Minimal | ✅ Done | MinimalChecker (dead declaration detection — unused sense + regulate field) |
| M108: Diagram Emit | ✅ Done | compile_mermaid_c4/sequence/state/flow — GS diagram-emitting property |
| M109: Property Tests | ✅ Done | PropertyBlock, PropertyChecker (QuickCheck 2000 → fast-check) |
| M110: UseCase | ✅ Done | UseCaseBlock, UseCaseChecker (Jacobson 1992 triple-derivation) |
| ALX-1 through ALX-4 | ✅ Done | All 6 ALX gates pass, S_realized ≥ 0.85 |

## Current Context
- Branch: `docs/lineage-collapsed-loop`
- **748 tests passing**, 0 failures
- All M1-M110 milestones implemented and tested
- All ALX gates pass
- M78-M91 scientific primitives: all passing (see m78_m82_test, m83_m87_test, m84_test, m85_test, m86_m92_test, m88_test, m89_test)

## Next Steps
1. **publish-merge**: Merge `docs/lineage-collapsed-loop` → `main`, then `cargo publish loom-lang v0.1.0`
2. arXiv submission: BIOISO convergence paper (`docs/bioiso-loom-convergence.md`)
3. Fix GitHub Actions (they were cluttering inbox — review `.github/workflows/`)
4. BIOISO finance/crypto demo program (see `docs/bioiso-finance-design.md` + aegis/automaton inspiration)

## Architecture Decision Log
| Date | Decision | Rationale | Status |
| 2025-07-18 | Loom uses single `=` for equality | Language design: simpler syntax | Active |
| 2025-07-18 | Refined types resolve to base in inference | Enables arithmetic on refined params | Active |
| 2025-07-18 | Feature-gated expensive passes | Cargo features for optional Z3/SMT | Active |
| 2025-07-18 | Bare predicates accepted without `self` | Backward compat with `valid_email` pattern | Active |
| 2026-04-06 | Annotation payload collects all tokens between () | Supports @foreign_key(Table.field) syntax | Active |
