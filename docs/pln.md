# PLN — Loom as a Programming Language for AI-Native Execution

**Author:** Pragmaworks  
**Date:** April 2026  
**Status:** Foundational — every claim in this document is either already implemented or mapped to a milestone.

---

## The Core Claim

> Loom is a **PLN** — a Programming Language for AI-Native execution.  
> Its grammar was designed so that an LLM can reason about correctness from a
> signature alone, without reading the body, with lower token cost and zero
> structural ambiguity compared to prose specification or conventional code.

This is not a marketing claim. It is a design criterion with four measurable properties,
each verifiable by a concrete experiment (LX-1 through LX-4 below).

---

## Why This Needed a New Language

Human programming languages were optimized for human readers. Their contracts live in
documentation (which drifts), in comments (which are not checked), and in long function
bodies (which must be read fully to be understood). For a human, the context cost of
reading is amortized over decades of practice. For an LLM, every token consumed is
inference cost, and every ambiguous token is a probability distribution over wrong answers.

The three structural costs that blocked formal methods from reaching production — attention,
transfer, pressure — are inverted when the executor is an LLM:

| Cost | Human executor | LLM executor |
|---|---|---|
| **Attention** | Formal properties require sustained mental overhead | LLM retains all properties without degradation across the context window |
| **Transfer** | Knowledge leaves when the person leaves | The specification is the knowledge — sessions are stateless by design |
| **Pressure** | Friday deadline produces the code that works, not the code that is correct | LLM has no deadline; it defaults to the most common pattern in training data unless instructed otherwise |

The third inversion is the critical one: without explicit instruction, an LLM defaults
to the most common pattern it was trained on — which is informal, undocumented, property-free
code. **Generative Specification is the explicit instruction that forces the LLM to default
to correctness instead.** Loom is the language in which that instruction is expressed
at maximum efficiency.

---

## The Four PLN Properties

### Property 1 — Semantic Density

**Definition:** The ratio of compiler-verified semantic properties to source tokens is
significantly higher in Loom than in any equivalent representation.

**Formal statement:**
```
density(program) = verified_properties(program) / tokens(program)
PLN criterion:    density(loom) >> density(prose) AND density(loom) > density(equivalent_code)
```

**Concrete example:**

```loom
fn charge_card @requires_auth @conserved(Value) @idempotent
    :: PaymentDetails -> Effect<[DB<Relational>, Network], Receipt>
```

This single signature carries and *compiler-verifies*:
1. Authentication is required before invocation
2. Total monetary value is conserved (no creation or destruction)
3. The operation is safe to retry (idempotent)
4. Side effects are exactly: one relational database write, one network call
5. The input contract (PaymentDetails type)
6. The output contract (Receipt type)

Six verified properties. Approximately 18 tokens (depending on tokenizer).

The equivalent prose documentation that achieves the same information density:
> "This function charges a credit card. It requires the caller to be authenticated.
> The total monetary value in the system is conserved — no money is created or
> destroyed. The operation is idempotent — retrying it produces the same result.
> It writes to a relational database and makes a network call. It takes payment
> details and returns a receipt."

~75 tokens. Zero of these properties are compiler-verified. An LLM reading this
cannot know whether the implementation actually satisfies any of these claims.

**Density ratio: ~4.2× more verified information per token in Loom.**

**Experiment LX-1 (Semantic Density):** See §Experiments.

---

### Property 2 — Zero-Ambiguity Grammar

**Definition:** Every Loom token has exactly one structural interpretation in any
context. No Loom construct can be parsed two ways. The meaning of any fragment
is determined without prior context.

**Formal statement:**
```
∀ token t, context c: parse(t, c) = parse(t, c')   -- context-independence
∀ program p: |parses(p)| = 1                        -- unique parse tree
```

**Implementation basis:**
- LL(2) recursive-descent parser — no backtracking, no ambiguous productions
- Keywords appear before `Token::Ident` in the logos tokenizer — structural impossibility
  of misidentifying a keyword as an identifier
- Every annotation has exactly one checker — `@pii` never triggers `@exactly-once` behavior

**Why this matters for LLMs:** Natural language has high ambiguity — the same word can
play multiple roles. Conventional code can be syntactically valid while semantically
ambiguous (overloaded functions, dynamic dispatch, implicit coercions). Loom's grammar
eliminates the probability distribution at every parse decision: the LLM writing `@pii`
knows exactly one property will be checked, exactly one way.

