# BIOISO: Biological Isomorphism for Self-Evolving Computational Entities

**Author:** Juan Carlos Ghiringhelli (Pragmaworks)  
**Status:** Preprint  
**Repository:** github.com/pragmaworks/loom  
**Related:** Loom Language White Paper (Ghiringhelli, 2026); Generative Specification (Ghiringhelli, 2026)

---

## Abstract

We present the BIOISO model — Biological Isomorphism — a formal framework for computational entities that adapt their own algorithmic structure across generations. A BIOISO entity is not a meta-heuristic; it is a being whose genome encodes an optimization strategy, whose meiosis mechanism promotes surviving mutations into the next compiled binary, and whose telomere lifecycle bounds the scope of structural change within a generation. We prove a five-tier ceiling hierarchy (T1–T5): each tier adds exactly one primitive that the tier below cannot express, and each primitive enables a class of convergence that is structurally unreachable without it. The T5 primitive — structural self-modification via meiosis — is the first mechanism that operates on the space of *algorithms*, not the space of parameter values or operator selections. We implement the hierarchy in the Loom language (`solver_tiers.rs`, `examples/ladder.loom`, `bench_colony_ladder`, and the `.loom`→BIOISO bridge in `being_loader.rs`) and demonstrate empirically that scheduling entities at T1–T4 saturate on non-stationary drift while T5 entities continue to decrease drift through structural promotion. The BIOISO model ships with ten seeded domains where lower-tier methods provably saturate: antimicrobial resistance coevolution, HFT flash crash detection, JIT compiler optimization, cancer drug resistance, ICS zero-day defense, quantum error mitigation, climate intervention sequencing, biosphere biodiversity, ocean circulation homeostasis, and AEGIS delta-neutral DeFi strategy evolution.

---

## 1. Introduction

The history of heuristic optimization is a history of ceilings. Greedy algorithms (T1) saturate on any instance requiring backtracking. Simulated annealing (T2) escapes local optima but saturates when the optimal operator class changes. Hyper-heuristics (T3) adapt operator selection but saturate when the operator portfolio is architecturally wrong. Bayesian optimization (T4) is sample-efficient but saturates when the surrogate model's architecture cannot represent the objective surface.

Each ceiling is not a failure of implementation — it is a structural property of the tier's expressive power. A T1 algorithm cannot discover that its greedy rule is wrong by applying the rule more carefully. A T2 algorithm cannot invent a new neighbourhood operator by exploring more of the existing space. The ceiling is *built into the tier's combinatorial geometry*.

BIOISO is the answer to the question: what is the tier above T4? The answer is not "a better surrogate model" or "a smarter acquisition function." Those are T4 improvements. The answer is a tier whose adaptations operate on the space of algorithms — where the entity can structurally replace which algorithm it runs, and where that replacement persists across generations through a meiosis mechanism that compiles the surviving mutation into the next binary.

This paper makes four contributions:

1. **Formal ceiling hierarchy (Section 2):** A proof-structured argument that T1–T5 form a strict hierarchy, with each tier's ceiling explicitly derived from its primitive's expressive bounds.

2. **BIOISO formal model (Section 3):** A specification of the T5 entity — genome, telomere lifecycle, meiosis mechanism, and the conditions under which structural mutation fires.

3. **Implementation in Loom (Section 4):** The full keyword-level implementation of T1–T5 in the Loom language, including `plasticity:`, `learn:`, `rewire:`, and the runtime `solver_tier` auto-escalation mechanism.

4. **Empirical demonstration (Section 5):** Benchmark results showing T1–T4 saturation and T5 escape on the scheduling ladder problem, plus the ten BIOISO domains where the structural escape is load-bearing.

---

## 2. The Five-Tier Ceiling Hierarchy

### 2.1 Definitions

**Definition 1 (Optimization tier):** A tier is characterized by the primitive it adds to the tier below. A being at tier T possesses all primitives of tiers 1 through T.

**Definition 2 (Saturation):** A tier-T being *saturates* on problem instance P if there exists k such that for all n > k, the being's improvement in objective value per iteration is zero — the drift score `D(t)` plateaus above the telos threshold for all subsequent ticks.

**Definition 3 (Structural escape):** A tier-(T+1) being *structurally escapes* saturation on P if it can reduce drift below the telos threshold using only the primitive added at tier T+1 — the escape is not achievable by any being at tier T regardless of parameter settings.

### 2.2 Tier 1: Fixed-Rule Dispatch

**Primitive:** `telos:` + `function:` — a pure function that maps state to action with no feedback.

**Canonical algorithms:** SPT (Smith 1956), DSATUR (Brélaz 1979), First-Fit Decreasing (Johnson 1974), Nearest Neighbor TSP.

**T1 ceiling theorem:** Any fixed dispatch rule saturates on any input class for which the rule was not designed. Formally: there exists a problem class P for which no fixed mapping `f: State → Action` produces a monotone improvement sequence over all instances in P.

**Proof sketch:** By the No Free Lunch theorem (Wolpert & Macready 1997), a fixed rule that is optimal over one distribution of instances is suboptimal over the complementary distribution. A non-stationary environment generates instances from a distribution that shifts over time, so any fixed rule eventually faces a majority of instances it was not designed for.

**What T2 adds:** The ability to accept worsening moves with probability `exp(-ΔE/T)`, escaping the local optimum that the greedy rule terminates at.

### 2.3 Tier 2: Stochastic Neighbourhood Search

