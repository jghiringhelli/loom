# Loom v0.2.0 — BIOISO + Verification Pipeline + Scalper Demo

## What is Loom?

Loom is an AI-native functional language that transpiles a single `.loom` file into **Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema** simultaneously. Its compiler enforces semantic contracts drawn from 50 years of programming language research — and, as of v0.2, from molecular biology — before emitting a single line of output.

```loom
being ScalpingAgent
  telos: "maximize risk-adjusted PnL"
    telos_function: fn(state: AgentState, env: Market) -> Float<fitness>
    bounded_by: pnl_safety_ok
    modifiable_by: operator
  end

  regulate SpreadSignal
    trigger: spread > pos_threshold
    action: | aggressive -> increase_position
             | default   -> hold
    bounds: (neg_threshold, pos_threshold)
  end

  evolve strategy
    | aggressive when spread > threshold
    | conservative when spread < neg_threshold
    | hold otherwise
  end

  die-by: max_trades via quiescence
end
```

This compiles to Rust with `debug_assert!` contract enforcement, Kani proof harnesses, proptest property tests, and full event store structs. One source file. Zero gaps.

---

## What's New in v0.2

### BIOISO — Biological Isomorphisms as Language Constructs (M117–M119)

The complete biological isomorphism layer is now in the compiler. Every entry in the table below is a first-class keyword with a parser, a semantic checker, and a code generator:

| Keyword | Biological origin | Formal identity |
|---|---|---|
| `regulate:` | Homeostasis | Lyapunov stability with typed bounds |
| `telos:` + `telos_function:` | Final cause | Quantified goal objective, machine-checkable |
| `evolve:` | Directed evolution | Stochastic optimisation with convergence constraint |
| `telomere:` / `die-by:` / `wither-at:` | Replicative senescence | Bounded counter with exhaustion protocol |
| `propagate:` | Cytokine signalling | Typed effect chain across ecosystem members |
| `intent_coordinator:` | Quorum sensing + consensus | Multi-agent telos alignment with arbitration |
| `crispr:` | Gene editing | Controlled self-modification with declared target locus |
| `morphogen:` | Reaction-diffusion patterning | Turing instability + Gierer-Meinhardt kinetics |
| `autopoietic: true` | Autopoiesis | Operational closure + self-production trait |

These are not metaphors. They are formal identities — the same problem, discovered independently by biology and type theory, expressed in different notation.

**New in M117:** `trigger/action` patterns in `regulate:`, `telos_function:` for quantified fitness objectives.  
**New in M118:** `die-by:` / `wither-at:` / `exhaust-at:` telomere aliases, `intent_coordinator:` for multi-agent ecosystems.  
**New in M119:** `propagate:` blocks for typed effect chains across ecosystem members.

### Verification Pipeline V1–V9

All critical verification gates are now closed:

| Gate | Status |
|---|---|
| V1: Contracts compile (debug_assert! + rustc) | ✅ PROVED |
| V2: Kani formal verification harnesses emitted | ✅ EMITTED |
| V3: Proptest property tests generated | ✅ EMITTED |
| V4: Effect tracking across module boundaries | ✅ PROVED |
| V5: All 13 store kinds → typed Rust structs | ✅ PROVED |
| V6: Audit headers in all generated files | ✅ PROVED |
| V7: Binary round-trip (loom → rustc → ./binary) | ✅ PROVED |
| V8: ALX convergence contracts | ✅ EMITTED |
| V9: Dafny theorem prover integration | 🔲 Pending |

**V7 is the hard one:** all five core example modules compile through `loom compile → rustc → ./binary` with zero errors. Generated code executes.

### The Killer Demo: Scalping Agent

`experiments/scalper/` is a complete working backtest:

```
loom compile scalper.loom
rustc scalper.rs -o scalper_agent
cd experiments/scalper && cargo run
```

| Metric | Result | Acceptance criterion |
|---|---|---|
| Trades | 491 | — |
| Win rate | **53.4%** | ≥ 50% ✅ |
| Sharpe ratio | **0.760** | ≥ 0.5 ✅ |
| Net PnL | +$30.13 | — |

The runner fetches real OHLCV data from CoinGecko, caches to `experiments/scalper/data/btc_ohlc.json`, and falls back to synthetic Ornstein-Uhlenbeck price data. The agent's entire decision logic is declared in `scalper.loom`. The runner just executes it.

### Parser Fix: Ecosystem Telos Sub-blocks

`ecosystem:` blocks with a `telos:` containing sub-blocks (`bounded_by:`, `measured_by:`) no longer cause premature module parse termination. Stores and functions declared after the ecosystem are now correctly parsed and emitted.

### LPN — Loom Protocol Notation

An AI-to-AI communication protocol layer for describing compiler tasks in a minimal, unambiguous notation. See `src/lpn/` and `tests/lpn_test.rs`.

---

## Numbers

- **119 milestones** complete across 9 phases
- **800+ tests** passing (90+ test suites, 0 failures on CI)
- **7 emission targets**: Rust, TypeScript, WASM, OpenAPI 3.0, JSON Schema, Mesa ABM, NeuroML
- **10 semantic checkers** that run before any output is written
- **4 BIOISO demos** validated end-to-end

---

## Install

```sh
# From source (Cargo v1.75+):
git clone https://github.com/PragmaWorks/loom
cd loom
cargo build --release
./target/release/loom compile examples/01-hello-contracts.loom

# Coming soon:
cargo install loom-lang
```

---

## Breaking Changes

None. v0.2 is additive — all v0.1 `.loom` files compile unchanged.

---

## What's Next (v0.3 targets)

- **REPL / playground** — `loom run` for interactive evaluation
- **ALX killer demo** — Autonomous Learning Explorer as a full BIOISO demonstration
- **Standard library expansion** — network, crypto, file domains
- **V9** — Dafny theorem prover integration
- **Runtime verification** — executing Kani proofs in CI

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). New emission targets, standard library domains, BIOISO constructs, and examples are all welcome.
