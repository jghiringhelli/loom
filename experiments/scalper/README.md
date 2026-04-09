# ScalpingAgent — Loom Killer Demo

A self-governing mean-reversion scalping agent declared in Loom.

This experiment demonstrates the full Loom value chain:
- **Declare** biology-inspired structure in `scalper.loom`
- **Compile** to idiomatic Rust via `loom compile`
- **Run** a synthetic backtest with `runner.rs`

## What it demonstrates

| Loom feature | Where |
|---|---|
| Refinement types (`SpreadBps`, `BoundedSize`, `PositivePrice`) | scalper.loom:64–66 |
| Privacy labels (`@never-log` on PnL fields) | scalper.loom:55–58 |
| Safety annotations (`@mortal @corrigible @sandboxed @bounded_telos`) | scalper.loom:180–184 |
| `being:` with Aristotle's four causes | scalper.loom:177+ |
| `telos:` with convergence thresholds | scalper.loom:204–213 |
| `regulate:` homeostatic parameter bounds | scalper.loom:217–240 |
| `evolve:` gradient-descent parameter search | scalper.loom:243–250 |
| `epigenetic:` regime switching on loss events | scalper.loom:254–264 |
| `telomere:` Hayflick-limit lifecycle bound | scalper.loom:267–270 |
| `autopoietic:` operational closure declaration | scalper.loom:275 |
| `scenario:` executable acceptance criteria | scalper.loom:278–290 |
| `ecosystem:` multi-being composition | scalper.loom:304+ |
| `signal:` typed inter-being channel | scalper.loom:307–311 |
| OU stochastic process type | scalper.loom:99–118 |
| Cauchy tail-risk model | scalper.loom:120–128 |
| GBM baseline drift model | scalper.loom:130–143 |

## The strategy

**Ornstein-Uhlenbeck mean reversion**. When bid-ask spread deviation exceeds
one standard deviation from its long-run mean (θ=2.0, μ=0.0, σ=0.15):

- **Long** when spread compressed (deviation < −1σ) — expect reversion upward
- **Short** when spread expanded (deviation > +1σ) — expect reversion downward
- **Exit** when OU deviation crosses zero (reversion confirmed)
- **Stop** when adverse move exceeds the stop threshold

## Quick start

```bash
# 1. Compile the Loom spec to Rust
loom compile scalper.loom

# 2. Run the synthetic backtest
rustc runner.rs --edition 2021 -o runner && ./runner
```

Expected output (1000 OU ticks, seed fixed):

```
═══════════════════════════════════════════════════════════════
 Loom ScalpingAgent — Synthetic OU Backtest
═══════════════════════════════════════════════════════════════
 Strategy : OU mean-reversion (θ=2.0, μ=0.0, σ=0.15)
 Universe : 1 synthetic instrument, 1000 ticks
 Risk     : 5% stop-loss, 2% take-profit per trade
─── Results ───────────────────────────────────────────────────
 Trades      : 491
 Wins        : 262 (53.4%)
 Realized PnL: $30.13
 Max drawdown: $20.43
 Sharpe ratio: 0.760
─── Acceptance criteria from scalper.loom: ────────────────────
   scenario ProfitableOnOU:
     pnl.realized > -500.0 → ✓ PASS ($30.13)
   scenario DrawdownBounded:
     positive Sharpe on OU data → ✓ PASS (0.760)
```

Both acceptance criteria declared in `scenario:` blocks inside `scalper.loom` pass.

## What Loom enforces at compile time

| Claim | Enforcement |
|---|---|
| PnL fields never logged | `@never-log` generates `// NEVER LOG` comment + emitter suppresses field in debug output |
| Agent cannot run forever | `telomere: limit: 100` → runtime limit emitted |
| Telos is bounded (no "maximize forever") | `@bounded_telos` checker rejects open-ended utility terms like "maximize" |
| Agent is corrigible | `@corrigible` → audit annotation in generated code |
| Parameter bounds maintained | `regulate:` blocks → generated bound assertions |
| Strategy converges toward telos | `evolve: constraint: "E[distance_to_telos] decreasing..."` |

## Why OU?

The Ornstein-Uhlenbeck process (Uhlenbeck & Ornstein 1930) is the continuous-time
analogue of an AR(1) process. It guarantees mean reversion in expectation:

```
E[X(t)] → μ as t → ∞
Var[X(t)] → σ²/(2θ) as t → ∞
```

For a market maker, the bid-ask spread is approximately OU around its long-run mean.
When the spread deviates beyond one standard deviation, the expected profit from
waiting for reversion exceeds the expected loss from the stop-loss threshold.
The `@probabilistic` annotation in `scalper.loom` makes this structural assumption
explicit and auditable.

## The Cauchy tail-risk model

`fn estimate_slippage @probabilistic` uses a Cauchy distribution (location=0, scale=0.002).
The Cauchy distribution has **no defined mean or variance** — it violates the Central
Limit Theorem. This is intentional: large orders in thin markets experience fat-tailed
slippage. The Loom emitter generates a warning comment and an inverse-CDF sampler:

```rust
// LOOM[structure:Cauchy]: estimate_slippage
// WARNING: NO defined mean or variance. CLT and LLN do NOT apply.
```

## File manifest

| File | Description |
|---|---|
| `scalper.loom` | Loom specification — the single source of truth |
| `scalper.rs` | Generated Rust (output of `loom compile scalper.loom`) |
| `runner.rs` | Standalone backtest runner — compile with `rustc` |
| `README.md` | This file |
