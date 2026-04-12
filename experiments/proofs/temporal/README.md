# Proof: Temporal Logic (Amir Pnueli, 1977)

**Theory:** Properties of systems that evolve over time can be expressed as logical formulas over sequences of states: *always P*, *eventually Q*, *P before Q*.  
**Claim:** Loom's `temporal: before:/after:` blocks encode ordering constraints that the temporal checker verifies. Lifecycle state machines enforce that certain states are only reachable via valid transition sequences.  
**Turing Award:** Amir Pnueli, 1996.

## Properties proved

| Property | Type | Loom construct |
|---|---|---|
| Payment precedes shipment | Safety (bad thing never happens) | `temporal: before: validate_payment` |
| Auth precedes authorization | Safety | `temporal: before: authenticate` |
| Every paid order eventually deliverable | Liveness (good thing eventually happens) | Lifecycle reachability |
| Shipped only reachable from Paid | Invariant | `lifecycle: transitions:` |

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test temporal_payment_before_shipment ... ok
test temporal_cannot_ship_before_payment ... ok
test temporal_full_lifecycle_sequence ... ok
test temporal_liveness_eventually_delivered ... ok
test temporal_cannot_deliver_without_shipping ... ok
```

## Layman explanation

You can't graduate before you enroll. You can't get a refund before you've paid. Temporal logic formalizes these "before/after" rules for software. Pnueli showed that these properties can be automatically verified — you don't need to trace every possible execution by hand. Loom's `temporal:` blocks encode these rules so the compiler can verify them before the code ever runs.
