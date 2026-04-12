# Proof: Dependent Types (Per Martin-Löf, 1975)

**Theory:** In dependent type theory, types can depend on *values*. The canonical example: `Vec(n, T)` is a vector of `T` whose *length `n` is part of the type*. The `append` function has type `Vec(n, T) → Vec(m, T) → Vec(n+m, T)` — the output length is computed from the input lengths in the type.  
**Claim:** Loom's `dependent:` block encodes these constraints. Rust const generics are the runtime approximation; Dafny stubs are emitted for the full formal proof.  
**Pioneer:** Per Martin-Löf (1975). Practical adoption: Agda, Coq, Lean, Idris.  
**Status:** 🔶 EMITTED — Rust const generics approximate; Dafny required for formal proof.

## Why dependent types matter

| Bug | With dependent types |
|---|---|
| `append(3-element array, 2-element array)` but expect 6 elements | Type error at compile time |
| `array[5]` on a 5-element array (off-by-one) | Type error if using `Fin(n)` index |
| Pass wrong-length vector to FFT | Type error (FFT requires `Vec(power_of_2, T)`) |

## How to run

```bash
# Rust encoding (const generics approximation)
loom compile proof.loom -o proof.rs
cargo test

# Formal proof (emitted Dafny)
# dafny verify proof.dfy  (requires Dafny installation)
```

Expected:
```
test length_is_in_type ... ok
test append_length_is_sum ... ok
test replicate_exact_length ... ok
test safe_index_no_panic ... ok
```

## Layman explanation

Normally, the type `List` just means "a list." You don't know if it has 3 elements or 300 until you check at runtime — and that's where crashes come from. Dependent types let you write `List(5)` in the *type* and have the compiler enforce it. If your function says it returns a `List(n + m)`, the compiler verifies that every possible path through your code produces exactly `n + m` elements — not just the paths you remembered to test.
