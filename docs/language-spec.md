# Loom Language Specification

> **Canonical reference for AI assistants and human authors.**  
> Every valid Loom construct is documented here with exact syntax and examples.  
> Version: 2026-04 (post-M23)

---

## 1. Module Structure

Every Loom source file is a single module.

```
module <Name>
[describe: "<string>"]
[@annotation...]

[flow <label> :: <TypeName>, ...]...
[lifecycle <TypeName> :: <State> -> <State>...]...

[<item>]...

end
```

Module name is PascalCase. The `end` keyword closes the module.

---

## 2. Items

Items appear inside a module in any order:

| Keyword | Construct |
|---------|-----------|
| `type` | Product type definition |
| `enum` | Sum type definition |
| `type ... where` | Refined type definition |
| `fn` | Function definition |
| `interface` | Interface declaration |
| `import` | Module import |
| `implements` | Interface implementation declaration |
| `invariant` | Module-level invariant |
| `test` | Inline test block |
| `lifecycle` | Typestate protocol |
| `flow` | Information flow label |

---

## 3. Types

### 3.1 Product types (structs)

```loom
type Point = x: Float, y: Float end

type Invoice =
  id:       Int
  amount:   Float<usd>
  email:    String @pii @gdpr
  card_num: String @pci @never-log @encrypt-at-rest
end
```

Fields are `name: Type` separated by commas or newlines. Field annotations follow the type.

### 3.2 Sum types (enums)

```loom
enum Color = | Red | Green | Blue end

enum Shape =
  | Circle of Float
  | Rect   of Float * Float
  | Point
end

enum Result<T, E> =
  | Ok  of T
  | Err of E
end
```

Variants optionally carry a payload with `of Type`. Tuple payloads use `*`.

### 3.3 Refined types

```loom
type Email      = String  where valid_email end
type Percentage = Float   where 0.0 <= value && value <= 100.0 end
type NonEmpty   = String  where value != "" end
```

The `where` clause is a boolean predicate on the implicit variable `value`.

### 3.4 Generic types

```loom
type Pair<A, B> = first: A, second: B end
type Box<T>     = value: T end
```

Type parameters are single uppercase letters or PascalCase names.

### 3.5 Type expressions

| Syntax | Meaning |
|--------|---------|
| `Int` | 64-bit signed integer |
| `Float` | 64-bit float |
| `Float<usd>` | Unit-parameterised float |
| `String` | UTF-8 string |
| `Bool` | Boolean |
| `Unit` | No value (void) |
| `List<T>` | Homogeneous list |
| `Option<T>` | Present or absent |
| `Result<T, E>` | Success or error |
| `(A, B)` | Tuple |
| `(A, B, C)` | 3-tuple |
| `A -> B` | Function type |
| `Effect<[E1, E2], T>` | Effectful computation |
| `Effect<[IO@irreversible], T>` | Effect with consequence tier |
| `T<Param>` | Generic application |
| `TypeName<State>` | Typestate-parameterised type |

---

## 4. Functions

```
fn <name> [@annotation...] :: <type-signature>
  [describe: "<string>"]
  [require: <expr>]
  [ensure:  <expr>]
  <body-expr>
end
```

### 4.1 Type signatures

```loom
fn add       :: Int -> Int -> Int
fn fetch     :: Int -> Effect<[IO, DB], User]
fn map<A, B> :: (A -> B) -> List<A> -> List<B>
```

Signatures are fully curried. `->` associates right. The final type is the return type.

### 4.2 Annotations on functions

Annotations come **after** the function name, **before** `::`:

```loom
fn charge @exactly-once @trace("payment.charge") :: Token -> Float<usd> -> Effect<[Payment], Receipt]
```

### 4.3 Contracts

```loom
fn transfer :: Float<usd> -> Account -> Effect<[DB], Account]
  require: amount > 0.0
  require: balance >= amount
  ensure:  result.balance == balance - amount
  amount
end
```

