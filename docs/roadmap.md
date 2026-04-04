# Loom — Phase 2 & Phase 3 Roadmap

> Phase 1 is complete (lexer · parser · type checker · effect checker · Rust emitter · CLI).
> Phase 2 is complete (type inference · exhaustiveness · WASM back-end · LSP).
> Phase 3 extends the compiler with dependency injection, stdlib type mappings, generics, and multi-module compilation.

---

## Phase 2 Milestone Index

| # | Milestone | Status | Branch |
|---|-----------|--------|--------|
| M1 | Type Inference | ✅ Done | `main` |
| M2 | Pattern Exhaustiveness Checking | ✅ Done | `main` |
| M3 | WASM Back-end | ✅ Done | `main` |
| M4 | Language Server Protocol | ✅ Done | `main` |

---

## Phase 3 Milestone Index

| # | Milestone | Status | Branch |
|---|-----------|--------|--------|
| M5 | Dependency Injection (`requires`/`with`) | ✅ Done | `main` |
| M6 | Standard Library Type Mappings | ✅ Done | `main` |
| M7 | Generic Functions (Type Parameters) | ✅ Done | `main` |
| M8 | Multi-Module Project Compilation | ✅ Done | `main` |

Recommended execution order: **M5 → M6 → M7 → M8**

---

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

---

## M5 — Dependency Injection (`requires` / `with`)

### What it is
Complete the DI system: when a module declares `requires { db: DbConn, log: Logger }`, the
compiler emits a `ModuleContext` struct and threads it through every function that declares
`with [db]` or `with [log]`.  This makes Loom's effect-isolation model fully functional.

### Scope
- In `codegen/rust.rs`, detect `module.requires` and emit a `pub struct <ModName>Context { … }`.
- For each `FnDef` where `with_deps` is non-empty, prepend `ctx: &<ModName>Context` as the
  first parameter and access injected deps as `ctx.db`, `ctx.log`, etc.
- In `checker/types.rs`, validate that every name listed in a function's `with [dep]` clause
  is declared in the enclosing module's `requires` block.
- Add a new `LoomError::UndeclaredDependency { name, span }` variant.
- Corpus: add `corpus/di_demo.loom` exercising the DI system.

### Key files
- `src/codegen/rust.rs` — emit context struct + inject `ctx` param
- `src/checker/types.rs` — validate `with_deps` against module `requires`
- `src/error.rs` — `UndeclaredDependency` variant
- `corpus/di_demo.loom` — new corpus example
- `tests/di_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests: module with `requires` emits a context struct; function with `with` gets `ctx`
  param; referencing an undeclared dep produces `UndeclaredDependency` error.
- `corpus/di_demo.loom` compiles to valid Rust.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Implement the Dependency Injection (DI) system for the Loom compiler (Rust, src/).
Phase 2 is fully complete. Do not break any of the 60 existing tests.

Context:
- AST: Module.requires is Option<Requires> (list of (name, TypeExpr) pairs).
  FnDef.with_deps is Vec<String> (dep names the function consumes).
- Parser already handles both — they are populated in the AST but ignored by codegen and
  the type checker.
- Codegen entry: src/codegen/rust.rs RustEmitter::emit(&Module).
- Type checker: src/checker/types.rs TypeChecker::check(&Module).
- Compile pipeline: lib.rs compile() → lex → parse → infer → type_check → exhaustiveness →
  effect_check → emit.

What to build:
1. In src/codegen/rust.rs:
   - If module.requires is Some(reqs), emit:
       pub struct <ModName>Context { pub <dep_name>: <dep_type>, … }
     inside the module (before any function definitions).
   - For each FnDef where with_deps is non-empty, prepend
       ctx: &<ModName>Context
     as the first parameter.
2. In src/checker/types.rs:
   - After checking each FnDef, verify every name in with_deps is present in
     module.requires. If not, emit LoomError::UndeclaredDependency { name, span }.
3. Add LoomError::UndeclaredDependency { name: String, span: Span } to src/error.rs.
   Update span() and kind() match arms.
4. Create corpus/di_demo.loom: a module with requires { db: DbConn } containing
   fn find :: Int -> Effect<[IO], String> with [db] ... end.
5. Write tests in tests/di_test.rs using TDD (failing test first).

Constraints:
- Max 5 files per phase. cargo test must pass after each phase.
- TDD: write failing test, show failure, implement, show green.
- Do not modify the parser or lexer.
- The WASM emitter should gracefully skip context injection (DI is Rust-only in M5).
```

---

## M6 — Standard Library Type Mappings

### What it is
Map Loom collection types to Rust's standard library so that `List<T>`, `Map<K,V>`,
and `Set<T>` compile to `Vec<T>`, `HashMap<K,V>`, and `HashSet<T>` respectively.
Emit the necessary `use` imports automatically.

### Scope
- Extend `RustEmitter::emit_type_expr` to map `List<T>` → `Vec<T>`, `Map<K,V>` →
  `HashMap<K,V>`, `Set<T>` → `HashSet<T>`.
- Detect when `HashMap` or `HashSet` appear in the emitted module and prepend
  `use std::collections::{HashMap, HashSet};` to the module body.
- Extend the type checker to know that `List<T>`, `Map<K,V>`, `Set<T>` are valid
  generic types (no `UndefinedType` error for them).
- Corpus: add `corpus/collections_demo.loom` exercising all three types.

### Key files
- `src/codegen/rust.rs` — extend type mapping + import injection
- `src/checker/types.rs` — recognise stdlib generics
- `corpus/collections_demo.loom` — new corpus example
- `tests/stdlib_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests: `List<Int>` emits `Vec<i64>`; `Map<String, Int>` emits `HashMap<String, i64>`;
  a module using `HashMap` contains `use std::collections::HashMap`.
- `corpus/collections_demo.loom` compiles without errors.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Add standard library type mappings to the Loom compiler (Rust, src/).
Phase 2 + M5 are complete. Do not break existing tests.

Context:
- RustEmitter::emit_type_expr in src/codegen/rust.rs currently handles:
  Base, Generic, Effect, Option, Result, Tuple, TypeVar.
  The Generic arm passes through unknown names verbatim (e.g. List<T> → List<T>).
- TypeChecker in src/checker/types.rs validates that Base types refer to declared
  types. It needs to recognise stdlib generic names so they don't produce errors.

What to build:
1. In src/codegen/rust.rs emit_type_expr, map:
   - Generic("List", [T])  → Vec<T>
   - Generic("Map", [K,V]) → HashMap<K, V>
   - Generic("Set", [T])   → HashSet<T>
2. Track whether HashMap/HashSet are used during emit; if so, inject
   `use std::collections::{HashMap, HashSet};` at the top of the module body.
3. In src/checker/types.rs, add "List", "Map", "Set" to the set of known generic
   type constructors so they don't produce TypeError.
4. Create corpus/collections_demo.loom with functions that use List<Int>,
   Map<String, Int>, and Set<Bool>.
5. Write tests in tests/stdlib_test.rs using TDD (failing test first).

Constraints:
- Max 5 files per phase. cargo test must pass after each phase.
- TDD: failing test first.
- Do not change any existing corpus file.
```

---

## M7 — Generic Functions (Type Parameters)

### What it is
Allow users to declare polymorphic functions with explicit type parameters:
`fn identity<T> :: T -> T`.  The type parameters are resolved during HM inference
and emitted as Rust generics with appropriate bounds.

### Scope
- Add `type_params: Vec<String>` to `FnDef` in `src/ast.rs`.
- Extend the parser to parse `fn name<A, B> :: …` (angle-bracket type params).
- In `codegen/rust.rs`, emit `pub fn name<A, B>(…)` when `type_params` is non-empty.
- In `checker/infer.rs`, introduce fresh `TypeVar`s for each type param and unify against
  them during constraint generation; after solving, any remaining `TypeVar` bound to a
  named type param stays as the named param in the output.
- In `checker/types.rs`, treat type param names as valid type names within the function's scope.

