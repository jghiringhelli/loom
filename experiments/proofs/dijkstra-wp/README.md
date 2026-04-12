# Proof: Dijkstra Weakest Precondition (Edsger Dijkstra, 1975)

**Theory:** For any command C and postcondition Q, the *weakest precondition* wp(C, Q) is the most general condition on the initial state that guarantees Q holds after C executes. This gives a systematic way to derive correct programs from their specifications.  
**Claim:** Loom's `require:` encodes the weakest precondition. The Loom SMT bridge computes `wp(body, ensure:)` and verifies that `require: ⟹ wp(body, ensure:)`.  
**Turing Award:** Edsger Dijkstra, 1972.

## Computing weakest preconditions

| Program C | Postcondition Q | wp(C, Q) |
|---|---|---|
| `x := x + 1` | `x > 5` | `x > 4` |
| `if x≥0 then x else -x` | `result ≥ 0` | `TRUE` |
| `y := x+1; z := y*2` | `z > 10` | `x > 4` |

The `require:` in `increment_past_five` is exactly `x > 4` — not `x >= 4`, not `x > 3`. It is the **weakest** (most permissive) precondition that still guarantees the postcondition.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test wp_increment_boundary ... ok
test wp_absolute_value_no_precondition ... ok
test wp_sequential_composition ... ok
test wp_is_minimal_condition ... ok
```

## Layman explanation

You're planning a road trip. The postcondition is "arrive with at least half a tank." The weakest precondition is: "leave with at least X litres." Too little and you don't make it. More than X and you're safe — but X is the exact minimum. Dijkstra showed how to compute this X for any program from its desired outcome. Loom's `require:` is that X, and the SMT bridge verifies the compiler computed it correctly.