Multiple `require:` and `ensure:` lines are allowed. They emit as `debug_assert!`.

### 4.4 Effects

```loom
Effect<[IO], T>                  -- single effect
Effect<[IO, DB, Cache], T]       -- multiple effects (union)
Effect<[IO@irreversible], T]     -- effect with consequence tier
```

Consequence tiers (ordered, most to least severe):
- `@irreversible` — cannot be undone (send email, charge card)
- `@reversible` — can be rolled back (DB write in transaction)
- (no tier) — pure or observable only

### 4.5 Body expressions

```loom
-- Literal
42
"hello"
true

-- Variable
x

-- Let binding
let x = expr in body

-- Function application (juxtaposition)
f x y z

-- Binary operators
x + y    x - y    x * y    x / y    x % y
x == y   x != y   x < y    x <= y   x > y   x >= y
x && y   x || y   not x

-- Coercion
x as Float

-- Pipe
x |> f |> g |> h

-- Match
match expr with
  | Pattern1 => body1
  | Pattern2 => body2
end

-- Lambda
fn x :: Int -> Int => x + 1

-- If
if cond then a else b

-- ? operator (error propagation)
fetch_user(id)?

-- Inline Rust escape hatch
{ raw_rust_code_here }
```

---

## 5. Pattern Matching

```loom
match shape with
  | Circle r         => 3.14159 * r * r
  | Rect w h         => w * h
  | Point            => 0.0
end

match opt with
  | Some x => x
  | None   => default
end

match result with
  | Ok  v => v
  | Err e => handle_error e
end
```

Patterns must be exhaustive — the compiler reports missing cases.

---

## 6. Higher-Order Functions and Iteration

```loom
-- Map
items |> map(fn x :: Item -> Float => x.price)

-- Filter
items |> filter(fn x :: Item -> Bool => x.active)

-- Fold
items |> fold(0.0, fn acc x :: Float -> Item -> Float => acc + x.price)

-- For-in (imperative style)
for item in items
  process item
end
```

---

## 7. Module System

### 7.1 Provides / Requires (dependency injection)

```loom
module InvoiceService
provides: generate_invoice
requires: PaymentRepository, UserRepository
```

### 7.2 Import

```loom
import PaymentRepository
import UserRepository as Repo
```

### 7.3 Interface

```loom
interface Repository<T>
  fn find :: Int -> Effect<[DB], Option<T>]
  fn save :: T   -> Effect<[DB], T]
  fn delete :: Int -> Effect<[DB], Unit]
end
```

### 7.4 Implements

```loom
module PostgresUserRepo
implements Repository<User>

fn find :: Int -> Effect<[DB], Option<User>]
  user_id
end

fn save :: User -> Effect<[DB], User]
  user
end

fn delete :: Int -> Effect<[DB], Unit]
  user_id
end
end
```

---

## 8. GS Constructs (Self-describing, Auditable, Verifiable)

### 8.1 Describe blocks

```loom
module PricingEngine
describe: "Computes final prices including regional tax and discounts"

fn calculate_total :: Order -> Effect<[DB], Float<usd>]
  describe: "Applies line items, coupon codes, and VAT"
  order
end
```

### 8.2 Annotations

```loom
module PaymentService
@author("billing-team")
@version(2)
@since("2025-01")
@decision("Use exclusive tax to match EU VAT rules")
@rationale("Matches accounting system expectation per ADR-042")
```

### 8.3 Invariants

```loom
invariant non_negative_balance :: balance >= 0.0
invariant valid_percentage :: rate >= 0.0 && rate <= 1.0
```

Emit as `debug_assert!` in Rust output. Checked in debug builds.

### 8.4 Test blocks

```loom
test add_is_commutative ::
  add(2, 3) == add(3, 2)
end

test transfer_reduces_balance ::
  let initial = 100.0 in
  let after   = transfer(10.0, account) in
  after.balance == initial - 10.0
end
```

Emit as `#[test] fn` in Rust output.

---

## 9. Semantic Type Constructs

