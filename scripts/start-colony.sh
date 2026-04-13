#!/usr/bin/env sh
# CEMS colony startup script.
#
# 1. Spawns all 11 pre-configured BIOISO domain entities (idempotent — skips
#    entities that are already registered in the signal store).
# 2. Starts the CEMS evolution daemon.
#
# Used as the Railway start command instead of bare `loom runtime start`.

set -e

DB="${DB_PATH:-/data/bioiso.db}"
TICK="${TICK_MS:-5000}"

echo "bioiso: initialising colony at ${DB}"

# ── Spawn domain entities ─────────────────────────────────────────────────────
# Each spawn call is idempotent on the entity ID — if the entity already exists
# in the store the command exits 0 without modifying it.

spawn_if_new() {
  entity_id="$1"
  name="$2"
  telos="$3"
  echo "  spawning ${entity_id} (${name})..."
  loom runtime spawn "${entity_id}" \
    --db "${DB}" \
    --name "${name}" \
    --telos "${telos}" \
    2>/dev/null || echo "  (${entity_id} already registered — skipping)"
}

spawn_if_new "climate"        "Climate Change Mitigation"     '{"target":"limit warming to 1.5°C"}'
spawn_if_new "epidemics"      "Epidemic Response"             '{"target":"suppress Rt below 1.0"}'
spawn_if_new "antibiotic_res" "Antibiotic Resistance (AMR)"   '{"target":"reduce AMR deaths below 700k/yr"}'
spawn_if_new "grid_stability" "Power Grid Stability (ERCOT)"  '{"target":"maintain frequency ±0.5Hz of 60Hz"}'
spawn_if_new "soil_carbon"    "Soil Carbon Sequestration"     '{"target":"increase SOC by 4‰/yr"}'
spawn_if_new "sepsis"         "ICU Sepsis Protocol"           '{"target":"reduce 28-day mortality below 20%"}'
spawn_if_new "flash_crash"    "HFT Flash Crash Prevention"    '{"target":"prevent order book collapse"}'
spawn_if_new "nuclear_safety" "Nuclear Reactor Safety"        '{"target":"maintain reactor within safety envelope"}'
spawn_if_new "supply_chain"   "Supply Chain Resilience"       '{"target":"fill rate >95% lead time <14d"}'
spawn_if_new "water_basin"    "Water Basin Allocation"        '{"target":"aquifer recharge >90%"}'
spawn_if_new "urban_heat"     "Urban Heat Island Mitigation"  '{"target":"urban-rural delta below 2°C"}'

echo "bioiso: colony initialised — starting evolution daemon (tick=${TICK}ms)"

# ── Start evolution daemon ────────────────────────────────────────────────────
exec loom runtime start --db "${DB}" --tick-ms "${TICK}"