### Key files
- `src/ast.rs` — add `type_params: Vec<String>` to `FnDef`
- `src/parser/mod.rs` — parse `<A, B>` after the function name
- `src/codegen/rust.rs` — emit `<A>` suffix on fn signature
- `src/checker/infer.rs` — handle type-param TypeVars
- `tests/generics_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests: `fn id<T> :: T -> T` emits `pub fn id<T>(arg0: T) -> T`; two-param generic
  `fn map<A, B>` emits correctly; unification of `id(42)` resolves to `i64`.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Add generic (polymorphic) functions to the Loom compiler (Rust, src/).
Phase 2 + M5 + M6 are complete. Do not break existing tests.

Context:
- FnDef in src/ast.rs currently has no type_params field.
- Parser in src/parser/mod.rs: parse_fn_def() reads `fn name :: sig`.
- InferenceEngine in src/checker/infer.rs generates and solves constraints.
- RustEmitter::emit_fn_def in src/codegen/rust.rs emits plain fn signatures.

What to build:
1. Add `pub type_params: Vec<String>` to FnDef in src/ast.rs.
   Update all construction sites to provide an empty Vec (no breaking change).
2. In src/parser/mod.rs, after parsing the `fn` keyword and name,
   optionally parse `<A, B, ...>` to populate type_params.
3. In src/codegen/rust.rs emit_fn_def, if type_params is non-empty emit:
   pub fn name<A, B>(…) → …
4. In src/checker/types.rs, when type-checking a FnDef, add each type_param
   name to the local type namespace so Base(name) is valid inside the body.
5. In src/checker/infer.rs, treat each type_param as an unconstrained TypeVar
   that is generalised (not solved to a concrete type).
6. Write tests in tests/generics_test.rs using TDD (failing test first).

Constraints:
- Max 5 files per phase. cargo test must pass after each phase.
- TDD: failing test first.
- Backward-compatible: all current corpus examples have type_params = [] and must
  continue to compile identically.
```

---

## M8 — Multi-Module Project Compilation

### What it is
A `loom build` command and `loom.toml` project manifest that compile a set of `.loom`
files in dependency order and emit a `lib.rs` re-exporting all modules.

### Scope
- Define a `loom.toml` format: `name`, `version`, `modules` (list of `.loom` file paths).
- Add a `build` sub-command to the CLI (`src/main.rs`).
- Add `src/project.rs`: parse `loom.toml`, build a dependency graph from module
  `requires` clauses, topologically sort it, compile in order, write each `.rs` file,
  emit a top-level `lib.rs`.
- Error if a cycle is detected in the dependency graph.
- Corpus: add `corpus/project/` with a multi-file example.

### Key files
- `src/main.rs` — add `build` sub-command
- `src/project.rs` — new file (manifest parser + build orchestrator)
- `loom.toml` (root) — example project manifest
- `corpus/project/` — new multi-file corpus
- `tests/project_test.rs` — new test file

### Dependencies
- Requires M5 (DI) to be complete so that `requires` inter-module links are meaningful.

### Success criteria
- `cargo test` passes with zero regressions.
- `loom build loom.toml` compiles all modules, writes `.rs` files, and emits `lib.rs`.
- Cycle detection: a project with A → B → A produces a `CyclicDependency` error.
- `corpus/project/` compiles successfully with `loom build`.

### Development prompt

```
Read .claude/index.md → .claude/core.md → .claude/standards/architecture.md.

Task: Add multi-module project compilation to the Loom compiler (Rust, src/).
Phase 2 + M5 + M6 + M7 are complete. Do not break existing tests.

Context:
- CLI entry point: src/main.rs with `loom compile` sub-command (clap).
- lib.rs exposes compile(source: &str) → Result<String, Vec<LoomError>>.
- Module AST has name, requires, provides fields for inter-module relations.

What to build:
1. Define loom.toml format (toml crate, add as dependency):
     [project]
     name = "my-project"
     version = "0.1.0"
     modules = ["src/a.loom", "src/b.loom"]
     output = "out/"
2. Add `build` sub-command to src/main.rs.
3. Create src/project.rs with:
   - ProjectManifest: parse loom.toml
   - DependencyGraph: build from parsed Module.requires names
   - topo_sort(graph) → Result<Vec<PathBuf>, LoomError>  (CyclicDependency error)
   - build(manifest) → Result<(), Vec<LoomError>>: compile each module, write .rs, emit lib.rs
4. Add LoomError::CyclicDependency { cycle: Vec<String>, span: Span } to error.rs.
5. Corpus: corpus/project/ with a.loom (pure) and b.loom (requires a).
6. Write tests in tests/project_test.rs using TDD (failing test first).

Constraints:
- Max 5 files per phase. cargo test must pass after each phase.
- TDD: failing test first.
- loom compile must remain unchanged — only loom build is new.
```

---

## Phase 3 Execution Order

```
M5 (DI)      ──── independent, completes a core language feature

M6 (stdlib)  ──── independent, quick win

M7 (generics) ─── independent, largest milestone

M8 (multi-module) ── depends on M5 (requires links meaningful)
```

Recommended sequence: **M5 → M6 → M7 → M8**

---

---

# Loom — Phase 4, Phase 5 & Phase 6 Roadmap

> Phase 4 (M9–M12): Language Completeness — make real programs possible without escape hatches.
> Phase 5 (M13–M16): GS-Native Constructs — embed every Generative Specification property directly
>   into the language surface.
> Phase 6 (M17–M18): Multi-Target Derivation — one Loom spec generates Rust, TypeScript, and
>   OpenAPI from the same source.

Design rubric: every milestone is evaluated against the 7 GS properties
(Self-describing · Bounded · Verifiable · Defended · Auditable · Composable · Executable).
Reference: `docs/white-paper/GenerativeSpecification_WhitePaper.md`

---

## Phase 4 Milestone Index

| # | Milestone | Status | Branch |
|---|-----------|--------|--------|
| M9  | `inline rust {}` Escape Hatch | ⬜ Planned | — |
| M10 | Numeric Coercion + Parenthesized Expressions | ⬜ Planned | — |
| M11 | First-Class Iteration (closures · map · filter · fold) | ⬜ Planned | — |
| M12 | Tuples · `Option<T>` · `Result<T,E>` First-Class | ⬜ Planned | — |

---

## Phase 5 Milestone Index

| # | Milestone | GS Property | Status | Branch |
|---|-----------|-------------|--------|--------|
| M13 | `describe:` Blocks + Audit Annotations | Self-describing · Auditable | ⬜ Planned | — |
| M14 | `invariant:` Declarations + Consequence Tiers | Defended | ⬜ Planned | — |
| M15 | `test:` Blocks + Real `ensure:` Assertions | Verifiable | ⬜ Planned | — |
| M16 | `import` + Explicit `interface` Declarations | Composable | ⬜ Planned | — |

---

## Phase 6 Milestone Index

| # | Milestone | Status | Branch |
|---|-----------|--------|--------|
| M17 | TypeScript Emission Target | ⬜ Planned | — |
| M18 | Contract Materialisation (OpenAPI · JSON Schema) | ⬜ Planned | — |

---

---

## M9 — `inline rust {}` Escape Hatch

### What it is
Allow function bodies to be written directly in Rust inside an `inline { ... }` block.
The Loom header (name, type signature, effects, contracts, DI bindings) remains the
specification mold; the inline body is the implementation the AI reader fills in.
Until M11 ships full iteration support, `inline` is the only way to express non-trivial
algorithms in Loom programs.

> **GS alignment**: The header IS the GS spec. The `inline` body is the foundry output
> inserted by the human or AI author. The AI reader always reasons about the header alone;
> the body is an implementation detail.

### Scope
- Add `Expr::InlineRust(String)` to `src/ast.rs`.
- Extend the parser to recognise `inline { ... }` as an expression anywhere a function
  body expression is expected.  The braces may contain arbitrary text — the parser collects
  everything until the matching closing `}` (brace-depth counting).
- In `src/codegen/rust.rs`, emit the inline string verbatim inside the function body (no
  wrapping, no indentation normalisation — the author is responsible for valid Rust).
- The type checker and effect checker skip `InlineRust` nodes (treat them as opaque
  — the same way `todo` is treated today).