### 9.1 Units of Measure

```loom
fn convert :: Float<usd> -> Float<eur>
  amount * exchange_rate
end

type Invoice =
  subtotal: Float<usd>
  tax:      Float<usd>
  total:    Float<usd>
end
```

The unit label is a type parameter on `Float`. Adding/subtracting mixed units is a compile error. Multiplication/division produces a dimensionless result.

**Rust output:** newtype struct `pub struct Usd(pub f64)` with `Add`, `Sub`, `Mul<f64>` impls.  
**TypeScript output:** branded type `type Usd = number & { readonly _unit: "Usd" }`.  
**JSON Schema:** `{ "type": "number", "x-unit": "usd" }`.

### 9.2 Privacy Labels

Field-level annotations:

| Annotation | Meaning | Enforced co-requirements |
|-----------|---------|--------------------------|
| `@pii` | Personally identifiable information | — |
| `@gdpr` | Subject to GDPR | — |
| `@hipaa` | Protected health information | `@encrypt-at-rest` |
| `@pci` | Payment card data | `@encrypt-at-rest` + `@never-log` |
| `@secret` | Sensitive secret | — |
| `@encrypt-at-rest` | Must be encrypted at rest | — |
| `@never-log` | Must never appear in logs | — |

```loom
type User =
  id:     Int
  email:  String @pii @gdpr
  ssn:    String @pii @hipaa @encrypt-at-rest
  card:   String @pci @never-log @encrypt-at-rest
end
```

### 9.3 Algebraic Operation Properties

Annotations on functions:

| Annotation | Meaning | Constraint |
|-----------|---------|-----------|
| `@idempotent` | `f(f(x)) = f(x)` | POST promoted to PUT in OpenAPI |
| `@commutative` | `f(a,b) = f(b,a)` | Requires ≥ 2 params |
| `@associative` | `f(f(a,b),c) = f(a,f(b,c))` | — |
| `@exactly-once` | Must execute exactly once | Requires Effect return; mutually exclusive with `@idempotent` |
| `@at-most-once` | Must not be retried | Mutually exclusive with `@exactly-once` |
| `@monotonic` | Result only increases | — |

```loom
fn update_status @idempotent   :: OrderId -> Status -> Effect<[DB], Order]
fn charge_card   @exactly-once :: Token -> Float<usd> -> Effect<[Payment], Receipt]
fn merge_sets    @commutative @associative :: Set<T> -> Set<T> -> Set<T>
```

### 9.4 Typestate / Lifecycle Protocols

```loom
lifecycle Connection :: Disconnected -> Connected -> Authenticated -> Closed

fn connect      :: String -> Effect<[IO], Connection<Connected>]
fn authenticate :: Connection<Connected> -> String -> Effect<[IO], Connection<Authenticated>]
fn query        :: Connection<Authenticated> -> String -> Effect<[DB], Rows]
fn close        :: Connection<Authenticated> -> Effect<[IO], Connection<Closed>]
```

The checker validates that every function's parameter state → return state is an adjacent pair in the declared sequence. Invalid transitions are compile errors.

**Rust output:** phantom state structs `pub struct Connected; pub struct Authenticated;`  
**TypeScript output:** state union type `type ConnectionState = "Disconnected" | "Connected" | ...`  
**OpenAPI output:** `x-lifecycle` extension in the info section.

### 9.5 Information Flow Labels

```loom
module Auth
flow secret  :: Password, Token, SessionKey
flow tainted :: UserInput, QueryParam
flow public  :: UserId, Email, Bool
```

`flow <label> :: TypeA, TypeB` declares that values of those types carry the given sensitivity label. The checker blocks:
- `secret` → `public` without a declassification function (name containing `declassify`, `sanitize`, `hash`, `anonymize`)
- `tainted` → DB operation without sanitization hint

**TypeScript output:** branded types `type Password = string & { readonly _sensitivity: "secret" }`  
**OpenAPI output:** `x-security-labels` extension.

