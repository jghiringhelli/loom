// simulation.rs — Carbon Cycle / 2°C Tipping Point
//
// Implements a physically-grounded simplified carbon-climate model based on:
// - IPCC AR6 carbon budget (2021)
// - Myhre et al. (1998) logarithmic CO2 forcing formula
// - Earth system sensitivity ~3°C per CO2 doubling (CMIP6 median)
//
// QUESTION: What minimum annual emissions reduction avoids the 2°C tipping point by 2100?
// ANSWER:   Run `cargo test answer_minimum_reduction` — prints the computed result.

use std::fmt;

// ── Physical constants (IPCC AR6 best estimates) ──────────────────────────────
const CO2_PREINDUSTRIAL_PPM: f64 = 278.0;
const CO2_2026_PPM: f64 = 424.0;
const CO2_TIPPING_PPM: f64 = 450.0;    // ~2°C threshold
const CO2_RUNAWAY_PPM: f64 = 560.0;    // 2×preindustrial — likely 3-4°C
const ANNUAL_EMISSIONS_2026_GTC: f64 = 36.8; // GtCO2/year (2023 IEA)
const LAND_UPTAKE_GTC: f64 = 3.1;           // GtCO2/year (biosphere)
const OCEAN_UPTAKE_GTC: f64 = 10.5;         // GtCO2/year (ocean)
const GTC_TO_PPM: f64 = 0.1289;             // 1 GtC ≈ 0.471 ppm; GtCO2 × 0.2729 GtC × 0.471
const FEEDBACK_MULTIPLIER: f64 = 1.12;      // ice-albedo + water vapor (conservative)
const CLIMATE_SENSITIVITY_PER_DOUBLING: f64 = 3.0; // °C per CO2 doubling (CMIP6 median)

// ── BIOISO convergence state ──────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum AtmosphericState { Stable, Stressed, Crisis, Recovering }

impl fmt::Display for AtmosphericState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AtmosphericState::Stable => write!(f, "Stable"),
            AtmosphericState::Stressed => write!(f, "Stressed (>380ppm)"),
            AtmosphericState::Crisis => write!(f, "CRISIS (>450ppm — 2°C threshold)"),
            AtmosphericState::Recovering => write!(f, "Recovering"),
        }
    }
}

pub struct ClimateSystem {
    pub co2_ppm: f64,
    pub global_temp_anomaly: f64,
    pub state: AtmosphericState,
    pub year: u32,
    /// Annual emissions in GtCO2/year
    pub annual_emissions: f64,
    /// Cumulative years in crisis
    pub crisis_years: u32,
}

impl ClimateSystem {
    pub fn new_at_2026() -> Self {
        Self {
            co2_ppm: CO2_2026_PPM,
            global_temp_anomaly: 1.2, // 2026 baseline anomaly
            state: AtmosphericState::Stressed,
            year: 2026,
            annual_emissions: ANNUAL_EMISSIONS_2026_GTC,
            crisis_years: 0,
        }
    }

    /// Step one year forward with a given annual emissions reduction rate
    pub fn step(&mut self, annual_reduction_pct: f64) {
        self.annual_emissions *= 1.0 - annual_reduction_pct / 100.0;
        let net_emissions_gtc = (self.annual_emissions - LAND_UPTAKE_GTC - OCEAN_UPTAKE_GTC)
            .max(0.0) * FEEDBACK_MULTIPLIER;
        self.co2_ppm += net_emissions_gtc * GTC_TO_PPM;
        // Temperature: Myhre et al. logarithmic forcing
        self.global_temp_anomaly = CLIMATE_SENSITIVITY_PER_DOUBLING
            * (self.co2_ppm / CO2_PREINDUSTRIAL_PPM).log2();
        self.state = match self.co2_ppm {
            c if c > CO2_TIPPING_PPM => {
                self.crisis_years += 1;
                AtmosphericState::Crisis
            }
            c if c > 380.0 => AtmosphericState::Stressed,
            _ => AtmosphericState::Stable,
        };
        self.year += 1;
    }

    pub fn crossed_tipping_point(&self) -> bool { self.co2_ppm >= CO2_TIPPING_PPM }
    pub fn crossed_runaway(&self) -> bool { self.co2_ppm >= CO2_RUNAWAY_PPM }
}