- The inference engine assigns `TypeVar` to `InlineRust` nodes and does not constrain them.
- Add `corpus/inline_demo.loom` showing a function whose body is `inline { ... }`.
- Error case: an `inline` block that is not inside a `fn` body must produce
  `LoomError::InvalidContext("inline block outside function body")`.

### Key files
- `src/ast.rs` — add `Expr::InlineRust(String)`
- `src/parser/mod.rs` — parse `inline { ... }` in `parse_expr`
- `src/codegen/rust.rs` — emit `InlineRust` verbatim in `emit_expr`
- `src/checker/infer.rs` — skip `InlineRust` in `collect_free_vars` and constraint gen
- `corpus/inline_demo.loom` — new corpus example
- `tests/inline_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/inline_test.rs`:
  - A function with `inline { vec![1, 2, 3] }` emits the Rust verbatim.
  - A function with both a Loom signature and an `inline` body round-trips through
    the full E2E pipeline (lex → parse → check → emit → `rustc` → run).
  - `inline` outside a `fn` body returns `LoomError::InvalidContext`.
  - Existing `todo` bodies still work (no regression).
- `corpus/inline_demo.loom` compiles with `loom compile`.

### Development prompt

```
Read docs/roadmap.md (M9 section) and docs/white-paper/GenerativeSpecification_WhitePaper.md.
Read src/ast.rs, src/parser/mod.rs, src/codegen/rust.rs, src/checker/infer.rs.
All of Phase 1–3 + real-world tests are complete. Do not break any existing test.

Task: Add `inline rust {}` escape hatch to the Loom compiler.

Context:
- Function bodies are Expr values. The most common body today is Expr::Ident("todo")
  which emits as todo!() in Rust.
- The parser's parse_expr dispatches on the current token to choose which sub-parser
  to call. You need to add a branch for the `inline` keyword.
- The codegen emit_expr match arms handle every Expr variant. You need a new arm for
  InlineRust that writes the string verbatim.
- The checker/infer collect_free_vars function must NOT enter InlineRust nodes
  (add it to the BUILTINS exclusion list or add an explicit arm returning empty set).

What to build:
1. Add `Expr::InlineRust(String)` to src/ast.rs.
2. In src/parser/mod.rs parse_expr, add: if current token is Ident("inline"), consume it,
   expect LBrace, then collect tokens/chars until brace depth returns to 0, storing the
   raw string. Return Expr::InlineRust(content).
3. In src/codegen/rust.rs emit_expr, add arm: Expr::InlineRust(s) => write s verbatim.
4. In src/checker/infer.rs, ensure collect_free_vars returns empty for InlineRust.
   Also ensure constraint generation produces no constraints for InlineRust bodies.
5. Add corpus/inline_demo.loom with at least one function using inline {}.
6. Write tests/inline_test.rs with TDD (failing test first, then implement, then green).

Constraints:
- Max 5 files per phase. cargo test must pass after each phase.
- The inline body is emitted VERBATIM — do not add semicolons, indentation, or wrappers.
- The existing todo!() behaviour must not regress.
- Do not change the Loom grammar for anything outside inline expressions.
```

---

## M10 — Numeric Coercion + Parenthesized Expressions

### What it is
Two focused parser/codegen fixes that unblock real arithmetic programs.
(1) Add the `as` coercion operator so `Int` and `Float` values can be explicitly
    widened: `price as Float`. This fixes the `quantity * unit_price` bug in
    `corpus/pricing_engine.loom`.
(2) Add parenthesized expression support `(a + b)` to the parser so sub-expression
    grouping works.

### Scope
- **`as` coercion**
  - Add `Expr::As(Box<Expr>, TypeExpr)` to `src/ast.rs`.
  - Extend the parser's postfix loop (`parse_postfix`) to consume `as TypeExpr` after
    any primary/postfix expression, producing `Expr::As`.
  - In `src/codegen/rust.rs`, emit `(expr as rust_type)`.
  - In the type checker, validate that the source and target types are both numeric
    (`Int`/`Float`) or both `String`/compatible; error on nonsensical casts.
  - Fix `corpus/pricing_engine.loom`: cast `quantity as Float` before multiplying.
- **Parenthesized expressions**
  - In `parse_primary`, add a branch for `LParen` that parses an inner expression then
    expects `RParen`, returning the inner expression unchanged (no new AST node needed).
  - Add tests: `(a + b) * c` parses correctly; `((x))` round-trips.
- **Named parameter annotations** (bonus, same PR)
  - Extend `FnTypeSignature` to optionally carry parameter names:
    `fn add :: (x: Int, y: Int) -> Int`. Names are emitted in the Rust signature.
  - This eliminates the positional free-variable matching limitation documented in
    `src/checker/infer.rs`.

### Key files
- `src/ast.rs` — `Expr::As`, optional param names in `FnTypeSignature`
- `src/parser/mod.rs` — `parse_postfix` (as), `parse_primary` (parens), param name parsing
- `src/codegen/rust.rs` — emit `as` cast, named params
- `src/checker/types.rs` — validate `as` source/target types
- `corpus/pricing_engine.loom` — fix `quantity as Float`
- `tests/coercion_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/coercion_test.rs`:
  - `quantity as Float` emits `quantity as f64`.
  - `(a + b) * c` parses and emits correctly.
  - Casting `Int as String` produces a type error.
  - Named parameter `fn add :: (x: Int) -> Int` emits `pub fn add(x: i64) -> i64`.
- `corpus/pricing_engine.loom` compiles end-to-end with `loom compile` and the emitted
  Rust is accepted by `rustc`.

### Development prompt

```
Read docs/roadmap.md (M10 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs, src/checker/types.rs, corpus/pricing_engine.loom,
tests/realworld_test.rs (for context on existing assertion style).

Task: Add `as` coercion, parenthesized expressions, and named fn parameters to Loom.

Context:
- The pricing_engine corpus has `quantity: Int * unit_price: Float` which fails in Rust
  because i64 * f64 is not allowed. The fix is `quantity as Float * unit_price`.
- parse_postfix in src/parser/mod.rs handles function-call syntax (LParen trigger).
  The `as` keyword should be added as another postfix operator in that same loop.
- parse_primary is the bottom of the precedence hierarchy; parenthesized expressions
  belong there (consume LParen, parse_expr, expect RParen).
- FnTypeSignature currently stores only a Vec<TypeExpr> for params. To support named
  params, change it to Vec<(Option<String>, TypeExpr)>.

What to build:
1. Add Expr::As(Box<Expr>, TypeExpr) to src/ast.rs.
2. In parse_postfix, after the function-call branch, add: if current token is
   Ident("as"), consume it, parse a TypeExpr, return Expr::As(lhs, ty).
3. In parse_primary, add LParen branch: consume, parse_expr, expect RParen, return inner.
4. In emit_expr, add Expr::As(e, ty) => format!("({} as {})", emit_expr(e), emit_type_expr(ty)).
5. In types.rs, validate As: both types must be numeric (Int↔Float) or the cast is invalid.
6. Optional: extend FnTypeSignature with named params; update parser and codegen.
7. Fix corpus/pricing_engine.loom to use `quantity as Float`.
8. Write tests/coercion_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- Do not break existing corpus files except pricing_engine.loom (which is being fixed).
- Named params are optional for this milestone — skip if it risks breaking inference tests.
```

---

## M11 — First-Class Iteration (Closures · map · filter · fold)

### What it is
Add closures (anonymous functions), `for` loop expressions, and the three fundamental
higher-order functions (`map`, `filter`, `fold`) as first-class citizens.  This is the
largest single language addition in Phase 4 and is the prerequisite for writing any
non-trivial data transformation without `inline rust {}`.

### Scope
- **Closure syntax**: `fn(x: Int) -> x + 1` or `|x| x + 1` (two syntactic forms).
  - Add `Expr::Lambda { params: Vec<(String, Option<TypeExpr>)>, body: Box<Expr> }` to AST.
  - Parse `|param, param| expr` (Rust-style) and `fn(param: Type) -> expr` (Loom-style).
  - Emit as Rust closures: `|x: i64| x + 1` or `|x| x + 1` when types are inferred.
