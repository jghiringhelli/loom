# ADR-001: Implement the Loom compiler in Rust

**Date**: 2026-04-04  
**Status**: Accepted  
**Decided by**: Engineering

---

## Context

Loom is a source-to-source compiler (transpiler) that reads `.loom` files and
emits Rust or WebAssembly Text (WAT). We needed to choose an implementation
language for the compiler itself.

The project TechSpec template had a placeholder "Runtime: typescript" that was
never formally evaluated. Before starting Phase 1 we audited the options.

### Options considered

| Language | Pros | Cons |
|----------|------|------|
| **TypeScript** | Familiar to many web devs; fast iteration | GC pauses; needs Node runtime; AST nodes need manual boxing; weaker algebraic types |
| **Go** | Simple; fast compile; good stdlib | No ADTs; pattern matching is verbose; interface{} boxing for AST nodes |
| **Haskell / OCaml** | Ideal for compilers; pattern matching; HM native | Small hiring pool; harder to integrate into CI/toolchain; build system friction |
| **Rust** | Zero-cost ADTs; no GC; ships as single binary; cargo built-in; WASM target native | Steeper initial learning curve; borrow checker requires care with recursive ASTs |

---

## Decision

**Use Rust** as the sole implementation language for the Loom compiler.

---

## Rationale

### 1. ADTs + Pattern Matching are compiler primitives

The AST requires sum types (enums) and exhaustive pattern matching.  
Rust `enum` + `match` with the compiler enforcing exhaustiveness is exactly
what compiler passes need — the exhaustiveness checker practically writes itself.

```rust
// Rust — the AST node IS the type
enum Expr {
    Literal(Literal),
    Ident(String),
    BinOp { op: BinOpKind, left: Box<Expr>, right: Box<Expr>, span: Span },
    Match { subject: Box<Expr>, arms: Vec<MatchArm>, span: Span },
    // ...
}
```

In TypeScript this requires class hierarchies, discriminated unions with manual
`kind` discriminants, and `as` casts throughout.

### 2. Ownership enforces correct tree structure

The AST is a tree — owned top-down, no shared mutation. Rust's ownership model
enforces this **at compile time**, preventing entire classes of bugs (double-free,
use-after-free, accidental aliasing of nodes). In a GC language these bugs are
runtime errors or silent data corruption.

### 3. `Result<T, E>` and `?` for the pipeline

The multi-stage pipeline (`lex → parse → infer → check → exhaustiveness →
effects → emit`) is a chain of fallible operations. Rust's `?` operator and
`Result` type make error propagation explicit and zero-overhead:

```rust
pub fn compile(src: &str) -> Result<String, Vec<LoomError>> {
    let tokens = Lexer::tokenize(src)?;
    let module = Parser::new(&tokens).parse_module()?;
    InferenceEngine::new().check(&module)?;
    // ...
}
```

No exceptions, no thrown objects, no missing error paths.

### 4. Single static binary, no runtime dependency

`loom compile file.loom` ships as a single binary with no JVM, no Node.js, no
Python interpreter. This matters for:
- CI/CD pipelines (install once, run anywhere)
- Editor integrations (LSP binary runs without npm/node)
- Embedded toolchains

### 5. WASM is a first-class target

Loom emits WebAssembly Text (WAT). Rust also compiles to WASM natively
(`wasm32-unknown-unknown`). If we ever want to ship the Loom compiler itself
as a WASM module (e.g., in-browser playground), zero additional work is needed.

### 6. `cargo test` is built-in

No test framework decisions. `cargo test` discovers, runs, and reports tests
including doc-tests, unit tests, and integration tests. The entire test
infrastructure is `Cargo.toml` + `tests/` directory.

### 7. Long-term stability

Rust has a **6-year stability guarantee** (Edition model). Code written today
compiles without modification in future Rust editions. No Node major-version
breaking changes, no Python 2→3 migrations.

---

## Consequences

### Positive
- Type-safe AST construction prevents malformed trees reaching codegen
- Pattern match exhaustiveness catches missed cases at compile time
- Zero-cost abstractions mean no performance compromise for correctness
- Binary size is small (~3 MB); startup time is instant (no JIT warm-up)
- `tower-lsp` provides a production LSP server in Rust with async support

### Negative / Mitigations
- **Borrow checker + recursive ASTs**: Solved with `Box<T>` for child nodes and
  `Vec<T>` for sequences. No `Rc<RefCell<>>` needed anywhere in the compiler.
- **Learning curve**: The compiler is ~2,000 lines of Rust. The patterns used
  (enum + match + Result) are the same ~10 patterns repeated throughout.
- **Async for LSP**: `tower-lsp` requires `tokio`. Isolated to `src/lsp.rs` and
  `src/bin/loom_lsp.rs`; the core compiler is fully synchronous.

---

## Update to TechSpec

The TechSpec.md "Runtime: typescript" field was a template placeholder and was
never evaluated. It should be updated to reflect:

```
Runtime: Rust (stable, edition 2021)
Build:   cargo 1.x
Binary:  loom (single static binary)
LSP:     loom-lsp (tower-lsp, tokio)
```

---

## Tags

`COMPILER`, `RUST`, `ARCHITECTURE`
