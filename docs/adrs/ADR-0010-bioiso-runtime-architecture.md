# ADR-0010: BIOISO Runtime Architecture — Three-Tier Synthesis

**Date**: 2026-04-12
**Status**: Accepted

## Context

Loom has a complete compile-time type system including `being:`, `telos:`, `signal`,
`epigenetic:`, `evolve:`, `crispr:`, `telomere:`, and safety annotations (`@mortal`,
`@corrigible`, `@sandboxed`). These constructs are fully parsed and checked at compile
time. However, they only describe *intent*. There is no runtime infrastructure that:

- Measures actual entity behavior against declared `telos:`
- Emits structured telemetry from running entities
- Proposes mutations when entities drift from their telos
- Type-checks proposed mutations before deploying them
- Executes canary deployments and rolls back on regression

The language grammar is ~80% of the vision. The runtime is entirely absent.

The target system is one where a running Loom program is a *living entity*:
it emits signals, its telos is continuously measured, and when drift is detected
the system proposes — and executes — type-safe mutations to restore alignment.
This is BIOISO: Biologically-Organized Intelligent Self-Optimizing systems.

## Decision

### 1. Add `compile_runtime()` to the compiler pipeline

A new emitter `compile_runtime()` generates a Rust runtime binary alongside the
existing emission targets. The runtime:
- Supervises entity lifecycle (birth, health, telomere exhaustion, death)
- Executes the signal/channel infrastructure declared in `signal` blocks
- Evaluates telos drift continuously against declared `telos:` bounds
- Drives the three-tier synthesis loop

### 2. Three-Tier Synthesis (biological model)

Mutation proposals escalate through three tiers, modeled on biological nervous system
organization:

```
Tier 1 — Polycephalum (Physarum polycephalum / slime mold)
  Deterministic Rust rule engine. No network. < 50ms.
  Local gradient-following. Each entity carries its own rules.
  Handles: ParameterAdjust, EntityRollback, small epigenetic triggers.
  Model: a slime mold navigating a maze — pure local chemical gradient following,
  no central coordination, emergent optimality.

Tier 2 — Ganglion (nerve cluster / enteric nervous system)
  Small local LLM via Ollama (Phi-3 mini, Gemma 2B, or similar).
  Runs on-device. Handles cluster-level synthesis.
  Handles: EntityClone, StructuralRewire, compound epigenetic adjustments.
  Triggered when: drift score > 0.7 OR Tier 1 fails to converge in N cycles.

Tier 3 — Mammal Brain (cortex / central nervous system)
  External LLM API (Claude / ANTHROPIC_API_KEY).
  Called only when Tier 1+2 cannot converge. High latency, has cost.
  Handles: novel entity proposals, telos revision, cross-system rewiring.
  Triggered when: drift persists > W minutes after Tier 2 attempts.
  Cost guard: never > N API calls/hour (configurable, default 10).
```

Escalation is automatic. Tier 1 always runs first. The system prefers local,
fast, deterministic computation and escalates only when necessary.

### 3. MutationProposal type system

All proposals regardless of tier are expressed as typed `MutationProposal` values:

```rust
pub enum MutationProposal {
    ParameterAdjust { entity_id: EntityId, param: String, delta: f64 },
    EntityClone     { source_id: EntityId, new_id: EntityId },
    EntityRollback  { entity_id: EntityId, checkpoint_id: CheckpointId },
    EntityPrune     { entity_id: EntityId, reason: String },
    StructuralRewire { from_id: EntityId, to_id: EntityId, signal_name: String },
}
```

Every proposal — regardless of which tier generated it — passes through the
**Type-Safe Mutation Gate** before execution.

### 4. Type-Safe Mutation Gate (mandatory)

No mutation is deployed without passing through `compile()`. The gate:
1. Applies the proposed diff to the `.loom` source
2. Runs all 11 semantic checkers
3. Enforces: autopoietic mutations require `@mortal @corrigible @sandboxed`
4. Writes acceptance/rejection + checker errors to the audit trail in the signal store
5. Rejected proposals are logged and escalated to the next tier

This means the AI (at any tier) cannot propose a mutation that violates type safety,
breaks a Hoare contract, violates session types, or removes safety annotations.
The language's 11 checkers are the immune system of the runtime.