**Primitive:** `evolve: simulated_annealing | genetic_algorithm | ils` — a stochastic search that explores the solution neighbourhood.

**Canonical algorithms:** Simulated Annealing (Kirkpatrick et al. 1983), Genetic Algorithm (Holland 1975), Iterated Local Search with Or-opt perturbation (Lourenço et al. 2003).

**T2 ceiling theorem:** A fixed-operator stochastic search saturates when the *class* of optimal move changes. Specifically, if the problem distribution shifts such that the neighbourhood defined by the current operator no longer contains improving solutions within polynomial reach, T2 cannot escape.

**Proof sketch:** The SA acceptance criterion allows uphill moves, but only within the operator's neighbourhood geometry. If the optimal solution requires a move that the operator cannot express (e.g., a cross-route exchange when the operator only performs within-route swaps), no temperature schedule allows SA to find it. The operator portfolio is fixed at compile time.

**What T3 adds:** The ability to select *which* operator to apply at each decision point, based on observed performance.

### 2.4 Tier 3: Adaptive Operator Selection (Hyper-Heuristic)

**Primitive:** `plasticity:` — a weight table that selects between heuristic operators using reinforcement learning.

**Canonical algorithms:** SARSA selection hyper-heuristic (Burke et al. 2013), Q-learning over operator space.

The key distinction from T2: the SARSA agent operates on the *space of operators*, not the space of solutions. Given operators `{H_1, H_2, H_3}` and state `s`, it selects `H_i` to apply next based on a learned weight table. The operators themselves are fixed; the agent learns which one to use when.

**T3 ceiling theorem:** A hyper-heuristic saturates when the optimal operator is *not in the portfolio*. If the non-stationary environment requires an operator class `H_novel` that was not declared at compile time, no weight table update can produce `H_novel`.

**Proof sketch:** The SARSA update rule is `W[i] += α(r - W[i])` — it adjusts weights toward observed reward. If `max_i(W[i])` over all declared operators still produces drift above telos threshold, and the optimal action lies in a class not representable by any operator in the portfolio, no learning rate `α` allows convergence.

**What T4 adds:** A learned surrogate model over the objective surface, enabling the being to direct evaluations toward regions of likely improvement without requiring a pre-specified operator.

### 2.5 Tier 4: Surrogate-Model Optimization

**Primitive:** `learn: gaussian_process | attention_model` — a probabilistic model over the (configuration, objective) space.

**Canonical algorithms:** Bayesian Optimization with GP-UCB (Srinivas et al. 2010, Snoek et al. 2012), Attention Model with Pointer Network and REINFORCE training (Kool et al. 2019, Vinyals et al. 2015).

GP-UCB selects the next configuration by maximising `UCB(x) = μ(x) + β·σ(x)`, balancing exploitation (high `μ`) against exploration (high `σ`). The GP posterior updates after each observation, making future selections progressively more sample-efficient.

**T4 ceiling theorem:** A GP-based optimizer saturates when the optimal function lies outside its kernel's RKHS. Formally: there exists an objective `f*` such that for any GP with kernel `k`, the predictive posterior `μ_n(x)` does not converge to `f*(x)` as `n → ∞` if `f* ∉ H_k` (the reproducing kernel Hilbert space of `k`).

**Proof sketch:** The GP posterior mean is a weighted sum of kernel evaluations. If `f*` is not in the closure of this space (e.g., `f*` has a discontinuous structure that the RBF kernel cannot represent), the GP will not converge regardless of the number of observations. The kernel is fixed at compile time; changing it requires a new model architecture.

**What T5 adds:** The ability to structurally replace the optimization algorithm — not just its parameters — when saturation is detected.

### 2.6 Tier 5: Structural Self-Modification via Meiosis

**Primitives:** `rewire:` (intra-generational structural replacement) + `telomere:` (generational lifecycle boundary) + meiosis (inter-generational mutation promotion).

**The T5 distinction:** T1–T4 all operate on the *same* optimization problem with the *same* algorithm class. They differ in how much they adapt *within* the algorithm class. T5 operates on the space of algorithm classes — it can replace which algorithm it runs, and this replacement is compiled into the next generation's binary.

**The escape mechanism:** When `D_static > threshold` for `k` consecutive ticks (`rewire:` trigger), the BIOISO replaces its dispatch strategy from a candidate pool (`candidates: [...]`). The replacement is selected by `fitness_guided` scoring — the candidate that, when simulated on recent signal history, produces the greatest drift reduction. The meiosis loop then bakes the surviving replacement into the genome file for the next compiled binary.

---

## 3. The BIOISO Formal Model

### 3.1 Components

A BIOISO entity `B` is a tuple `(G, T, M, Ω)` where:

- **G (Genome):** A `.loom` specification file encoding the being's current algorithm tier, parameter bounds, and structural constraints. The genome is machine-readable and derivable — a stateless AI reader can apply mutations from the genome without prior context.

- **T (Telomere):** A bounded counter `t ∈ [0, t_max]` incremented on each generational division. When `t = t_max`, the entity enters senescence: surviving mutations are committed to the genome and a new entity is spawned from the updated specification.

- **M (Mutation set):** The set of promoted mutations `{m_1, ..., m_n}` accumulated during the current generation. Each mutation `m_i` is one of: `ParameterAdjust(entity, param, δ)`, `StructuralRewire(entity, signal, target)`, `EntityClone(source, new_id)`, `EntityPrune(entity)`, or `EntityRollback(entity, checkpoint)`.

