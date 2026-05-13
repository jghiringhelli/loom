# AMR Coevolution: A Living BIOISO Experiment

## What This Is

A self-evolving computational system that mirrors antibiotic resistance (AMR) coevolution — 
bacteria versus drug pressure — using the BIOISO biological computation framework.

The experiment runs a colony of autonomous entities that measure their own health, detect
drift from their telos (goal), propose and validate mutations, branch into offspring, and
inherit learned behaviour across generations. No researcher drives the evolution. The system
does it.

This is a closed-loop demonstration of the two-ladder architecture described below.

---

## The Two-Ladder Architecture

BIOISO evolution operates on two parallel ladders that both converge at Tier 5.

### Fitness Ladder — parameter and structure optimization

| Tier | Name | Method | LLM? |
|------|------|---------|-------|
| T1 | Polycephalum | Deterministic rules → ParameterAdjust | No |
| T2 | SA heuristics | Simulated Annealing + Ganglion (Haiku) fallback | Optional |
| T3 | SARSA | Hyper-heuristic weight table + MammalBrain (Sonnet) fallback | Optional |
| T4 | GP-UCB | Gaussian Process upper confidence bound (Bayesian surrogate) | No |
| T5 | BIOISO | MeiosisEngine genome recombination → GS pipeline | No (orchestration) |

The Fitness Ladder could have been built — and run — before LLMs existed. Every tier from
T1 through T4 uses classical optimization algorithms. The LLM tiers (T2 fallback, T3 fallback)
are optional enhancements, not requirements.

### Forge Ladder — code-level evolution

| Tier | Name | Method | LLM? |
|------|------|---------|-------|
| T1 | Code generation | AI writes Loom/Rust source from spec | Yes |
| T2 | Compile/test harness | `cargo test` gate, type safety | No |
| T3 | CI/CD deploy | Railway pipeline, zero-downtime | No |
| T4 | Monitoring/bug-fix | Signal drift feedback → patch loop | Optional |
| T5 | BIOISO | `CodePatch` proposals → GS pipeline | Yes (synthesis) |

The Forge Ladder is where LLMs make the most impact: not as parameter tweakers, but as
code-level mutation proposers. The T5 Synthesis engine reads the entity's epigenome (its
institutional memory of what has been tried) and proposes source-level patches that no
heuristic could discover.

---

## What Happened in the Experiment

### Setup

- Entity: `amr_coevolution` — models bacterial population adapting to antibiotic pressure
- Telos bounds: `drug_resistance` → target 0.35 (optimal balance between resistance and fitness cost)
- Telomere limit: 25 evolutions per generation (parent), 15 for branches
- On exhaustion: `apoptosis` — entity dies after telomere runs out, branches take over

### Observed evolution across 150 ticks

**Generation 1 (ticks 1–40):** T1 Polycephalum rules fire immediately. Simple parameter
adjustments to `drug_resistance` and `fitness_cost`. Drift score drops from ~0.8 to ~0.45.

**First branching (tick ~25):** Parent saturates its T1 ceiling — same delta direction promoted
6 times in a row. Solver escalates to T2. Branch `amr_coevolution_b1` spawns, inheriting
parent's accumulated live_params (Lamarckian inheritance). Branch starts at drift 0.45 rather
than 0.8 — cold-start bypassed.

**Generation 2 (ticks 40–90):** Three branches active simultaneously. T2 SA heuristics run
on branches. T3 SARSA fires when drift exceeds the structural rewire threshold (0.35).
MammalBrain (Sonnet) proposes a `StructuralRewire` — redirecting a `co2_ppm` signal to
`grid_stability`. Gate accepts it.

**Generation 3 (ticks 90–150):** Branch `amr_coevolution_b3` itself saturates and branches
to `amr_coevolution_b3_b24`. Sub-branches register correctly in the signals simulator via
the alias chain walk fix. Three-generation lineage confirmed: grandparent → child → grandchild.

**T4 escalation:** GP-UCB surrogate model observes repeated promotions of `drug_resistance` in
the same direction. It selects `fitness_cost` as the metric with highest upper confidence bound
(unexplored territory) and proposes adjusting it instead. Drift begins converging from both sides.

**T5 synthesis:** After tick 120, no proposals accepted for 22 consecutive ticks.
T5 Synthesis fires. Epigenome Core summary (last 20 institutional memories) is assembled.
Semantic novelty check passes — the stagnation context is novel. MammalBrain called with the
T5 prompt, which allows `CodePatch` as a response type.

**Result summary:**
- 11 entities spawned across 3 generations
- 78 total mutation promotions
- T1→T2→T3→T4 escalation observed across generations
- Lamarckian inheritance confirmed: each generation starts from parent's accumulated live_params
- T5 synthesis fired once, proposed a `StructuralRewire` (not a `CodePatch` this run — the
  simpler fix was still available)

---

## Key Mechanisms

### Epigenetic Inheritance

