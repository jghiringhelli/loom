# Proof: Liskov Substitution Principle (Barbara Liskov & Jeannette Wing, 1987)

**Theory:** If S is a subtype of T, then objects of type S may be substituted for objects of type T without altering the correctness of the program.  
**Claim:** Loom's `interface` + `implements` enforces this structurally. Missing methods are `LoomError::MissingInterfaceMethod`. Weakened postconditions are caught by the contract checker.  
**Turing Award:** Barbara Liskov, 2008.

## What is being proved

**LSP guarantee:** Every function that accepts a `Shape` works correctly with `Square`, `Circle`, and any future implementation — because the compiler verifies all three conditions:
1. All interface methods are implemented (completeness)
2. Preconditions are not strengthened (no extra `require:`)  
3. Postconditions are not weakened (all `ensure:` hold)

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test square_satisfies_shape_contract ... ok
test circle_satisfies_shape_contract ... ok
test lsp_total_area_works_with_any_shape ... ok
test all_implementations_are_substitutable ... ok
```

## Layman explanation

If your recipe calls for "any flour," you should be able to use wheat flour, rice flour, or almond flour and get a valid result. LSP says: if a type claims to be a Shape, it must behave exactly like a Shape in every context. The Loom compiler checks this — you cannot publish a `Triangle` that claims to be a `Shape` but doesn't implement `perimeter`.
