# Loom Roadmap

> Public roadmap. Last reviewed: 2026-11-12.
> Loom is a **research compiler with a serious test suite** — not yet a production language.
> This document says what is done, what is being proven, and what is deliberately not promised.
> Milestone-level specs (M1–M18 development prompts) live in [`docs/roadmap.md`](docs/roadmap.md).

## Where Loom is today

| Area | State | Evidence |
|---|---|---|
| Compiler (lex → parse → 11 semantic checkers → codegen) | Working | CI green on `main` |
| Emission targets: Rust, TypeScript, WASM, OpenAPI 3.0, JSON Schema | Working (WASM limited) | `tests/`, `corpus/` |
| Biological layer (`being:`, `telos:`, `regulate:`, `evolve:`, SafetyChecker) | Implemented | `tests/`, `experiments/` |
| BIOISO runtime (colony, T1–T5 ladders, meiosis) | Experimental | `experiments/bbob`, `experiments/aegis` |
| Self-applicability (ALX) | **Not proven** — last recorded S_realized = 0.339, gate is 0.90 | `experiments/alx` |
| Stateless derivability (LX-4) | **Not yet run** | `experiments/lx/LX-4-*` |
| Users / ecosystem | None outside the author | — |

## What "open to the public" means here

Opening the repository is not the same as claiming maturity. The plan is to open it
**with the claims scoped to the evidence**, and let the experiments in
[`docs/maturity-experiments.md`](docs/maturity-experiments.md) move the claims up.

## Phase A — Open the repo (blockers before announcing)

- [ ] Fix repository metadata: `Cargo.toml` points to `PragmaWorks/loom`, the remote is `jghiringhelli/loom`. Pick one.
- [ ] Re-baseline test counts in CI output (docs quote 314 / 339 / 410 depending on the file). One source of truth: the CI log.
- [ ] Ship a tagged `v0.2.0` release with prebuilt binaries (Linux, macOS, Windows) — currently no cross-platform artifacts.
- [ ] `README` quick start verified on a clean machine (Windows included) in ≤ 5 minutes.
- [ ] Triage and label GitHub issues (`good first issue`, `research`, `ALX gap`).
- [ ] Publish `SECURITY.md` and confirm `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` are current.
- [ ] Deploy `website/` (static Astro site) or drop the claim that it is live.

## Phase B — Prove or scope the central claims (experiments)

Each claim has a pass/fail gate in [`docs/maturity-experiments.md`](docs/maturity-experiments.md).
A claim that fails its gate is rewritten, not defended.

- [ ] **ALX**: raise S_realized from 0.339 toward ≥ 0.90 (gaps G11–G15). Publish only if the gate passes.
- [ ] **LX-4**: run the five stateless-derivability trials in fresh sessions and record results.
- [ ] **BBOB**: extend from 4 to the full 24-function suite; report where T5 does *not* help (sphere, Rastrigin already show no gain).
- [ ] **Compiler maturity**: fuzzing, mutation testing, and a differential check of emitted Rust against `rustc`.

## Phase C — Make it usable (after A and B)

- [ ] Publish to crates.io.
- [ ] Standard library of consequence (collections, strings, time, IO effects).
- [ ] LSP beyond the stub (hover types, go-to-definition, diagnostics) + VS Code extension.
- [ ] Build targets M24–M30: Dockerfile, Kubernetes, Terraform, OpenTelemetry, migrations.
- [ ] Reduce the "monolith" risk: keep checkers stateless, one module per construct, no cross-checker coupling.

## Phase D — Research directions (no dates, no promises)

- Self-healing loop (observe → drift → AI patch → compile → verify → deploy), M31–M40.
- BIOISO ecosystem specification as a shared format.
- Structural (type-theoretic) safety for self-modifying systems, beyond the current SafetyChecker.
- The "synthetic life" claim is a philosophical argument awaiting formal critique, not an established result.

## Explicitly not promised

- Production readiness, stability guarantees or semver stability before v1.0.
- That the biological isomorphisms are theorems — they are structural analogies with historical traceability.
- That BIOISO's T5 tier beats simpler baselines on all landscapes (current data says it does not).

## How to help

Critique is the most valuable contribution: PL theorists on the checkers, biologists on the
isomorphisms, anyone on reproducing the experiments. Open an issue with the experiment ID
(e.g. `LX-4`, `ALX-G12`, `BBOB-f15`).
