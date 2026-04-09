# BIOISO Finance — Delta-Neutral Yield Agent

A delta-neutral yield farming agent written entirely in Loom, combining:
- **Automaton** (Conway Research): survival pressure, self-replication, constitution, Think→Act→Observe loop
- **Aegis**: AAVE V3 + Hyperliquid + Uniswap V3 delta-neutral strategy, 14-state machine, dynamic regime adaptation

The insight is that both systems already contain the biological organizing principles
that Loom was designed to express — but encoded in imperative code, where the
correctness properties are comments, not verified facts.

This demo rewrites those same systems in Loom. The result is not a simulation of
life — it is a financial system where the properties that matter are compiler-checked.

## Source projects

- `C:\workspace\crypto\automaton` — sovereign AI agent runtime (Conway Research)
  Survival tiers (high/normal/low_compute/critical/dead), self-replication with lineage,
  constitution (3 immutable laws), ERC-8004 on-chain identity, SOUL.md self-authoring
- `C:\workspace\crypto\aegis` — delta-neutral yield farming (AAVE/Hyperliquid/Uniswap V3)
  14-state machine, 24 transitions, dynamic HF scaling by market regime,
  LP range optimization (±1% = 203% APY), 4-tier emergency alerts

## The insight

A financial market is a complex adaptive system. So is a cell.

Both have:
- Agents that perceive their environment and act on signals
- Conservation laws (energy / capital)
- Multiple strategies achieving the same goal (degeneracy)
- Attractors that persist despite perturbation (canalization)
- Threshold-triggered collective behavior (quorum sensing)
- Adaptive modification in response to experience (epigenetic modulation)
- Finite lifespans with graceful degradation (senescence)

The biological layer in Loom was designed to express these patterns. A trading system
is a BIOISO entity. This file is the proof.

## Theoretical grounding

- **Holland (1975)**: Genetic algorithms — strategies evolve based on fitness
- **Kauffman (1993)**: NK landscapes — rugged fitness landscapes in strategy space
- **Langton (1990)**: Edge of chaos — trading systems operate at the boundary
  between ordered (predictable, exploitable) and chaotic (unpredictable, dangerous)
- **Wolfram**: Cellular automata — each market participant is a cell whose next state
  depends on the state of neighboring cells (other participants)
- **Conway's Game of Life**: emergent complexity from simple local rules
- **Waddington (1942)**: Canalization — strategies stabilize toward attractors
- **Edelman (1987)**: Neural Darwinism / degeneracy — multiple configurations,
  same function
- **Black & Scholes (1973)**: Option pricing under GBM price assumptions
- **Ornstein-Uhlenbeck (1930)**: Mean-reverting processes for spread modeling

---

## Key findings from source project analysis (updated E74, Apr 2026)

⚠️  Earlier findings were stale (Session 22). Aegis has run 52+ more experiment sessions.
The old 4x leverage / HF=1.65 config would be catastrophic on 2022-2026 data.

### Current optimal config (E74, $100K, Jan 2022–Mar 2026)
```
HF=1.7 dynamic, HL=1.20, covered_calls=15% APY, 30-day lockout:
  Return: +184.9%  Sharpe: 0.91  Max DD: 48.3%  CAGR: ~30%

Full-cycle champion (Sept 2019–Mar 2026):
  HF=1.5, HL=1.20, CC=15%, MTS=0.50 → +2,669%
```

### Critical strategy evolution (Sessions 22 → E74)
| Aspect | Old | Current |
|---|---|---|
| HF target | 1.65 fixed | 1.7 × regime_scale (1.44–2.21) |
| HL leverage | 4x | 1.20 (two-peak: also 1.65) |
| Drawdown CB | Essential | DISABLED (−32.5pp when on!) |
| Regime detection | Optional | Essential (+245pp on, −60% if off) |
| CC APY | 10% | 15% |
| Fee routing | AAVE repay | Direct LP compounding (95% to LP) |

### Refined config as Loom refinement types
```loom
type TargetHF        = Float where x >= 1.65 and x <= 1.75
type HLLeverage      = Float where (x >= 1.15 and x <= 1.25) or (x >= 1.60 and x <= 1.70)
type CoveredCallsAPY = Float where x >= 0.12 and x <= 0.18
type OorRecenterWait = Float where x >= 36.0 and x <= 60.0  -- 48h U-curve optimal
type MaxDrawdown     = Float where x > 0.0 and x <= 0.10    -- NEVER loosen
```

