# Derivation log (ALX-R2 run2)

Inputs used: only `input/loom.loom` and `input/language-spec.md`. Where the two disagree the
appendix "REFERENCE BEHAVIOUR" of loom.loom was followed (as both files instruct).

Build: `cargo build -j 2 --target-dir target` (stable-x86_64-pc-windows-gnu). Own tests:
`tests/spec_tests.rs` (40 tests, all pass). Generated Rust for sample modules was compiled with
`rustc` and its generated `#[test]`s pass.

## Gaps, ambiguities, contradictions and the assumption taken

### Crate layout / API
1. **Token layout.** Spec lists `TokenKind` variants but there is no `=` (assignment), `=>`, `?`
   token, and `Eq` is ambiguous. Assumed `Eq` = `==`; added extra kinds `Assign` (`=`),
   `FatArrow`, `Question`, `InlineCode` (R7.4 says token layout is free).
2. **logos.** Optional per R1; a hand-written lexer is used. `logos` is still declared in
   Cargo.toml (unused) to match the R1 dependency list.
3. **Newlines.** `Newline` is in `TokenKind` while "whitespace is skipped". Assumed newline tokens
   ARE emitted (the grammar is line based for contracts, invariants, bodies).
4. **`inline { ... }` lexing.** Inline Rust may contain characters Loom cannot lex (`&`, `#`, `'`).
   The lexer captures the brace-balanced contents verbatim as one `InlineCode` token.
5. **Hyphenated annotation names** (`@never-log`) are lexed as one identifier only directly after `@`.
6. **`FnTypeSignature.return_type`**: spec type says `TypeExpr` (not boxed); implemented unboxed.
7. **PartialEq.** Spec says only some AST types derive PartialEq; all AST structs derive it
   (superset; cannot break tests).
8. **Extra public items** not in spec but harmless: `loom::parse_source`, `loom::check_source`,
   `parser::parse`, `parser::expr` submodule (body-text expression parser), `checker::run_all`,
   `checker::check_implements`, `checker::check_dependencies`, `InferenceChecker`,
   `ExhaustivenessChecker`, `AlgebraicChecker`, `TeleosChecker`, `codegen::wasm::WasmEmitter`,
   `lsp::diagnostics_for`, `lsp::run_stdio`, `LoomError::{lex, parse, message}`.
9. **R1 source layout** lists fixed files; `src/parser/expr.rs` was added as a submodule.

### Parsing
10. **Annotation ownership.** "Annotations before the first definition belong to the module" vs
    "annotations directly before a fn belong to it" conflict for the first fn. Assumed: module-only
    keys (author, version, decision, rationale, tag, service, environment, depends-on, slo, alert,
    ontology, prefix) seen before any item go to the module; other pending annotations attach to the
    next fn/being unless a blank line separates them from it (then module).
11. **Bodiless fns** (`fn a :: Sig` with no body/end, used in language-spec 9.3/9.4 snippets):
    accepted. An `end` right after an empty fn is consumed as the fn's `end` unless it is the last
    token of the file (then it closes the module); inside `function:` it is consumed only if followed
    by `fn` or `end`.
12. **`describe:` after `::`** is "not recognised" (R0.3). Assumed: the line is skipped silently
    (neither stored nor treated as a body statement).
13. **Refined type without `end`**: an `end` on the following line is taken as the refined type's
    `end` unless it is the final token of the file. Same rule for `test` blocks and prose invariants.
14. **Type alias `type X = Int` (no `where`, no fields)**: not in grammar; ParseError.
15. **`invariant name: "prose" end`** (form used inside loom.loom itself, not in the grammar):
    accepted, condition = the string. Allows the self-spec to parse.
16. **Match arms on one line** (`match x | A -> 1 | B -> 2 end`): supported by splitting on depth-0
    `|` once the arm has its `->` (unless a lambda follows).
17. **Telos inside a being** must be closed by `end` (R3); if a non-key token follows the telos line
    the telos block is closed implicitly.
18. **Ecosystem `telos:`** is a plain string with no `end` (R3); a being's telos has its own `end`.
19. **Plasticity**: `update:` is accepted as an alias of `rule:`; `learning_rate:` is ignored.
    Unknown rule names are a ParseError. `learn:` / `rewire:` blocks (14A) have no AST field and
    are skipped.
20. **Epigenetic `trigger:` / morphogen `gradient:`** (old names) are accepted as fallbacks.
21. **Unknown `search:` strategy** -> ParseError.
22. **Telomere `limit:`** missing or unparsable -> 0 (then rejected by teleos as "positive").

### G10 parameter-name inference
23. Spec list of skipped words is "keywords, PascalCase, call targets, result, string literals,
    let LHS". Assumed additionally: identifiers directly after `.` (field names) are skipped, and
    "keywords" = every word the lexer turns into a keyword token (including being words such as
    `signal`, `limit`, `from`, `to`) plus `in for as todo inline and or self when where`.
    Lambda parameters and match bindings are NOT skipped (spec does not say so).

