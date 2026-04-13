# Tech Spec: Loom CEMS

## Overview

Loom is an AI-native compiled programming language and self-evolving runtime. The
compiler (written in Rust, stable edition 2021) transpiles `.loom` source files to
Rust, TypeScript, WASM, or OpenAPI. The **CEMS runtime** wraps every compiled program
in a biological lifecycle: entities emit signals, the runtime continuously measures
telos alignment, and when drift is detected it proposes, type-checks, and deploys
self-correcting mutations — autonomously, without operator intervention.

The language philosophy is **BIOISO** (Biologically-Organised Intelligent Self-Optimising
Systems): programs are *beings* with declared purpose (`telos:`), self-repair capability
(`evolve:`, `crispr:`), and bounded lifespans (`@mortal`, `telomere:`). The CEMS
runtime is the execution engine that makes that philosophy operative at runtime.

---

## Architecture

### Two-Axis CEMS Model

The CEMS runtime has two distinct architectural axes that must not be conflated:

**Axis A — The Linear Pipeline (Stages 0–5)**

Sequential. Each stage has a single, bounded responsibility. Escalation moves upward
only on failure. No stage knows about cross-cutting concerns directly; those are injected.

```
[Signal / External Mutation]
         │
         ▼
Stage 0: Membrane         — SHA-256 genome integrity + rate limiting (always runs)
         │ ADMIT
         ▼
Stage 1: Polycephalum     — Tier 1 deterministic rule engine, DeltaSpec, < 50 ms
         │ escalate if no convergence
         ▼
Stage 2: Ganglion         — Tier 2 Ollama HTTP, local LLM synthesis
         │ escalate if Tier 2 fails after window W
         ▼
Stage 3: Mammal Brain     — Tier 3 Claude API, cost-guarded, remote LLM
         │
         ▼
Mutation Gate             — loom::compile() on every proposal, type-safe
         │ APPROVED
         ▼
Simulation Stage          — DigitalTwin, MeioticPool, SVD cosine independence
         │
         ▼
Canary Deployer           — soft release, auto-rollback on regression
         │ observation window passed
         ▼
Survival Gauntlet         — CAE + LTE adversarial hardening (mandatory)
         │ PASSED
         ▼
Stable                    — entity promoted; Epigenome distilled; Mycelium gossip
```

**Axis B — Cross-Cutting Concerns (C · E · M)**

Orthogonal to the pipeline. Always active. No sequence relative to each other.

| Axis | Module | Responsibility |
|---|---|---|
| **C** — Circadian | `circadian.rs` | Temporal gating. Suppresses synthesis during known low-quality windows. Kalman SNR pre-filter rejects noisy signals before drift evaluation. |
| **E** — Epigenome | `epigenome.rs` | Institutional memory. Four tiers: Buffer → Working → Core → Security. Distillation compacts tiers over time. `inherit_from(parent, child)` enables offspring warm-start. |
| **M** — Mycelium | `mycelium.rs` | Colony coordination. Gossip protocol broadcasts promoted mutations. ACO pheromone stigmergy reinforces high-fitness paths. Offline queue for partition tolerance. |

### Module Map

```
src/
  compiler/         — .loom → Rust / TS / WASM / OpenAPI emitters
  runtime/
    orchestrator.rs — CEMS evolution daemon, tick loop
    membrane.rs     — Stage 0: genome integrity, rate limiting, quarantine
    drift.rs        — telos drift computation, Kalman SNR pre-filter
    polycephalum.rs — Stage 1: deterministic rule engine, DeltaSpec
    ganglion.rs     — Stage 2: Ollama HTTP client
    mammal_brain.rs — Stage 3: Claude API client, cost guard
    gate.rs         — Mutation Gate: loom::compile() validation
    simulation.rs   — DigitalTwin, MeioticPool, SVD independence
    canary.rs       — Canary Deployer, auto-rollback
    gauntlet.rs     — Survival Gauntlet: CAE + LTE phases
    epigenome.rs    — E axis: Buffer/Working/Core/Security tiers
    circadian.rs    — C axis: cron gating, Kalman filter
    mycelium.rs     — M axis: gossip, ACO pheromones, offline queue
    bioiso_runner.rs — 11 domain entity specs, RetroValidator
    store.rs        — rusqlite Signal Store interface
  cli/
    main.rs         — clap entry point, all subcommand dispatch
```

