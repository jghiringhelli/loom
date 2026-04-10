# Launch Posts — Loom v0.2.0

## Hacker News — Show HN

**Title:**
> Show HN: Loom – an AI-native language that transpiles to Rust/TS/WASM and rejects your code if it violates biological invariants

**Body:**
```
Loom is a functional language I've been building for the last several months.
One .loom file compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and
JSON Schema simultaneously. The compiler enforces semantic contracts — drawn
from 50 years of PL research and, in this release, from molecular biology —
before emitting a single line of output.

The practical pitch: every annotation pays for itself across all five targets
at once. You write `require: amount > 0.0` once. The compiler generates
debug_assert!, a Kani proof harness, and an OpenAPI `x-precondition` field.

The deeper claim: life and type theory independently solved the same problems.
Homeostasis is a feedback controller with bounds. Telos is a convergence target
with a fitness function. Replicative senescence is a bounded counter with an
exhaustion protocol. These are not metaphors. They are the same formal object
described in different notation. Loom makes every entry in the isomorphism table
a first-class keyword.

What the compiler rejects before emitting output:
- being: without telos: (every entity must have a declared goal)
- autopoietic: true without @mortal (unbounded self-replication = cancer by type)
- regulate: with unreachable bounds
- @corrigible without telos.modifiable_by (corrigibility is structural, not opted-in)
- telomere: decrements that can underflow

The verification pipeline is now closed through V7: all five example modules
compile through `loom compile → rustc → ./binary` with zero errors.

The killer demo is a scalping agent written in 378 lines of Loom. It compiles
to Rust, the runner fetches real BTC price data from CoinGecko, and backtests:
491 trades, 53.4% win rate, Sharpe 0.760.

GitHub: https://github.com/PragmaWorks/loom
119 milestones, 800+ tests, MIT license.

Happy to answer anything about the language design, the biological isomorphisms,
or why I think the alignment problem is a specification completeness problem.
```

---

## Reddit — r/rust

**Title:**
> I built a language that transpiles to Rust (and 4 other targets) and rejects code that violates biological invariants — Show r/rust: Loom v0.2

**Body:**
```
Hey r/rust — I've been building Loom, a functional language that compiles to
Rust (among other targets), and wanted to share the v0.2 release.

**What it does mechanically:**
One .loom source file → Rust + TypeScript + WebAssembly + OpenAPI 3.0 +
JSON Schema. The compiler runs 10 semantic checker passes before emitting
anything.

**The Rust output is not a sketch:**
- `require:` / `ensure:` → `debug_assert!` at runtime + `#[cfg(kani)] #[kani::proof]` harnesses
- `store TimeSeries` → full EventStore/Aggregate/EventBus trait implementation
- `lifecycle Connection :: Disconnected -> Connected -> Authenticated -> Closed` → phantom state structs
- All 5 example modules compile through `rustc` with zero errors (V7 verified)

**What's new in v0.2 — BIOISO:**
This release adds the biological isomorphism layer. The claim is that biology
and type theory independently solved the same problems:

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

  die-by: max_trades via quiescence
end
```

This compiles to Rust structs with trait impls, debug_assert! bounds enforcement,
and Kani proof harnesses. The `die-by:` is syntactic sugar for a telomere block —
a bounded AtomicU64 counter that enforces mortality at the type level.

**The killer demo:**
`experiments/scalper/` is a complete backtest. The agent's logic is in Loom,
the runner executes it with real CoinGecko BTC data: 491 trades, 53.4% win
rate, Sharpe 0.760. Both acceptance criteria satisfied.

**Why Rust specifically:**
Rust was the obvious choice for the primary target. The ownership model maps
cleanly to Loom's typestate and effect tracking. The `#[cfg(kani)]` proof
harness pattern is idiomatic. And `cargo` makes the "does it actually compile
and run" verification gate straightforward.

GitHub: https://github.com/PragmaWorks/loom
119 milestones | 800+ tests | MIT | Written in Rust

Curious what the r/rust community thinks about the generated output quality.
The emission for more complex constructs (stores, ecosystems, propagate chains)
is where I'd most appreciate expert eyes.
```

---

## Twitter/X Thread

**Tweet 1:**
> Loom v0.2 is out.
>
> One .loom file compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema simultaneously.
>
> The compiler rejects your code before emitting a line if it violates biological invariants.
>
> 🧵

**Tweet 2:**
> Life spent 3.5 billion years solving goal-directed, self-correcting systems.
> Type theory spent 80 years rediscovering the same solutions.
>
> BIOISO makes every isomorphism a keyword:
>
> `telos:` = quantified convergence target
> `regulate:` = Lyapunov stability with bounds
> `die-by:` = bounded counter with exhaustion protocol

**Tweet 3:**
> The killer demo: scalping agent in 378 lines of Loom.
>
> loom compile → rustc → cargo run
>
> 491 trades
> 53.4% win rate ✅ (threshold: 50%)
> Sharpe 0.760 ✅ (threshold: 0.5)
>
> The agent's decision logic is declared. The runner executes it.

**Tweet 4:**
> The verification pipeline is now closed through V7:
>
> V1 ✅ Contracts compile (debug_assert!)
> V2 ✅ Kani formal proofs emitted
> V3 ✅ Proptest property tests generated
> V5 ✅ All 13 store kinds → typed Rust structs
> V7 ✅ loom → rustc → ./binary for all 5 examples

**Tweet 5:**
> The compiler enforces:
> • autopoietic: true without @mortal → compile error (unbounded self-replication = cancer by type)
> • being: without telos: → compile error (every entity must have a goal)
> • @corrigible without modifiable_by → compile error
>
> The alignment problem is a specification completeness problem.

**Tweet 6:**
> 119 milestones
> 800+ tests
> MIT license
> Written in Rust
>
> github.com/PragmaWorks/loom
>
> cargo install loom-lang (coming to crates.io this week)

---

## LinkedIn Post

**Announcing Loom v0.2 — AI-Native Language with Biological Invariants**

I'm releasing Loom v0.2 today — a compiler project I've been working on that I think represents a genuinely new approach to language design.

**The technical premise:** Life and type theory independently solved the same problems. Homeostasis is a feedback controller. Telos is a convergence target. Replicative senescence is a bounded counter. These aren't analogies — they're the same formal object in different notation. Loom makes every entry in the isomorphism table a keyword with a compiler check.

**What it does:** One `.loom` source file compiles to Rust, TypeScript, WebAssembly, OpenAPI 3.0, and JSON Schema simultaneously. The compiler runs 10 semantic passes — including biological invariant checks — before generating any output.

**The killer demo:** A scalping agent written in 378 lines of Loom, compiled to Rust, backtested against real Bitcoin price data: 491 trades, 53.4% win rate, Sharpe ratio 0.760. Both acceptance criteria passed.

**Why this matters beyond the language:** Loom is the practical implementation of Generative Specification — the methodology where a single, machine-readable specification derives all artifacts (code, types, tests, documentation, API contracts) without gaps. The AI assistant doesn't fill gaps; the specification has no gaps to fill.

119 milestones complete. 800+ tests. MIT license.

GitHub: https://github.com/PragmaWorks/loom

I'm particularly interested in connecting with researchers working on programming language theory, formal verification, AI safety, and systems biology. The isomorphism table is deep and I've only mapped part of it.
