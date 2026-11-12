# Status.md

## Last Updated: 2026-11-12
## Branch: main

## Current State
- CI on `main` (HEAD `7f2d8eb`, 2026-05-15): **green**.
- No development activity between 2026-05-15 and 2026-11-12 apart from repo tidy-up.
- Test counts quoted elsewhere (314 / 339 / 410) were **not re-verified on 2026-11-12** — no Rust toolchain on the current dev machine. Treat the CI log as the source of truth.
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
3. ALX reached S_realized = 1.000 (386/386, 2026-04-27) but with manual patches to derived code in phases 4–6. Run **ALX-R** (blind re-derivation from the final spec) and LX-4.

## History
Earlier status (2026-04-20, T5 AEGIS + colony fixes) is preserved in git history of this file.
