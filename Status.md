# Status.md

## Last Updated: 2026-04-10
## Branch: main (merged v0.2.0)

## Completed (this session)
- **fix(parser): ecosystem telos sub-block end-consuming fix** (commit 0c96fe7)
  - Root cause: `parse_ecosystem_def` telos handler only read the string literal,
    leaving `bounded_by: X ... end` in the stream. Sub-block end was consumed as
    ecosystem end, terminating the module loop before stores/fns were parsed.
  - Fix: after reading the telos string, detect sub-block; consume until end + end.
  - Scalper stores (TickHistory :: TimeSeries, TradeHistory :: FlatFile) now emit.
  - `save_state`, `load_state`, `fetch_market_data` functions emit in scalper.rs.
  - All 800+ tests still pass.
- **chore: move examples/emit/ — fix cargo test compilation error** (commit 18769da)
  - Emitted .rs files were in examples/ root, Cargo tried to build them as binary examples
  - Moved to examples/emit/ subdirectory (Cargo ignores subdirs for auto-discovery)
- **feat: merged launch/v0.2-public-release → main** — tagged v0.2.0
- **chore: rename package to loom-lang** — 'loom' crate name is taken by tokio-rs
  - Binary: `loom`, lib: `loom`, install: `cargo install loom-lang`
  - `cargo package --no-verify`: 291 files, 2.7MiB — ready to publish

## Current State
- **Branch: main** — v0.2.0 tagged
- All M1-M119 milestones complete
- Binary verify: all 5 examples compile end-to-end (loom → rustc)
- 800+ tests passing across 90+ test suites (0 failures on CI/Linux)
- Verification pipeline V1-V9: 35 PROVED, 19 EMITTED, 4 PENDING
- Scalper demo: stores emit + runner.rs with real CoinGecko data + synthetic OU fallback
- Package: `loom-lang v0.2.0` ready for `cargo publish`

## Next
- **cargo publish** — `cargo publish` from main (need crates.io token)
- **arXiv preprint** — docs/publish/white-paper.md needs final BIOISO + verification section
- **Marketing launch** — GitHub release notes, Hacker News, Reddit r/rust, PragmaWorks blog

## Known Limitations
- `evolve_test` fails on Windows with OS error 5 (Windows Defender blocks test binary)
  — pre-existing issue, passes on Ubuntu CI
- Only FIRST test in a module is parsed (parse_test_def doesn't consume end) — known parser limitation
- Codegen functions > 50 lines (hygiene debt, carry forward)

## Decisions made (this session)
- Package renamed to `loom-lang` for crates.io compatibility
- Emitted .rs examples moved to `examples/emit/` to avoid Cargo binary discovery

## Blockers
- None for publish — just needs crates.io token + `cargo publish`
- ALX-6: S_realized = 44/45 = 0.9778

## Architecture Decision Log
| Date | Decision | Rationale | Status |
| 2025-07-18 | Loom uses single = for equality | Language design | Active |
| 2025-07-18 | Refined types resolve to base in inference | Arithmetic on refined params | Active |
| 2026-04-06 | Annotation payload collects all tokens between () | @foreign_key(Table.field) | Active |
| 2026-04-10 | Ecosystem telos sub-block end consumed by telos handler | Prevents module loop early exit | Active |