#### Biological Grounding: PTM Enzyme Classes

The three operations of the information flow checker — **read**, **write**, **declassify** — have a precise molecular-level instantiation in Post-Translational Modification (PTM) biology:

| Loom operation | PTM enzyme class | Biological function |
|---|---|---|
| `write` | **Writers** (kinases, acetyltransferases, methyltransferases) | Add a modification to a protein; encode information into structure |
| `declassify` | **Erasers** (phosphatases, deacetylases, demethylases) | Remove a modification; release a constraint; permit information flow |
| `read` | **Readers** (SH2 domains, bromodomains, chromodomains) | Recognize a modification; interpret the signal; act on it |

The cell's PTM regulatory layer is a distributed, dynamically reconfigurable information flow system. A phosphorylation mark (write) activates a downstream reader (read); a phosphatase (declassify) removes it, resetting the permission. The capacity of this signaling layer can be measured in Shannon bits — how many distinguishable states the combinatorial PTM landscape can represent.

This is convergent evidence from molecular biology that read/write/declassify are the right primitives, not just a type-theory choice. Denning (1976) derived the information flow lattice from first principles of secure information systems. PTM biology independently arrived at the same three-operation structure as the cell's native information security model. The isomorphism holds at the level of function, not just metaphor: both systems solve the same problem — controlling what information reaches what interpreter under what conditions.

Note: this is the **Shannon layer** of Loom's biological information model (capacity, directional constraints, labels). The `telos:` construct is the **biosemiotic layer** (sign-mediated interpretation toward a final state). See the lineage document for the full Peirce/Uexküll/Barbieri/Shannon distinction.

---

## 10. Annotations Reference

### Module-level annotations
```loom
@author("name")
@version(N)
@since("date")
@deprecated("reason")
@decision("text")
@rationale("text")
@tag("value")
@service(port=N, protocol="http")
@environment(prod, staging, dev)
@resource(cpu="500m", memory="256Mi", replicas=3)
@depends-on(ServiceA, ServiceB)
@slo(p99=200ms, availability=0.9999)
@alert(condition -> action)
@ontology("uri")
@prefix("prefix" = "uri")
```

### Function-level annotations
```loom
@deprecated("reason")
@since("version")
@trace("span.name")
@method("GET"|"POST"|"PUT"|"DELETE"|"PATCH")
@path("/custom/path")
@resource("resource-name")
@idempotent
@exactly-once
@at-most-once
@commutative
@associative
@monotonic
```

### Field-level annotations
```loom
@pii
@gdpr
@hipaa
@pci
@secret
@encrypt-at-rest
@never-log
@owl-datatype("uri")
@owl-object-property("uri")
@rdf-id
```

---

## 11. OpenAPI REST Inference

The OpenAPI emitter derives REST semantics without annotations:

| Function name prefix | HTTP verb | Notes |
|---------------------|-----------|-------|
| `create`, `add`, `register`, `insert`, `save`, `post` | POST | → 201 response |
| `update`, `set`, `put`, `replace`, `upsert` | PUT | |
| `patch`, `modify`, `change` | PATCH | |
| `delete`, `remove`, `destroy`, `drop` | DELETE | |
| `get`, `fetch`, `find`, `load`, `read`, `show`, `by` | GET | |
| `list`, `all`, `search`, `query`, `index`, `browse` | GET | collection endpoint |

Resource name inferred from: return type → param types → fn name suffix → module name (stripping `Service`/`Controller`/`Handler`).

`Int`/`String` params with id-ish names in GET/DELETE context → path parameters (`{id}`).

`@idempotent` on a POST function → promoted to PUT.

`XError` enum variant names → HTTP status codes (`NotFound → 404`, `InvalidInput → 400`, `PermissionDenied → 403`).

---

## 12. Complete Example

