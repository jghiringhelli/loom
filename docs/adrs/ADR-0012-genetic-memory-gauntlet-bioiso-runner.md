# ADR-0012: Genetic Memory, Survival Gauntlet, and BIOISO Runner

**Date**: 2026-04-14
**Status**: Accepted
**Extends**: ADR-0011 (CEMS Runtime Architecture)

---

## Context

ADR-0011 completed the CEMS runtime with the full R1–R13 implementation: Circadian
temporal gating, Epigenome institutional memory, Mycelium colony coordination, and
the Stages 0–3 synthesis pipeline with SHA-256 genome integrity (Stage 0 Membrane),
deterministic Polycephalum rule engine (Stage 1), Ganglion local LLM (Stage 2), and
Mammal Brain remote LLM (Stage 3).

After R8–R13 were complete, three architectural gaps remained that prevent the system
from fulfilling the BIOISO promise at scale:

### Gap 1 — No offspring warm-start

When a parent entity has spent 50–200 ticks learning that parameter `learning_rate`
performs best in the range 0.02–0.06 for its telos, a freshly-spawned offspring
(clone or fork) discards that knowledge entirely. It cold-starts from defaults and
re-discovers the same parameter space, wasting ticks and temporarily degrading its
telos alignment.

The Epigenome Core tier already stores `Declarative` memory entries in a structured
`param=X value=Y` format, specifically to enable this transfer. The mechanism existed;
the hook to invoke it during spawn did not.

### Gap 2 — No pre-promotion hardening

Under the R1–R13 architecture, an entity could progress from Canary to Stable after
the canary observation window passed without regression — provided the normal drift
distribution held during observation. A brittle entity that happened to be promoted
during a calm period would survive to Stable and then fail catastrophically on the
first real stress event.

The Survival Gauntlet concept was present in design discussions but never codified as
a mandatory gate. The result: entities could be fragile and still be promoted to Stable.

### Gap 3 — No domain entity library

Every experiment required manually configuring telos bounds, signal names, and entity
parameters from scratch. There was no reusable library of pre-configured domain entities
for common research and production patterns. The RetroValidator — the mechanism for
scoring CEMS discoveries against academic baselines — was similarly absent, making
it impossible to answer: "Does CEMS discover what the literature already knows?"

---

## Decision

### 1. Genetic Memory — `Epigenome::inherit_from`

Add `Epigenome::inherit_from(parent_id: &EntityId, child_id: &EntityId, now: Timestamp)`
to the Epigenome module.

**What is copied:**

| Memory Tier | Memory Kind | Copied? | Rationale |
|---|---|---|---|
| Core | Semantic | ✓ Yes | Declarative world-knowledge; valid for offspring |
| Core | Procedural | ✓ Yes | Learned strategies; applicable to same-domain offspring |
| Core | Declarative | ✓ Yes | Structured `param=X value=Y` entries; directly warm-start offspring |
| Core | Episodic | ✗ No | Time-stamped events specific to parent's lived history |
| Core | Relational | ✗ No | Parent's peer relationships are not valid for a new identity |
| Security | Audit | ✗ No | Immutable per-entity record; must not be inherited |
| Buffer / Working | Any | ✗ No | Transient / distillation-in-progress; not stable knowledge |

**Copied entries receive prefix:** `[inherited:parent_id]` in the memory text.

**Warm-start parameter parsing:** `warm_start_params(entity_id) -> HashMap<String, f64>`
parses all Declarative Core entries of the form `param=<name> value=<float>` and
returns them as the entity's initial parameter map. Offspring bypass cold-start
and begin evolution from the parent's learned parameter baselines.

**Invocation:** `loom runtime spawn --parent <parent_id> <entity_spec>` triggers
`inherit_from` immediately after genome registration and before the first tick.

### 2. Survival Gauntlet — mandatory pre-promotion hardening

Add `SurvivalGauntlet` as a required gate between Canary and Stable states.
No entity may be promoted to Stable without passing both phases.

**Phase 1 — CAE (Catastrophic Adversarial Event):**
- Drive all monitored signals to maximum telos bounds simultaneously for N ticks
  (default: `GAUNTLET_CAE_TICKS=10`).
- Follow with a recovery window of N ticks at nominal signal levels.
- Pass condition: entity's drift score returns below threshold within the recovery window.
- Failure: entity is rolled back to the pre-canary checkpoint. A `brittle_mutation`
  record is written to the Epigenome Working tier.

**Phase 2 — LTE (Long-Term Exposure):**
- Sustain signals at 2× the normal drift level for M ticks
  (default: `GAUNTLET_LTE_TICKS=20`).
- Pass condition: entity's drift score remains below `GAUNTLET_LTE_MAX_DRIFT` throughout.
- Failure: same rollback and Working tier record as Phase 1.

**Implementation notes:**
- The Gauntlet uses inline bound drift computation. It does not call back to the
  Orchestrator during hardening phases to avoid tick-loop coupling.
- Both phases must pass in sequence. A Phase 2 pass does not compensate for a
  Phase 1 failure.
- Gauntlet parameters are runtime-configurable via environment variables:
  `GAUNTLET_CAE_TICKS`, `GAUNTLET_LTE_TICKS`, `GAUNTLET_LTE_MAX_DRIFT`.

### 3. BIOISO Runner — domain entity library and RetroValidator

Add `BIOISORunner` with 11 pre-configured domain entity specifications covering:

| Domain | Telos Example | Primary Signal |
|---|---|---|
| Climate Modelling | Forecast RMSE < 0.15 | temperature_anomaly |
| Epidemiological Spread | R₀ estimation error < 0.05 | infection_rate |
| Ecological Population | Population variance < 0.20 | carrying_capacity_ratio |
| Financial Risk | VaR breach rate < 0.02 | portfolio_drawdown |
| Supply Chain | Stockout probability < 0.03 | inventory_level |
| Neural Architecture | Validation loss plateau < 5 epochs | val_loss_delta |
| Protein Folding (proxy) | Energy score deviation < 0.10 | folding_energy |
| Power Grid Stability | Frequency deviation < 0.01 Hz | grid_frequency |
| Traffic Flow | Average delay < 45 s | intersection_queue |
| Antibiotic Resistance | Resistance emergence delay > 20 gen | mrsa_generation |
| Forest Carbon Sink | Net sequestration error < 0.08 | carbon_flux |

Each entity spec includes default telos bounds, default parameter values, and
warm-startable initial state sufficient for immediate use without manual configuration.

**RetroValidator:** `RetroValidator::validate(entity_id, historical_episodes)` replays
a set of recorded historical signal episodes through the CEMS pipeline and compares
the mutations proposed (and promoted) by CEMS against the interventions documented
in the corresponding academic literature. Scoring:

- `discovery_score`: fraction of academically-known interventions that CEMS independently
  discovered within the episode window.
- `novelty_score`: fraction of CEMS proposals not in the academic record (potentially new
  findings or false positives — requires expert review).
- `convergence_ticks`: median ticks to first promotion matching the academic intervention.

RetroValidator output is written to the Signal Store and surfaced via `loom runtime log
--retro <entity_id>`.

---

## Alternatives Considered

### Full Lamarckian inheritance (copy ALL memory types including Episodic and Relational)

**Rejected.** Episodic memories are time-stamped records of specific events in the
parent entity's lifetime (`"tick 47: promoted ParameterAdjust learning_rate +0.01"`).
These events are meaningless to an offspring that has not lived through them. Copying
Episodic memories would pollute the offspring's Working and Core distillation pipeline
with stale event data, potentially biasing its evolution away from its own telos context.

Relational memories encode the parent's specific peer relationships. An offspring has
a new identity; inheriting the parent's peer graph would create phantom relationships
to entities that have no corresponding record of the new offspring.

### Full statistical inheritance (copy parameter distributions, not values)

**Deferred.** Instead of copying `param=learning_rate value=0.031`, statistical
inheritance would copy a distribution `param=learning_rate mean=0.031 std=0.008`.
This is richer and more biologically accurate (Mendelian/quantitative genetics) but
requires a prior distribution representation in the Epigenome schema. Deferred to
a future ADR when the Epigenome schema is extended for distribution-valued entries.

### Optional Gauntlet (configurable bypass flag)

**Rejected.** A bypass flag would be used routinely to skip hardening under time
pressure, defeating the purpose of the gate. The CAE and LTE phases are fast
(30 ticks default) and use inline computation with no network calls. There is no
operational justification for bypassing them. The Gauntlet is mandatory.

### Curated entity library as external package (separate crate)

**Rejected for now.** The 11 domain entities are closely coupled to the CEMS runtime
internals (telos bound format, signal naming conventions, epigenome schema). Separating
them into an external crate before the internal API stabilises would create maintenance
overhead. They ship as `src/runtime/bioiso_runner.rs` in the main crate. When the
runtime API is stable (≥ v1.0), extraction to a `loom-bioiso-domains` crate is
appropriate and should be tracked as a future ADR.

---

## Consequences

### What becomes easier

- **Offspring efficiency:** Spawned entities skip ~10–50 ticks of parameter learning
  that the parent already completed. Colony scaling is faster and cheaper.
- **Academic comparison:** CEMS discoveries are now executable against historical
  episodes, not just claimed. `loom runtime log --retro` produces a scored report
  ready for publication appendix inclusion.
- **Production safety:** The mandatory Gauntlet prevents brittle entities from
  reaching Stable state. Catastrophic post-promotion failures caused by stress events
  that never occurred during the canary window are structurally prevented.
- **Experiment bootstrapping:** Researchers can start a domain experiment with one
  command (`loom runtime spawn --entity climate_modelling`) instead of manually
  configuring telos bounds, signal names, and initial parameters.

### What becomes harder or constrained

- **Spawn path is now longer:** `loom runtime spawn --parent` requires the parent's
  Epigenome Core tier to be readable at spawn time. If the Signal Store is unavailable
  at spawn, warm-start falls back to cold-start with a warning — not a hard failure.
- **Canary promotion is slower:** The Gauntlet adds `CAE_TICKS + LTE_TICKS` (default: 30)
  ticks before Stable promotion. For fast-cycling systems (TICK_MS = 100 ms) this is
  3 seconds. For slow-cycling systems (TICK_MS = 60 000 ms) this is 30 minutes. Set
  `GAUNTLET_CAE_TICKS` and `GAUNTLET_LTE_TICKS` to lower values in time-sensitive
  environments.
- **BIOISO Runner entity specs must be maintained:** As the CEMS runtime evolves, the
  11 domain entity specs must be updated to remain valid. Each spec is a test fixture
  as well as a library item — any breaking change to telos bound format or signal
  conventions will surface as a test failure in `BIOISORunner::validate_all_specs()`.

### Required follow-on work

- [ ] Extend Epigenome schema for distribution-valued Declarative entries (statistical
  inheritance — deferred from this ADR).
- [ ] Extract `loom-bioiso-domains` crate when runtime API reaches v1.0.
- [ ] Add `GAUNTLET_*` environment variables to `loom.toml` manifest schema so they
  can be set per-project without relying on OS environment.
