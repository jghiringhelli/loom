# Maturity & Efficacy Experiments

> Purpose: decide, with pass/fail gates fixed **before** running, what Loom may claim publicly.
> Rule: a gate that fails means the claim is rewritten. Results are committed under
> `experiments/<id>/evidence/` with seeds and exact commands. Last reviewed: 2026-11-12.

## Prerequisite — a reproducible toolchain

Experiments below need a Rust toolchain (`rustup`, stable) and `cargo`. CI already has one;
a fresh developer machine may not. Record `rustc --version` and the commit SHA in every result.

## Status board

| ID | Question | Status | Gate (pass if…) | Existing data |
|---|---|---|---|---|
| M0 | Does `main` build and test clean? | CI green (2026-05-15) | `cargo test` 0 failures on 3 OSes | CI logs |
| M1 | Does emitted Rust compile? | To run | 100% of `corpus/` + `examples/` emit Rust that passes `rustc --edition 2021` | — |
| M2 | Do tests catch bugs? | Partial (`mutants.out` was local) | Mutation score ≥ 80% on `checker/` | cargo-mutants, 2026 local runs (archived) |
| M3 | Does the parser survive hostile input? | To run | 1M fuzz execs on lexer+parser, 0 panics | — |
| M4 | Are checkers sound on negative cases? | To run | Every checker has ≥ 5 *must-fail* programs that fail with the expected error | — |
| LX-1 | Semantic density vs TS/Rust | Done | ≥ 2× tokens/property | `experiments/lx/LX-1-semantic-density` (2.66× avg) |
| LX-4 | Can a fresh LLM session extend Loom from the spec alone? | **Pending** | ≥ 4/5 features compile within 2 attempts | `experiments/lx/LX-4-*` |
| ALX | Can a blind reader derive the compiler from `loom.loom`? | In progress | S_realized ≥ 0.90 | 0.339 (Phase 3) |
| BBOB | Does T5 help optimization beyond T1–T4? | Partial (4 of 24 fns, DIM=10, 30 trials) | T5 better on ≥ 1 class **and** not worse elsewhere, Wilcoxon p < 0.05 | `experiments/bbob/evidence` |
| AEGIS | Does inter-generational meiosis transfer? | Partial | Held-out period beats parent genome | `experiments/aegis/evidence` |
| COL | Does the colony run unattended and stay safe? | Unknown since 2026-04-20 | 7 days, 0 safety-floor violations, drift bounded | Railway logs |

## Experiment protocols

### M1 — Emitted-code validity
For each `.loom` in `corpus/` and `examples/`: `loom compile` → `rustc --crate-type lib`. Record pass rate and the first error per failure. Pass: 100%.

### M2 — Mutation testing
`cargo mutants --package loom -- --lib` restricted to `src/checker/`. Report surviving mutants grouped by checker. Each survivor becomes a test or a documented equivalent-mutant.

### M3 — Fuzzing
`cargo fuzz` targets: `lex(bytes)`, `parse(str)`, `compile(str)`. Seed from `corpus/`. Success = no panic, no hang > 1 s, no OOM.

### M4 — Negative-case suite
One table-driven test per checker. This measures *soundness of rejection*, which positive tests cannot.

### LX-4 — Stateless derivability
Follow `experiments/lx/LX-4-stateless-derivability/protocol.md`. Fresh session each trial, no repo access beyond the prompts. Record verbatim output.

### ALX — Self-applicability
Follow `experiments/alx/protocol.md`. Report S_realized per phase and the correction log (information content of each fix).

### BBOB — Optimization efficacy
Extend to all 24 noiseless functions × DIMs {5, 10, 20} × 30 seeds. Baselines: T1–T4 and plain CMA-ES. Report convergence rate, median final NF with IQR, and an effect size. **Report losses and ties with equal prominence.** (Current data: gains on f2 and f24; none on f1 and f15.)

### COL — Colony soak
7-day Railway run with the oracle safety floors from `docs/invellum-fitness-oracle.md §1.4`. Pass: no floor violated; every promoted mutation has an audit record.

## Reporting template

```
id:        <experiment id>
commit:    <git sha>
toolchain: <rustc --version>
command:   <exact command>
seeds:     <list>
result:    <numbers>
gate:      PASS | FAIL
notes:     <what this does NOT show>
```

## Decision rule for going public

- **Open the repo** when M0, M1 and M4 pass (Phase A in `ROADMAP.md`).
- **Claim "AI-derivable"** only when LX-4 and ALX gates pass.
- **Claim "BIOISO T5 improves optimization"** only on the function classes where BBOB shows it.
- **Claim "mature"** is not on the table before v1.0; the target wording is *research compiler*.
