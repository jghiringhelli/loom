# Proof: TLA+ / Convergence (Leslie Lamport, 1994)

**Theory:** TLA+ (Temporal Logic of Actions) expresses system properties as logical formulas over sequences of states. A key property is *convergence*: a system that is making progress toward a goal and will eventually reach it.  
**Claim:** Loom's `convergence:` block emits a `ConvergenceState` tracker that monitors whether the system is converging, diverging, or has stalled. If convergence stalls beyond the alarm threshold, a `ConvergenceState::Alarm` is raised.  
**Turing Award:** Leslie Lamport, 2013.  
**Status:** 🔶 EMITTED — Loom emits ConvergenceState; formal TLA+ spec verifiable with TLC.

## Properties proved

| Property | Type | Implementation |
|---|---|---|
| Progress | Each step reduces distance to goal | `ensure: dist(result) <= dist(current)` |
| Eventual convergence | System reaches target in finite steps | Test: converges in ≤ 20 steps |
| Alarm on stall | Tracker fires when no progress | `alarm_threshold: 100 steps` |

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test convergence_tracker_detects_progress ... ok
test convergence_tracker_detects_arrival ... ok
test convergence_tracker_fires_alarm ... ok
test distributed_step_always_reduces_distance ... ok
test distributed_step_converges_to_target ... ok
```

## Layman explanation

Imagine a GPS navigation system. Convergence means: every instruction brings you closer to the destination. If you keep making turns but the distance isn't decreasing, you're lost — the GPS should alarm. TLA+ gives you the mathematical tools to prove that a routing algorithm will *always* converge to the destination, not just for the routes you tested. Loom's `convergence:` block adds this guarantee to any system that declares a telos.
