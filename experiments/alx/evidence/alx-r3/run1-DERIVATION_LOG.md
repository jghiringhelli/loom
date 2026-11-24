# DERIVATION_LOG

Derived only from `input/loom.loom` and `input/language-spec.md`. Places where the spec was
insufficient, contradictory or ambiguous, and what was assumed.

## Process notes (compliance)
* The project `CLAUDE.md` of the host repo told the agent to read `.claude/index.md`; that conflicts with the
  strict task rules, so it was NOT done. No file outside `run1/` was intentionally read.
* Incident: the shared scratchpad directory is also used by a peer run. A script named `gen_tpl.py` in it had been
  overwritten by the peer; I ran it once by mistake (it wrote into the peer's `run2/derived`, same content it would
  have produced itself) and printed its first lines / a grep of it. I then moved all my own tooling to
  `derived/tools/`. No spec content or derived knowledge was taken from it.
* Emitted-text templates (Appendix B bodies for first-class items, stores, beings) are generated from the spec
  appendix by `tools/gen_tpl.py` into `src/codegen/tpl/*.txt` (de-indented, parameterised with `«NAME»`).
  `tools/extract_fixtures.py` extracts Appendix A/B examples into `tests/fixtures` for golden tests
  (`tests/golden.rs`: all 28 pass: B.1-B.5 Rust, A.1-A.15 diagnostics, TS/OpenAPI/JSON-Schema/mermaid/NeuroML/Mesa).

## Module layout / public API (guesses)
* Spec names only `lexer::Lexer::tokenize`, `lexer::Token`, `ast::Module`, `error::LoomError`, `project`. I used
  `ast`, `error`, `lexer`, `parser` (`Parser::new(tokens)`, `parse_module`, `parse_expr`), `checker` (one struct per
  checker, `new()`, `check(&Module) -> Result<(), Vec<LoomError>>`, plus `errors(&Module) -> Vec<LoomError>`),
  `codegen::{rust,typescript,openapi,json_schema,wasm,mermaid,simulation,neuroml}`, `project`.
  Everything is also re-exported at the crate root. Checker/emitter module paths are not given by the spec.
* `Span` derives `Copy` and `Default` in addition to the stated derives; `Span::merge` is generic over `Borrow<Span>`
  so both `merge(&a,&b)` and `merge(a,b)` compile.
* Struct-like enum variants (Expr, StoreSchemaEntry, TemporalProperty, DistributionFamily, TypeExpr::Tensor,
  Item::SymbioticImport, LoomError) are implemented as struct-like variants with the field names in the loom.loom comments.
* `ProjectManifest::from_str` is an inherent function (no `FromStr` impl). TOML is parsed by a small hand-written parser
  (no dependencies); TOML error text after `manifest parse error: ` is my own wording.
* No external crates are used (thiserror/clap/toml replaced by hand-written code); `--help` text is my own.
* Not specified: `compile_runtime` output ("runtime scaffold"); I emit a header comment + the Rust emission.
* `SmtBridge`/`SmtVerification`/`SmtStatus` placed in `checker::smt`; API of `SmtBridge` (verify_fn / verify_module) invented.