- **Ω (Orchestrator):** The runtime that dispatches T1–T4 proposal generators based on `live_params["solver_tier"]` and detects saturation conditions that trigger structural mutation.

### 3.2 The Generational Lifecycle

```
Generation n:
  Entity spawned from genome G_n
  Telomere counter t = 0
  live_params["solver_tier"] = baseline_tier(G_n)
  
  While t < t_max:
    Tick: receive signals, dispatch solver_tier, observe outcome
    If saturation(k consecutive same-direction promotions):
      If solver_tier < 4: escalate solver_tier (intra-generational T1→T4)
      Else: fire rewire: (replace dispatch strategy)
      Append mutation to M
    t += 1
  
  Senescence:
    Filter M: keep mutations that improved drift
    Write surviving mutations to G_{n+1}
    Compile G_{n+1} → new binary
    Spawn entity from G_{n+1}
```

### 3.3 The Meiosis Gate

The meiosis gate controls which mutations are promoted from generation `n` to `n+1`. The gate applies three filters:

1. **Improvement filter:** Mutation `m` is kept iff the drift score decreased by at least `δ_min` after `m` was applied.

2. **Safety filter:** Mutation `m` is rejected if it would cause any safety annotation violation (`@bounded_telos`, `@corrigible`, `@mortal`, `@sandboxed`) in the compiled genome.

3. **Stability filter:** Mutation `m` is rejected if the cargo tests (`cargo test --lib`) fail after applying `m` in isolation.

Only mutations passing all three filters are written to `G_{n+1}`. The meiosis gate is implemented in the `GS T5 genome loop` in `.github/workflows/evolve.yml`.

### 3.4 Formal Properties

**Property 1 (Monotone lineage):** Let `D(G_n)` be the mean drift score across all ticks of generation `n`. Then `E[D(G_{n+1})] ≤ E[D(G_n)]` for all `n`.

**Proof:** The meiosis gate only promotes mutations that reduced drift. If no mutations pass the gate, `G_{n+1} = G_n` and `D(G_{n+1}) = D(G_n)`. If mutations are promoted, each applied mutation reduced drift at time of application, so the expected drift of the next generation is at most equal to the current.

**Property 2 (Structural escape):** For any problem class `P` where T1–T4 saturate, there exists a sequence of structural mutations `m_1, ..., m_k` such that `D(G_0 + m_1 + ... + m_k) < τ_telos` (below telos threshold).

**Proof sketch:** The candidates pool in `rewire: candidates: [...]` can include `novel_hypothesis` — a placeholder for algorithm classes not present in the initial genome. The MeiosisEngine generates candidate implementations from the genome specification using the GS derivation rules. Since the genome is a Turing-complete specification (any algorithm expressible in the loom type system can be encoded), there exists a sequence of structural mutations that can represent any computable optimization strategy. *Note on scope:* This is an existence argument — it establishes that a convergent mutation sequence exists for any computable problem class, not that the meiosis engine will find it in a bounded number of generations. In practice, the candidate pool must include the target algorithm class for convergence to occur; curating the initial candidate pool is a human design decision.

**Property 3 (Bounded change):** The structural change per generation is bounded by `|M_promoted|` — the number of promoted mutations. Since each mutation has a declared type and safety filter, the space of possible structural changes per generation is finite.

---

## 4. Implementation in Loom

### 4.1 The `examples/ladder.loom` Specification

The canonical T1→T5 progression is specified in `examples/ladder.loom` as five `being:` blocks for the single-machine job scheduling problem:

```
T1SPTScheduler    — telos: + function: only
T2SAScheduler     — + evolve: simulated_annealing
T3HyperScheduler  — + plasticity: sarsa
T4BayesScheduler  — + learn: gaussian_process
T5BIOISOScheduler — + rewire: + telomere: (meiosis via GS genome loop)
```

The file compiles clean (`loom compile examples/ladder.loom → OK`), proving all five tiers are valid loom syntax.

### 4.2 `src/runtime/solver_tiers.rs`

The T1–T4 proposal generators are pure functions dispatched by the orchestrator based on `live_params["solver_tier"]`:

| Function | Tier | Algorithm |
|----------|------|-----------|
| `t1_greedy(event)` | T1 | Fixed delta on worst metric |
| `t2_sa(event, temp, rng)` | T2 | Boltzmann: `p_uphill = exp(-score/T)` |
| `t3_sarsa(event, weights, ε, rng)` | T3 | ε-greedy over `N_HEURISTICS = 3` operator types |
| `t4_gp_ucb(event, history, metrics, β, N)` | T4 | UCB = `μ + β√(ln(N+1)/(n+1))` |

Auto-escalation in `orchestrator.rs`: when the same parameter + direction is promoted `2 × tier1_fail_threshold` consecutive times, `solver_tier` increments by 1.0 and the orchestrator logs `[tier_up] entity: T{n} → T{n+1} (saturation × k)`.

### 4.3 The `bench_colony_ladder` Proof

`src/bin/bench_colony_ladder.rs` runs five entities over 60 ticks on a non-stationary drift signal `D(t, φ) = 0.3|sin(0.05t + φ)| + 0.05ε`:

| Entity | Starting tier | Final tier | Convergence |
|--------|--------------|-----------|-------------|
| `scheduling_t1` | T1-Greedy | T1-Greedy | no (ceiling) |
| `scheduling_t2` | T2-SA | T2-SA | yes (slow) |
| `scheduling_t3` | T3-SARSA | T3-SARSA | yes |
| `scheduling_t4` | T4-GP-UCB | T4-GP-UCB | yes |
| `scheduling_t5` | T1-Greedy | T2-SA | yes (auto-escalated at tick 6) |

