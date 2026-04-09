# Loom Standard Library

Loom ships four stdlib modules, each written entirely **in Loom** using existing
primitives. No compiler changes are needed to add a stdlib — only Loom source
files. This is the self-describing property of the language in action.

## Available Modules

| Module | Milestone | Source | Purpose |
|---|---|---|---|
| `SenseStdlib` | M83 | `sense_stdlib.loom` | All physically measurable quantities as typed signal channels |
| `ChemistryStdlib` | M89 | `chemistry_stdlib.loom` | Stoichiometry, kinetics, thermodynamics |
| `FinanceStdlib` | M90 | `finance_stdlib.loom` | GBM, Black-Scholes, Markowitz, VaR/CVaR, fixed income |
| `QuantumStdlib` | M91 | `quantum_stdlib.loom` | Qubit states, gates, measurement, quantum information |

All four are embedded at compile time as `&str` constants in `src/stdlib/mod.rs`
via `include_str!` — zero runtime I/O.

---

## SenseStdlib (M83)

**Source:** `src/stdlib/sense_stdlib.loom`

The complete measurable universe as first-class typed `sense` channels.
Grounded in the SI 2019 redefinition and Uexküll's Umwelt theory.

### Contents
- 7 SI base dimensions: `Length`, `Mass`, `Time`, `ElectricCurrent`,
  `ThermodynamicTemperature`, `AmountOfSubstance`, `LuminousIntensity`
- Extended senses: `Frequency`, `Pressure`, `Energy`, `Power`, `Voltage`,
  `Resistance`, `MagneticField`, `Luminance`, `SoundPressure`,
  `InformationEntropy`, `AngularMomentum`, `Concentration`, `RadioactiveDose`

### Design principles
- Every channel has a declared SI `unit:` and `dimension:` symbol
- Beings with no `umwelt:` block receive any declared sense — the mantis-shrimp model
- Channels map to `Float<unit>` refinement types when emitted to Rust

---

## ChemistryStdlib (M89)

**Source:** `src/stdlib/chemistry_stdlib.loom`

Stoichiometry, enzyme kinetics, thermodynamics, and molecular graph structures.

### Primitives used
| Loom primitive | Role |
|---|---|
| M3 Kennedy units | `Float<mol>`, `Float<J/mol>`, `Float<K>`, `Float<mol/L>` |
| M56 Refinement types | `Concentration ≥ 0`, `pH ∈ [0,14]`, `Temperature > 0` |
| M86 `@conserved` | Mass conservation, charge conservation |
| M87 Tensor types | Stoichiometric matrices, Hessians |
| M92 Graph store | Molecular graphs — atoms as nodes, bonds as edges |

### Key functions
| Function | Law | Reference |
|---|---|---|
| `michaelis_menten` | Enzyme kinetics | Michaelis & Menten (1913) |
| `arrhenius` | Temperature-dependent rate constant | Arrhenius (1889) |
| `gibbs_free_energy` | ΔG = ΔH - TΔS | Gibbs (1875) |
| `henderson_hasselbalch` | pH = pKa + log([A⁻]/[HA]) | Henderson-Hasselbalch (1916) |
| `limiting_reagent` | Stoichiometric limiting factor | Lavoisier (1789) |

### Stores
- `MoleculeGraph :: Graph` — atoms as typed nodes, bonds as typed edges

---

## FinanceStdlib (M90)

**Source:** `src/stdlib/finance_stdlib.loom`

Stochastic processes, options pricing, portfolio optimisation, risk measures,
and fixed-income analytics.

### Primitives used
| Loom primitive | Role |
|---|---|
| M3 Kennedy units | `Float<USD>`, `Float<BPS>`, `Float<percent>` |
| M56 Refinement types | `Probability ∈ [0,1]`, `Volatility ≥ 0`, `Price > 0` |
| M84 Probabilistic types | GBM (Normal), jump processes (Poisson) |
| M85 Randomness discipline | `crypto_random` for key generation, seeded for simulation |
| M86 `@conserved` | `NoArbitrage`, `Capital` |
| M87 Tensor types | Covariance matrices, factor loadings |
| M92 Stores | `TimeSeries` price history, `Relational` positions, `Document` risk register |

