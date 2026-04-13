# ADR-0011: CEKS Runtime Architecture

**Date**: 2026-04-13
**Status**: Accepted
**Supersedes**: ADR-0010 (BIOISO Runtime — Three-Tier Synthesis)

## Context

ADR-0010 established the three-tier synthesis pipeline (Polycephalum → Ganglion →
Mammal Brain) and the full R1–R7 implementation. That system is complete and working.

During architectural review, three gaps were identified:

1. **No integrity layer.** Mutations arriving from outside the gate (repo push, operator
   patch, accidental overwrite) are not verified against the entity's registered genome
   hash. A tampered compiled artifact can enter the pipeline without detection.

2. **No memory across ticks.** Each orchestration tick starts cold. A proposal that was
   promoted three weeks ago for this drift pattern is invisible to the current tick. The
   system re-discovers the same solutions repeatedly without accumulating institutional
   knowledge.

3. **No temporal intelligence.** The pipeline fires on every drift event regardless of
   signal quality or known temporal patterns (market close, seasonal forcing, startup
   transients). This wastes Tier 2 and Tier 3 calls on noise.

4. **No cross-instance sharing.** Cloned entities (colony members) each run their own
   independent synthesis loop. A mutation that worked on ForestModel-2 on the GPU laptop
   is invisible to ForestModel-1 on the dev machine.

A deeper architectural observation: the pipeline stages (0–5) and the cross-cutting
concerns (memory, timing, coordination) are fundamentally different in kind. Numbering
them all together obscures both. They need separate taxonomy axes.

Additionally: the acronym **CEKS** — formed from the four cross-cutting concerns
(Circadian, Epigenetic, Kin/Colony, Stages 0–5) — maps precisely onto the **CEKS
abstract machine** from programming language theory (Control · Environment · Kontinuation
· Store), and mirrors what sexual reproduction does for biological organisms:
distributes adaptation laterally and across generations faster than any individual can
adapt alone.

## Decision

### 1. Rename the runtime model: BIOISO → CEKS

**BIOISO** remains the name for the Loom *language philosophy* — beings with telos,
autopoietic mutation, biological paradigm mapping. It is the "what."

**CEKS runtime** is the execution engine that implements BIOISO. It is the "how."

The module remains `src/runtime/`. The concept is now called the CEKS runtime in all
documentation, ADRs, and CLI help text.

### 2. Two-axis architecture

The CEKS runtime has two distinct axes that must not be conflated:

**Axis A — The Linear Pipeline (Stages 0–5)**

Sequential. Each stage has one job. Escalation goes upward only on failure.
No stage knows about cross-cutting concerns directly — those are injected.

**Axis B — Cross-Cutting Concerns (C, E, K)**

Orthogonal to the pipeline. Each modulates, remembers, or coordinates.
They have no sequence relative to each other. They are always active.

### 3. The Linear Pipeline — Stages 0–5

```
ENTRY POINTS
────────────
[Signal Emission]     entity emits a metric value during normal operation
[External Mutation]   repo push / operator patch / clone spawn / K broadcast

       │
       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 0: Membrane / Immune                         │
│                                                     │
│  Is this signal or mutation self or foreign?        │
│  Verifies SHA-256 hash of compiled artifact against │
│  registered genome lineage. Rejects non-self.       │
│  Also rejects signals from unregistered entities.   │
│  Never blocked by C (Circadian). Always runs.       │
└──────────────────────┬──────────────────────────────┘
                       │ ADMIT
                       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 1: Reflex Arc  (Polycephalum)                │
│                                                     │
│  Is there a known rule for this drift pattern?      │
│  Deterministic Rust rule engine. < 50ms.            │
│  No memory, no network.                             │
│  Reads: E (Core + Working tiers)                    │
└──────────────────────┬──────────────────────────────┘
                       │ MISS → escalate
                       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 2: Peripheral Nervous  (Ganglion / Ollama)   │
│                                                     │
│  Can local pattern matching synthesise a fix?       │
│  Small LLM on GPU laptop (Phi-3, Qwen2.5, etc.)    │
│  Reads: E (Core + Working tiers) as prompt prefix   │
└──────────────────────┬──────────────────────────────┘
                       │ MISS → escalate
                       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 3: Central Nervous  (Mammal Brain / Claude)  │
│                                                     │
│  Full genome synthesis. Global view.                │
│  External API, cost-guarded (default 10 calls/hr).  │
│  Reads: E (Core tier only — condensed)              │
└──────────────────────┬──────────────────────────────┘
                       │ PROPOSAL (any stage)
                       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 4: Type-Safe Gate  (loom::compile())         │
│                                                     │
│  Is this a valid Loom mutation?                     │
│  Runs all 11 semantic checkers.                     │
│  Enforces @mortal @corrigible @sandboxed.           │
│  Never blocked by C (Circadian). Always runs.       │
│  Writes verdict + errors to signal store.           │
└──────────────────────┬──────────────────────────────┘
                       │ ACCEPTED
                       ▼
┌─────────────────────────────────────────────────────┐
│  Stage 5: Canary Deploy                             │
│                                                     │
│  Apply to subset. Monitor. Promote or rollback.     │
│  Always runs — never blocked by C.                  │
└──────────────────────┬──────────────────────────────┘
                       │
          ┌────────────┴────────────┐
          ▼                         ▼
   [PROMOTED]                [ROLLED BACK / REJECTED]
   → write to E              → write to E (negative)
   → broadcast to K          → write to E (compiler violation)
```

