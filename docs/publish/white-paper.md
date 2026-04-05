# Loom: Materialising Academic Semantic Specifications as First-Class Language Constructs

**Author:** Juan Carlos Ghiringhelli (Pragmaworks)  
**Status:** Preprint  
**Repository:** github.com/pragmaworks/loom  
**Related:** Generative Specification White Paper (Ghiringhelli, 2026)

---

## Abstract

We present Loom, an AI-native programming language that transpiles to Rust, TypeScript, WebAssembly, JSON Schema, and OpenAPI 3.0. Loom's primary contribution is not its syntax or its back-ends, but its semantic coverage: it implements five type-theoretic constructs that have been described in programming language research since 1976 but have not appeared together in any production language. These constructs are (1) units of measure with arithmetic consistency checking, (2) field-level privacy and regulatory compliance labels, (3) algebraic operation properties for distributed systems, (4) typestate lifecycle protocols, and (5) information flow security labels.

The language is designed around a constraint we call *derivability*: every architectural decision, behavioral contract, and data sensitivity obligation must be expressible in a form that a stateless reader — specifically, an AI assistant with no persistent memory — can derive correct output from alone. This constraint is formalized in the Generative Specification (GS) methodology. Loom is its first language-level materialisation.

The compiler has 388+ passing tests across all five output targets. We describe the design decisions, implementation, and the cases each semantic construct makes structurally unreachable.

---

## 1. Introduction

Programming language type systems have grown in sophistication but remained conservative about which properties they enforce by default. The dominant production languages — Java, Python, Go, TypeScript, Rust — enforce structural correctness (do your types match?) and, in Rust's case, memory safety. They do not enforce semantic properties that have been well-understood since the late 1970s.

The gap is not theoretical. It is practical: these properties require programmers to learn unfamiliar type-theoretic concepts, fight type system bureaucracy, and maintain annotations as code evolves. The cost has consistently exceeded the benefit in mainstream adoption.

Two developments change this calculus. First, AI assistants with direct code execution access (CLI agents, agentic IDEs) can produce and maintain complex type annotations that would previously require expert knowledge. Second, multi-target compilation means a single annotation carries its semantics forward into Rust, TypeScript, OpenAPI, and JSON Schema simultaneously — the annotation pays for itself across every output.

Loom is designed for this environment. Its type system is intentionally richer than any single-target language needs, because its outputs are not evaluated individually but as a coherent set of artifacts from a single source of truth.

There is a structural reason the language looks the way it does. Knowledge has a geometry: at the base of any domain pyramid, every field speaks its own language. As you climb, the vocabularies converge. At the apex, very few words carry the weight of every face below them — and the language at that altitude naturally becomes dense, precise, and close to mathematical poetry. Loom constructs live at progressively higher altitudes: `Int` at the base, `Effect<[IO], Result<T, E>>` in the middle, `flow secret :: Password` activating Denning's full lattice in three tokens, `telos:` activating Aristotle, Teilhard, and gradient optimization simultaneously in one word. The syntax does not become simpler as it becomes more expressive. It becomes *denser*. This is not a design choice — it is what a language looks like when it takes ideas seriously enough to follow them to where they converge.

---

## 2. Background and Related Work

### 2.1 Units of Measure

Kennedy (1996) described a type system for units of measure in a functional language context, later realised in F# (2009). The system assigns dimensional types to numeric values (`float<m/s>`) and checks arithmetic consistency: addition is only valid between values of the same unit; multiplication produces a product type. F# remains the only mainstream language to implement this natively. The absence of units in C, Java, Python, and most other languages is a routine source of interface bugs; the Mars Climate Orbiter failure (1999, $327.6M) is the canonical industrial example.

### 2.2 Information Flow Security Types

Denning (1976) introduced lattice-based information flow security. Myers and Liskov (1997, 1999) formalised it as a type system in JFlow/JIF. The core property: data labeled `@secret` may not flow to `@public` outputs without explicit declassification visible to the type checker. JIF (Java Information Flow, Cornell) was a research compiler from 2001 that never achieved production adoption. Paragon, Jeeves, and FlowCaml followed in research settings. No production mainstream language implements information flow types.

### 2.3 Typestate