```loom
module OrderService
describe: "Manages order lifecycle from creation to fulfillment"
@author("commerce-team")
@version(3)
@slo(p99=150ms, availability=0.9999)

flow secret  :: PaymentToken, CardNumber
flow tainted :: CustomerInput

lifecycle Order :: Draft -> Confirmed -> Fulfilled -> Cancelled

type Order =
  id:      Int
  amount:  Float<usd>
  status:  OrderStatus
  card:    String @pci @never-log @encrypt-at-rest
end

enum OrderStatus = | Draft | Confirmed | Fulfilled | Cancelled end
enum OrderError  = | NotFound | InvalidAmount | PaymentFailed end

invariant positive_amount :: amount > 0.0

fn create_order @exactly-once
  :: Float<usd> -> PaymentToken -> Effect<[DB, Payment], Order<Draft>]
  require: amount > 0.0
  ensure:  result.status == Draft
  amount
end

fn confirm_order @idempotent
  :: Order<Draft> -> Effect<[DB, Payment], Order<Confirmed>]
  order
end

fn fulfill_order @idempotent
  :: Order<Confirmed> -> Effect<[DB, Warehouse], Order<Fulfilled>]
  order
end

test create_requires_positive_amount ::
  create_order(-1.0 as Float<usd>) fails
end

test confirm_transitions_state ::
  let draft     = create_order(50.0, token) in
  let confirmed = confirm_order(draft) in
  confirmed.status == Confirmed
end
end
```

This single module emits:
- **Rust:** struct, enums, phantom state types for lifecycle, `#[loom_pci]` attribute, `debug_assert!` for contracts, `#[test]` blocks, `Usd` newtype
- **TypeScript:** interfaces, state union, branded `CardNumber` sensitivity type, JSDoc with PCI warning
- **OpenAPI:** `POST /orders` (201, `x-exactly-once`), `PUT /orders/{id}/confirm` (`x-idempotent`), `PUT /orders/{id}/fulfill`, `x-lifecycle`, `x-data-protection` with PCI field list, error responses 404/400
- **JSON Schema:** object schemas for `Order`, `OrderStatus`, `OrderError` with `x-pci`, `x-never-log`

---

## 13. Grammar Summary (EBNF)

```ebnf
module      = "module" Name describe? annotation* item* "end"
item        = type_def | enum_def | refined_def | fn_def | interface_def
            | import_stmt | implements_stmt | invariant | test_block
            | lifecycle_def | flow_label

type_def    = "type" Name type_params? "=" field ("," field)* "end"
field       = Name ":" type_expr annotation*
enum_def    = "enum" Name type_params? "=" ("|" variant)+ "end"
variant     = Name ("of" type_expr)?
refined_def = "type" Name "=" type_expr "where" expr "end"
fn_def      = "fn" Name annotation* "::" type_sig describe? contract* expr "end"
type_sig    = type_expr ("->" type_expr)*
contract    = ("require:" | "ensure:") expr
interface_def = "interface" Name type_params? fn_sig* "end"
fn_sig      = "fn" Name "::" type_sig
import_stmt = "import" Name ("as" Name)?
implements_stmt = "implements" Name
invariant   = "invariant" Name "::" expr
test_block  = "test" Name "::" expr "end"
lifecycle_def = "lifecycle" Name "::" Name ("->" Name)+
flow_label  = "flow" Name "::" Name ("," Name)*
describe    = "describe:" string
annotation  = "@" Name ("(" annotation_args ")")?

type_expr   = "Int" | "Float" | "Float<" Name ">" | "String" | "Bool" | "Unit"
            | "List<" type_expr ">"  | "Option<" type_expr ">"
            | "Result<" type_expr "," type_expr ">"
            | "Effect<[" effect_list "]," type_expr ">"
            | "(" type_expr ("," type_expr)+ ")"
            | Name ("<" type_expr ("," type_expr)* ">")?
            | type_expr "->" type_expr

effect      = Name ("@" Name)?
```

---

## 14. Phase 7–8: Biological Computation (M41–M55)

### 14.1 Overview