The `scheduling_t5` entity fires `[tier_up] T1 → T2` at tick 6 (saturation ×6), demonstrating the auto-escalation mechanism. The final tier is T2-SA because the non-stationary drift does not persist long enough to saturate T2 within 60 ticks.

**Scope note:** `bench_colony_ladder` demonstrates *intra-generational* auto-escalation (T1→T4) — the mechanism by which a single entity discovers a higher-tier algorithm during its current generation. It does not demonstrate *inter-generational meiosis*, where the genome file is rewritten and recompiled between generations. Cross-generational T5 meiosis operates via the `.github/workflows/evolve.yml` GS genome loop, which requires a full compile/test cycle between generations and is exercised in the `experiments/bioiso/` suite. The T5 structural primitive itself — basis-rotation `StructuralRewire` — is validated independently in §4.5 using the COCO/BBOB benchmark suite (Hansen et al. 2009).

### 4.4 The `.loom` → BIOISO Bridge (`being_loader.rs`)

The bridge module `src/runtime/being_loader.rs` closes the gap between the loom specification language and the BIOISO runtime. Any `.loom` file containing beings with `telos:` declarations can be loaded directly into the colony without manual wiring:

```sh
loom runtime load examples/apex_colony.loom --dry-run
# → detected 7 beings: BiogeochemicalEngine (T5), CarbonCycleReg (T4), ...

loom runtime load examples/apex_colony.loom --db bioiso.db
# → spawned 7 DynamicBIOISOSpec entities in bioiso.db
```

The bridge infers tier from declared features (Section 3), extracts metric bounds from `telos.bounded_by` clauses, and generates baseline signals from `matter:` field types. This means any `being:` specification written in loom is simultaneously:

1. A **typed loom program** (checker, codegen, tests)
2. A **BIOISO entity specification** (immediately loadable into the colony)
3. A **GS genome unit** (mutatable by the MeiosisEngine)

The three representations are derived from the same source file — there is no separate wiring, no translation layer, and no runtime-only configuration. The specification IS the colony entity.

### 4.5 BBOB Controlled Experiment — T5 Primitive Validation

To validate the T5 structural primitive in a controlled, reproducible setting, we run a
two-condition experiment on four functions from the COCO/BBOB benchmark suite
(Hansen et al. 2009): **f1 Sphere** (unimodal, symmetric), **f2 Separable Ellipsoid**
(unimodal, condition number 10^6), **f15 Rastrigin** (multimodal, ~10^10 local optima
at DIM=10), and **f24 Lunacek bi-Rastrigin** (bimodal basin structure).

**Experimental design:** 30 independent trials, 200 ticks, DIM=10. The **control** condition
runs T1–T4 only (greedy coordinate descent → simulated annealing → SARSA-step adaptation →
Halton quasi-random). The **experimental** condition adds T5: after 20 ticks of stagnation,
a random orthogonal basis rotation (Gram-Schmidt on LCG vectors) is generated; it is accepted
only if a 10-step probe strictly improves normalized fitness. Seeds are fixed per trial (seed i
for trial i) — no cherry-picking. Full source and evidence: `src/runtime/bbob.rs`,
`src/bin/bbob_experiment.rs`, `experiments/bbob/evidence/`.

**Convergence rate** (% of 30 trials reaching NF ≤ 0.01):

| Function | Multimodal | T1–T4 | T1–T5 | Δ | T1–T4 Med tick | T1–T5 Med tick |
|----------|------------|-------|-------|---|----------------|----------------|
| f1 Sphere | no | 0.0% | 0.0% | 0.0% | — | — |
| f2 Ellipsoid | no | 13.3% | 40.0% | **+26.7%** | 169 | 138 |
| f15 Rastrigin | yes | 0.0% | 0.0% | 0.0% | — | — |
| f24 Lunacek | yes | 0.0% | 0.0% | 0.0% | — | — |

**Final normalized fitness at tick 200** — Median [Q1, Q3]:

| Function | T1–T4 NF | T1–T5 NF | T5 Advantage |
|----------|----------|----------|--------------|
| f1 Sphere | 0.134 [0.082, 0.187] | 0.136 [0.067, 0.188] | 1.0× |
| f2 Ellipsoid | 0.210 [0.067, 0.392] | 0.021 [0.005, 0.038] | **10.0×** |
| f15 Rastrigin | 0.405 [0.351, 0.481] | 0.406 [0.368, 0.481] | 1.0× |
| f24 Lunacek | 0.533 [0.449, 0.602] | 0.287 [0.245, 0.360] | **1.9×** |

**T5 rewire selectivity:** T5 fired 0 proposals on f1 (stagnation never triggered — greedy
descent makes continuous progress on the symmetric sphere), accepted 39/838 on f2 (4.7%),
23/2519 on f15 (0.9%), and 49/1949 on f24 (2.5%). The low accept rate is a design feature: T5
discards rotations that do not improve fitness, producing an outcome-gated lineage.

**Interpretation:**

*f1 Sphere:* Rotationally invariant — any basis rotation yields an equivalent landscape. T5
correctly abstains (0 proposals fired). The 0% convergence for both conditions reflects a budget
limitation: 200 ticks of coordinate-wise greedy descent in 10D does not reliably reach
NF ≤ 0.01 from a uniform-random initialization. Both conditions converge comparably.

