# Proof: Algebraic Effects (Gordon Plotkin & John Power, 2001)

**Theory:** Side effects (IO, DB, network) can be treated as typed algebraic operations with handlers that eliminate them — giving pure functional semantics to effectful programs.  
**Claim:** Loom's `effect: [IO, DB]` declaration tracks effects in the type. A pure function calling an effectful one is a compile-time error in the Loom checker.  
**Turing Award:** Gordon Plotkin, 2023.

## What is being proved

**Plotkin's insight:** Effects are not exceptions or global state — they are typed operations that propagate up the call stack until a handler eliminates them. This makes effects composable, testable, and statically verifiable.

**Correct:** effectful function called from effectful context, or through a handler → compiles.  
**Violation:** effectful function called from pure context (no `effect:` declaration) → `LoomError::UncheckedEffect`.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test pure_fn_has_no_effect_wrapper ... ok
test effectful_fn_requires_handler_to_extract_value ... ok
test combined_effects_handled_together ... ok
```

## Layman explanation

Like ingredient labels on food. A "contains nuts" label on a dish means you must handle it (serve only to non-allergic guests) before it reaches someone who can't. A dish with no nuts label can go anywhere. Algebraic effects are the type-level "contains nuts" label for side effects — and the compiler checks that every effect is handled before it reaches a pure context.
