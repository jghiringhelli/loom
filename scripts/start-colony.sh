#!/bin/sh
# CEMS colony startup — autonomous experiment mode.
#
# Seeds all 11 entities (idempotent) then runs the full experiment:
#   inject signals -> drift -> T1/T2/T3 proposals -> gate -> canary -> branch
#
# Environment variables:
#   DB_PATH     SQLite store (default: /data/bioiso.db)
#   TICK_MS     Milliseconds between ticks (default: 5000)
#   EXP_TICKS   Total ticks to simulate (default: 50000)
#   EXP_SEED    RNG seed for signal simulator (default: 42)
#   EXP_LOG     Path for JSON-lines experiment log (default: /data/experiment.jsonl)

set -e

DB="${DB_PATH:-/data/bioiso.db}"
TICK="${TICK_MS:-5000}"
TICKS="${EXP_TICKS:-50000}"
SEED="${EXP_SEED:-42}"
LOG="${EXP_LOG:-/data/experiment.jsonl}"

echo "bioiso: seeding colony at ${DB}"
loom runtime seed --db "${DB}"

echo "bioiso: starting autonomous experiment (ticks=${TICKS} tick_ms=${TICK} seed=${SEED})"
exec loom runtime experiment \
  --db "${DB}" \
  --ticks "${TICKS}" \
  --tick-ms "${TICK}" \
  --seed "${SEED}" \
  --summary-interval 20 \
  --branch-threshold 3 \
  --max-branches 2 \
  --log-path "${LOG}"