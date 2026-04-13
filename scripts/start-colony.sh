#!/usr/bin/env sh
# CEMS colony startup script.
#
# 1. Seeds all 11 pre-configured BIOISO domain entities with expert-calibrated
#    telos bounds and baseline signals (idempotent — skips already-registered entities).
# 2. Starts the CEMS evolution daemon.

set -e

DB="${DB_PATH:-/data/bioiso.db}"
TICK="${TICK_MS:-5000}"

echo "bioiso: seeding colony at ${DB}"
loom runtime seed --db "${DB}"

echo "bioiso: starting evolution daemon (tick=${TICK}ms)"
exec loom runtime start --db "${DB}" --tick-ms "${TICK}"
