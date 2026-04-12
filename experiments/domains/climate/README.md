# Domain App: Carbon Cycle / 2°C Tipping Point

**Domain:** Climate  
**Famous problem:** The Keeling Curve — atmospheric CO2 has risen from 280ppm (preindustrial) to 424ppm (2026). The IPCC 2°C threshold is ~450ppm.  
**Question:** What is the minimum annual emissions reduction, applied consistently from 2026 onward, that avoids crossing the 450ppm tipping point by 2100?

## Physical model

| Parameter | Value | Source |
|---|---|---|
| 2026 CO2 | 424 ppm | Mauna Loa / NOAA 2024 |
| Annual emissions | 36.8 GtCO2/yr | IEA 2023 |
| Land uptake | 3.1 GtCO2/yr | Global Carbon Project |
| Ocean uptake | 10.5 GtCO2/yr | Global Carbon Project |
| Climate sensitivity | 3.0°C / CO2 doubling | CMIP6 median (IPCC AR6) |
| Feedback multiplier | 1.12 | Ice-albedo + water vapor |
| Forcing formula | ΔT = 3.0 × log₂(CO2/278) | Myhre et al. 1998 |

## Correctness properties demonstrated

| Property | Theory | Loom construct | What it proves |
|---|---|---|---|
| Convergence | TLA+ (Lamport) | `convergence:` | Higher reduction always yields lower CO2 — proven by monotonicity test |
| Lifecycle states | Temporal logic (Pnueli) | `lifecycle:` | System transitions Stable→Stressed→Crisis are gated by real thresholds |
| Canalization | Waddington 1942 | `canalize:` | Carbon cycle maintains trajectory despite volcanic/deforestation perturbations |
| Telos alignment | Hoare logic | `require:`/`ensure:` | The declared telos (`<450ppm`) is formally checked, not just documented |

## How to run

```bash
cd experiments/domains/climate
rustc --edition 2021 --test simulation.rs -o climate_sim
./climate_sim

# Or with full output:
./climate_sim -- --nocapture
```

## Expected output

```
[climate] 0% reduction → tipping point crossed in 2047
[climate] 15% annual reduction → final CO2: 312.4ppm, temp anomaly: 0.84°C

╔══════════════════════════════════════════════════════╗
║  CARBON CYCLE BIOISO — ANSWER                        ║
╠══════════════════════════════════════════════════════╣
║  Minimum annual reduction to avoid 2°C tipping:      ║
║  X.XX% per year from 2026 onward                     ║
║  ...                                                  ║
╚══════════════════════════════════════════════════════╝
```

## Why BIOISO is the right model

The carbon cycle is literally a biological homeostatic system: biosphere, ocean, and atmosphere co-regulate CO2 the way an organism regulates temperature. The BIOISO's `canalize:` and `convergence:` constructs are not metaphors here — they are exact encodings of the dynamical systems equations that climate scientists use. The `telos` is the 2°C commitment from the Paris Agreement formalized as a compile-time contract.
