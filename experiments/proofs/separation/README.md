# Proof: Separation Logic (John Reynolds, 2002)

**Theory:** Separation logic extends Hoare logic with a *separating conjunction* P * Q meaning "P holds for this part of the heap AND Q holds for a disjoint part." This allows modular reasoning about heap-allocated resources.  
**Claim:** Loom's `separation: owns:/disjoint:` blocks emit Rust code where disjointness is guaranteed by the ownership system. Two owned resources cannot alias — the borrow checker is a separation logic verifier.  
**Status:** 🔶 EMITTED — Rust's type system proves disjointness; Prusti provides the formal frame rule proof.

## The frame rule

The key theorem of separation logic:
```
{P} C {Q}
─────────────────────────────
{P * R} C {Q * R}
```
If C transforms P to Q, and R describes a disjoint part of the heap, then C leaves R unchanged. In Loom/Rust: `transfer(from, to, amount)` cannot affect `unrelated` because it doesn't have ownership of it.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test

# Formal verification with Prusti (Linux):
# cargo prusti
```

Expected:
```
test transfer_moves_funds_correctly ... ok
test separation_disjointness_enforced_by_move_semantics ... ok
test frame_rule_unrelated_account_unchanged ... ok
```

## Layman explanation

Like safety deposit boxes at a bank: each box has one key, one owner. The bank can move money between two boxes (separation conjunction: two disjoint resources) without touching anyone else's box (frame rule). If someone tried to use the same box as both "from" and "to" in a transfer, the key system physically prevents it — you can't hand your key to two people at once. Rust's move semantics are the key system.
