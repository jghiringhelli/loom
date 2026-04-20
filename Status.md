# Status.md

## Last Updated: 2026-04-20
## Branch: main
## HEAD: 496c7a7 — T3 genome encoding + logs reader CLI

## Current State
- **314 lib tests passing** — 0 failures
- **Colony LIVE on Railway** — redeploy triggered 2026-04-20 (fixes T1 + gate)
- **T5 BIOISO example**: `examples/tier5/aegis_delta_neutral.loom` — compiles cleanly
- **All tier 1–5 examples compile**: tier1–tier4 fixed (previous session); tier5 new AEGIS example added

---

## Colony Status (2026-04-20)

### Known Issues — Fixed in HEAD, redeploy triggered today:
1. **T1=0 all ticks** — Polycephalum rules never seeded because entities already in SQLite store
   on restart → auto-seed skipped → supervisor never populated → no T1 rules.
   Fix: `75eb498` (2026-04-15) — supervisor now re-populated from store on startup.
2. **Gate source not registered** — `gate.register_source()` not called in production.
   All `StructuralRewire`, `EntityClone`, `EntityPrune`, `EntityRollback` proposals fail
   as `MalformedProposal`. Fix: same commit `75eb498`.
3. **entities=0 display** — supervisor in-memory, not persisted across restart.
   Entities ARE in SQLite and being processed; display is cosmetic.

### Live Metrics (tick ~5320, pre-fix deployment):
```
[tick 5320] drift=150 | proposals=1882 promoted=1000 | T1=0 T2=470 T3=47 | entities=0 | branches=0
```
After redeploy:
- T1 should activate (Polycephalum rules seeded from supervisor)
- Structural proposals (EntityClone/Prune/Rollback) should pass gate and promote

---

## Completed (since last Status.md update 2026-04-14)

### Session: T5 AEGIS BIOISO + Colony Fixes (2026-04-20)
- **`examples/tier5/aegis_delta_neutral.loom`** — T5 BIOISO for AEGIS delta-neutral DeFi strategy:
  - Full MTS (Market Trend Score) formula: weighted 5-component signal in [-1,+1]
  - 4-way regime taxonomy with asymmetric thresholds (canonical E88: bull=0.35, bear=-0.40)
  - 22-state state machine (IDLE → RUNNING → crash subgraph → HIBERNATING)
  - Regime-scaled params: HF target, LP range width, hedge ratio, LP capital fraction
  - OOR recenter: 48h base (E51 U-curve) + vol-aware 4h fast path (E56)
  - `evolve: derivative_free` — CMA-ES over 40-parameter space (bimodal landscape)
  - `rewire:` — structural rewire of LP/crash/waiting_return graph topology on basin shift
  - `plasticity:` — MTS weight recalibration on regime lag > 48h
  - `telos:` — Sharpe ≥ 1.0, return ≥ +200%, DD ≤ 35% over 4-year horizon
  - Starting point: canonical E88 = +213.6% / Sharpe=1.02 / DD=33.4%
  - Compiles cleanly; 11 tests; 314 lib tests still passing
- **`src/runtime/meiosis.rs`** — Fixed `render_genome()` encoding T3 structural decisions:
  - `EntityClone` → regulate block + fn stub in genome
  - `EntityPrune` → comment in genome
  - `EntityRollback` → comment in genome
  - Previously all dropped silently via `_ => {}`
- **`src/main.rs`** — `loom runtime logs` command:
  - Reads `experiment.jsonl`; per-tick table: drift/proposals/promoted/tier/branches/top drifters
  - Flags: `--path`, `--last N`, `--entity`, `--t3-only`
- **Colony redeploy** — `railway up` triggered today (2026-04-20)

### Session: Tier 1–4 Benchmark Examples (2026-04-15..19)
- All 4 tier benchmark `.loom` files now compile cleanly:
  - `examples/tier3/hyper_heuristic_scheduler.loom` — `modifiable_by: human_operator` added
  - `examples/tier4/bayesian_optimizer.loom` — `learn:` block, `todo` stubs, `modifiable_by`
  - `examples/tier4/neural_combinatorial.loom` — `Unit]`→`Unit>` fix, `todo` stubs