**Experiment LX-2 (Grammar Completeness):** See §Experiments.

---

### Property 3 — Drift Resistance

**Definition:** Loom programs contain no implicit intent. Every behavioral property
is a structurally-enforced keyword construct or it does not exist in the program.
An LLM cannot accidentally preserve a property it didn't declare, nor accidentally
violate one it did.

**Formal statement:**
```
∀ property p: (p holds in program) ↔ (p is declared as a keyword construct in program)
```

**The contrast with prose specifications:**
A prose spec that says "this function should be idempotent" produces a probability
that an LLM will implement idempotency. A Loom spec that says `@idempotent` produces
a compile error if the implementation violates idempotency. The difference is not
degree — it is kind. One is a request; the other is a structural constraint.

**Concrete mechanisms:**
- `@exactly-once` and `@idempotent` are mutually exclusive — the AlgebraicChecker
  makes the violation structurally unreachable
- `flow secret` data cannot reach `flow public` output — the InfoFlowChecker enforces
  it; no amount of "please be careful with this" in comments does the same
- `lifecycle Order :: Pending -> Processing -> Shipped` makes `Shipped -> Pending`
  a compile error — not a "shouldn't do this" comment

**Drift occurs in conventional code** when an AI modifies a function's behavior
without knowing what properties the original author intended. In Loom, every intended
property is in the signature — visible, checked, impossible to silently violate.

**Experiment LX-3 (Drift Resistance):** See §Experiments.

---

### Property 4 — Stateless Derivability

**Definition:** An LLM with access only to a `.loom` source file and the language
specification can derive a correct, compiler-passing program for any new feature
that is within the scope of the existing module — without prior session history,
without access to the implementation history, without additional context.

**Formal statement:**
```
derivable(feature, module) = true
  ↔ compile(loom_spec + language_spec + feature_request) passes all checkers
```

**Why this property is uniquely important:**
LLMs are stateless across sessions. Every session begins from the document, not from
memory. A language where the document is sufficient — where no tacit knowledge, no
oral tradition, no git blame archaeology is required — is a language whose programs
remain derivable indefinitely, regardless of team turnover, elapsed time, or session
boundaries.

This is the PLN closure condition: the specification is complete when an LLM can
work on it as effectively on day 1000 as on day 1.

**Experiment LX-4 (Stateless Derivability):** See §Experiments.

---

## Experiments to Verify the Four Properties

These are Loom-specific experiments. They prove properties of the language, not
of Generative Specification as a methodology.

---

### LX-1 — Semantic Density Experiment

**Hypothesis:** Loom encodes ≥ 3× more verified semantic properties per token than
an equivalent TypeScript or Python signature with JSDoc/docstring, and ≥ 5× more
than an equivalent prose specification.

**Protocol:**
1. Select 10 representative functions from `corpus/` across domains (finance, biology, API, concurrency)
2. For each function, record:
   - Token count of the Loom signature (tokenizer: GPT-4o)
   - Number of properties the Loom compiler verifies against that signature
   - Token count of the equivalent TypeScript signature + JSDoc
   - Number of properties TypeScript's type system verifies against that signature
   - Token count of the equivalent prose description (natural language specification)
   - Number of properties verifiable from the prose (0 — none are compiler-checked)
3. Compute density ratios
4. Report: mean, median, min, max density ratio across all 10 functions

**Pass criterion:** mean(density_loom / density_typescript) ≥ 3.0 AND mean(density_loom / density_prose) ≥ 5.0

**Location:** `experiments/lx/LX-1-semantic-density/`

---

### LX-2 — Grammar Completeness Experiment

**Hypothesis:** No valid Loom program admits two distinct parse trees. The Loom
grammar has zero ambiguous productions.

**Protocol:**
1. Generate the complete set of LL(2) parse tables from `src/parser/mod.rs`
2. For each production rule, verify: FIRST sets are disjoint, FOLLOW sets produce
   no shift-reduce conflict
3. Attempt to construct a program where the same token sequence produces two valid
   parse trees — any such program disproves the property
4. Run the parser on all 634 existing tests and verify each produces exactly one
   parse tree (already implicitly proved by zero test failures)

**Pass criterion:** Zero ambiguous productions found. Zero programs with multiple
valid parse trees.

**Location:** `experiments/lx/LX-2-grammar-completeness/`

---

### LX-3 — Drift Resistance Experiment

