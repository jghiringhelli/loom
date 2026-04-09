# ADR-0007: Ganglionic Monitoring Architecture

**Date:** 2026-04-07  
**Status:** Accepted  
**Context:** Self-monitoring, signal emission, and fallback intelligence for Loom programs

---

## Context

Loom programs that run as autonomous agents (BIOISO beings) need health monitoring,
anomaly detection, and self-reporting. The naive approach is a central LLM that watches
everything. This creates a single point of failure, requires internet access, and
introduces latency that violates the real-time constraints of financial and scientific systems.

The question is: **what is the minimal, robust, dependency-free architecture for
self-monitoring Loom programs?**

Biological nervous systems provide a four-paradigm answer, each appropriate to a
different layer of Loom's architecture.

---

## Decision

Loom adopts a **four-layer nervous system architecture** drawn from biological precedent.
No layer depends on the layer above it. Each layer degrades gracefully when higher layers
are unavailable.

---

## The Four Layers

### Layer 1 — Ganglionic (Cockroach nervous system)
**Biology:** Cockroaches have segmental ganglia — nerve clusters one per body segment.
Each ganglion controls its segment autonomously. The animal continues to walk, mate, and
respond to stimuli for days after decapitation. No central brain required for survival.

**Loom mapping:** Every `being:` is a ganglion. It monitors its own health using:
- `regulate:` bounds — if a field exceeds its bounds, the being fires an anomaly signal
- `journal:` — records every `evolve:` step; anomalies are detectable from the record
- `telomere:` countdown — the being knows its own age and fires a pre-apoptosis signal
- `@bounded_telos` — enforces that the being stays within its declared operational envelope

**Implementation:** Purely structural. No runtime LLM. The compiler enforces the contracts;
the signals are structured Loom values that any observing being can consume.

**Resilience:** Works with zero external dependencies. The ganglion fires even when the
central AI, the network, and the GitHub API are all down.

---

### Layer 2 — Nerve Net (Hydra nervous system)
**Biology:** Hydra has no brain. Its neurons form a distributed mesh where every neuron
talks to its neighbors directly. Behavior emerges from local interactions. There is no
coordinator, no hierarchy, no center.

**Loom mapping:** Peer-to-peer signal propagation between beings in the same module.
When Being A detects an anomaly, it emits a signal into the module's signal bus.
Being B, which has declared `umwelt: detects: [AnomalySignal]`, receives it and
may escalate or absorb it. No coordinator required.

**Implementation:** The `umwelt:` + `regulate:` combination. A being's declared
detects list determines which signals it responds to. This is pure module-level
reactive programming — no goroutines, no event loop, no broker.

**Key property:** The mesh is self-organizing. Adding a new being automatically
extends the signal coverage without modifying any existing being.

---

### Layer 3 — Micro-LLM (Cerebellum + Hippocampus)
**Biology:**
- **Cerebellum:** Handles fast learned patterns — balance, fine motor control, prediction.
  Operates below conscious awareness. ~69 billion neurons but handles only pattern matching.
- **Hippocampus:** Encodes new experiences as index entries. During sleep, replays
  experiences to the neocortex for consolidation. Short-term → long-term transfer.

**Loom mapping:** A Loom-specific transformer model, 50–100M parameters, trained
exclusively on:
1. All `.loom` source files (positive examples)
2. All compiler error outputs (labeled `(program, error)` pairs)
3. ALX experiments (complex cross-feature programs)
4. The GS spec, ADRs, roadmap (architectural knowledge)
5. `journal:` replay logs from deployed beings (hippocampal consolidation)

**Why tiny is correct:**
- Loom's token vocabulary is ~200 tokens. GPT-4 has ~100,000. A Loom-specific model
  needs two orders of magnitude fewer parameters to achieve complete coverage.
- A 100M parameter model in INT4 quantization (GGUF format) fits in ~50MB RAM.
  Runs on any CPU, no GPU, no API, no internet.
- The training data is ~750 test cases + 5 ALX experiments = small but perfectly labeled.

**Training cycle (sleep consolidation):**
```
collect journal: entries from running beings
   ↓
replay (program, outcome) pairs through fine-tuning
   ↓
prune weak associations (dropout on low-frequency patterns)
   ↓
consolidate into updated GGUF weights
   ↓
distribute new weights to all being instances
```

**Implementation path:**
1. Extract Loom token vocabulary → custom BPE tokenizer
2. Train base model on `.loom` corpus (self-supervised, next-token prediction)
3. Fine-tune on `(program, error)` pairs (supervised)
4. Quantize to GGUF INT4 via llama.cpp
5. Embed in Loom runtime as a `with_micro_llm` feature flag (Cargo feature)

---