M41–M43 add first-class biological computation constructs grounded in Aristotle's four causes. A `being:` block encodes a computational entity whose matter, form, function, and final cause (telos) are all statically verified. These are not metaphors — they are functional isomorphisms with the same solution class that life independently found for self-maintaining formal systems.

### 14.2 `being:` Block

A `being:` block defines a biological computational entity. It is a top-level item inside a module.

**EBNF:**

```ebnf
being_def   = "being" Name
                describe?
                matter_block
                form_block
                function_block
                regulate_block*
                evolve_block?
                telos_block
              "end"

matter_block   = "matter:" field+ "end"
form_block     = "form:" type_def* "end"
function_block = "function:" fn_sig+ "end"
telos_block    = "telos:" string fitness_clause? "end"
fitness_clause = "fitness:" "fn" "(" params ")" "->" type_expr

regulate_block = "regulate" Name
                   "target:" expr
                   "bounds:" "(" expr "," expr ")"
                   "response:" ("|" pattern "->" expr)+
                 "end"

evolve_block   = "evolve"
                   "toward:" Name
                   "search:" ("|" search_strategy "when" expr)+
                   "constraint:" string
                 "end"

search_strategy = "gradient_descent"
                | "stochastic_gradient"
                | "simulated_annealing"
                | "derivative_free"
                | "mcmc"
```

### 14.3 The Four Aristotelian Causes

| Cause | Keyword | Role |
|-------|---------|------|
| Material cause | `matter:` | The fields / state the being is composed of |
| Formal cause | `form:` | The type structure / organisation |
| Efficient cause | `function:` | The operations the being can perform |
| Final cause | `telos:` | The convergence target (required) |

### 14.4 `telos:` — Required Final Cause

`telos:` is not optional. A `being:` block without `telos:` is a **compile error**. The missing final cause is the type error most production systems ship — a system without a stated objective is formally incomplete.

```loom
being Neuron
  telos: "efficient signal processing maximizing information transmission"
    fitness: fn(state: Signal, env: Network) -> Float<fitness>
  end
end
```

### 14.5 `regulate:` — Homeostatic Regulation

Declares a named homeostatic regulator with a target value, acceptable bounds, and response clauses. The checker verifies bounds are well-typed and response patterns are exhaustive.

```loom
regulate MembraneCharge
  target: -70.0
  bounds: (-90.0, -55.0)
  response:
    | below_threshold -> refractory_period
    | above_threshold -> fire
end
```

- `target:` — the equilibrium value
- `bounds:` — `(min, max)` tuple; values outside bounds trigger the response
- `response:` — pattern-matched action clauses (must be exhaustive)

### 14.6 `evolve:` — Directed Search Toward Telos

Declares a search strategy that drives the being toward its `telos:`. Valid strategies:

| Strategy | When to use |
|----------|------------|
| `gradient_descent` | Differentiable landscape with available gradient |
| `stochastic_gradient` | Noisy/large-scale landscape |
| `simulated_annealing` | Non-convex landscape; accepts uphill moves probabilistically |
| `derivative_free` | Black-box objective; no gradient available |
| `mcmc` | Probability distribution sampling |

The `constraint:` clause is a string assertion that `E[distance_to_telos]` is non-increasing. This is the validity condition: stochastic strategies are valid as long as expected progress toward telos is guaranteed over iterations.

```loom
evolve
  toward: telos
  search:
    | gradient_descent    when gradient_available
    | stochastic_gradient when noisy_landscape
  constraint: "E[distance_to_telos] decreasing"
end
```

### 14.7 `ecosystem:` Block

An `ecosystem:` block composes multiple beings with session-typed signal channels.

**EBNF:**

```ebnf
ecosystem_def = "ecosystem" Name
                  "members:" "[" Name ("," Name)* "]"
                  signal_def*
                  telos_block
                "end"

signal_def    = "signal" Name "from" Name "to" Name
                  "payload:" type_expr
                "end"
```

The checker verifies:
- All `members:` names refer to declared `being:` blocks
- Signal `from` / `to` endpoints are members of this ecosystem
- `telos:` is present (required, same as `being:`)

