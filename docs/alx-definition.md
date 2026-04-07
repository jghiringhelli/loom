# ALX — Autopoietic Loom eXperiments

ALX is the validation gate for Loom v1. It is not a test suite in the unit-test
sense. It is a series of Loom programs that collectively demonstrate every feature
works, composes with every other feature, and that the language can describe itself.

The criterion: **S_realized ≥ 0.90** — 90% of claimed correctness properties are
proved, not just parsed or type-checked.

---

## ALX-1: Feature Matrix

**What:** A single Loom program that exercises every construct from M1–M100 at least
once. Every parser path, every checker, every codegen target is hit.

**Gate:** Compiles to Rust without error. Zero checker warnings.

**What it proves:** The compiler wiring is complete. No feature was added to the AST
but forgotten in the checker pipeline.

```loom
-- experiments/alx/ALX-1-feature-matrix.loom
-- One construct per milestone, in dependency order.
-- If this file compiles: all 100 milestones are wired end-to-end.

module ALX1_FeatureMatrix

  -- M56: Refinement type
  type Probability = Float where x >= 0.0 and x <= 1.0

  -- M87: Tensor type
  fn correlation_matrix :: Unit -> Tensor<2, [10, 10], Float>
    describe: "10x10 correlation matrix"
  end

  -- M86: Conservation annotation
  fn transfer_capital @conserved(Value) @idempotent
      :: Float -> Float -> Float
    require: amount > 0.0
    ensure: result = amount
  end

  -- M84: Distribution family
  fn model_returns @probabilistic
      :: Unit -> Float
    distribution:
      family: GeometricBrownian(drift: 0.05, volatility: 0.2)
    end
  end

  -- M85: Randomness quality
  fn generate_nonce @crypto_random @requires_auth
      :: Unit -> Bytes
    describe: "Cryptographically secure nonce"
  end

  -- M88: Stochastic process
  fn model_rate @probabilistic
      :: Unit -> Float
    distribution:
      family: OrnsteinUhlenbeck(mean: 0.05, theta: 0.5, sigma: 0.02)
    end
    process:
      kind: OrnsteinUhlenbeck
      mean_reverting: true
    end
  end

  -- M98: Session type
  session ExchangeProtocol
    client:
      send: OrderRequest
      recv: OrderAck
    end
    server:
      recv: OrderRequest
      send: OrderAck
    end
    duality: client <-> server
  end

  -- M92: Polyglot store
  store PriceHistory :: TimeSeries
    event Tick :: {
      symbol: String,
      price:  Float,
      volume: Float
    }
    retention: "90d"
    resolution: "1s"
  end

  -- M66: AOP aspect
  aspect AuditAspect
    pointcut: fn where @conserved
    after: emit_audit_record
    order: 1
  end

  -- M41: Being (biological entity)
  being MarketSensor
    telos: "Detect price anomalies"
    sense: PriceSignal {
      channel: MidPrice(price: Float)
      channel: Spread(width: Float)
    }
  end

  -- M67: Correctness report
  correctness_report:
    proved:
      - conservation_of_value: @conserved_checker_passed
      - refinement_bounds:     probability_in_0_1
      - tensor_rank_shape:     matrix_2d_10x10
      - duality:               exchange_protocol_dual
      - crypto_randomness:     nonce_uses_csprng
    unverified:
      - smt_completeness:      requires_z3_feature
  end

end
```

---

## ALX-2: Cross-Feature Coherence

**What:** Features that interact — AOP + biological + formal types + stores all in
one module. The goal is to surface interaction bugs that unit tests miss.

**Gate:** Compiles. Every claimed interaction is verified by the checker, not just
parsed.

**Key compositions to test:**
- `@requires_auth` + `SecurityAspect` pointcut + `temporal: precedes verify_token before`
- `being` with `degenerate:` + `canalize:` + `senescence:` + `@conserved(Value)`
- `store: TimeSeries` + `sense:` channel reading from that store
- `distribution: GeometricBrownian` + `process: kind: GeometricBrownian` coherence
- `@pseudo_random` + `@requires_auth` → compiler rejects (negative test)
- `session` duality violation → compiler rejects (negative test)

---

## ALX-3: Self-Description