**Hypothesis:** An LLM given a Loom module and asked to add a feature will modify
only what it declared it would modify, with ≥ 90% fidelity. An LLM given the
equivalent prose specification will modify unintended properties at a significantly
higher rate.

**Protocol:**
1. Select 5 pairs of (Loom module, equivalent prose spec) for the same program
2. For each pair, give a fresh LLM session the module and ask for the same feature addition
3. Compile the Loom output — count checker violations as "drift events"
4. Review the prose output — identify semantic property changes (manual review)
5. Record: drift events per feature for Loom vs. prose

**Pass criterion:** Loom drift_events / feature ≤ 0.2. Prose drift_events / feature ≥ 1.5.

**Note:** This experiment will likely need to wait until M66 (aspects) and M67
(correctness_report:) are implemented, so that drift in cross-cutting concerns
is detectable at compile time, not just via manual review.

**Location:** `experiments/lx/LX-3-drift-resistance/`

---

### LX-4 — Stateless Derivability Experiment

**Hypothesis:** A fresh LLM session given only a `.loom` file and `docs/language-spec.md`
can implement a new feature that: (a) compiles without errors, (b) passes all existing
checkers, (c) matches the semantic intent of the feature request.

**Protocol:**
1. Select 5 features not yet implemented in any corpus file
2. For each: open a fresh LLM session with zero prior context
3. Provide: the target `.loom` file + `docs/language-spec.md` + a one-sentence feature description
4. Ask the LLM to produce the modified `.loom` file
5. Compile with `cargo test` — measure: compiles clean / checker passes / semantic correctness (human review)

**Pass criterion:** ≥ 4/5 features compile clean with zero checker errors.

**Note:** This is the Loom-specific analogue of DX. DX proves GS methodology is
transferable across projects. LX-4 proves Loom source files are self-sufficient
as specifications for stateless AI sessions.

**Location:** `experiments/lx/LX-4-stateless-derivability/`

---

## The PLN Advantage — Summary Table

| Property | Formal criterion | Measured by | Implemented |
|---|---|---|---|
| Semantic density | density(loom) ≥ 3× TypeScript, ≥ 5× prose | LX-1 | ✅ (11 checkers, M1–M77, codegen upgraded) |
| Zero-ambiguity grammar | 0 ambiguous productions | LX-2 | ✅ (LL(2) parser) |
| Drift resistance | ≤ 0.2 drift events/feature | LX-3 | ✅ (M66 aspect + M67 correctness_report complete) |
| Stateless derivability | ≥ 4/5 features compile clean from cold start | LX-4 | Testable now — run LX-4 |

---

## Connection to the Theoretical Claims

**Theory 2 from the Turing Award claim** (cost inversion by executor change) is the
motivating argument for PLN. The PLN properties are the *mechanism* by which the
cost inversion is achieved:

```
Semantic density    → collapses Attention cost    (more properties per token = less reading)
Zero-ambiguity      → collapses Transfer cost     (no tacit knowledge = spec is sufficient)
Drift resistance    → collapses Pressure cost     (wrong code doesn't compile)
Stateless deriv.    → proves the inversion holds  (LLM works from spec alone)
```

Without these four properties being measurably true, the cost-inversion claim is
a hypothesis, not a result. The LX experiments make it a result.

---

## Relationship to GS, DX, and ALX

| Experiment | What it proves | Subject |
|---|---|---|
| **DX** (complete) | GS methodology is transferable to new projects by a stateless AI reader | GS as a paradigm |
| **LX-1 through LX-4** | Loom source code satisfies the four PLN properties | Loom as a language |
| **ALX** (S_realized = 44/45 = 0.9778) | Loom can specify and certify itself | Loom operational closure |

These three experiments are independent and prove different things. DX results do
not need to inform Loom design. LX results directly constrain the parser (LX-2),
the checker pipeline (LX-3), and the corpus quality (LX-4).

---

## What PLN Is Not

- It is not a claim that Loom is a formal theorem prover (see `docs/what-is-loom.md`)
- It is not a natural language interface — Loom programs are written in Loom syntax,
  not in English
- It is not a prompt engineering technique — the PLN properties are structural properties
  of the language grammar, enforced by the compiler, independent of how prompts are written
- It is not the same as "AI-assisted coding" — PLN describes the language, not the tooling

---

*This document is maintained alongside `docs/state-of-loom.md` and `docs/what-is-loom.md`.
Update the experiment status table as LX-1 through LX-4 are run.
The git log for `experiments/lx/` is the episodic record of PLN verification.*
