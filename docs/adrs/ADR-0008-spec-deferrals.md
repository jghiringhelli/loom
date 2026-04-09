# ADR-0008: Spec Session Deferrals — loom-spec-session-completo.md

**Date**: 2026-04-14  
**Status**: Accepted

## Context

A prior Claude AI session produced a 1566-line spec document (`docs/loom-spec-session-completo.md`)
exploring Loom's semantic model across seven thematic areas:

- Part I: Live Intent Taxonomy (3 modes, 5 signal sources, IntentRefiner, governance classes)
- Part II: BIOISO vs DES, ExperimentGenerator, signal_taxonomy
- Part III: Telos as typed function (`compute :: BeingState -> SignalSet -> Float`)
- Part IV: Distributed nervous system, quorum, collective_telos_metric
- Part V: Formal system properties
- Part VI: Senses/Effectors, messaging primitives (7 patterns)
- Part VII: Orthogonal structural/formal checker dimensions

The session was exploratory — not all ideas are appropriate language features.
About 60% are sound; 40% reflect AI enthusiasm without grounding in what a
compiler can actually enforce.

## Decision

### Implemented (M112–M116)

These were accepted because they are compiler-enforceable and directly strengthen
the telos/homeostasis semantic model:

| Milestone | Feature | Where |
|---|---|---|
| M112 | `TelosDef` upgrade: `measured_by`, `thresholds` (convergence/warning/divergence/propagation), `guides` | `ast/being.rs`, `checker/teleos.rs` |
| M113 | TelosImmutability: `modifiable_by` without `@corrigible` → error | `checker/safety.rs` |
| M114 | `telos_contribution: f64` on regulate blocks (validates [0.0, 1.0]) | `ast/being.rs`, `checker/teleos.rs` |
| M115 | `signal_attention` block: `prioritize_above`/`attenuate_below` thresholds | `ast/being.rs`, `checker/signal_attention.rs` |
| M116 | `messaging_primitive` module-level construct | `ast/items.rs`, `checker/messaging.rs` |

### Deferred

#### Parts I–II: Live Intent Governance / ExperimentGenerator

**What was proposed**: A `live_intent:` runtime mode system with `IntentRefiner`,
`signal_taxonomy`, `ExperimentGenerator`, governance classifications, and a
`collective_telos_metric` aggregating multiple beings' telos evaluations.

**Why deferred**: These are runtime orchestration systems — applications to build
*with* Loom, not features of the Loom language itself. A Loom module can express
intent governance as a `being` with `regulate:` + `evolve:` + `telos:` today.
The `ExperimentGenerator` is a higher-order program that generates `.loom` programs.
Both belong in `stdlib/intent-live/` or `examples/` once the language is stable.

**Condition for reconsideration**: Stable Loom v0.1.0 release; identify use-case
that requires language-level support vs library-level.

---

#### `entity<N,E,Annotations>` Generics

**What was proposed**: Generic entity types parameterized by node type, edge type,
and annotation set — a form of higher-kinded types for structural pattern abstraction.

**Why deferred**: Requires full parametric polymorphism in the type system. The
current type system supports named types, function types, and refinement types
but not type-level parameters. This is a substantial type system extension (likely
M150+ in the roadmap scope).

**Condition for reconsideration**: After implementing type inference for polymorphic
functions; after Rust emitter supports generic code generation.

---

#### `interface_layer Senses { ... }` / `interface_layer Effectors { ... }`

**What was proposed**: New top-level construct `interface_layer` for declaring
perceptual and actuation surfaces, separate from `being`.

**Why deferred**: Expressible today with existing constructs:
```
being SensorLayer
  telos: "detect environmental changes"
  end
  signal:
    from: Environment
    channels: [temperature, pressure]
  end
end
```
The `umwelt:` block (M80) already declares a being's perceptual world.
A new keyword adds complexity without semantic gain.

**Condition for reconsideration**: If there's a need for static interface composition
semantics (multiple beings sharing the same sensor surface) that cannot be expressed
via the type system.

---

#### Part IV: Distributed Nervous System / Quorum / collective_telos_metric

**What was proposed**: Cross-being synchronization primitives (quorum voting,
distributed consensus on telos), `collective_telos_metric` as a module-level aggregate.

**Why deferred**: Requires a runtime with distributed coordination semantics.
The compiler can check *type safety* of distributed protocols (already done via
`session_type` and M116 `messaging_primitive`) but cannot verify distributed
consensus properties without a formal model of the execution environment.
The `collective_telos_metric` is a runtime aggregate, not a compile-time property.

**Condition for reconsideration**: After defining a formal execution model for
multi-being ecosystems; after the session type checker is extended to handle
multi-party protocols.

---

#### Part VII: OrthogonalityChecker

**What was proposed**: A dedicated checker proving that `StructuralTaxonomy`
and `FormalConstraints` are fully independent dimensions — each annotation can
appear on any structural element without semantic conflict.

**Why deferred**: The claim of full orthogonality is an idealization. The spec
itself acknowledges that annotations interact through shared context (e.g.,
`@corrigible` affects telos semantics, which affects structural validity). A
"proof of orthogonality" checker would produce false assurances. The correct
approach is thorough integration tests (ALX-2) proving that valid combinations
compile, not a blanket orthogonality claim.

**Condition for reconsideration**: If annotation interaction bugs accumulate that
suggest a formal conflict-detection mechanism is warranted.

## Consequences

### What becomes easier
- M112–M116 are now in the compiler. Loom programs can express telos convergence
  quantitatively, validate signal attention policies, and name messaging patterns.
- The safety invariant (TelosImmutability, M113) closes a gap: `@corrigible` was
  only checked forward (needs `modifiable_by`); now the reverse is also checked
  (has `modifiable_by` without `@corrigible` → error).
- Future sessions start from this ADR rather than re-analyzing the full spec.

### What is harder
- The deferred constructs cannot be encoded directly in `.loom` source without
  workarounds until they are implemented.
- AI sessions must read this ADR before suggesting re-implementation of the
  deferred ideas to avoid duplicating this analysis.

### What the AI needs to know
- Parts I–II constructs (live intent, ExperimentGenerator) belong in `stdlib/` or
  `examples/`, not the compiler.
- `entity<N,E,Annotations>` requires type system work before it can be implemented.
- `interface_layer` as a keyword is DEFERRED — use `being` + `signal:` instead.
- `collective_telos_metric` is a RUNTIME construct, not a compile-time checker rule.
- The m112_test.rs file covers M111 (CognitiveMemory); M112–M116 tests are in
  `tests/m113_m116_test.rs`.
