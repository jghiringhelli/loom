# Loom — Phase 2 Roadmap

> Phase 1 is complete (lexer · parser · type checker · effect checker · Rust emitter · CLI).
> Phase 2 extends the compiler with four independent milestones.
> Work one milestone at a time. Each has its own development prompt — paste the prompt
> at the start of a new Copilot session to load the full context for that milestone.

---

## Milestone Index

| # | Milestone | Status | Branch |
|---|-----------|--------|--------|
| M1 | Type Inference | ⬚ Not Started | `feat/type-inference` |
| M2 | Pattern Exhaustiveness Checking | ⬚ Not Started | `feat/exhaustiveness` |
| M3 | WASM Back-end | ⬚ Not Started | `feat/wasm-backend` |
| M4 | Language Server Protocol | ⬚ Not Started | `feat/lsp` |

---

## M1 — Type Inference

### What it is
Replace the current explicit-annotation-only type checker with a Hindley-Milner
(HM) unification engine so that function bodies and `let` bindings can omit type
annotations. The existing explicit annotations become optional, not mandatory.

### Scope
- Add a `TypeVar` variant to `TypeExpr` representing an unresolved inference variable.
- Implement a `Substitution` (mapping from type-variable ID → `TypeExpr`) and a
  `unify(t1, t2, subst)` function that returns `Result<Substitution, LoomError>`.
- Add a constraint-generation pass that walks the AST and emits `(TypeExpr, TypeExpr)`
  equality constraints for every expression.
- Add a constraint-solving pass that runs `unify` on each constraint and builds the
  final substitution.
- Apply the substitution to the AST, replacing every `TypeVar` with its resolved type.
- Wire the inference pass into `lib.rs::compile` between parsing and the existing
  type checker; the existing checker becomes the post-inference consistency check.
- Existing corpus examples (`corpus/pricing_engine.loom`, `corpus/user_service.loom`)
  must continue to compile without modification.
- Property: `infer(annotated_source) == infer(unannotated_source)` for all valid programs.

### Key files
- `src/checker/types.rs` — extend with the inference engine
- `src/checker/infer.rs` — new file for constraint generation and solving
- `src/checker/mod.rs` — expose `InferenceEngine`
- `src/ast.rs` — add `TypeExpr::TypeVar(u32)`
- `src/error.rs` — add `UnificationError` variant
- `src/lib.rs` — wire inference between parse and check

### Success criteria
- `cargo test` passes with zero regressions.
- New unit tests in `tests/unit/infer.rs` cover: basic let inference, function
  argument inference, recursive function inference, unification failure (mismatched
  types), and occurs-check violation.
- Property test: annotated and annotation-free versions of the same program produce
  identical inferred types.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Implement Hindley-Milner type inference for the Loom compiler (Rust, located
in src/). Phase 1 is fully built; do not break existing behavior.

Context:
- AST is in src/ast.rs. TypeExpr is the type representation enum.
- The existing type checker is in src/checker/types.rs (symbol resolution only —
  it does NOT infer missing annotations).
- lib.rs::compile runs: lex → parse → TypeChecker::check → EffectChecker::check → emit.
- Corpus examples in corpus/ compile today and must continue to compile.

What to build:
1. Add TypeExpr::TypeVar(u32) to src/ast.rs for inference variables.
2. Create src/checker/infer.rs with:
   - TypeVar counter (thread-local or passed as &mut)
   - Substitution type: HashMap<u32, TypeExpr> with apply() and compose()
   - unify(t1: &TypeExpr, t2: &TypeExpr, subst: &mut Substitution) → Result<(), LoomError>
     with occurs-check
   - constrain(expr: &Expr, env: &TypeEnv) → Vec<(TypeExpr, TypeExpr)>
   - solve(constraints: &[(TypeExpr, TypeExpr)]) → Result<Substitution, Vec<LoomError>>
