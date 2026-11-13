# Derivation log — ALX-R2 run3

Inputs: `input/loom.loom` (self-spec) and `input/language-spec.md` only. Nothing else was read.
Each entry is a place where the two files were insufficient, contradictory or ambiguous, with
the assumption made. "R…/G…" refer to sections of `loom.loom`; "LS §…" to `language-spec.md`.

## A. Lexer / tokens

1. **Token shape contradicts itself.** The `Token` type is `{kind, text, span}`, the `lex` fn says
   "(Token, Span) pairs", G2 says `Vec<Token>`, R7.4 says "token layout is free". Assumed
   `struct Token { kind, text, span, line }`. The extra `line` field is required because bodies
   are line-oriented (R3) while whitespace/newlines are "skipped (not emitted)" and the parser
   only receives tokens (no source text).
2. **TokenKind is incomplete for the documented syntax.** No variant exists for `=` (let, type
   definitions), `=>`, `?`, `&`, or for the raw text of `inline { ... }`. Added `Assign`,
   `FatArrow`, `Question`, `Amp`, `InlineRust`. `Lt`/`Gt` vs `LAngle`/`RAngle` cannot be
   distinguished lexically: `<`/`>` always lex as `LAngle`/`RAngle`; `Lt`, `Gt`, `Newline`,
   `Whitespace`, `Comment`, `Error` are never produced.
3. **`inline { rust }` cannot be tokenised with Loom rules** (Rust text contains `'`, `#`, `;`…).
   Assumed: the lexer captures the balanced-brace block after the word `inline` verbatim
   (skipping Rust string literals when balancing) as one `InlineRust` token.
