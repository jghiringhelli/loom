# Landscape & prior-art assessment (2026-11-12)

> Two independent web-research passes (language/spec side, BIOISO side). Search tooling was limited:
> Firecrawl returned 401, so evidence is WebSearch result summaries, not full pages. Items marked
> *(unverified)* were not confirmed. Re-check every citation before using it in a paper.

## Verdict in one paragraph

Almost every individual idea in Loom has prior art, much of it better funded and more mature.
What is **not** found elsewhere is the *combination* (one typed language carrying units + information
flow + typestate + algebraic-property checks + privacy labels + multi-target emission) and a small
set of specific mechanisms (consequence-tiered effects; compile-time safety annotations for
self-modifying beings). The BIOISO optimisation evidence is far below the standard of the
LLM-evolution and BBOB literature. The defensible niche is **declared, statically checked safety
and governance properties for AI-patched, self-modifying systems**, not "a better optimiser" and
not "a new general-purpose language".

## Language / spec side

| Loom property | Closest prior art | Loom adds? |
|---|---|---|
| `require:`/`ensure:`, refined types | Dafny, Verus, Liquid Haskell, SPARK, Creusot, Kani (SMT, benchmarks, industry use) | No, unless contracts are actually *proved*. Today they become `debug_assert!`; Z3 path exists but is optional |
| Units of measure | F#, Rust `uom` | No (solved problem) |
| Information flow | Jif, Paragon, FlowCaml (research, never adopted) | No; also a warning about demand |
| Typestate | Plaid, Mungo, Rust `typestate` crate | No (integration only) |
| `@idempotent` / `@exactly-once` checking | Restate / Temporal handle it at *runtime* (journals, keys) | Possibly unoccupied — but exactly-once against external side effects is not provable by a compiler; check can only be a local model |
| Privacy / regulatory labels | TyPAOL (GDPR type system, 2026), Owe et al., Fides, Privado | Partly. HIPAA/PCI as compile-time labels not found; labels cannot capture legal basis/purpose |
| Multi-target from one file | TypeSpec (GA), Smithy (AWS), Dafny multi-backend | OpenAPI/JSON Schema fully covered. Rust+TS+WASM program targets from the same file is less common — **but M1 shows Loom's Rust output compiles for only 13/42 programs** |
| AI derives code from spec | Vericoding benchmark (Dafny 82 %, Verus 44 %, Lean 27 %); Spec Kit, Kiro, Tessl ($125M) | Only if Loom publishes held-out pass rates; ALX is one self-referential run |
| Self-applicability | Anthropic's LLM-built C compiler (2026): test oracle, heavy scaffolding, criticised | ALX-R (blind re-derivation) is the missing control |

**Claims that can be falsified:** "first language with all five" (check TyPAOL/Jif/Fides before saying
"first"); "privacy labels are checked" (soundness argument needed, else it is a lint); "`@exactly-once`
is checked" (contradicted in general); "AI-derivable" (needs held-out specs and contamination controls).

## BIOISO side

| Claim | Closest prior art | BIOISO adds? |
|---|---|---|
| (a) mortal, goal-bearing, self-modifying beings with compile-time `@mortal/@corrigible/@sandboxed/@auditable` | Autonomic / Organic Computing (MAPE-K), Avida, Darwin Gödel Machine | **Only candidate novelty** — no type-level treatment of corrigibility found (search was shallow). Static guarantees do not survive runtime self-modification; corrigibility is undecidable in general (Nayebi 2025) |
| (b) T1–T5 ladder | STOP (self-taught optimiser), EoH/ReEvo, LLaMEA, hyper-heuristics, Bayesian optimisation | Each rung is standard; no evidence the ladder beats a tuned single optimiser or portfolio |
| (c) BBOB result | Adaptive Coordinate Descent (GECCO 2011), LLaMEA (full 24-fn BBOB, beats CMA-ES/DE) | Little. 4 of 24 fns, 200 evals (20×D), baseline = coordinate descent (low bar on an ellipsoid) |
| (d) "autopoietic ⇒ structurally alive" | Enactivism (Froese, Di Paolo): autopoiesis alone is insufficient | Contested; state as definitional stipulation or drop |
| (e) LLM applies mutations, CI gates | AlphaEvolve, OpenEvolve, ShinkaEvolve, DGM, SWE-agent, RepairAgent | Nothing technical beyond the PR/test/canary wrapper |

### What a reviewer from the optimisation community would ask for
Full 24-function BBOB at several dimensions and a budget ladder to ≥100×D; CMA-ES (and BIPOP), ACD and a
Bayesian optimiser as baselines; COCO / IOHprofiler ECDFs and fixed-target runtimes with
multiple-comparison correction; per-tier ablations; and an end-to-end run where **LLM-generated
patches**, not a hand-coded rotation, produce the gain.

## Where Loom still has value (recommended positioning)

1. **Governance layer for AI-patched systems**: untrusted LLM patches + declared, statically checked
   safety properties + audit log. Evaluate on *violations caught / rollbacks / auditability*, not on
   optimiser performance.
2. **One spec for contract + API + compliance obligations** (a narrower, honest version of the five
   properties): compile-time privacy/consequence labels carried into OpenAPI/JSON Schema/TS.
3. **A reproducible experiment on AI derivability** (ALX-R + LX-4 with held-out specs and a different
   model) — publishable whichever way it comes out.

## What to stop claiming
"First/only", "synthetic life", "mature", BIOISO T5 as a general optimiser improvement, and anything
about `@exactly-once` being *proved*.
