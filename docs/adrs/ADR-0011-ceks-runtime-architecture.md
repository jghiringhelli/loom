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

### 8. Build roadmap (R8–R12)

| Phase | Component | Depends on |
|---|---|---|
| R8 | Stage 0: Membrane / Immune (hash lineage + security metadata → E) | Store (done) |
| R9 | E: Epigenetic Buffer tier (raw outcome logging) | Store (done), Stage 5 (done) |
| R10 | C: Circadian (cron gates + SNR gate) | Store, Orchestrator (done) |
| R11 | E: Working + Core tiers (Ollama/Claude distillation) | R9 (Buffer), GPU laptop |
| R12 | K: Colony gossip (HTTP, cross-node E sync, offline cache, hibernation) | R9 (E), multi-node setup |

R8 and R9 can be built without new infrastructure.
R10 requires the `cron` crate (pure Rust, no OS daemon).
R11 requires Ollama running on the GPU laptop.
R12 requires both nodes running and discoverable on the network.

---

### 9. Extended Validation Pipeline (R13–R15)

After Stage 4 (Gate), the proposal enters a four-stage biological validation pipeline
that mirrors the drug development pathway: _in silico_ → _in vitro_ → _in vivo partial_
→ full production.

```
[4: Gate] ──► [5: Simulation] ──► [6: Soft Release] ──► [7: Acclimatization] ──► [8: Propagation]
               in silico           in vitro                in vivo partial            full wild
```

**Stage 5 — Simulation (digital twin, _in silico_)**

Biological analog: predictive coding / mental rehearsal. The mammalian brain simulates
outcomes before committing to action. Zero-cost elimination of visibly bad strategies.

- Replay entity's historical signals against the proposed mutation
- Measure telos delta across the full signal history
- Check whether the entity reaches Senescent or Dead lifecycle state in simulation
- Failure writes negative Semantic entry to Epigenome: "this mutation class collapses
  telos under these signal conditions"

**Stage 6 — Soft Release (mesocosm, _in vitro_-equivalent)**

Biological analog: conservation biology soft release — animals reintroduced to a
protected enclosure within their natural habitat before full wild release.

- Deploy to isolated real environment with no production traffic
- Run N ticks with real infrastructure (DB, network, clocks — no mocks)
- Survival criteria: entity alive, telos not worse, no invariant fires, signals continue
- **Security hardening**: full pre-release security test suite runs here
  (MITM simulation, DoS probe, input fuzzing, reverse-engineering resistance checks)
- **Passing writes to Epigenome Procedural tier** (never decays — immunological memory)
- Failure: rollback to checkpoint, negative Procedural candidate flagged to Cortex

**Stage 7 — Acclimatization (low-traffic real, _in vivo_ partial)**

Biological analog: acclimatization — physiological adjustment to new conditions via
partial exposure before full habitat release.

- Deploy to low-traffic production slice
- Measure telos delta against full-traffic baseline
- Colony notified of delta; siblings receive the Episodic entry regardless of outcome
- Failure: rollback + Episodic entry; Gaia telos aggregate recalculated

---

### 10. Propagation Decision — Mitosis vs. Meiosis (R16)

At Stage 8, before production, the system decides: **mutate internally** or **spawn a
new entity instance**. This maps to mitosis vs. meiosis.

**Mitosis** (internal update):
- Same entity instance, new behavioral parameters
- Triggered when: telos improves, structural divergence is low, no new signal types

**Meiosis** (spawn new entity):
- A new entity instance is created with a recombined genome
- Triggered when: structural divergence exceeds threshold, new signal types detected,
  or operator configures explicit clone policy
- Registered in the Colony K layer with parent lineage

**Multi-parent recombination** (generalized, not limited to 2 parents):
- New offspring can inherit orthogonal traits from N parents
- Each parent contributes the trait package where it has highest Epigenome fitness score
- Cortex (Stage 3) selects the recombination map when N > 2
- Biological analog: horizontal gene transfer + RNA segment reassortment (influenza)

**Git lineage protocol**:
- Structurally distinct entities get their own branch: `entity/<colony-id>/<variant-id>`
- Branch is preserved when the entity exhausts its telomere or is confirmed worse via
  Epigenome evidence — it becomes read-only history, never deleted
- Confirmed-worse mutation + epigenetic evidence → accelerated telomere burn
- Clones inherit most Epigenome entries of parent (Procedural: all; Semantic: copy;
  Episodic: none — a new instance starts with no personal history)

**Mutation Independence Testing — Eigendecomposition of the Effect Matrix**