4. **Which characters are a LexError** is only given by one phrase ("unexpected character").
   Assumed: anything outside identifiers, numbers, strings and the operator set above
   (e.g. `$`, `#`, `~`, `` ` ``, `'`, `^`, `\`). All lex errors are collected.
5. Keywords used as names (`signal`, `threshold`, `limit`, `from`, `to`, `test`, `type` …) are
   everywhere in the examples (`threshold: Float<mv>` in `matter:`). Assumed: wherever a name is
   expected any word-shaped token is accepted.
6. `not`/`true`/`false` kinds: `not` -> `Not`, booleans -> `BoolLit`, lone `_` -> `Underscore`.

## B. Parser / AST

7. **Body line splitting is unspecified** when tokens carry no newlines (the spec only says
   collectors stop at certain keywords). Assumed: a logical line ends at a source line break
   unless brackets are open, the next line starts with a binary operator / `|>` / `.`, or the
   previous token is a binary operator / `=` / `,`. `match … end` is folded into the line it
   starts on (`let z = match y { … }`).
8. **`FnDef.body` for `inline` bodies**: unspecified. Assumed `body = []`, `inline_body =
   Some(text between the braces, trimmed)`. `todo` -> `body = ["todo"]`.
9. **`let x = e in` followed by a new line**: stored as its own line `let x = e in`; emitters
   flatten `let … in` chains into statements.
10. **Optional `end`s are ambiguous** (R3: refined `type … where p` "end optional"; `test name ::
    expr` vs LS §8.4 `test … end`; being `telos: "t"` + `end`). Assumed:
    * refined type: an `end` on the same line as the predicate is consumed; an `end` on a later
      line is consumed only if it is not the last token of the file;
    * test: a following `end` is consumed unless it is the last token of the file (then it closes
      the module);
    * being `telos:` with no sub-fields: a following `end` is consumed; if that leaves the module
      one `end` short at EOF the module is still accepted.
11. **`function:` block**: LS §14.2 EBNF says signatures only (`fn_sig+`), R3 and LS §14.10 say
    full definitions with bodies. Both are accepted: after a signature, `fn` or a single `end`
    not followed by `end`/`fn` means "signature only".
12. **Annotation ownership at module level** (R3 "before the first definition belong to the
    module" vs parser comment "merged into the next FnDef"). Assumed: before the first
    definition, algebraic/REST keys (`idempotent`, `exactly-once`, `at-most-once`, `commutative`,
    `associative`, `monotonic`, `pure`, `trace`, `method`, `path`) directly before a `fn` go to
    that fn and all others to the module; annotations directly before `being` go to the being;
    after the first definition they go to the next `fn`/`being`.
13. **Annotation argument forms** (`@slo(p99=200ms, …)`, `@version(3)`): only the single-string
    case is specified. Other arguments are stored as their source text. The `(` must be adjacent
    to the name.
14. **`describe:` after `::`** is "not recognised": it is consumed and discarded (not treated as a
    body line).
15. **Parenthesised function types** are listed as unimplemented, yet LS §4.1 still shows
    `(A -> B) -> List<A>`. Accepted leniently as `TypeExpr::Fn`. `()` parses as `Base("Unit")`.
16. **Effect tiers lose their effect association** (`effect_tiers` is only the tier words in
    order). The Rust comment `// effect-tier: IO -> irreversible` pairs tier *i* with effect *i*.
17. **G10 parameter-name inference details**: the keyword list is not given (assumed: let in
    match if then else true false not as for todo end with fn type enum inline require ensure
    and or where when of result `_`). Additionally skipped: identifiers after `.` (field names),
    after `:` (lambda type annotations), and any name bound by a `let` anywhere in the body
    (the spec only says "skip the LHS"; using a later occurrence of a local as a parameter name
    would be wrong). Lambda parameters and match bindings are *not* excluded (spec silent).
18. **`TestDef.body`**: one String; multi-line bodies are joined with `\n`; `inline { x }` is
    stored as the text `inline { x }`.
19. **Ecosystem `signal`** without `payload:`/`end` is not specified: accepted; an `end` on the
    same line is consumed.
20. **`learn:` / `rewire:` blocks** (LS §14A) have no AST: skipped up to their `end`.
    `plasticity` accepts `update:` as a synonym of `rule:` and ignores `learning_rate:`.
    Epigenetic accepts `trigger:` and morphogen `gradient:` as synonyms of `signal:` (R0.10).
21. Unknown plasticity rule / search strategy -> ParseError (not specified).
22. Annotations inside a being after `autopoietic:` are dropped, as LS §14.12 states.
23. `FnTypeSignature.return_type` is a plain `TypeExpr` (not boxed) as written in the spec type.
24. Tokens after the module's closing `end` are a ParseError (not specified).
25. All AST types additionally derive `PartialEq`; `LoomError` additionally derives `PartialEq`.

## C. Checkers

26. **Inference strength** (R5 "conservative: when unsure, accept"). A mismatch is reported only
    between known primitives (Int/Float/Bool/String). Integer literals unify with Int and Float;
    Int-vs-Float arithmetic is treated as unknown; a non-primitive declared return type is never
    a mismatch (otherwise LS §4.3 `transfer` would fail). Only top-level module fns are checked.
27. **TypeChecker**: "unknown lowercase bare type names" — names declared in the module and fn
    type parameters are in scope; the argument of `Float<…>`/`Int<…>` is always a unit label.
    Missing interface method -> `TypeError` (R2 says General, R4.2 says TypeError/General).
28. **Exhaustiveness**: which enum a match ranges over is not given (bodies are untyped text).
    Assumed: the first enum (module enums, being `form:` enums, then Option, Result) containing
    all — else any — of the variant names used by the arms. Span = the fn span.
29. **Effects**: callee detection is textual (`name(` with a word boundary, not after `.`); the
    pipe form `x |> f` is not treated as a call. Tier rule generalised to rank
    pure < reversible < irreversible; a caller without tiers is never a tier error.
30. **Units**: implemented exactly as the R4.6 heuristic (two different labels among the
    parameters and ` + `/` - ` in the body).
31. **Typestate**: an unknown state name is also reported as "invalid lifecycle transition".
32. **InfoFlow**: a return type is "protected" if any type name in it is labelled non-public.
    Labels are looked up on every type name nested in a parameter type.
33. **Teleos**: regulate-variable-in-matter and `on_exhaustion` validity are *not* enforced
    (R4.10 says the oracle does not exercise them). Ecosystem members are not required to be
    declared beings (LS §14.10 uses undeclared members). LS §14.8 "inverted bounds" cannot be
    checked because bounds are identifiers.
34. **Safety messages** carry the being name as a suffix after the exact required sentence.
35. Pipeline order: R-section order (inference first), not the different order in LS §15.3.
    Being `function:` fns are not run through the fn-level checkers.

## D. Emitters

36. **Rust privacy attributes**: the literal `#[loom_pii]` is required but is not valid Rust
    without a proc-macro; emitted inside a comment line (`// #[loom_pii]`) so the text is present
    and the output stays compilable. Same for pci/hipaa/gdpr; `@never-log` -> `// NEVER LOG…`.
37. **Enum match arms**: both options of G12/R6.1 were allowed; chose `use self::Enum::*;` after
    each enum and bare arm patterns (so the literal `match shp { Circle(r) => 1, … }` appears).
38. **`as` shape**: M10 says `((x) as f64 * y)`, R6.1 says `((x as f64) * y)`. Emitted
    `((x) as f64 * y)` (contains `as f64` and `*`).
39. **Contracts** use the raw predicate text inside `debug_assert!((…), "…")`; `result` is
    replaced by `_loom_result` in ensures. Effectful fns wrap the final expression in `Ok(…)`.
40. **implements**: fns matching interface methods are emitted only inside
    `impl Iface for <Mod>Impl` (with `&self`); when the interface is not declared in the module
    all fns go into the impl. Trait methods take `&self`.
41. **Being/ecosystem method bodies and signatures** beyond the required substrings are invented
    (e.g. `regulate_x(&self, value, lo, hi) -> bool`, `TELOMERE_LIMIT` const, signal unit
    structs, `coordinate()` returning the routing table). `///` is used only for describes and
    annotations so that "no describe -> no `///`" holds; everything else uses `//`.
42. **`for_all` tests**: edge-case array `[0, 1, -1, i64::MAX, i64::MIN]`, one nested loop per
    lambda parameter. Plain test bodies become `assert!(expr);`.
43. **TypeScript**: Zod schemas (mentioned once in the header) are not emitted — R6.2 does not
    describe them. `match` becomes an IIFE with `if` chains; `Option<T>` -> `T | null`;
    `Result<T,E>` -> `T`; `provides` -> `interface <Mod>Api`; inline bodies throw.
44. **OpenAPI**: default path is `/<fn name>`; no path parameters / pluralised resources / error
    status mapping from LS §11 (R6.3 says only `@path` is tested). GET/DELETE parameters are
    query parameters. Verb prefixes must be followed by `_`, an upper-case letter or the end of
    the name. `x-telos`, `x-beings`, `x-ecosystems`, `x-lifecycle` are objects keyed by name;
    `pii-fields` includes `@gdpr` fields.
45. **JSON Schema**: property entries are written `"name": {compact schema}` inside otherwise
    compact objects (the spec text mixes both); `$ref` siblings carry the `x-…` extensions;
    beings are added to `$defs` with `x-being`, `x-telos`, `x-bounds`, `x-telomere`.
    `compile_json_schema` describe line says draft-07; R0.6 (2020-12) wins.
46. **WASM**: String/other parameter types map to `i32`; `Unit` has no result; float arithmetic
    uses `f64.*`; calls to module fns emit `call $name`; `todo`/`inline`/unknown shapes emit
    `unreachable`; non-final expression statements are followed by `drop`. Merely *declaring* an
    enum is not an error; using a variant name or `match` in a body is.
47. **Simulation**: only declared beings are added to an ecosystem's schedule (undeclared members
    become comments); no Model class is emitted when there is no ecosystem.
48. **NeuroML**: `ReinforcementLearning` -> `rule="reinforcement_learning"` (not specified).
    `<morphology>`/`<biophysicalProperties>` are written without attributes as quoted in R6.7.

## E. Tooling

49. **CLI**: besides R7.1, `--target=…`/`--output=…`, `-t`, `--check`, `loom lsp` (stdio server)
    and `loom help` were added. `loom build` without an argument uses `loom.toml`.
50. **build_project** returns the original compile errors (variant preserved) rather than
    wrapping them with the file name; read failures are `General` errors with span 0..0.
    Nothing is written to `lib.rs` when any file fails.
51. **Manifest**: multi-line `modules = [` arrays are accepted; keys outside `[project]` ignored.
52. `ProjectManifest` also implements `std::str::FromStr` (the inherent `from_str` is the one in
    the spec).
53. Extra public surface not in the spec: `loom::expr` (expression mini-parser used by emitters
    and checkers), `loom::parse`, `loom::check`, `loom::parse_and_check`, `InferenceChecker`,
    `ExhaustivenessChecker`, `AlgebraicChecker`, free functions `check_*`/`emit_*`,
    `checker::check_all`, `LoomError::lex/parse/message`.
54. `logos` is declared (R1) but unused (hand-written lexer).
