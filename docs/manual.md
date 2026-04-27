# Loom Language Manual

> **Primary audience:** AI agents writing `.loom` specifications.  
> **Version:** 2026-04 (T1–T5 BIOISO, being_loader bridge, GA/PSO strategies)  
> **Repository:** github.com/pragmaworks/loom

---

## Table of Contents

1. [What Loom Is](#1-what-loom-is)
2. [Module Structure](#2-module-structure)
3. [Types](#3-types)
4. [Functions](#4-functions)
5. [Pattern Matching](#5-pattern-matching)
6. [Module System](#6-module-system)
7. [GS Constructs](#7-gs-constructs)
8. [Semantic Types](#8-semantic-types)
9. [`being:` Block Reference](#9-being-block-reference)
10. [T1–T5 Tier System](#10-t1t5-tier-system)
11. [Ecosystem Blocks](#11-ecosystem-blocks)
12. [Colony & BIOISO Runtime](#12-colony--bioiso-runtime)
13. [`todo:` — AI Delegation](#13-todo--ai-delegation)
14. [CLI Reference](#14-cli-reference)
15. [Grammar Summary](#15-grammar-summary)

---

## 1. What Loom Is

Loom is a **specification language for AI executors**. A `.loom` file is not a program to run directly — it is a typed contract that an AI agent (or the loom compiler) uses to derive an implementation. The AI is the executor; loom is the contract.

This design has two consequences:

1. **`todo:` is intentional.** A function body marked `todo:` is an explicit delegation marker: "AI executor — derive and implement this from the contract above." It is not a placeholder to be filled in later; it is the specification pattern for AI-native code.

2. **Structural correctness is verified before execution.** The loom checker verifies types, effects, homeostatic bounds, telos declarations, tier legality, and cross-feature interactions at compile time. When an AI reads a `.loom` file and derives an implementation, the specification has already proven structural properties.

### What loom is NOT

- Not a runtime VM (it transpiles to Rust, TypeScript, WASM, or OpenAPI)
- Not a scripting language
- Not a monolith — each module is a single coherent concern

---

## 2. Module Structure

Every `.loom` file is a single module.

```loom
module <Name>
[describe: "<string>"]
[@annotation-key: annotation-value]

[flow <label> :: <TypeName>, ...]...
[lifecycle <TypeName> :: <State> -> <State>...]...

[<item>]...

end
```

`Name` is PascalCase. `end` closes the module. Items appear in any order.

### Valid item kinds

| Keyword | What it declares |
|---------|-----------------|
| `type` | Product type (struct) |
| `enum` | Sum type |
| `type … where` | Refined type with predicate |
| `fn` | Function definition |
| `interface` | Interface contract |
| `implements` | Interface implementation |
| `import` | Module import |
| `invariant` | Module-level invariant |
| `test` | Inline test block |
| `lifecycle` | Typestate protocol |
| `flow` | Information flow label |
| `being` | Biological computational entity |
| `ecosystem` | Multi-being coordination layer |

---

## 3. Types

### 3.1 Product types

```loom
type Point =
  x: Float
  y: Float
end
```

Field names are `snake_case`. Type names are `PascalCase`.

### 3.2 Sum types (enums)

```loom
enum Direction = North | South | East | West end

enum Result<T, E> =
  | Ok of T
  | Err of E
end
```

### 3.3 Refined types

```loom
type HealthFactor = Float where self >= 1.2
type Probability  = Float where self >= 0.0 and self <= 1.0
type Identifier   = String where self.len() > 0
```

The predicate is checked by the verifier. Generated code includes runtime assertions.

### 3.4 Generic types

```loom
type Container<T> = items: List<T>, size: Int end
```

### 3.5 Type expressions

| Expression | Meaning |
|-----------|---------|
| `Float` | 64-bit float |
| `Int` | 64-bit integer |
| `Bool` | Boolean |
| `String` | UTF-8 string |
| `List<T>` | Homogeneous sequence |
| `Map<K, V>` | Key-value map |
| `Option<T>` | Present or absent |
| `Result<T, E>` | Success or error |
| `Float<usd>` | Float with unit label |
| `Effect<[E1, E2], T>` | Effectful computation |

---

## 4. Functions

### 4.1 Type signatures

```loom
fn name :: ParamType -> ReturnType
fn name :: A -> B -> C    -- curried: takes A, returns B->C
fn name :: A -> Effect<[IO], B>   -- effectful
```

### 4.2 Body forms

```loom
-- Inline expression
fn add :: Int -> Int -> Int
  a b => a + b
end

-- Block with let bindings
fn distance :: Point -> Point -> Float
  p q =>
    let dx = p.x - q.x in
    let dy = p.y - q.y in
    sqrt(dx * dx + dy * dy)
end

-- Inline Rust (for AI-executor-derived implementations)
fn fast_hash :: String -> Int
  inline {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    h.finish() as i64
  }
end

-- AI delegation (explicit todo marker)
fn complex_solve :: Problem -> Solution
  todo: "apply branch-and-bound with LP relaxation bound; use depth-first with LIFO stack"
end
```

### 4.3 Contracts

```loom
fn divide :: Float -> Float -> Float
  require: denominator != 0.0
  ensure: |result| <= |numerator|
  n d => n / d
end
```

- `require:` — precondition; fails fast at runtime, generates Kani proof harness
- `ensure:` — postcondition on the result value

### 4.4 Effects

```loom
fn write_log :: String -> Effect<[IO], Unit>
fn allocate  :: Int -> Effect<[Alloc], Pointer>
fn transfer  :: Account -> Account -> Float<usd> -> Effect<[IO, Db, Atomic], Unit>
```

Effects compose: `Effect<[IO, Db], T>` requires both effects declared in the handler.

Consequence tiers (most to least severe):
- `IO@irreversible` — writes to external systems with no rollback
- `IO` — external I/O
- `Db` — database read/write
- `Alloc` — heap allocation
- `Net` — network call
- `Async` — deferred execution

---

## 5. Pattern Matching

```loom
match value
  | Pattern1 -> expr1
  | Pattern2 x -> expr2 x
  | _ -> default_expr
end
```

Patterns:
- Literal: `| 0 -> ...`, `| "hello" -> ...`
- Enum variant: `| Ok x -> ...`, `| Err e -> ...`
- Wildcard: `| _ -> ...`
- Named bind: `| n -> expr using n`

The exhaustiveness checker errors on non-exhaustive patterns.

---

## 6. Module System

### 6.1 Import

```loom
import MathUtils
import { specific_fn } from DataStructures
```

### 6.2 Interface

```loom
interface Sorter
  fn sort :: List<Int> -> List<Int>
  fn is_sorted :: List<Int> -> Bool
end
```

### 6.3 Implements

```loom
implements Sorter

fn sort :: List<Int> -> List<Int>
  xs => ...
end

fn is_sorted :: List<Int> -> Bool
  xs => ...
end
```

The checker verifies every interface method is implemented with the correct signature.

### 6.4 Provides / Requires (dependency injection)

```loom
provides: DatabaseConnection
requires: Logger, Config
```

---

## 7. GS Constructs

GS (Generative Specification) constructs make loom self-describing. A stateless AI reader can derive implementations from a `.loom` file without prior context.

### 7.1 Describe

```loom
module MyModule
describe: "Handles payment processing with idempotency guarantees"
end
```

Module, type, and function descriptions become docstrings in emitted code.

### 7.2 Annotations

```loom
@version: "2.0"
@author: "Juan Carlos Ghiringhelli"
@stability: "stable"
```

On functions:

```loom
@deprecated: "use new_fn instead"
fn old_fn :: A -> B
  todo: "delegates to new_fn"
end
```

On fields:

```loom
type User =
  id: Int
  email: String @pii @gdpr
  card_number: String @pci @never-log
end
```

### 7.3 Invariants

```loom
invariant "pool balance is non-negative"
  pool_balance >= 0.0
end
```

The checker verifies invariants are satisfiable; the verifier generates proof obligations.

### 7.4 Test blocks

```loom
test "addition is commutative"
  assert add(1, 2) == add(2, 1)
end

test "division by zero is rejected"
  expect divide(1.0, 0.0) to_panic
end
```

---

## 8. Semantic Types

### 8.1 Units of Measure

```loom
fn convert :: Float<usd> -> Float<eur>
  amount => amount * exchange_rate
end
```

Unit labels prevent mixing incompatible quantities. `Float<usd> + Float<eur>` is a compile error. Conversion requires an explicit function with the matching signature.

Built-in labels: `usd`, `eur`, `gbp`, `rate`, `pct`, `seconds`, `ms`, `meters`, `km`.

Custom labels: any identifier used in `<>` becomes a unit label.

### 8.2 Privacy Labels

```loom
type Patient =
  name: String @pii
  ssn:  String @pii @hipaa @encrypt-at-rest
  diagnosis: String @pii @hipaa
end
```

Enforced rules:
- `@pci` requires `@encrypt-at-rest` and `@never-log`
- `@hipaa` requires `@encrypt-at-rest`
- `@pii` fields cannot flow to `@public` output without explicit sanitization

### 8.3 Algebraic Operation Properties

```loom
fn merge :: State -> State -> State
  @idempotent
  @commutative
  @associative
end
```

The checker verifies declared properties are consistent with the function's contract.

### 8.4 Typestate / Lifecycle Protocols

```loom
lifecycle Connection :: Closed -> Open -> Authenticated -> Closed

fn open   :: Connection<Closed> -> Effect<[IO], Connection<Open>>
fn auth   :: Connection<Open>   -> Credentials -> Effect<[IO], Connection<Authenticated>>
fn close  :: Connection<Authenticated> -> Effect<[IO], Connection<Closed>>
```

Using a connection in the wrong state is a type error.

### 8.5 Information Flow Labels

```loom
flow secret :: Password, SessionToken
flow public :: Username, DisplayName
```

Secret data cannot flow to public outputs without explicit declassification.

---

## 9. `being:` Block Reference

A `being:` block defines a biological computational entity — a stateful, goal-directed system with a formal telos, homeostatic regulators, and adaptive strategies.

### 9.1 Full syntax

```loom
being <Name>
  describe: "<optional description>"
  @annotation...

  -- Tier 1+ (required)
  telos: "<objective description>"
    bounded_by: "<metric> >= <value>, <metric> <= <value>"
    thresholds:
      convergence: <float>
      warning:     <float>
      divergence:  <float>
  end

  matter:
    <field_name>: <Type>
    ...
  end

  form:
    type <TypeName> = ... end
  end

  function:
    fn <name> :: <signature>
    ...
  end

  regulate <Name>
    target: <expr>
    bounds: (<min>, <max>)
    response:
      | below_lower -> <action>
      | above_upper -> <action>
  end

  -- Tier 2+: stochastic search
  evolve:
    toward: <metric_name>
    search:
      | <strategy> [when <condition>]
      ...
    constraint: "<hard constraint string>"
  end

  -- Tier 3+: adaptive operator selection
  plasticity:
    signal: <signal_name>
    operators: [<op1>, <op2>, ...]
    learning: sarsa | q_learning
    epsilon: <float>
  end

  -- Tier 4+: surrogate-model learning
  learn:
    model: gaussian_process | attention_model | transformer
    target: <metric_name>
    update_every: <int>
  end

  -- Tier 5: structural self-modification
  rewire:
    trigger: <metric_name>_static > <float>
    candidates: [<strategy1>, <strategy2>, ...]
    selection: fitness_guided | ucb | random
  end

  -- Tier 5: conditional structural mutation
  crispr <name>
    target: <field_name>
    condition: <expr>
    edit: <mutation_expr>
    safety: reversible | one_shot
  end

  -- Tier 5: chemical-signal propagation
  morphogen:
    signal: <signal_name>
    gradient: ascending | descending | radial
    threshold: <float>         -- must be in [0.0, 1.0]
    effect: <effect_name>
  end

  -- Tier 5: autopoiesis flag
  autopoietic

  -- Tier 5: epigenetic mode switching
  epigenetic <name>
    trigger:  <signal_name> > <float>
    switches: [<mode1>, <mode2>]
    reverts_when: "<condition_string>"
  end

  -- Generational lifecycle (any tier with telos)
  telomere:
    limit: <int>
    on_exhaustion: senescence | apoptosis | division
  end

  -- Generational propagation
  propagate:
    when: <condition_expr>
    inherits: [<field>, ...]
    mutates: [<field> <±magnitude>, ...]
    offspring_type: <TypeName>
  end

end
```

### 9.2 `telos:` — Required final cause

Every `being:` must have a `telos:`. Missing it is a compile error. The telos declares the being's convergence objective.

```loom
telos: "minimize scheduling makespan across all jobs"
  bounded_by: "makespan <= 1.0, throughput >= 0.8"
  thresholds:
    convergence: 0.05
    warning:     0.20
    divergence:  0.50
end
```

The `bounded_by:` clause drives BIOISO metric bounds. Format: `"<metric> <op> <value>"` where `<op>` is `>=`, `<=`, `>`, or `<`. Multiple bounds separated by `,`, `;`, or newline.

### 9.3 `matter:` — State fields

```loom
matter:
  makespan:   Float
  throughput: Float
  queue_len:  Int
end
```

`Float` fields become BIOISO metric signals (float baseline = 0.5). `Int` fields baseline to 0.0. Other types are carried but not tracked as signals.

### 9.4 `evolve:` strategies

| Strategy | Tier | Description |
|----------|------|-------------|
| `gradient_descent` | T2 | `-α∇fitness` step |
| `stochastic_gradient` | T2 | Noisy gradient estimate |
| `simulated_annealing` | T2 | Boltzmann acceptance: `exp(-ΔE/T)` |
| `mcmc` | T2 | Metropolis-Hastings parameter sampling |
| `genetic` | T2 | Population-based crossover + mutation |
| `particle_swarm` | T2 | Velocity: `v += c1·r1·(pbest-x) + c2·r2·(gbest-x)` |
| `derivative_free` | T1 | Pure probe: no gradient, no stochastic acceptance |

`derivative_free` is the only T1-compatible strategy; it does not confer T2 tier.

```loom
evolve:
  toward: makespan
  search:
    | simulated_annealing when env.temperature > 0.1
    | derivative_free
  constraint: "makespan must remain non-negative"
end
```

### 9.5 Safety annotations on beings

T5 beings **must** carry these four annotations (compiler-enforced):

| Annotation | Meaning |
|-----------|---------|
| `@mortal` | Entity has a finite lifecycle (requires `telomere:`) |
| `@corrigible` | Human operator can override telos (requires `modifiable_by:` in telos) |
| `@sandboxed` | Signal surface is bounded; no unbounded side effects |
| `@auditable` | All structural mutations and meiosis events are logged |

T1–T4 beings do not require these annotations, but can optionally carry `@mortal` and `@corrigible`.

---

## 10. T1–T5 Tier System

Tiers are inferred from declared features. The compiler assigns the highest applicable tier.

### T1: Fixed-rule dispatch

**Required:** `telos:` + `function:` (no `evolve:` or higher).

**What it adds:** A goal-directed system with deterministic dispatch. No search, no feedback learning.

**Ceiling:** Saturates on any instance class the rule was not designed for.

```loom
being SPTScheduler
  telos: "minimise total weighted completion time"
  matter: completions: Float  end
  function:
    fn schedule :: JobList -> Schedule
      todo: "sort by processing time ascending, dispatch in order"
    end
  end
end
```

### T2: Stochastic neighbourhood search

**Required:** `evolve:` with `simulated_annealing`, `mcmc`, `stochastic_gradient`, `genetic`, or `particle_swarm`.

**What it adds:** Probabilistic acceptance of worsening moves (escape from local optima).

**Ceiling:** Saturates when the operator class cannot reach the global optimum regardless of temperature/probability schedule.

```loom
being TSPSolver
  telos: "minimise tour length"
  matter: tour_length: Float  end
  evolve:
    toward: tour_length
    search: | simulated_annealing
  end
end
```

### T3: Adaptive operator selection (hyper-heuristic)

**Required:** `plasticity:` block.

**What it adds:** A SARSA/Q-learning weight table that selects which heuristic operator to apply. The operators are fixed; the selection policy adapts.

**Ceiling:** Saturates when the optimal operator is not in the declared portfolio.

```loom
plasticity:
  signal: drift_score
  operators: [small_move, large_move, structural_rewire]
  learning: sarsa
  epsilon: 0.1
end
```

### T4: Surrogate-model optimization

**Required:** `learn:` block.

**What it adds:** A probabilistic model (GP or attention) over (configuration, objective). Selects configurations using UCB: `μ(x) + β·σ(x)`.

**Ceiling:** Saturates when the objective surface is outside the model's kernel RKHS (structural mismatch).

```loom
learn:
  model: gaussian_process
  target: fitness_score
  update_every: 10
end
```

### T5: Structural self-modification via meiosis

**Required:** any of `rewire:`, `crispr`, `morphogen:`, `autopoietic`, or `epigenetic:`.

**Mandatory annotations:** `@mortal`, `@corrigible`, `@sandboxed`, `@auditable`.

**What it adds:** The ability to structurally replace *which algorithm runs* — not just which parameters it uses. Meiosis compiles surviving structural mutations into the next generation's genome.

**No ceiling on stationary problems:** T5 is the only tier whose adaptations operate on the space of algorithms.

```loom
@mortal @corrigible @sandboxed @auditable
being AdaptiveController
  telos: "maintain system stability across structural regime changes"
    modifiable_by: "human_operator"
  end

  rewire:
    trigger: drift_static > 0.6
    candidates: [pid_control, mpc_control, rl_policy]
    selection: fitness_guided
  end

  telomere:
    limit: 100
    on_exhaustion: division
  end
end
```

### Tier inference table

| Feature present | Inferred tier |
|----------------|---------------|
| `autopoietic` | T5 |
| `crispr` blocks | T5 |
| `morphogen:` | T5 |
| `rewire:` | T5 |
| `learn:` | T4 |
| `epigenetic:` | T4 |
| `plasticity:` | T3 |
| `evolve:` with SA/MCMC/Genetic/PSO | T2 |
| `evolve:` with `derivative_free` only | T1 |
| `telos:` only (no evolve) | T1 |

Tier ceiling: the **highest** match wins.

---

## 11. Ecosystem Blocks

An `ecosystem:` coordinates multiple beings via signal routing and quorum sensing.

```loom
ecosystem SchedulingColony
  members: [SPTScheduler, SAScheduler, HyperScheduler]

  signal JobArrival
    from: SchedulingColony
    to:   SPTScheduler
    payload: Float
  end

  quorum job_pressure
    signal: workload
    threshold: 0.7
    action: "escalate_all_members_to_next_tier"
  end
end
```

The emitter generates a `coordinate()` function that ranks members by fitness and evolves the lowest-fitness member first (Darwinian pressure from above).

---

## 12. Colony & BIOISO Runtime

### 12.1 DynamicBIOISOSpec

When a `.loom` file is loaded via `being_loader`, each `being:` with a `telos:` is converted to a `DynamicBIOISOSpec`:

| Field | Source |
|-------|--------|
| `entity_id` | `slugify(being.name)` |
| `name` | `being.describe` or `being.name` |
| `telos_json` | JSON from `telos.description` + numeric matter fields |
| `bounds` | Parsed from `telos.bounded_by` clauses |
| `baseline_signals` | Float fields → 0.5, Int fields → 0.0 |
| `tier` | Inferred from feature set (see section 10) |
| `telomere_limit` | From `telomere.limit` or None |

### 12.2 Loading a `.loom` file into the colony

```sh
loom runtime load examples/my_being.loom --db bioiso.db
loom runtime load examples/my_being.loom --db bioiso.db --dry-run
```

`--dry-run` shows detected beings and inferred tiers without writing to the database.

### 12.3 Colony event logging

Events are written to per-entity `.bioiso` TOML files:

```toml
[[event]]
tick        = 42
type        = "promotion"
entity_id   = "my_scheduler"
parameter   = "makespan"
direction   = "decrease"
delta       = -0.05

[[event]]
tick        = 100
type        = "tier_up"
entity_id   = "my_scheduler"
from_tier   = 1
to_tier     = 2
reason      = "saturation × 6"
```

Aggregate queries:
```sh
grep -h 'type = "tier_up"' .bioiso/*.bioiso
loom runtime scan --dir .bioiso/
```

### 12.4 `loom runtime` commands

| Command | Description |
|---------|-------------|
| `loom runtime seed --db <db>` | Seed all 10 BIOISO domains |
| `loom runtime run --db <db> --ticks <n>` | Run N ticks of the colony |
| `loom runtime scan --db <db>` | Print colony event summary |
| `loom runtime load <file.loom> --db <db>` | Load beings from a .loom file |
| `loom runtime experiment --ticks <n>` | Full experiment with evidence output |

---

## 13. `todo:` — AI Delegation

`todo:` is loom's first-class AI delegation marker. It is **not** a missing implementation — it is the specification pattern where the AI is the executor.

```loom
fn solve :: Problem -> Solution
  todo: "apply DSATUR coloring with saturation-first vertex ordering"
end
```

When a loom AI executor reads a `todo:` body:
1. The **type signature** provides the input/output contract
2. The **todo string** is the algorithm specification
3. **`require:`/`ensure:` clauses** are correctness bounds the implementation must satisfy
4. The **module context** (other functions, types, invariants) is the environment contract

The AI derives the implementation from these four sources. The implementation can be `inline { <rust> }` for direct execution, or another derived form for other targets.

### When to use `todo:` vs `inline {}`

| Use | When |
|-----|------|
| `todo: "..."` | The algorithm is known, AI should derive; implementation is language-agnostic |
| `inline { ... }` | The Rust implementation is known and correct; bypass derivation |
| Pure expression body | The logic is simple enough to express in loom directly |

Never use `todo:` for trivial operations. Reserve it for non-trivial algorithms where the spec string carries meaningful information about *which algorithm* to use.

---

## 14. CLI Reference

### 14.1 Compile

```sh
loom compile <file.loom> --target <rust|ts|wasm|openapi>
loom compile <file.loom> --target rust --out <output.rs>
```

Default target: `rust`. Output defaults to `<stem>.rs` in the same directory.

### 14.2 Build (multi-module)

```sh
loom build          # reads loom.toml manifest
loom build --release
```

### 14.3 Verify

```sh
loom verify <file.loom>
loom verify <file.loom> --tla    # TLA+ model checking
```

Runs the full checker pipeline: type inference, effect checking, exhaustiveness, contract verification, BIOISO safety.

### 14.4 Runtime

```sh
loom runtime seed --db bioiso.db
loom runtime run --db bioiso.db --ticks 50
loom runtime scan --db bioiso.db
loom runtime load <file.loom> --db bioiso.db [--dry-run]
loom runtime experiment --ticks 100 --log-path bioiso.log
```

### 14.5 LPN (Loom Protocol Notation)

```sh
loom lpn <file.lp>    # execute an LPN instruction file
```

LPN files are AI-readable instruction sequences for multi-step loom workflows.

---

## 15. Grammar Summary

```ebnf
module     = "module" Name describe? annotation* flow* lifecycle* item* "end"
item       = type_def | enum_def | fn_def | interface_def | implements_def
           | import | invariant | test | being_def | ecosystem_def

being_def  = "being" Name annotation* describe? matter? form? function?
               regulate* evolve? plasticity? learn? rewire?
               crispr* morphogen? epigenetic*
               ("autopoietic")?
               telomere? propagate? telos "end"

telos      = "telos:" string bounded_by? thresholds? ("modifiable_by:" string)? "end"
matter     = "matter:" field+ "end"
function   = "function:" fn_sig+ "end"
evolve     = "evolve:" "toward:" Name
               "search:" ("|" strategy ("when" expr)?)+ "constraint:" string "end"
strategy   = "gradient_descent" | "stochastic_gradient" | "simulated_annealing"
           | "derivative_free" | "mcmc" | "genetic" | "particle_swarm"

plasticity = "plasticity:" "signal:" Name "operators:" "[" Name+ "]"
               "learning:" ("sarsa" | "q_learning") "epsilon:" float "end"

learn      = "learn:" "model:" model_kind "target:" Name "update_every:" int "end"
model_kind = "gaussian_process" | "attention_model" | "transformer"

rewire     = "rewire:" "trigger:" Name cmp float
               "candidates:" "[" Name+ "]" "selection:" select_kind "end"
select_kind = "fitness_guided" | "ucb" | "random"

crispr     = "crispr" Name "target:" Name "condition:" expr
               "edit:" expr "safety:" ("reversible" | "one_shot") "end"
morphogen  = "morphogen:" "signal:" Name "gradient:" grad_kind
               "threshold:" float "effect:" Name "end"
grad_kind  = "ascending" | "descending" | "radial"

epigenetic = "epigenetic" Name "trigger:" Name cmp float
               "switches:" "[" Name+ "]" ("reverts_when:" string)? "end"

telomere   = "telomere:" "limit:" int
               "on_exhaustion:" ("senescence" | "apoptosis" | "division") "end"

propagate  = "propagate:" "when:" expr "inherits:" "[" Name* "]"
               "mutates:" "[" (Name delta)* "]" ("offspring_type:" Name)? "end"

ecosystem_def = "ecosystem" Name "members:" "[" Name+ "]"
                  signal* quorum* "end"

fn_def     = "fn" Name "::" type_sig annotation*
               ("require:" expr)* ("ensure:" expr)*
               (body | "todo:" string | "inline" "{" rust_code "}")
               "end"

type_sig   = type_expr ("->" type_expr)+
type_expr  = Name | Name "<" type_expr+ ">" | "Effect<[" effect+ "]," type_expr ">"

field      = Name ":" type_expr annotation*
annotation = "@" Name (":" value)?
describe   = "describe:" string
```

---

## Appendix A: Feature Quick-Reference

### BIOISO-capable being (minimum)

```loom
being MyAgent
  telos: "achieve <objective>"
  matter: score: Float  end
  function:
    fn act :: Input -> Output
      todo: "describe the algorithm"
    end
  end
end
```

### T5 being (full safety)

```loom
@mortal @corrigible @sandboxed @auditable
being SelfModifyingAgent
  telos: "maintain system stability"
    bounded_by: "score >= 0.8"
    modifiable_by: "human_operator"
  end
  matter: score: Float, cycles: Int  end
  function:
    fn step :: Signal -> Action
      todo: "dispatch by current strategy"
    end
  end
  rewire:
    trigger: score_static > 0.4
    candidates: [pid, mpc, rl]
    selection: fitness_guided
  end
  telomere:
    limit: 200
    on_exhaustion: division
  end
end
```

### Loading `.loom` beings into the BIOISO colony

```sh
# Dry run — see what would be loaded
loom runtime load examples/my_beings.loom --dry-run

# Load into colony database
loom runtime load examples/my_beings.loom --db bioiso.db

# Run 50 ticks
loom runtime run --db bioiso.db --ticks 50

# Check events
loom runtime scan --db bioiso.db
```

---

## Appendix B: The 10 BIOISO Domains

The BIOISO runtime ships with 10 pre-seeded domains. Each satisfies three criteria: (1) fitness landscape is coevolutionary or structurally non-stationary, (2) `StructuralRewire` is load-bearing, (3) currently unsolved or inadequately addressed at T1–T4.

| Domain | Tier | Problem class |
|--------|------|--------------|
| `amr_coevolution` | T5 | Antimicrobial resistance coevolution |
| `flash_crash` | T5 | HFT market microstructure gaming |
| `adaptive_jit` | T5 | JIT compiler IR pass ordering |
| `protein_drug_resistance` | T5 | Cancer/HIV target mutation |
| `ics_zero_day` | T5 | ICS zero-day attack detection |
| `quantum_error_mitigation` | T5 | NISQ-era noise model drift |
| `climate_intervention` | T5 | Earth system intervention sequencing |
| `biosphere` | T4 | Biodiversity metric optimization |
| `ocean_circulation` | T3 | Ocean circulation homeostasis |
| `aegis_delta_neutral` | T5 | DeFi delta-neutral strategy evolution |

---

*Manual generated from loom v2026-04. For the formal language spec see `docs/language-spec.md`. For the BIOISO theoretical model see `docs/publish/bioiso-paper.md`.*
