# Loom Verification Matrix

**Purpose:** Every correctness claim Loom makes — its test coverage, verification level, 
Rust ecosystem backing, and known gaps. This is the authoritative scorecard.

**Verification Levels (ascending rigour):**
| Level | Symbol | What it proves |
|---|---|---|
| Parse/emit | 🔵 | The syntax parses and emits Rust that compiles |
| Checker | 🟡 | The static checker rejects violations |
| Runtime binary | 🟢 | The emitted binary runs and behaves correctly |
| Property (edge) | 🟢+ | Edge-case loop over boundary inputs passes |
| Property (random) | 🟠 | proptest random sampling — NOT YET integrated |
| Formal | 🔴 | Kani / Z3 symbolic exhaustion — NOT YET integrated |
| Community | ⚫ | Rust ecosystem crate with peer-reviewed correctness |

---

## Pillar 1 — Behavioral Correctness (Design by Contract)

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Preconditions enforced at runtime** | `require: n > 0` | `debug_assert!((n > 0), ...)` | `e2e_test::v1_contracts_emit_compilable_rust` `e2e_test::v1_violated_precondition_panics` | 🟢 binary panics with right message | [`contracts` crate](https://crates.io/crates/contracts) [`design_by_contract`](https://crates.io/crates/design_by_contract) | Release builds: `debug_assert` is stripped. Need `assert!` mode for release contracts. |
| **Postconditions on return value** | `ensure: result > n` | `let _loom_result = ...; debug_assert!(...)` | `e2e_test::v1_ensure_contract_fires_on_return_value` | 🟢 binary passes for valid case | same | No test for postcondition violation path (result violates ensure). |
| **Invariants on property blocks** | `property: forall x: Int, x = x` | `#[test] fn property_...(){ edge_cases loop; assert!(...) }` | `e2e_test::v3_property_test_runs_over_edge_cases` `m109_test::test_m109_property_emits_test_stub` | 🟢+ edge-case binary passes | [`proptest`](https://crates.io/crates/proptest) [`quickcheck`](https://crates.io/crates/quickcheck) | Only 7 boundary values. No random sampling. proptest crate not yet integrated. |

---

## Pillar 2 — Type-Level Correctness

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Refinement types reject invalid values** | `refined Int as PositiveInt: self > 0` | `TryFrom<i64>` with validation | `refinement_test` (8 tests) | 🟡 checker + 🔵 emit | [`newtype_derive`](https://crates.io/crates/newtype_derive) [`nutype`](https://crates.io/crates/nutype) | No binary test that TryFrom rejects invalid. No error-correction codegen binary test. |
| **Dependent types (size bounds)** | `proposition: vec.len() = n` | compile-time phantom size check | `dependent_test` (6 tests) | 🟡 checker | [`typenum`](https://crates.io/crates/typenum) [`generic-array`](https://crates.io/crates/generic-array) | No binary verification. Runtime vector length not checked. |
| **Gradual typing (blame tracking)** | `gradual block ... end` | blame annotations in comments | `gradual_test` (5 tests) | 🔵 emit | [`miette`](https://crates.io/crates/miette) for error display | Blame labels not wired to runtime panics. |
| **Algebraic types (Sum/Product)** | `sum\|product type ...` | Rust `enum`/`struct` | `algebraic_test` (15+ tests) | 🔵 emit | Core Rust | No exhaustiveness check beyond parse. |
| **Typestate protocol** | `state Open -> Closed requires close()` | phantom type state machine | `typestate_test` | 🟡 checker | [`typestate`](https://crates.io/crates/typestate) crate | No binary test that wrong-state call fails to compile. |
| **Session types (protocol duality)** | `session A: send Int; recv Bool` | phantom types + trait impls | `m98_test` (10 tests) | 🔵 emit | [`session_types`](https://crates.io/crates/session_types) | Duality check is parse-time only. No binary protocol enforcement test. |

---

## Pillar 3 — Memory and Concurrency Safety

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Separation logic (ownership)** | `owns x; disjoint x y; frame {x}` | checker annotations → comments | `separation_test` (15 tests) | 🟡 checker | Rust borrow checker is the native impl; [`creusot`](https://github.com/creusot-rs/creusot) for formal | Emitted Rust relies on Rust's borrow checker. Loom checker adds cross-domain ownership checks. No formal proof output. |
| **Side-channel safety** | `timing_safety: constant_time` | constant-time assertions in comments | `sidechannel_test` (6 tests) | 🟡 checker | [`subtle`](https://crates.io/crates/subtle) crate for ct operations | No binary test. `subtle` crate not yet imported in emitted code. |
| **Information flow** | `tainted/untainted` labels | `#[tainted]` annotations | `infoflow_test` | 🟡 checker | [`flowistry`](https://github.com/willcrichton/flowistry) (research) | Taint propagation is static only. No dynamic taint tracking. |

---

## Pillar 4 — Logical and Mathematical Correctness

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Temporal logic ordering** | `precedes A B; always P; never Q` | checker validates sequencing | `temporal_test` (10 tests) | 🟡 checker | [`templar`](https://github.com/) / LTL model checkers | No runtime enforcement. Temporal constraints are documentation only in emitted code. |
| **Category theory laws** | `functor F; monad M` | law verification in checker | `category_test` (12 tests) | 🟡 checker | [`fp-core`](https://crates.io/crates/fp-core) | Functor/monad laws checked at parse time. No generated law tests. |
| **Curry-Howard proofs** | `proof P := ...` | generic fn with type-as-proposition signature | `curryhow_test` (8 tests) + `experiments/proofs/curry-howard/proof.rs` | 🟢 type-system verification | Rust type system IS the proof assistant | — |
| **SMT bridge** | `proposition: x > 0 implies y > 0` | SMT-LIB2 output (feature-gated) | `m100_test` | 🔵 emit | [Z3](https://github.com/Z3Prover/z3) via [`z3`](https://crates.io/crates/z3) crate | SMT output is generated but not yet piped to a solver. Z3 verification not automated. |

---

## Pillar 5 — Probabilistic and Biological Correctness

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Distribution typing** | `distribution: Normal(mu, sigma)` | distribution annotations | `probabilistic_test` (10 tests) | 🟡 checker | [`statrs`](https://crates.io/crates/statrs) | No runtime sampling test. Distributions not wired to statrs at codegen. |
| **Autopoiesis lifecycle** | `lifecycle: autopoietic` | lifecycle methods emitted | `autopoietic_test` (15 tests) | 🔵 emit | — | Runtime lifecycle enforcement not yet a binary test. |
| **Teleos convergence** | `telos: measured_by fitness > 0.9` | fitness threshold assertions | `alx_convergence_test` | 🟡 checker | — | Convergence tracked in test; no running simulation binary. |
| **Evolution vectors** | `evolve: toward ...` | cosine similarity checker | `evolve_test` (m111_test) | 🟡 checker | — | No binary test of cosine similarity computation. |

---

## Pillar 6 — Persistence and Operational Contracts

| Claim | Loom Syntax | Emitted Rust | Tests | Verified | Ecosystem | Gaps |
|---|---|---|---|---|---|---|
| **Relational store** | `store Users: Relational` | Rust struct stub | `m93_m94_test` | 🔵 emit stub only | [`sqlx`](https://crates.io/crates/sqlx) [`diesel`](https://crates.io/crates/diesel) [`sea-orm`](https://crates.io/crates/sea-orm) | **V5 GAP**: No real struct with derives. No sqlx/diesel/sea-orm integration. |
| **Key-value store** | `store Cache: KeyValue` | stub | same | 🔵 stub | [`dashmap`](https://crates.io/crates/dashmap) [`sled`](https://crates.io/crates/sled) | Same as above. |
| **Time-series store** | `store Metrics: TimeSeries` | stub | same | 🔵 stub | [`influxdb`](https://crates.io/crates/influxdb) [`timeseries`](https://crates.io/crates/timeseries) | Same. |
| **Graph store** | `store Graph: Graph` | stub | same | 🔵 stub | [`petgraph`](https://crates.io/crates/petgraph) | Same. |
| **Vector store** | `store Embeddings: Vector` | stub | same | 🔵 stub | [`candle`](https://github.com/huggingface/candle) | Same. |
| **In-memory store** | `store Cache: InMemory(lru)` | stub | same | 🔵 stub | [`lru`](https://crates.io/crates/lru) [`moka`](https://crates.io/crates/moka) | Same. |
| **OpenAPI emit** | `being User { ... }` | OpenAPI YAML | `schema_test` | 🔵 emit | [`utoipa`](https://crates.io/crates/utoipa) | Emit tested, not validated against OpenAPI spec. |

---

## Pillar 7 — Implicit Disciplines (Not Yet Implemented)

| Claim | Status | What Loom Should Emit | Ecosystem Target |
|---|---|---|---|
| **CRUD operations** | ❌ not yet | `create/read/update/delete` methods on stores | sqlx / sea-orm |
| **HATEOAS links** | ❌ not yet | hypermedia link structs in API responses | axum + serde |
| **Markov chain types** | ❌ not yet | transition matrix struct + probability checks | statrs |
| **DAG validation** | ❌ not yet | cycle-detection at construction time | petgraph |
| **CQRS separation** | ❌ not yet | Command/Query trait split, no reads in commands | — |
| **Event sourcing** | ❌ not yet | Event enum + apply/fold pattern | — |

---

## Pillar 8 — Cross-Cutting Infrastructure

| Claim | Loom Syntax | Tests | Verified | Gaps |
|---|---|---|---|---|
| **Audit trail in emitted code** | `correctness_report block` | `correctness_report_test` | 🔵 emit | V7 GAP: generated Rust has no inline explanation of WHY each emit choice was made |
| **Manifest liveness** | `manifest: [file.json, ...]` | `m101_test` | 🟡 checker | File existence checked at loom compile time, not at Rust compile time |
| **Migration safety** | `migration: ...` | `m106_test` | 🔵 emit | No dry-run migration test |
| **Dead code detection** | `minimal: on` | `m107_test` | 🟡 checker | Checker fires; no Rust `#[warn(dead_code)]` enforcement in emitted code |
| **Scenario / BDD** | `scenario:` blocks | `m105_test` | 🔵 emit | Scenarios emit comments; no `#[test]` test generation from them |
| **Taxonomy: domain/role/relates_to** | `domain:`, `role:`, `relates_to:` on beings | `m186_test`, `m187_test` | 🟡 checker | Static annotation + OWL export; no runtime enforcement |
| **Micro-LLM classifiers** | `classifier Name ... retrain_trigger:` | `m185_test`, `m188_test`, `m189_test` | 🟡 checker | Trigger syntax validated; actual model invocation not emitted |
| **Telos/classifier consistency** | `regulate: trigger: classifier:` requires `telos: measured_by:` | `m191_telos_classifier_checker_test` | 🟡 checker | M191 rule enforced at compile time |
| **BIOISO programs (5)** | Full BIOISO w/ taxonomy vocabulary | `experiments/bioiso/*.loom` | 🟡 compile | All 5 programs compile; runtime execution deferred to V4 |

---

## Gap Priority Matrix

Ranked by: (impact × verifiability gap).

| Priority | Gap | Fix | Ecosystem |
|---|---|---|---|
| P0 | Store codegen (V5) | Emit real Rust structs with derives + store-specific impls | sqlx, sled, petgraph, lru |
| P0 | proptest integration (V3+) | Emit `proptest!` macros not just edge-case loops | proptest crate |
| P1 | Contract release-mode (V1+) | Emit `assert!` as an option for production contracts | contracts crate |
| P1 | Kani symbolic verification (V2) | Pipe contracts to Kani as verification harnesses | kani |
| P1 | Audit trail in emitted code (V7) | Inline `// LOOM:` comments explaining each decision | — |
| P2 | Scenario → #[test] | Each scenario block emits a `#[test]` fn | — |
| P2 | CRUD implicit methods (disciplines) | Auto-emit CRUD methods on store-owning beings | sqlx |
| P2 | Distribution → statrs | Wire `distribution:` to statrs sampling + assertions | statrs |
| P2 | subtle for timing-safe ops | Emit `subtle::Choice` for timing_safety blocks | subtle |
| P3 | SMT piping (V2/M100) | Actually invoke Z3 solver on propositions | z3 crate |
| P3 | Category law tests | Emit `#[test]` fns verifying functor/monad laws | — |
| P3 | CQRS implicit split | Auto-enforce Command/Query separation from store access | — |

---

## Canonical Experiments Reference

Community experiments that independently validate Loom's claims exist in `experiments/`.

| File | Claim tested | Result |
|---|---|---|
| `experiments/verification/v1_contracts.loom` | require:/ensure: fire at runtime | ✅ V1 verified |
| `experiments/verification/v3_property_tests.loom` | property: emits edge-case test | ✅ V3 verified |

---

## How to Add a New Claim

1. Add a row to the relevant Pillar table above.
2. Write the Loom syntax example.
3. Identify the emitted Rust.
4. Write the test (unit, e2e binary, or property) and link it.
5. Record the verification level symbol.
6. Add an experiment file in `experiments/verification/` with a human-readable claim spec.
7. Record any Rust ecosystem crate that independently validates the same property.
8. Document known gaps honestly.

**A claim with only 🔵 parse/emit is a declaration, not a proof.**
The goal is to get every correctness-critical claim to 🟢 or higher.