---

## Tech Stack

| Component | Technology | Version | Notes |
|---|---|---|---|
| Language | Rust (stable) | edition 2021 | Primary implementation language |
| Build | cargo | 1.x | Standard Rust toolchain |
| CLI framework | clap | ^4 | Derive macros for subcommand dispatch |
| Serialisation | serde + serde_json | ^1 | JSON for API payloads and config |
| Config manifests | serde + toml | ^0.8 | `loom.toml` project manifests |
| Persistence | rusqlite | ^0.31 | Embedded SQLite Signal Store |
| HTTP client | reqwest | ^0.12 | Ganglion (Ollama) + Mammal Brain (Claude API) |
| Cryptographic hash | sha2 | ^0.10 | SHA-256 genome integrity (Stage 0 Membrane) |
| Linear algebra | nalgebra | ^0.33 | SVD cosine independence in Simulation Stage |
| Random / mutation | rand | ^0.8 | MeioticPool crossover sampling |
| Async runtime | tokio | ^1 | Async HTTP calls in Ganglion and Mammal Brain |
| LSP server | tower-lsp | ^0.20 | `loom-lsp` language server (separate binary) |
| Testing | built-in `cargo test` | — | Unit + integration tests in `tests/` |

---

## Data Flow

```
.loom source
     │ loom compile
     ▼
compiled Rust artifact  ──→  SHA-256 genome hash  ──→  Signal Store (entity_registry)
     │
     │ loom runtime start
     ▼
Orchestrator (tick loop every TICK_MS ms)
     │
     ├─ entity.emit_signal(name, value)
     │         │
     │         ▼
     │   Stage 0 Membrane ──(hash verify + rate check)──→ ADMIT / QUARANTINE
     │         │ ADMIT
     │         ▼
     │   Drift Engine ──(Kalman SNR)──→ drift_score
     │         │
     │         ├─ drift < threshold  ──→  Epigenome Buffer write
     │         │
     │         └─ drift ≥ threshold
     │               │
     │         Circadian gate open?
     │               │ YES
     │               ▼
     │         Polycephalum (Tier 1)
     │               │ no convergence → Ganglion (Tier 2) → Mammal Brain (Tier 3)
     │               ▼
     │         MutationProposal
     │               │
     │         Mutation Gate (loom::compile)
     │               │ APPROVED
     │               ▼
     │         Simulation (DigitalTwin + MeioticPool + SVD)
     │               │ passes
     │               ▼
     │         Canary Deployer (observation window)
     │               │ no regression
     │               ▼
     │         Survival Gauntlet (CAE + LTE)
     │               │ PASSED
     │               ▼
     │         Stable promotion
     │               │
     │         Epigenome distil (Buffer → Working → Core)
     │               │
     │         Mycelium gossip + ACO pheromone deposit
     │               │
     │         Signal Store write (all telemetry)
     │
     └─ loom runtime status / log / rollback  ──→  reads Signal Store
```

---

## API Contracts — loom CLI

All commands follow the convention: `loom <noun> <verb> [flags]`.
Exit codes: `0` success, `1` general error, `2` usage/argument error.

### Compiler commands

| Command | Description |
|---|---|
| `loom compile <file.loom>` | Compile `.loom` source; emit Rust by default |
| `loom compile --target rust\|ts\|wasm\|openapi <file.loom>` | Choose emission target |
| `loom build` | Compile all `.loom` files in the project (reads `loom.toml`) |

### Runtime commands