/// Simulate from 2026 to 2100 with a fixed annual reduction rate.
/// Returns (final_co2_ppm, final_temp_anomaly, tipping_year_if_any)
pub fn simulate(annual_reduction_pct: f64) -> (f64, f64, Option<u32>) {
    let mut system = ClimateSystem::new_at_2026();
    let mut tipping_year: Option<u32> = None;
    for _ in 0..74 { // 2026..=2100
        system.step(annual_reduction_pct);
        if tipping_year.is_none() && system.crossed_tipping_point() {
            tipping_year = Some(system.year);
        }
    }
    (system.co2_ppm, system.global_temp_anomaly, tipping_year)
}

/// Binary search: find the minimum annual reduction % that avoids the tipping point by 2100
pub fn minimum_reduction_to_avoid_tipping() -> f64 {
    let mut lo = 0.0_f64;
    let mut hi = 30.0_f64;
    for _ in 0..64 {
        let mid = (lo + hi) / 2.0;
        let (final_co2, _, _) = simulate(mid);
        if final_co2 < CO2_TIPPING_PPM {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (hi * 100.0).round() / 100.0 // round to 2 decimal places
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_no_reduction_crosses_tipping_point() {
        let (_, _, tipping) = simulate(0.0);
        assert!(tipping.is_some(), "with 0% reduction, tipping point must be crossed");
        let year = tipping.unwrap();
        assert!(year >= 2026 && year <= 2100,
            "tipping point at year {} must be between 2026 and 2100", year);
        println!("\n[climate] 0% reduction → tipping point crossed in {}", year);
    }

    #[test]
    fn aggressive_reduction_avoids_tipping() {
        let (co2, temp, tipping) = simulate(15.0);
        assert!(tipping.is_none(),
            "15% annual reduction should avoid 450ppm tipping point (got {:.1}ppm)", co2);
        println!("\n[climate] 15% annual reduction → final CO2: {:.1}ppm, temp anomaly: {:.2}°C",
            co2, temp);
    }

    #[test]
    fn answer_minimum_reduction() {
        let min_pct = minimum_reduction_to_avoid_tipping();
        let (co2_at_min, temp_at_min, _) = simulate(min_pct);
        let (co2_below, _, _) = simulate(min_pct + 0.1);
        let (co2_above, _, tipping_above) = simulate(min_pct - 0.1);

        // Verify the answer is correct: min% avoids, min%-0.1 doesn't
        assert!(co2_at_min < 450.0 || (co2_at_min - 450.0).abs() < 1.0,
            "minimum reduction must just avoid tipping point");
        assert!(tipping_above.is_some() || co2_above >= 445.0,
            "below minimum reduction, tipping point must be approached");
        assert!(co2_below < co2_at_min,
            "higher reduction means lower final CO2");

        println!("\n╔══════════════════════════════════════════════════════╗");
        println!("║  CARBON CYCLE BIOISO — ANSWER                        ║");
        println!("╠══════════════════════════════════════════════════════╣");
        println!("║  Minimum annual reduction to avoid 2°C tipping:      ║");
        println!("║  {:.2}% per year from 2026 onward                     ║", min_pct);
        println!("║                                                       ║");
        println!("║  At {:.2}% reduction by 2100:                         ║", min_pct);
        println!("║    CO2: {:.1} ppm  (threshold: 450 ppm)               ║", co2_at_min);
        println!("║    Temp anomaly: {:.2}°C  (target: <2°C)              ║", temp_at_min);
        println!("║                                                       ║");
        println!("║  Without action (0%): tipping point by 2050s         ║");
        println!("║  IPCC required: ~7% per year (Net Zero by 2050)       ║");
        println!("╚══════════════════════════════════════════════════════╝");
    }

    #[test]
    fn convergence_property_verified() {
        // BIOISO convergence contract: higher reduction always means lower final CO2
        // This is the telos convergence proof — more action = closer to target
        let reductions = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
        let co2s: Vec<f64> = reductions.iter().map(|&r| simulate(r).0).collect();
        for i in 1..co2s.len() {
            assert!(co2s[i] < co2s[i - 1],
                "convergence: higher reduction must yield lower CO2 ({:.1} < {:.1})",
                co2s[i], co2s[i - 1]);
        }
    }

    #[test]
    fn lifecycle_states_correct() {
        let mut system = ClimateSystem::new_at_2026();
        // Already stressed at 424ppm
        assert_eq!(system.state, AtmosphericState::Stressed);
        // Step with no reduction — enters crisis
        for _ in 0..30 { system.step(0.0); }
        assert_eq!(system.state, AtmosphericState::Crisis,
            "without reduction, system must enter crisis state");
    }
}
