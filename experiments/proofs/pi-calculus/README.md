# Proof: π-Calculus / Mobile Communication (Robin Milner, 1992)

**Theory:** The π-calculus extends process calculi with *mobile channels* — channels can be passed as values in messages. This allows dynamic reconfiguration of communication topology, which is necessary to model real distributed systems.  
**Claim:** Loom's `ecosystem:` + `signal:` blocks encode inter-being communication. The `SignalChannel<T>` type makes channels first-class values that can be routed at runtime. Signal types are verified at compile time.  
**Turing Award:** Robin Milner, 1991.

## What is being proved

**The mobility property:** A `ForecastModel` can send a new `SignalChannel<Float>` to a `CoolingActuator`, changing the communication topology at runtime. The types remain safe throughout — the actuator knows exactly what type of values it will receive on the new channel.

This maps directly to BIOISO ecosystems: beings can dynamically reconfigure their signal routing (analogous to neural rewiring), but the type system prevents misrouted signals.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test temperature_signal_routing ... ok
test actuator_responds_to_temperature_signal ... ok
test mobile_channel_pi_calculus_pattern ... ok
```

## Layman explanation

The π-calculus is about how processes can hand each other phone numbers (channels) mid-conversation. You call Alice; Alice says "actually, call Bob directly at this number"; now you're talking to Bob through a channel you didn't know about when you started. The π-calculus proves this can be done safely — you always know what language you'll be speaking on any channel you receive. Loom's `signal:` types are the phone book entries that the compiler verifies before the calls are made.