3. Expose InferenceEngine from src/checker/mod.rs.
4. Wire InferenceEngine into lib.rs::compile between parse and TypeChecker::check.
5. Write tests in tests/unit/infer.rs following TDD — write failing tests first,
   then implement.

Constraints:
- Max 5 files per phase. Complete and verify each phase before continuing.
- Follow TDD: write a failing test, run it (show failure), implement, show green.
- Run cargo test after every phase. Zero failures required before proceeding.
- No changes outside src/checker/ and src/lib.rs in Phase 1.
```

---

## M2 — Pattern Exhaustiveness Checking

### What it is
Detect non-exhaustive `match` expressions at compile time. A `match` on a sum type
(`enum`) that does not cover every variant (and has no wildcard `_`) must produce a
compile error listing the uncovered variants.

### Scope
- After the existing type checker pass, add an `ExhaustivenessChecker` pass that
  walks every `Expr::Match` node.
- For each match on a known enum type, collect the set of variants covered by the
  arms (including wildcard and variable binders as catch-alls).
- If any variant is not covered and there is no wildcard/variable arm, emit a
  `LoomError::NonExhaustiveMatch` listing the missing variants.
- Nested patterns (e.g., `| Some(Red)`) require recursive exhaustiveness analysis.
- Wire the new pass into `lib.rs::compile` after `TypeChecker::check`.

### Key files
- `src/checker/exhaustiveness.rs` — new file
- `src/checker/mod.rs` — expose `ExhaustivenessChecker`
- `src/error.rs` — add `NonExhaustiveMatch { missing: Vec<String>, span: Span }`
- `src/lib.rs` — wire after type check

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/unit/exhaustiveness.rs` cover: fully exhaustive match (passes),
  missing one variant (fails with correct variant name in error), wildcard covers all
  (passes), guard-only arm does not count as covering (fails), nested enum exhaustiveness.
- Both corpus examples compile without error.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Implement pattern exhaustiveness checking for match expressions in the Loom
compiler (Rust, src/).

Context:
- AST is in src/ast.rs. Relevant nodes: Expr::Match, MatchArm, Pattern (Variant,
  Ident, Wildcard, Literal), EnumDef (name + variants).
- The type checker in src/checker/types.rs already resolves enum names to their
  definitions. You can reuse or extend that symbol table.
- lib.rs::compile pipeline: lex → parse → TypeChecker::check → EffectChecker::check → emit.
- Corpus in corpus/ must compile with no new errors.

What to build:
1. Create src/checker/exhaustiveness.rs with ExhaustivenessChecker.
   - check(&Module) → Result<(), Vec<LoomError>>
   - For every Expr::Match, determine the scrutinee type.
   - If the scrutinee is an enum type, compute the set of covered variants from the
     arm patterns. Pattern::Wildcard and Pattern::Ident(_) are total covers.
   - If covered_variants ⊊ all_variants: emit NonExhaustiveMatch with the missing names.
   - Recurse into nested patterns for nested enum types.
2. Add LoomError::NonExhaustiveMatch { missing: Vec<String>, span: Span } to src/error.rs.
3. Expose ExhaustivenessChecker from src/checker/mod.rs.
4. Insert ExhaustivenessChecker::new().check(&module) in lib.rs::compile after TypeChecker.
5. Write tests in tests/unit/exhaustiveness.rs — TDD, failing test first.