### Key functions
| Function | Model | Reference |
|---|---|---|
| `gbm_next_price` | Geometric Brownian Motion step | Bachelier (1900) |
| `gbm_expected_price` | GBM expectation `E[S(T)] = S₀eᵘᵀ` | — |
| `black_scholes_call` | European call `C = SN(d₁) - Ke⁻ʳᵀN(d₂)` | Black & Scholes (1973) |
| `black_scholes_put` | Put-call parity | — |
| `black_scholes_delta` | Option delta `N(d₁)` | — |
| `portfolio_variance` | `wᵀΣw` quadratic form | Markowitz (1952) |
| `sharpe_ratio` | `(E[Rp]-Rf) / σp` | Sharpe (1966) |
| `value_at_risk` | Parametric VaR | Basel (1996) |
| `conditional_value_at_risk` | CVaR / Expected Shortfall | Rockafellar & Uryasev (2000) |
| `bond_price` | DCF `P = Σ C/(1+y)ᵗ + F/(1+y)ᵀ` | — |
| `macaulay_duration` | Weighted average cash flow time | Macaulay (1938) |
| `modified_duration` | Interest rate sensitivity | — |

### Stores
- `PriceHistory :: TimeSeries` — ticker → adjusted close price
- `PortfolioPositions :: Relational` — position, cost basis, current price
- `RiskRegister :: Document` — per-factor VaR, CVaR, stress loss

---

## QuantumStdlib (M91)

**Source:** `src/stdlib/quantum_stdlib.loom`

Qubit state vectors, unitary gate operators, Born-rule measurement, Heisenberg
uncertainty verification, Schrödinger time evolution, and quantum information.

### Primitives used
| Loom primitive | Role |
|---|---|
| M3 Kennedy units | `Float<eV>`, `Float<Hz>`, `Float<rad>` |
| M56 Refinement types | `BornProb ∈ [0,1]`, `Phase ∈ [0,2π)`, `NumQubits > 0` |
| M84 Probabilistic types | Born rule measurement (Bernoulli outcomes) |
| M86 `@conserved` | `Unitarity`, `Probability`, `Energy` |
| M92 KeyValue store | Quantum register — basis state index → amplitude |

### Key functions
| Function | Quantum law | Reference |
|---|---|---|
| `born_probability` | `P(k) = |⟨k|ψ⟩|²` | Born (1926) |
| `normalise_check` | `Σ|αₖ|² + |βₖ|² = 1` | Dirac (1930) |
| `measurement_collapse` | Wavefunction collapse to `|k⟩` | von Neumann (1932) |
| `hadamard_alpha/beta` | `H: |+⟩ = (|0⟩+|1⟩)/√2` | — |
| `pauli_x_alpha` | Bit-flip `X: |0⟩↔|1⟩` | — |
| `phase_gate` | `R(θ): |1⟩ → eⁱᶿ|1⟩` | — |
| `cnot_target_alpha` | Controlled-NOT entanglement | — |
| `heisenberg_satisfied` | `ΔxΔp ≥ ℏ/2` | Heisenberg (1927) |
| `energy_eigenvalue` | `Ê|ψ⟩ = E|ψ⟩` | Schrödinger (1926) |
| `time_evolution_phase` | `e^(-iEt/ℏ)` phase | — |
| `von_neumann_entropy` | `S(ρ) = -Tr(ρ log ρ)` | von Neumann (1932) |
| `fidelity` | `F(ψ,φ) = |⟨ψ|φ⟩|²` | Nielsen & Chuang (2000) |

### Stores
- `QuantumRegister :: KeyValue` — basis state index → real amplitude component

---

## Adding a new stdlib module

1. Create `src/stdlib/<name>_stdlib.loom` — write the module in pure Loom.
2. Add `pub const X_STDLIB: &str = include_str!("<name>_stdlib.loom");` to `src/stdlib/mod.rs`.
3. Write tests in `tests/<milestone>_test.rs` covering: parse, key symbols present, representative types, stores, annotations.
4. Run `cargo test --test <milestone>_test`.

No compiler changes required. If a new Loom primitive is needed to express the domain (e.g. a new store kind), add it to the AST + parser + emitter — then the stdlib is the first consumer.