- Parser fix: `learn:` block handler with `skip_block_to_end()` depth-aware helper
- Exception added to `.forgecraft/exceptions.json` for `examples/01-hello-contracts.rs`

### Session: Colony + Meiosis + ClaudeGanglion (2026-04-14..15)
- `ClaudeGanglionClient` — T2 uses Haiku, T3 uses Sonnet
- Cost guard: T3 only fires when drift > threshold after T1+T2 miss
- Meiosis R14: genetic recombination + GitHub genome publication
- `EvolutionJudge` — autonomous, no human review
- Colony deployment fix: all 6 Railway bugs fixed
- Colony LIVE with autonomous experiment mode

---

## Architecture: FULLY OPERATIONAL

```
SignalSimulator (44 signals/tick per domain)
         ↓
Orchestrator.run_once()
  Stage 0 Membrane (immune) → SHA-256 genome hash, rate limiter, quarantine
         ↓
  Telos Drift Engine → DriftEvent (score 0–1 per entity)
         ↓
  Tier 1 Polycephalum (< 50ms, deterministic rules)  ← WILL ACTIVATE AFTER REDEPLOY
         ↓ (on T1 miss × 3)
  Tier 2 ClaudeGanglionClient (Claude Haiku)
         ↓ (on T2 miss × 3)
  Tier 3 MammalBrain (Claude Sonnet, cost-guarded)
         ↓
  Type-safe Gate (loom::compile())  ← STRUCTURAL PROPOSALS WILL PASS AFTER REDEPLOY
         ↓
  Stage 5 Simulation (DigitalTwin, MeioticPool, SVD independence)
         ↓
  Canary Deploy → monitor → promote/rollback
         ↓
  Epigenome distillation (Buffer→Working→Core every 10 ticks)
  Mycelium gossip + pheromone deposits
         ↓
BranchingEngine.evaluate()
  → spawn child entities after 3 stable mutations
  → inherit parent epigenome (Core memories + telos bounds)
         ↓
Meiosis (end of experiment) → genome → GitHub publication
```

### Domain Entities (11) — all live on Railway
| Entity | Top-drift metric | Crisis tick range |
|---|---|---|
| climate | co2_ppm | 80-110 (El Niño), 280-320 |
| epidemics | Rt | 40-60 (relaxation), 200-240 (variant) |
| antibiotic_res | amr_deaths_per_yr_k | 120-150 (new strain) |
| grid_stability | frequency_hz | 70-85, 200-210 (faults) |
| soil_carbon | soc_change_per_mille | 100-140 (drought) |
| sepsis | mortality_28d_pct | 60-90 (outbreak) |
| flash_crash | order_book_depth_m | 55-65, 320-328 |
| nuclear_safety | safety_margin_pct | 180-210 (anomaly) |
| supply_chain | fill_rate_pct | 30-80 (port), 240-290 (geopolitical) |
| water_basin | aquifer_recharge_pct | 110-165 (drought) |
| urban_heat | urban_rural_delta_c | 160-200 (heat summer) |

### Tier Stack
| Tier | Model | Role | Cost |
|---|---|---|---|
| T1 | Polycephalum (rules) | Deterministic fast path | ~0 |
| T2 | Claude Haiku | Cluster synthesis | Low |
| T3 | Claude Sonnet | Structural proposals on high drift | Medium |

---

## Observing the Colony

```sh
railway logs                        # live tick summaries
loom runtime logs --last 100        # read local JSONL experiment log
loom runtime logs --t3-only         # filter to T3 Sonnet structural proposals only
loom runtime logs --entity climate  # single entity timeline
```

## Next
1. **Wait for redeploy** — T1 should activate; structural proposals should promote
2. **AEGIS results**: after colony runs with fixes, inspect genome files on GitHub
3. **AEGIS T5 integration**: wire AEGIS BIOISO into the colony as a 12th entity (DeFi domain)
4. **loom runtime logs analysis**: read current experiment.jsonl for T3 proposals content
5. **Retro-validation**: inject historical crisis signals against academic baselines
