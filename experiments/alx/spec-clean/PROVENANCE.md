# Provenance of the clean spec (ALX-R)

Purpose: `loom.loom` and `language-spec.md` in this directory were written to be an *independent-derivability*
spec: derived from the compiler's public surface, its non-test source and observation of the real binary, and
**not** from the test oracle.

## Confirmation about the oracle

* I **never opened, listed, grepped or read anything under `tests/`**.
* I never opened, listed or read `experiments/alx/derived/` or `experiments/alx/evidence/`. (The single
  directory listing `ls experiments/alx` at the start printed the *names* `derived` and `evidence` among the
  entries of `experiments/alx`; I listed no content of either.)
* I never ran `cargo test`, never read test output, and never grepped test output for assertions. I ran
  `cargo build -j 2` only (the compiler and its binaries, no test harness).
* I did **not** read `experiments/alx/spec/loom.loom` (the earlier repaired spec) except its first 200 lines
  (header and the AST-type section) at the start; I copied nothing from it, and in fact found it disagrees with the
  real AST (for example it gives `FnDef.body` as strings and `TypeDef.type_params`, which do not exist). I did not read `docs/language-spec.md`
  except lines 981-1210 (sections 14.12-15.1: the being sub-block syntax and tiers); everything else of that file is unread.

## Disclosed incidental exposures to `#[cfg(test)]` content (not used)

* `src/codegen/rust/mod.rs`: I read the file with a line range that crossed its `#[cfg(test)]` module
  (lines 1286-1450, three small unit tests that build a `Module` by hand and assert that the Rust output contains
  `pub struct Point` / `pub enum Color` / `debug_assert!`). Nothing in this spec depends on them; those facts are also visible in the emitter code and in binary output.
* Two grep result lines came from test modules: `src/checker/effects.rs:286` (a one-line test source string) and
  `src/checker/infer.rs:896` (an assertion message about the occurs check). Not used.
* After that I located the `#[cfg(test)]` cut-off line of every file with a grep for the attribute and read only code above it
  (all other reads used explicit ranges below the cut-off, or a script that stops at the first `#[cfg(test)]` line).

## What was read

Project instructions: `CLAUDE.md`, the first 30 lines of `.claude/index.md`.

Source (`src/`), non-test regions only:
`lib.rs`, `main.rs` (CLI definitions and the compile/verify/build/lpn handlers, lines 1-700), `error.rs`, `project.rs`;
`ast/mod.rs`, `ast/types.rs`, `ast/expressions.rs`, `ast/items.rs`; `ast/being.rs` (struct/enum/field skeleton via grep, plus lines 613-824 for a few blocks);
`lexer/mod.rs` (1-766); `parser/mod.rs` (1-1460 and the token helper 3040-3125), `parser/expressions.rs`, `parser/types_parser.rs`,
`parser/items.rs` (1-130, 130-220, 786-830, 1250-1810, 1941-2332), `parser/being.rs` (2060-2120, 1905-1930);
`checker/mod.rs`, `checker/loom_checker.rs`, `checker/types.rs` (1-356), `checker/exhaustiveness.rs`, `checker/effects.rs` (1-266),
`checker/typestate.rs`, `checker/gradual.rs`, `checker/privacy.rs`, `checker/algebraic.rs`, `checker/category.rs`, `checker/units.rs` (1-140), `checker/smt_bridge.rs` (30-447);
quoted-string extraction (script that stops at `#[cfg(test)]`) over the other checker files for diagnostic wording;
`codegen/rust/mod.rs` (1-1285 and 1452-1538), `codegen/rust/functions.rs`, `codegen/rust/types.rs`, `codegen/rust/exprs.rs`, `codegen/rust/contracts.rs`;
`codegen/typescript.rs`, `codegen/openapi.rs`, `codegen/schema.rs`, `codegen/wasm.rs`, `codegen/mermaid.rs`, `codegen/simulation.rs`, `codegen/neuroml.rs`;
`lpn/ast.rs` (top part). Not read: `runtime/`, `lsp.rs`, `stdlib/`, `codegen/rust/{beings,structures,disciplines,stores,telos,template}.rs`, `codegen/owl.rs`, `codegen/runtime.rs`
(their output was observed through the binary instead; Appendix B reproduces it).

Docs: `docs/language-spec.md` lines 981-1210 only (see above).
`examples/`: `01-hello-contracts.loom`, `02-payment-api.loom`, `03-typestate-lifecycle.loom`, `05-autonomous-agent.loom` read in full. `corpus/`: none read (compiled only).
`loom.toml` (repo root) and `corpus/project/` were used only to run `loom build`.

## What was run

`cargo build -j 2` (GNU toolchain; `target/debug/loom.exe`), then `loom compile ... [--check-only] [--target ...]` on:
programs I wrote (scratch probes outside the repository), every `.loom` under `examples/`, `corpus/` and `corpus/project/`
(all 40+ compile with `--check-only`), and `loom build loom.toml` (the generated `out/` directory was deleted afterwards).
I also ran `loom compile --check-only` on the `experiments/alx/*.loom` programs (feature-matrix and bioiso files) **without reading them**, only to learn that they compile
(and one `grep -l` for item keywords over them, which printed file names only).
Appendices A and B of `language-spec.md` are verbatim binary output for the inputs shown there; `loom.loom` was checked with the real
binary (`loom compile loom.loom --check-only` -> `ok — no errors`).

## Method notes / known limits

* Behaviour marked "(observed)" was confirmed by running the binary; "(read)" or unmarked text about checkers/emitters comes from reading non-test source.
* Not probed individually: every checker rule of sections 6.2 (messages were extracted from source; triggers for the hard-to-reach checkers such as safety, probabilistic, tensor, bio checkers were not exercised).
* The being / structure / store / discipline emitters (not read) are specified only by example output.
* The BIOISO runtime (`loom runtime ...`) and the LSP are out of scope.
