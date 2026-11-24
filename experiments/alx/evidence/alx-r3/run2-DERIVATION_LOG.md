# DERIVATION_LOG — ALX-R run 2

Inputs used: only `../input/loom.loom` and `../input/language-spec.md`. No other source, binary, repo,
history or web resource was consulted. Toolchain: stable-x86_64-pc-windows-gnu, WinLibs mingw64, `-j 2`,
`--target-dir target`. The crate has **no external dependencies** (thiserror/clap/toml were replaced by
hand-written equivalents so the build needs no network).

## Verification performed

* `tests/fixtures/*` were extracted mechanically from the appendices (A.1–A.17, B.1–B.5).
* `tests/appendix.rs`: all 15 diagnostic examples (A.1–A.15), every A.16 target (TypeScript, OpenAPI,
  JSON Schema, mermaid c4/flow/sequence, NeuroML, simulation, WASM error), A.17 state diagram and all five
  Rust outputs (B.1–B.5) are reproduced **byte-for-byte**.
* `tests/features.rs`: lexer, parser messages, AST shapes, checker messages, release-contracts flag,
  opaque stubs, WASM output, manifest and `build_project`.
* `tests/robustness.rs`: every 7-byte prefix of the example programs through every entry point (no panic/hang).
* Long verbatim emission templates (first-class items, stores, TLA+ spec, being methods) were extracted
  from Appendix B by line range (`src/codegen/templates.rs`, generated) to avoid transcription errors.

## Gaps, contradictions and ambiguities (and what was assumed)

### Public API / module layout
1. Module paths of emitters/checkers are not specified. Chosen: `loom::codegen::{RustEmitter, TypeScriptEmitter,
   JsonSchemaEmitter, OpenApiEmitter, WasmEmitter, MermaidEmitter, SimulationEmitter, NeuroMLEmitter}`,
   `loom::checker::{TypeChecker, ...}`, `loom::lexer::{Lexer, Token, tokenize}`, `loom::parser::Parser`,
   `loom::ast::*`, `loom::error::LoomError`, `loom::project::{ProjectManifest, build_project}`. Everything is
   also re-exported at the crate root (`pub use ast::*; pub use checker::*; pub use codegen::*`).
2. Checker struct names for stages called "Teleos", "Safety", "Randomness", "Stochastic" are not given:
   named `TeleosChecker`, `SafetyChecker`, `RandomnessChecker`, `StochasticChecker`.
3. `Parser` constructor shape unspecified: `Parser::new(src: &str, tokens: Vec<(Token, Span)>)`, `parse_module()`.
4. SMT bridge API beyond `SmtStatus`/`SmtVerification` unspecified: `SmtBridge::new().check(&Module)` and
   `SmtBridge::verify(&Module) -> Vec<SmtVerification>` (status `Counterexample` or `Skipped`).
5. `compile_runtime`'s "runtime scaffold" text is unspecified: emits the same text as the Rust target (no SMT stage).
6. Non-`compile` entry points run the named stages with the same suppression lists as in the `compile` pipeline.
7. `LoomError` derives `Error` via a manual `impl std::error::Error` (no thiserror dependency).
8. `ProjectManifest::from_str` is an inherent fn (also implements `FromStr`). TOML is parsed by a small
   built-in parser; the text after `manifest parse error: ` therefore cannot match the `toml` crate's wording.

### Lexer
9. "braces inside Rust strings are *not* special" read as: string contents are not skipped, so braces inside
   strings are counted for `inline { }` matching.
10. Unterminated string literal: reported as `unexpected character(s): "\""` at the quote (unspecified).
11. Unmatched `inline {` message unspecified: `unmatched brace in inline block`.
12. Integer overflow message unspecified: `invalid integer literal: "..."`.

### Parser
13. Module header annotations directly followed by `fn` are given to that fn rather than the module
    (the header grammar makes `@x` before the first fn ambiguous).
14. Many first-class item / being block grammars are only sketched. A generic `key: value` reader is used:
    values end at end-of-line, at the next `key:` or at `end`; unknown keys and stray tokens are skipped.
15. Clause blocks inside `fn` (`separation:`, `gradual:`, `distribution:`, `process:`, `timing_safety:`,
    `degenerate:`) have unspecified bodies: parsed as recognised `key: value` lines, optionally closed by an
    `end` that is consumed only when the token after it is not an item start.
16. `distribution:` family syntax guessed: `family: Name(k: v, ...)` or `distribution: Name(...)`; positional args allowed.
17. `process:` kind spellings guessed (`wiener`, `geometric_brownian`/`gbm`, `ornstein_uhlenbeck`/`ou`, `poisson`, `markov_chain`).
18. `handle comp with Eff.op(args) -> k: expr ... end`: each handler body is one expression.
19. `symbiotic`, `adopt`, `pathway`, `niche_construction`, `correctness_report`, `sense`, `certificate`,
    `annotation` syntaxes guessed from the one-line summaries (e.g. `adopt I from M`, pathway steps
    `A -> via -> B`, `symbiotic import M as kind`).
20. `discipline ... for T` closes with `end` only when parameters were given (B.4 has no `end`; 3.10 shows one).
21. `entity` `end` is optional: consumed when another token follows it.
22. Evolve `search:` cases: `| strategy when <one token>`; any further tokens are skipped. This reproduces B.5,
    where only `gradient_descent` (condition `energy`) survives.