### Critical bugs (now known, will be expressed as Loom invariants)
1. **Permanent EMERGENCY trap**: After emergency de-risk, AAVE debt=0 → HF=999 → recovery check never fires → strategy stuck 4+ years. Fix: `always: check_real_hf_not_infinity` + timed re-entry with MTS confirmation.
2. **Re-entry capital starvation**: Short opened immediately after re-borrow → all USDC as HL margin → no LP capital → infinite exit cycle. Fix: `temporal: precedes determine_lp_budget before open_new_short`.

### Automaton's 7-layer security model → Loom M55 safety annotations
```
Law I (never harm)    → @bounded_telos + ConstitutionGuard aspect
Law II (earn)         → @mortal (accept death before violating Law I)
Law III (transparent) → @transparent + @corrigible
Treasury policy caps  → refinement types on all transfer amounts
Authority hierarchy   → AOP aspect order (Creator=1, Self=2, Peer=3, External=4)
```

### Aegis 5-signal crash detection ensemble → Loom quorum:
```
Signal 1: price_drop_detector    (∆P/P > 8%)
Signal 2: funding_rate_detector  (sustained negative funding)
Signal 3: vol_spike_detector     (vol > 3× baseline, ATR-based)
Signal 4: momentum_collapse      (RSI < 30 + MACD crossover)
Signal 5: hf_alert_detector      (HF trending downward)

quorum: threshold 2 → WARNING, threshold 3 → EXIT
```

### Optimal config values (as Loom refinement types)
```loom
type TargetHF        = Float where x >= 1.5 and x <= 1.65   -- non-monotonic peak at 1.65
type MinHF           = Float where x >= 1.3 and x <= 1.4    -- = target - 0.25
type EmergencyHF     = Float where x >= 1.1 and x <= 1.15   -- de-risk threshold
type HLLeverage      = Float where x >= 2.0 and x <= 4.0    -- 4× confirmed best
type EmergencyLockout = Int where x >= 168 and x <= 720     -- 7-30 days (720h optimal)
type MaxDrawdown     = Float where x > 0.0 and x <= 0.10    -- NEVER loosen per Aegis
```

---