### Checkers
24. **Inference.** Int vs Float operands in arithmetic are allowed (result Float); only Bool/String
    mixed with numbers is an operand mismatch. An Int body for a declared Float return IS a
    mismatch (implied by M10's "false UnificationError" example). Float<unit> is compatible with
    Int/Float. Any expression involving an Unknown operand yields Unknown (no error).
25. **Types.** Only fn signatures are scanned for unknown lowercase types (as R4.2 says); field types
    are not.
26. **Effects.** Transitive effect sets are used for the "effect not declared" check. A caller with
    no tier calling an irreversible callee is accepted (only reversible -> irreversible errors).
    Self-recursion is ignored.
27. **Info flow.** Labels inside generics/Option/tuples propagate to the outer type. Built-in
    types listed in flow labels (e.g. `Bool`) are labelled too. Note: language-spec's own complete
    example (OrderService: `PaymentToken` (secret) -> `Order` (unlabelled)) is therefore an
    info-flow error under R4.9; R4.9 was followed literally.
28. **Teleos.** Not enforced (spec says the oracle does not exercise them): regulate variable must
    be a matter field; telomere on_exhaustion must be senescence/apoptosis/quiescence/a fn;
    ecosystem members must be declared beings (language-spec 14.8 says error, but its own example
    14.10 references undeclared Synapse/GlialCell; R4.10 omits the rule). Added: numeric bounds with
    lo >= hi -> "inverted bounds" (only when both parse as numbers).
29. **Safety.** Rules 3-6 (@mortal/@corrigible/@bounded_telos requirements) apply to any being that
    carries the annotation, not only autopoietic ones. Annotations written after `autopoietic: true`
    inside a being are dropped (language-spec 14.12).
30. **Implements conformance** runs inside TypeChecker (R4.2) and again as the last module-level
    stage (R4.12); because the pipeline short-circuits, errors are reported once.
31. **Error message wording**: only the required substrings are guaranteed; surrounding text is mine.

### Emitters
32. **`#[loom_pii]`** etc. are emitted inside `//` comment lines (`// #[loom_pii]`) so the substring is
    present AND the generated Rust compiles. `NEVER LOG` likewise in a comment.
33. **Contracts**: the raw predicate text (with `not/and/or/result/as Float` rewritten) is used in
    `debug_assert!((raw), "precondition violated: (raw)")`, so the source text appears verbatim.
34. **`as` emission**: `x as Float * y` -> `((x) as f64 * y)` (M10 form) rather than R6.1's
    `((x as f64) * y)` "shape"; both contain `as f64` and `*`.
35. **Effect tiers comment**: FnDef.effect_tiers loses the effect/tier association; tiers are zipped
    with the effect list in order.
36. **Match qualification**: glob `use self::<Enum>::*;` after every enum (R6.1 allows it).
37. **implements of an interface NOT declared in the module**: all module fns go into the
    `impl X for <Mod>Impl` block (no `pub`). Matching fns of a declared interface are emitted only
    inside the impl, not also as free fns. Trait methods take `&self`.
38. **provides trait**: methods take `&self` and `arg0..` parameter names.
39. **Being Rust** derives `Default`; unit structs get a manual `impl Default` (keeps the exact
    `#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]` line). Duplicate method names
    (e.g. two plasticity blocks modifying the same field) are emitted once.
40. **Auxiliary comments** use `//`, never `///`, so "no describe/annotations -> no `///`" holds for
    plain modules. Being describe/telos use `///`.
41. **TypeScript**: module-level JSDoc always present (starts with "Module <Name>"); a `zod` import
    and `<Type>Schema` consts are emitted ("interface types + Zod schemas"); unit/flow branded types
    use `export type`; imports are placed before the namespace. Match expressions become ternary
    chains; inline Rust bodies become `throw new Error("not implemented")`.
42. **OpenAPI**: default path is `/<fn name>` (unique); GET/DELETE params become query parameters;
    `"201"` only for post; error enums named `*Error` add 404/400/403/401/409 responses by variant
    name; `x-data-protection` also has `gdpr-fields`; `x-lifecycle` placed at the document end
    (R6.3) rather than in `info` (language-spec 9.4); OpenAPI `$ref`s use `#/components/schemas/`.
    Method prefix matching requires a word boundary after the prefix (`_`, uppercase, digit or end).
43. **JSON units**: R6.4 `"x-unit":"usd"` (compact) chosen over language-spec `"x-unit": "usd"`.
    Int is `{"type":"integer"}` without the optional format. Beings/ecosystems are added to `$defs`
    with `"x-being": true` / `"x-ecosystem": true`.
44. **WASM**: non-numeric types map to `i32`; enum-use detection = any reference to a declared
    variant/enum name; unsupported shapes become `unreachable`; statement results are `drop`ped.
45. **Simulation / NeuroML**: regulate/evolve/telomere handling emitted for every being (not only
    autopoietic). ReinforcementLearning rule is written `rule="reinforcement_learning"` (unspecified).
    Morphogen `<segment>` also carries `name` and `produces` attributes.
46. **CLI**: `--target` aliases accepted (ts, wat, ...); `loom check` and `loom lsp` added; the
    `--check-only` line is `ok — no errors in `<file>``; build prints `build ok: ...`.
47. **Manifest**: multi-line arrays and trailing `#` comments are tolerated; a non-array `modules`
    or unquoted string value is an Err.
48. **build_project** writes nothing if any file fails (all errors collected first); error messages
    are prefixed with the file name.