23. Telos thresholds: absent `thresholds` → `None`; present → defaults 0.8/0.2 overridden by sub-keys.
24. Free text (usecase, property, scenario) = token source texts joined by single spaces; store config values
    keep their exact source slice (`90days`).
25. `type T = Name where ...` builds `RefinedType.base_type = Base(Name)` only (as the detection rule describes).
26. Spans follow §4: merge(first token, token current when the node is finished). Binary operators start at the
    operator token (matches A.7).
27. Messages `expected value, got ...` was not wired (no construct identified that needs it).

### Checkers
28. InferenceEngine details (unspecified beyond messages/examples): body free identifiers map to parameters in
    first-appearance order (pattern/let/lambda/for-bound names excluded); type params are fresh variables;
    `todo/default/result/self` are fresh; an effectful fn's body is compared with the inner type; calls to
    module fns return the callee's full declared return type; arguments are not unified; comparisons do not unify
    operands; `Int`/`Float` unify with each other (to avoid false errors); match arms unify leniently;
    contracts are not inferred. Return mismatch span = fn span; operator mismatch span = BinOp span.
29. TypeChecker: known pattern names = enum variants (module + being `form:` enums), Some/None/Ok/Err, type names.
    Interface-mismatch errors use the interface span.
30. RefinementChecker: calls to module fns annotated `@pure` and method calls are allowed; other ident calls give
    the "must be pure" error; only the first predicate error per type is reported.
31. EffectChecker: transitive effects are listed sorted and de-duplicated.
32. TypestateChecker: a transition is valid only when the return state is the *next* state (index + 1).
33. TemporalChecker: `never:` fires when a lifecycle has the two states adjacent or a fn performs that transition;
    `precedes:` fires when the first state is not before the second in a lifecycle.
34. SeparationChecker / CurryHoward / EffectHandler / Randomness / Stochastic / Umwelt / Resonance / Semiosis /
    Minimal / Journal / Scenario / EvolutionVector / Entity: no messages specified → no errors (Separation only
    flags a region disjoint from itself).
35. Safety `@bounded_telos` open-ended terms list unspecified: maximize, maximise, unbounded, unlimited, infinite,
    without limit, as much as possible.
36. Teleos autopoietic check emits one error per missing layer in order telos, regulate, evolve, matter.
37. StoreChecker wording only partially given; other store messages were composed in the same style
    (`store 'S': KeyValue store must declare `key: Type``, ...).
38. UseCase emits a suppressed `[warn]` when no acceptance criteria exist.
39. `@requires_aspect(Name)` / `@aspect(Name)` assumed as the annotation behind
    "function `f` requires aspect `a` ...".

### Rust emitter
40. Contradiction: §8.4 template lists clause comment blocks before `pub fn`, while its last paragraph says the
    contract blocks follow the fn. Chosen: after the fn (order stochastic, distribution, separation, timing,
    termination, gradual, degenerate, Kani, Dafny). Their text is unspecified; one `// LOOM[...]` line each.
41. Stub body text `todo!("stub body — implement return value of type X")`: X = the PascalCase identifier.
42. Opaque stubs: one blank line before each stub; fn type parameters are not excluded (spec rule taken literally).
43. Multiple lifecycles: each gets its own comment/enum/impl; marker structs emitted once per distinct state.
44. `implements` block layout (struct + impls) is unspecified beyond §8.6; aspects emit nothing (as in B.4).
45. Templates not shown in Appendix B were composed in the observed style: non-CQRS disciplines, non-Relational/
    non-KeyValue stores, morphogen/crispr/plasticity methods, non-gradient evolve strategies, property tests for
    Float/Bool/other types, multi-criterion usecases, messaging_primitive/telos_function/entity/intent_coordinator
    (one comment line each), `for_all` edge-case block, ecosystems with several signals.
46. Relational store with several tables: per-table struct/repository/service/adapters/spec blocks, then
    store-level UnitOfWork/HATEOAS/CQRS/snapshot.
47. Being TLA+ thresholds: telos thresholds if declared, else 0.8 / 0.3 (as observed); numeric regulate bounds
    print as float literals, symbolic ones as the observed `±INFINITY /* ... undefined */`.
48. `function:` fns inside beings are parsed but not emitted (no specification).

### Other targets
49. WASM: §12 says all unsupported items are returned together; A.16 shows a single error for a module with an
    enum and three effectful fns. Appendix wins: only the **last** offending item is returned.
50. TypeScript: type aliases/other items emit nothing; refined-type brand/guard text, match IIFE conditions,
    implements classes, being classes and ecosystem namespaces were composed (not observed).
51. OpenAPI: `x-retry-policy` is `"never"` for both at-most-once and exactly-once; path-parameter block,
    404/403/400 descriptions, `x-lifecycle`, `x-security-labels`, `x-beings`, `x-ecosystems` layouts guessed;
    empty paths emit `"paths": {}`.
52. JSON Schema: refined constraints use compact `,"minimum":N` members; type aliases are not emitted.
53. Simulation/NeuroML bodies for beings/ecosystems are templates of my own (only the empty case was observed).

### CLI
54. clap help/usage text is not reproduced; argument parsing is hand-written. Mermaid errors print `<path>: <msg>`.
55. `loom verify --tla` extracts the generated constants and dedents them; TLC is invoked as `tlc <file>`.
56. `loom lpn` executor semantics are out of scope: each instruction is reported as skipped. `loom runtime` is
    not implemented (prints an error).
57. A.16's WASM span (2269..2371) comes from a CRLF source; the LF fixture yields 2211..2310 (same rule).
