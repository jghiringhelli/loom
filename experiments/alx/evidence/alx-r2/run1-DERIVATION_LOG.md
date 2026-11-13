# Derivation log (ALX-R2 run1)

Inputs used: `input/loom.loom` and `input/language-spec.md` only. Own tests: `tests/*.rs` (smoke only).
Each entry: where the spec was insufficient / contradictory / ambiguous, and what was assumed.

## Lexer / tokens
1. `Token` layout "free" but the parser needs line boundaries and the spec says whitespace/newlines are not emitted. Assumed: `Token { kind, text, span, line }` (extra `line` field); no `Newline` tokens.
2. TokenKind list has both `Lt/Gt` and `LAngle/RAngle`, `Eq` and no assignment token. Assumed `<`/`>` lex as `Lt`/`Gt`, `==` as `Eq`, `=` as extra `Assign`; `=>` as extra `FatArrow`; added `For, Where, As, In, Todo, Inline, RawBlock, Question`. Keywords are matched by text in the parser, so words like `signal`, `from`, `limit` still work as plain names.
3. `inline { ... }` content contains arbitrary Rust; the lexer scans it as one `RawBlock` token (brace matching, string aware) so "unexpected character" is never raised inside it.
4. Unknown character -> `LexError` containing "unexpected character". `!` is a valid token (`Bang`) so `this is not valid loom source !!!` fails in the parser (ParseError), not the lexer.

## Parser
5. `describe:` vs annotations on module/fn/being: annotations before the first definition go to the module; afterwards annotations preceding `fn`/`being` go to that item. (R3 gives both rules; they collide for a first-item fn with a preceding annotation - module wins.)
6. Optional `end` ambiguity (refined type `where`, `test`): an `end` is consumed by the type/test only if more tokens follow it; otherwise it belongs to the enclosing module. Tests without `end` stop at the next item keyword at line start.
7. `telos:` inside a being: always consumes one trailing `end` if present (language-spec 14.11 EBNF shows two `end`s, the examples show one). Ecosystem `telos:` is a plain string.
8. `module Bad\nfn broken :: this is bad\nend` must be a ParseError although lowercase type names are accepted by the parser. Assumed: two consecutive plain identifiers on the signature line after the type -> ParseError.
9. Annotations after `autopoietic: true` inside a being are parsed and dropped (14.12 says "ignored").
10. `regulate` `target:`/`bounds:` starting with a digit -> ParseError (14.5 "numeric literals are not accepted"). The "inverted bounds" check therefore only fires for programmatic ASTs.
11. `plasticity:` also accepts `update:` (-> rule) and `learning_rate:` (ignored); `learn:`/`rewire:` blocks are accepted and skipped (14A, no AST slot).
12. `RegulateBlock.response: Vec<String>` format not given. Assumed one string `"<pattern> -> <action>"` per clause.
13. `TestDef.body` format not given. Assumed source text; `inline {X}` bodies stored as `inline { X }`.
14. `FnDef.body` for `todo` is `["todo"]`; for `inline { }` body is empty and `inline_body` is `Some`.
15. Body line continuation (operators / open brackets / leading `|>`) joins physical lines into one stored string; token text is re-joined with a single space wherever the source had whitespace.
16. Parenthesised function types `(A -> B) -> C` are accepted (language-spec 4.1 uses them although section 0 says they are not accepted).
17. `type Pair<A,B> =` / `enum R<T,E>` generics are parsed leniently (spec says not accepted but gives no required error).
18. Param-name inference (G10): order is requires, ensures, body; additionally skipped: names after `.` and `|>`, lambda params, `for` variables, match-arm pattern bindings. (A require-first order can swap parameters; the spec literally says to scan contracts then body.)

## Checkers
19. Pipeline order per loom.loom; implements-conformance is run inside `TypeChecker::check` and again as a final module-level stage (harmless duplication), DI last.
20. Inference: Int/Float treated as mutually compatible (lenient: "when unsure, accept"); only Bool/String/numeric conflicts error. `Float<unit>` compatible with Int/Float.
21. Language-spec section 12 example fails the info-flow rule as literally specified in R4.9 (secret param, unlabelled return). Followed R4.9.
22. Teleos: "regulate variable must be a matter field" and "on_exhaustion must be a known name/fn" and "ecosystem members must be declared beings" are NOT enforced (the spec says the oracle does not exercise them; the 14.10 example has undeclared members).
23. Effect tiers: `effect_tiers` is a flat list, so which effect owns a tier is lost; the `// effect-tier: E -> tier` doc line pairs tiers with effects by position. Caller tier = most severe tier word; only reversible-caller/irreversible-callee errors.
24. `Effect<[Pure], T>` treated as "no real effects".

## Emitters
25. Rust: enum patterns are qualified (`Shape::Circle(r) =>`) when the variant name is unique across the module's enums. Effect fn bodies are not wrapped in `Ok(..)`. `Float<usd>` -> `Usd`. Lifecycle states emitted as unit structs; typestate args erased.
26. Rust `ensure` uses `_loom_result`; requires/ensures use the raw predicate text (no translation).
27. Rust lambda bodies are emitted without outer parentheses; every other binary expression is parenthesised.
28. TypeScript: JSDoc layout, `Usd` branded type used for `Float<usd>` in signatures, flow labels skipped for built-in type names, match compiled to an IIFE. No Zod schemas (loom.loom mentions them, appendix R6.2 does not).
29. OpenAPI/JSON Schema: hand-assembled JSON with pretty structural keys and compact schema fragments, as in R6.3/R6.4. Resource path inference is simply `/<fn name>` (or `@path`). Extension key shapes for `x-lifecycle`, `x-beings`, `x-ecosystems`, `x-security-labels` are my own (only the key names are specified).
30. WASM: Float -> f64 (spec lists it but no instruction names); unsupported bodies -> `unreachable`.
31. Simulation / NeuroML bodies beyond the listed substrings are free-form.
32. CLI output extension for openapi is `.openapi.yaml` (content is JSON, per R0.6).
33. LSP server implements only `initialize`, `initialized`, `shutdown`.

## Not specified at all (left minimal)
- Public names for individual checker stages beyond G4/G7 (`check_all`, `check_implements`, `check_dependencies` are extras).
- Exact diagnostics for tokens-after-module (`unexpected tokens after module end` ParseError).
