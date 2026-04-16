#!/bin/sh
# CEMS colony startup — autonomous experiment mode.
#
# Seeds all 20 entities (idempotent) then runs the full experiment:
#   inject signals -> drift -> T1/T2/T3 proposals -> gate -> canary -> branch
#
# Environment variables:
#   DB_PATH          SQLite store (default: /data/bioiso.db)
#   TICK_MS          Milliseconds between ticks (default: 1 = maximum speed)
#   EXP_TICKS        Total ticks to simulate (default: 50000)
#   EXP_SEED         RNG seed for signal simulator (default: 42)
#   EXP_LOG          Path for JSON-lines experiment log (default: /data/experiment.jsonl)
#   TELOMERE_LOG     Path for telomere audit JSONL (default: /data/telomere.jsonl)
#   MANIFEST_PATH    Path for bioiso.toml project manifest (default: /data/bioiso.toml)
#   MAX_ENTITY_COUNT Hard cap on living entities to bound LLM cost (default: 50)

set -e

DB="${DB_PATH:-/data/bioiso.db}"
TICK="${TICK_MS:-1}"
TICKS="${EXP_TICKS:-50000}"
SEED="${EXP_SEED:-42}"
LOG="${EXP_LOG:-/data/experiment.jsonl}"
TEL_LOG="${TELOMERE_LOG:-/data/telomere.jsonl}"
MANIFEST="${MANIFEST_PATH:-/data/bioiso.toml}"
MAX_ENT="${MAX_ENTITY_COUNT:-50}"

echo "bioiso: seeding colony at ${DB}"
loom runtime seed --db "${DB}"

echo "bioiso: starting autonomous experiment (ticks=${TICKS} tick_ms=${TICK} seed=${SEED} max_entities=${MAX_ENT})"
exec loom runtime experiment \
  --db "${DB}" \
  --ticks "${TICKS}" \
  --tick-ms "${TICK}" \
  --seed "${SEED}" \
  --summary-interval 20 \
  --branch-threshold 3 \
  --max-branches 2 \
  --max-entities "${MAX_ENT}" \
  --log-path "${LOG}" \
  --telomere-log "${TEL_LOG}" \
  --manifest-path "${MANIFEST}"