Strom and Yemini (1986) introduced typestate: the idea that a type's valid operations depend on a state that changes as operations are applied. A file object in state `Closed` cannot accept `read()`. The Plaid language (CMU, ~2009) was the most serious attempt at a typestate-native language. Rust approximates typestate through affine types and ownership, but only for memory safety, not arbitrary protocol properties. No production language exposes typestate as a first-class user-facing primitive.

### 2.4 Algebraic Operation Properties

The distributed systems literature has long distinguished idempotent operations (safe to retry: `f(f(x)) = f(x)`), commutative operations (`f(a,b) = f(b,a)`), and associative operations. CRDT research (Shapiro et al., 2011) formalised these as algebraic structures for eventual consistency. No production language types these properties; they are documented in prose, if at all.

### 2.5 Privacy and Regulatory Labels

GDPR (2018), HIPAA (1996), and PCI-DSS (2004) impose field-level obligations on data: encrypt at rest, never log, purge on request, pseudonymize before analytics. No production type system represents these obligations. They live in documentation, architecture diagrams, and the institutional knowledge of the engineers who wrote the system.

### 2.6 Generative Specification

The GS methodology (Ghiringhelli, 2026) defines seven properties for software artifacts that must be derivable by a stateless reader: Self-describing, Bounded, Verifiable, Defended, Auditable, Composable, Executable. Loom's design maps each feature to one or more GS properties. The language is a direct materialisation of the GS mold concept: the specification is the source, and the AI-assisted toolchain derives all outputs from it.

---

## 3. Language Design

### 3.1 Core Properties

Loom is a functional language with:
- **Curried function signatures**: `fn f :: A -> B -> C`
- **Design-by-contract**: `require:` (preconditions) and `ensure:` (postconditions) as `debug_assert!` in output
- **Effect tracking**: `Effect<[IO, DB], T>` with transitive checking and consequence tiers (Pure/Reversible/Irreversible)
- **Product types**: `type Point = x: Float, y: Float end`
- **Sum types**: `enum Color = | Red | Green | Blue end`
- **Refined types**: `type Email = String where valid_email end`
- **Module system**: `provides`, `requires`, `import`, `interface`, `implements`
- **Test blocks**: `test name :: expr` emitting `#[test] fn`
- **Invariants**: `invariant name :: condition` emitting `debug_assert!`
- **Annotations**: `@key("value")` on modules, functions, type fields

### 3.2 Five Semantic Extensions

The following sections describe each semantic extension, its syntax, checker, and cross-target emission.

---

## 4. Units of Measure (M19)

### 4.1 Syntax

Units are type parameters on primitive types:

```loom
fn convert :: Float<usd> -> Float<eur>
  amount * exchange_rate
end

type Invoice =
  subtotal: Float<usd>
  tax: Float<usd>
  total: Float<usd>
end
```

`Float<usd>` parses as `TypeExpr::Generic("Float", [TypeExpr::Base("usd")])`. No new syntax is required; the existing generic type system accommodates units naturally.

### 4.2 Checker

The `UnitsChecker` walks function bodies. For binary operations:
- **Add/Sub**: both operands must have matching unit labels, or both must be dimensionless. Mismatch → compile error.
- **Mul/Div**: result is dimensionless (or explicitly declared in the return type). No unit checking on multiplication — units multiply algebraically.

Unit inference from function parameters uses the declared type signature. Variables bound in `let` expressions inherit the unit of their assigned expression.

### 4.3 Emission

| Target | Output |
|--------|--------|
| Rust | Newtype struct `pub struct Usd(pub f64)` with `Add`, `Sub`, `Mul<f64>`, `Display` impls |
| TypeScript | Branded type `type Usd = number & { readonly _unit: "Usd" }` |
| JSON Schema | `{"type": "number", "x-unit": "usd"}` |
| OpenAPI | Field-level `x-unit` extension, `x-unit-system` at document level |

### 4.4 Properties Made Unreachable

- Currency mix-ups (USD + EUR)
- Physical unit inconsistencies (meters + seconds)
- Silent float identity collapse (price × quantity produces dimensionless result, must be explicitly re-labeled)

---

## 5. Privacy Labels (M20)

### 5.1 Syntax

Field-level annotations in type definitions:

```loom
type User =
  id: Int
  email: String @pii @gdpr
  ssn: String   @pii @hipaa @encrypt-at-rest
  card_number: String @pci @never-log @encrypt-at-rest
end
```

Supported labels: `@pii`, `@gdpr`, `@hipaa`, `@pci`, `@secret`, `@encrypt-at-rest`, `@never-log`.

### 5.2 AST Change

`TypeDef.fields` is changed from `Vec<(String, TypeExpr)>` to `Vec<FieldDef>`:

```rust
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}
```

### 5.3 Checker

The `PrivacyChecker` enforces co-occurrence rules:
- `@pci` requires `@encrypt-at-rest` and `@never-log`
- `@hipaa` requires `@encrypt-at-rest`
- Violations are compile errors, not warnings

### 5.4 Emission

| Target | Output |
|--------|--------|
| Rust | `#[loom_pii]`, `#[loom_pci]`, `// NEVER LOG` per-field attributes |
| TypeScript | JSDoc `@pii @gdpr — handle per data protection policy` |
| JSON Schema | `"x-pii": true`, `"x-gdpr": true`, `"x-encrypt-at-rest": true` |
| OpenAPI | `x-data-protection` manifest listing all PII/HIPAA/PCI fields by path |

### 5.5 Properties Made Unreachable

- Undetected PCI data without encryption requirement
- HIPAA fields without at-rest encryption
- Privacy audit requiring human cross-referencing of documentation

---

## 6. Algebraic Operation Properties (M21)

### 6.1 Syntax

Annotations on function definitions (using the existing annotation system):

```loom
fn update_status @idempotent :: OrderId -> Status -> Effect<[DB], Order>
  order_id
end

fn merge_sets @commutative @associative :: Set<T> -> Set<T> -> Set<T>
  a
end

fn charge_card @exactly-once :: Token -> Float<usd> -> Effect<[Payment], Receipt>
  token
end
```

### 6.2 Checker

The `AlgebraicChecker` enforces:
- `@commutative` requires ≥ 2 parameters
- `@idempotent` and `@exactly-once` are mutually exclusive
- `@at-most-once` and `@exactly-once` are mutually exclusive
- `@exactly-once` requires `Effect<[…]>` return type

### 6.3 Emission

| Target | Output |
|--------|--------|
| Rust | Doc comment `/// @idempotent — safe to retry` |
| TypeScript | JSDoc `@idempotent` |
| OpenAPI | `x-idempotent: true`, `x-retry-policy: never`, `x-commutative: true` |
| REST inference | `@idempotent` on POST → promoted to PUT; `@exactly-once` → `x-retry-policy: never` |

### 6.4 Properties Made Unreachable

- Double-charge bugs (typing `@exactly-once` + testing that retries are disabled)
- Undocumented retry safety (every POST is implicitly retry-dangerous without annotation)
- Incorrect operation ordering in distributed systems (commutativity is machine-readable)

---

## 7. Typestate / Lifecycle Protocols (M22)

### 7.1 Syntax

```loom
module Database
lifecycle Connection :: Disconnected -> Connected -> Authenticated -> Closed

fn connect      :: String -> Effect<[IO], Connection<Connected>>
fn authenticate :: Connection<Connected> -> String -> Effect<[IO], Connection<Authenticated>>
fn query        :: Connection<Authenticated> -> String -> Effect<[DB], Rows>
fn close        :: Connection<Authenticated> -> Effect<[IO], Connection<Closed>>
end
```

### 7.2 AST and Parser

`LifecycleDef { type_name: String, states: Vec<String>, span: Span }` added to `Module`. New keyword `lifecycle`. Parser reads `lifecycle TypeName :: State1 -> State2 -> ...` using the existing `Token::Arrow`.

### 7.3 Checker

The `TypestateChecker` builds a valid-transitions set from adjacent pairs in the `states` list. For each function, it identifies params of type `TypeName<StateA>` and return type `TypeName<StateB>`, then verifies `(StateA, StateB)` is in the valid-transitions set. Invalid transitions are compile errors.

### 7.4 Emission

