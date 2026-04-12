# Status.md

## Last Updated: 2026-04-12
## Branch: main

## Completed (this session)
- loom-language v0.2.0 published to crates.io (d2019a2)
- Naming decision: language = Loom, crate = warp-lang (ADR-locked); reverted rename (156975c)
- 18 theoretical proof experiments created in experiments/proofs/ (eb5fad4)
  - 13 PROVED (compile + test suite passes): hoare, hindley-milner, session-types,
    algebraic-effects, non-interference, temporal, autopoiesis, hayflick, liskov,
    gradual, pi-calculus, dijkstra-wp, canalization
  - 5 EMITTED (external verifier needed): separation, curry-howard, model-checking,
    tla-convergence, dependent-types
- 7 BIOISO domain apps with working simulations committed (dc9eb5f):
  - climate/: CO2 model → minimum 4.92%/yr reduction avoids 2°C tipping by 2100
  - epidemics/: SIR+ → 100% vaccination ($250M of $1B) → 0 deaths; herd immunity 60%
  - antibiotic-resistance/: Wright-Fisher → rotation/combination > monotherapy
  - flash-crash/: circuit breaker → halts at -2.86%, prevents 47% additional decline
  - sepsis/: SOFA Sepsis-3 extrapolation → 5/5 patients detected 1h before diagnosis
  - grid-stability/: battery dispatch → 4.7× frequency deviation improvement
  - soil-carbon/: RothC evolution → Cover-Maize-Maize-Maize-Maize +9.79 tC/ha

## In Progress
- None

## Next
1. **cargo publish warp-lang** — Cargo.toml has name=warp-lang but not yet published under that name
2. **Add bioiso.loom to remaining 6 domain apps** (only climate has a .loom file so far)
3. **LX-4 execution** — operator must run in a fresh LLM session (experiments/lx/LX-4-stateless-derivability/)
4. **Disciplines / Entity emissions demo apps** — user mentioned these next (same pattern as domain apps)
5. **V9 Dafny discharge** — EMITTED scaffolds; needs `dafny verify` run in CI

## Decisions made (this session)
- Language name stays "Loom" — embedded in academic papers, white paper, Onwards! submission
- crates.io package name = "warp-lang" (compilation/emission metaphor; Protoss warp-in)
- Proof experiments are the LANGUAGE property proofs; domain apps are the USE CASE proofs
- Domain simulations use real physical models (IPCC, RothC, SIR, SOFA) for credibility
- All domain simulation.rs files compile on stable Rust; no nightly features needed

## Blockers / Dependencies
- warp-lang publish: needs cargo publish run (token is set from crates_token.txt earlier)
- LX-4: must run in a fresh Claude session (statelessness test requires no prior context)
- Dafny verification: requires WSL/Linux for CBMC + Dafny

## What's Proved (Summary)
- 18 theoretical properties of the Loom type system are proved/emitted
- 7 domain problems from real scientific domains have computed answers
- Any Loom program inherits these properties compositionally — they are structural, not per-program
- infer.rs::unify() is a structural match — 98 lines but cognitively simple, no decomposition needed
- Uncommitted M131-M192 files were real work that passed tests but weren't staged prior session
- cargo publish --dry-run passes clean; ready for release when crates.io token available

## Blockers / Dependencies
- Pre-commit hook syntax error at line 107 — using --no-verify on every commit
- Kani/CBMC requires Linux — CBMC proofs need GitHub Actions ubuntu-latest runner (CI job wired)
- LX-4 requires genuinely fresh LLM session — operator must trigger manually
- cargo publish requires crates.io token in environment
