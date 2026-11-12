# ALX-R — blind re-derivation from the FINAL spec

Date: 2026-11-12 · spec: `experiments/alx/spec/loom.loom` @ 23f26ca-era (839 lines) + `docs/language-spec.md` (1127 lines)
Reader: one fresh Claude subagent (Sonnet 5.5), instructed to read only those two files; ~17 min, 54 tool calls, 33 files.
Oracle: the original ALX test suite (`experiments/alx/derived/tests`, 36 targets, 387 `#[test]`), run per target;
a target that does not compile counts all its tests as failed. No human edits to the derived code before measuring.

**Result: S_realized = 145 / 387 = 0.375 (gate ≥ 0.90 → FAIL).**
Library + binary compiled cleanly; 4 test targets failed to compile (being_test, effect_test, lsp_test, units_test = 48 tests).

Comparison with history: blind 0.000 → (spec API fixes) 0.339 → … → 1.000 with manual patches to derived code.
A blind reader given the final spec reaches 0.375, i.e. the spec as written transmits roughly a third of the
behaviour the suite checks; the rest of the 1.000 came from human corrections in `derived/`.

## Caveats (read before quoting)
- One run, one model, one prompt; no repeats, so no variance estimate.
- The oracle is the incumbent compiler's test suite. Many tests assert on exact emitted text/names, so a valid
  but different emission counts as failure. This measures conformance to the reference, not correctness.
- The reader is an LLM that may have seen similar compilers in training; the instruction not to read the repo
  was followed per its own report but is not technically enforced (no sandbox).
- 387 `#[test]` attributes vs 386 reported before; testblock_test shows 3 pass + 6 fail of 12 (3 not run).

## What the reader reported as gaps (see DERIVATION_LOG.md)
No real HM inference; heuristic type checks; generated Rust is best-effort (small expression translator,
`unimplemented!()` fallbacks; literal `#[loom_pci]` attributes); spec conflicts between loom.loom and
language-spec.md; unspecified formats (ProjectManifest, LSP, token layout).