- **`for` loop**: `for n in list { body }` as an expression yielding `()`.
  - Add `Expr::ForIn { var: String, iter: Box<Expr>, body: Box<Expr> }`.
  - Emit as `for n in list.iter() { body }`.
- **Built-in HOFs**: `map(list, f)`, `filter(list, pred)`, `fold(list, init, f)`.
  - These are recognised during codegen as special call forms.
  - `map(xs, f)` → `xs.iter().map(f).collect::<Vec<_>>()`
  - `filter(xs, pred)` → `xs.iter().filter(pred).collect::<Vec<_>>()`
  - `fold(xs, init, f)` → `xs.iter().fold(init, f)`
- Effect checker: lambdas that call effectful functions inherit the parent fn's effect set.
- Type checker: lambdas get a fresh `FnType` in the type environment; HOF calls are
  type-checked by unifying the lambda's input type with the list element type.

### Key files
- `src/ast.rs` — `Expr::Lambda`, `Expr::ForIn`
- `src/parser/mod.rs` — parse `|...| expr` and `fn(...)` lambda syntax, `for n in`
- `src/codegen/rust.rs` — emit lambdas and HOF call forms
- `src/checker/infer.rs` — constraint generation for lambdas and HOF calls
- `src/checker/effects.rs` — propagate effect set through lambdas
- `tests/iteration_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/iteration_test.rs`:
  - `map(xs, |x| x + 1)` emits `xs.iter().map(|x| x + 1).collect::<Vec<_>>()`.
  - `filter(xs, |x| x > 0)` emits filter/collect.
  - `fold(xs, 0, |acc, x| acc + x)` emits fold.
  - `for n in list { ... }` emits a Rust `for` loop.
  - Lambda with inferred type infers the parameter type from context.
  - Lambda inside a pure function calling an effectful function → EffectError.
- E2E test: a Loom program using `map` on `List<Int>` compiles and runs correctly via `rustc`.

### Development prompt

```
Read docs/roadmap.md (M11 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs, src/checker/infer.rs, src/checker/effects.rs.
Phase 1–3 + M9 + M10 are complete. Do not break any existing tests.

Task: Add closures, for-in loops, and map/filter/fold built-ins to Loom.

Context:
- AST Expr variants live in src/ast.rs. Adding variants requires updating every
  match arm that handles Expr in: codegen/rust.rs emit_expr, checker/infer.rs
  collect_free_vars + constrain, checker/effects.rs check_expr.
- The parser's parse_primary is the entry point for new expression forms.
  The pipe `|` character is probably already a lexer token (BinOp or Pipe) — check
  lexer.rs and reuse or add a new Pipe token.
- HOF calls (map/filter/fold) can be handled in emit_expr's Expr::Call arm:
  if the callee is Expr::Ident("map") | "filter" | "fold", emit the chained Rust form.
  Otherwise fall through to the normal emit path.

What to build:
1. Add to src/ast.rs:
   - Expr::Lambda { params: Vec<(String, Option<TypeExpr>)>, body: Box<Expr> }
   - Expr::ForIn { var: String, iter: Box<Expr>, body: Box<Expr> }
2. In src/parser/mod.rs parse_primary:
   - If token is Pipe, parse |param, ...| expr as Lambda.
   - If token is Ident("fn") followed by LParen, parse fn(x: T) -> expr as Lambda.
   - If token is Ident("for"), parse for VAR in EXPR { BODY } as ForIn.
3. In emit_expr:
   - Lambda { params, body } → |p1, p2| { body }
   - ForIn { var, iter, body } → for var in iter.iter() { body }
   - Call { callee: Ident("map"), args: [list, f] } → list.iter().map(f).collect::<Vec<_>>()
   - Call { callee: Ident("filter"), ... } → filter/collect
   - Call { callee: Ident("fold"), args: [list, init, f] } → fold
4. In infer.rs collect_free_vars: Lambda introduces its param names as bound vars
   (do not emit them as free). ForIn introduces its var as bound.
5. In effects.rs: lambda body is checked; if it calls effectful fns, the containing
   fn must declare those effects.
6. Write tests/iteration_test.rs — TDD (failing first).

Constraints:
- cargo test must pass after each phase (max 5 files per phase).
- map/filter/fold HOF emission must not break plain function calls named map/filter/fold
  by the user — only trigger on calls with the exact built-in arity.
- Rust closures capturing variables use move semantics only when necessary; start simple.
```

---

## M12 — Tuples · `Option<T>` · `Result<T,E>` First-Class

### What it is
Complete the core type algebra by adding tuple types, and promoting `Option<T>` and
`Result<T,E>` from type-mapping stubs to first-class matchable types with real pattern
support, constructor expressions, and the `?` propagation operator.

### Scope
- **Tuples**
  - `TypeExpr::Tuple(Vec<TypeExpr>)` — already partially present; verify and complete.
  - Tuple expression `(a, b, c)` — add `Expr::Tuple(Vec<Expr>)`.
  - Tuple destructuring in `let`: `let (a, b) = pair`.
  - Pattern `Pattern::Tuple(Vec<Pattern>)` in `match`.
  - Emit `(T1, T2)` in Rust, `(expr1, expr2)` for tuple values, and destructuring in
    `let (a, b) =`.
- **`Option<T>`**
  - `Some(expr)` and `None` as constructor expressions (currently they only work as
    patterns if the user manually defines an Option-like enum).
  - `Pattern::Some(Box<Pattern>)` and `Pattern::None` recognised by the exhaustiveness
    checker.
  - Emit `Some(x)` and `None` directly; `Option<T>` maps to Rust `Option<T>`.
- **`Result<T,E>`**
  - `Ok(expr)` and `Err(expr)` constructor expressions.
  - `Pattern::Ok(Box<Pattern>)` and `Pattern::Err(Box<Pattern>)`.
  - Exhaustiveness checker understands Ok/Err as covering `Result<T,E>`.