### Layer 4 — Central AI (Prefrontal Cortex)
**Biology:** The prefrontal cortex handles deliberation, planning, cross-domain reasoning,
and working memory. It is the most recent evolutionary addition and the most expensive to
run. It relies on all lower layers functioning.

**Loom mapping:** External LLM (Claude, GPT, local Ollama) connected via MCP or API.
Receives structured signals from Layer 1-3, performs cross-domain reasoning, files
GitHub issues, proposes ALX experiments, and writes new ADRs.

**Key constraint:** Layer 4 is the ONLY layer that can fail without consequences.
Layers 1-3 operate independently. Layer 4 is an enhancement, not a requirement.

**Signal format (structured, not free-form):**
```loom
type AnomalyReport
  being_name: String
  signal_type: SignalType
  severity: Severity  -- Critical | High | Medium | Low
  journal_entries: List<JournalEntry>
  telos_drift: Float  -- measured divergence from stated purpose
  timestamp: Timestamp
end
```

---

## The Hybrid is Stronger

```
Layer 4: Central AI ──── cross-domain reasoning, strategic decisions
    ↑
Layer 3: Micro-LLM ──── Loom-universe pattern matching, error explanation
    ↑
Layer 2: Nerve net ──── peer signal propagation, emergent module behavior
    ↑
Layer 1: Ganglionic ──── being-local health, survive with all above offline
```

Each layer receives signals from below and passes escalated signals above.
The system operates at full capability when all four layers are active.
It degrades gracefully: Layer 4 offline → Layer 3 handles. Layer 3 offline →
Layer 2 handles. Layer 2 offline → Layer 1 still guarantees local health enforcement.

The cockroach can still walk without its head. The head is just better when present.

---

## Additional Biological Paradigms (Future Layers)

### C. elegans connectome — formal verification layer
302 neurons, fully mapped, deterministic, reproducible. Every possible behavior
is enumerable. Maps to Loom's `certificate:` + SMT bridge + `correctness_report:`.
A being can be formally verified (complete connectome) or heuristically monitored
(partial mapping). The SMT bridge (M100) is the C. elegans layer.

### Octopus arm autonomy — domain module intelligence
2/3 of the octopus's neurons are in its eight arms. Each arm solves local problems
independently; the central brain coordinates but does not micromanage. Maps to
Loom's domain libraries: a `finance` module has its own sophisticated checker suite
that the core compiler need not understand. The arm is smart; the brain stays clean.

### Immune system — innate + adaptive defense
- **Innate:** Static checkers (type safety, boundary checks, safety constraints).
  Fast, non-specific, always-on. Cannot be bypassed.
- **Adaptive:** Micro-LLM fine-tuned on project-specific error patterns.
  Slow, specific, learns from exposure. Improves with experience.
The innate system (compile-time checkers) runs in microseconds like a macrophage.
The adaptive system (micro-LLM) runs in milliseconds like a B-cell response.

### Proprioception — self-location
The body's sense of its own position. Not sensing the world, but sensing itself.
Maps to `@transparent` + `manifest:` + ALX self-description. Loom knowing precisely
where it is in its own evolutionary trajectory, what it has proved about itself,
and what remains unverified.

### Sleep consolidation — continuous learning
The hippocampus replays experiences to the neocortex during sleep. Weak connections
are pruned; strong ones reinforced. Maps to the micro-LLM training cycle:
nightly replay of all `journal:` entries + ALX runs → fine-tune → deploy updated
weights. The system gets better at Loom-universe pattern matching over time.

---

## Consequences

**What becomes easier:**
- Deploying Loom programs in air-gapped environments (no internet, no central AI)
- Progressive enhancement: start with Layer 1, add layers as infrastructure permits
- Fault tolerance: any layer can fail without system collapse
- Cost: the micro-LLM eliminates API costs for Loom-universe reasoning

**What becomes harder:**
- Training the micro-LLM requires a training infrastructure (one-time setup)
- Keeping micro-LLM weights current requires a consolidation pipeline
- The boundary between Layer 3 (micro-LLM) and Layer 4 (central AI) must be explicit
  to avoid double-processing (same signal escalated twice)

**What the AI needs to know:**
- The ganglionic layer is structural enforcement, not observational. It does not "watch"
  — it enforces. A violated bound is a compile error or a signal, not a log message.
- The micro-LLM handles only Loom-universe reasoning. Cross-domain knowledge
  (chemistry formulas, financial regulations, physics constants) belongs to Layer 4.
- Layers 1-3 are features of the Loom runtime. Layer 4 is always external.

---

