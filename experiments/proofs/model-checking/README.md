# Proof: Model Checking (Clarke, Emerson & Sifakis, 1981)

**Theory:** Model checking exhaustively explores all possible states of a system and verifies that temporal logic properties hold in every state. Unlike testing (which checks specific inputs), model checking is complete within its bounds.  
**Claim:** Loom's Kani integration emits `#[kani::proof]` harnesses. Kani uses symbolic execution (CBMC) to verify ALL possible inputs satisfy the declared contracts — not just the ones you thought to test.  
**Turing Award:** Clarke, Emerson & Sifakis, 2007.  
**Status:** 🔶 EMITTED — Kani required (Linux/WSL).

## The key difference from testing

| Approach | `safe_increment(127)` | `safe_increment(255)` | ALL values 0–255 |
|---|---|---|---|
| Unit test | ✅ checked | ✅ checked | ❌ only if written |
| Model checking | ✅ | ✅ | ✅ **all automatically** |

Kani's `kani::any()` generates a symbolic value representing ALL possible values. The verifier proves the property holds for every one of them simultaneously.

## How to run

```bash
# Standard tests (specific inputs)
loom compile proof.loom -o proof.rs
cargo test

# Model checking — ALL inputs (Linux/WSL required)
cargo kani --harness model_check_safe_increment_all_counters
cargo kani --harness model_check_mutex_all_states
```

## Layman explanation

Testing is like checking 10 locks in a building. Model checking is like proving that *every* lock in the building satisfies the fire code — all 10,000 of them — by verifying the general design, not each individual lock. The 2007 Turing Award was given for this insight: for finite-state systems, you can check ALL states automatically, not just the ones a human thought to test.