| Command | Description |
|---|---|
| `loom runtime start [--tick-ms N] [--db PATH]` | Start CEMS orchestrator daemon for this project |
| `loom runtime status [--entity <id>]` | Print entity states, drift scores, last mutations |
| `loom runtime log [--entity <id>] [--tail N] [--retro]` | Stream or tail runtime log; `--retro` shows RetroValidator scores |
| `loom runtime rollback --entity <id> --checkpoint <id>` | Restore entity to a named checkpoint |
| `loom runtime spawn <entity_spec> [--parent <parent_id>]` | Spawn a new BIOISO entity; `--parent` enables genetic memory warm-start |
| `loom runtime stop` | Gracefully shut down the CEMS orchestrator |

### Package / module commands

| Command | Description |
|---|---|
| `loom lpn install <package>` | Install a Loom package from the registry |
| `loom lpn publish` | Publish the current project to the Loom package registry |

### Flag conventions

| Flag | Applies to | Meaning |
|---|---|---|
| `--verbose` / `-v` | All commands | Increase log verbosity |
| `--quiet` / `-q` | All commands | Suppress non-essential output |
| `--json` | status, log | Machine-readable JSON output |
| `--output <path>` | compile, build | Write emitted files to directory |
| `--version` | top-level | Print version and exit |

---

## Key Environment Variables

| Variable | Default | Description |
|---|---|---|
| `CLAUDE_API_KEY` | _(required for Tier 3)_ | Anthropic API key for Mammal Brain synthesis |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama endpoint for Ganglion (Tier 2) synthesis |
| `DB_PATH` | `./loom_signals.db` | Path to SQLite Signal Store |
| `TICK_MS` | `1000` | CEMS evolution tick interval in milliseconds |
| `CLAUDE_MAX_CALLS_PER_HOUR` | `10` | Cost guard: maximum Tier 3 API calls per hour |
| `GAUNTLET_CAE_TICKS` | `10` | Survival Gauntlet CAE phase duration (ticks) |
| `GAUNTLET_LTE_TICKS` | `20` | Survival Gauntlet LTE phase duration (ticks) |
| `GAUNTLET_LTE_MAX_DRIFT` | `0.65` | Maximum drift score allowed during LTE phase |
| `GANGLION_DRIFT_THRESHOLD` | `0.7` | Drift score above which Tier 2 is triggered |

All variables can also be set in `loom.toml` under `[runtime]`.

---

## Signal Store Schema (SQLite)

```sql
-- Registered BIOISO entities and their genome hashes
CREATE TABLE entity_registry (
    entity_id       TEXT PRIMARY KEY,
    genome_hash     TEXT NOT NULL,          -- SHA-256 of compiled artifact
    parent_id       TEXT,                   -- NULL if cold-start
    spawned_at      INTEGER NOT NULL,       -- Unix timestamp ms
    state           TEXT NOT NULL           -- Spawned|Active|Canary|Stable|Senescent|Dead|Quarantined|Hibernating|Rollback
);

-- Every signal emitted by a running entity
CREATE TABLE signals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    signal_name     TEXT NOT NULL,
    value           REAL NOT NULL,
    emitted_at      INTEGER NOT NULL,
    drift_score     REAL
);

-- Mutation proposals and their outcomes
CREATE TABLE mutations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    proposal_type   TEXT NOT NULL,          -- ParameterAdjust|EntityClone|EntityRollback|StructuralRewire
    tier            INTEGER NOT NULL,       -- 1|2|3
    proposal_json   TEXT NOT NULL,
    gate_result     TEXT NOT NULL,          -- APPROVED|REJECTED
    canary_result   TEXT,                   -- PROMOTED|ROLLBACK|NULL
    gauntlet_result TEXT,                   -- PASSED|FAILED|NULL
    proposed_at     INTEGER NOT NULL
);

-- Epigenome memory tiers (Buffer/Working/Core/Security)
CREATE TABLE epigenome (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    tier            TEXT NOT NULL,          -- Buffer|Working|Core|Security
    kind            TEXT NOT NULL,          -- Semantic|Procedural|Declarative|Episodic|Relational|Audit
    content         TEXT NOT NULL,
    recorded_at     INTEGER NOT NULL
);

-- Canary checkpoints for rollback
CREATE TABLE checkpoints (
    checkpoint_id   TEXT PRIMARY KEY,
    entity_id       TEXT NOT NULL,
    state_json      TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

-- Membrane security events
CREATE TABLE security_events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    event_type      TEXT NOT NULL,          -- HashMismatch|RateLimitBreach|UnregisteredSignal
    detail          TEXT,
    occurred_at     INTEGER NOT NULL
);

-- Mycelium ACO pheromone trails
CREATE TABLE pheromone_trails (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    mutation_hash   TEXT NOT NULL,          -- hash of the MutationProposal content
    fitness_score   REAL NOT NULL,
    deposit_count   INTEGER NOT NULL DEFAULT 1,
    last_deposit_at INTEGER NOT NULL
);
```

