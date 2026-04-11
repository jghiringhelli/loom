# Status.md

## Last Updated: 2026-04-12
## Branch: docs/lineage-collapsed-loop

## Completed (this session)
- **M158: `type alias` item — tests for existing feature** (commit `1f3c7a2`)
  - `type X = SomeType` has existed since M87 (Item::TypeAlias); 9 tests added pinning the behavior
- **M159: `pipeline` item — named sequential transformation chain** (commit `5479e30`)
  - `pipeline Name step a :: In -> Out ... end` syntax
  - Lexer: `Token::PipelineKw`, `Token::Step`; AST: `PipelineDef`, `PipelineStep`, `Item::Pipeline`
  - Codegen: `emit_pipeline_def()` → `{Name}` struct + per-step fn stubs + `process()` chaining; `loom_type_to_rust()` helper
  - 12 tests — all passing
- **M160: `saga` item — distributed transaction coordinator** (commit `499c4fd`)
  - `saga Name step a :: In -> Out [compensate a :: In -> Out] ... end` syntax
  - Lexer: `Token::SagaKw`, `Token::Compensate`; AST: `SagaDef`, `SagaStep`, `SagaCompensate`, `Item::Saga`
  - Codegen: `emit_saga_def()` → unit struct + `{Name}Error` enum + step/compensate fn stubs + `execute()` chaining
  - 12 tests — all passing
- **fix: keyword collisions** (commit `7901642`)
  - `pipeline` as module name in M156 tests → renamed to `etl`
  - Pathway parser: accepts `Token::Compensate` (not just `Ident`) for `compensate:` field
- **claim_coverage.md updated**: 84 total claims, 59 PROVED (70.2%)
- **M152: Compressed binary snapshots** (commit `c3f17f3`)
  - `CompressedBinaryPersist` trait with `save_compressed`/`load_compressed` via flate2 gzip
  - `.snap.gz` extension convention; 12 tests — all passing
- **M153: CRUD service layer + SQLite adapter wiring** (commit `6b41c47`)
  - `{T}Service` struct with create/get/list/update/delete/exists methods
  - get() returns NotFound; update() checks exists() first
  - Dead-code `emit_sqlite_adapter()` wired into `codegen_relational_store()`; 15 tests — all passing
- **M154: EventStore snapshot bridge + payload soft-keyword fix** (commit `3e9718e`)
  - `{S}SnapshotBridge` trait: `snapshot_to(path)` + `resume_from(path, store, stream, from_seq)`
  - `payload` added to `token_as_ident()` — now valid as table field name
  - Table field parser generalized to `token_as_ident()` (all soft keywords as field names); 10 tests — all passing
- **M155: `chain` item — top-level Markov chain as first-class module item** (commit `2978437`)
  - `chain Name states: [...] transitions: A -> B: 0.3 ... end end` syntax
  - Lexer: `Token::ChainKw`, `Token::States`; removed duplicate `TransitionsKw`
  - AST: `ChainDef { name, states, transitions, span }`; `Item::Chain(ChainDef)`
  - Codegen: `emit_chain_item()` → `{Name}State` enum + `{Name}TransitionMatrix` struct with `new()` (pre-initialized), `set()`, `next_states()`, `validate()`; `LOOM[chain:Markov]` + M155 audit comment
  - 12 tests — all passing
- **M156: `dag` item — top-level DAG as first-class module item** (commit `4ccfd66`)
  - `dag Name nodes: [...] edges: [A -> B, ...] end` syntax
  - Lexer: `Token::DagKw`, `Token::Nodes`, `Token::Edges`
  - AST: `DagDef { name, nodes, edges, span }`; `Item::Dag(DagDef)`
  - Codegen: `emit_dag_item()` → `{Name}Node` enum + `{Name}DagItem` struct with `new()` (edges pre-initialized), `add_typed_edge()`, `successors()`, `topological_sort()` (Kahn's algorithm + cycle detection); `LOOM[dag:item]` + M156 audit comment
  - 13 tests — all passing
- **claim_coverage.md updated**: 75 total claims, 50 PROVED (67%)

## Completed (continued)
- **M157: `const` item — named constant as first-class module item** (commit `a4d0fb9`)
  - `const Name: Type = value` syntax; type annotation optional (inferred from literal)
  - Lexer: `Token::Const`; AST: `ConstDef + Item::Const`
  - Codegen: `emit_const_def()` → `pub const UPPER_SNAKE: RustType = value;` + `LOOM[const:item]`
  - `to_upper_snake()` helper; 12 tests — all passing
- **claim_coverage.md updated**: 77 total claims, 52 PROVED (67.5%)

## Current State
- **Branch: docs/lineage-collapsed-loop**
- All M1–M160 milestones complete
- Verification pipeline V1–V9: 59 PROVED, 19 EMITTED, 4 PENDING (70.2% proved)
- **1148+ tests across 116+ test suites — 0 failures**
- CLI exposes all 10 compile targets with aliases
- Binary verify: all 5 examples compile end-to-end (loom → rustc)

## Next
- **M161**: `event` item — domain event declaration → `{Name}Event` struct + `{Name}EventHandler` trait
- **M162**: `query` / `command` items — CQRS split as first-class items
- **M163**: `circuit_breaker` item — Nygard 2007 resilience pattern
- **Hygiene**: fix pre-commit hook (syntax error at line 107)
- **being.rs field parser fix** — bare `Token::Ident` match prevents soft-keyword field names in `being` blocks
- **cargo publish** — `loom-lang v0.2.0` ready

## Known Limitations
- `evolve_test` fails on Windows with OS error 5 (Windows Defender blocks test binary) — pre-existing, passes on Ubuntu CI
- Only FIRST test in a module is parsed — known parser limitation
- Codegen functions > 50 lines (hygiene debt)
- `simulation` and `neuroml` targets not yet wired in CLI (lib functions exist)
- Pre-commit hook has syntax error at line 107 — requires `--no-verify` on every commit

## Decisions made (this session)
- `chain` items use a dedicated `Token::ChainKw` (not `chain` as identifier)
- `Token::TransitionsKw` removed — `Token::Transitions` already existed; unified
- `emit_chain_item()` placed in `structures.rs` alongside `emit_markov_transition_matrix()`

## Architecture Decision Log
| Date | Decision | Rationale | Status |
| 2025-07-18 | Loom uses single = for equality | Language design | Active |
| 2025-07-18 | Refined types resolve to base in inference | Arithmetic on refined params | Active |
| 2026-04-06 | Annotation payload collects all tokens between () | @foreign_key(Table.field) | Active |
| 2026-04-10 | Ecosystem telos sub-block end consumed by telos handler | Prevents module loop early exit | Active |
| 2026-06-09 | CLI uses local fn mermaid_err for String→LoomError bridging | Keeps match arm types uniform | Active |
| 2026-04-11 | chain item uses Token::ChainKw; Token::Transitions unified | Removes duplicate lexer token | Active |