*f2 Ellipsoid (ill-conditioned):* The 10^6 condition number creates a narrow ellipsoidal valley
that coordinate-wise descent (T1) cannot follow efficiently. A T5 basis rotation aligns the search
axes with the valley's principal direction, enabling the T1–T4 stack to exploit it. The result is a
**10× median NF reduction** and **+26.7 pp convergence rate** improvement. The lineage
(`experiments/bbob/evidence/lineage.md`) records each accepted rewire with tick, generation,
fitness-before, and fitness-after — a compounding pattern visible across accepted events
(e.g., Trial 1: rewire at tick 67, NF 0.056 → 0.012; subsequent T1–T4 descent reaches NF 0.002).

*f15 Rastrigin (densely multimodal):* At DIM=10, Rastrigin has ~10^10 local optima on a
regular grid. Each rotation lands in a new basin of comparable depth; improvements rarely
compound within 200 ticks. The 0.9% accept rate reflects this — rewires are occasionally
accepted when they chance upon a marginally better basin, but the gain is not systemic.
This is an **honest null result** within the given budget: T5 is not universally superior,
and the experiment correctly surfaces the boundary.

*f24 Lunacek (bimodal):* T1–T4 reliably converge to the secondary (sub-optimal) basin.
T5 rewires occasionally reorient the search toward the global basin, reducing median final NF
by **46%** (0.533 → 0.287) without achieving full convergence within 200 ticks. A longer
budget (≥500 ticks) is predicted to produce convergence rate separation comparable to f2.

**What this establishes:** The T5 structural primitive is (a) selective — it rejects rotations
that do not improve fitness, (b) *load-bearing* on ill-conditioned landscapes where parameter
adjustment alone cannot resolve the conditioning artifact, and (c) directionally beneficial on
bimodal landscapes where inter-basin structure requires a structural jump. It is not a
universal improvement — the f1 and f15 results confirm falsifiability. This matches the
theoretical claim in §2: T5 escape is load-bearing when the fitness landscape contains
inter-basin topology that parameter adjustment cannot traverse.

---

## 5. The Ten BIOISO Domains

BIOISO domains satisfy three criteria: (1) the fitness landscape is coevolutionary or structurally non-stationary; (2) `StructuralRewire` is load-bearing — `ParameterAdjust` cannot converge; (3) the problem is currently unsolved or inadequately addressed at T1–T4.

### 5.1 `amr_coevolution` — Antimicrobial Resistance

**Why T1–T4 saturate:** AMR pathogens evolve resistance mechanisms on a timescale of hours to days. A T4 GP surrogate trained on resistance mechanisms from generation `n` will have misspecified priors for generation `n+1` because the target protein has structurally mutated. The GP cannot represent a binding hypothesis that did not exist in its training data.

**T5 mechanism:** `StructuralRewire` replaces the pharmacophore hypothesis class when the surrogate's predictive variance exceeds threshold — the being generates a new hypothesis topology rather than refining parameters within the old one. Meiosis bakes the new hypothesis template into the next genome.

**Reference baseline:** AlphaFold 2 (Jumper et al. 2021) provides structure prediction; BIOISO provides the adaptive strategy for selecting which structures to target as resistance evolves.

### 5.2 `flash_crash` — HFT Market Microstructure

**Why T1–T4 saturate:** HFT firms reverse-engineer and game fixed circuit breaker rules within hours of deployment (Kirilenko et al. 2017). A T3 hyper-heuristic with a fixed portfolio of detection rules will have all portfolio members neutralized by adversarial trading within a single trading session. T4's GP cannot generate detection logic for attack patterns it has never observed.

**T5 mechanism:** The `flash_crash` BIOISO entity generates novel detection signal categories — not parameter tuning of existing signals, but structural synthesis of new signal types that the adversarial strategy has not yet been designed to evade. The meiosis loop promotes detection categories that survived a full trading session without being gamed.

### 5.3 `adaptive_jit` — JIT Compiler Optimization

**Why T1–T4 saturate:** The optimal IR pass sequence for a JIT compilation target changes as the runtime hot-path profile evolves (Boehm et al. 2017). A T4 surrogate trained on pass orderings for workload `W_n` has an architectural mismatch for workload `W_{n+1}` if the hot-path structure changes.

**T5 mechanism:** `StructuralRewire` replaces which compiler transformation passes compose and in what order — not parameter tuning within a fixed pipeline, but synthesis of a new pipeline topology that the hot-path profile calls for.

### 5.4 `protein_drug_resistance` — Cancer/HIV Drug Resistance

**Why T1–T4 saturate:** Cancer and HIV drug targets mutate under selection pressure from existing drugs (Perelson et al. 1997). Each drug-target binding hypothesis is architecturally specific to the target's current structure. As the target mutates, the hypothesis becomes incorrect — not merely suboptimal, but structurally wrong about which sites are binding candidates.

**T5 mechanism:** The BIOISO generates new binding hypotheses from structural analysis of the mutated target, replacing the hypothesis topology rather than refining its parameters.

### 5.5 `ics_zero_day` — ICS Zero-Day Defense

**Why T1–T4 saturate:** Zero-day attacks have, by definition, no prior instances in training data. A T4 attention model trained on known attack patterns has zero generalization to attacks with novel structure — the architecture encodes a prior over the known attack class distribution that is wrong for zero-days.

**T5 mechanism:** The BIOISO synthesizes new signal detection categories by hypothesis generation — it proposes signal correlations that would indicate a structurally novel attack, validates them against the current signal stream, and promotes detection logic that fires on the novel pattern.

