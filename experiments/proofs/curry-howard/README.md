# Proof: Curry-Howard Isomorphism (Curry 1934, Howard 1969)

**Theory:** There is a deep correspondence between mathematical logic and type theory: propositions correspond to types, and proofs correspond to programs. Writing a function of type `A → A` is the same as proving "A implies A."  
**Claim:** Loom's `proof:` annotation makes this explicit. A function with `proof: "A implies A"` is verified by the Loom checker to have the declared type, and the body is the constructive proof. Emits Dafny stubs for formal verification.  
**Status:** 🔶 EMITTED — type verification in Rust; formal proof in Dafny.

## The correspondence table

| Logic | Type Theory | Loom/Rust |
|---|---|---|
| Proposition A | Type `A` | Any Rust type |
| Proof of A | Program of type `A` | A value of type `A` |
| A implies B | Function `A → B` | `fn(A) -> B` |
| A and B | Product `(A, B)` | Tuple `(A, B)` |
| A or B | Sum `Either<A, B>` | `enum Either<A, B>` |
| False | Empty type | `!` (never type) |

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test identity_is_proof_of_a_implies_a ... ok
test fst_is_proof_of_conjunction_implies_left ... ok
test left_is_proof_of_disjunction_introduction ... ok
test transitivity_is_proof_of_implication_chain ... ok
test swap_is_proof_of_commutativity ... ok
```

## Layman explanation

A mathematical proof is just a very careful argument. A computer program is also a very careful argument — it has to be exactly right or it won't compile. Curry and Howard showed these are the *same thing*: writing a function that takes an A and returns a B is the same as proving "if A is true, then B is true." The type checker IS the proof checker. Every program that compiles is a proof that its type (its proposition) is true.