Constraints:
- Max 5 files per phase. Verify cargo test before continuing.
- Write failing tests first. Show failure output. Then implement. Show green.
- Do not modify the parser or lexer.
```

---

## M3 — WASM Back-end

### What it is
Add a second code generation target that emits [WebAssembly Text format (WAT)](https://webassembly.github.io/spec/core/text/index.html)
instead of Rust source. Loom programs that use only pure functions, product types,
and integer arithmetic can be compiled directly to `.wat` for execution in any WASM
runtime.

### Scope
- Create `src/codegen/wasm.rs` with a `WasmEmitter` that mirrors the interface of
  the existing `RustEmitter`: `emit(&Module) -> String`.
- The emitted WAT must be valid and executable by `wasmtime` or `wasmer`.
- Support for Phase 1 target subset: `Int`, `Float`, `Bool` primitives; product types
  as linear memory structs; arithmetic and comparison `BinOp`; function definitions
  with explicit type annotations; `let` bindings; function calls; `return`.
- Explicitly unsupported in M3 (emit a clear `TODO` WAT comment and a `LoomError::WasmUnsupported`):
  effect types, `requires`/`provides`, refined types, `match` expressions.
- Add `--target [rust|wasm]` flag to the CLI (`src/cli/`).
- Add corpus example `corpus/wasm_demo.loom` that exercises the supported subset.

### Key files
- `src/codegen/wasm.rs` — new file
- `src/codegen/mod.rs` — expose `WasmEmitter`
- `src/cli/` — add `--target` flag
- `src/error.rs` — add `WasmUnsupported { feature: String, span: Span }`
- `corpus/wasm_demo.loom` — new corpus example
- `tests/integration/wasm.rs` — round-trip test

### Success criteria
- `cargo test` passes with zero regressions.
- `loom compile corpus/wasm_demo.loom --target wasm` emits valid WAT that passes
  `wasm-tools validate` (or equivalent).
- Integration test in `tests/integration/wasm.rs` compiles the demo corpus and
  validates the WAT output structurally.
- Attempting to compile an effect-typed function with `--target wasm` returns
  `WasmUnsupported` error.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Add a WASM (WebAssembly Text format) back-end to the Loom compiler (Rust, src/).

Context:
- The existing Rust emitter is in src/codegen/rust.rs implementing RustEmitter::emit(&Module) → String.
- codegen/mod.rs re-exports RustEmitter.
- AST is in src/ast.rs. The supported subset for WASM in this milestone:
    Int, Float, Bool primitives; TypeDef (product types); FnDef with explicit
    type signatures; BinOp arithmetic and comparison; Expr::Let; Expr::Call; Expr::Literal.
- Unsupported subset (emit WasmUnsupported error): effect types, requires/provides,
    refined types, Expr::Match.
- CLI entry point is src/cli/. Currently only accepts --check-only and -o flags.

What to build:
1. Create src/codegen/wasm.rs with WasmEmitter:
   - emit(&Module) → Result<String, Vec<LoomError>>
     (returns Err with WasmUnsupported for unsupported features)
   - emit_fn(fn_def: &FnDef) → Result<String, LoomError>
   - emit_expr(expr: &Expr) → Result<String, LoomError>
   - Map Loom types: Int → i64, Float → f64, Bool → i32
2. Add LoomError::WasmUnsupported { feature: String, span: Span } to src/error.rs.
3. Expose WasmEmitter from src/codegen/mod.rs.
4. Add --target [rust|wasm] to the CLI; default is rust (no breaking change).
5. Create corpus/wasm_demo.loom using only the supported subset.
6. Write tests in tests/integration/wasm.rs using TDD.

Constraints:
- Max 5 files per phase. Cargo test must pass after each phase.
- The Rust back-end must be completely unaffected.
- WAT output must be syntactically valid — validate with a WAT parser in tests if possible,
  otherwise structurally assert module/func/export presence.
- TDD: write failing test, show failure, implement, show green.
```

---

## M4 — Language Server Protocol (LSP)