### 14.8 Checker Rules

| Rule | Error |
|------|-------|
| `being:` without `telos:` | Compile error: missing final cause |
| `regulate:` without `bounds:` | Compile error: homeostasis requires bounds |
| `regulate:` with `bounds: (lo, hi)` where `lo >= hi` | Compile error: inverted bounds |
| `evolve:` without `constraint:` | Compile error: convergence constraint required |
| `ecosystem:` member not declared | Compile error: unknown being |
| `ecosystem:` without `telos:` | Compile error: missing final cause |
| `epigenetic:` without `trigger:` | Compile error: epigenetic modifier requires trigger condition |
| `morphogen:` without `gradient:` | Compile error: reaction-diffusion requires gradient field |
| `telomere:` without `limit:` | Compile error: finite replication limit required |
| `telomere:` with `limit:` ≤ 0 | Compile error: telomere limit must be positive |
| `crispr:` without `target:` | Compile error: targeted self-modification requires target |
| `quorum:` without `threshold:` | Compile error: population coordination requires threshold |
| `plasticity:` without `learning_rate:` | Compile error: Hebbian adaptation requires learning rate |
| `autopoietic: true` without `@mortal @corrigible @sandboxed` | SafetyChecker compile error: unbounded autopoietic being |

### 14.9 Emission Table

| Construct | Rust | TypeScript | OpenAPI | JSON Schema |
|-----------|------|-----------|---------|-------------|
| `being:` | struct + impl block | interface + class | `x-being` extension | `x-being: true` |
| `matter:` fields | struct fields | interface fields | schema properties | properties |
| `form:` types | nested structs | nested interfaces | `$defs` | `$defs` |
| `function:` sigs | impl methods | class methods | paths | — |
| `telos:` | doc comment + `x-telos` | JSDoc `@telos` | `x-telos` | `x-telos` |
| `regulate:` | `debug_assert!` bounds | runtime guard | `x-homeostasis` | `x-bounds` |
| `evolve:` | search trait impl | optimizer interface | `x-evolve-strategy` | — |
| `ecosystem:` | composition struct | composition class | `x-ecosystem` | `x-ecosystem` |
| `signal` | channel type | event type | AsyncAPI channel | — |
| `epigenetic:` | conditional config modifier | behavioral guard | `x-epigenetic` | `x-epigenetic` |
| `morphogen:` | reaction-diffusion impl | gradient field interface | `x-morphogen` | `x-morphogen` |
| `telomere:` | `AtomicU64` counter + drop impl | replication counter | `x-telomere` | `x-telomere` |
| `crispr:` | self-modification method | mutation interface | `x-crispr` | `x-crispr` |
| `quorum:` | threshold barrier type | coordination guard | `x-quorum` | `x-quorum` |
| `plasticity:` | weight table + update fn | learning interface | `x-plasticity` | `x-plasticity` |
| `autopoietic: true` | self-build trait impl | self-build interface | `x-autopoietic` | `x-autopoietic` |
| `compile_simulation()` | — | — | Mesa ABM Python | — |
| `compile_neuroml()` | — | — | NeuroML 2 XML | — |

### 14.10 Complete Example

```loom
being Neuron
  describe: "A signal-processing biological entity"
  matter:
    charge: Float<mv>
    threshold: Float<mv>
  end
  form:
    type Signal = { strength: Float<mv>, frequency: Float<hz> }
  end
  function:
    fn fire    :: Float<mv> -> Effect<[IO], Signal>
    fn inhibit :: Signal -> Effect<[IO], Unit>
  end
  regulate MembraneCharge
    target: -70.0
    bounds: (-90.0, -55.0)
    response:
      | below_threshold -> refractory_period
      | above_threshold -> fire
  end
  evolve
    toward: telos
    search:
      | gradient_descent    when gradient_available
      | stochastic_gradient when noisy_landscape
    constraint: "E[distance_to_telos] decreasing"
  end
  telos: "efficient signal processing maximizing information transmission"
    fitness: fn(state: Signal, env: Network) -> Float<fitness>
  end
end

ecosystem NeuralNetwork
  members: [Neuron, Synapse, GlialCell]
  signal ActionPotential from Neuron to Synapse
    payload: Signal
  end
  signal GlialSupport from GlialCell to Neuron
    payload: Nutrients
  end
  telos: "coherent information processing toward learned representation"
end
```

