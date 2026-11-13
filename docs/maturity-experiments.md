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
| M1 | Does emitted Rust compile? | **FAIL, 40/42** (2026-11-13) | 100% of `corpus/` + `examples/` emit Rust that passes `rustc --edition 2021` | 13 → 40/42. Remaining: `pricing_engine` (ill-typed source the checker accepts), `neural_combinatorial` (inline block needs `rand`). Part of the green is weak: named predicates compile as unchecked, undeclared types become empty stubs, one test asserts an undefined `score_valid` (`evidence/summary.md`) |
| M2 | Do tests catch bugs? | **PASS on the 3 files measured** (2026-11-13) | Mutation score ≥ 80% on `checker/` | 40% → 108/120 caught = 90% (99% excluding 11 documented equivalent mutants) after 54 behavioural tests; `smt_bridge`, `refinement`, `property_checker` only — 8 other checker files unmeasured. Final full rerun after the last added test not done. `experiments/maturity/evidence/m2/` |
| M3 | Does the parser survive hostile input? | **PASS on the stress harness, not on cargo-fuzz** (2026-11-13) | 1M fuzz execs on lexer+parser, 0 panics | Seeded harness (`tests/fuzz_smoke_test.rs`, seed `0x4C4F4F4D00000003`): 1M inputs × (lex, parse, compile) = 3M calls. First run: 0 panics, 40 hangs, all one bug (unterminated `morphogen: produces: [` looped forever) — fixed + regression test; a debug-profile run then found a stack overflow (lexer, 189k-digit literal; abort, not a panic) — capped identifier/number runs at 4096 chars + regression test. Final release rerun: 0 panics / 0 hangs / no OOM, 551 s. Caveats: Z3 absent so SMT path skipped; not coverage-guided; `fuzz/` + `fuzz.yml` written but not executed (needs Linux nightly). Rerun with `LOOM_FUZZ_N=1000000 cargo test --release --test fuzz_smoke_test -- --nocapture` |
| M4 | Are checkers sound on negative cases? | To run | Every checker has ≥ 5 *must-fail* programs that fail with the expected error | — |
| LX-1 | Semantic density vs TS/Rust | Done | ≥ 2× tokens/property | `experiments/lx/LX-1-semantic-density` (2.66× avg) |
| LX-4 | Can a fresh LLM session extend Loom from the spec alone? | **Pending** | ≥ 4/5 features compile within 2 attempts | `experiments/lx/LX-4-*` |
| ALX | Guided derivation + corrections reaches full test pass? | Done (caveat) | S_realized ≥ 0.90 | 1.000, 386/386 (2026-04-27); phases 4–6 patched derived code directly |
| ALX-R | Can a **blind** reader re-derive the compiler from the *final* spec? | **PASS on paper, with a leakage caveat** (2026-11-13) | S_realized ≥ 0.90 with zero human edits to derived code | Before spec repair 0.375 (1 run). After repair: Sonnet 380/385 = 0.987, Opus 380/385 = 0.987. Fable 380/385 = 0.987 too. All three miss the same 5 `algebraic_test` cases because the spec did not say compile_openapi skips type inference; spec note added afterwards (not re-measured). **Caveat:** the spec repair was done by an agent that read the incumbent test suite, so the spec now encodes the oracle; this measures spec-vs-oracle completeness, not independent derivability. `experiments/alx/evidence/alx-r2/` |
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
