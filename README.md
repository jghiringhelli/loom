# Loom

**Loom** is an AI-native declaration engine that compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema from a single source file.

It is designed around one constraint: every architectural decision, behavioral contract, and data-sensitivity obligation must be expressible in a form that a stateless reader — an AI assistant with no persistent memory — can derive correct output from alone. This is the [Generative Specification](docs/publish/white-paper.md) principle.

**904 tests · 5 emission targets · 116 milestones complete · LPN AI-to-AI protocol · all examples rustc-verified ✓**

---

## Why Loom

Traditional code has three structural problems that compound as AI becomes the primary executor:

1. **Ambiguity** — natural-language intent + code must be reconciled every session. Loom makes intent the source of truth, not comments alongside code.
2. **Correctness gap** — Rust's type system is powerful but expressing contracts, privacy rules, effect tiers, and lifecycle protocols requires boilerplate that most developers skip. Loom makes them the default, not the exception.
3. **Knowledge gap** — proven disciplines (refined types, session types, separation logic, information flow) are hard to learn and easy to skip. Loom enforces them structurally; the developer cannot accidentally bypass them.

Loom bridges theory and implementation — a gap that has persisted across the entire history of computer science.

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
Contracts emit as `debug_assert!` in Rust when the body is implemented. They are also the input for Kani formal proofs.

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

### Biological / autonomous agent constructs

Loom supports a class of constructs for autonomous, self-regulating agents:

```loom
being ScalpingAgent
  regulate:     drawdown < 0.02
  telomere:     trades < 5000
  epigenetic:   if vol_regime = high then ou_sigma *= 1.5
  autopoietic:  if drawdown > 0.015 then pause 60s
  @mortal @corrigible @sandboxed
end
```

These emit safety checks, kill-switches, and regime-adaptation logic in the generated Rust.

### Stochastic processes and finance

```loom
process ScalpSignal = OrnsteinUhlenbeck { theta: 2.0, mu: 0.0, sigma: 0.15 }
type TailRisk = Float<usd> where distribution = Cauchy { location: 0.0, scale: 0.02 }
```

### Module system
```loom
module PaymentService
describe: "Handles payment processing"

interface Repository
  fn find :: Int -> Effect<[DB], User>
  fn save :: User -> Effect<[DB], Unit>
end

import UserRepository
implements Repository
provides: process_payment
requires: UserRepository
end
```

### GS constructs (self-describing, auditable, verifiable)
```loom
describe: "Computes final invoice price with tax"
@author("billing-team")
@decision("Use exclusive tax to match EU VAT rules")

invariant non_negative_balance :: balance >= 0.0

test transfer_reduces_balance ::
  transfer(100.0 : Float<usd>) |> result.balance = initial - 100.0
end
```

### OpenAPI REST inference

Loom derives full REST semantics from type signatures — no annotations required:

```loom
fn get_order    :: Int   -> Effect<[DB], Order>        -- GET  /orders/{id}
fn create_order :: Order -> Effect<[DB], Order>        -- POST /orders  (201)
fn delete_order :: Int   -> Effect<[DB], Unit>         -- DELETE /orders/{id}
fn list_orders  :: Unit  -> Effect<[DB], List<Order>>  -- GET  /orders
```

`@idempotent` on POST promotes it to PUT. `@exactly-once` emits `x-retry-policy: never`.

---

## LPN — AI-to-AI Protocol

Loom ships a minimal AI-to-AI wire format (`.lp` files) for orchestrating the compiler pipeline:

```lp
# Tier 1: atomic ops
EMIT rust PaymentAPI FROM examples/02-payment-api.loom
CHECK all examples/02-payment-api.loom

# Tier 2: compound ops
IMPL ScalpingAgent USING [M41,M55,M84-M89] EMIT rust VERIFY compile+types

# Tier 3: named experiments
ALX n=7 domain=biotech coverage>=0.95 emit=rust verify=compile+run evidence=store
SCALPER ticks=10000 ou_theta=2.0 ou_sigma=0.15 emit=rust run=backtest
```

```sh
loom lpn experiment.lp
loom lpn experiment.lp --format json   # machine-readable output
```

LPN eliminates prompt ambiguity between AI agents. Each instruction is unambiguous, token-efficient, and fully typed.

---

## Install

```sh
cargo build --release
# binary at target/release/loom (or target/release/loom.exe on Windows)
```

Or run directly:

```sh
cargo run -- compile examples/01-hello-contracts.loom
cargo run -- compile examples/02-payment-api.loom --target openapi
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

---

## Milestone index (M1–M116)

<details>
<summary>Click to expand all 116 milestones</summary>

| # | Feature | # | Feature |
|---|---------|---|---------|
| M1 | Type inference (Hindley-Milner) | M2 | Pattern exhaustiveness |
| M3 | WebAssembly back-end | M4 | Language Server Protocol |
| M5 | Dependency injection | M6 | Standard library mappings |
| M7 | Generic functions | M8 | Multi-module compilation |
| M9 | Inline Rust escape hatch | M10 | Numeric coercion (`as`) |
| M11 | First-class iteration | M12 | `Option`, `Result`, `?` |
| M13 | `describe:` + audit annotations | M14 | `invariant:` + consequence tiers |
| M15 | `test:` blocks + `ensure:` | M16 | `import` + `interface`/`implements` |
| M17 | TypeScript emission | M18 | OpenAPI 3.0 + JSON Schema |
| M19 | Units of measure | M20 | Privacy labels |
| M21 | Algebraic properties | M22 | Typestate / lifecycle |
| M23 | Information flow labels | M24 | Kani formal proof harnesses |
| M25 | Proptest property generation | M26 | Session type channels |
| M27–M55 | Struct translation, stores, CRUD, HATEOAS, DAG, Markov, event sourcing, CQRS | |
| M56–M89 | Biological constructs, stochastic processes, ecosystem blocks, stdlib | |
| M90 | Finance stdlib | M91 | Quantum stdlib |
| M92–M116 | Verification pipeline, Dafny, TLA+, audit headers, LPN | |

All 116 milestones are ✅ complete. See [`docs/roadmap.md`](docs/roadmap.md) for detail.
</details>

---

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/getting-started.md`](docs/getting-started.md) | 10-minute install → compile → run guide |
| [`docs/language-spec.md`](docs/language-spec.md) | Complete language reference |
| [`docs/lifecycle.md`](docs/lifecycle.md) | Full software lifecycle spec |
| [`docs/publish/white-paper.md`](docs/publish/white-paper.md) | Academic white paper (arXiv preprint) |
| [`docs/roadmap.md`](docs/roadmap.md) | Full milestone roadmap |
| [`docs/TechSpec.md`](docs/TechSpec.md) | Compiler architecture |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to submit issues, propose features, and open pull requests.

Loom welcomes contributions in:
- New emission targets (LLVM IR, C, Python)
- Verification pipeline (Prusti, Lean4, Coq)
- Standard library modules
- Language examples and tutorials
- Editor extensions (VS Code, Neovim)

---

## License

MIT — see [LICENSE](LICENSE).


