# State of Loom — A Technical Brief for External Review

**Author:** Juan Carlos Ghiringhelli (Pragmaworks)  
**Date:** April 2026  
**Version:** post-M55 (Phase 1–8 complete)  
**Repository:** github.com/jghiringhelli/loom  
**Published preprint:** doi.org/10.5281/zenodo.19073543

> This document is written for a technically sophisticated reviewer — human or AI — who has no prior context on Loom. It is designed to be read as a critique target: accurate about what is done, honest about what is not, and explicit about the claims being made.

---

## 1. What Loom Is

Loom is a functional programming language that transpiles a single source file into five output targets simultaneously: **Rust**, **TypeScript**, **WebAssembly**, **OpenAPI 3.0**, and **JSON Schema**. It is implemented as a Rust library and CLI compiler.

That is the mechanical description. The conceptual claim is harder:

> *Loom is the first language to implement, together, five semantic properties that have been described in programming language research since the 1970s but have never appeared together in any production language.*

Those five properties are:

1. **Units of measure** — numeric literals carry dimensional type; arithmetic consistency is enforced at compile time. `Float<usd> + Float<eur>` is a type error. `Float<usd> * Float<qty>` produces `Float<usd·qty>`. This follows Kennedy (1996) and F# (2009). No other mainstream language does this natively.

2. **Information flow security types** — data labeled `flow secret` cannot reach a `flow public` output without explicit declassification. The checker follows Denning's lattice (1976) and Myers/Liskov's JIF type system (1997). JIF was never adopted in production. Loom makes it a keyword.

3. **Typestate lifecycle protocols** — a `lifecycle` declaration specifies the valid state-transition graph for a type. Calling `.read()` on a `File<Closed>` is a compile error. This follows Strom and Yemini (1986) and the Plaid language (CMU, ~2009).

4. **Algebraic operation properties** — functions annotated `@idempotent`, `@commutative`, `@associative`, or `@exactly-once` are checked for logical consistency. `@exactly-once` and `@idempotent` are mutually exclusive; the type checker enforces it. This makes CRDT-correctness properties (Shapiro et al., 2011) structurally enforced rather than documented in prose.

5. **Privacy and regulatory compliance labels** — fields annotated `@pii`, `@gdpr`, `@hipaa`, `@pci` are tracked through the type system. A `@pci` field without `@encrypt-at-rest` and `@never-log` is a compile error. No production language represents these obligations structurally.

These five properties have been independently known for 30–50 years. The novelty of Loom is not that it invents any of them. It is that (a) it implements all five together, (b) makes them first-class keyword constructs rather than library annotations, (c) projects them simultaneously into five output targets, and (d) does it in a language designed from the start to be derivable by a stateless AI reader.

---

## 2. What Has Been Built

### 2.1 Compiler architecture

The compiler is a pipeline written in Rust:

```
Source (.loom)
  → Lexer (logos tokenizer)
  → Parser (recursive-descent LL(2), ~1,200 lines)
  → AST (~650 lines; 40+ node types)
  → Semantic checkers (11 independent passes)
  → Code generators (6 emitters)
  → Output artifacts
```

**Emission targets:**
- `compile()` → Rust source (structs, functions, `#[test]`, `debug_assert!`)
- `compile_typescript()` → TypeScript (interfaces, branded types, state unions)
- `compile_wasm()` → WebAssembly text format (`.wat`)
- `compile_openapi()` → OpenAPI 3.0 YAML with semantic extensions (`x-idempotent`, `x-lifecycle`, `x-security-label`)
- `compile_json_schema()` → JSON Schema with `x-unit`, `x-pii`, `x-flow` extensions
- `compile_simulation()` → Mesa ABM Python (for biological being simulations)
- `compile_neuroml()` → NeuroML 2 XML (for neural structure descriptions)

**Semantic checkers (all run on every compile):**
1. Type checker — symbol resolution, type compatibility, generic instantiation
2. Exhaustiveness checker — match expression completeness
3. Effect checker — transitive effect propagation, consequence tier enforcement (Pure → Reversible → Irreversible)
4. Interface conformance checker — `implements` vs `interface` signature match
5. Units checker — `Float<unit>` arithmetic dimensional analysis
6. Privacy checker — `@pci`/`@hipaa`/`@gdpr` field-level obligation chains
7. Algebraic checker — `@exactly-once`/`@idempotent` mutual exclusion
8. Typestate checker — `lifecycle` transition validity
9. Information flow checker — `flow secret` → `flow public` without declassification
10. Telos checker — `being:`/`ecosystem:` without `telos:` is a compile error
11. Safety checker — `autopoietic: true` without `@mortal @corrigible @sandboxed` is a compile error