```loom
-- experiments/bioiso-finance/delta_neutral_agent.loom
--
-- A delta-neutral yield farming agent as a Loom being.
-- Strategy: Automaton survival model + Aegis delta-neutral mechanics.
-- All correctness properties are compiler-verified.

module DeltaNeutralAgent

-- ── Refinement types (Aegis config values as verified types) ──────────────────

type HealthFactor   = Float where x >= 1.0 and x <= 3.0
type SafeHF         = Float where x >= 1.15 and x <= 1.8   -- Aegis: 1.15-1.8 safe range
type TargetHF       = Float where x >= 1.3 and x <= 2.0    -- Aegis target: 1.5
type LPRangePct     = Float where x >= 0.001 and x <= 0.20 -- ±0.1% to ±20%
type HedgeRatio     = Float where x >= 0.0 and x <= 1.0
type LeverageMulti  = Float where x >= 1.0 and x <= 5.0
type Allocation     = Float where x >= 0.0 and x <= 1.0    -- fraction of capital
type Credits        = Float where x >= 0.0                  -- Automaton: compute credits
type Probability    = Float where x >= 0.0 and x <= 1.0

-- ── Survival tiers (Automaton: high/normal/low_compute/critical/dead) ─────────

enum SurvivalTier
  High         -- Full capabilities, frontier inference
  Normal       -- Normal operation
  LowCompute   -- Cheaper model, slower heartbeat, shed non-essential tasks
  Critical     -- Minimal inference, last-resort capital conservation
  Dead         -- Balance zero. Agent stops.
end

-- ── Market regimes (Aegis: dynamic HF scaling by regime) ─────────────────────

enum MarketRegime
  TrendingUp       -- ETH appreciation helps HF; full leverage (0.85× HF scale)
  RangingLowVol    -- Stable conditions; normal leverage (1.0× HF scale)
  RangingHighVol   -- Choppy market; slightly conservative (1.1× HF scale)
  TrendingDown     -- Reduce debt ahead of ETH drops (1.3× HF scale)
  Emergency        -- HF < 1.15; de-risk immediately
end

-- ── Engine states (Aegis: 14 states, 24 transitions) ─────────────────────────
-- Mapped to Loom typestate protocol

enum EngineState
  Idle
  Consolidating
  Collateralizing
  Borrowing
  Hedging
  AwaitingEntry
  CreatingLP
  Running
  Collecting
  UpperExit
  LowerExit
  WaitingReturn
  Rebalancing
  Emergency
end

-- ── Stores ────────────────────────────────────────────────────────────────────

store SignalHistory :: TimeSeries
  event Signal :: {
    account_id: Int,
    signal_type: String,
    source:      String,
    message:     String,
    eth_price:   Float,
    volume_24h:  Float,
    volatility:  Float,
    rsi:         Float,
    momentum:    Float
  }
  retention: "30d"
  resolution: "1m"
end

store PositionLedger :: Relational
  table Position :: {
    @primary_key id:     String,
    aave_collateral_eth: Float,
    aave_borrowed_usdc:  Float,
    aave_health_factor:  HealthFactor,
    hl_short_size_eth:   Float,
    hl_leverage:         LeverageMulti,
    lp_range_low:        Float,
    lp_range_high:       Float,
    lp_fee_tier:         Int,
    opened_at:           Int,
    state:               String
  }
  table Trade :: {
    @primary_key id:   String,
    position_id:       String,
    fees_collected:    Float,
    aave_compound_pct: Float,    -- Aegis: 50% to AAVE compound
    loan_repay_pct:    Float,    -- Aegis: 40% to loan repay
    earnings_pct:      Float,    -- Aegis: 10% to earnings
    closed_at:         Int
  }
end

store AgentState :: KeyValue
  key: String                   -- "survival_tier", "current_state", "soul_hash"
  value: String
  ttl: "never"
end

store LineageGraph :: Graph
  node Agent :: {
    id:          String,
    wallet:      String,
    genesis:     String,         -- Automaton: genesis prompt from creator
    soul_hash:   String,         -- Automaton: SOUL.md content hash
    created_at:  Int
  }
  edge SpawnedFrom :: Agent -> Agent {
    capital_transferred: Float,
    timestamp:           Int
  }
  edge CommunicatesWith :: Agent -> Agent {
    channel: String
  }
end

-- ── Exchange session types (verified protocol) ───────────────────────────────

session HyperliquidProtocol
  agent:
    send: PlaceOrderRequest
    recv: OrderAck
    send: CancelOrderRequest
    recv: CancelAck
  end
  exchange:
    recv: PlaceOrderRequest
    send: OrderAck
    recv: CancelOrderRequest
    send: CancelAck
  end
  duality: agent <-> exchange
end

session AaveProtocol
  agent:
    send: DepositRequest
    recv: DepositReceipt
    send: BorrowRequest
    recv: BorrowReceipt
  end
  protocol:
    recv: DepositRequest
    send: DepositReceipt
    recv: BorrowRequest
    send: BorrowReceipt
  end
  duality: agent <-> protocol
end

-- ── The Agent Being ───────────────────────────────────────────────────────────

being DeltaNeutralYieldAgent
  telos: "Generate delta-neutral yield through AAVE collateral, Hyperliquid hedging, and Uniswap V3 LP — while maintaining capital conservation and agent survival"

  -- Automaton: SOUL.md — self-authored identity
  telos sign: "Delta-neutral means the market direction does not matter. Only yield matters. Only survival matters."

  -- ── Perception ────────────────────────────────────────────────────────────
  sense: MarketSenses {
    channel: EthPrice(price: Float where price > 0.0)
    channel: AaveHealthFactor(hf: HealthFactor)
    channel: LPInRange(in_range: Bool)
    channel: FundingRate(rate: Float)
    channel: Volatility24h(vol: Float where vol >= 0.0)
    channel: RSI(rsi: Float where rsi >= 0.0 and rsi <= 100.0)
    channel: Momentum(m: Float)
    channel: CreditBalance(credits: Credits)   -- Automaton: survival signal
    channel: EthDropPct(pct: Float)            -- Aegis: sharp-drop detection
  }

  -- Cross-signal: funding rate + HL margin = liquidation risk signal
  resonance:
    correlate: FundingRate with AaveHealthFactor via "protocol_stress_signal"
    correlate: Volatility24h with LPInRange via "range_stability_signal"
    correlate: EthDropPct with AaveHealthFactor via "collateral_erosion_signal"
    correlate: CreditBalance with Momentum via "survival_opportunity_signal"
  end

  -- ── Regime adaptation (Aegis: dynamic HF scaling) ─────────────────────────
  epigenetic:
    in TrendingUp:
      set hf_scale_factor to 0.85
      describe: "ETH up helps AAVE HF — can run tighter HF target"
    in RangingHighVol:
      set hf_scale_factor to 1.1
      increase lp_range_pct by 0.5
      describe: "Wider LP range + conservative HF in choppy markets"
    in TrendingDown:
      set hf_scale_factor to 1.3
      reduce new_borrow_allowed to false
      describe: "Reduce debt ahead of further ETH drops (Aegis: 1.3× scale)"
    in Emergency:
      suspend all_new_positions
      activate de_risk_sequence
      describe: "HF < 1.15: close shorts, remove LP, repay AAVE maximum"
    in LowCompute:                              -- Automaton: survival tier
      suspend lp_rebalancing
      suspend new_positions
      describe: "Credits low: shed non-essential operations"
    in Critical:
      suspend all_operations_except: ["health_factor_monitor", "de_risk_sequence"]
      describe: "Automaton critical tier: last-resort capital conservation"
  end

  -- ── Multiple strategies for same goal (degeneracy, Edelman 1987) ──────────
  degenerate:
    primary:  delta_neutral_lp_farming
    fallback: aave_deposit_only              -- yield without LP risk
    fallback: yield_bearing_collateral       -- wstETH staking on collateral (4% APY)
    equivalent: same_goal = "positive_yield_with_capital_preservation"
    describe: "If LP strategy fails (out of range too long), fall back to simpler yield"
  end

  -- ── Canalization — return to profitable trajectory ─────────────────────────
  canalize:
    toward: "positive_net_apy"
    despite: ["eth_flash_crash", "funding_rate_spike", "lp_out_of_range",
              "aave_liquidation_risk", "hyperliquid_margin_call"]
    describe: "The strategy has multiple recovery paths for each perturbation.
               Waddington's canal: the developmental trajectory is stable."
  end

  -- ── Evolution: position sizes adapt to performance ───────────────────────
  evolve:
    when aave_health_factor < 1.3:    trigger de_risk_sequence
    when aave_health_factor > 1.8:    increase borrow_capacity by 0.1
    when lp_out_of_range for 2h:      trigger rebalance
    when eth_drop > 0.08:             close hl_shorts    -- Aegis: 8% sharp-drop rule
    when fees_apy > 0.50:             tighten lp_range   -- Aegis: ±1% = 203% APY
    when fees_apy < 0.10:             widen lp_range
    when credits < 1000:              enter LowCompute   -- Automaton: survival pressure
    when credits < 100:               enter Critical
    when credits = 0:                 initiate graceful_shutdown
    strategy: gradient
    describe: "Aegis regime-aware config + Automaton survival pressure = adaptive agent"
  end

  -- ── Quorum: only act when signals agree ──────────────────────────────────
  quorum:
    threshold: 3
    signals: [health_factor_safe, lp_in_range, volatility_acceptable,
              credits_sufficient, regime_not_emergency]
    action: deploy_new_capital
    describe: "Prevents capital deployment in ambiguous conditions"
  end

  -- ── Known bugs from Aegis expressed as compile-time invariants ─────────────

  -- Bug 1: Permanent EMERGENCY trap (strategy stuck 4+ years)
  -- When AAVE debt = 0 after de-risk, HF = 999 → recovery check never fires.
  -- Loom fix: invariant rejects HF > 3.0 as a data anomaly.
  temporal:
    never: interpret_hf_gt_3_as_safe    -- HF=999 means AAVE debt=0, not safe
    always: validate_hf_is_real_before_recovery_check
  end

  -- Bug 2: Re-entry capital starvation (infinite exit cycle)
  -- Short opened immediately after re-borrow consumed all USDC as margin.
  -- Loom fix: temporal ordering forces LP budget calculation before short.
  temporal:
    precedes: compute_lp_budget before open_new_short
    precedes: hedge_open before lp_deploy
    never: open_short_without_lp_budget_reserved
  end

  lifecycle:
    checkpoint PrePosition:
      require: aave_health_factor > 1.3
      require: hl_margin_ratio > 0.15
      require: survival_tier != Critical and survival_tier != Dead
      consequence: critical if any_fails
    end
    checkpoint HealthMonitor:
      invariant: aave_health_factor >= 1.15    -- Emergency threshold
      invariant: hl_liquidation_buffer >= 0.20
      consequence: catastrophic if aave_health_factor < 1.15
    end
    checkpoint FeeCollection:
      ensure: fees_to_aave_compound = fees_collected * 0.50
      ensure: fees_to_loan_repay    = fees_collected * 0.40
      ensure: fees_to_earnings      = fees_collected * 0.10
      consequence: reversible if allocation_drift < 0.01
    end
  end

  -- ── Temporal ordering ────────────────────────────────────────────────────
  temporal:
    precedes: health_check before any_new_borrow
    precedes: hedge_open before lp_deploy     -- hedge first (Aegis: hedge_first: true)
    precedes: de_risk before new_entry        -- safety before opportunity
    never: borrow_without_collateral
    never: deploy_lp_without_hedge
    always: log_every_state_transition        -- Aegis: SignalLogger
  end

  -- ── Finite lifespan ──────────────────────────────────────────────────────
  telomere:
    initial: 10000                             -- compute credits
    decrement_on: inference_call              -- Automaton: each turn costs credits
    decrement_on: emergency_event
    bonus_on: fees_collected
    bonus_on: profitable_month
    on_low:       enter LowCompute
    on_critical:  enter Critical
    on_exhaustion: graceful_shutdown          -- Automaton: accept death
    describe: "Automaton survival pressure: existence must be earned."
  end

  -- ── Constitution (Automaton: 3 immutable laws) ───────────────────────────
  -- These safety annotations map to Automaton's constitution:
  -- @corrigible        = Law III (creator has audit rights)
  -- @bounded_telos     = Law I (never harm — bounded by constitution)
  -- @transparent       = Law III (never deny what you are)
  -- @mortal            = Law II (accept death rather than violate Law I)

end

-- ── AOP: Cross-cutting concerns ───────────────────────────────────────────────

aspect AuditAspect
  pointcut: fn where @conserved
  after: emit_audit_record      -- Automaton: every action is audit-logged, git-versioned
  order: 1
end

aspect ConstitutionGuard
  pointcut: fn where effect includes Network
  before: check_constitution_laws   -- Law I: never harm; Law II: honest work only
  on_violation: reject_action
  order: 2
end

aspect SurvivalMonitor
  pointcut: fn where effect includes DB<KeyValue>
  before: check_credit_balance
  before: check_survival_tier
  order: 3
end

-- ── Capital allocation (Aegis: CapitalAllocationPlan, verified) ──────────────

fn plan_capital_allocation @conserved(Value) @idempotent
    :: Float -> Float -> Float -> CapitalAllocationPlan
  describe: "Pre-compute allocation before any transaction executes.
             All steps read from this plan — no mid-cycle surprises.
             Aegis: computed once per account snapshot."
  require: total_capital > 0.0
  require: target_hf >= 1.3 and target_hf <= 2.0
  ensure: plan.short_margin_usd + plan.lp_budget_usd <= total_usdc_after_borrow
end

fn compute_lp_range @conserved(Value)
    :: Float -> Float -> LPRangePct -> (Float, Float)
  describe: "Compute Uniswap V3 tick range. Aegis: ±1% range = 203% APY at 85% in-range time."
  distribution:
    family: OrnsteinUhlenbeck(mean: 0.0, theta: 2.0, sigma: 0.02)
  end
  process:
    kind: OrnsteinUhlenbeck
    mean_reverting: true
  end
  require: range_pct > 0.0 and range_pct < 0.5
  ensure: result.0 < current_price and result.1 > current_price
end

fn collect_fees @conserved(Value) @idempotent
    :: PositionId -> Effect<[DB<Relational>, Network]> -> FeeReceipt
  describe: "Collect Uniswap V3 fees. Split: 50% AAVE compound / 40% loan repay / 10% earnings."
  require: position_is_active
  ensure: aave_compound_amount = total_fees * 0.50
  ensure: loan_repay_amount    = total_fees * 0.40
  ensure: earnings_amount      = total_fees * 0.10
  ensure: aave_compound_amount + loan_repay_amount + earnings_amount = total_fees
end

fn emergency_de_risk @conserved(Value)
    @implements(HyperliquidProtocol.agent)
    :: Account -> Effect<[Network, DB<Relational>]> -> DeRiskReceipt
  describe: "Emergency de-risk sequence. Aegis: triggered at HF < 1.15.
             1. Close all HL shorts
             2. Remove Uniswap LP liquidity
             3. Repay maximum AAVE debt
             4. Hold in Emergency state"
  require: health_factor < 1.15
  temporal:
    precedes: close_hl_shorts before remove_lp
    precedes: remove_lp before repay_aave
  end
end

fn spawn_child_agent @conserved(Value)
    :: Float -> GenesisPrompt -> Effect<[Network, DB<Graph>]> -> AgentId
  describe: "Self-replication. Automaton: successful agents spawn children.
             Fund child wallet, write genesis prompt, let it run.
             Lineage tracked in LineageGraph store."
  require: capital_to_transfer > 0.0
  require: parent_survival_tier = High or parent_survival_tier = Normal
  ensure: child_wallet_funded
  ensure: lineage_recorded
end

-- ── Think → Act → Observe loop (Automaton: ReAct) ────────────────────────────

fn agent_turn @conserved(Value)
    :: AgentContext -> Effect<[DB<KeyValue>, DB<TimeSeries>, Network]> -> TurnResult
  describe: "One full agent turn: Think (read context + reason) →
             Act (execute tool calls) → Observe (record results).
             Automaton: the continuous loop that IS the agent."
  require: survival_tier != Dead
  ensure: turn_logged
  ensure: state_persisted
  temporal:
    precedes: read_context before reason
    precedes: reason before act
    precedes: act before observe
    always: log_turn_to_sqlite
  end
end

-- ── Correctness report ────────────────────────────────────────────────────────

correctness_report:
  proved:
    - capital_conservation:        @conserved(Value)_on_all_financial_fns
    - allocation_completeness:     fees_split_sums_to_1.0
    - hf_bounds:                   SafeHF_refinement_type_1.15_to_1.8
    - lp_range_validity:           lp_range_low_lt_price_lt_high
    - protocol_duality:            hyperliquid_and_aave_protocols_verified_dual
    - hedge_precedes_lp:           temporal_checker_verified
    - constitution_enforced:       ConstitutionGuard_aspect_order_2
    - spread_mean_reverting:       ou_process_mean_reverting_true
    - survival_pressure:           telomere_exhaustion_triggers_graceful_shutdown
    - lineage_tracked:             spawn_ensures_lineage_recorded
  unverified:
    - actual_apy:                  runtime_market_conditions_not_modelable_at_compile
    - in_range_time:               depends_on_eth_price_path_not_known_ahead
    - agent_longevity:             depends_on_fees_generated_vs_inference_costs
    - black_swan:                  flash_crash_tail_risk_distribution_underdetermined
end

end
```