## ADR Cross-References
- ADR-0001: Core stack selection
- ADR-0002: Authentication strategy  
- ADR-0003: Hexagonal architecture
- M102: Provenance annotations (auditable layer)
- M103: Boundary blocks (composable layer)
- M104: Journal (hippocampal episodic memory)
- M106: Migration (evolvable layer)
- M108: Diagram emission (proprioception layer)
- M111: Evolution vector checker (semantic clustering)

---

## Chronicle Integration — The Hippocampal Layer is Already Built

**Chronicle MCP** (`workspace/PragmaWorks/mcp/chronicle`) implements a fully working
hippocampal memory system. Layer 3 of this architecture does NOT need to be built from
scratch. Chronicle IS the hippocampus. Loom's monitoring architecture integrates with it.

### Memory type mapping

| Chronicle type | Decay | Loom source |
|---|---|---|
| `episodic` | fast (half-life ~7d) | `journal:` block entries (M104) — events, anomalies, what happened |
| `semantic` | slow (half-life ~35d) | type lattice facts, checker warnings — what is currently true |
| `procedural` | none | Migration patterns (M106), ALX-proven sequences — how to evolve |
| `architectural` | none | ADRs, `manifest:` blocks (M101) — why it was built this way |
| `insight` | none (synthesized) | M111 evolutionary clusters — cross-being patterns |

### Storage tier mapping

| Chronicle tier | Loom equivalent |
|---|---|
| `buffer` → `working` → `core` | Layer 1 signal → Layer 2 propagation → Layer 3 consolidation |
| Tier promotion at 3/10 accesses | Pattern is "learnt" when confirmed across 3+ beings or 10+ ALX runs |
| Weight formula: `w += boost × (1-w)` | Same formula for evolutionary pattern reinforcement in M111 |

### What Chronicle's roadmap item "vector embeddings" already is

Chronicle's roadmap lists "Vector embeddings for semantic `recall`" as a future item.
Loom's M111 EvolutionVectorChecker provides exactly these vectors: a 12-dimensional
type-lattice embedding where each dimension encodes semantic position in the type space
(numeric_int, numeric_float, numeric_precise, string_raw, ..., monetary, boolean, composite).

These vectors can be stored directly in Chronicle's `embedding` field on each memory.
A `journal:` entry about a Float→Double migration gets vector `[0,1,0, 0,0,0, ...]`.
Chronicle's recall then surfaces semantically similar past migrations — cross-project,
cross-session, across all beings that have ever evolved through similar type space.

### The Distill pipeline IS the sleep consolidation cycle

Chronicle's `DistillService` builds a prompt from session memories and asks an LLM to
synthesize `profile`, `lesson`, `playbook`, and `bias` insights. This is exactly the
hippocampal sleep consolidation pattern:

```
journal: entries (episodic)  →  Chronicle session_end  →  distill  →  insight memories
                                                                          ↓
                                                              micro-LLM fine-tune corpus
```

The micro-LLM's training data IS Chronicle's `insight` table. The distill step
consolidates episodic journal entries into cross-session procedural and architectural
memories that then become training examples for the Loom-specific model.

### Integration architecture

```
Loom being (runtime)
  │  journal: entry (M104) ─────────────────────────────► Chronicle: remember(episodic)
  │  migration: step (M106) ────────────────────────────► Chronicle: remember(procedural)
  │  regulate: violation ────────────────────────────────► Chronicle: remember(episodic) + trigger
  │  M111 vector match ──────────────────────────────────► Chronicle: embedding on memory
  │
  └─ ALX experiment result ─────────────────────────────► Chronicle: remember(architectural)

Chronicle session_end
  └─ distill ─────────────────────────────────────────► insights table
                                                         └─ micro-LLM fine-tune corpus
                                                              └─ GGUF INT4 weights
                                                                   └─ Loom runtime Layer 3
```

### What this means for implementation order

1. **Now (structural):** `journal:` and `migration:` blocks already emit the right data.
   No new Loom compiler work needed for Chronicle integration.
2. **Layer 1 complete:** Ganglionic monitoring is structural enforcement (compile-time).
3. **Layer 2 next:** Wire `umwelt: detects:` signals to a signal bus interface that
   Chronicle's trigger system can observe.
4. **Layer 3:** Chronicle + M111 vectors + distill pipeline → micro-LLM training corpus.
   Build the BPE tokenizer on Loom's ~200-token vocabulary. Train, quantize (GGUF INT4).
5. **Layer 4:** MCP connection to external LLM, consuming Chronicle's `core` tier memories
   as context and filing GitHub issues from `AnomalyReport` structured values.

The key simplification: **do not build a separate memory system for Loom's self-monitoring.
Use Chronicle. Loom programs ARE Chronicle clients.** Each being that runs produces
Chronicle memories. The distill pipeline is the connection between episodic experience
and architectural knowledge. The micro-LLM emerges from the consolidated insights table.
