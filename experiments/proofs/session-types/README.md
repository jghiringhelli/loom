# Proof: Session Types (Kohei Honda, 1993)

**Theory:** Communication protocols can be encoded in types so that wrong-order message sends are compile-time errors.  
**Claim:** A Loom `session:` declaration emits a Rust typestate machine where calling steps out of order is a type error — caught by `rustc`, not at runtime.  
**Key researchers:** Kohei Honda (1993), Gay & Hole (2005).

## What is being proved

**Honda's guarantee:** If every participant follows the typed protocol, no communication errors occur. In Loom, the type system *structurally prevents* wrong-order calls — there is no method to call.

**Correct usage (compiles):**
```rust
let ch = AuthProtocolClientChannel::new();  // State: Step0
let ch = ch.send("credentials".to_string()); // State: Step1
let (_, token) = ch.recv();                 // State: Done
```

**Wrong order (type error — does not compile):**
```rust
let ch = AuthProtocolClientChannel::new();  // State: Step0
let (_, token) = ch.recv();                // ERROR: no method `recv` on Step0
```

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo build   # must succeed

# To prove the violation: uncomment the wrong-order block in proof.rs
# cargo build  # must FAIL with type error
```

## Layman explanation

Like a vending machine that physically won't let you take a drink before inserting money — not because it checks, but because the slot for money comes before the slot for drinks. The machine's *structure* prevents the wrong order. Session types do the same for software protocols.
