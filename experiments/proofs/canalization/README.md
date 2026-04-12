# Proof: Waddington Canalization (C.H. Waddington, 1942)

**Theory:** Developmental pathways are "canalized" — they tend toward stable trajectories and resist perturbations, like a ball rolling in a valley. Strong perturbations are absorbed; weak ones are buffered. The system always returns to its developmental channel.  
**Claim:** Loom's `canalize: channel:` block defines the acceptable state range. The emitted `regulate:` loop enforces convergence. The epigenetic response increases resilience under persistent stress.

## What is being proved

**The canalization property:**
1. Perturb the system → it leaves the channel
2. Allow regulation → it returns to the channel  
3. Under persistent perturbation → epigenetic resilience increases (adaptation)

This is directly analogous to Waddington's epigenetic landscape: the developmental trajectory is a valley, perturbations push the ball up the slope, but gravity (the regulate: loop) pulls it back.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test system_starts_in_channel ... ok
test perturbation_knocks_out_of_channel ... ok
test canalization_restores_channel_after_perturbation ... ok
test fitness_higher_in_channel_than_out ... ok
test resilience_increases_under_persistent_perturbation ... ok
```

## Application to BIOISO

Every BIOISO program that manages a controlled system (climate regulation, antibiotic dosing, energy grid balancing) needs canalization — the ability to absorb disturbances without diverging. The `canalize:` block encodes this formally, and the compiler verifies that every canalizing being has the required regulation loops.