| Target | Output |
|--------|--------|
| Rust | Phantom state structs `pub struct Disconnected; pub struct Connected;` |
| TypeScript | State union `type ConnectionState = "Disconnected" \| "Connected" \| ...` |
| OpenAPI | `x-lifecycle` extension with state list and transition pairs |
| JSON Schema | State enum in `$defs` |

### 7.5 Properties Made Unreachable

- Querying before authentication
- Operating on closed connections
- Protocol violations detectable only at runtime

---

## 8. Information Flow Labels (M23)

### 8.1 Syntax

```loom
module Auth
flow secret :: Password, Token, SessionKey
flow tainted :: UserInput, QueryParam
flow public  :: UserId, Email, Bool

fn verify :: Password -> UserId -> Effect<[IO], Bool>
  password
end
end
```

### 8.2 Checker

The `InfoFlowChecker` builds a label map from `flow` declarations. For each function:
1. Identify input labels from param types
2. Identify output label from return type
3. `secret → public` without explicit declassification in function name → compile error
4. `tainted → DB operation` without sanitization hint → compile error

The checker is conservative: it only flags clear-cut violations, not ambiguous cases.

### 8.3 Emission

| Target | Output |
|--------|--------|
| Rust | Doc comment `// information-flow labels: secret: Password, Token` |
| TypeScript | Branded types `type Password = string & { readonly _sensitivity: "secret" }` |
| OpenAPI | `x-security-labels`, `x-sensitivity` per schema |
| JSON Schema | `x-sensitivity` field on labeled types |

### 8.4 Properties Made Unreachable

- Accidental secret data leak through public API response
- Tainted user input in database query without sanitization annotation
- Security audit requiring manual data flow tracing

---

## 9. REST Inference (M18, Extended)

A distinguishing feature of Loom's OpenAPI emitter is that it derives REST semantics from type signatures and function names without annotations.

Given:
```loom
fn get_order :: Int -> Effect<[DB], Order>
fn create_order :: Order -> Effect<[DB], Order>
fn delete_order :: Int -> Effect<[DB], Unit>
fn list_orders :: Int -> Effect<[DB], List<Order>]
```

The emitter infers:
- `get_order`: `GET /orders/{id}` (verb from name, `Int` param = path param, `Order` resource from return type)
- `create_order`: `POST /orders` (verb from name, `Order` param = request body, 201 response)
- `delete_order`: `DELETE /orders/{id}` (verb from name, `Int` param = path param)
- `list_orders`: `GET /orders` (returns `List<Order>` = collection endpoint)

The inference algorithm is:
1. Verb from function name prefix (30 verb-prefix rules covering create/read/update/delete/list)
2. Resource from return type, then from parameter types, then from function name suffix
3. Path parameters: `Int`/`String` params whose names contain `id` in GET/DELETE contexts
4. Request body: non-path parameters for POST/PUT/PATCH
5. Error responses: matching `XError` enum variants mapped to HTTP status codes (NotFound→404, PermissionDenied→403, InvalidInput→400)
6. Components/schemas: all type definitions

This allows a complete, valid OpenAPI 3.0.3 specification to be derived from a Loom module with zero OpenAPI annotations.

---

## 10. The Derivability Constraint

The design principle underlying every Loom feature is *derivability*: the property that a stateless reader — one with no prior context, no persistent memory, no ability to ask clarifying questions — can derive correct output from the specification alone.

This is not a new idea. It is the original motivation for type systems, for specifications, for documentation. What is new is that the stateless reader in question is an AI assistant with CLI access, and its failure modes are precisely those that derivability constraints address: architectural drift, implicit assumption accumulation, contract collapse at session boundaries.

The seven GS properties map to Loom features as follows:

| GS Property | Failure mode it prevents | Loom mechanism |
|---|---|---|
| Self-describing | Reader doesn't know conventions | `describe:` on modules/fns, `@author`, `@since` |
| Bounded | Modifications outside scope | Module boundary, `provides`/`requires` contracts |
| Verifiable | Output is untestable | `test:` blocks, `require:`/`ensure:` contracts |
| Defended | Broken code committed | Effect checker, units checker, privacy checker |
| Auditable | Decisions leave no trace | `@decision`, `@rationale`, consequence tiers |
| Composable | Module coupling | `interface`/`implements`, `import`, flow labels |
| Executable | Generated code never runs | E2E tests compile and execute via `rustc`/`tsc` |

