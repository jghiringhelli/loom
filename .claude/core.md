# loom — Core

> Always loaded. Contains only what is true across all domains.
> Hard limit: 80 lines.

## Domain Identity
Loom — an AI-native functional language that transpiles to Rust, TypeScript, WASM, OpenAPI 3.0, and JSON Schema from a single source file. Designed as a Generative Specification (GS) mold: every construct must be derivable by a stateless reader with no prior context.

## Tags
[UNIVERSAL] [CLI] [LIBRARY] [COMPILER] [AI-NATIVE]

## Current State
- **339 tests passing** across 28 test suites
- **23 milestones complete** (M1–M23)
- All semantic checkers active in pipeline

## Two-Ladder BIOISO Architecture
Both ladders converge at T5 (BIOISO colony):

**Fitness Ladder** (parameter/structure optimization):
- T1 Polycephalum — deterministic rules → ParameterAdjust
- T2 SA heuristics — Boltzmann annealing + Ganglion (Haiku) fallback
- T3 SARSA hyper-heuristic — weight table + MammalBrain (Sonnet) fallback
- T4 GP-UCB — Bayesian surrogate model (no LLM)
- T5 BIOISO — MeiosisEngine genome recombination → GitHub GS pipeline

**Forge Ladder** (code-level evolution):
- T1 AI writes code — LLM generates Loom/Rust source
- T2 Compile/test harness — `cargo test` gate
- T3 CI/CD deploy — Railway pipeline
- T4 Monitoring/bug-fix — signals + drift feedback loop
- T5 BIOISO — `CodePatch` proposals → GS pipeline (git apply → test → canary)

**T5 Synthesis trigger**: fires after `T5_STAGNATION_THRESHOLD` ticks (default 20)
with no accepted proposals. Semantic novelty guard (Jaccard, threshold 0.65) prevents
redundant exploration. `CodePatch` is the T5-exclusive mutation type.

## Emission Targets
`compile()` → Rust | `compile_typescript()` → TS | `compile_wasm()` → WASM
`compile_openapi()` → OpenAPI 3.0 | `compile_json_schema()` → JSON Schema

## Key Language Constructs
- `fn name @annotation :: A -> B -> Effect<[IO, DB], C]` — curried function with effects
- `require: expr` / `ensure: expr` — Hoare-style contracts → `debug_assert!`
- `type T = field: Type @pii @gdpr end` — product type with privacy labels
- `enum E = | A | B of T end` — sum type
- `type E = T where predicate end` — refined type
- `Float<usd>` — unit-parameterised numeric (arithmetic checked)
- `lifecycle T :: S1 -> S2 -> S3` — typestate protocol
- `flow secret :: TypeA, TypeB` — information flow label
- `invariant name :: condition` — module-level invariant
- `test name :: expr` — inline test block → `#[test]`
- `being: ... end` / `ecosystem: ... end` — biological computation blocks (M41–M43)
- `matter:` / `form:` / `function:` / `telos:` — Aristotle's four causes inside `being:`
- `regulate Name ... end` — homeostatic bounds enforcement inside `being:`
- `evolve ... end` — directed search toward telos (gradient_descent, stochastic_gradient, simulated_annealing, derivative_free, mcmc)
- `signal Name from A to B` — session-typed channel inside `ecosystem:`
- `epigenetic: trigger: ... end` — Waddington behavioral modulation without genome change (M45)
- `morphogen: gradient: ... end` — Turing reaction-diffusion spatial differentiation (M46)
- `telomere: limit: N on_exhaustion: ... end` — Hayflick finite replication limit (M47)
- `crispr: target: ... end` — Doudna targeted self-modification (M48)
- `quorum: threshold: N ... end` — Bassler population-threshold coordination (M49)
- `plasticity: learning_rate: ... end` — Hebb synaptic weight adjustment (M50)
- `autopoietic: true` — Maturana/Varela operational closure; requires `@mortal @corrigible @sandboxed @auditable` (M51)
- `@mortal` — requires `telomere:` block; SafetyChecker compile error if missing (M55)
- `@corrigible` — requires `telos.modifiable_by` field; SafetyChecker compile error if missing (M55)
- `@sandboxed` — autopoietic effects must stay within `matter:` and `ecosystem:` (M55)
- `@auditable` — all structural mutations and meiosis events are logged (M55)
- `@bounded_telos` — telos must not contain open-ended utility terms; requires `bounded_by:` (M55)
- `compile_simulation()` — Mesa ABM Python emitter for autopoietic beings (M52)
- `compile_neuroml()` — NeuroML 2 XML emitter for neural structure (M53)
- `describe: "..."` / `@key("value")` — GS self-describing annotations
- `interface I ... end` / `implements I` — structural interface conformance
- `import ModuleName` — cross-module dependency
- `Effect<[IO@irreversible], T>` — effect with consequence tier

## Semantic Checkers (all run in compile pipeline)
1. Type checker — symbol resolution, type compatibility
2. Exhaustiveness — match completeness
3. Effect checker — transitive propagation, consequence tiers
4. Interface conformance — implements vs interface signature
5. Units checker — Float<unit> arithmetic consistency
6. Privacy checker — @pci requires @encrypt-at-rest + @never-log
7. Algebraic checker — @exactly-once/@idempotent mutual exclusion
8. Typestate checker — lifecycle transition validity
9. Info-flow checker — secret → public without declassification
10. Telos checker (TeleosChecker) — being:/ecosystem: without telos: is a compile error; regulate: requires bounds; evolve: requires convergence constraint
11. Safety checker (SafetyChecker) — autopoietic beings without @mortal @corrigible @sandboxed are compile errors (M55)

## Annotation Syntax
Annotations come AFTER `fn name`, before `::`:
```
fn process_payment @exactly-once @trace("pay.create") :: A -> B
```
Module-level annotations come before any items:
```
module Foo
@author("team") @version(2)
```
Field annotations come after field type:
```
type User = email: String @pii @gdpr end
```

## File Layout
```
src/
  ast.rs            — all AST node types
  lexer/mod.rs      — logos tokenizer
  parser/mod.rs     — recursive-descent LL(2)
  checker/          — all semantic checkers
  codegen/          — rust.rs, typescript.rs, wasm.rs, schema.rs, openapi.rs
  lib.rs            — pipeline entry points
  cli.rs            — CLI
tests/              — 27 test suites, one per feature area
corpus/             — real-world example .loom files
docs/
  language-spec.md  — canonical language reference
  lineage.md        — intellectual history (Aristotle → Loom)
  lifecycle.md      — full software lifecycle spec
  publish/          — white-paper.md, article.md
```

## Layer Map
```
[CLI] → [lib.rs pipeline] → [lexer → parser → checkers → codegen]
All checkers are stateless: check(&Module) -> Result<(), Vec<LoomError>>
```

## Invariants
- Every new AST field requires updating ALL Module struct literals in codegen tests
- Annotations before `fn` keyword → accumulated as pending_annotations on Parser
- Token keywords must appear before Token::Ident in logos enum
- All commits use --no-verify (pre-commit hook has syntax error at line 107)
- PATH must include `$HOME\.cargo\bin` before any cargo commands on this machine
- **telos: is REQUIRED** — a `being:` or `ecosystem:` without `telos:` is a checker error, not a warning
- **SafetyChecker gate** — `autopoietic: true` without `@mortal @corrigible @sandboxed` is a compile error (M55); runs after TeleosChecker
- `compile_simulation()` emits Mesa ABM Python; `compile_neuroml()` emits NeuroML 2 XML