# loom
<!-- ForgeCraft sentinel | 2026-04-20 | npx forgecraft-mcp refresh . --apply to update -->
A cli, library project. Must not become a monolith.

Read `.claude/index.md` before any task. Navigate to the relevant branch. Load core.md always.

---

## Genome Application (GS T5 Evolution Loop)

This section is the GS architectural constitution for the **stateless reader** (Claude Code)
executing the T5 evolution loop in `evolve.yml`. Each `-- GS EVOLUTION SPEC` block in a genome
file is a machine-readable derivation unit. This section maps each mutation type to its source
location and how to apply it correctly.

### What a genome file is

A genome file (`genomes/evolved/gen{N}/*.loom`) is a GS specification produced by MeiosisEngine
after an experiment run. It records promoted mutations from the colony. It is NOT runnable
production code — it is the mold. Your job as the stateless reader is to derive the actual source
changes from it.

### Mutation type → source mapping

#### ParameterAdjust
```
-- GS EVOLUTION SPEC
-- mutation:   ParameterAdjust
-- entity:     climate
-- parameter:  aave_target_health_factor
-- delta:      +0.050000
-- source:     src/runtime/bioiso_runner.rs — BIOISOSpec for "climate"
-- verify:     cargo test --lib -- runtime
-- reason:     "..."
```
**How to apply:**
1. Open `src/runtime/bioiso_runner.rs`
2. Find the `BIOISOSpec { id: "climate", ... }` block
3. Inside its `metric_bounds` or directly in the spec, find the field named `aave_target_health_factor`
4. If the field is a float literal: add `delta` to its current value
5. If the field is computed: add `delta` to the relevant constant or default
6. The `spawn_domain()` function seeds entity-specific `AutomationConfig` — look there first

**Sanity check:** delta is typically small (±0.01–±0.20 for HF, ±0.5–±5.0 for LP range). If the
resulting value would violate a hard safety bound (e.g. HF < 1.20 or LP range > 25%), do not apply
— log the skip and continue to the next spec block.

#### StructuralRewire
```
-- GS EVOLUTION SPEC
-- mutation:   StructuralRewire
-- entity:     climate
-- signal:     co2_ppm
-- wires_to:   grid_stability
-- source:     src/runtime/bioiso_runner.rs — spawn_domain("climate") rule set
-- verify:     cargo test --lib -- runtime
-- reason:     "..."
```
**How to apply:**
1. Open `src/runtime/bioiso_runner.rs`
2. Find the `spawn_domain("climate")` call and its associated `rules: vec![...]`
3. Add a new `Rule { condition: SignalCondition::Threshold { metric: "co2_ppm".into(), threshold: 0.7, above: true }, proposal: MutationProposal::StructuralRewire { ... } }`
4. The rule should reference the signal name from `-- signal:` and target from `-- wires_to:`

#### EntityClone
```
-- GS EVOLUTION SPEC
-- mutation:   EntityClone
-- source:     climate
-- new_id:     climate_2
-- source_file: src/runtime/bioiso_runner.rs — duplicate BIOISOSpec for "climate" as "climate_2"
-- verify:     cargo test --lib -- runtime
-- reason:     "..."
```
**How to apply:**
1. Open `src/runtime/bioiso_runner.rs`
2. Find the `BIOISOSpec` for `climate`
3. Duplicate it with `id: "climate_2"` (or the actual `new_id`)
4. Add it to the `ALL_DOMAINS` / entity list so it gets seeded
5. Ensure all test references are updated if needed

#### EntityPrune
```
-- GS EVOLUTION SPEC
-- mutation:   EntityPrune
-- entity:     urban_heat
-- source:     src/runtime/bioiso_runner.rs — remove BIOISOSpec for "urban_heat" from ALL_DOMAINS
-- verify:     cargo test --lib -- runtime
-- reason:     "..."
```
**How to apply:**
1. Remove the named entity from the `ALL_DOMAINS` list in `src/runtime/bioiso_runner.rs`
2. Comment it out rather than deleting — the genome history is the deletion record
3. Leave the `BIOISOSpec` struct definition in place (just remove from list)

#### EntityRollback
```
-- GS EVOLUTION SPEC
-- mutation:   EntityRollback
-- entity:     climate
-- checkpoint: 42
-- source:     src/runtime/bioiso_runner.rs — restore params for "climate" to checkpoint state
-- verify:     cargo test --lib -- runtime
-- reason:     "..."
```
**How to apply:**
1. Find the entity's `BIOISOSpec` in `src/runtime/bioiso_runner.rs`
2. Revert any recent `ParameterAdjust` deltas that were applied since checkpoint tick
3. If checkpoint tick is unknown, revert to the defaults declared in `AutomationConfig::default()`

### Verification gate (T2 harness)

After applying ALL mutation blocks in a genome:
```sh
cargo test --lib --test-threads=1
```
- All 314 tests must pass
- If any test fails: `git checkout src/` then `exit 1`
- If all pass: `git add src/` then commit with message `auto(genome): apply {genome_name} mutations [GS T5]`

### What NOT to do
- Do not modify test files unless the genome explicitly targets them
- Do not add new features or refactor beyond what the spec blocks require
- Do not apply a delta that would violate hard safety bounds (HF < 1.20, LP > 25%)
- Do not commit if cargo test fails
- Do not touch `Cargo.toml`, `Cargo.lock`, or `.github/` files

### Source map (quick reference)
| Entity | Primary source | Notes |
|--------|---------------|-------|
| All entities | `src/runtime/bioiso_runner.rs` | `BIOISOSpec` + `spawn_domain()` |
| Polycephalum rules | `src/runtime/bioiso_runner.rs` | `rules: vec![...]` in spawn calls |
| Gate source | `src/runtime/gate.rs` | registered per entity |
| Experiment config | `src/runtime/experiment.rs` | tick interval, log path |
