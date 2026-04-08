# ADR-0006: Checker Interface and the LoomChecker Trait

**Date**: 2026-04-08  
**Status**: Accepted  
**Supersedes**: The 263-line imperative `compile()` function

## Context

When Loom reached M112, `compile()` had grown to 263 lines of hardcoded checker
calls with three inconsistent signatures:

```rust
// Pattern 1 — fails with ?
checker::TypeChecker::new().check(&module)?;

// Pattern 2 — needs manual filter
let errs = checker::StoreChecker::new().check(&module)
    .into_iter().filter(|e| !e.to_string().contains("[hint]")).collect();
if !errs.is_empty() { return Err(errs); }

// Pattern 3 — static, no self, out-param
checker::RandomnessChecker::check(&module, &mut errors);
```

This violated DI (no injection point), SOLID/OCP (adding a checker required
modifying `compile()`), and made sub-pipelines (WASM, TypeScript) copy-paste
subsets of stages by hand.

## Decision

`pub trait LoomChecker: Send + Sync` with a single method
`fn check_module(&self, module: &Module) -> Vec<LoomError>`.

Rules:
1. Every new checker MUST implement `LoomChecker`.
2. Checkers returning `Result<(), Vec<LoomError>>` get a blanket impl via
   `impl_result_checker!` macro in `loom_checker.rs`.
3. Checkers returning `Vec<LoomError>` get a blanket impl via `impl_vec_checker!`.
4. Outlier checkers (no `self`, out-param, different input) get named adapter
   unit structs (`SafetyCheckerAdapter`, `TeleosCheckerAdapter`, etc.).
5. Warning suppression is encoded in the pipeline via `CheckerStage::warn_only()`
   or `CheckerStage::suppressing()`, NOT inside the checker.
6. The SMT bridge stays inline in `compile()` because its output type is
   `Vec<SmtVerification>`, not `Vec<LoomError>`.

## Alternatives Considered

| Option | Reason rejected |
|---|---|
| Change all checker signatures to `Vec<LoomError>` | Would break existing tests that use `checker.check()` directly; too much churn for a refactor |
| `trait LoomChecker { fn check(&self, ...) }` | Name collision with existing `.check()` method on every struct |
| Return `Result<(), Vec<LoomError>>` from trait | Makes warn-only stages awkward — how do you return "errors" that don't fail? |
| Single `Checker` enum | Each variant needs a match arm to dispatch; as bad as the original if/else chain |

## Consequences

- `compile()` is now ~100 lines of declarative pipeline. Adding a checker = 1 line.
- Sub-pipelines (`compile_wasm`, `compile_typescript`) can be composed from the
  same stage constructors.
- The `suppress` string-prefix mechanism is a known debt. Future: add a
  `Severity` field to `LoomError` and filter by enum value instead of substring.
- Adapter structs (`SafetyCheckerAdapter` etc.) are boilerplate debt. Future:
  normalise those checkers' signatures to match the trait directly.
- **AI-aware**: when adding a new checker, do NOT add it as a one-off call in
  `compile()`. Implement `LoomChecker`, add it to the blanket macro in
  `loom_checker.rs`, and add a `CheckerStage` to the pipeline. The pattern
  is established — follow it.
