# ADR-0004: Sequential Fail-Fast Checker Pipeline

**Date**: 2026-04-08  
**Status**: Accepted

## Context

After parsing, Loom runs ~40 semantic-analysis passes (type inference,
exhaustiveness, effect checking, biological validity, documentation liveness,
etc.). These passes need an architecture that:

1. Accumulates errors from a single pass before stopping (not just the first).
2. Short-circuits after a pass that produces hard errors (later passes may
   produce nonsensical results on an already-broken module).
3. Allows warning-only passes (M111 evolution vectors, M112 semantic memory)
   that inform but never block compilation.
4. Is composable: sub-pipelines for WASM, TypeScript, NeuroML targets share a
   subset of the stages.
5. Is auditable: the pipeline is a data structure, not 263 lines of imperative
   code.

## Decision

All checkers implement `pub trait LoomChecker { fn check_module(&self, module: &Module) -> Vec<LoomError>; }`.
The pipeline in `compile()` is a slice of `CheckerStage` values:

```rust
let pipeline: &[CheckerStage] = &[
    CheckerStage::hard(TypeChecker::new()),
    CheckerStage::warn_only(ManifestChecker::new()),
    CheckerStage::suppressing(StoreChecker::new(), &["[hint]", "[warn]", "[info]"]),
    ...
];
for stage in pipeline { stage.run(&module)?; }
```

Each `CheckerStage` holds a `Box<dyn LoomChecker>` and a `suppress: &'static [&'static str]`
filter. The `run()` method filters diagnostics whose string representation contains
a suppressed prefix before deciding whether to return `Err`.

Four outlier checkers with non-standard signatures (`SafetyChecker`, `TeleosChecker`,
`RandomnessChecker`, `StochasticChecker`) are wrapped in adapter unit structs.
The SMT bridge remains inline because its result type is `Vec<SmtVerification>`,
not `Vec<LoomError>`.

## Alternatives Considered

| Option | Reason rejected |
|---|---|
| **Parallel checker execution** | Checkers share no state, but error accumulation order matters for UX; parallel results would be non-deterministic in ordering |
| **Single mega-checker** | Already existed (263-line `compile()`). Removed because it was not composable or auditable |
| **Event-driven / reactive pipeline** | Over-engineered; Loom is a batch compiler, not a language server in hot path |
| **Separate `is_warning` field on `LoomError`** | Would require changing all 50+ checker return types; prefix convention is simpler and already established |

## Consequences

- `compile()` shrinks from 263 lines to ~100 lines of declarative pipeline.
- Adding a new checker = implementing `LoomChecker` + one `CheckerStage::*()` line.
- Sub-pipelines (`compile_wasm`, `compile_typescript`) can share stages by composing
  subsets of `CheckerStage` values.
- The `suppress` mechanism is string-based (fragile if diagnostic text changes).
  Future: replace with a structured `Severity` field on `LoomError`.
- **AI-aware**: when adding a new milestone checker, implement `LoomChecker`,
  add it to `checker/mod.rs` re-exports, and add a `CheckerStage` line to the
  pipeline in `lib.rs`. Do not revert to the old imperative pattern.