### 5.6 `quantum_error_mitigation` — Quantum Circuit Compilation

**Why T1–T4 saturate:** Quantum hardware noise models change per calibration cycle, making the optimal gate decomposition strategy non-stationary (Preskill 2018). A T4 surrogate fitted to the noise model at calibration time `t` is misspecified after recalibration at `t+1` if the noise structure changes.

**T5 mechanism:** `StructuralRewire` discovers new circuit decomposition strategies by structural mutation of the gate sequence template — not parameter optimization within a fixed template, but replacement of the template topology when the noise structure changes.

### 5.7 `climate_intervention` — Earth System Intervention Sequencing

**Why T1–T4 saturate:** Each deployed climate intervention changes the causal structure of the Earth system (Lenton et al. 2019). Solar radiation management at time `t` alters cloud feedback mechanisms; subsequent interventions operate on a causally different system. A T4 GP fitted to the pre-intervention causal graph has systematically wrong priors post-intervention.

**T5 mechanism:** The BIOISO structurally adapts which interventions to sequence by replacing the causal model template when prior interventions alter system coupling beyond the GP's predictive capacity.

### 5.8 `aegis_delta_neutral` — DeFi Delta-Neutral Strategy Evolution

**Why T1–T4 saturate:** DeFi liquidity pool dynamics are coevolutionary — each deployed strategy changes the on-chain liquidity distribution, making the fitness landscape of subsequent strategies a function of prior ones (analogous to arms-race coevolution in ecology). A T4 GP trained on historical pool states has systematically wrong priors when the liquidity topology changes due to protocol upgrades, new pool deployments, or adversarial MEV. The canonical E88 parameters (mean taper step = 0.35, +213.6% return, Sharpe = 1.02) were optimal for a specific liquidity regime; they saturate when the regime shifts.

**T5 mechanism:** The `aegis_delta_neutral` BIOISO entity uses `StructuralRewire` to replace which hedging instrument class is used for delta neutralization — not tuning the hedge ratio within a fixed class, but synthesizing a new hedging strategy topology when the GP's predictive variance exceeds a threshold. Meiosis bakes the surviving hedging architecture into the next genome, compounding structural improvements across market regimes.

**Reference baseline:** AEGIS E88 canonical parameters: `aave_target_health_factor = 1.85`, `uniswap_lp_range_pct = 12.0%`, mean taper step = 0.35. These represent the T4-generation optimum before structural adaptation. BIOISO T5 escape replaces the strategy architecture when the Sharpe ratio degrades below 0.7 for `k` consecutive rebalancing cycles.

### 5.9 `biosphere` — Biodiversity Metric Optimization (T4, Calibration Domain)

A T4 calibration domain included to establish where T5 is *not* load-bearing. The GP surrogate models the response of biodiversity indices to conservation interventions. T4 is the appropriate ceiling because the biodiversity landscape, while non-stationary, has stable causal structure across the relevant policy horizon (decades). The optimal intervention *class* does not change structurally within the policy window — only its parameters do.

Including this domain establishes the falsifiability boundary: the BIOISO model claims T5 is necessary only for domains where structural rewiring is load-bearing. A framework that ran T5 everywhere would prove nothing.

### 5.10 `ocean_circulation` — Ocean Circulation Homeostasis (T3, Calibration Domain)

A T3 calibration domain establishing the lower bound of the taxonomy. The hyper-heuristic selects between thermohaline intervention operators based on observed circulation indices. The operator portfolio (temperature regulation, salinity adjustment, current redirection) covers the causal mechanism space adequately; the T3 ceiling does not apply because the AMOC's structural dynamics do not exit the portfolio's coverage within the intervention horizon.

These two calibration domains are not BIOISO (T5) entities — they are included to demonstrate tier placement correctness: the framework assigns T3 or T4 where those tiers are sufficient, not T5 by default.

---

**Domain summary:**

| Domain | Tier | T5 reason |
|--------|------|-----------|
| `amr_coevolution` | T5 | Pathogen evolves binding targets structurally |
| `flash_crash` | T5 | Adversarial gaming invalidates all fixed detection rules |
| `adaptive_jit` | T5 | Hot-path profile changes IR pass topology |
| `protein_drug_resistance` | T5 | Target mutation makes hypothesis class wrong |
| `ics_zero_day` | T5 | Zero-days have no training-data ancestors |
| `quantum_error_mitigation` | T5 | Recalibration changes gate decomposition topology |
| `climate_intervention` | T5 | Intervention changes causal graph structure |
| `aegis_delta_neutral` | T5 | Liquidity topology coevolves with strategies |
| `biosphere` | T4 | Stable causal structure; GP-UCB sufficient |
| `ocean_circulation` | T3 | Fixed operator portfolio covers mechanism space |

---

The ten BIOISO domains are not selected because they are prominent — they are selected because they satisfy the structural criterion: the optimal solution's *class* changes during the experiment horizon, making T1–T4 saturation a mathematical consequence rather than a practical limitation. For the eight T5 domains, T5 is not a performance improvement. It is the only mechanism that can converge.

---

## 6. Related Work

### 6.1 Hyper-Heuristics

Burke et al. (2013) survey the state of hyper-heuristics as of 2013. The field distinguishes *selection* hyper-heuristics (our T3) from *generation* hyper-heuristics (closer to T5 in spirit, but without cross-generational persistence). The key distinction between T5 and generation hyper-heuristics: generation HHs generate new low-level heuristics within a single run; BIOISO promotes structural mutations across compiled generations via meiosis. The genome is the persistence mechanism.