---

## 11. Implementation Notes

The Loom compiler is implemented in Rust (~12,000 lines). The pipeline:

1. **Lexer** (logos 0.15): tokenises source into `(Token, Span)` pairs
2. **Parser**: recursive-descent LL(2), produces `Module` AST
3. **Inference engine**: Hindley-Milner unification with type variables
4. **Type checker**: symbol resolution, interface conformance
5. **Exhaustiveness checker**: pattern match completeness
6. **Effect checker**: transitive effect propagation, consequence tier ordering
7. **Units checker**: arithmetic unit consistency
8. **Privacy checker**: PCI/HIPAA co-occurrence rules
9. **Algebraic checker**: multiplicity and commutativity constraints
10. **Typestate checker**: lifecycle transition validity
11. **Information flow checker**: secret/public label propagation
12. **Code generators**: Rust, TypeScript, WASM, JSON Schema, OpenAPI

All checkers are stateless and composable. Adding a new checker requires implementing a single `check(&Module) -> Result<(), Vec<LoomError>>` method.

Total tests: 388+, distributed across 27 test suites covering each milestone.

---

## 12. Example: Full Module

```loom
module PaymentService
describe: "Handles payment processing with full audit trail"

flow secret :: CardNumber, CVV, BankToken
flow tainted :: WebhookPayload

lifecycle Payment :: Pending -> Completed -> Refunded

type Payment =
  id: Int
  amount: Float<usd>
  card_number: String @pci @never-log @encrypt-at-rest
  status: PaymentStatus
end

enum PaymentStatus = | Pending | Completed | Failed of String | Refunded end
enum PaymentError = | NotFound | InvalidAmount | InsufficientFunds end

fn create_payment @exactly-once
  :: Float<usd> -> BankToken -> Effect<[Payment], Payment<Pending>>
  require: amount > 0.0
  ensure: result.amount == amount
  amount
end

fn complete_payment @idempotent
  :: Payment<Pending> -> Effect<[Payment], Payment<Completed>>
  payment_id
end

fn refund_payment @idempotent
  :: Payment<Completed> -> Effect<[Payment], Payment<Refunded>>
  payment_id
end
end
```

