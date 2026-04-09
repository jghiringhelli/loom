# ADR-0002: Recursive-Descent LL(2) Parser

**Date**: 2026-04-08  
**Status**: Accepted

## Context

Loom needs to parse a novel biological DSL with constructs that have no
equivalent in mainstream languages (being, telos, regulate, evolve, memory,
ecosystem). The parser must:
- Give precise, actionable error messages at the token that caused the failure.
- Remain readable enough that adding new constructs (M1–M112+) doesn't require
  understanding a grammar DSL.
- Stay self-contained — no build-time code generation dependency.
- Support LL(2) lookahead for disambiguation (e.g. `type Name:` vs `type Name<T>:`).

## Decision

Hand-written recursive-descent LL(2) parser backed by a `&[(Token, Span)]`
slice. Each grammar production maps directly to a `fn parse_*` method on
`Parser<'src>`. The parser struct is split across domain submodules
(`being.rs`, `items.rs`, `types_parser.rs`, `expressions.rs`) to keep each
file under 2000 lines while sharing `impl Parser` methods.

## Alternatives Considered

| Option | Reason rejected |
|---|---|
| **nom / chumsky** (parser combinator) | Combinators produce opaque error messages; learning curve for contributors; lifetime complexity with Loom's span tracking |
| **pest / LALR grammar** (grammar DSL) | Requires build-time codegen; grammar files diverge from the AST; harder to embed rich span diagnostics |
| **Tree-sitter** | JavaScript grammar definition; runtime dependency; designed for syntax highlighting not semantic compilation |

## Consequences

- **Easier to extend**: adding a new keyword means adding a `parse_*` method and
  wiring it in `parse_item()` or `parse_being_def()`. No grammar file to update.
- **Precise errors**: `expect(Token::End)` failure reports the exact line/column
  where the parser lost track.
- **More boilerplate**: each construct needs explicit parsing code. Mitigated by
  the submodule split (each file < 2000L) and helper methods (`expect_any_name`,
  `parse_value_as_string`, etc.).
- **LL(2) limitation**: constructs requiring more lookahead must be restructured.
  In practice Loom's grammar has been designed to avoid this.
- **AI-aware**: future sessions adding grammar constructs should add to the
  appropriate submodule (being constructs → `being.rs`, top-level items → `items.rs`).
