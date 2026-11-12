# ALX-R Phase 1 derivation log

Inputs used: `language-spec.md` and `loom.loom` only. No dependencies (hand-written lexer instead of `logos`, to stay offline).

## (a) Ambiguities / silent spots and choices made

Lexer
- `Lexer::tokenize` skips whitespace/comments; `Newline`/`Whitespace`/`Comment`/`Error` variants exist but are never emitted. Unknown characters -> `LoomError::LexError`.
- Token `text` is the raw source slice (string literals keep their quotes).
- `==` -> `Eq`, `=` -> extra variant `Assign`, `=>` -> extra `FatArrow`, `?` -> extra `Question`. `<`/`>` always lex as `Lt`/`Gt` (`LAngle`/`RAngle` never emitted; parser treats `>` and `]` as closers of type arguments).
- Keywords are lowercase words from `TokenKind` (e.g. `describe`, `signal`, `limit`, `rule`...); `Effect` is the capitalised keyword; `hebbian`/`boltzmann` lowercase. `for`, `in`, `as`, `where`, `when`, `function` lex as `Ident`. Parser accepts keyword tokens as names (e.g. field `end: Int`, `from`, `to`).
- Line information needed by the parser (contract / invariant / body line splitting) is passed through a thread-local set filled by `Lexer::tokenize` and read by `Parser::new` (the token type has no newline info). Hand-built token vectors therefore lose line info.

Parser
- `Parser::new(&[Token])`, `parse_module(&mut self) -> Result<Module, LoomError>`.
- `TypeExpr` shape: `Base(String)`, `Generic(String, Vec<TypeExpr>)`, `Effect(Vec<String>, Box<TypeExpr>)`, `Option(Box)`, `Result(Box, Box)`, `Tuple(Vec)`, `TypeVar(u32)`. Function types are `Generic("Fn", [params.., ret])`. `Float<usd>` = `Generic("Float", [Base("usd")])`.
- Effect type closes with either `>` or `]` (the spec uses both). `FnDef.effect_tiers` holds only explicitly written tiers (e.g. `"irreversible"`), not one entry per effect.
- `FnDef.param_names` per G10 (explicit `x y =>` else inferred from require/ensure/body, only when >= N names found, else empty).
- Annotation `value`: raw argument text; a single string argument is unquoted. Hyphenated names (`never-log`) are joined from adjacent tokens.
- Module-level annotations: annotations before the first definition go to `Module.annotations`; later ones are pending and merge into the next `fn`/`being`, else flushed to the module.
- `InterfaceDef.methods` holds method names only (signatures are not retained).
- `TypeDef`/`EnumDef` drop type parameters (AST has no field for them).
- Refined type: `type X = T where <pred> end` -> `Item::RefinedType`; alias without `where` becomes `RefinedType` with empty predicate. Brace-form `type X = { a: T }` needs no `end`.
- `invariant name :: expr` (single line, no `end`) and the loom.loom form `invariant name: "text" end` are both accepted.
- `telos:` inside a `being` is ambiguous (own `end` vs none). Resolved by trying "no own end" first, validating what follows (an item keyword, or `end` + EOF), then retrying with an own `end`. Sub-fields (`fitness:`, `modifiable_by:`, `bounded_by:`) always imply an own `end`. In `ecosystem`, `telos: "..."` has no own `end`.
- Block mapping where AST lacks fields: `epigenetic` accepts `signal:` or `trigger:` -> `signal`; `morphogen` `gradient:` -> `signal`; `plasticity` `update: sarsa|q_learning|reinforce` -> `ReinforcementLearning`, default rule `Hebbian`; `telomere.limit` missing -> 0; `regulate` accepts `regulate:` (no name -> empty `variable`) and bounds in `()` or `[]` (stored as inner text `"lo, hi"`).
- `learn:` / `rewire:` blocks (T4/T5) are parsed and discarded (no AST nodes in loom.loom).
- Trailing tokens after the final module `end` are ignored. Missing `end` is a ParseError.
- `spec Name` is consumed and stored in `Module.spec`.

Checkers (all return `Result<(), Vec<LoomError>>`; `SafetyChecker::check` is static returning `Vec`)
- Added names beyond the spec list: `InferenceChecker` (alias `InferenceEngine`), `ExhaustivenessChecker`, `AlgebraicChecker`, `TaintChecker` (label map). `checker::check_all` runs the 11-stage pipeline in the documented order.
- Heuristic, body-text based analysis (bodies are `Vec<String>`): inference only flags confident primitive mismatches; exhaustiveness resolves the enum from the arm names (or Option/Result); effects checks calls to module fns by name; units checks mixed-unit `+`/`-` over params; typestate requires strictly adjacent states (self-transitions rejected).
- Safety follows loom.loom rules 1-6 exactly (autopoietic needs `@mortal` and `@sandboxed`; `@corrigible` alone is NOT required to be present, although language-spec 14.8 says it is). Word-boundary matching for `maximize/unlimited/any/all`.
- Spec 14.8 "plasticity without learning_rate" is not enforced: `PlasticityBlock` has no such field.
- OpenAPI: followed language-spec (`@idempotent` on POST -> PUT; `@exactly-once` -> `x-exactly-once`), not the contradictory remark in loom.loom.

Codegen
- Ecosystem Rust module is `pub mod <snake_case_name>`. Field annotations emit literal `#[loom_pci]`-style attributes (will not compile with rustc; as spec'd). Typestate args are erased in Rust types. Effects are erased (`Effect<[..],T>` -> `T`) with a doc comment.
- Expression translation (`codegen::expr`) is a small best-effort recursive translator; unsupported bodies become `unimplemented!(..)`. Invariants emit `debug_assert!(true, "<text>")` (free variables cannot be resolved).
- WASM: effect-bearing fns are skipped by `emit`, and `compile_wasm` returns `WasmUnsupported` if any exists. Simulation emits all beings as Mesa agents (autopoietic flagged). NeuroML emits `<cell>` only for beings with plasticity; output starts with `<neuroml`.
- `project::ProjectManifest::from_str` accepts `key = value`/`key: value` lines (`name`, `modules`, `output`); format was unspecified. `build_project` writes `<stem>.rs`.
- LSP: simple utility functions (`diagnostics`, `hover`, `completions`, `document_symbols`) and in-memory `LoomLspServer`; no JSON-RPC.
- CLI: `loom compile <file> [--target ..] [-o out]` (stdout default), `check`, `lex`, `build`; errors print `error: <file>: <msg>` and exit 1.

## (b) Not implemented / known gaps
- Real Hindley-Milner inference (TypeVar unification) - only heuristic primitive mismatch detection.
- Full symbol resolution of type names (undeclared types are accepted, because the spec's own examples use undeclared types).
- Complete Rust expression translation (`find_as_outside_parens`/`param_type_map` of G10/G12 replaced by a token-based translator that qualifies variants via a variant->enum map); generated Rust is not guaranteed to compile for arbitrary programs.
- Unit-aware Rust arithmetic, regulate `response` clauses (emitted as comments), `form:` enums in OpenAPI `$defs`, AsyncAPI channels for signals.
- `learn:`/`rewire:` semantics, T-tier analysis, `solver_tier` runtime (Section 14A.6 / BIOISO) - out of compiler scope.
- LSP wire protocol.
- No test suite written (none requested).

## (c) Build status
`cargo build` (lib + bin `loom`): succeeds, a few dead-code warnings only. Sanity checks: `loom.loom` itself parses and passes all checkers; hand-made negative cases for telos, privacy, exhaustiveness, units, typestate, algebraic, unification and safety produce errors.
