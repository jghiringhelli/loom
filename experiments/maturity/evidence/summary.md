# Maturity experiments — evidence summary

Run date: 2026-11-12 · commit: d8efbcf · toolchain: rustc 1.100.0 (stable-x86_64-pc-windows-gnu) · OS: Windows 11

## M0 — build + full test suite
- Command: `cargo test --no-fail-fast -- --test-threads=1` (same as CI)
- Result: **1975 passed, 0 failed, 17 ignored, 169 suites** (PASS)
- Findings that were fixed on the way (all committed in d8efbcf):
  1. `for n in xs { n }` failed to parse — record-literal branch (added 2026-04-27) swallowed `xs {`.
  2. `tests/cli_test.rs` hardcoded `loom.exe` → 28 tests failed on Linux.
  3. CI ran `cargo test | tee` without `pipefail`: failures at the first broken suite were hidden and
     ~1500 tests never ran; CI reported 480 tests and "success". Fixed with `shell: bash` + `--no-fail-fast`.
  4. `iteration_test` e2e temp-dir collisions on Windows (flaky).

## M1 — emitted Rust compiles
- Command: `experiments/maturity/evidence/m1-run.sh` (all `corpus/` + `examples/` `.loom`)
- Result: **12 / 42 compile (29 %)** — gate was 100 % → **FAIL**
- `loom compile` exits 0 on all 42; the failures appear only when `rustc` is run on the output.
- Raw data: `m1-emitted-rust.tsv`, first compiler error per failing file: `m1-first-errors.tsv`.
