# Proof: Autopoiesis (Maturana & Varela, 1972)

**Theory:** A living system is defined by its capacity to continuously produce and maintain its own components — self-bounded, self-produced, self-regulated.  
**Claim:** Loom's `autopoietic: true` annotation requires structural completeness: a boundary (`matter:`), at least one homeostatic regulation loop (`regulate:`), and a telos. Missing any component → `LoomError::AutopoiesisContractViolation`.

## The three Maturana-Varela conditions (all encoded in Loom)

| Condition | Biological meaning | Loom construct |
|---|---|---|
| 1. Self-boundary | Membrane distinguishes inside from outside | `matter:` fields including a boundary variable |
| 2. Self-production | System produces its own structural components | `autopoietic: true` + `propagate:` or repair |
| 3. Self-regulation | Homeostatic loops maintain viability | `regulate: trigger:... action:...` |

**Finite lifespan** (Hayflick complement): `telomere: max_generations:` adds a fourth constraint — all living things have finite lifespan.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test autopoiesis_condition_1_boundary_exists ... ok
test autopoiesis_condition_2_self_production ... ok
test autopoiesis_condition_3_self_regulation ... ok
test telomere_limits_lifespan ... ok
test survival_score_bounded_0_to_1 ... ok
test cell_maintains_viability_through_regulation ... ok
```

## Layman explanation

A candle is not alive — it consumes itself and goes out. A cell is alive because it continuously rebuilds itself: it repairs its membrane, replenishes its energy, and replicates. Loom's BIOISO framework encodes these three properties as compiler-checked contracts. A "being" that claims to be autopoietic but has no self-regulation loops is rejected at compile time — the same way a cell without a membrane isn't a cell.
