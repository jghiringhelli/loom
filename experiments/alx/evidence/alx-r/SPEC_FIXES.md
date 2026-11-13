# ALX-R spec fixes — gap → spec change

Input: `summary.md`, `DERIVATION_LOG.md`, `per-target.tsv` (ALX-R: 145/387 = 0.375, gate 0.90).
Scope: only `experiments/alx/spec/loom.loom` and `docs/language-spec.md` changed. No Rust in `src/` and no derived code was touched.
Method: for every gap, the behaviour was taken from the oracle suite (`experiments/alx/derived/tests`, which is what scores a
re-derivation) and cross-checked against the real compiler (`target/debug/loom`) and `tests/`. The spec states behaviour,
formats and short examples; no test or `src/` code was pasted. "R*" = section of the new REFERENCE BEHAVIOUR appendix at the
end of `loom.loom`; "§" = section of `docs/language-spec.md`.

## A. Compile failures (4 targets, 48 tests: being, effect, lsp, units)

| Gap | Why the target did not compile | Spec change |
|-----|--------------------------------|-------------|
| `LoomError::kind()` unspecified | effect_test and units_test call `e.kind() == "EffectError"/"TypeError"` | R2: full `LoomError` contract, `kind()` strings, `span()`, constructors, Display must contain the message |
| `EffectError`, `InfoFlowError` variants missing from the G8 list | tests/checkers construct and match them | G8 list extended; R2 maps each stage to the variant it raises |
| `Span::synthetic()`, `Span::merge`, `ast::Span` re-export unspecified | being_test builds AST with `Span::synthetic()` and imports `loom::ast::Span` | R2 + comment above `Annotation` |
| `RegulateBlock.bounds` was `Option<String>` here but tests build `Some(("35","39"))` | being_test | `bounds: Option<(String, String)>`; R3 shows `bounds: (lo, hi)` |
| `tower-lsp`, `tokio` dependencies never mentioned; LSP format unspecified (log: "no JSON-RPC") | lsp_test imports `tower_lsp::lsp_types` and `#[tokio::test]` | R1 (exact dependency list) and R7.3 (signatures of `byte_offset_to_position_raw`, `loom_error_to_diagnostic`, `LoomLspServer::new`, clamping, severity/source) |
| Checker constructors/paths (`loom::checker::UnitsChecker`, `codegen::openapi::OpenApiEmitter`) | being/units tests | G3 note: every codegen submodule is `pub`; G4 already lists checkers; R4 gives each checker's entry point |

## B. Exact AST shapes (tests build `Module`/`BeingDef` literals, so every field matters)

| Gap | Spec change |
|-----|-------------|
| `TypeExpr` variants carried `String` payloads (spec text) while the parser/tests need structured values | Exact Rust shape documented above the enum (Base/Generic(String, Vec)/Effect(Vec<String>, Box)/Option/Result/Tuple/TypeVar(u32)/Fn); `Float<usd>` = `Generic("Float",[Base("usd")])`; effect tiers split into `FnDef.effect_tiers` |
| `TypeDef`/`EnumDef` lacked `type_params`; `FnDef` lacked `inline_body`, `param_names` | fields added |
| `InterfaceDef.methods: List<String>`, `Provides.interfaces`, `Requires.dependencies` | replaced by `methods: List<FnDef>`, `Provides.ops`, `Requires.deps` with the source syntax shown |
| Derives (Debug/Clone/PartialEq) unspecified | comment above `Annotation` |
| `TelomereBlock.limit` type | noted as i64 |
| Body storage convention (checkers re-parse text) | R3 "AST body storage": one string per line, `match` stored as `match x { P => e, ... }`, contracts, `with`, tiers |

## C. Contradictions between `loom.loom` and `docs/language-spec.md` (resolved against compiler + oracle)