### 5. Signal Store

An append-only SQLite database (one file per running system). Tables:
- `entities(id, name, telos_json, born_at, state)`
- `signals(id, entity_id, metric, value, ts)`
- `telos_bounds(entity_id, metric, min, max, target)`
- `drift_events(id, entity_id, score, ts, triggering_signal)`
- `mutation_proposals(id, entity_id, tier, proposal_json, verdict, checker_errors, ts)`
- `checkpoints(id, entity_id, state_json, ts)`

The signal store is the episodic memory of the running system. It is the input
to all three synthesis tiers and the audit trail for all mutations.

### 6. Canary Deployment + Auto-Rollback

Mutations are deployed to a configurable fraction of entity instances first.
A monitoring window measures telos score improvement. If the score worsens,
the deployment is automatically reverted to the last checkpoint.

```
propose → gate → checkpoint → canary deploy (N%) → monitor (W sec)
  → if telos improved: promote to 100%
  -> if telos unchanged or worsened: rollback to checkpoint
```

### 7. CLI surface

New subcommand `loom runtime`:
- `loom runtime start <file.loom>` — compile and start the runtime loop
- `loom runtime status` — show entity health, drift scores, tier activity
- `loom runtime log` — tail the signal store (signals + drift events + mutations)
- `loom runtime rollback <entity_id> <checkpoint_id>` — manual rollback

## Alternatives Considered

**Alternative A: Pure AI loop (no deterministic tier)**
Rejected. An LLM-only mutation loop has no latency guarantees, has cost per
mutation, cannot run offline, and has no structural upper bound on what it proposes.
The polycephalum tier provides the offline, fast, deterministic baseline.

**Alternative B: Rule engine only (no LLM)**
Rejected. A deterministic rule engine cannot discover novel mutations — it can only
adjust parameters within known rules. Complex telos failures require synthesis that
exceeds what rules can express. The three-tier escalation handles both cases.

**Alternative C: External orchestration (Kubernetes + sidecars)**
Rejected for initial implementation. The goal is a self-contained `loom runtime`
command that works without infrastructure. Container orchestration can be layered on
top later once the core loop is proven.

**Alternative D: Separate runtime binary**
Rejected. The runtime is compiled FROM the `.loom` source by `compile_runtime()`.
This keeps the source as the single source of truth. The runtime is a derivation,
not a separate product.

## Consequences

**Makes easier:**
- Loom programs that actually evolve toward their declared telos at runtime
- Formal audit trail of every mutation with type-checker evidence
- Self-healing systems where the language's safety properties prevent dangerous mutations
- Offline operation via Tier 1 + Tier 2 (no external network required)

**Makes harder:**
- The compiler now has two output modes: static (current) and runtime (new)
- Testing the runtime loop requires a running SQLite instance
- The ganglion tier requires Ollama to be installed locally
- The end-to-end loop is complex to test deterministically

**What the AI must know:**
- `compile_runtime()` is distinct from `compile()` — different output, same input
- The mutation gate is non-negotiable — no tier can bypass it
- `@mortal @corrigible @sandboxed` are required for autopoietic mutations at runtime
  (currently enforced at compile time by SafetyChecker; runtime enforces again at mutation gate)
- The signal store is append-only — no updates, only inserts + new checkpoints
- Tier escalation is automatic; tiers do not need to know about each other

## Module Location

```
src/
  runtime/
    mod.rs          — public API: start(), status(), rollback()
    signal.rs       — Signal type, emitter, channel infrastructure
    store.rs        — SQLite signal store (rusqlite)
    supervisor.rs   — Entity supervisor, lifecycle state machine
    drift.rs        — Telos evaluator, drift scorer, DriftEvent
    mutation.rs     — MutationProposal type, serializer
    gate.rs         — Type-safe mutation gate (calls compile())
    polycephalum.rs — Tier 1: deterministic rule engine
    ganglion.rs     — Tier 2: Ollama HTTP client + engine
    brain.rs        — Tier 3: Claude API client + engine
    deploy.rs       — Canary deployment + checkpoint + rollback
    orchestrator.rs — Main evolution daemon loop
```
