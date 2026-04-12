# Proof: Hayflick Limit (Leonard Hayflick & Paul Moorhead, 1961)

**Theory:** Normal human somatic cells have a finite replication capacity (~50 divisions). Telomeres shorten with each division; when exhausted, the cell enters senescence.  
**Claim:** Loom's `@mortal` + `telomere: max_generations:` encodes this as a compiler-verified constraint. A `@mortal` being without a `telomere:` block → `LoomError::MissingTelomere`. A being that exceeds `max_generations` triggers senescence — division returns `None`.

## What is being proved

**The Hayflick guarantee:** No BIOISO being can claim to be mortal without declaring its finite lifespan. The compiler verifies structural completeness, and the runtime enforces the limit.

**This matters for BIOISO apps** because it prevents runaway processes — every being that is born must eventually terminate, and the termination condition is declared at design time, not discovered at runtime.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test cell_divides_while_viable ... ok
test telomere_shortens_each_division ... ok
test hayflick_limit_stops_division ... ok
test cell_reaches_senescence_through_division ... ok
test fitness_declines_with_age ... ok
```

## Layman explanation

Every normal human cell has a counter — it can divide about 50 times and then it stops. This isn't a bug; it's a protection against cancer. Loom's BIOISO framework encodes the same principle: every program-organism declares its maximum lifespan. A being that claims to be mortal but provides no death condition is rejected at compile time. Software processes that can run forever without a declared termination condition are the cancer of distributed systems.