**What:** The Loom compiler itself, described in Loom. Every public function in
`src/checker/` has a Loom signature with `require:`, `ensure:`, and `@conserved`
where applicable.

**Gate:** The self-description compiles. The `correctness_report:` block has
≥ 10 `proved:` entries.

**What it proves:** Loom is expressive enough to describe its own semantics.
Self-description without self-contradiction is a strong consistency signal.

```loom
module LoomCompiler

  describe: "The Loom compiler — self-certifying since M65"

  fn compile @idempotent @pure
      :: Source -> Result<CompiledOutput, Vec<Error>>
    describe: "Parse + check + emit. Deterministic for any given source."
    require: source.len() > 0
    ensure: result.is_ok() implies all_checkers_passed
  end

  fn check_conservation @conserved(InformationContent)
      :: Module -> Vec<Error>
    describe: "Verify @conserved annotations. Noether (1915)."
    ensure: result.len() = 0 implies no_conservation_violations
  end

  fn check_duality
      :: SessionDef -> Vec<Error>
    describe: "Verify session type duality. Honda (1993)."
    ensure: result.len() = 0 implies protocol_deadlock_free
  end

  correctness_report:
    proved:
      - parse_deterministic:     idempotent_annotation
      - checker_pipeline_order:  aspect_order_verified
      - conservation_preserved:  noether_grounded
      - duality_complete:        honda_1993
      - refinement_bounds_held:  smt_discharged
    unverified:
      - turing_completeness:     out_of_scope_by_design
      - runtime_correctness:     target_language_responsibility
  end

end
```

---

## ALX-4: SMT Fix Loop

**What:** A Rust test harness that:
1. Compiles ALX-3 and reads its `correctness_report:` block
2. For each `unverified:` claim, generates a more constrained Loom program
3. Recompiles. If the claim can now be proved, it moves to `proved:`
4. Loops until all claims are proved OR provably unprovable (undecidable by design)
5. Terminates in ≤ 10 iterations

**Gate:** Convergence. S_realized = proved / (proved + unverified) ≥ 0.90.

**What it proves:** The self-fix loop works. Loom can identify its own specification
gaps and close them iteratively — the core BIOISO property.

---

## ALX-5: Polyglot Coherence

**What:** A single being that uses every store kind, reads every sense channel, and
has temporal + conservation + AOP constraints all active simultaneously.

**Gate:** Compiles to Rust, TypeScript, and WASM from the same source. All three
outputs are semantically consistent (same store interfaces, same effect types).

**Purpose:** Proves the multiple emission targets don't diverge for complex programs.

---

## ALX-6: Distribution Integrity

**What:** Systematically tests that every statistical claim the compiler makes about
distributions is correct:
- Cauchy/Lévy: CLT/LLN convergence claims rejected
- Beta: α, β > 0 enforced
- Gaussian: std_dev > 0 enforced
- GeometricBrownian + Gaussian: distribution/process mismatch rejected
- `@true_random` in `@requires_auth` context: accepted
- `@pseudo_random` in `@requires_auth` context: rejected

**Gate:** All 12 distribution checker tests pass. Zero false positives (valid
programs not rejected), zero false negatives (invalid claims not caught).

**Academic standard:** Every rejection rule is grounded in a published source
(NIST SP 800-90A, Cauchy (1853), Lévy (1937), Blum-Blum-Shub (1986)).

---

## S_realized Metric

```
S_realized = proved_claims / (proved_claims + unverified_claims)
```

Where:
- `proved_claims`: entries in `correctness_report: proved:` that have a passing
  checker backing them (not just a string)
- `unverified_claims`: entries in `unverified:` that are structurally unverifiable
  (e.g., runtime behavior, Turing completeness)

**Gate: S_realized ≥ 0.90 across all six ALX programs combined.**

If S_realized < 0.90 after ALX-4's fix loop: the gap becomes the roadmap for v1.1.

---

## ALX Execution Order

```
ALX-1 (feature matrix)  ─┐
ALX-6 (distributions)   ─┤─→ ALX-3 (self-description) ─→ ALX-4 (fix loop)
ALX-5 (polyglot)        ─┘                                      │
ALX-2 (cross-feature)                                           ↓
                                                          S_realized ≥ 0.90
                                                                 │
                                                                 ↓
                                                          publish-merge
```