This solves the **epistasis problem** (also: clonal interference, sign epistasis,
Muller's Ratchet). It is the fundamental reason sexual reproduction exists.

**The problem:** Mutation A has telos effect +0.3. Mutation B has telos effect −0.3.
If both are applied together and measured as a scalar, Δtelos = 0. The system
concludes neither mutation is significant and discards both. Mutation A — which
was beneficial — is silently lost.

**Root cause:** telos is not a scalar. It is a vector across M measurable dimensions
(throughput, latency, accuracy, cost, survival rate, …). Two mutations that cancel
each other in one dimension may be entirely independent in the others.

**The Meiotic Pool — mutation staging area:**

Mutations do not go directly from Gate (Stage 4) to Propagation (Stage 8).
They accumulate in a staging area and are tested individually in Simulation
(Stage 5) first, computing a full telos **effect vector** (not a scalar):

```
effect_vector[i] = Δtelos per dimension for mutation i
                 = [Δthroughput, Δlatency, Δaccuracy, Δcost, ...]
```

Once N mutations have accumulated (configurable, default N=5), or a time window
expires, the system runs eigendecomposition of the effect matrix E (shape: M×T,
M mutations × T telos dimensions):

```rust
// Conceptually:
let E: Matrix<f64> = build_effect_matrix(&pending_mutations);
let (singular_values, left_vecs, right_vecs) = svd(E);
// right_vecs: orthogonal basis of telos-effect space
// Each mutation projected onto this basis reveals its independence
```

**Reading the result:**

| Relationship between mutations | Effect vector geometry | Action |
|---|---|---|
| Orthogonal | Vectors perpendicular in telos space | Combine in single offspring — effects are additive, no interference |
| Parallel (same direction) | Vectors point the same way | Combine — reinforcing, test together |
| Anti-parallel (opposite direction) | Vectors cancel | **Separate lineages** — do not combine. Each is tested in isolation. If one is +0.3 and the other −0.3, combining them hides both. |
| Partially correlated | Oblique angle | Decompose: project onto orthogonal components. Combine the orthogonal part; defer the correlated part. |

**Biological grounding:**
- Orthogonal mutations = mutations on different chromosomes (free recombination)
- Correlated mutations = mutations on the same chromosome (linkage disequilibrium)
- Anti-parallel mutations = sign epistasis — the known case where beneficial alleles
  cancel each other when combined; the reason recombination evolved

**The algorithm in practice:**
1. Every mutation passing Stage 4 (Gate) enters the Meiotic Pool with status `pending_independence_test`
2. Simulation (Stage 5) runs each pending mutation **in isolation** → records effect vector
3. Once the pool reaches threshold N or timer fires, SVD of effect matrix
4. Orthogonal clusters → candidate for a single multi-parent offspring (Propagation Stage 8)
5. Anti-parallel pairs → each gets its own lineage branch; telomere budget allocated separately
6. Results written to Epigenome Semantic tier: "mutations M3 and M7 are anti-parallel on
   latency dimension — do not recombine"

**ICA extension (long-term):**
SVD finds *uncorrelated* components. ICA (Independent Component Analysis) finds
*statistically independent* components — a stronger guarantee. Once the Epigenome
has accumulated enough signal history, an ICA pass on the full mutation effect corpus
can discover the true orthogonal axes of the entity's telos space, potentially
revealing that the declared telos dimensions are not the natural coordinates of the
system. These empirical coordinates are federated back to the Cortex and used to
update the entity's `.loom` telos block in the next meiotic cycle.

---

### 11. Ganglion Micro-LM Evolution Path

The current Ganglion (Stage 2) starts as Ollama with a general-purpose model.
The long-term architecture is a distributed localized nervous system: self-contained
micro-LMs that understand both BIOISO semantics and the specific app they serve.

**Evolutionary stages:**

| Stage | Technology | Training | Infrastructure |
|---|---|---|---|
| G1 (current) | Ollama + general model (Phi-3, Qwen2.5) | None | GPU laptop |
| G2 | Ollama + fine-tuned on promoted outcomes | Epigenome Procedural corpus | GPU laptop |
| G3 | GGUF-quantized micro-LM, ONNX runtime | Entity-specific signal history | CPU, per-node |
| G4 | Distributed micro-LMs per signal domain | Domain-specific promoted outcomes | CPU, per-entity |

**Training pipeline** (G2+):
- Source: Epigenome Procedural + Semantic tiers (highest-quality, non-decaying knowledge)
- Format: (signal_context, genome_excerpt) → mutation_proposal pairs
- Supervised by Cortex: Claude reviews training examples before they enter the corpus
- Distillation: large Cortex model supervises compression into small Ganglion model

**Infrastructure goal**: each deployed entity node carries its own micro-LM. No network
required for Stage 2 synthesis. True peripheral nervous system — autonomous, distributed,
no dependency on GPU laptop or external API for the common case.

---

### 12. Security Metadata at Membrane → Epigenome (R8 extension)

Stage 0 (Membrane) does not just admit or reject — it classifies and annotates.
Every admitted signal and mutation receives a security category written to the
Epigenome Security tier (a new sub-type alongside Episodic/Semantic/Procedural/Preference).

**Security classification written at Stage 0:**

| Threat | Detection mechanism | Epigenome entry |
|---|---|---|
| MITM / signal spoofing | HMAC signature on all signals | `security::signal_tamper_detected` |
| DoS | Token bucket rate limiting per source entity | `security::rate_limit_exceeded` |
| Genome tampering | SHA-256 lineage hash chain break | `security::genome_hash_mismatch` |
| Faulty hardware / sensor | Signal plausibility bounds check | `security::implausible_signal` |
| Unauthorized mutation | Source entity not in colony registry | `security::unregistered_mutation_source` |

**Effect on pipeline:**
- Security entries are read by Reflex (Stage 1) as rule-matching context
- A source with repeated `signal_tamper_detected` entries is down-weighted automatically
- Cortex (Stage 3) receives security summary in its system prompt
- Soft Release (Stage 6) includes a security hardening test pass before promotion

---

### 13. Umwelt Spectrum

Umwelt is not binary. Each entity declares a perception scope on a three-level spectrum:

| Level | Name | Sees | Use case |
|---|---|---|---|
| 0 | Restricted (default) | Only signals declared in this entity's `.loom` source | All domain entities |
| 1 | Domain | All signals emitted within the entity's ecosystem | Cross-entity correlation |
| 2 | Omniscient | All signals across all entities in the entire runtime | Analytics, monitoring, Gaia-level observers |

The Membrane (Stage 0) enforces the declared scope. Signals outside scope are
silently dropped at entry — they do not exist for that entity.

Omniscient entities enable cross-input distribution discovery (correlation, anomaly
detection across entity boundaries) without exposing that scope to domain entities.
Gaia Telos is computed by an Omniscient observer; individual entities remain scoped.

---

### 14. Bayesian Allostery

Allostery — context-sensitive signal interpretation — is implemented via Bayesian
inference in the Reflex stage (Stage 1) rule-matching layer.

**Formulation:**
```
P(interpretation | signal, context) ∝ P(signal | interpretation, context) × P(interpretation | context)
```

Where:
- **Prior** `P(interpretation | context)`: Epigenome history of this `(signal_type, entity_state)` pair
- **Likelihood** `P(signal | interpretation, context)`: current signal value + adjacent context signals
- **Posterior**: updated distribution over interpretations; the rule fires on the MAP estimate

**Effect**: the same signal value of `0.8` for `metabolism_rate` means "critical high"
when entity is in `Stressed` state and `normal_operating` when in `Peak` state — because
the prior distributions differ.

**Implementation note**: Naïve Bayes sufficient for initial implementation (independence
assumption across context signals). Full Bayes (Bayesian network) if cross-signal
dependencies are needed later.

---

### 15. Colony Offline Resilience + Hibernation (R12 extension)

**Local cache:**
- Persist last N Mycelium broadcasts to disk (SQLite, same store as signal store)
- Cache survives process restart; replayed on reconnect in chronological order

**Fallback mode (colony unreachable):**
- Entity continues on local Epigenome only
- Colony broadcast queue accumulates locally
- No proposals written to shared K layer until reconnect

**Catch-up on reconnect:**
- Replay queued broadcasts in timestamp order
- Merge Epigenome deltas: newer timestamp wins; Procedural memories are protected
  from remote overwrites (local Procedural is never overwritten by colony broadcast —
  only reinforced or supplemented)

**Hibernation:**
- Threshold: `disconnected_duration > T` AND `local_telos_stable` (variance < ε over last W ticks)
- Effect: reduce tick rate to `H%` of normal (configurable, default 10%)
- Biological analog: tardigrade cryptobiosis / bear hibernation — reduce metabolic cost
  when environment is stable and no communication is available
- Wake: on reconnect OR telos drift exceeds emergency threshold

---

### 16. Classic Optimization Algorithms Embedded in CEKS

Each biological behavior in the runtime is backed by a well-understood classical
optimization algorithm, applied locally within its scope:

| Biological behavior | Classic algorithm | CEKS application | Location |
|---|---|---|---|
| Stigmergy (pheromone trails) | Ant Colony Optimization (ACO) | Signal weight = pheromone strength; Circadian decay = evaporation rate; Colony broadcast = trail reinforcement | Signal store + K layer |
| Punctuated equilibrium | Simulated Annealing (SA) | Temperature = current telos stability score; mutation rate = f(1/T); cooling schedule = telos stability trend over W ticks | Orchestrator tick scheduler |
| Multi-parent recombination | Genetic Algorithm (GA) crossover | Parent trait packages selected by Epigenome fitness per domain; crossover point selected by Cortex | Stage 8 Propagation Decision |
| Colony coordination | Particle Swarm Optimization (PSO) | Entity positions in telos space; velocity = drift direction; swarm attractor = Gaia telos target | K layer + Gaia telos aggregate |
| Signal noise suppression | Kalman filter | State estimate = running signal mean; measurement noise covariance = Circadian SNR | Stage 0 Membrane + SNR gate |
| Mutation independence testing | SVD / Eigendecomposition ("eigen" = German for *intrinsic*) | Effect matrix E[mutation × telos_dim]; SVD reveals orthogonal vs anti-parallel mutations; prevents sign epistasis masking | Meiotic Pool (Stage 5 → Stage 8) |
| Telos space discovery | ICA (Independent Component Analysis) | Finds statistically independent axes of telos space from accumulated signal history; updates declared telos dimensions in `.loom` source | Epigenome distillation cycle (monthly) |

These algorithms are not novel. They are deployed locally within their biological
analog's scope — they are not global optimizers for the whole system.

---

### 17. Immune Checkpoint Healing (quarantine + selective reconstruction)

**Biological analog:** Leukocyte response — white blood cells isolate, neutralize, and
signal the colony. The organism doesn't shut down; the *affected compartment* does.

**Attack detected at Stage 0 → Quarantine protocol:**

```
ATTACK DETECTED
      │
      ▼
Entity → Quarantine state (stop processing external signals)
      │
      ▼
Increment quarantine_window (exponential backoff): 1m → 5m → 30m → 1h → 24h
      │
      ▼
At each window: re-probe — is threat signature still present?
  YES → extend window, broadcast quarantine to Colony (K)
  NO  → reconstruct affected module from last promoted checkpoint
      → selective redeploy (only the compromised module, not the full entity)
      → resume with heightened Membrane sensitivity for N ticks
```

**Modular binary reconstruction:**
- Each compiled module tracks its own hash in the immune registry independently
- If `auth.loom` module is compromised, reconstruct only that module from checkpoint
- Other modules continue operating — the entity is partially isolated, not dead
- Biological analog: immune response isolates a tissue region; the organism continues

**This closes the tolerance gap**: promoted mutations write their new hash to the
immune registry as *self*. The checkpoint healing system rebuilds from the most recent
*promoted* checkpoint — the last known-good, compiler-verified, telos-improving state.
If all promoted checkpoints are compromised, escalate to git lineage (immutable history).

---

### 18. Degeneracy — Pass-Through Fallback Chain

Each pipeline layer has exactly two degenerate paths: **pass-through** or **sleep**.
No new mechanism — the escalation chain already *is* degeneracy. Made explicit:

| Layer | Primary | Degenerate path A | Degenerate path B |
|---|---|---|---|
| Membrane (0) | Reject non-self | Colony-vouched pass-through with warning | Block all + quarantine |
| Reflex (1) | Match rule | No rule matched → pass-through to Ganglion | — |
| Ganglion (2) | Ollama synthesis | Offline → pass-through to Cortex | — |
| Cortex (3) | Claude synthesis | Budget exceeded → pass last known good proposal | No proposal → pass-through to Gate empty |
| Gate (4) | Compile + check | Never degenerates — always runs or blocks | — |
| Simulation (5) | Digital twin | No history → skip, pass-through to Soft Release | — |
| Soft Release (6) | Isolated env | Environment unavailable → skip with warning | — |
| Propagation (8) | Mitosis/meiosis | Meiotic Pool empty → mitosis default | — |

The escalation chain IS degeneracy. Each layer's "miss" is its degenerate path.
No new module needed. Degeneracy is a documentation property of the chain, not a system.

---

### 19. Developmental Gating via Relative Telomere Length

Juvenile entities cannot reproduce. Senescent entities cannot mutate.
No new mechanism — telomere already tracks this. Made explicit:

```
relative_telomere = current_telomere / max_telomere

> 0.9  →  Juvenile:   mitosis only, no meiosis, no offspring spawning
0.5–0.9 →  Mature:    full pipeline, meiosis enabled, multi-parent recombination allowed
0.1–0.5 →  Senescent: mutations allowed but Cortex is consulted on every proposal
< 0.1  →  Terminal:   read-only, no new mutations, prepare checkpoint, notify Colony
= 0    →  Dead:       apoptosis, final Colony broadcast, git branch archived
```

Checked at Stage 8 (Propagation) before any meiosis decision.
The threshold values are configurable per entity type in the `.loom` source.

---

### 20. Circadian + Colony → Infrastructure Autoscaling

Sleeping cycles make infrastructure scaling a natural consequence of biology,
not a separate orchestration problem.

**Mechanism:**
- Colony hibernation (§15): entity reduces tick rate to H% → CPU/memory usage drops
- Circadian suppression (§4): no proposals generated in suppression window
- Together these produce measurable low-load periods that map to scale-down signals

**What the entity emits:**
```rust
// Entity becomes a first-class signal source for its own infrastructure
Signal::InfrastructureHint {
    direction: ScaleDirection::Down,
    reason: "circadian_suppression + colony_hibernation",
    suggested_instances: 1,
    resume_at: next_cron_window,
}
```

**Receiving side:** any orchestrator (Kubernetes HPA, Docker Swarm, bare systemd)
listens on this signal channel and acts accordingly. The entity defines its own
scaling policy via Circadian cron expressions — `0 2 * * *` means "scale down at 2am"
without configuring anything in the orchestrator. The biology drives the infrastructure.

**Biological analog:** the organism's metabolic rate determines the resources it draws
from the ecosystem. A sleeping organism doesn't need to be told to reduce consumption —
it just does. The ecosystem (infrastructure) responds to the draw, not to instructions.

---

### 17. Updated module additions

```
src/runtime/
  immune.rs        — Stage 0: hash lineage + security classification → E (R8)
  epigenetic.rs    — E: memory bus Buffer/Working/Core + Security tier, decay (R9, R11)
  circadian.rs     — C: cron gates + SNR gate + Kalman pre-filter (R10)
  colony.rs        — K: HTTP gossip, offline cache, hibernation, ACO stigmergy (R12)
  simulation.rs    — Stage 5: digital twin signal replay, telos delta (R13)
  soft_release.rs  — Stage 6: isolated real env, security hardening, Procedural write (R14)
  acclimatization.rs — Stage 7: LTE slice, telos delta vs baseline (R15)
  propagation.rs   — Stage 8: mitosis/meiosis decision, multi-parent recombination (R16)
```

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
- Pipeline Stages 0–8 are sequential. Stages 0–4 were R1–R7; Stages 5–8 are R13–R16.
- C, E, K are cross-cutting modules: `circadian.rs`, `epigenetic.rs`, `colony.rs`
- The mutation gate (Stage 4) is non-negotiable and never gated by Circadian
- E is a read dependency for Stages 1, 2, 3 — inject before synthesis, not after
- E has 5 memory sub-types: Episodic / Semantic / Procedural / Preference / Security
- K writes back into E — this is the horizontal gene transfer mechanism
- Stage 6 (Soft Release) writes to Procedural memory (never decays — hardening)
- Stage 8 (Propagation) decides mitosis vs meiosis — internal update vs new instance
- Multi-parent recombination: N parents, Cortex selects trait map, Epigenome fitness scores guide selection
- Ganglion starts as Ollama; evolves toward CPU-optimized micro-LMs per signal domain
- Umwelt is a 3-level spectrum: Restricted / Domain / Omniscient — not binary
- Bayesian Allostery: context-sensitive rule matching uses Epigenome as prior
- Colony offline: local cache, hibernation threshold, catch-up on reconnect; Procedural memories are never overwritten by remote broadcasts
- ACO → stigmergy; SA → punctuated equilibrium; GA → multi-parent recombination; PSO → colony coordination; Kalman → SNR pre-filter
- BIOISO = the language philosophy; CEKS = the runtime that implements it
- Git branches track structural variants; dead branches are history, never deleted

## Module additions

```
src/runtime/
  immune.rs       — Stage 0: hash lineage verification (R8)
  epigenetic.rs   — E: memory bus, Buffer/Working/Core, decay (R9, R11)
  circadian.rs    — C: cron gates + SNR gate (R10)
  colony.rs       — K: HTTP gossip, colony registry, broadcast (R12)
```
