# Proof: Hoare Logic (Tony Hoare, 1969)

**Theory:** Every function with `require:` / `ensure:` encodes a Hoare triple `{P} C {Q}`.  
**Claim:** The Loom compiler enforces preconditions and postconditions as `debug_assert!` in Rust and as `#[kani::proof]` harnesses for formal verification.  
**Turing Award:** Tony Hoare, 1980.

## What is being proved

A Hoare triple states: if precondition **P** holds before executing **C**, then postcondition **Q** holds after.  
In Loom, `require:` is **P**, `ensure:` is **Q**, and the function body is **C**.

**Correct program (compiles, Kani verifies):**
```
loom compile proof.loom
cargo kani --harness hoare_withdraw_proof
```

**Violation (detected at compile/verify time):**  
Remove `require: balance >= amount` → Kani finds a counterexample where `result < 0`.

## How to run

```bash
# Compile to Rust
loom compile proof.loom -o proof.rs

# Build
cargo build

# Kani formal verification (Linux / WSL)
cargo kani --harness hoare_withdraw_proof
cargo kani --harness hoare_div_proof
cargo kani --harness hoare_clamp_proof
```

## Expected result

```
VERIFICATION SUCCESSFUL for hoare_withdraw_proof
VERIFICATION SUCCESSFUL for hoare_div_proof
VERIFICATION SUCCESSFUL for hoare_clamp_proof
```

## Layman explanation

Think of a contract: "I'll only accept this job if you pay me upfront, and I guarantee delivery by Friday."  
`require:` is "pay me upfront." `ensure:` is "delivery by Friday." The compiler is the notary — it refuses to stamp the contract if either side is missing.
