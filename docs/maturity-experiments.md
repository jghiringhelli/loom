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
| M0 | Does `main` build and test clean? | **PASS** (2026-11-12, Windows) — Linux CI re-verifying | `cargo test --no-fail-fast` 0 failures | 1975 passed / 0 failed / 17 ignored, 169 suites (`experiments/maturity/evidence/summary.md`). Previous "green" CI was masked by `\| tee` without pipefail and ran only ~480 tests |
| M1 | Does emitted Rust compile? | **FAIL** (2026-11-12; re-run 40/42) | 100% of `corpus/` + `examples/` emit Rust that passes `rustc --edition 2021` | 40/42 (95%) after codegen fixes (was 12/42). Remaining: `pricing_engine` (source returns Float for a record type; checker gap), `neural_combinatorial` (`inline` block needs the `rand` crate). See `evidence/summary.md` |
| M2 | Do tests catch bugs? | **FAIL** (2026-11-13, CI) | Mutation score ≥ 80% on `checker/` | 48 caught / 72 missed of 120 viable (40%) on `smt_bridge.rs` (44 missed), `refinement.rs` (27), `property_checker.rs` (1). Only 3 of the 11 checker files were mutated. Lists in `experiments/maturity/evidence/m2/`. The earlier CI gate parsed console text and was meaningless; fixed in `0a49955` |
| M3 | Does the parser survive hostile input? | To run | 1M fuzz execs on lexer+parser, 0 panics | — |
| M4 | Are checkers sound on negative cases? | To run | Every checker has ≥ 5 *must-fail* programs that fail with the expected error | — |
| LX-1 | Semantic density vs TS/Rust | Done | ≥ 2× tokens/property | `experiments/lx/LX-1-semantic-density` (2.66× avg) |
| LX-4 | Can a fresh LLM session extend Loom from the spec alone? | **Pending** | ≥ 4/5 features compile within 2 attempts | `experiments/lx/LX-4-*` |
| ALX | Guided derivation + corrections reaches full test pass? | Done (caveat) | S_realized ≥ 0.90 | 1.000, 386/386 (2026-04-27); phases 4–6 patched derived code directly |
| ALX-R | Can a **blind** reader re-derive the compiler from the *final* spec? | **FAIL** (2026-11-12) | S_realized ≥ 0.90 with zero human edits to derived code | 145/387 = **0.375** (1 run, 1 model; oracle = incumbent's tests, so partly measures output-format conformance). `experiments/alx/evidence/alx-r/` |
| BBOB | Does T5 help optimization beyond T1–T4? | **PASS, scoped** (2026-11-12): reproduced bit-for-bit; 4 of 24 fns, DIM=10, 30 trials | T5 better on ≥ 1 class **and** not worse elsewhere, p < 0.05 | Ellipsoid p=4.8e-5 (Cliff δ=0.61), Lunacek p=1.3e-8 (δ=0.86); sphere p=0.72 and Rastrigin p=0.99 = no difference (`evidence/stats.txt`). Limits: only 4 functions, baseline is T1–T4 not CMA-ES |
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

### ALX — Self-applicability (done, with caveat)
History: 0.000 → 0.339 → 0.641 → 0.780 → 0.900 → 1.000. Commits for phases 4, 5a, 5b modified `experiments/alx/derived/src` without modifying `spec/loom.loom`, so the final score reflects spec **plus** manual patches.

### ALX-R — Blind re-derivation from the final spec
Fresh session, no repo access except `docs/language-spec.md` and `experiments/alx/spec/loom.loom` (final version). Derive into a new directory, run the same test suite, record S_realized and the list of failures. Then diff against `derived/` to see which spec statements were insufficient. No manual edits allowed before measuring.

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
- **Claim "AI-derivable"** only when LX-4 and **ALX-R** gates pass (ALX 1.000 alone is not enough: it included manual patches).
- **Claim "BIOISO T5 improves optimization"** only on the function classes where BBOB shows it.
- **Claim "mature"** is not on the table before v1.0; the target wording is *research compiler*.
