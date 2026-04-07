# What Is Loom?

Loom is a programming language where **the primary executor is an AI agent, not a human**.

It compiles to Rust, TypeScript, and WebAssembly. A human can read it. But it was
designed for an AI to write, verify, and compose — token-efficiently, with correctness
properties embedded in the syntax rather than described in comments or inferred from
documentation.

---

## The premise

Human programming languages were designed so humans can understand them. The cost is
verbosity: you need to read an entire function to understand what it does and what
guarantees it makes. Comments and documentation exist to close this gap, but they
drift from the code and cannot be verified.

When an AI agent generates or reasons about code, the bottleneck is different:
- **Token efficiency**: every token consumed is inference cost
- **Semantic density**: the type signature + annotations should carry the full contract
- **Machine-checkable correctness**: properties that need verification should be
  verifiable without executing the code

Loom is designed around these constraints. The annotation `@conserved(Mass) @requires_auth @pii`
in a function signature tells an AI agent — without reading the body — that this
function preserves a physical invariant, requires an authenticated context, and handles
personal data. The compiler verifies all three. The AI can trust the contract and
compose the function correctly.

---

## What the compiler checks

Most languages check that your types line up. Loom also checks *correctness properties*:
physical laws, security contracts, communication protocols, information flow, temporal
ordering. If your code violates any of these, it doesn't compile.

---

## What "correctness properties" means in practice

Here are concrete examples of things Loom rejects at compile time that other compilers accept without comment:

**Physical laws**
```loom
fn process_reaction @conserved(Mass)
    :: Reactant -> Product
  -- If this function can lose mass, it's a compile error.
  -- Lavoisier (1789): mass is conserved. The compiler enforces it.
end
```

**Security context**
```loom
fn generate_session_token @pseudo_random @requires_auth
    :: User -> Token
  -- @pseudo_random (Mersenne Twister, LCG) in an auth context is a compile error.
  -- NIST SP 800-90A: deterministic PRNGs are insufficient for key material.
end
```

**Information flow**
```loom
fn log_request @pii
    :: Request -> LogEntry
  -- @pii-annotated data cannot flow to log output without explicit declassification.
  -- Compile error if it does.
end
```

**Communication protocols**
```loom
session AuthProtocol
  client:
    send: Credentials
    recv: Result<Token, AuthError>
  end
  server:
    recv: Credentials
    send: Result<Token, AuthError>
  end
  duality: client <-> server
  -- The compiler verifies that what the client sends, the server receives.
  -- Protocol mismatch is a compile error, not a runtime deadlock.
end
```

**Temporal ordering**
```loom
fn handle_payment @requires_auth
    :: PaymentDetails -> Receipt
  temporal:
    precedes: verify_token before charge_card
    -- If charge_card can run before verify_token, it's a compile error.
  end
end
```

---

## What it doesn't claim

Loom is not a theorem prover. It does not produce formal mathematical proofs of
correctness in the Coq or Lean sense.

What it does: if a program compiles with Loom, the specific properties the compiler
checks are held by that program. No more. The properties are documented — you can see
exactly what each annotation and checker verifies.

We do not say "provably correct." We say "if it compiles, these specific things are
true, and here is the academic grounding for each one."

---

## The organizational layer

Loom has constructs for expressing the *structure* of long-lived, adaptive systems:
lifecycle management, error correction, adaptive behavior, behavioral modulation.

These are metaphors from biology — the organizational principles that make biological
systems robust — translated into executable constructs. They are not simulation. They
do not model cells.

The metaphor is useful because biological systems have solved the problem of robustness
under uncertain conditions better than most software. If your service needs to degrade
gracefully under load, expire after a lifespan, repair itself from violations, and
coordinate with other services — these constructs give you a vocabulary for that.

You do not need to use them. The core language works without them.

---

## The data persistence layer

Loom treats persistent storage as a first-class type-level concern. Eleven storage
kinds are built into the type system: relational, key-value, document, graph,
columnar, time-series, vector, snowflake, hypercube, in-memory, and flat file.

The query pattern that suits each storage kind is known and academically grounded.
A graph store using relational join syntax is a type error. A columnar store without
a partition key is a warning. The compiler knows the right tool for each job.

---

## The scientific measurement layer

Loom's type system includes SI units and their combinations. A function that adds
meters to seconds is a type error. A function that claims to return a probability
outside [0, 1] is a type error. Tensors carry rank, shape, and unit — mismatched
matrix multiplication is a compile error.

The idea is that the same language that writes application logic can also express
the physics, chemistry, or finance that the application models — without importing
a library or adding an annotation layer on top.

---

## Why AI agents benefit from this

When an AI agent writes a Loom function, the contract is in the signature:

```loom
fn charge_card @requires_auth @conserved(Value) @idempotent
    :: PaymentDetails -> Effect<[DB<Relational>, Network]> -> Receipt
```

From this signature alone — without reading the body — an AI agent knows:
- Authentication is required before this function can be called
- The total value in the system is preserved
- The operation is safe to retry
- It touches a relational database and a network
- It produces a receipt

This is more semantic information per token than documentation. The compiler has
already verified all of it. An AI composing a payment workflow does not need to
read the implementation — the signature is the proof.

This is what Loom was designed for: **semantic density sufficient for AI-level
reasoning about correctness, without human-level verbosity**.

---

## Who it's for

**AI coding agents** that generate or reason about code and need machine-verifiable
contracts in the syntax, not in comments that might be wrong.

**Researchers** who want a language that expresses scientific correctness in the type
system rather than in external proofs.

**Systems builders** who want the Rust backend with correctness guarantees that live
at the source level — so both human reviewers and AI agents can verify them.

---

## Current state

Loom is a research compiler. It is not production-ready. The language is being
developed in the open; the compiler compiles to Rust, TypeScript, and WASM.
634 tests pass. The core language is stable; the standard libraries are in progress.

The specification and academic lineage for every language construct are documented
in `docs/language-spec.md` and `docs/lineage.md`. Every claim in the compiler is
traceable to a published source.

---

## How to try it

```bash
cargo install loom-lang
loom compile hello.loom --target rust
```

See `docs/language-spec.md` for the full language reference.
See `README.md` for installation and quick start.