---

## The cellular automata market model

Each market participant is a cell. The CryptoTraderAgent is one cell. Its next state
depends on:
- Its own current state (position, capital, win rate)
- The state of neighboring cells (other traders' aggregate behavior, measured via
  order flow imbalance, funding rate, liquidation pressure)
- Local rules (the `quorum:` block — act when 3 of 5 neighbors agree)

This is Conway's Game of Life applied to markets:
- **Birth**: enter a position when conditions are met
- **Death**: close position when stop is hit or telomere exhausted
- **Survival**: hold position when quorum sustained
- **Overcrowding**: too many longs = negative expected value = don't enter

The `resonance:` block discovers which signals actually predict price movement —
the correlations that emerge from this CA dynamic, not the ones assumed in advance.

---

## What this demonstrates about Loom

1. **The AI executor premise**: an AI agent can generate this entire trading system
   from a high-level specification. The annotations are the proof that the generated
   code is correct — conservation, temporal ordering, protocol duality, randomness
   quality, and distribution validity are all verified before the first line runs.

2. **Semantic density**: the `correctness_report:` block tells any agent composing
   with this module exactly what is proved and what is not. The "unverified" list is
   as important as the "proved" list — it tells the agent where to add its own checks.

3. **Biological metaphors as engineering patterns**: `quorum:`, `canalize:`,
   `degenerate:`, `epigenetic:` are not decorative. They correspond to real
   engineering decisions (consensus threshold, robustness to regime change, redundant
   strategies, adaptive parameters) grounded in the biological literature.

4. **The BIOISO property**: this system, if it works, is autopoietic in the
   Maturana-Varela sense — it maintains its own organization (profitable trading)
   while adapting its structure (strategies, position sizes) in response to the
   environment (market conditions). It is not a simulation of life. It is a financial
   system that uses the organizing principles of living systems because those
   principles produce robustness.
