# Status.md

## Last Updated: 2026-11-12
## Branch: main

## Current State
- Local full test run (2026-11-12, Windows, `cargo test --no-fail-fast -- --test-threads=1`): **1975 passed, 0 failed, 17 ignored**.
- Earlier CI "green" was misleading: `| tee` without `pipefail` hid failures and `cargo test` stopped at the first failing suite (~480 of ~1975 tests ran). Fixed in `d8efbcf`.
- **M1: only 12/42 corpus+example programs emit Rust that compiles.** Evidence in `experiments/maturity/evidence/`.
- No development activity between 2026-05-15 and 2026-11-12 apart from repo tidy-up.
- Test counts quoted elsewhere (314 / 339 / 410) are stale; use the numbers above.
- Colony on Railway: last known redeploy 2026-04-20. **Current state unknown** — check with `railway` before relying on it.
- Invellum gen1: skeleton committed; staging/prod status unknown.

## 2026-11-12 — Tidy-up
- Merged-branch cleanup: local branches deleted; remote deletion pending (blocked by push hook, see below).
- Kept on purpose: `genomes/evolved` — used by `.github/workflows/evolve.yml` (ref + GS T5 prompt).
- Untracked stray `out/*.rs`; added gitignore rules for emitted example artefacts.
- Runtime DBs/logs moved out of the repo to `../loom-archive/runtime-artifacts-2026-11-12/`.
- Added `ROADMAP.md` (public) and `docs/maturity-experiments.md` (pass/fail gates).

## Next
1. Install Rust toolchain locally; run M0–M4 from `docs/maturity-experiments.md`.
2. Phase A blockers in `ROADMAP.md` (repo URL mismatch, release binaries, README quick start).
3. ALX reached S_realized = 1.000 (386/386, 2026-04-27) but with manual patches to derived code in phases 4–6. **ALX-R was run 2026-11-12: S_realized = 0.375 (145/387), gate 0.90 not met** — see `experiments/alx/evidence/alx-r/`. Next: fix the spec using the reader's gap log, rerun ≥3×, add a second model; then LX-4.

## History
Earlier status (2026-04-20, T5 AEGIS + colony fixes) is preserved in git history of this file.
