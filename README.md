# loom

**loom** is an AI-native declaration language that compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema from a single source file — and runs an autonomous BIOISO colony that continuously optimizes nine real-world NP-hard domains using a three-tier AI synthesis stack.

It is designed around one constraint: every architectural decision, behavioral contract, and data-sensitivity obligation must be expressible in a form that a stateless reader — an AI assistant with no persistent memory — can derive correct output from alone. This is the [Generative Specification](docs/publish/white-paper.md) principle.

**314 lib tests · 5 emission targets · 9 BIOISO domains · T1/T2/T3 synthesis tiers · TLA+ formal verification · all examples rustc-verified**

---

## Why loom

Traditional code has three structural problems that compound as AI becomes the primary executor:

1. **Ambiguity** — natural-language intent + code must be reconciled every session. loom makes intent the source of truth, not comments alongside code.
2. **Correctness gap** — Rust's type system is powerful but expressing contracts, privacy rules, effect tiers, and lifecycle protocols requires boilerplate that most developers skip. loom makes them the default, not the exception.
3. **Knowledge gap** — proven disciplines (refined types, session types, separation logic, information flow) are hard to learn and easy to skip. loom enforces them structurally; the developer cannot accidentally bypass them.

loom bridges theory and implementation — a gap that has persisted across the entire history of computer science.

---

## Emission targets

| Target | CLI | API |
|--------|-----|-----|
| Rust | `loom compile src.loom` | `compile(&src)` |
| TypeScript | `loom compile src.loom --target ts` | `compile_typescript(&src)` |
| WebAssembly Text | `loom compile src.loom --target wasm` | `compile_wasm(&src)` |
| OpenAPI 3.0 YAML | `loom compile src.loom --target openapi` | `compile_openapi(&src)` |
| JSON Schema | `loom compile src.loom --target schema` | `compile_json_schema(&src)` |

---

## Language features

### Type system
```loom
type Point   = x: Float, y: Float end                          -- product type
enum Shape   = | Circle of Float | Rect of Float * Float end   -- sum type
type Email   = String where valid_email end                     -- refined type
type Pair<A,B> = first: A, second: B end                       -- generics
```

### Functions and contracts
```loom
fn transfer :: Float<usd> -> Account -> Effect<[DB], Account>
  require: amount > 0.0
  ensure:  result.balance >= 0.0
  amount
end
```
Contracts emit as `debug_assert!` in Rust. They are also the input for Kani formal proofs.

### Effect tracking
```loom
fn fetch_user  :: Int  -> Effect<[IO, DB], User>
fn pure_add    :: Int  -> Int -> Int                       -- no effects, pure
fn send_email  :: User -> Effect<[IO@irreversible], Unit>  -- consequence tier
```

### Semantic type constructs

| Construct | Syntax | What it enforces |
|-----------|--------|-----------------|
| Units of measure | `Float<usd>`, `Float<m/s>` | Arithmetic unit consistency |
| Privacy labels | `@pii @gdpr @pci @hipaa @never-log @encrypt-at-rest` | Regulatory co-occurrence rules |
| Algebraic properties | `@idempotent @commutative @exactly-once @at-most-once` | Retry safety, operation ordering |
| Typestate / lifecycle | `lifecycle Payment :: Pending -> Completed -> Refunded` | Valid state transitions |
| Information flow | `flow secret :: Password, Token` | Secret → public leak prevention |
| Refinement types | `type PositiveInt = Int where self > 0` | Predicate-checked subtypes |
| Session types | `channel :: !Send.?Ack.End` | Protocol-enforced ordering |
| Dependent types | `fn nth :: List<A> -> n:Nat -> A where n < list.len` | Length-indexed safety |

---

## BIOISO — Biological Isomorphic Optimizer

loom introduces a fifth-tier construct — the **BIOISO** — for systems where parameter adjustment is structurally insufficient and the *control law graph itself* must be rewired:

```loom
being FusionPlasmaController
  telos:
    confinement_quality_h98 >= 1.05   -- ITER Q=10 target
    disruption_probability  <= 0.02   -- safe operating envelope

  evolve: derivative_free
    objective: minimise drift_score(state)
    budget:    200

  rewire:
    trigger:    drift_exceeds 0.18
    candidates:
      - control_law_graph
      - mode_classifier_model
    selection:  fitness_guided
    cooldown:   5
  end

  plasticity:
    observe:   [confinement_quality_h98, disruption_probability]
    adjust_on: regime_transition | disruption_precursor
  end
```

The `rewire:` block is the load-bearing Tier 5 primitive. When a plasma instability class transitions to a novel regime, no parameter within the current control law recovers confinement — the graph must be structurally replaced. This is the formal distinction between Tier 4 (learning-based optimisation) and Tier 5 (biological isomorphic optimisation).

### The 5-Tier Optimization Taxonomy

| Tier | Class | Example | Why lower tier fails |
|------|-------|---------|---------------------|
| 1 | Heuristics | Hill-climbing, greedy | No landscape model |
| 2 | Meta-heuristics | Genetic algorithms, SA | Fixed neighbourhood structure |
| 3 | Hyper-heuristics | Algorithm selection, learning | Fixed algorithm space |
| 4 | Learning-based | Neural architecture search, Bayesian opt | Stationary fitness assumption |
| **5** | **BIOISO** | **Fusion plasma, AMR coevolution** | **Non-stationary landscape — fitness surface itself evolves** |

### The 9 BIOISO Colony Domains

The autonomous colony continuously backtests all nine domains using historical data:

| Domain | Why Tier 5 | Key non-stationarity |
|--------|-----------|---------------------|
| AMR Coevolution | Resistance mechanisms co-evolve with antibiotics | Fitness landscape = moving target |
| Flash Crash Dynamics | Market microstructure transitions between regimes | Causal graph shifts at phase boundary |
| Adaptive JIT Compilation | Workload topology changes with application phase | No fixed cost model |
| Protein Drug Resistance | Binding site evolves; drug-protein co-evolution | Landscape bifurcates with each mutation |
| ICS/SCADA Zero-Day Defense | Novel attack vectors not in training data | Adversarial non-stationarity |
| Quantum Error Mitigation | Decoherence channels shift with hardware drift | Noise topology changes dynamically |
| Climate Intervention | Earth system tipping points are path-dependent | Topology collapse = irreversible |
| Fusion Plasma Control | L-H transition / novel instability classes | Regime transition eliminates current law |
| Adaptive Self-Assembly | Configuration-space bifurcation collapses pathways | Protocol graph topology is path-dependent |

### Three-tier AI synthesis stack

```
T1 (Polycephalum)  — rule engine, microsecond proposals
    ↓ escalate on 3 consecutive misses
T2 (Ganglion/Haiku) — LLM mutation proposals, black-box search
    ↓ escalate when drift.score > 0.35 (structural rewire warranted)
T3 (Brain/Sonnet)  — full StructuralRewire evaluation, synthesis of new protocol graphs
```

Running live on Railway: `loom runtime experiment --ticks 50000 --tick-ms 5000`

---

## Formal verification

| Tier | Mechanism | Tool | Status |
|------|-----------|------|--------|
| V1 Runtime contracts | `require:`/`ensure:` → `debug_assert!` | `rustc` | PROVED |
| V2 Formal proofs | Contracts → `#[kani::proof]` harnesses | `cargo kani` | EMITTED |
| V3 Property tests | `forall:` → proptest blocks | `cargo test` | EMITTED |
| V4 Session types | Protocol steps → affine phantom types | `rustc` | PROVED |
| V8 Convergence | `telos:` → `ConvergenceState` + TLA+ spec + TLC config | `loom verify --tla` | EMITTED |
| V9 Dependent types | `proof:` → Dafny method stubs | `dafny verify` | EMITTED |

`loom verify --tla myagent.loom` writes `<being>_convergence.tla` and `<being>_convergence.cfg` and runs TLC if on PATH.

---

## Install

```sh
cargo build --release
# binary: target/release/loom (or loom.exe on Windows)
```

Or run directly:

```sh
cargo run -- compile examples/01-hello-contracts.loom
cargo run -- compile examples/02-payment-api.loom --target openapi
cargo run -- verify examples/tier5/fusion_plasma.loom --tla
```

See [Getting Started](docs/getting-started.md) for a 10-minute walkthrough.

---

## Examples

| File | What it demonstrates |
|------|---------------------|
| [`examples/01-hello-contracts.loom`](examples/01-hello-contracts.loom) | Contracts (`require:`/`ensure:`), inline tests |
| [`examples/02-payment-api.loom`](examples/02-payment-api.loom) | Units of measure, privacy labels, OpenAPI inference |
| [`examples/03-typestate-lifecycle.loom`](examples/03-typestate-lifecycle.loom) | Typestate protocol, session-typed channel |
| [`examples/04-finance-gbm.loom`](examples/04-finance-gbm.loom) | GBM, Black-Scholes, VaR, stochastic processes |
| [`examples/05-autonomous-agent.loom`](examples/05-autonomous-agent.loom) | Biological agent, `regulate:`, `evolve:`, `@mortal @corrigible @sandboxed` |
| [`examples/tier5/fusion_plasma.loom`](examples/tier5/fusion_plasma.loom) | Tier 5 BIOISO — fusion plasma confinement control |
| [`examples/tier5/adaptive_self_assembly.loom`](examples/tier5/adaptive_self_assembly.loom) | Tier 5 BIOISO — nanostructure protocol graph rewiring |
| [`examples/tier5/amr_coevolution.loom`](examples/tier5/amr_coevolution.loom) | Tier 5 BIOISO — antimicrobial resistance coevolution |

---

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/getting-started.md`](docs/getting-started.md) | 10-minute install → compile → run guide |
| [`docs/language-spec.md`](docs/language-spec.md) | Complete language reference |
| [`docs/taxonomy.md`](docs/taxonomy.md) | 5-tier optimization taxonomy and BIOISO ontology |
| [`docs/bioiso-loom-convergence.md`](docs/bioiso-loom-convergence.md) | BIOISO formal disciplines and compiler enforcement |
| [`docs/foundations.md`](docs/foundations.md) | Theoretical foundations (type theory, PLN, OU convergence) |
| [`docs/pln.md`](docs/pln.md) | Probabilistic reasoning and telos convergence estimates |
| [`docs/lifecycle.md`](docs/lifecycle.md) | Full software lifecycle spec |
| [`docs/publish/white-paper.md`](docs/publish/white-paper.md) | Academic white paper |
| [`docs/roadmap.md`](docs/roadmap.md) | Full milestone roadmap |
| [`docs/TechSpec.md`](docs/TechSpec.md) | Compiler architecture |
| [`docs/deploy-railway.md`](docs/deploy-railway.md) | Colony deployment on Railway |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to submit issues, propose features, and open pull requests.

loom welcomes contributions in:
- New emission targets (LLVM IR, C, Python)
- Verification pipeline (Prusti, Lean4, Coq)
- Standard library modules
- Language examples and tutorials
- Editor extensions (VS Code, Neovim)
- New BIOISO domain specs

---

## License

MIT — see [LICENSE](LICENSE).