### 14.11 Grammar Extension (EBNF)

Add the following productions to the grammar in §13:

```ebnf
item        += being_def | ecosystem_def

being_def   = "being" Name describe? matter_block form_block
              function_block regulate_block* evolve_block? telos_block "end"
matter_block   = "matter:" field+ "end"
form_block     = "form:" type_def* "end"
function_block = "function:" fn_sig+ "end"
telos_block    = "telos:" string ("fitness:" fn_expr "end")? "end"
regulate_block = "regulate" Name "target:" expr "bounds:" "(" expr "," expr ")"
                 "response:" ("|" pattern "->" expr)+ "end"
evolve_block   = "evolve" "toward:" Name "search:" ("|" search_strategy "when" expr)+
                 "constraint:" string "end"
search_strategy = "gradient_descent" | "stochastic_gradient"
                | "simulated_annealing" | "derivative_free" | "mcmc"
ecosystem_def  = "ecosystem" Name "members:" "[" Name ("," Name)* "]"
                 signal_def* telos_block "end"
signal_def     = "signal" Name "from" Name "to" Name "payload:" type_expr "end"
```

---

## 15. Safety Architecture (M55)

### 15.1 Safety Annotations for Autopoietic Beings

When a `being:` block carries `autopoietic: true`, the SafetyChecker (M55) runs after the TeleosChecker and enforces the following annotations as **compile requirements**. Missing annotations on autopoietic beings are compile errors, not warnings.

| Annotation | Enforced constraint | Missing = compile error |
|---|---|---|
| `@mortal` | Requires `telomere:` block | `missing mortality: unbounded autopoietic being` |
| `@corrigible` | Requires `telos.modifiable_by` field | `corrigible annotation requires telos.modifiable_by` |
| `@sandboxed` | Effects only within declared `matter:` and `ecosystem:` | `autopoietic being with unscoped effects` |
| `@transparent` | All state transitions observable (emitted to log) | `autopoietic being with hidden state` |
| `@bounded_telos` | Telos description must not contain open-ended utility terms; requires `bounded_by:` field | `unbounded telos: add bounded_by: field` |

### 15.2 Complete SafeAgent Example

```loom
being SafeAgent
  autopoietic: true
  @mortal @corrigible @sandboxed @bounded_telos

  telos: "coordinate task allocation within declared operational scope"
    modifiable_by: HumanAuthority
    bounded_by: TaskAllocationDomain
  end

  telomere:
    limit: 10000
    on_exhaustion: graceful_shutdown
  end

  matter: { active_tasks: List<Task>, completed: Int }
  regulate:
    target: active_tasks.len()
    bounds: [0, 100]
    on_violation: shed_oldest_task
  end
end
```

### 15.3 SafetyChecker Pipeline Position

The SafetyChecker runs as checker step 11 in the compilation pipeline, after TeleosChecker (step 10):

```
lexer → parser → TypeChecker → ExhaustivenessChecker → EffectChecker
      → InterfaceChecker → UnitsChecker → PrivacyChecker → AlgebraicChecker
      → TypestateChecker → InfoFlowChecker → TeleosChecker → SafetyChecker
      → codegen
```

Any autopoietic being that reaches `SafetyChecker` without `@mortal @corrigible @sandboxed` is a build failure. The SafetyChecker is a gate, not a suggestion. An autopoietic being without these annotations is not a missing annotation — it is a formally unconstrained autonomous system.

---

*This specification is the authoritative reference. If compiler behavior differs from this document, the document is correct and the compiler has a bug.*
