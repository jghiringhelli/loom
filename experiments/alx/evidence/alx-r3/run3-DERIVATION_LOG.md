# DERIVATION_LOG — Loom compiler, clean-room run 3

Inputs used: `input/loom.loom` and `input/language-spec.md` only. Nothing else was read (no Loom
repository, binary, git history, web search or hidden tests).

Status: `cargo build` succeeds with no warnings; all 28 observed input/output pairs of Appendix A and
Appendix B are reproduced **byte for byte** (`tests/spec_fixtures.rs`, fixtures extracted from the
spec by `tools/fixtures.py`).

How the crate was produced

* `tools/gen.py` generates `src/ast.rs` (from the `type`/`enum` blocks of `loom.loom`),
  `src/lexer_keywords.rs` (from the keyword list of spec section 2) and the large verbatim Rust text
  blocks under `src/codegen/templates/` (cut out of Appendix B by line range, with the example names
  replaced by placeholders). Everything else is hand-written.
* Layout: `src/lib.rs` (entry points), `error.rs`, `ast.rs`, `lexer/`, `parser/` (`mod.rs`, `expr.rs`,
  `items.rs`, `being.rs`), `checker/` (`mod.rs`, `infer.rs`, `bio.rs`, `smt.rs`), `codegen/`
  (`rust.rs`, `rust_items.rs`, `typescript.rs`, `openapi.rs`, `schema.rs`, `wasm.rs`, `misc.rs`,
  `names.rs`), `project.rs`, `lpn.rs`, `main.rs`.

Each entry below is a place where the spec was insufficient, contradictory or ambiguous, and what
was assumed. "GAP" = not specified, invented; "AMBIG" = two readings, one chosen; "CONTRA" = the spec
contradicts itself.

## 1. Public API surface and module layout

1. GAP. Only `src/lib.rs`, `src/error.rs`, `src/lexer`, `src/project.rs`, `src/runtime` and
   `ast::Module` are named. Module paths of the parser, checkers and emitters are not given.
   Assumed `loom::parser::Parser`, `loom::checker::*`, `loom::codegen::{rust, typescript, openapi,
   schema (alias json_schema), wasm, mermaid, simulation, neuroml}`; every checker and emitter is also
   re-exported from the crate root.
2. GAP. The parser's API is not given. Assumed `Parser::new(&[(Token, Span)])` and
   `parse_module(&mut self) -> Result<Module, LoomError>` (single error; `loom::parse` wraps it in a
   one-element vector).
3. AMBIG. Stage list names four stages without the `Checker` suffix (`Teleos`, `Safety`,
   `Randomness`, `Stochastic`). Implemented as `TeleosChecker`, `SafetyChecker`, `RandomnessChecker`,
   `StochasticChecker` with type aliases `Teleos`, `Safety`, `Randomness`, `Stochastic`.
4. AMBIG. "`NeuroMLEmitter::emit(&Module)`" could be an associated function or a method. All emitters
   are unit structs with `new()` and `emit(&self, &Module)`.
5. GAP. "SMT bridge (public checker API)" gives only `SmtStatus`/`SmtVerification`. They live in
   `ast.rs` (re-exported from `checker`); `checker::verify_contracts(&Module) -> Vec<SmtVerification>`,
   `checker::smt_stage` and a `SmtBridge` wrapper are invented. Status is `Counterexample(..)` or `Skipped`.
6. GAP. `checker::Stage`, `compile_stages()`, `run_stages()` (the pipeline machinery) are invented names.
7. GAP. `DistributionFamily` field names come from the comments (`{mean, std_dev}` ...) and are
   implemented as struct-like variants; `Unknown(String)` is a tuple variant.
8. GAP. Integer widths that the spec does not annotate default to `i64`.
9. `src/runtime` (BIOISO runtime, SQLite) is declared out of scope by the spec: `loom runtime ...`
   only parses its options and exits 1 with an explanatory message.

## 2. Lexer

10. GAP. Unterminated string literal: reported as `unexpected character(s): "\""` for the quote, then
    lexing continues after it.
11. GAP. Unterminated block comment `{- ...` silently consumes the rest of the input.
12. GAP. An integer literal that overflows i64 is reported as an `unexpected character(s)` lex error.
13. AMBIG. `inline` not followed by `{` is an ordinary identifier.
14. GAP. Token source text (used for "free text" fields) is reconstructed from the token, not sliced
    from the source: floats are re-printed with Rust `{}` (plus `.0` when integral), so `0.50` becomes `0.5`.

