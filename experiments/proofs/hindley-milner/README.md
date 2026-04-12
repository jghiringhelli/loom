# Proof: Hindley-Milner Type Inference (Robin Milner & Luis Damas, 1978)

**Theory:** Every well-typed expression has a unique *principal type* — the most general type that all valid types are instances of. This type can be inferred without any annotations.  
**Claim:** Loom's `InferenceEngine` implements Algorithm W, correctly assigning the most general type to every expression. Type mismatches are compile-time errors.  
**Turing Award:** Robin Milner, 1991.

## What is being proved

**The principal type property:** `identity` has type `∀a. a → a`. This is more general than `Int → Int` or `String → String`. Any call to `identity` is type-checked at the call site. The compiler never needs you to write the type annotation — it infers it.

**Loom's InferenceEngine** uses Algorithm W with unification. When two types cannot be unified (e.g., `Int` and `String` in an addition), the compiler emits `LoomError::TypeMismatch` with the conflicting types.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test identity_is_polymorphic ... ok
test compose_chains_functions ... ok
test map_preserves_length_and_transforms ... ok
test fold_accumulates_correctly ... ok
test sum_list_infers_f64 ... ok
test int_to_string_list_infers_correctly ... ok
test principal_type_is_most_general ... ok
```

## Layman explanation

When you write "let x = 3 + 4", you don't say "let x: Int = 3 + 4." The compiler figures it out. Hindley-Milner showed this can always be done — and always finds the *most general* correct type, not just *a* correct type. This means generic functions like `identity` work for any type, and the compiler verifies this without you writing a single type annotation.