| # | Contradiction | Truth now written in both files |
|---|---------------|----------------------------------|
| 1 | `Effect<[IO], T]` (bracket closer, language-spec) vs real `>` | `>` only; all examples in both files normalised; R0.1, §0 |
| 2 | `match x with \| P => e end` (language-spec) vs real `match x \| P -> e end` | R0.2, §0, §4.5, §5 rewritten |
| 3 | `describe:` after the signature (§4, §8.1) vs real: before `::` | R0.3, §4 template, §8.1 |
| 4 | `provides: a` / `requires: A,B` (§7.1) vs real brace blocks | R0.4, §7.1 (also explains context struct / `with`) |
| 5 | G10 `x y => body` explicit parameter names vs real: no such syntax, names inferred | G10 rewritten with the inference algorithm and `arg{i}` fallback; R3 |
| 6 | OpenAPI "YAML" / JSON Schema "draft-07" (loom.loom) vs real JSON text, OpenAPI 3.0.3, draft 2020-12 | emitter descriptions + R0.6, R6.3, R6.4 |
| 7 | `@exactly-once → POST promoted to PUT` (loom.loom) vs `@idempotent → PUT` only (§9.3, §11, real) | `@idempotent` only; `@exactly-once` keeps the verb, adds `x-exactly-once`/`x-retry-policy` |
| 8 | NeuroML `ensure: starts_with("<neuroml")` vs required `<?xml` prolog; synapse `rule="Hebbian"` vs lowercase | R0.7, R6.7, `emit_neuroml`/`compile_neuroml` ensures corrected |
| 9 | Simulation "only autopoietic beings" vs every being gets an Agent class | `emit_simulation` comment, R0.8, R6.6 |
| 10 | Safety: §14.8/§15.3 require `@mortal @corrigible @sandboxed`; loom.loom rules 1-6 require only `@mortal` + `@sandboxed` | §14.8, §15.3, SafetyChecker comment, R0.9, R4.11 |
| 11 | §14.8 rows for `plasticity` without `learning_rate`, `epigenetic` without `trigger`, `morphogen` without `gradient` (fields that do not exist) | table rows replaced by the real checks and messages |
| 12 | "12 stage" vs 13 stage pipeline (implements + DI checks were undocumented) | checker section + R4.12 |
| 13 | Language-spec forms the compiler never accepted (`if/then/else`, lambda `fn x :: T => e`, raw `{ }` bodies, generic type definitions, `import X as Y`, tuple payload `of A * B`, `{ }` type form, `(A -> B) -> C`, newline `for`, `interface Repo<T>`, numeric `regulate target/bounds`, `matter: { }`) | new §0 lists them as not-to-implement; examples in §3, §4.5, §5, §6, §7, §14.5, §14.10, §15.2 rewritten with forms verified against the real compiler (the five big examples were re-run through `loom compile --check-only`) |

## D. Unspecified formats / algorithms

| Gap (from DERIVATION_LOG) | Spec change |
|---------------------------|-------------|
| ProjectManifest format ("unspecified") | R7.2: TOML subset, keys, required `name`, defaults, `build_project` outputs (`<stem>.rs`, `lib.rs` with `pub mod`) |
| CLI behaviour ("stdout default") | R7.1: default writes a file, `compiled ...` line, `--check-only` prints `ok` and writes nothing, `build` prints `build ok`, error format |
| LSP wire protocol | R7.3: only the utility functions and a trivial `LanguageServer` impl are required |
| Hindley-Milner inference ("heuristic only") | R5: concrete algorithm (unify, literal/binop/comparison/`as` rules, unknown = no error), error message text, and a verdict table covering every inference test |
| Heuristic checker details | R4.1-R4.12: when each check fires, which variant, which message substrings the tests look for |
| being/effect/units output formats | R6.1 (Rust incl. units newtypes, effect tiers, beings, ecosystems), R6.2 (TypeScript), R6.5 (WASM), R6.6 (Mesa), R6.7 (NeuroML) |
| Emitted-text conventions (context struct, `_pipe`, `todo!()`, `Result<_, Box<dyn Error>>`, doc comments, imports, `_check_invariants`, `mod tests`) | R6.1 |
| JSON/OpenAPI layout (compact vs spaced keys) | R6.3, R6.4 |
| Naming rules (`AHL` → `a_h_l`, `MathLib` → `math_lib`/`math-lib`, `MImpl`) | R6.0 |
| Concrete syntax of self-modification blocks (epigenetic, morphogen, telomere, crispr, plasticity, quorum, autopoietic, annotation placement) — absent from language-spec | R3 and new §14.12 (example compiles) |
| Token/lexer details | unchanged; tests only use `Lexer::tokenize` + `Parser::new(&tokens)`, R7.4 states that |

## E. Gaps deliberately left open

* **Literal `#[loom_pii]`.** The frozen ALX oracle checks the substring `#[loom_pii]`; the production compiler and `tests/` have since moved to
  `#[cfg_attr(loom_runtime, loom_pii)]`. R6.1 specifies the oracle's form and says so. Reconcile before the next oracle refresh.
* **Same drift in other places** (refined-type `try_from` uses `if !(..)` in production vs `debug_assert!(pred` in the oracle; production OpenAPI paths are
  resource-inferred). The oracle form is specified; only the oracle-tested substrings are normative.
* **Full Hindley-Milner.** R5 specifies the observable verdicts, not a general algorithm over a typed AST; the AST keeps bodies as text, which caps what inference can be.
* **Complete Rust expression translation / e2e tests.** The `rustc`-running tests (e2e, iteration, coercion, inline) need valid Rust for the small
  expression subset in R3/R6.1; arbitrary programs are not specified.
* **Real LSP behaviour** (hover, completion) is out of scope; the oracle only exercises the utility functions.
* **`loom.loom` is not itself accepted by `loom compile`** (it was not before this change either): a `type Span` field named `end` and a `Module.requires` field collide with
  keywords. Fixing that would mean renaming fields the AST must keep, so it is left as is; nothing in `tests/` parses the spec file.
* **Unverified by a run.** No re-derivation was executed (orchestrator's job); spec statements were verified against the oracle tests, the human-patched
  derived code and the real binary, not against a blind reader.