### What it is
A standalone `loom-lsp` binary that implements the
[Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
over stdin/stdout, enabling hover types, go-to-definition, and live diagnostics
in any LSP-compatible editor.

### Scope
- Add `loom-lsp` as a second `[[bin]]` in `Cargo.toml`.
- Add `tower-lsp` (or `lsp-server` + `lsp-types`) as a dependency.
- Implement the following LSP capabilities:
  - `textDocument/didOpen` and `textDocument/didChange` → run the full compile
    pipeline and publish diagnostics (`LoomError` → LSP `Diagnostic`).
  - `textDocument/hover` → return the inferred type of the identifier under the cursor
    (requires M1 type inference to be complete first).
  - `textDocument/definition` → jump to the definition of the symbol under the cursor.
- The LSP binary must not share mutable state with the compiler library; it calls
  `loom::compile` and the checker APIs as a pure library consumer.
- Add a VS Code extension stub in `editors/vscode/` with `package.json` that launches
  `loom-lsp` (extension activation only — no TypeScript implementation required in this milestone).

### Key files
- `src/lsp/` — new module (server, document, capabilities, diagnostics)
- `src/lsp/main.rs` → `src/main_lsp.rs` or `src/bin/loom-lsp.rs`
- `Cargo.toml` — add `[[bin]] loom-lsp` and `tower-lsp` dependency
- `editors/vscode/package.json` — minimal VS Code extension manifest
- `tests/integration/lsp.rs` — LSP lifecycle tests using stdio transport

### Dependencies
- **Requires M1 (type inference)** to be complete for hover type display.
- M2 and M3 are independent.

### Success criteria
- `cargo build --bin loom-lsp` succeeds.
- Integration test sends `initialize` + `textDocument/didOpen` + `textDocument/hover`
  messages over a pipe and asserts correct LSP response shapes.
- Opening a `.loom` file with errors in VS Code (with the stub extension installed)
  shows red squiggles in the editor.
- `cargo test` passes with zero regressions.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Add an LSP (Language Server Protocol) server to the Loom compiler (Rust, src/).
This is Milestone 4. Milestone 1 (type inference) must already be complete.

Context:
- loom is a Rust workspace with one binary (src/main.rs) and a library (src/lib.rs).
- loom::compile(source: &str) → Result<String, Vec<LoomError>> is the public API.
- LoomError carries a Span (byte offsets) for each error — map these to LSP Position.
- The type checker in src/checker/types.rs has a symbol table that maps names to types;
  the inference engine (M1) extends this with inferred type annotations.
- Use tower-lsp (version-pin to match Cargo.lock) for the LSP server framework.
  Audit first: cargo audit before adding.

What to build:
1. Add src/lsp/ module with:
   - server.rs: LoomLanguageServer struct implementing LanguageServer trait
   - document.rs: DocumentStore (Arc<DashMap<Url, String>>) holding open document text
   - diagnostics.rs: loom_error_to_diagnostic(e: &LoomError, source: &str) → Diagnostic
2. Add src/bin/loom-lsp.rs as the LSP binary entry point.
3. Add to Cargo.toml:
   [[bin]]
   name = "loom-lsp"
   path = "src/bin/loom-lsp.rs"
   And the tower-lsp dependency (after cargo audit passes).
4. Implement handlers:
   - initialize: return server capabilities
   - text_document/did_open + did_change: run loom::compile, publish diagnostics
   - text_document/hover: return inferred type of symbol under cursor (from M1 type map)
   - text_document/definition: return definition span of symbol under cursor
5. Create editors/vscode/package.json stub that declares the loom-lsp server.
6. Write integration tests in tests/integration/lsp.rs that spin up the server
   over a pipe and exchange JSON-RPC messages.

Constraints:
- Max 5 files per phase. Cargo test must pass after each phase.
- The LSP binary must not import any async runtime other than tokio (already used by tower-lsp).
- loom::compile must remain a synchronous pure function — do not add async to the library.
- TDD: write failing test first, show failure, implement, show green.
- Run cargo audit before adding tower-lsp. Document result in docs/approved-packages.md.
```

---

## Execution Order

```
M2 (Exhaustiveness)  ──┐
                        ├── both independent of M1 and each other
M3 (WASM)            ──┘

M1 (Type Inference)  ──── must complete before M4

M4 (LSP)             ──── depends on M1
```

Recommended sequence: **M2 → M3 → M1 → M4**
(M2 and M3 are self-contained and provide quick wins; M1 is the largest and unlocks M4.)