---

## Security & Compliance

| Concern | Mechanism |
|---|---|
| Genome integrity | SHA-256 hash of every compiled artifact registered at spawn; re-verified by Stage 0 Membrane on every signal and external mutation |
| Rate limiting | Token-bucket per-entity rate limiter in Stage 0 Membrane; parameters configurable per entity spec |
| Quarantine | Entities triggering Membrane security events are quarantined for a configurable window before signals are re-admitted |
| Cost control | Tier 3 (Claude API) enforces `CLAUDE_MAX_CALLS_PER_HOUR`; call count persisted in Signal Store |
| Audit trail | Epigenome Security tier is immutable per entity; all security events written to `security_events` table |
| Secret handling | `CLAUDE_API_KEY` read from environment only; never written to Signal Store or `loom.toml` |

---

## Dependencies — Approved Packages

See [`docs/approved-packages.md`](../approved-packages.md) for the full registry with
audit status. Core runtime dependencies:

| Crate | Version | Purpose |
|---|---|---|
| clap | ^4 | CLI argument parsing and subcommand dispatch |
| serde | ^1 | Serialisation framework |
| serde_json | ^1 | JSON payloads for Ollama and Claude API |
| toml | ^0.8 | `loom.toml` project manifest parsing |
| rusqlite | ^0.31 | Embedded SQLite Signal Store |
| reqwest | ^0.12 | Async HTTP for Ganglion (Ollama) and Mammal Brain (Claude) |
| tokio | ^1 | Async runtime for reqwest |
| sha2 | ^0.10 | SHA-256 genome hash computation |
| nalgebra | ^0.33 | SVD cosine independence check in Simulation Stage |
| rand | ^0.8 | MeioticPool crossover sampling |
| tower-lsp | ^0.20 | `loom-lsp` language server (separate binary) |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Tier 3 cost overrun (Claude API) | M | H | `CLAUDE_MAX_CALLS_PER_HOUR` cost guard; call count persisted in Signal Store; alert on approach |
| Genome hash collision (SHA-256) | L | H | Collision probability negligible for artifact sizes; lineage chain stored for auditing |
| Signal Store corruption | L | H | rusqlite WAL mode; checkpoint-based recovery; entity state reconstructable from `mutations` table |
| Gauntlet false-negative (brittle entity promoted) | M | M | LTE phase sustains 2× drift for M ticks; CAE drives to max bounds; both phases mandatory |
| Mycelium gossip storm (large colony) | M | M | Gossip fan-out bounded per tick; offline queue prevents unbounded buffer growth |
| Ollama unavailability (Tier 2 offline) | M | L | Tier 2 failure triggers Tier 3 escalation; system degrades gracefully to higher tiers |
| Offspring warm-start data staleness | L | L | Declarative Core entries stamped with tick; offspring may override any inherited param after first learning cycle |