### 4. Cross-Cutting Concern C — Circadian

**Biological analog:** Hypothalamic circadian clock — governs *when* systems activate.

**Responsibility:** Gate Stages 1, 2, and 3 on time and signal quality.
Never blocks Stage 0 (Immune), Stage 4 (Gate), or Stage 5 (Deploy).

**Two independent gate types:**

**a) Cron gates** — time-based suppression or amplification rules:

```rust
pub struct CronGate {
    /// Standard cron expression (5-field: min hour dom month dow).
    pub expression: String,
    /// Which entity types this gate applies to. "*" = all.
    pub entity_pattern: String,
    /// Suppress proposal generation, amplify urgency, or require a minimum tier.
    pub action: CronAction,
    /// Human-readable reason logged in the audit trail.
    pub reason: String,
}

pub enum CronAction {
    /// Block Stages 1–3 from firing during this window.
    Suppress,
    /// Lower the drift threshold required to trigger proposals.
    Amplify { factor: f64 },
    /// Require at least this tier to handle proposals in this window.
    RequireMinTier(u8),
}
```

**Supported temporal scopes** (via standard cron expression fields):
- Minute-level: `*/5 * * * *` — every 5 minutes
- Hour-level: `0 16 * * MON-FRI` — weekdays at 16:00
- Day-of-week: `0 0 * * MON` — every Monday midnight
- Day-of-month: `0 0 1 * *` — first of each month
- Month/Season: `0 0 * 12,1,2 *` — winter months

**b) Signal SNR gate** — data quality suppression (always active, no cron):

```rust
pub struct SnrGate {
    /// Ratio of recent variance to baseline variance below which we suppress.
    pub min_snr: f64,
    /// Number of recent signals to measure against baseline window.
    pub recent_window: usize,
    pub baseline_window: usize,
}
```

If `recent_variance / baseline_variance < min_snr` → suppress Stages 1–3 this tick.
Prevents mutations triggered by sensor noise, startup transients, or data gaps.

### 5. Cross-Cutting Concern E — Epigenetic Memory

**Biological analog:** DNA methylation — same genome, different expression based on
accumulated environmental history. The genome doesn't change; what is expressed does.

**Responsibility:** Shared memory bus. Written by K (Colony) and Stage 5 (Canary
outcomes). Read by Stages 1, 2, and 3 as enrichment context.

**Three-tier promotion model** (from Chronicle architecture):

```
Buffer  ← raw outcome records, written after every Stage 5 result
Working ← Ollama distills Buffer weekly into pattern insight strings
Core    ← Claude distills Working into procedural laws when pattern repeats ≥5×
```

**Four memory types** (decay profiles from Chronicle):

| Type | BIOISO meaning | Decay |
|---|---|---|
| Episodic | What happened in this specific drift event | Medium (days) |
| Semantic | Facts about how this entity class behaves | Slow (weeks) |
| Procedural | What reliably works for this entity type | **Never decays** |
| Preference | Operator-configured mutation biases | Slow (months) |

**Weight system** (Chronicle model):

```
weight_new = weight_old + reinforcement_boost × (1 - weight_old)
```

Each access (injection into prompt, citation in rule) reinforces weight.
Nightly decay pass halves weights below promotion threshold.
Procedural memories skip decay entirely — too expensive to relearn.

**What each stage reads:**

| Stage | E tiers read | Injection point |
|---|---|---|
| Stage 1 (Reflex) | Core + Working | Injected as rule priority modifiers |
| Stage 2 (Peripheral) | Core + Working | Prepended to Ollama system prompt |
| Stage 3 (Central) | Core only | Condensed genome context prefix |

**Distillation cycle** (background, not on critical path):

```
Weekly:  Ollama reads Buffer entries → writes Working insights
Monthly: Claude reads Working entries → writes Core laws (if pattern count ≥ 5)
Nightly: Decay pass + tier promotion/demotion evaluation
```

### 6. Cross-Cutting Concern K — Colony (Kin)

**Biological analog:** Eusocial organisms + horizontal gene transfer — adaptations
discovered by one member propagate to all related members immediately, bypassing
generational lag. This is what sex does for populations: distributes successful
adaptations laterally faster than individual evolution can.

**Responsibility:** Connect cloned entity instances (from the same parent) across nodes.
When any colony member has a mutation promoted at Stage 5, broadcast to all siblings.

**Colony formation:** When `EntityClone` is promoted and deployed, the new instance is
registered in a colony record: `(parent_id, clone_id, node_address)`.

**Broadcast event on promotion:**