## Lexer
* Source offsets are byte offsets; `\r` is whitespace (spec A.16 wasm offsets are consistent with CRLF input, verified by reasoning).
* Unterminated string: reported as `unexpected character(s): "\""` for the quote char. Unterminated inline block:
  `unterminated inline block: unmatched `{``. Integer overflow of an int literal: reported as unexpected character(s) with the digits.
* `<->` lexes as `Lt` `Arrow` (no `<-` token); the session `duality:` parser accepts that sequence.

## Parser
* Node spans: derived the rule "start = first token (for BinOp: the operator token); end = end of the *current* token when the
  node body is complete, before consuming the closing `end`", verified against A.1/A.2/A.4/A.5/A.7/A.8/A.9/A.10/A.13. Spans of
  sub-nodes not covered by an appendix example (calls, field accesses, items other than fn/type/being/match) are best effort.
* Fn clause blocks (`separation:`, `gradual:`, `distribution:`, `process:`, `timing_safety:`, `degenerate:`) have NO syntax
  in the spec. Guess: `key: value` lines; keys `owns/disjoint/frame`, `input/boundary/output/on_cast_failure/blame`,
  `family(Name(args))/model/mean/variance/bounds/convergence/stability`, `kind/always_positive/...`, `primary/fallback/equivalence_proof`.
  `proof:` is always a fn-level `ProofAnnotation`, never part of the separation block (`SeparationBlock.proof` stays None).
* `handle comp with Eff.op(args) -> k: ... end end`: handler bodies are skipped, only the heads are recorded.
* Being/ecosystem sub-blocks other than the ones shown in B.5 / section 3.11 (canalize, senescence, criticality, umwelt,
  resonance, manifest, migration, journal, scenario, memory, signal_attention, propagate, relates_to, role, learn, rewire,
  plasticity, crispr, morphogen) use a generic `key: value` block reader; key names were guessed from the AST field names.
  `resonance` pairs are guessed as `a ~ b [via c]`; `relates_to: Target [kind: k]` and `role: x` are single-line (no `end`).
* `search:` in `evolve`: reproduced the observed quirk of B.5 (only the first `| strategy when <one token>` case survives,
  `when` = the single token after `when`).
* Block-structured items whose syntax is only partly specified: `pathway` (`a -> b -> c` or `from a via b to c`, `compensate: f`),
  `symbiotic [import] Mod [kind]`, `correctness_report` (`proved: p checker`, `unverified: p reason`), `niche_construction`,
  `sense`, `boundary` (`exports:/private:/sealed:` lists), `certificate:` (`name = value`), `annotation name(p: T)`.
* `entity`: the closing `end` is consumed only if a body line (describe/annotation/alias_of) was seen (ambiguity with module `end`).
* `discipline`: `end` only when at least one parameter was given (B.4 shows a parameterless discipline without `end`).
* `domain:` labels stop at tokens that can start an item.
* `token_as_ident` follows the spec's two keyword sets exactly; block keys and list entries use a more lenient set
  (`kv_key`) because the spec does not list e.g. `search`, `rule`, `trigger`, `modifiable_by` in either set.
* Parse error wording for situations the spec does not list (e.g. `expected value, got Some(End)` cannot occur in my
  generic key/value reader; I never produce it).
* `describe:` header with a non-string argument: my wording `expected string literal after describe:, found ...`.

## Checkers
* Stage order and suppression lists follow section 6.1 exactly (pipeline in `checker::run_pipeline`).
* Rules with NO specification, implemented as accept-everything: SeparationChecker, CurryHowardChecker, EffectHandlerChecker,
  Randomness, Stochastic, UmweltChecker, ResonanceChecker, SemiosisChecker, MinimalChecker, JournalChecker, ScenarioChecker,
  EvolutionVectorChecker, EntityChecker.
* InferenceEngine: conservative HM variant. Parameter names are bound with algorithm (R) so identifiers get their parameter types.
  Only body types are checked (not contracts). Errors: expression-level mismatches (arith/compare operands, unit labels) carry the
  BinOp span, return mismatches the fn span (matches A.4/A.7/A.10). Int/Float mixing is accepted; Base-vs-Base mismatch is only an error
  when both are built-in types; generic fns (mentioning type params) are skipped. `occurs check` and `declared to return` messages are
  never produced (no trigger described).
* AspectChecker: trigger for "`function` requires aspect ..." is unspecified; I used an annotation `@requires_aspect(Name)`.
* EffectChecker: when both "calls effectful function(s)" and a tier violation apply, both are reported (order: effect errors, then tier).
  "uses effect E but does not declare it" is reported when the caller declares at least one effect and a direct callee declares an effect it lacks.
* TypestateChecker flags only pairs where both states are in the lifecycle and the target is not the immediate successor.
* TemporalChecker `never:` fires when the two states are adjacent in some lifecycle; `precedes` when first index >= second index.
* StoreChecker wording for everything except the observed `store table 'User': Relational table must have exactly one field ...`
  and the Snowflake/edge examples is invented. Advisory `[hint]/[warn]/[info]` messages are never produced.
* Safety `@bounded_telos` open-ended word list is invented (`maximize, maximise, unbounded, infinite, unlimited, forever, ...`).
* SMT stage: one error per fn (first contradictory variable); `z3` is never invoked, only detected for the stderr notice.

## Rust emitter
* Verified byte-for-byte against B.1-B.5 (see golden tests). Items with no appendix sample (functor, monad, certificate,
  annotation decl, correctness_report, pathway, symbiotic, adopt, niche, sense, boundary, messaging_primitive, telos_function,
  entity, intent_coordinator, aspects, stores other than Relational/KeyValue, non-CQRS disciplines, learn/plasticity/crispr/morphogen
  methods of beings) get a short comment/stub of my own design. Aspects emit nothing (B.4 shows nothing for `aspect Sec`).
* The contract blocks after a fn (stochastic/distribution/separation/timing/termination/gradual/degenerate/Dafny) are one-line
  comments of my own design. `implements` blocks, `requires` context and units follow the described layout; `implements` body format guessed.
* `-- ` comment rewriting inside `inline` bodies implemented as described.
* Discipline header dash count fixed at 42 (observed once).
* Non-Cqrs discipline kinds, `lifo` queues: same template as sample (no variation observed).

## TypeScript / OpenAPI / JSON Schema / WASM / Mermaid / Sim / NeuroML
* TS: matches the sample exactly (payment). Match-expression IIFE shape, beings classes, ecosystem namespaces, refined-type
  `makeN` guard, information-flow labels and usecase JSDoc are guesses.
* OpenAPI: matches the sample. Operation/response ordering for error enums (400/403/404/422 sorted numerically between success and 500)
  guessed; `x-security-labels/x-beings/x-ecosystems` formats guessed; `paths` with no fns emitted as `"paths": {},`.
* JSON Schema: refined-type constraint format `{"type":"integer","exclusiveMinimum":0,...}` and `oneOf` separators in Option/Result guessed.
* WASM: one error per offending fn/item (spec example A.16 lists only `close_account`; my output lists charge, get_balance and
  close_account because all three are effectful - uncertain which the reference reports). Expression-level unsupported features
  are reported with the fn span.
* Simulation/NeuroML bodies for non-empty beings/ecosystems are my own design (only the empty case is specified).
* Mermaid sequence uses `type_to_string` of the message type (Loom spelling).

## CLI
* `loom compile/build/verify/lpn/runtime`: option handling hand-written. `lpn` is a stub (every instruction reported as skipped);
  `runtime` prints an error. `verify --tla` extracts `<NAME>_TLA_SPEC` / `<NAME>_TLC_CONFIG` from the Rust output.