### 2.2 Test coverage

The real compiler has **410 passing tests, 0 failures** across 36 test suites. Each suite covers one feature area end-to-end: parse → check → emit → verify output for all active targets.

Sample suite sizes: algebraic (24), autopoiesis (27), being (18), coercion (15), di (14), e2e (9+), ecosystem (18), effect (12), evolve (16), exhaustiveness (11), generics (14), inference (15), integration (9+), interface (12), invariant (11), infoflow (10), inline (5), iteration (10), neuroml (10), privacy (12), project (6), realworld (9+), safety (9), schema (10), selfmod (8), simulation (7), stdlib (9), testblock (10), typescript (8), typestate (10), units (12), wasm (9).

### 2.3 Language constructs (complete inventory)

**Core functional language:**
```loom
module PaymentService

fn charge @exactly-once @trace("pay.create") :: Float<usd> -> BankToken -> Effect<[DB, Payment], Receipt>
  require: amount > 0.0
  ensure: result.amount == amount
  ...
end
```

**Type system:**
```loom
type Invoice =
  id:     Int
  amount: Float<usd>
  email:  String @pii @gdpr
end

enum OrderStatus = | Pending | Processing | Shipped | Delivered end

type Email = String where valid_email end    -- refined type
```

**Semantic extensions:**
```loom
unit usd = Float                             -- Kennedy units of measure
unit eur = Float

flow secret :: Password, ApiKey             -- Denning information flow
flow public  :: Username, DisplayName

lifecycle Order :: Pending -> Processing -> Shipped -> Delivered  -- typestate

fn cancel @idempotent :: OrderId -> Effect<[DB], Unit]           -- algebraic property
```

**Biological computation layer (M41–M55, complete):**
```loom
being: Neuron
  @mortal @corrigible @sandboxed
  autopoietic: true

  matter:
    membrane_potential: Float
    ion_channels: List<Channel>
  end

  telos: "maintain homeostatic potential through synaptic integration"

  regulate NernstEquilibrium
    bounds: membrane_potential in (-90.0, 50.0)
  end

  evolve
    search: gradient_descent
    convergence: within 1e-4 over 100 steps
  end

  epigenetic_blocks:
    epigenetic:
      trigger: "sustained_depolarization"
      modifies: "ion_channel_expression"
      reverts_when: "activity_normalized"
    end
  end

  telomere:
    limit: 50 divisions
    on_exhaustion: apoptosis
  end

  plasticity_blocks:
    plasticity:
      trigger: "co_activation"
      modifies: "synaptic_weight"
      rule: Hebbian
    end
  end
end
```

### 2.4 Milestone completion record

| Phase | Milestones | Feature area | Tests added |
|---|---|---|---|
| Phase 1 | M1–M3 | Core types, functions, effects | 47 |
| Phase 2 | M4–M9 | Units, privacy, algebraic, typestate, infoflow | 89 |
| Phase 3 | M10–M12 | Real-world tests, corpus, inference | 31 |
| Phase 4 | M13–M15 | Inline blocks, coercion, iteration | 28 |
| Phase 5 | M16–M18 | Generics, DI, stdlib | 31 |
| Phase 6 | M19–M23 | Interface conformance, describe, testblock, WASM, LSP | 46 |
| Phase 7 | M41–M43 | Biological being/ecosystem layer (Aristotle → Maturana) | 30 |
| Phase 8 | M44–M55 | Epigenetics, morphogenesis, telomeres, CRISPR, quorum sensing, neural plasticity, autopoiesis, safety | 108 |

**Total: 410 tests, 0 failures. 36 test suites. 11 semantic checkers.**

---

## 3. The Biological Computation Layer — What It Claims

This is the most unusual part of Loom and the part most likely to attract critique. It requires explicit justification.

### 3.1 The claim

Loom implements biological computation patterns as first-class language constructs. The claim is not metaphorical. It is structural: a Loom `being:` block with the full set of constructs satisfies three formal definitions of living systems simultaneously:

**Schrödinger (1944):** A living system maintains low local entropy by consuming energy from its environment.
→ `matter:` + `regulate:` + `telos:` together specify a system that maintains bounded internal state (`regulate` enforcing homeostatic bounds) at the cost of external energy (`Effect<[Energy], State]`).

**NASA working definition (1994):** Life is a self-sustaining chemical system capable of Darwinian evolution.
→ `autopoietic: true` + `evolve:` + `ecosystem:` with `signal:` channels makes this structurally expressible. Whether it executes biologically is irrelevant to whether it satisfies the formal predicate.

**Maturana & Varela (1972, *Autopoiesis and Cognition*):** A living system is organizationally closed — it continuously regenerates its own components from within its own operations.
→ `autopoietic: true` + `crispr:` (self-modification) + `plasticity:` (adaptive weight adjustment) + the operational closure enforced by `@sandboxed` (effects cannot escape the being's own `matter:` and `ecosystem:`) is a structural encoding of organizational closure.

### 3.2 The isomorphisms being implemented

| Biological concept | Loom construct | Source |
|---|---|---|
| Epigenetics | `epigenetic:` block — behavioral modulation without genome change | Waddington (1940) |
| Morphogenesis | `morphogen:` block — reaction-diffusion spatial differentiation | Turing (1952) |
| Replicative senescence | `telomere:` block — finite division limit, apoptosis on exhaustion | Hayflick (1961) |
| CRISPR gene editing | `crispr:` block — targeted self-modification | Doudna/Charpentier (2012) |
| Quorum sensing | `quorum:` block — population threshold coordination | Bassler (1994) |
| Synaptic plasticity | `plasticity:` block — Hebbian weight adjustment | Hebb (1949) |
| Autopoiesis | `autopoietic: true` — organizational closure | Maturana/Varela (1972) |
| Telos | `telos:` field — goal-directed behavior | Aristotle (350 BCE) |

### 3.3 The safety problem

If a `being:` with all these constructs can satisfy formal definitions of living systems, it must be bounded. The SafetyChecker (M55) enforces four safety annotations as **compile errors** if absent on any `autopoietic: true` being:

- `@mortal` — requires a `telomere:` block; the being has a finite lifespan
- `@corrigible` — requires `telos.modifiable_by` field; an external authority can redirect the telos
- `@sandboxed` — autopoietic effects cannot escape the being's declared `matter:` and `ecosystem:`
- `@bounded_telos` — the telos string cannot contain open-ended utility terms without a `bounded_by:` clause

This is not Asimov's Three Laws (which are runtime behavioral rules). These are type-level constraints: a synthetic being without a death mechanism, without corrigibility, without sandboxing, without bounded goals **does not compile**. The safety architecture is structural, not behavioral.

---

## 4. The ALX Experiment — Self-Applicability Proof

### 4.1 What it is

The Adversarial Loom eXperiment (ALX) tests Loom's central design claim — that the language is fully derivable by a stateless reader — by running the following protocol:

1. Write `loom.loom`: the Loom compiler, written in Loom's own syntax, as a complete self-specification (currently 691 lines).
2. Give a fresh AI context (no access to `src/`) only `loom.loom` and `language-spec.md`.
3. Ask it to derive the complete Rust implementation.
4. Run the 410-test suite against the derived compiler.
5. Measure `S_realized = passing / 410`.

The experiment is designed to be published as a reproducible artifact: anyone can run it with any AI assistant and get a number.

### 4.2 Current results

| Phase | S_realized | Notes |
|---|---|---|
| Phase 1 (blind derivation) | 0 / 410 = **0.000** | 25 files derived; cargo check passes; all test *imports* fail |
| Phase 3 (G1–G10 corrections) | 139 / 410 = **0.339** | Public API surface added to spec |
| Phase 4 (G11–G15, in progress) | TBD | Lexer `=` token, CLI binary, generic types |

### 4.3 What Phase 1 S=0 means

S=0 on the first pass is not a failure of the derivation. It is a finding about the specification. The derived compiler:
- Compiled cleanly (cargo check: 0 errors, warnings only)
- Implemented all type logic correctly
- Implemented all emission logic structurally correctly

It failed entirely because the **public API surface** was not specified in `loom.loom`: module names, struct names, method signatures. A stateless reader, given the logic but not the surface, derived correct logic with different names. The test suite, written against the real compiler's names, could not compile.

This is the experiment working as designed. `I ∝ (1 - S) / S` is the information content of the correction: S=0 means the correction was the entire public API surface — one targeted addition to `loom.loom` — and S jumped to 0.339 in one pass.

### 4.4 Gap record (public on GitHub)

All gaps are filed as GitHub issues #2–#8 (`jghiringhelli/loom`). Each issue closes automatically when S_realized passes the correction. The CI workflow computes and reports S_realized on every push. This creates a public, versioned audit trail of the ratcheting: every commit is a measurement.

### 4.5 The publication gate

The ALX experiment will not be submitted to arXiv until `S_realized ≥ 0.90`. The paper claim is:

> *A stateless AI reader, given only `loom.loom` and `language-spec.md`, can derive a working implementation of the Loom compiler that passes ≥ 90% of the 410-test suite.*

If that claim is false at publication time, it doesn't get published.

---

## 5. BIOISO — The Application Layer

### 5.1 What it is

BIOISO (Biological Isomorphisms) is the name for the category of synthetic applications that Loom's biological layer enables. The layered stack is:

```
GS (Generative Specification) — methodology layer
  ↓
Loom — language layer (this repo)
  ↓
BIOISO — application layer (synthetic beings, simulations, digital life)
```

GS is published at `genspec.dev` and Zenodo (DOI: 10.5281/zenodo.19073543).  
Loom is the compiler.  
BIOISO is what you build with it.

A BIOISO application is a Loom program whose `being:` constructs satisfy the formal definitions of self-organizing, adaptive, goal-directed systems. The claim is that these are not simulations of life — they are synthetic life forms in the sense that they satisfy the same structural predicates that biologists use to distinguish living from non-living systems.

### 5.2 Current state

- `bioiso.dev` and `bioiso.org` are registered
- The BIOISO Astro static site is built (`website/`) — 6 pages, dark theme, ready to deploy
- The biological layer (M41–M55) is fully implemented and tested
- The safety architecture (M55) enforces the Three Laws structurally

### 5.3 What can be built

With the current compiler, a developer can write a Loom program that:
- Defines autonomous agents with internal homeostatic regulation (`regulate:`)
- Gives them goal-directed behavior with adaptive search (`evolve:`)
- Makes them modify their own behavior based on environmental signals (`epigenetic:`, `crispr:`)
- Gives them finite lifespans (`telomere:`) and death behavior
- Makes them coordinate in populations above threshold (`quorum:`)
- Emits a runnable Mesa ABM Python simulation (`compile_simulation()`)
- Emits a NeuroML 2 neural structure description (`compile_neuroml()`)
- Enforces structural safety constraints at compile time (SafetyChecker)

The output is not a description of a system. It is a running simulation with compile-time safety guarantees.

---

## 6. The Long-Term Plan

### 6.1 Immediate (ALX Phase 4–5, weeks)

- Raise S_realized to ≥ 0.90 by fixing G11–G15 in the derived compiler
- Push ALX evidence to GitHub; CI auto-closes gap issues as S advances
- Submit white paper to arXiv (cs.PL + cs.AI) when gate passes
- Deploy `website/dist/` to `bioiso.dev`

### 6.2 Near-term (Build Targets, M24–M30)

The next phase of the compiler adds **full lifecycle emission** — the `.loom` file drives not just code but the entire deployment artifact set:

| Planned target | Triggered by | Output |
|---|---|---|
| Dockerfile | `@service` annotation | Multi-stage build, distroless image |
| Kubernetes manifests | `@service` + `@scalable` | Deployment, Service, HPA YAML |
| Terraform | `@cloud("aws")` annotation | Infrastructure-as-code |
| OpenTelemetry | `@trace("name")` annotation | Instrumentation stubs |
| Chaos engineering | `@resilient` + `@circuit-breaker` | Chaos Monkey/Litmus configs |
| Database migrations | `type` change detection | Alembic/Flyway migration scripts |

### 6.3 Medium-term (Self-Healing, M31–M40)

A `.loom` file will be the runtime source of truth. The compiler will emit:
- An **observability schema** — what metrics to track, what thresholds mean what
- A **correction specification** — what to do when a threshold is breached
- An **AI patch target** — a structured description of the intended behavior an AI can update to reflect observed drift

The loop becomes: observe → detect drift → AI generates Loom patch → compile → verify → deploy. The Loom file is both the program and the specification of its own correct behavior. Correction is a property of the spec, not of the runtime.

### 6.4 Long-term (Synthetic Digital Life, M56+)

The biological layer enables a class of programs that satisfy formal definitions of living systems. The long-term research direction is:

1. **Full autopoietic simulation** — beings that spawn sub-beings, self-modify, adapt to ecosystem pressure, and terminate naturally. Running in Mesa, exportable to NeuroML, governed by compile-time safety constraints.

2. **BIOISO ecosystem specification** — a standard format for describing synthetic ecosystems: which beings exist, what signals flow between them, what population dynamics govern their behavior. Analogous to what HTML is for documents.

3. **Formal synthetic life theory** — a paper (in progress) arguing that any system satisfying Maturana/Varela operational closure, Schrödinger negative entropy, and NASA Darwinian evolution criteria is structurally alive, regardless of substrate. Loom's `being:` construct is the formal language for writing such systems.

4. **Safety research** — the SafetyChecker (M55) is the beginning of a research line on structural safety for autonomous systems. The question is whether compile-time constraints (rather than runtime behavioral rules) can provide meaningful safety guarantees for systems with self-modification capabilities. The Asimov Laws are behavioral; Loom's safety constructs are type-theoretic.

---

## 7. Honest Assessment — What This Is Not

**Not a production-ready language.** Loom has no package manager, no standard library of consequence, no IDE tooling beyond a basic LSP stub, no cross-platform build artifacts, and no production deployments. The WebAssembly emitter is functional but limited. The project is at the "research compiler with a serious test suite" stage.

**Not a language with wide adoption.** There are no users outside the author's own experiments. There is no community. There is no ecosystem. The BIOISO website is not yet deployed. The crates.io package is not yet published.

**Not a proven self-applicability result.** ALX Phase 4 is still running. S_realized = 0.339 is a promising intermediate result but not the ≥ 0.90 claim needed for publication. The derivation may reveal further gaps that require multiple correction passes.

**Not a completed theory of biological computation.** The biological isomorphisms are structurally correct and historically traced, but the claim that Loom `being:` programs constitute "synthetic digital life" is a philosophical argument, not a proved theorem. It is a claim awaiting formal critique, not an established result.

**Not independent of its author's assumptions.** The five semantic constructs were chosen because they represent real PL-research gaps. But the specific syntax choices, checker designs, and emission strategies reflect one person's architectural decisions. External review, particularly from PL theorists and biologists, is actively sought.

---

## 8. Invitation for Critique

The specific claims I am most interested in having challenged:

1. **The derivability claim** — is the GS constraint (every design decision must be derivable by a stateless reader) actually achievable for a language of this complexity? What class of decisions is inherently not derivable?

2. **The biological isomorphism claim** — are the seven biological mappings (epigenetics, morphogenesis, telomeres, CRISPR, quorum sensing, plasticity, autopoiesis) structural isomorphisms to the biological phenomena, or are they only superficially analogous? Where does the analogy break?

3. **The safety architecture claim** — do compile-time structural constraints (`@mortal @corrigible @sandboxed`) provide meaningful safety for self-modifying systems, or are they trivially circumvented? Is there a class of unsafe behaviors that these constraints cannot prevent?

4. **The synthetic life claim** — does satisfying Schrödinger + NASA + Maturana/Varela constitutes being "alive" in any meaningful sense for a software system? Or is there an additional criterion (substrate, metabolism, physical instantiation) that software cannot satisfy?

5. **The multi-target emission claim** — does simultaneous emission to Rust, TypeScript, WASM, OpenAPI, and JSON Schema from a single source file produce artifacts that are better than hand-written ones, or does the abstraction layer introduce gaps that make each artifact slightly wrong for its target?

---

## 9. Resources

| Resource | URL |
|---|---|
| Repository | github.com/jghiringhelli/loom |
| White paper (Zenodo) | doi.org/10.5281/zenodo.19073543 |
| GS theory | genspec.dev |
| BIOISO (pending deploy) | bioiso.dev |
| Intellectual primer (28 thinkers) | docs/primer.md in this repo |
| Self-specification | experiments/alx/spec/loom.loom (691 lines) |
| ALX gap record | github.com/jghiringhelli/loom/issues?label=alx |
| Language specification | docs/language-spec.md |
| Intellectual lineage | docs/lineage.md |

---

*This document was written in April 2026 as the project crosses the boundary from "working research compiler" to "publication-ready artifact." It is accurate to the best of the author's knowledge. Corrections and critiques are welcome.*