## 3. Parser

15. AMBIG/quirk avoided. `domain:` labels "continue while the next token is a name-like token"; with
    set (b) that would swallow a following `type`/`store`/... item keyword. Label collection stops at
    keywords that start an item, at `symbiotic`/`niche_construction`/`correctness_report`, and at any
    identifier followed by `:`.
16. GAP. Header: spec lists a mandatory order; implemented exactly that order (each part optional).
    Annotations directly after the header belong to the module, later loose `@` lines to the next `fn`
    (or, before `annotation`, become its meta-annotations).
17. GAP. `test` names: an identifier or a string literal is accepted.
18. GAP. Spans. Rule used (consistent with every observed offset): a node spans from its first token to
    the token current when it is finished; block constructs closed by `end` take the span *before*
    consuming that `end` (so they end at the `end` token); `BinOp` starts at the operator token
    (required by A.7 `63..70`); parse errors carry the span of the offending token (last token at EOF).
19. GAP. Type declarations accept and discard `<T, ...>` after the name (no `type_params` field exists).
    `type N = T where ...` is accepted for any base type expression, not only a bare name.
20. GAP. A record literal is recognised only when `{` is followed by `name :` or `}` (in addition to the
    spec's "preceding expression is a plain identifier"); record parsing is also suppressed in a `match`
    subject.
21. GAP. `let x: T = v` (type annotation) is accepted and the annotation dropped. `||` in primary
    position is a zero-parameter lambda. A `for` body is exactly one expression.
22. GAP. Because there are no statement separators, an expression followed on the next line by `(`,
    `[`, `-` or `{name:` continues the previous expression. Assumed to be the reference behaviour too.
23. GAP (large). Concrete syntax of the fn clause blocks is not given. Assumed tolerant
    `key: value ... end` blocks: `separation:` (owns/disjoint/frame/proof), `gradual:` (input_type/
    boundary/output_type/on_cast_failure/blame), `distribution:` (`family:` or `model:` `Name(args)` plus
    named parameters), `process:` (`[Kind]` then kind/always_positive/martingale/mean_reverting/
    long_run_mean/rate/integer_valued/states), `timing_safety:` (constant_time/leaks_bits/method),
    `degenerate:` (primary/fallback/equivalence_proof); `termination: ident`, `proof: ident` are single
    lines. `handle comp with Eff.op(args) -> k: <opaque tokens> end ... end`.
24. GAP (large). Being blocks beyond B.5 (`learn`, `canalize`, `senescence`, `criticality`, `umwelt`,
    `resonance`, `manifest`, `migration`, `journal`, `scenario`, `boundary`, `memory`, `signal_attention`,
    `propagate`, `relates_to`, `role`, `crispr`, `plasticity`, `rewire`, `morphogen`) are parsed as
    tolerant `key: value` blocks closed by `end`, mapping keys to the AST field names. `role: x` and
    `relates_to: T [kind: k]` have no `end`. An unknown token inside a being is a parse error
    (`expected End, found ...`).
25. Observed quirk reproduced (B.5): in `evolve`, each `| strategy when <cond>` keeps only the first
    token of the condition and the search list ends at the first token that is not `|`; unknown tokens up
    to `end` are skipped (so only `gradient_descent` survives in the example).
26. AMBIG. `thresholds: convergence: .. divergence: .. end` — unclear whether `thresholds` owns an `end`.
    Blocks are flattened; if the block closed and a telos key (`guides:`, `bounded_by:` ...) follows, parsing
    continues to the next `end`. Otherwise a single `end` closes the item.
27. CONTRA. Section 3.10 says all first-class items are closed with `end`, but B.4 has
    `discipline CQRS for Users` without one and 3.10 itself writes `entity ... [end]`. `discipline` takes
    `end` only when it has parameters; `entity` takes a following `end` only when more tokens follow it.
28. GAP. Syntax of `correctness_report`, `sense`, `adopt`, `pathway`, `symbiotic`, `niche_construction`,
    `certificate`, `boundary`, ecosystem `tipping_point`/`coevolution`/`coupling` is not given; tolerant
    forms were invented (`adopt I from M`, `symbiotic [import] M kind: k`, `pathway N  a -> via -> b ... end`,
    kv blocks for the rest).
29. GAP. Store config values: tokens up to the next `key:`/entry keyword are concatenated (`90days`).
    Store entry `consumer C :: offset: earliest` and `mapreduce J map: .. reduce: .. [combine: ..] end` as
    written in 3.10.
30. GAP. Where the spec says "strictly `Ident`" for "most Name positions in first-class items" without
    listing them, item names accept any identifier or keyword (more lenient than the reference might be).
31. GAP. `expected value, got {Option<&Token>:?}` is used wherever a literal value is required
    (chain probability, aspect `order:`/`max_attempts:`, `autopoietic:`, empty certificate value).

## 4. Checkers

32. GAP (large). InferenceEngine internals. Assumptions: free lower-case body identifiers (not local,
    not call targets, not `todo/result/self/default`) are bound to the parameter types in first-appearance
    order **only when their count equals the parameter count**; literals have types Int/Float/String/Bool/
    Unit; unknown identifiers, calls to unknown functions, field accesses, records, tuples, lambdas, `for`,
    `?`, index and inline blocks are fresh type variables; a call to a module fn has its declared return
    type; arithmetic unifies both operands, comparison unifies and yields Bool; integer literals are
    numerically polymorphic (needed because `-x` desugars to `0 - x`); match arms are unified; the body is
    unified with the declared return type (or with the inner type of a declared `Effect<..>` unless the
    body itself is an `Effect`); fn type parameters, type aliases, refined types and propositions unify
    with anything. `function ... is declared to return ...` is never produced (no trigger is documented).
33. AMBIG. Category checker: for a monad with fewer than 3 laws both the "must declare 3 laws" error and
    one error per missing law are reported.
34. GAP. `function f requires aspect a ...`: triggered by a fn annotation `@aspect(Name)`.
35. GAP. TypeChecker accepts a `with` dependency that matches either a `requires` name or its type name.
36. GAP. RefinementChecker "known type": builtins (`Int Float String Str Bool Unit Nat ...`) plus
    declared types; skipped when the module has imports. Allowed predicate forms: binary operators,
    literals, identifiers, field access.
37. AMBIG. EffectChecker tier rule implemented literally (a fn's tier is pure unless `@pure`/`@tier`
    says otherwise, so calling an `@irreversible` fn from a fn without tiers is an error). Transitive effect
    lists are sorted alphabetically.
38. GAP. TypestateChecker treats only `index(S') == index(S) + 1` as valid (same-state is invalid).
39. GAP. TemporalChecker: `never: A transitions to B` fails when some lifecycle has `A -> B` adjacent;
    `precedes: A before B` fails when both are states of a lifecycle and A is not earlier.
40. GAP. SafetyChecker's "open-ended utility terms" list is invented: maximize, maximise, unbounded,
    unlimited, infinite, at all costs, at any cost, forever.
41. GAP. Wording invented for StoreChecker rules the spec only paraphrases (KeyValue, Graph "at least
    one node", TimeSeries, Vector, Columnar, Snowflake dimension, Distributed, DistributedLog).
42. GAP. No rules are documented for SeparationChecker, CurryHowardChecker, EffectHandlerChecker,
    Randomness, Stochastic, UmweltChecker, ResonanceChecker, SemiosisChecker, MinimalChecker,
    JournalChecker, ScenarioChecker, EvolutionVectorChecker, EntityChecker: they always pass. No
    `[warn]`/`[hint]`/`[info]` advisory text is documented, so none is produced.
43. GAP. Span carried by most checker errors is not documented; the enclosing item's span is used.
44. SMT stage: z3 is never invoked (the spec says its result never changes the outcome); the two-line
    "z3 not found" notice is printed once per process when no `z3` is on PATH.

## 5. Rust emitter

45. GAP. Text of opaque-stub placement: emitted after the `use self::Enum::*;` lines, each preceded by a
    blank line. fn type parameters and unit labels are never stubbed.
46. GAP. Aspects: B.4 shows no text for an aspect; nothing is emitted and no advice is woven.
47. GAP. `implements` block layout (one-line method bodies) and the fn-level comment blocks for
    separation/gradual/distribution/timing_safety/termination/proof/degenerate/stochastic/Dafny are
    invented (`// LOOM[...]` comments).
48. GAP. Text for items with no observed output (functor, monad, certificate, annotation decl,
    correctness_report, pathway, symbiotic, adopt, niche_construction, sense, boundary,
    messaging_primitive, telos_function, entity, intent_coordinator, non-CQRS disciplines, stores other
    than Relational/KeyValue) is a short invented `// LOOM[kind]: ...` comment or struct translation.
49. GAP. Being text beyond B.5 is invented: numeric regulate bounds (`0.1_f64`), search strategies other
    than `gradient_descent`, morphogen/crispr/plasticity methods, fns of a `function:` block (emitted
    after the impl as ordinary fns). `form:` types are not emitted (as in B.5).
50. GAP. Convergence thresholds: without `thresholds`, TLC constants are 0.8 / 0.3 and the drift
    classifier uses `1 - x` formatted `{:.3}` (matches the observed `0.200` / `0.700`); with thresholds the
    same formulas are applied to the declared values.
51. GAP. `for_all(|x: T| pred)` "edge-case assert block" is not shown; emitted as an ordinary call.
52. GAP. Fn doc line for an annotation without argument: `/// @key`. `#[deprecated]` is placed after the
    doc lines; effect-tier comments after it.
53. GAP. Ensure clause when the body is not a stub but no ensure mentions `result`: the assertion is
    emitted with the unbound condition before the final expression.
54. GAP. Kani type for `Float<unit>` parameters is `f64`.
55. GAP. Usecase with several acceptance criteria: tests separated by one blank line. Property edge cases
    for Bool use `proptest::bool::ANY` in the proptest strategy.
56. GAP. `compile_runtime` "runtime scaffold" has no specified text: the Rust output followed by
    `// LOOM[runtime...]` comment lines.
57. Condition rendering applies the documented textual rewrites literally (so an identifier containing
    `not ` or the substring `result` is rewritten too, as the spec describes).

## 6. Other targets

58. TypeScript: everything in A.16 is reproduced. Invented: refined-type `makeN` body, lifecycle/
    interface/implements/usecase/being/ecosystem text, flow-label brand types, `match` IIFE conditions,
    `let ... in` IIFE, `for` loop, inline-rust placeholder.
59. OpenAPI: A.16 reproduced. Invented: `parameters` array for path parameters, 404/403/400 response
    descriptions (`Not found`, `Forbidden`, `Bad request`), `x-lifecycle`, `x-security-labels`, `x-beings`,
    `x-ecosystems` layouts, `summary` = `describe` when present. A verb prefix must end at a word
    boundary (`_`, end or upper-case letter). Two fns with the same path and verb: first wins.
60. JSON Schema: type aliases also get a `$defs` entry; empty `$defs` renders as `{` newline `}`.
61. WASM: CONTRA. Section 12 says all unsupported items are collected; A.16 shows only the
    `close_account` error. All errors are collected in item order (enum, then fns).
62. Simulation / NeuroML: only the empty-module text is observed; class, cell and network templates are
    invented.
63. Mermaid: as specified; sequence arrows print the type in Loom syntax.

## 7. CLI, manifest, LPN

64. Mermaid targets in the CLI re-run `parse` first so lex/parse errors are printed in the standard
    `path:start:end: Kind: Display` form.
65. `loom verify --tla`: file contents are the constants with the 4-space module indent removed; TLC is
    invoked as `tlc <file>` when present.
66. LPN: only the instruction grammar of section 16 is implemented (`LpnParser::parse_str_lenient`,
    `LpnExecutor::new(dir).execute_all`); `EMIT` and `CHECK` run the compiler, every other instruction is
    reported as skipped. Type shapes (`LpnInstruction`, `LpnResult`, `LpnStatus`) are invented.
67. `ProjectManifest::from_str` is an inherent method; the `<toml error>` text is that of the `toml`
    crate version resolved at build time.

## 8. Notes on the spec itself

* A.16's quoted WASM offset (`2269..2371`) does not match the byte offsets of the source as printed in
  the spec (`2211..2310` with LF line endings); the difference equals the number of preceding lines, so
  the reference run evidently used a CRLF file.
* A.17 shows only output (no input); it is covered by an own test with an equivalent lifecycle.