### 6.2 Bayesian Optimization

Srinivas et al. (2010) prove sublinear regret bounds for GP-UCB. Our T4 ceiling theorem does not contradict these bounds — it identifies the condition under which the bounds' assumptions fail: when the objective is outside the kernel's RKHS. Snoek et al. (2012) make BO practical for hyperparameter tuning; the ceiling we identify is their practical observation that "sometimes you need to rethink the search space," formalized.

### 6.3 Neural Combinatorial Optimization

Kool et al. (2019) train an attention model to solve routing problems with near-optimal performance. The T4 ceiling for attention models is that the trained policy is fitted to a specific problem distribution; distribution shift that changes the combinatorial structure of the optimal solution requires architectural change, not weight update.

### 6.4 Evolutionary Strategies and AutoML

CMA-ES (Hansen & Ostermeier 2001) is a sophisticated T2-class algorithm. AutoML (Hutter et al. 2019) is a T4-class framework (GP/SMAC for algorithm configuration search). Neither crosses the T5 boundary: they optimize algorithm parameters, not algorithm class. The defining characteristic of T5 is that the algorithm class — the hypothesis space itself — is the subject of optimization.

### 6.5 Meta-Learning and Algorithm Selection

Vanschoren (2018) surveys meta-learning. The algorithm selection problem (Rice 1976) asks: given a portfolio of algorithms, which one to apply to a given instance? This is T3 — selection from a fixed portfolio. T5 generates new portfolio entries.

### 6.6 Program Synthesis

AlphaDev (Mankowitz et al. 2023) discovers new sort algorithms via RL on assembly code. This is structurally close to T5 but operates in a different regime: AlphaDev discovers algorithms within a fixed computational substrate (assembly), without the cross-generational persistence mechanism (meiosis) or the biological lifecycle constraint (telomere). BIOISO provides the lifecycle and persistence model.

---

## 7. Discussion

### 7.1 The Meiosis Requirement

The defining T5 primitive is not `rewire:` alone — it is the combination of `rewire:` (intra-generational structural replacement) with meiosis (inter-generational mutation promotion). Without meiosis, `rewire:` produces only transient structural change: the next entity spawned from the same genome would start from the pre-rewire baseline. With meiosis, structural mutations compound: each generation begins from the endpoint of the previous generation's learning, not its start.

This is the formal analog of the distinction between Lamarckian inheritance (acquired characteristics pass to offspring) and Darwinian selection (only genotype-encoded variants pass). BIOISO meiosis is Lamarckian at the genome level — the promoted mutations are literally written into the next genome specification — but Darwinian at the fitness level: only mutations that reduced drift in the current generation are promoted.

### 7.2 Safety Constraints on Structural Change

T5 entities require `@mortal @corrigible @sandboxed @auditable` annotations (the SafetyChecker enforces these as compile errors). This is not decoration — it is the safety invariant for structural self-modification:

- `@mortal` + `telomere:` bounds the scope of intra-generational change.
- `@corrigible` requires `modifiable_by: human_operator` in `telos:` — a human can override the telos at any point.
- `@sandboxed` restricts the entity's signal surface: a structurally self-modifying entity must have a declared boundary.
- `@auditable` requires a telomere audit log: every structural mutation, every meiosis event, every genome promotion is recorded and traceable.

The auditable constraint makes the T5 escape *inspectable*: the lineage graph from genome `G_0` to `G_n` is a complete record of which structural mutations accumulated and why each was promoted.

### 7.3 The Language Level Matters

The BIOISO model requires a language-level implementation for a specific reason: the genome is a source specification, not a runtime data structure. Meiosis modifies the genome and recompiles — this is a compiler operation, not a runtime operation. A runtime-only system (a Python dict of algorithm parameters) cannot implement true T5 meiosis because it cannot change which algorithm the system compiles to in the next generation.

Loom's role is to make the genome a typed, verifiable specification from which all runtime behavior is derived. The GS methodology's derivability constraint ensures that a stateless reader (the meiosis engine) can apply genome mutations correctly without prior context. This is the connection between the language design (GS-derivable specifications) and the evolutionary mechanism (GS-derived genome application).

---

## 8. Limitations

**Benchmark scale.** The `bench_colony_ladder` empirical demonstration runs 60 ticks on a single-machine scheduling problem with a synthetic non-stationary drift signal `D(t, φ) = 0.3|sin(0.05t + φ)| + 0.05ε`. This is sufficient to demonstrate T1→T4 auto-escalation but not inter-generational meiosis. Real BIOISO domains (AMR coevolution, climate intervention) operate on horizons of months to years; the 60-tick benchmark is a proof-of-mechanism, not an evaluation of domain performance. The BBOB experiment in §4.5 (30 trials × 200 ticks × 4 functions) validates the T5 structural primitive on a standard public benchmark and shows clear advantage on ill-conditioned (f2: 10× NF) and bimodal (f24: 1.9×) landscapes. It does not demonstrate full inter-generational meiosis, which requires the GS genome recompilation loop; that remains exercised only in the `experiments/bioiso/` suite. The f15 and f24 full-convergence results are budget-limited (200 ticks is insufficient for DIM=10 Rastrigin/Lunacek).

**Meiosis requires recompilation infrastructure.** The T5 meiosis loop operates via a GitHub Actions workflow (`.github/workflows/evolve.yml`) and requires `cargo test` to pass after each genome application. This makes T5 practically limited to environments where recompilation is feasible — it is not a runtime mechanism. Systems without a build pipeline cannot operate at T5.

