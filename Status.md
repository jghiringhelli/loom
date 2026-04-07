# Status.md

## Last Updated: 2026-04-06
## Session Summary
M83-M87: sense stdlib and tensor types. M93/M94: operational and analytical store checkers with codegen stubs. 621 tests passing.

## Test Count
- **Total tests:** 621/621 passing ✅
- **M93 operational stores:** Relational (unique PK, field uniqueness, FK references), KeyValue (TTL validation, hashed key hint), Document (schema-flexible, dynamic Json fields)
- **M94 analytical stores:** Columnar (scalar field requirement, partition key), Snowflake (fact+dimension validation, measure annotations), Hypercube (axis/measure structure, >12 dimension warning per Gray 1996)
- **Rust codegen:** Typed struct stubs with connector comments for all store kinds

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
| ALX-1 through ALX-4 | ✅ Done | Self-fix loop experiments all green |

## Current Context
- Branch: `docs/lineage-collapsed-loop`
- All 621 tests passing (594 baseline + 6 M93/M94 + 21 M83-M87/M95-M97)
- Annotation parser extended: `@annotation(Ident.field)` syntax now supported

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