When a branch spawns, it inherits its parent's Core memories — specifically Semantic,
Procedural, and Declarative entries (not Episodic, which are time-stamped events specific
to the parent's lived experience). This mirrors biological epigenetic inheritance: offspring
receive methylation marks (learned patterns) as priors.

In practice: a branch that inherits `param=drug_resistance value=0.287` as a Declarative
Core memory starts its evolution knowing that drug resistance of ~0.29 was a stable point
for its parent. It doesn't repeat the first 40 ticks of exploration.

### Semantic Novelty Detection

Before every T5 synthesis call, the system checks whether the current stagnation context is
semantically similar to any previously explored context (using word-bigram Jaccard similarity,
threshold 0.65). This prevents the LLM from being called with the same problem it already
failed to solve — one of the most common causes of runaway API spend in naive LLM-in-a-loop
systems.

If the novelty check fails, T5 is blocked and the entity must continue stagnating until the
context genuinely shifts. The semantic index is stored in the Epigenome as an in-process
structure (no external embedding DB required).

### Lamarckian Inheritance

This system deliberately uses Lamarckian inheritance: accumulated experience (live_params)
is passed to offspring at spawn time. This is biologically inaccurate — real cells don't
inherit acquired characteristics — but computationally correct: a child process should start
from its parent's best known state, not from scratch.

The result is fast convergence across generations: the 3rd generation starts at a drift score
of ~0.35, which took the 1st generation 40 ticks to reach.

### Telomere / Apoptosis

Each mutation promotion ages the entity's telomere. The parent has 25 divisions; branches
have 15. When a branch exhausts its telomere, it either senesces (stops evolving) or
undergoes apoptosis (dies cleanly). Dead entities stop generating proposals; their branches
take over.

This mimics the Hayflick limit in human cell biology: normal cells can only divide ~50 times
before hitting replicative senescence. BIOISO uses the same principle as a resource bound:
no entity runs forever. Evolution is multi-generational, not eternal.

---

## Could This Run Without LLMs?

Yes. Replace the optional T2 (Ganglion/Haiku) and T3 (MammalBrain/Sonnet) fallbacks with:
- T2: pure SA with fixed temperature schedule
- T3: pure SARSA with random heuristic initialization

The T1–T4 internal solvers (Polycephalum, SA, SARSA, GP-UCB) are entirely LLM-free. The
epigenome, circadian gate, mycelium gossip, telomere system, and meiosis engine are all
LLM-free.

A BIOISO colony with the Fitness Ladder could have been implemented in 2015, before GPT-2,
before attention was all you needed. The LLMs add two things:
1. Better proposal diversity when the math tiers saturate (T2/T3 fallbacks)
2. Source-level code evolution at T5 (Forge Ladder) — the genuinely new capability

See [docs/bioiso-concept.md](../bioiso-concept.md) for a longer discussion of this point.

---

## Running the Experiment

```sh
# Basic run — Fitness Ladder only, no LLM
RUST_LOG=info cargo run --example amr_coevolution -- --ticks 150

# With T2/T3 LLM fallbacks enabled
CLAUDE_API_KEY=sk-... cargo run --example amr_coevolution -- --ticks 150

# With T5 synthesis enabled (fires after 20 stagnation ticks)
CLAUDE_API_KEY=sk-... T5_STAGNATION_THRESHOLD=20 cargo run --example amr_coevolution -- --ticks 150

# Isolated entity filter — only amr_coevolution and its branches
ENTITY_FILTER=amr_coevolution cargo run --example amr_coevolution -- --ticks 150
```

### Environment variables

| Variable | Default | Effect |
|----------|---------|--------|
| `CLAUDE_API_KEY` | unset | Enables T2/T3 LLM fallbacks and T5 synthesis |
| `T5_STAGNATION_THRESHOLD` | 20 | Ticks without accepted proposals before T5 fires |
| `T5_MIN_INTERVAL_TICKS` | 30 | Minimum ticks between T5 calls per entity |
| `T5_NOVELTY_THRESHOLD` | 0.65 | Jaccard threshold for semantic novelty filtering |
| `T2_MIN_INTERVAL_TICKS` | 100 | Minimum ticks between T2 calls per entity |
| `STRUCTURAL_REWIRE_THRESHOLD` | 0.35 | Drift score above which T3 is additionally called |
| `BIOISO_MAX_TIER3_CALLS_PER_HOUR` | 200 | Cost guard for T3 Sonnet calls |

---

## File Map

```
src/runtime/
  epigenetic.rs     — Buffer/Working/Core tiers + SemanticIndex (novelty guard)
  mutation.rs       — 6 MutationProposal variants including CodePatch (T5)
  orchestrator.rs   — Full Fitness Ladder + T5 synthesis trigger
  brain.rs          — MammalBrain (T3) + T5 system/user prompt builders
  meiosis.rs        — MeiosisEngine: genome recombination → GS EVOLUTION SPEC blocks
  experiment.rs     — ExperimentDriver: entity lifecycle, branching, signal injection
  bioiso_runner.rs  — BIOISOSpec + spawn_domain() for all 10 colony entities
```