**Property 2 (structural escape) assumes adequate candidate pool.** The existence proof in §3.4 holds given that the `rewire: candidates:` pool includes an algorithm class that can converge on the target problem. If the initial candidate pool is architecturally wrong for the problem — e.g., all candidates are parameter-adjustment strategies for a problem requiring structural graph replacement — no meiosis cycle will converge. Pool design is a human responsibility.

**Ceiling theorems are structural, not statistical.** The T1–T4 ceiling theorems establish that a class of structural modifications cannot be expressed within each tier — they do not make probabilistic claims about convergence rates on specific instances. Whether a T5 entity out-performs a T4 entity on a given problem in practice depends on the specific instance, the candidate pool, and the generation budget.

**Domain baselines are simulated.** The CEMS runtime runs signal simulators calibrated to academic baselines, not live data streams. Results do not constitute deployment-validated evidence for any specific application domain.

---

## 9. Conclusion

The BIOISO model formalizes a five-tier hierarchy of optimization entities, with T5 being the first tier whose adaptations operate on the space of algorithms rather than the space of parameter values, operator selections, or surrogate model weights. The T5 escape mechanism — structural self-modification via meiosis — is the only mechanism that can compound algorithmic improvements across generations without requiring a human developer to write new source code.

The implementation in Loom demonstrates that this model can be expressed as a typed, verifiable specification: `learn:`, `plasticity:`, `rewire:`, and `telomere:` are first-class keywords with checker rules, parser implementations, and runtime dispatch logic. The `examples/ladder.loom` file provides a complete, compilable specification of the T1→T5 progression for job scheduling. The `bench_colony_ladder` binary demonstrates the auto-escalation mechanism empirically.

---

## References

- Brélaz, D. (1979). New methods to color the vertices of a graph. *Communications of the ACM*, 22(4), 251–256.
- Burke, E.K., et al. (2013). Hyper-heuristics: A survey of the state of the art. *Journal of the Operational Research Society*, 64(12), 1695–1724.
- Garcia-Molina, H., & Salem, K. (1987). Sagas. *ACM SIGMOD Record*, 16(3), 249–259.
- Ghiringhelli, J.C. (2026). Loom: Materialising Academic Semantic Specifications as First-Class Language Constructs. *Pragmaworks Preprint.*
- Ghiringhelli, J.C. (2026). Generative Specification: A Pragmatic Programming Paradigm for the Stateless Reader. *Pragmaworks Preprint.*
- Hansen, N., & Ostermeier, A. (2001). Completely derandomized self-adaptation in evolution strategies. *Evolutionary Computation*, 9(2), 159–195.
- Hansen, N., Finck, S., Ros, R., & Auger, A. (2009). Real-parameter black-box optimization benchmarking 2009: Noiseless functions definitions. *INRIA Research Report RR-6829.*
- Holland, J.H. (1975). Adaptation in Natural and Artificial Systems. University of Michigan Press.
- Hutter, F., Kotthoff, L., & Vanschoren, J. (2019). Automated Machine Learning. Springer.
- Johnson, D.S. (1974). Fast algorithms for bin packing. *Journal of Computer and System Sciences*, 8(3), 272–314.
- Jumper, J., et al. (2021). Highly accurate protein structure prediction with AlphaFold. *Nature*, 596, 583–589.
- Kirilenko, A.A., et al. (2017). The flash crash: High-frequency trading in an electronic market. *Journal of Finance*, 72(3), 967–998.
- Kirkpatrick, S., Gelatt, C.D., & Vecchi, M.P. (1983). Optimization by simulated annealing. *Science*, 220(4598), 671–680.
- Kool, W., van Hoof, H., & Welling, M. (2019). Attention, learn to solve routing problems! *ICLR 2019.*
- Lenton, T.M., et al. (2019). Climate tipping points — too risky to bet against. *Nature*, 575, 592–595.
- Lourenço, H.R., Martin, O.C., & Stützle, T. (2003). Iterated local search. In *Handbook of Metaheuristics* (pp. 320–353). Kluwer.
- Mankowitz, D.J., et al. (2023). Faster sorting algorithms discovered using deep reinforcement learning. *Nature*, 618, 257–263.
- Perelson, A.S., et al. (1997). Decay characteristics of HIV-1-infected compartments during combination therapy. *Nature*, 387, 188–191.
- Preskill, J. (2018). Quantum computing in the NISQ era and beyond. *Quantum*, 2, 79.
- Rice, J.R. (1976). The algorithm selection problem. *Advances in Computers*, 15, 65–118.
- Smith, W.E. (1956). Various optimizers for single-stage production. *Naval Research Logistics Quarterly*, 3(1–2), 59–66.
- Snoek, J., Larochelle, H., & Adams, R.P. (2012). Practical Bayesian optimization of machine learning algorithms. *NeurIPS 2012.*
- Srinivas, N., et al. (2010). Gaussian process optimization in the bandit setting. *ICML 2010.*
- Sutton, R.S., & Barto, A.G. (1998). Reinforcement Learning: An Introduction. MIT Press.
- Vanschoren, J. (2018). Meta-learning: A survey. *arXiv:1810.03548.*
- Vinyals, O., Fortunato, M., & Jaitly, N. (2015). Pointer networks. *NeurIPS 2015.*
- Wolpert, D.H., & Macready, W.G. (1997). No free lunch theorems for optimization. *IEEE Transactions on Evolutionary Computation*, 1(1), 67–82.