```rust
pub struct ColonyBroadcast {
    /// The entity that had the promoted mutation.
    pub source_id: EntityId,
    /// The colony (all entities sharing the same parent lineage).
    pub colony_id: String,
    /// The mutation that was promoted.
    pub proposal: MutationProposal,
    /// The E memory entry to replicate to all colony members.
    pub epigenetic_entry: EpiEntry,
    pub ts: Timestamp,
}
```

**Transport:** HTTP gossip between nodes. Each node runs a lightweight HTTP server
on a configurable port. Colony members register their node address at spawn time.

**K writes into E:** Every broadcast writes an Episodic memory entry into the
epigenetic store of every receiving colony member. This means a solution discovered
on the GPU laptop is immediately available to the rule engine (Stage 1) on the dev
machine at the next tick — without any human intervention.

**K activation:** K is not on the critical path. It activates after Stage 5 returns
a PROMOTED verdict. It does not block the pipeline.

### 7. Architecture summary diagram

```
                    ┌─────────────────────────┐
                    │  C — Circadian          │
                    │  cron gates + SNR gate  │
                    │  gates Stages 1, 2, 3   │
                    │  never gates 0, 4, 5    │
                    └──────────┬──────────────┘
                               │ permits / suppresses
                               ▼
[Signal] ──► [0:Immune] ──► [1:Reflex] ──► [2:Peripheral] ──► [3:Central] ──► [4:Gate] ──► [5:Canary]
                                ▲                ▲                  ▲
                                └────────────────┴──────────────────┘
                                         reads E (Memory Bus)
                                                 ▲
                                                 │ writes (outcomes + K broadcasts)
                                    ┌────────────┴──────────────┐
                                    │  E — Epigenetic Memory    │
                                    │  Buffer / Working / Core  │
                                    │  4 types, weight+decay    │
                                    └────────────▲──────────────┘
                                                 │ writes on promotion
                                    ┌────────────┴──────────────┐
                                    │  K — Colony (Kin)         │
                                    │  HTTP gossip, sibling E   │
                                    │  sync across nodes        │
                                    └───────────────────────────┘
```

### 8. Build roadmap

| Phase | Component | Depends on |
|---|---|---|
| R8 | Stage 0: Membrane / Immune (hash lineage) | Store (done) |
| R9 | E: Epigenetic Buffer tier (raw outcome logging) | Store (done), Stage 5 (done) |
| R10 | C: Circadian (cron gates + SNR gate) | Store, Orchestrator (done) |
| R11 | E: Working + Core tiers (Ollama/Claude distillation) | R9 (Buffer), GPU laptop |
| R12 | K: Colony gossip (HTTP, cross-node E sync) | R9 (E), multi-node setup |

R8 and R9 can be built without new infrastructure.
R10 requires the `cron` crate (pure Rust, no OS daemon).
R11 requires Ollama running on the GPU laptop.
R12 requires both nodes running and discoverable on the network.

## Alternatives Considered

**Keep the 0–6 numbering (pipeline + cross-cutting in one sequence)**
Rejected. Mixing sequential and orthogonal concerns in a single number line obscures
both. Stages 0–5 are sequential; C, E, K are always-on. Different kind, different axis.

**Use a different name (not CEKS)**
Rejected. CEKS is the exact acronym for the four architectural axes
(Circadian · Epigenetic · Kin · Stages). It independently maps to the CEKS abstract
machine from programming language operational semantics (Control · Environment ·
Kontinuation · Store) — a convergence too meaningful to discard.

**Build E before Stage 0 (Immune)**
Rejected. Stage 0 is a defensive layer. Memory without integrity is unreliable — the
epigenetic store could be poisoned by unverified mutations. Stage 0 must come first.

## Consequences

**Makes easier:**
- The system accumulates institutional knowledge that persists across sessions
- Proposal quality improves over time without retraining a model
- Multi-node deployments share adaptations automatically via K
- Temporal patterns (market rhythms, seasonal forcing) are handled declaratively
- The pipeline architecture is unambiguous — two distinct axes, not one mixed list

**Makes harder:**
- E requires a distillation cycle (background job or scheduled tick)
- K requires a network-reachable endpoint on each node
- C requires parsing and evaluating cron expressions at runtime
- The system now has more moving parts that need operational observability

**What the AI must know to work in this codebase:**
- `src/runtime/` is the CEKS runtime — the execution engine for BIOISO entities
- Stages 0–5 are sequential pipeline steps (`immune.rs`, existing layers, `deploy.rs`)
- C, E, K are cross-cutting modules: `circadian.rs`, `epigenetic.rs`, `colony.rs`
- The mutation gate (Stage 4) is non-negotiable and never gated by Circadian
- E is a read dependency for Stages 1, 2, 3 — inject before synthesis, not after
- K writes back into E — this is the horizontal gene transfer mechanism
- BIOISO = the language philosophy; CEKS = the runtime that implements it

## Module additions

```
src/runtime/
  immune.rs       — Stage 0: hash lineage verification (R8)
  epigenetic.rs   — E: memory bus, Buffer/Working/Core, decay (R9, R11)
  circadian.rs    — C: cron gates + SNR gate (R10)
  colony.rs       — K: HTTP gossip, colony registry, broadcast (R12)
```
