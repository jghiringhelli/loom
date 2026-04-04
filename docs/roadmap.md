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
