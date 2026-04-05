# Loom

**Loom** is an AI-native functional language that compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema from a single source file.

It is designed around one constraint: every architectural decision, behavioral contract, and data-sensitivity obligation must be expressible in a form that a stateless reader — an AI assistant with no persistent memory — can derive correct output from alone. This is the [Generative Specification](docs/publish/white-paper.md) principle.

**311 tests · 5 emission targets · 23 milestones complete**

---

## Emission targets

| Target | CLI flag | API |
|--------|----------|-----|
| Rust | `loom compile src.loom` | `compile(&src)` |
| TypeScript | `loom compile src.loom --target ts` | `compile_typescript(&src)` |
| WebAssembly | `loom compile src.loom --target wasm` | `compile_wasm(&src)` |
| OpenAPI 3.0 | `loom compile src.loom --target openapi` | `compile_openapi(&src)` |
| JSON Schema | `loom compile src.loom --target schema` | `compile_json_schema(&src)` |

---

## Language features

### Type system
```loom
type Point = x: Float, y: Float end                          -- product type
enum Shape = | Circle of Float | Rect of Float * Float end   -- sum type
type Email = String where valid_email end                     -- refined type
type Pair<A, B> = first: A, second: B end                    -- generics
```

### Functions and contracts
```loom
fn transfer :: Float<usd> -> Account -> Effect<[DB], Account]
  require: amount > 0.0
  ensure:  result.balance >= 0.0
  amount
end
```

### Effect tracking
```loom
fn fetch_user    :: Int  -> Effect<[IO, DB], User]    -- IO + DB effects
fn pure_add      :: Int  -> Int -> Int                -- no effects
fn send_email    :: User -> Effect<[IO@irreversible], Unit]  -- consequence tier
```

### Semantic type constructs (unique to Loom)

| Construct | Syntax | What it enforces |
|-----------|--------|-----------------|
| Units of measure | `Float<usd>`, `Float<m/s>` | Arithmetic unit consistency |
| Privacy labels | `@pii @gdpr @pci @hipaa @never-log @encrypt-at-rest` | Regulatory co-occurrence rules |
| Algebraic properties | `@idempotent @commutative @exactly-once @at-most-once` | Retry safety, operation ordering |
| Typestate / lifecycle | `lifecycle Payment :: Pending -> Completed -> Refunded` | Valid state transitions |
| Information flow | `flow secret :: Password, Token` | Secret → public leak prevention |

### Module system
```loom
module PaymentService
describe: "Handles payment processing"

interface Repository
  fn find :: Int -> Effect<[DB], User]
  fn save :: User -> Effect<[DB], Unit]
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
  transfer(100.0 : Float<usd>) |> result.balance == initial - 100.0
end
```

### Higher-order functions
```loom
fn totals :: List<Order> -> List<Float<usd>]
  orders |> map(fn o :: Order -> Float<usd> => o.amount)
end
```

---

## Install

```sh
cargo build --release
# binary at target/release/loom
```

## Usage

```sh
loom compile src/payment.loom                    # → Rust
loom compile src/payment.loom --target ts        # → TypeScript
loom compile src/payment.loom --target openapi   # → OpenAPI 3.0
loom compile src/payment.loom --check-only       # type/effect/semantic check only
```

---

## OpenAPI REST inference

Loom derives full REST semantics from type signatures — no annotations required:

```loom
fn get_order    :: Int   -> Effect<[DB], Order]        -- GET  /orders/{id}
fn create_order :: Order -> Effect<[DB], Order]        -- POST /orders  (201)
fn delete_order :: Int   -> Effect<[DB], Unit]         -- DELETE /orders/{id}
fn list_orders  :: Unit  -> Effect<[DB], List<Order>]  -- GET  /orders
```

`@idempotent` on POST promotes it to PUT. `@exactly-once` emits `x-retry-policy: never`. Error enum variants map to HTTP status codes (`NotFound → 404`, `InvalidInput → 400`).

---

## Milestone index

| # | Feature | Status |
|---|---------|--------|
| M1 | Type inference (Hindley-Milner) | ✅ |
| M2 | Pattern exhaustiveness checking | ✅ |
| M3 | WebAssembly back-end | ✅ |
| M4 | Language Server Protocol | ✅ |
| M5 | Dependency injection (`requires`/`with`) | ✅ |
| M6 | Standard library type mappings | ✅ |
| M7 | Generic functions | ✅ |
| M8 | Multi-module project compilation | ✅ |
| M9 | Inline Rust escape hatch (`{ ... }`) | ✅ |
| M10 | Numeric coercion (`as`) | ✅ |
| M11 | First-class iteration (map/filter/fold/for-in) | ✅ |
| M12 | Tuples, `Option<T>`, `Result<T,E>`, `?` operator | ✅ |
| M13 | `describe:` blocks + audit annotations | ✅ |
| M14 | `invariant:` declarations + consequence tiers | ✅ |
| M15 | `test:` blocks + `ensure:` assertions | ✅ |
| M16 | `import` + explicit `interface`/`implements` | ✅ |
| M17 | TypeScript emission target | ✅ |
| M18 | OpenAPI 3.0 + JSON Schema emission | ✅ |
| M19 | Units of measure (`Float<usd>`) | ✅ |
| M20 | Privacy labels (`@pii @gdpr @pci @hipaa`) | ✅ |
| M21 | Algebraic operation properties | ✅ |
| M22 | Typestate / lifecycle protocols | ✅ |
| M23 | Information flow labels | ✅ |

---

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/language-spec.md`](docs/language-spec.md) | Complete language reference for AI assistants |
| [`docs/lifecycle.md`](docs/lifecycle.md) | Full software lifecycle spec (design → self-heal) |
| [`docs/publish/white-paper.md`](docs/publish/white-paper.md) | Academic white paper (arXiv preprint) |
| [`docs/publish/article.md`](docs/publish/article.md) | Technical article for publication |
| [`docs/roadmap.md`](docs/roadmap.md) | Full milestone roadmap |
| [`docs/TechSpec.md`](docs/TechSpec.md) | Compiler architecture |

---

## Related

- [Generative Specification white paper](../gs/generative-specification/docs/white-paper/GenerativeSpecification_WhitePaper.md) — the methodology Loom is built on