- **`?` propagation**
  - Postfix `?` operator: `Expr::Try(Box<Expr>)`.
  - Parse `expr?` in `parse_postfix`.
  - Emit as `expr?` in Rust.
  - Effect checker: a function using `?` on a `Result`-returning expression must declare
    a compatible return type (or it's a type error).

### Key files
- `src/ast.rs` — `Expr::Tuple`, `Expr::Try`, `Pattern::Tuple`, `Pattern::Some`, `Pattern::None`, `Pattern::Ok`, `Pattern::Err`
- `src/parser/mod.rs` — tuple expressions, `?` postfix
- `src/codegen/rust.rs` — emit tuples, Option/Result constructors, `?`
- `src/checker/exhaustiveness.rs` — Option and Result coverage
- `src/checker/infer.rs` — unify tuples element-wise; infer `Option<T>` / `Result<T,E>`
- `tests/algebraic_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/algebraic_test.rs`:
  - `(1, "hello")` emits `(1i64, "hello".to_string())`.
  - `let (a, b) = pair` emits `let (a, b) = pair;`.
  - `match opt { Some(x) => x | None => 0 }` — exhaustiveness passes and emits correctly.
  - `match res { Ok(v) => v | Err(e) => 0 }` — same.
  - `parse_int(s)?` emits `parse_int(s)?`.
  - Missing `None` arm in `match opt { Some(x) => x }` → `NonExhaustiveMatch` error.
- E2E: a function returning `Option<Int>` with `Some`/`None` match compiles and runs.

### Development prompt

```
Read docs/roadmap.md (M12 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs, src/checker/exhaustiveness.rs, src/checker/infer.rs.
Phase 1–3 + M9–M11 are complete. Do not break any existing tests.

Task: Add tuples, Option<T>, Result<T,E>, and the ? operator to Loom.

Context:
- TypeExpr::Tuple may already exist as a stub — check ast.rs first.
- Pattern variants (Variant, Wildcard, Ident, Literal) live in ast.rs. You need
  Tuple, Some, None, Ok, Err variants (or handle Some/Ok/Err as Variant with name check).
  Prefer using the existing Pattern::Variant mechanism for Some/Ok/Err so the exhaustiveness
  checker already handles them via name-based variant sets — just pre-seed "Option" and
  "Result" into the known-enums table in types.rs.
- The ? operator is a postfix expression. parse_postfix already loops; add a branch for
  the Question token.

What to build:
1. Ensure TypeExpr::Tuple(Vec<TypeExpr>) exists. Add Expr::Tuple(Vec<Expr>) and
   Expr::Try(Box<Expr>).
2. In parse_primary, recognise LParen followed by comma-separated exprs as tuple.
   (Distinguish from parenthesized single expr by presence of a comma.)
3. In parse_postfix, add: if token is Question, consume and return Expr::Try(lhs).
4. Pre-seed the type checker's enum knowledge: "Option" has variants ["Some", "None"];
   "Result" has variants ["Ok", "Err"]. This makes exhaustiveness checking work for free.
5. In emit_expr:
   - Tuple(exprs) → (expr1, expr2, ...)
   - Try(e) → e?
   - Ident("None") → None (already correct), Ident("Some") handled by Call arm.
6. In emit_type_expr, Tuple(ts) → (T1, T2, ...).
7. In infer.rs, unify tuples element-wise.
8. Write tests/algebraic_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- Tuple-vs-paren disambiguation: a single expression in parens is NOT a tuple.
  Only emit Tuple when there is at least one comma inside the parens.
- Do not change the lexer token set if avoidable.
```

---

## Phase 4 Execution Order

```
M9 (inline)   ──── independent; enables real programs today

M10 (coerce)  ──── independent; fixes arithmetic and parser gap

M11 (iter)    ──── depends on M10 (closures use parens and coerce)

M12 (alg)     ──── depends on M11 (? operator uses Result which uses closures)
```

Recommended sequence: **M9 → M10 → M11 → M12**

---

---

## M13 — `describe:` Blocks + Audit Annotations

### What it is
Embed human-readable (and AI-readable) intent directly into the Loom source with
`describe:` blocks and `@`-prefixed audit annotations.  This addresses two GS properties
simultaneously: **Self-describing** (every module and function declares its own purpose)
and **Auditable** (design decisions, deprecations, and version history are traceable from
the source).

> **GS quote**: "A GS artifact is self-describing: it carries within it the rationale and
> boundary of every decision, so a stateless reader can reconstruct intent without external
> context."

### Scope
- **`describe:` block** at module level and at function level.
  - Grammar: `describe: "free-form text"` — a string literal following the keyword.
  - Add `describe: Option<String>` to `Module` and `FnDef` in `src/ast.rs`.
  - Parser: recognise `describe:` as the first item in a module/function header.
  - Codegen: emit the string as a Rust doc comment `///` above the module / function.
- **`@`-annotations** as attribute-like markers in the module/function header:
  - `@since("v0.1.0")` — version this item was introduced.
  - `@decision("rationale")` — embedded design decision note.
  - `@deprecated("use X instead")` — marks the item as deprecated.
  - `@author("name")` — optional attribution.
  - Add `annotations: Vec<Annotation>` to `Module` and `FnDef`.
  - `Annotation` is `{ key: String, value: String }`.
  - Emit as Rust doc comments: `/// @since v0.1.0`, `/// @deprecated: use X instead`.
  - `@deprecated` items additionally emit `#[deprecated(note = "use X instead")]`.
- The `loom compile --verbose` flag (new in this milestone) prints a `describe:` summary
  for each compiled module.
- LSP: hover over a function name returns its `describe:` text as the hover documentation.

### Key files
- `src/ast.rs` — `describe: Option<String>`, `annotations: Vec<Annotation>` on `Module` / `FnDef`
- `src/parser/mod.rs` — parse `describe: "..."` and `@key("value")` in module/fn headers
- `src/codegen/rust.rs` — emit doc comments and `#[deprecated]`
- `src/lsp/mod.rs` — hover response returns `describe:` text
- `tests/describe_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/describe_test.rs`:
  - `describe: "does X"` on a module emits `/// does X` above the Rust module.
  - `describe: "does X"` on a function emits `/// does X` above the Rust function.
  - `@since("1.0")` emits `/// @since 1.0`.
  - `@deprecated("use Y")` emits both `/// @deprecated: use Y` and `#[deprecated(note = "use Y")]`.
  - A module without `describe:` compiles normally (field is optional).
- LSP hover test: hover over a function with `describe:` returns that text as hover docs.
- `corpus/user_service.loom` updated with `describe:` blocks; compiles with no regressions.

### Development prompt

```
Read docs/roadmap.md (M13 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs. Phase 1–3 are complete (142 tests passing). Do not break them.

Task: Add describe: blocks and @-annotations to Loom — GS Self-describing + Auditable.

Context:
- Module is defined in src/ast.rs as a struct with fields: name, requires, provides,
  type_defs, fn_defs. Add describe: Option<String> and annotations: Vec<Annotation>.
- FnDef similarly has name, params, return_type, effects, body. Add describe and annotations.
- The parser's parse_module and parse_fn_def read the header tokens. describe: is
  parsed before fn_defs; @key annotations are parsed between header items.
- Rust doc comments are emitted by prepending "/// line\n" before the item.
  The /// lines must come before any #[...] attributes.

What to build:
1. Add to src/ast.rs:
   - struct Annotation { key: String, value: String }
   - describe: Option<String> and annotations: Vec<Annotation> to Module and FnDef.
2. In parse_module, before parsing fn defs, check for `describe:` token sequence and
   `@`-prefixed annotations; parse and attach them.
3. In parse_fn_def, similarly parse describe and annotations in the fn header.
4. In emit (src/codegen/rust.rs):
   - For each module: if describe is Some, prepend `/// {text}\n` to the module doc.
   - For each fn: same.
   - For each annotation: emit `/// @{key}: {value}\n`.
   - For @deprecated: also emit `#[deprecated(note = "{value}")]\n`.
5. In src/lsp/mod.rs hover handler: if the hovered symbol resolves to a FnDef with
   describe set, return that text as the hover markdown content.
6. Update corpus/user_service.loom with describe blocks on at least 2 functions.
7. Write tests/describe_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- describe: and annotations are strictly optional — all existing Loom code compiles unchanged.
- Do not invent new tokens; reuse Ident for "describe" and handle "@" as a token prefix.
```

---

## M14 — `invariant:` Declarations + Consequence Tiers

### What it is
Add structural invariants that constrain module state and function behaviour, and a
three-tier consequence model (`@pure` · `@reversible` · `@irreversible`) that classifies
every effectful operation by the severity of its side effect.  This implements the GS
**Defended** property: the system's own structure prevents invalid states from being
representable.

> **GS quote**: "A GS artifact is defended: its invariants are not enforced by convention
> but by the grammar of the specification itself."

### Scope
- **`invariant:` declarations** at module level.
  - Grammar: `invariant name :: condition_expr` where `condition_expr` is a Loom boolean
    expression that may reference module-level type fields.
  - Add `invariants: Vec<Invariant>` to `Module` in `src/ast.rs`.
  - `Invariant` is `{ name: String, condition: Expr }`.
  - Codegen: emit each invariant as a `debug_assert!(condition, "invariant '{name}' violated")`
    inside a module-level `fn _check_invariants()` that is called at the start of every
    public function (in debug builds only via `#[cfg(debug_assertions)]`).
  - The type checker validates that the condition expression is a `Bool` expression.
- **Consequence tiers** on effect declarations.
  - Extend `EffectDecl` to accept optional tier: `effect [IO@reversible, DB@irreversible]`.
  - Add `tier: Option<ConsequenceTier>` to `EffectDecl` in `src/ast.rs`.
  - `ConsequenceTier` enum: `Pure | Reversible | Irreversible`.
  - The effect checker enforces: an `@irreversible` effect cannot be called from a function
    whose own declared effect tier is `@reversible` or `@pure`.
  - Codegen: emit tier as a doc comment: `// effect-tier: irreversible`.
- **`@pure` on functions** as shorthand for "this function must have no effects".
  - If a function annotated `@pure` calls any effectful function, the effect checker errors.

### Key files
- `src/ast.rs` — `Invariant`, `ConsequenceTier`, `invariants` on `Module`, `tier` on `EffectDecl`
- `src/parser/mod.rs` — parse `invariant name :: expr` and `effect [X@tier]`
- `src/codegen/rust.rs` — emit `_check_invariants()` and tier doc comments
- `src/checker/effects.rs` — consequence tier enforcement
- `src/checker/types.rs` — validate invariant condition is Bool
- `tests/invariant_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/invariant_test.rs`:
  - `invariant non_negative :: amount >= 0` emits a `debug_assert!` in `_check_invariants`.
  - A function with an `@irreversible` effect called from a `@reversible` function → `EffectError`.
  - `@pure` function calling an IO function → `EffectError`.
  - Module with no invariants compiles unchanged.
- E2E: a module with an `invariant` compiles to valid Rust and runs.

### Development prompt

```
Read docs/roadmap.md (M14 section). Read src/ast.rs, src/checker/effects.rs,
src/codegen/rust.rs, src/parser/mod.rs. Phase 1–3 + M13 complete. Do not break tests.

Task: Add invariant: declarations and consequence tiers to Loom — GS Defended property.

Context:
- Module struct in ast.rs needs invariants: Vec<Invariant> added.
- EffectDecl is currently a simple struct {name: String}. Add tier: Option<ConsequenceTier>.
- ConsequenceTier = Pure | Reversible | Irreversible derives Debug, Clone, PartialEq.
- The @pure annotation from M13 can reuse the annotations Vec<Annotation> mechanism or
  be a dedicated FnDef field — choose the cleaner option.

What to build:
1. Add Invariant { name: String, condition: Expr } and ConsequenceTier to src/ast.rs.
   Add invariants: Vec<Invariant> to Module.
   Add tier: Option<ConsequenceTier> to EffectDecl.
2. In parse_module, parse `invariant NAME :: EXPR` entries before fn_defs.
3. In parse_effect_list, after each effect name check for @tier suffix.
4. In types.rs check_invariant: typecheck condition expr; it must be Bool.
5. In effects.rs:
   - Track each fn's consequence tier from its effect declarations.
   - If fn F calls fn G where G has a tier MORE severe than F's declared tier → EffectError.
   - If fn F is annotated @pure (tier = Pure) and calls any effectful fn → EffectError.
6. In codegen emit, after emitting all fns in a module:
   - If invariants non-empty, emit:
       #[cfg(debug_assertions)]
       fn _check_invariants(...) {
           debug_assert!(condition, "invariant 'name' violated");
           ...
       }
7. Write tests/invariant_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- Invariants reference module types — keep condition checking simple (no full HM re-solve).
- Tier enforcement is additive; existing effect tests must pass without changes.
```

---

## M15 — `test:` Blocks + Real `ensure:` Assertions

### What it is
Promote the two existing specification-as-comment mechanisms — `ensure:` postconditions
(currently emitted as comments) and property-based test sketches — to first-class language
constructs that emit real, runnable assertions.  This implements the GS **Verifiable**
property: every stated postcondition is machine-checkable, not documentation.

> **GS quote**: "A GS artifact is verifiable: every property it asserts about the system
> can be automatically checked against an implementation."

### Scope
- **`ensure:` as real assertions**
  - `ensure:` in a function body currently emits a `// ensure: ...` comment.
  - Change codegen to emit `debug_assert!(condition, "ensure: {text}");` for boolean
    `ensure:` conditions.
  - For `ensure:` with a string description only (no checkable expr), keep as comment.
  - Type checker: validate that `ensure:` conditions are `Bool` expressions.
- **`test:` blocks** — in-language unit tests.
  - Grammar: `test name :: body_expr` at module level (similar to `fn` without params).
  - Add `test_defs: Vec<TestDef>` to `Module`; `TestDef { name: String, body: Expr }`.
  - Emit as `#[test] fn test_name() { body }` inside a `#[cfg(test)] mod tests { ... }`
    block appended to the module.
  - `test` bodies may call any pure function from the same module.
  - Effect checker: `test:` bodies may declare effects (test isolation boundary).
- **Property test syntax** (lightweight, no external crate dependency).
  - `test name :: for_all(|x: Int| invariant_expr(x))` — syntactic sugar.
  - Emit as a loop over a fixed set of generated values (edge cases: 0, 1, -1, MAX, MIN)
    calling the predicate and asserting it holds.
  - This avoids a `proptest`/`quickcheck` dependency while still providing property coverage.

### Key files
- `src/ast.rs` — `TestDef`, `test_defs: Vec<TestDef>` on `Module`
- `src/parser/mod.rs` — parse `test name :: expr` at module level
- `src/codegen/rust.rs` — emit `#[cfg(test)] mod tests { ... }` with `#[test]` fns; real `debug_assert!` for `ensure:`
- `src/checker/types.rs` — validate `ensure:` conditions are Bool
- `src/checker/effects.rs` — test body effect isolation
- `tests/testblock_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/testblock_test.rs`:
  - `ensure: result > 0` in a function body emits `debug_assert!(result > 0, ...)`.
  - `test my_test :: assert_eq!(add(1, 2), 3)` emits `#[test] fn my_test() { assert_eq!(...) }`.
  - A module with `test:` blocks compiles and its emitted Rust tests pass when run with `cargo test`.
  - `test prop :: for_all(|x: Int| x * 2 >= x)` emits a loop over edge-case values.
  - `ensure:` with a non-Bool expression → `TypeError`.
- E2E: the emitted `#[cfg(test)]` tests are runnable and pass (verified by running `rustc` + the test binary).

### Development prompt

```
Read docs/roadmap.md (M15 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs, src/checker/types.rs. Phase 1–3 + M13–M14 complete. No regressions.

Task: Upgrade ensure: to real assertions and add test: blocks — GS Verifiable property.

Context:
- ensure: in the current codebase is parsed as a special Expr variant or as a comment
  in the function header. Check src/ast.rs and src/parser for the current handling.
  If ensure: is already an Expr variant with a condition, just change emit_expr to emit
  debug_assert! instead of a comment.
- test: blocks are structurally identical to fn defs without parameters. They live at
  module level alongside fn_defs.
- #[cfg(test)] mod tests { ... } must be appended to the module body string in emit(),
  only if test_defs is non-empty.

What to build:
1. Add TestDef { name: String, body: Expr } to src/ast.rs.
   Add test_defs: Vec<TestDef> to Module.
2. In parse_module, parse `test NAME :: EXPR` entries (same level as fn_defs).
3. In codegen emit():
   a. Change ensure: Expr emission from comment to:
      `debug_assert!({condition}, "ensure: {description}");`
   b. After the main module body, if test_defs non-empty, append:
      `#[cfg(test)] mod tests {\n use super::*;\n {test fns}\n }`
      where each test fn is: `#[test]\n fn {name}() {{ {body} }}`
4. In types.rs, validate ensure: conditions are Bool (add a check in check_fn_body).
5. For for_all(|x: T| pred(x)) property tests: recognise this call form in emit_expr
   and emit a loop body testing at EDGE_CASES = [0, 1, -1, i64::MAX, i64::MIN].
6. Write tests/testblock_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- The #[cfg(test)] block is only emitted when test_defs is non-empty (no empty mod).
- for_all is syntactic sugar only — it does not need a real runtime implementation,
  just the edge-case loop.
- Do not add external test-framework crates.
```

---

## M16 — `import` Declarations + Explicit `interface`

### What it is
Add explicit module import syntax and named interface declarations so that cross-module
dependencies and composition contracts are first-class language constructs rather than
inferred from `requires`/`provides` names.  This completes the GS **Composable** property:
every boundary between modules is explicit, typed, and verifiable.

> **GS quote**: "A GS artifact is composable: its boundaries are explicit and typed so
> that any valid composition of two artifacts produces a valid combined specification."

### Scope
- **`import ModuleName` declarations** at the top of a module.
  - Grammar: `import ModuleName` (one per line, before `module` keyword or after it).
  - Add `imports: Vec<String>` to `Module`.
  - In multi-module project compilation (`src/project.rs`), use imports to build the
    dependency graph instead of (or in addition to) `requires` names.
  - Codegen: emit `use super::module_name::*;` at the top of the Rust module.
  - Circular import detection: add `LoomError::CircularImport` (extends the existing
    `CyclicDependency` error from M8).
- **`interface` declarations** — named, reusable capability contracts.
  - Grammar:
    ```
    interface PaymentGateway
      fn charge :: (Amount, Card) -> Result<Receipt, PaymentError>
      fn refund :: (ReceiptId) -> Result<(), PaymentError>
    end
    ```
  - Add `InterfaceDef { name: String, methods: Vec<FnSignature> }` to `Module`.
  - `implements InterfaceName` in a module header declares conformance.
  - The type checker validates that every method in the interface is implemented by
    a matching `fn` in the module (name + type signature match).
  - Codegen: emit interface as a Rust `trait`; `implements` emits a `impl Trait for ModuleContext`.
  - This supersedes the existing `provides` mechanism for capability contracts (the old
    `provides` continues to work but `interface` is the preferred form going forward).

### Key files
- `src/ast.rs` — `imports: Vec<String>`, `InterfaceDef`, `implements: Vec<String>` on `Module`
- `src/parser/mod.rs` — parse `import NAME` and `interface ... end` blocks
- `src/codegen/rust.rs` — emit `use super::*`, `trait`, `impl Trait`
- `src/checker/types.rs` — validate `implements` conformance
- `src/project.rs` — use `imports` for dependency graph
- `tests/interface_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/interface_test.rs`:
  - `import MathLib` emits `use super::math_lib::*;` in the Rust output.
  - An `interface Greeter` with one method, `implements Greeter` on a module with that
    method → compiles cleanly.
  - `implements Greeter` on a module missing a method → `TypeError: missing method 'greet'`.
  - Circular import A → B → A → `CircularImport` error.
  - Existing `provides`-based corpus examples compile unchanged.
- E2E: a two-module Loom project using `import` and `interface` compiles end-to-end.

### Development prompt

```
Read docs/roadmap.md (M16 section). Read src/ast.rs, src/parser/mod.rs,
src/codegen/rust.rs, src/checker/types.rs, src/project.rs. Phase 1–3 + M13–M15 done.

Task: Add import declarations and interface definitions to Loom — GS Composable property.

Context:
- Module already has requires: Vec<String> and provides: Vec<FnSignature> for DI.
  import is a complementary mechanism for inter-module file dependencies.
  requires = runtime DI; import = compile-time module reference.
- InterfaceDef methods are FnSignature (name + type) without bodies — they are contracts.
- The type checker's validate_implements pass: for each name in module.implements,
  find the InterfaceDef (from this module or an imported one), then check that each
  method has a matching FnDef in the implementing module.

What to build:
1. Add to src/ast.rs:
   - imports: Vec<String> to Module
   - InterfaceDef { name: String, methods: Vec<FnSignature> }
   - interface_defs: Vec<InterfaceDef> to Module
   - implements: Vec<String> to Module
2. In parse_module header, parse `import NAME` lines and `implements NAME` declarations.
3. Parse `interface NAME ... end` blocks producing InterfaceDef.
4. In codegen emit():
   - Emit `use super::{snake_case(name)}::*;\n` for each import at module top.
   - Emit each InterfaceDef as a Rust `pub trait`.
   - For each implements name, emit `impl {name} for {ModuleContext} { ... }` delegating
     each method to the corresponding fn.
5. In types.rs add validate_implements: for each implements name, check method coverage.
6. In project.rs build graph: also add edges from import names.
7. Detect cycles including import edges; emit CircularImport error.
8. Write tests/interface_test.rs — TDD.

Constraints:
- cargo test must pass after each phase.
- provides continues to work unchanged — interface is the preferred NEW form.
- snake_case conversion for module names: "MathLib" → "math_lib".
```

---

## Phase 5 Execution Order

```
M13 (describe)   ──── independent; pure syntax sugar, no logic changes

M14 (invariant)  ──── depends on M13 (annotations mechanism)

M15 (test)       ──── depends on M14 (ensure: is a Defended construct)

M16 (interface)  ──── can run in parallel with M13–M15
```

Recommended sequence: **M13 → M14 → M15 ‖ M16**

---

---

## M17 — TypeScript Emission Target

### What it is
Add a second emission target so that the exact same Loom specification generates both
a Rust implementation (for the server/core) and TypeScript types (for the client/API layer).
One mold, two foundry outputs.  This is the first concrete proof of the GS derivation
principle: the spec is target-agnostic; the targets are derived from it.

> **GS quote**: "The mold is not the output. The mold constrains the output. Any valid
> foundry can pour from the same mold."

### Scope
- **`loom compile --target typescript`** (or `--target ts`).
  - Extend the CLI flag (already `--target [rust|wasm]` from M3) with `typescript`.
- **Create `src/codegen/typescript.rs`** with a `TypeScriptEmitter`:
  - `emit(&Module) -> String` producing a `.ts` file.
  - Loom `type Foo { field: Type }` → TypeScript `interface Foo { field: TypeScriptType }`.
  - Loom `enum Color { Red | Green | Blue }` → TypeScript `type Color = "Red" | "Green" | "Blue"` (string union).
  - Loom `fn name :: (params) -> ReturnType` → TypeScript `declare function name(params): ReturnType` (declaration stub).
  - Loom `provides [fn1, fn2]` → `export { fn1, fn2 }`.
  - Loom effect annotations → TypeScript JSDoc comment: `/** @effect IO, DB */`.
  - Loom `describe: "..."` → TypeScript JSDoc `/** ... */`.
  - Loom `Option<T>` → `T | null`.
  - Loom `Result<T, E>` → `{ ok: true; value: T } | { ok: false; error: E }`.
  - Loom `List<T>` → `T[]`, `Map<K,V>` → `Record<K, V>`, `Set<T>` → `Set<T>`.
- **Type mapping table** (`src/codegen/typescript.rs`):
  - `Int` → `number`, `Float` → `number`, `String` → `string`, `Bool` → `boolean`.
- **`loom compile --target typescript --out-dir ./types`** writes `{ModuleName}.ts` files.

### Key files
- `src/codegen/typescript.rs` — new emitter
- `src/codegen/mod.rs` — expose `TypeScriptEmitter`
- `src/main.rs` — add `typescript` to `--target` enum
- `src/lib.rs` — expose `compile_ts(&str) -> Result<String, Vec<LoomError>>`
- `tests/typescript_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/typescript_test.rs`:
  - `type User { name: String, age: Int }` emits `interface User { name: string; age: number; }`.
  - `enum Status { Active | Inactive }` emits `type Status = "Active" | "Inactive";`.
  - `fn greet :: String -> String` emits `declare function greet(arg0: string): string;`.
  - `Option<Int>` maps to `number | null`.
  - `Result<String, Error>` maps to `{ ok: true; value: string } | { ok: false; error: Error }`.
  - `List<Int>` maps to `number[]`.
  - `loom compile --target typescript` CLI flag produces a `.ts` file.
- The emitted TypeScript is valid: run `tsc --noEmit --strict` on the output and get zero errors.

### Development prompt

```
Read docs/roadmap.md (M17 section). Read src/ast.rs, src/codegen/rust.rs,
src/codegen/mod.rs, src/main.rs, src/lib.rs.
Phase 1–3 + Phase 5 milestones are complete. Do not break existing tests.

Task: Add a TypeScript emission target to the Loom compiler — GS multi-target derivation.

Context:
- The existing RustEmitter in src/codegen/rust.rs implements emit(&Module) -> String.
  Create TypeScriptEmitter with the same interface in src/codegen/typescript.rs.
- The Module AST contains: name, type_defs, fn_defs, enum_defs, provides, describe,
  annotations, test_defs, interface_defs. All are available for TS emission.
- TypeScript type mappings for Loom primitives:
    Int/Float → number, String → string, Bool → boolean,
    Option<T> → T | null, Result<T,E> → {ok:true;value:T}|{ok:false;error:E},
    List<T> → T[], Map<K,V> → Record<K,V>, Set<T> → Set<T>

What to build:
1. Create src/codegen/typescript.rs:
   - pub struct TypeScriptEmitter;
   - pub fn emit(&self, module: &Module) -> String
   - fn emit_type_def(td: &TypeDef) -> String  → interface
   - fn emit_enum_def(ed: &EnumDef) -> String  → type union
   - fn emit_fn_sig(fd: &FnDef) -> String      → declare function
   - fn emit_type_expr(te: &TypeExpr) -> String → TS type string
2. Expose TypeScriptEmitter from src/codegen/mod.rs.
3. Add TypeScript variant to the target enum in src/main.rs CLI.
   When selected, call TypeScriptEmitter::new().emit(&module) and write .ts output.
4. Add compile_ts(source: &str) -> Result<String, Vec<LoomError>> to src/lib.rs.
5. Write tests/typescript_test.rs — TDD (failing first, then implement).
6. Verify the emitted TS with tsc --noEmit (requires tsc in PATH; skip gracefully if absent).

Constraints:
- cargo test must pass after each phase.
- TypeScript emission is additive — zero changes to Rust emission.
- Function bodies are NOT emitted in TypeScript — only declarations (declare function).
  The Rust target emits the implementation; TypeScript emits the type contract.
- Inline blocks are ignored in TS output (they are Rust-specific implementations).
```

---

## M18 — Contract Materialisation (OpenAPI · JSON Schema)

### What it is
The final milestone completes the GS derivation chain: a Loom `provides` block IS an
API contract.  Add a `loom schema` command that materialises the full contract surface
of a Loom module into an OpenAPI 3.0 YAML document and JSON Schema `$defs`, with no
additional annotations required.  If the spec is written correctly, the schema is free.

> **GS quote**: "The specification is the contract. The API documentation is not a
> separate artifact — it is the specification rendered in a different notation."

### Scope
- **`loom schema` command** (new sub-command).
  - Reads a single `.loom` file or a `loom.toml` project.
  - Emits `{module_name}.openapi.yaml` and `{module_name}.schema.json` to the output directory.
- **`src/codegen/schema.rs`** with `OpenApiEmitter` and `JsonSchemaEmitter`.
- **OpenAPI 3.0 emission**:
  - Each `provides [fn_name]` entry generates an OpenAPI path item.
  - Function parameter types → request body schema (`application/json`).
  - Return type → `200` response schema.
  - Effect declarations → `x-effects` extension field.
  - `describe:` → OpenAPI `description` field.
  - `@deprecated` → OpenAPI `deprecated: true`.
  - Loom `ensure:` conditions → OpenAPI parameter constraints where mappable
    (e.g., `ensure: amount > 0` → `minimum: 1`).
- **JSON Schema `$defs` emission**:
  - Each `type` definition → a `$def` with `type: object` and `properties`.
  - Each `enum` → a `$def` with `enum: [...]`.
  - Refined types (`Int where x > 0`) → JSON Schema `minimum`/`maximum`/`pattern` constraints.
  - `Option<T>` → `oneOf: [T schema, {type: null}]`.
  - `Result<T, E>` → `oneOf: [{ok: true, value: T}, {ok: false, error: E}]`.
- **CLI `loom schema --format [openapi|jsonschema|both]`** (default: `both`).

### Key files
- `src/codegen/schema.rs` — `OpenApiEmitter` + `JsonSchemaEmitter`
- `src/codegen/mod.rs` — expose both emitters
- `src/main.rs` — add `schema` sub-command
- `src/lib.rs` — expose `compile_schema(source: &str, format: SchemaFormat) -> Result<String, Vec<LoomError>>`
- `tests/schema_test.rs` — new test file

### Success criteria
- `cargo test` passes with zero regressions.
- New tests in `tests/schema_test.rs`:
  - A `type User { name: String, age: Int }` emits correct JSON Schema `$def` with `string`/`number` properties.
  - A `provides [get_user]` fn emits an OpenAPI path with correct request/response shapes.
  - `@deprecated` on a fn emits `deprecated: true` in OpenAPI.
  - `describe: "..."` emits as `description:` in OpenAPI.
  - `Int where x > 0` (refined) emits `minimum: 1` in JSON Schema.
  - `loom schema` CLI produces a `.yaml` and a `.json` file.
- The emitted OpenAPI YAML is schema-valid: run it through the OpenAPI validator
  (`npx @redocly/cli lint`) and get zero errors.

### Development prompt

```
Read docs/roadmap.md (M18 section). Read src/ast.rs, src/codegen/typescript.rs (M17),
src/codegen/mod.rs, src/main.rs, src/lib.rs.
Phase 1–5 + M17 are complete. Do not break existing tests.

Task: Add loom schema command — OpenAPI + JSON Schema derivation from Loom specs.

Context:
- This milestone completes the GS derivation chain. The same Module AST that drives
  Rust and TypeScript emission now drives schema generation.
- OpenAPI 3.0 YAML structure:
    openapi: "3.0.3"
    info: { title: ModuleName, version: "0.1.0" }
    paths:
      /{fn_name}:
        post:
          description: {describe text}
          requestBody: { content: { application/json: { schema: { $ref: '#/components/schemas/Input' } } } }
          responses:
            200: { content: { application/json: { schema: { ... } } } }
- JSON Schema structure:
    $schema: https://json-schema.org/draft/2020-12/schema
    $defs:
      TypeName: { type: object, properties: { field: { type: ... } } }
- For refined types (TypeExpr::Refined { base, constraint }), parse the constraint
  expression and map to JSON Schema keywords:
    x > N  → minimum: N+1
    x >= N → minimum: N
    x < N  → maximum: N-1
    x <= N → maximum: N
    x matches "pattern" → pattern: "..."

What to build:
1. Create src/codegen/schema.rs:
   - pub struct OpenApiEmitter; with emit(&Module) -> String (YAML string)
   - pub struct JsonSchemaEmitter; with emit(&Module) -> String (JSON string)
   - fn emit_type_to_json_schema(te: &TypeExpr) -> serde_json::Value
   - fn emit_type_def_to_schema(td: &TypeDef) -> serde_json::Value
   - fn refined_constraint_to_schema(expr: &Expr) -> serde_json::Value
2. Add serde_json and serde_yaml crate dependencies to Cargo.toml.
3. Add schema sub-command to src/main.rs with --format flag.
4. Add compile_schema to src/lib.rs.
5. Write tests/schema_test.rs — TDD.
6. Run npx @redocly/cli lint on the emitted YAML in a test (skip gracefully if npx absent).

Constraints:
- cargo test must pass after each phase.
- Use serde_json for JSON Schema (structured) and manual YAML string building or serde_yaml
  for OpenAPI — choose the simpler approach.
- Only functions in the `provides` list become OpenAPI paths. Internal fns are omitted.
- Gracefully handle modules with no provides (emit empty paths: {}).
```

---

## Phase 6 Execution Order

```
M17 (typescript)  ──── depends on Phase 5 (describe/annotations enrich TS output)

M18 (schema)      ──── depends on M17 (schema emitter shares type mapping logic)
```

Recommended sequence: **M17 → M18**

---

---

## Full Roadmap Execution Order (Phases 4–6)

```
Phase 4:  M9 → M10 → M11 → M12
Phase 5:  M13 → M14 → M15 ‖ M16  (M16 can run in parallel with M13–M15)
Phase 6:  M17 → M18
```

Each phase is a prerequisite for the next. Phase 5 requires Phase 4 complete.
Phase 6 requires Phase 5 complete.

## GS Property Coverage by Milestone

| GS Property | Current | Phase 4 | Phase 5 | Phase 6 |
|-------------|---------|---------|---------|---------|
| Self-describing | ⚠️ Partial | ⚠️ Partial | ✅ M13 | ✅ |
| Bounded | ✅ | ✅ | ✅ | ✅ |
| Verifiable | ⚠️ Partial | ⚠️ Partial | ✅ M15 | ✅ |
| Defended | ⚠️ Partial | ⚠️ Partial | ✅ M14 | ✅ |
| Auditable | ❌ Missing | ❌ | ✅ M13 | ✅ |
| Composable | ✅ Good | ✅ | ✅ M16 | ✅ |
| Executable | ⚠️ Partial | ✅ M9–M12 | ✅ | ✅ |
