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
- Result (first run): **12 / 42 compile (29 %)** — gate was 100 % → **FAIL** (see re-run below: 40 / 42)
- `loom compile` exits 0 on all 42; the failures appear only when `rustc` is run on the output.
- Raw data: `m1-emitted-rust.tsv`, first compiler error per failing file: `m1-first-errors.tsv`.

### M1 re-run after codegen fixes (branch `worktree-agent-a8fac74895c915397`)
- Result: **40 / 42 compile (95 %)** (was 13/42 at the start of this pass; the 12/42 above was the first recorded run) — gate 100 % → still **FAIL**, but the remaining two are not codegen bugs.
- Fixes (all general, in `src/codegen/rust/`):
  1. Param naming (`functions.rs`): contract/body free variables are bound to the parameter slot whose *type* matches how the name is used (`x.len()` → list/String, `x.f` → record, `x > 0` → Int, `x > 0.0` → Float, `a + b = 1.0` → Float). `result` is never a param name. Fixes the E0599/E0369 clusters.
  2. `xs.len()` / `xs.length` emit `(xs.len() as i64)`; `xs.all(p)`/`xs.any(p)` emit `xs.iter().cloned().all(p)`. Int literal beside a float literal is widened (`0 - 1.0`).
  3. Trigger/action `regulate:` blocks get fn names from the action (was a snake-cased token dump) and unit return; symbolic regulate bounds (`bounds: (min_energy, max_energy)`, never defined) become `±INFINITY` instead of `min_energy_f64`.
  4. `edit_<guide>` takes `guide: &str` (was the guide *name* used as a type); beings without `evolve:` get a no-op `evolve_step()` so ecosystem `coordinate()` compiles; duplicate epigenetic method names are disambiguated; lifecycle state marker structs are emitted once per module.
  5. Types referenced but never declared (`DbConn`, `Logger`, `GPState`, ...) get an opaque unit-struct stub (skipped when the module has imports).
  6. `todo!()` bodies with `ensure:` are treated as stubs; `inline {}` bodies are bound as a block and `argN` aliases are bound to the recovered param names; Loom `-- comment` trailers inside inline Rust become `//`; string-literal tail of a `String` fn gets `.to_string()`; refined types get `Deref`, and an undefined named predicate (`where valid_email`) is bound locally to `true`.
- Remaining failures (2), not fixable in codegen:
  - `corpus/pricing_engine.loom` (E0308): `fn compute_total :: OrderLine -> OrderTotal` returns a `Float` (`total`) where `OrderTotal` is a 3-field record. The source is ill-typed and the Loom checker accepts it (checker gap, not an emitter gap).
  - `examples/tier4/neural_combinatorial.loom` (E0433): an `inline { }` body calls `rand::random`; plain `rustc` has no `rand` crate. Needs a crate-dependency story for inline blocks, or a source change.
- Known semantic weaknesses behind the green count: unresolved named predicates and symbolic regulate bounds compile as "unchecked" (always true / unbounded); undeclared types are empty stubs; parameter names are still *recovered* from usage (signatures carry no names), so a program that never mentions a parameter gets `argN`.
