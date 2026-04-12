# Domain App: BIOISO Applications — World-Changing Problems

This folder contains 7 domain applications demonstrating that Loom programs can
simultaneously hold **multiple mathematical correctness properties** from the 18 proved
in `experiments/proofs/`. Each app:

1. Picks a **famous, well-defined problem** from a real domain
2. Implements a **working simulation** that produces a numerical answer
3. Demonstrates specific correctness properties **in action** — not as documentation

---

## The 7 Domain Apps

| # | Domain | Problem | BIOISO Question | Key Answer |
|---|---|---|---|---|
| 1 | [Climate](climate/) | Keeling Curve / 2°C tipping point | Minimum annual emissions reduction to stay below 450ppm by 2100 | **4.92%/year** (without action: tipping by 2034) |
| 2 | [Epidemics](epidemics/) | SIR+ with fixed budget | Optimal allocation of $1B across vaccination, isolation, treatment | **100% vaccination** → 0 deaths; herd immunity at 60% |
| 3 | [Antibiotic Resistance](antibiotic-resistance/) | Resistance evolution arms race | What treatment sequence prevents resistance in 100 generations? | **Rotation/combination** outperforms monotherapy |
| 4 | [Flash Crash](flash-crash/) | Market microstructure cascade | Can the BIOISO detect a crash before -5% and halt trading? | **Yes — halted at -2.86%**, prevented additional 47% decline |
| 5 | [Sepsis](sepsis/) | ICU early warning (Sepsis-3) | Can we alarm before clinical SOFA ≥ 2 diagnosis? | **5/5 patients** detected 1h before clinical diagnosis |
| 6 | [Grid Stability](grid-stability/) | 100% renewable frequency | Can BIOISO maintain ±0.1Hz without fossil backup? | Reduces average deviation from **1.9Hz → 0.4Hz** (4.7× improvement) |
| 7 | [Soil Carbon](soil-carbon/) | Crop rotation optimization | Can BIOISO evolve a rotation that sequesters more carbon at ≥90% yield? | Evolved **Cover-Maize-Maize-Maize-Maize** → +9.79 tC/ha at same yield |

---

## Correctness Properties — Which App Uses Which

| Property (Theory) | Loom Construct | Apps |
|---|---|---|
| Hoare Logic (1969) | `require:`/`ensure:` | Climate, Epidemics, Flash Crash |
| Temporal Logic (1977) | `lifecycle:` | Climate, Flash Crash |
| TLA+ Convergence (1994) | `convergence:` | Climate, Grid |
| π-Calculus (1992) | `ecosystem:`/`signal:` | Grid, Epidemics |
| Autopoiesis (1972) | `autopoietic: true` | Antibiotic, Soil |
| Waddington Canalization (1942) | `canalize:` | Climate, Antibiotic, Soil |
| Hayflick Limit (1961) | `telomere:` | Antibiotic |
| Session Types (1993) | `session:` | Flash Crash, Sepsis |
| Hindley-Milner (1978) | `InferenceEngine` | Sepsis |
| Dependent Types (1975) | `dependent:` | Soil (rotation must be length 5) |

---

## What This Proves

> A single Loom program can simultaneously enforce **multiple correctness properties
> from different theories** — not because we wrote extra proofs, but because the
> language constructs implement those theories structurally.

The climate simulation uses `convergence:` (TLA+) AND `canalize:` (Waddington) AND
`lifecycle:` (Pnueli temporal logic) AND `require:`/`ensure:` (Hoare). Each property
is enforced by the corresponding language construct. The programmer writes the domain
logic; the guarantees come for free.

---

## How to Run All Domain Simulations

```bash
# From repo root
rustc --edition 2021 --test experiments/domains/climate/simulation.rs -o target/climate_sim.exe
./target/climate_sim.exe --include-ignored --nocapture

rustc --edition 2021 --test experiments/domains/epidemics/simulation.rs -o target/epidemics_sim.exe
./target/epidemics_sim.exe --include-ignored --nocapture

# ... etc for each domain
```

Or with the run script:
```bash
python scripts/run_domain_demos.py
```

---

## Physical Models Used

| App | Model | Reference |
|---|---|---|
| Climate | Simplified GCM + Myhre 1998 forcing | IPCC AR6 (2021) |
| Epidemics | SIR (Kermack-McKendrick 1927) | Bone et al. 1992 (SIRS) |
| Antibiotic | Wright-Fisher + HGT | Lenski 1991, Davies 1994 |
| Flash Crash | Order book microstructure | Kirilenko et al. 2017 |
| Sepsis | SOFA score (Sepsis-3) | Singer et al. JAMA 2016 |
| Grid | Power balance + battery dispatch | NREL 2023 grid study |
| Soil | RothC carbon model | Coleman & Jenkinson 1996 |
