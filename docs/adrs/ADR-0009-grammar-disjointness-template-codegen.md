# ADR-0009: Grammar Disjointness and Template-Based Codegen

**Date**: 2026-04-24  
**Status**: Accepted

## Context

Loom's code generator in `disciplines.rs` accumulated severe maintainability debt from
Rust's `format!` escape rules: every literal `{` in generated Rust code required `{{`,
and every `}` required `}}`. Multi-line templates became unreadable and bug-prone —
a single missing double-brace produced a silent wrong output or a cryptic compile error.

Two related design questions emerged:

1. **Code generation**: how do we write idiomatic, readable Rust template strings inside
   a Rust source file without fighting the `format!` escape grammar?

2. **Language grammar**: which of Loom's surface keywords overlap with Rust's, and is
   that intentional or accidental?

## Decision

### 1. Regex template engine (`src/codegen/rust/template.rs`)

All multi-structure code generation uses `subst(template, vars)` and `ts(template, vars)`:

```rust
// Before — unreadable, fragile:
format!("impl {name}Repository {{\n    fn find(&self, id: &str) -> Option<{name}> {{\n        ...\n    }}\n}}")

// After — readable raw string, no escaping:
ts(r#"
impl {Name}Repository {
    fn find(&self, id: &str) -> Option<{Name}> {
        ...
    }
}
"#, &[("Name", table)])
```

- Templates are raw string literals (`r#"..."#`) — no escape sequences anywhere.
- Placeholders use `{identifier}` syntax, substituted by regex globally.
- `{` and `}` in the *generated* Rust code are literal and never confused with placeholders
  because Loom placeholder identifiers never start with Rust syntax tokens like `{`, `}`, `:`.
- Unknown placeholders are left as-is (safe for partial application).

This is the **required pattern** for all new code generators. Existing `format!`-based
emitters should be migrated to `ts()` incrementally.

### 2. Grammar disjointness principle

Loom's surface keywords fall into two categories:

#### Intentionally shared with Rust (`fn`, `let`, `type`, `mod`)
These map directly to equivalent Rust concepts. A Loom `fn` IS a function; a Loom `type`
IS a type alias or struct. Sharing these keywords makes Loom's semantics immediately
legible to Rust developers and keeps the conceptual mapping unambiguous.
There is no conflict because `.loom` files are only ever parsed by Loom's parser.

#### Intentionally disjoint from Rust (Loom-specific semantic constructs)
These must **never** use Rust keywords, because they carry different semantic weight:

| Loom keyword | Domain | Rust equivalent (none) |
|---|---|---|
| `store` | Persistence algebra | — |
| `session` | Session types / linear protocols | — |
| `effect` | Algebraic effects | — |
| `process` | Stochastic process annotation | — |
| `distribution` | Probabilistic type | — |
| `separation` | Separation logic block | — |
| `timing_safety` | Side-channel safety | — |
| `termination` | Termination clause | — |
| `gradual` | Gradual typing boundary | — |
| `degenerate` | Degeneracy fallback | — |
| `aspect` | AOP advice | — |
| `telos` | Convergence / temporal logic | — |
| `require` / `ensure` | Design-by-contract | — |
| `forall` / `exists` | Predicate quantifiers | — |

These keywords are chosen to be ASCII identifiers that Rust's parser would treat as
ordinary identifiers (not reserved words), ensuring that any accidental mixing of Loom
and Rust source files produces a clear parse error rather than silent misinterpretation.

#### Rule for new keywords
New Loom keywords **must not** be any of Rust's reserved words
(`as`, `break`, `const`, `continue`, `crate`, `else`, `enum`, `extern`, `false`,
`for`, `if`, `impl`, `in`, `loop`, `match`, `move`, `mut`, `pub`, `ref`, `return`,
`self`, `Self`, `static`, `struct`, `super`, `trait`, `true`, `type`, `unsafe`,
`use`, `where`, `while`, `async`, `await`, `dyn`, `abstract`, `become`, `box`,
`do`, `final`, `macro`, `override`, `priv`, `try`, `typeof`, `unsized`, `virtual`,
`yield`).

If a Loom concept maps naturally to one of these, prefix it: e.g. `loom_type` or
choose a domain-specific synonym.

## Consequences

- `template.rs` becomes the **canonical code generation API**. Direct `format!` usage
  for multi-line code templates is an architecture violation.
- New disciplines added to `disciplines.rs` MUST use `ts()`.
- Existing emitters (stochastic, statistical, concurrency) should be migrated as
  they are touched — not all at once.
- Grammar disjointness is enforced by documentation and code review. The checker
  could optionally validate that no Loom keyword shadows a Rust reserved word.
- The `{placeholder}` convention in `ts()` templates means any Rust code containing
  `{someidentifier}` (without spaces) would be misinterpreted as a placeholder. In
  practice this only occurs in macro invocations, which can use a `$`-prefix convention
  in the template if needed: `${placeholder}` → handled by adjusting the regex.