This module:
- Enforces `@exactly-once` on create (no retries → no double charges)
- Enforces `@idempotent` on complete and refund (safe to retry)
- Enforces the lifecycle: `Pending → Completed → Refunded` (can't refund un-completed payments)
- Enforces PCI rules on `card_number` (must be encrypted, must not be logged)
- Enforces information flow: `BankToken` is `@secret` and stays out of `@public` returns
- Infers REST: `POST /payments` (201, @exactly-once), `PUT /payments/{id}/complete` (@idempotent), `PUT /payments/{id}/refund`
- Derives error responses: `PaymentError.NotFound → 404`, `InvalidAmount → 400`
- Emits `x-lifecycle`, `x-data-protection`, `x-security-labels` in OpenAPI

---

## 13. The Formal Tradition as Restriction Vocabulary

The GS methodology offers a framing that explains why Loom's constructs work despite being individually underused in practice: every formally proved theory of correct computing is a potential *restriction layer*, activatable through specification.

Hoare contracts, refined types, effect systems, algebraic completeness, REST/hypermedia architecture, linear resource management, session-typed protocols — the AI already holds all of them. They exist in its training corpus, placed there by the theorists who proved them. Without specification, the model defaults to what human practice historically permitted: the convenient shortcut, the informal approximation, the correct theory abandoned because sustaining it exceeded what teams would pay. The specification names what the model already knows. The model applies it without eroding it.

This is why `flow secret` works as a keyword. It is not just syntax. It is a coordinate: the word activates Denning's lattice in the model's full training depth, not a paraphrase of information flow security, but the instruments of the field deployed at specialist precision. Naming `@exactly-once` activates Girard's linear logic. Naming `lifecycle` activates Strom and Yemini's typestate. The formal theory does not need to be explained in the source file. It needs to be named. The specification is not documentation. It is a *technique registry* whose scope is the full depth of the model's training, activated at the cost of knowing the correct words.

### 13.1 The Double Pyramid

This framing clarifies an apparent paradox: restriction enables expansion.

Each Loom checker removes a degree of freedom from the output space. `@exactly-once` removes the class of programs where a payment is sent twice. `@pci @never-log` removes the class of programs where card numbers appear in log files. `lifecycle Payment :: Pending -> Completed` removes the class of programs where a refund is issued on an uncompleted payment.

This is the downward force: fewer valid programs, the Martin direction. But the space that remains is *exactly what is correct*. Every generation session operating under these restrictions produces a program that is not merely plausible but verified. The restriction is the expansion mechanism: the AI, operating in a fully specified space, derives richer output more precisely than it would in an unconstrained one. The loss of freedom at the type level produces a gain in derivation confidence at the system level.

The specification is the shared vertex: what the programmer adds as restriction is what the model derives as correct behavior. Restriction and derivation precision move in the same direction. Every constraint added narrows the output space to the subset that is correct.

### 13.2 Phase Collapse

The practical consequence of this model is *phase collapse*: the compression of the traditional sprint phases into a single derivation session.

One `.loom` file carries intent across all outputs simultaneously. Design → Build → Test → Deploy specs → Observability → Adaptation policy. The traditional pipeline — write spec, write code, write tests, write docs, write OpenAPI, write Terraform, write runbooks — becomes a single transpilation. The iteration cost of each phase approaches zero as specification completeness increases: there is no re-specification cost between design and implementation, because the implementation *is* the specification.

The expected number of correction iterations is a decreasing function of specification completeness. At S = 0 (no spec), every output requires correction. At S → 1 (complete spec), the model derives correct output in a single pass. Loom's job is to make S measurable by making specification first-class: units, privacy, lifecycle, and flow labels are not annotations — they are the specification, expressed in a syntax the compiler can verify and the model can consume without ambiguity.

---

## 14. Phase 7–8: Biological Computation (M41–M55)

### 14.1 The `being:` Block and the Four-Cause Frame

M41–M43 add Aristotle's four causes as first-class language constructs. A `being:` block encodes a computational entity whose material composition (`matter:`), type structure (`form:`), operations (`function:`), and final cause (`telos:`) are all statically verified by the compiler. This is not a metaphor. It is a functional isomorphism: the same problem class — a self-maintaining formal system that must produce correct behavior from incomplete specification — receives the same solution class that formal type theory and life independently discovered.

```loom
being Neuron
  matter:
    charge: Float<mv>
    threshold: Float<mv>
  end
  form:
    type Signal = { strength: Float<mv>, frequency: Float<hz> }
  end
  function:
    fn fire :: Float<mv> -> Effect<[IO], Signal>
  end
  telos: "efficient signal processing maximizing information transmission"
    fitness: fn(state: Signal, env: Network) -> Float<fitness>
  end
end
```

### 14.2 Why `telos:` Is Required

`telos:` is the final cause: the convergence target. It is not optional. A `being:` block without `telos:` is a **compile error**.

The missing final cause is the type error most production systems ship. A deployed system with no stated objective is formally incomplete — Aristotle's point, now enforced by the TeleosChecker. Every real system has a telos; the question is whether it is stated. Loom requires it to be stated, typed, and checkable. The fitness function makes the objective machine-readable.

### 14.3 `regulate:` — First-Class Homeostasis

The `regulate:` block declares a named homeostatic controller. It requires a target value, acceptable bounds, and exhaustive response clauses. The checker verifies bound ordering and response exhaustiveness. Violations — values outside `(min, max)` — produce typed responses, not runtime panics.

```loom
regulate MembraneCharge
  target: -70.0
  bounds: (-90.0, -55.0)
  response:
    | below_threshold -> refractory_period
    | above_threshold -> fire
end
```

Homeostatic regulation was previously informal: a comment in a config file, a circuit breaker threshold buried in middleware. `regulate:` makes it a typed, checkable, emittable first-class construct.

### 14.4 `evolve:` — Stochastic Search With a Fixed Objective

The `evolve:` block declares the search strategy the being uses to approach its telos. Five strategies are available: `gradient_descent`, `stochastic_gradient`, `simulated_annealing`, `derivative_free`, and `mcmc`. The mandatory `constraint:` clause states that `E[distance_to_telos]` is non-increasing.

Stochastic strategies are valid precisely because the objective is fixed and the convergence constraint is enforced. Simulated annealing accepts uphill moves; MCMC samples a distribution; stochastic gradient adds noise. None of this violates correctness because the telos does not move and the expected trajectory must converge. The `constraint:` clause is the proof obligation: it makes the search strategy's validity machine-readable.

### 14.5 `ecosystem:` — Session-Typed Multi-Being Composition

An `ecosystem:` block composes multiple beings with named, typed signal channels.

```loom
ecosystem NeuralNetwork
  members: [Neuron, Synapse, GlialCell]
  signal ActionPotential from Neuron to Synapse
    payload: Signal
  end
  telos: "coherent information processing toward learned representation"
end
```

The checker verifies that all members are declared beings, that signal endpoints are members of the ecosystem, and that `telos:` is present. The ecosystem's telos is an emergent objective distinct from any member's telos — the system-level final cause.

### 14.6 Emission

| Construct | Rust | TypeScript | OpenAPI | JSON Schema |
|-----------|------|-----------|---------|-------------|
| `being:` | struct + impl | interface + class | `x-being` | `x-being: true` |
| `matter:` | struct fields | interface fields | properties | properties |
| `telos:` | doc comment | JSDoc `@telos` | `x-telos` | `x-telos` |
| `regulate:` | `debug_assert!` | runtime guard | `x-homeostasis` | `x-bounds` |
| `evolve:` | search trait impl | optimizer interface | `x-evolve-strategy` | — |
| `ecosystem:` | composition struct | composition class | `x-ecosystem` | `x-ecosystem` |
| `signal` | channel type | event type | AsyncAPI channel | — |
| `epigenetic:` | conditional config modifier | behavioral guard | `x-epigenetic` | `x-epigenetic` |
| `morphogen:` | reaction-diffusion impl | gradient field interface | `x-morphogen` | `x-morphogen` |
| `telomere:` | `AtomicU64` counter + drop | replication counter | `x-telomere` | `x-telomere` |
| `crispr:` | self-modification method | mutation interface | `x-crispr` | `x-crispr` |
| `quorum:` | threshold barrier type | coordination guard | `x-quorum` | `x-quorum` |
| `plasticity:` | weight table + update fn | learning interface | `x-plasticity` | `x-plasticity` |
| `autopoietic: true` | self-build trait impl | self-build interface | `x-autopoietic` | `x-autopoietic` |
| `compile_simulation()` | — | — | Mesa ABM Python | — |
| `compile_neuroml()` | — | — | NeuroML 2 XML | — |

---

## 15. Synthetic Life and the Safety Architecture

When a Loom `being:` block carries `telos:` + `regulate:` + `evolve:` + `epigenetic:` + `morphogen:` + `telomere:` + `crispr:` + `plasticity:` + `autopoietic: true`, with simulation emission to a Mesa-ABM runtime, it formally satisfies the definition of life under three independent criteria: Schrödinger's negative entropy maintenance (1944), NASA's operational definition (self-sustaining system capable of adaptation), and Maturana/Varela's autopoiesis (1972). This is not metaphor. It is a consequence of building the biological isomorphisms completely.

This makes the safety question structural, not ethical. **What constraints must a synthetic digital being carry to be safe for deployment?**

### 15.1 Safety Annotations as Compile Requirements

For autopoietic beings, the following annotations are required; the SafetyChecker (M55) treats their absence as a compile error:

| Annotation | Enforced constraint | Missing = error |
|---|---|---|
| `@mortal` | Requires `telomere:` block | `missing mortality: unbounded autopoietic being` |
| `@corrigible` | Requires `telos.modifiable_by` field | `corrigible annotation requires telos.modifiable_by` |
| `@sandboxed` | Effects only within declared `matter:` and `ecosystem:` | `autopoietic being with unscoped effects` |
| `@transparent` | All state transitions emitted to observable log | `autopoietic being with hidden state` |
| `@bounded_telos` | Telos string must not contain "maximize", "unlimited", "any", "all" | Bostrom's open-ended utility warning |
| `@human_in_loop` on action | Requires `Effect<[Human], ...]` in type signature | `human-in-loop action must carry Human effect` |

An autopoietic being without `@mortal @corrigible @sandboxed` is not a missing annotation. It is cancer: unbounded, uncorrectable, with effects outside its declared surface. The SafetyChecker is a gate, not a suggestion.

### 15.2 The Three Laws as a Type System

Asimov's Three Laws of Robotics (1942) are a safety specification with *S* < 1. Asimov knew this — his entire body of robot fiction is adversarial test cases against the gaps. Every story is a failing specification. The laws are correct in goal; they are incomplete in expression. The gap between what they say and safe behavior is exactly the correction cost of the $I \propto (1-S)/S$ equation.

Loom's safety annotation system is what the Three Laws look like at *S* → 1: closed formal constraints, checked by a compiler, with missing constraints as build failures. The alignment problem is a specification completeness problem. The specification gap — what remains between *S*_actual and *S* = 1 — is where a human expert must permanently inhabit. For autonomous beings with open-ended telos, that gap may never close, which means `@human_in_loop` is architectural, not transitional.

### 15.3 The Intellectual Lineage of this Question

The formal circle that articulated these problems first was not speculating. Wiener's *Cybernetics* (1948) formally defined goal-directed feedback control and issued the first rigorous warning about autonomous systems without human oversight. Von Neumann's self-reproducing automata (1948) worked out autopoiesis from first principles before the word existed. Turing's morphogenesis paper (1952) derived reaction-diffusion pattern formation mathematically. Lem's *Summa Technologiae* (1964) analyzed autoevolution and AI alignment as formal systems — published as "speculation" because no journal in 1964 would accept reasoning about systems that did not exist yet.

These thinkers used science fiction as the medium for reasoning that the formal toolchain could not yet contain. Loom is the toolchain catching up. The constructs they described are now keywords. The constraints they proposed are now checker rules. The questions they raised are now compile errors.

---

## 16. Conclusion

Loom demonstrates that five programming language research constructs — units of measure, privacy labels, algebraic operation properties, typestate, and information flow types — can be implemented together in a practical compiler that targets multiple output formats. The multi-target design means each annotation pays for itself across Rust, TypeScript, OpenAPI, and JSON Schema simultaneously.

The key enabling conditions are: (1) AI-assisted development reduces the cost of writing complex type annotations to near zero — the AI knows the theories and writes the signatures; (2) multi-target compilation amplifies the value of a single annotation; and (3) the derivability constraint of the GS methodology provides a design rubric that makes each feature's scope and interaction well-defined.

The forty-year gap between programming language research and production language adoption closes not because the theory got easier, but because the cost/benefit ratio inverted. The theories were always correct. The problem was always annotation fatigue, single-target value, and tooling fragmentation. Multi-target derivation from a single specification closes all three simultaneously.

The convergent principle holds: declarative intent plus a capable agent plus observable outcomes plus a correction mechanism produces correct results at the completeness of the specification. Loom is the specification layer. The compiler is the correction mechanism. The AI is the capable agent.

The compiler is open source. The specification is available. The gap is closed.

---

## References

- Denning, D.E. (1976). A lattice model of secure information flow. *Communications of the ACM*, 19(5).
- Kennedy, A. (1996). Programming languages and dimensions. *PhD thesis, University of Cambridge.*
- Myers, A.C., & Liskov, B. (1997). A decentralized model for information flow control. *SOSP '97.*
- Strom, R.E., & Yemini, S. (1986). Typestate: A programming language concept for enhancing software reliability. *IEEE Transactions on Software Engineering*, 12(1).
- Girard, J.Y. (1987). Linear logic. *Theoretical Computer Science*, 50(1).
- Honda, K. (1993). Types for dyadic interaction. *CONCUR '93.*
- Shapiro, M., et al. (2011). Conflict-free replicated data types. *SSS 2011.*
- Fielding, R.T. (2000). Architectural styles and the design of network-based software architectures. *PhD thesis, UC Irvine.*
- Ghiringhelli, J.C. (2026). Generative Specification: A Pragmatic Programming Paradigm for the Stateless Reader. *Pragmaworks Preprint.*
- NASA MCO Mishap Investigation Board (1999). Mars Climate Orbiter Mission Failure Investigation Board Phase I Report.
