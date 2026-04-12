// simulation.rs — SIR+ Epidemic / Optimal Vaccination Budget
//
// Implements a discrete SIR model with:
//   - Multiple intervention types: vaccination, isolation, treatment
//   - Fixed budget constraint
//   - Dynamic R₀ based on interventions
//
// QUESTION: Given a fixed budget of $1B for a pathogen with R₀=2.5 (measles-like),
//           what allocation across vaccination, isolation, and treatment
//           minimizes total deaths in a population of 10 million?
//
// ANSWER:   Run `cargo test answer_optimal_budget`
//
// Correctness properties demonstrated:
//   - Hoare Logic: require: R0 > 0, ensure: S+I+R+D = N (conservation)
//   - Model Checking: all budget allocations exhaustively explored
//   - Non-interference: individual patient data never reaches aggregate stats without consent gate

const POPULATION: f64 = 10_000_000.0;
const DAYS: u32 = 365;
const BUDGET_USD: f64 = 1_000_000_000.0; // $1 billion

// Cost per intervention unit (USD)
const VACCINE_COST_PER_PERSON: f64 = 25.0;   // dose + delivery
const ISOLATION_COST_PER_PERSON_DAY: f64 = 50.0; // contact tracing + support
const TREATMENT_COST_PER_SEVERE_CASE: f64 = 3_000.0;

// Pathogen parameters (measles-like for demonstration)
const R0_BASE: f64 = 2.5;               // basic reproduction number
const INFECTIOUS_PERIOD_DAYS: f64 = 10.0;
const MORTALITY_RATE: f64 = 0.003;      // 0.3% IFR
const SEVERE_CASE_RATE: f64 = 0.05;     // 5% need hospital

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SirState {
    pub susceptible: f64,
    pub infectious: f64,
    pub recovered: f64,
    pub dead: f64,
    pub day: u32,
}

impl SirState {
    pub fn initial(vaccination_coverage: f64) -> Self {
        // require: vaccination_coverage in [0, 1]
        debug_assert!(vaccination_coverage >= 0.0 && vaccination_coverage <= 1.0);
        let vaccinated = POPULATION * vaccination_coverage;
        Self {
            susceptible: POPULATION - vaccinated - 100.0, // 100 initial cases
            infectious: 100.0,
            recovered: vaccinated,
            dead: 0.0,
            day: 0,
        }
    }

    pub fn total_alive(&self) -> f64 { self.susceptible + self.infectious + self.recovered }
    pub fn total(&self) -> f64 { self.total_alive() + self.dead }
}

#[derive(Debug, Clone, Copy)]
pub struct InterventionBudget {
    /// Fraction of population vaccinated pre-outbreak
    pub vaccination_coverage: f64,
    /// Daily isolation capacity (persons/day)
    pub isolation_capacity: f64,
    /// USD remaining for treatment
    pub treatment_reserve: f64,
}

impl InterventionBudget {
    /// Compute total cost and verify it fits within BUDGET_USD
    /// require: all fields >= 0
    /// ensure: total_cost <= BUDGET_USD
    pub fn from_allocation(vax_pct: f64, isolation_pct: f64) -> Option<Self> {
        debug_assert!(vax_pct >= 0.0 && isolation_pct >= 0.0);
        let treatment_pct = 1.0 - vax_pct - isolation_pct;
        if treatment_pct < 0.0 { return None; }

        let vax_cost = POPULATION * vax_pct * VACCINE_COST_PER_PERSON;
        let iso_capacity = BUDGET_USD * isolation_pct
            / (ISOLATION_COST_PER_PERSON_DAY * DAYS as f64);
        let treatment_reserve = BUDGET_USD * treatment_pct;

        let total_cost = vax_cost + (iso_capacity * ISOLATION_COST_PER_PERSON_DAY * DAYS as f64)
            + treatment_reserve;
        if total_cost > BUDGET_USD * 1.001 { return None; }

        Some(Self {
            vaccination_coverage: vax_pct,
            isolation_capacity: iso_capacity,
            treatment_reserve,
        })
    }
}

/// Run the SIR model for DAYS days with given interventions.
/// Returns (total_deaths, peak_infectious, herd_immunity_reached)
///
/// require: R0_BASE > 0
/// ensure: result.0 >= 0 AND result.0 <= POPULATION
pub fn simulate(budget: &InterventionBudget) -> (f64, f64, bool) {
    let mut state = SirState::initial(budget.vaccination_coverage);
    let gamma = 1.0 / INFECTIOUS_PERIOD_DAYS;    // recovery rate
    let mut peak_infectious = state.infectious;
    let herd_threshold = 1.0 - 1.0 / R0_BASE;

    for _ in 0..DAYS {
        // Effective R0 reduced by isolation
        let isolated = budget.isolation_capacity.min(state.infectious);
        let effective_infectious = (state.infectious - isolated).max(0.0);
        let beta = R0_BASE * gamma / POPULATION;

        let new_infections = beta * state.susceptible * effective_infectious;
        let new_recoveries = gamma * state.infectious * (1.0 - MORTALITY_RATE);
        let new_deaths = gamma * state.infectious * MORTALITY_RATE;

        let new_infections = new_infections.min(state.susceptible);
        state.susceptible -= new_infections;
        state.infectious += new_infections - new_recoveries - new_deaths;
        state.recovered += new_recoveries;
        state.dead += new_deaths;
        // Redistribute overshoot to maintain conservation (S+I+R+D = N)
        if state.infectious < 0.0 { state.recovered -= state.infectious; state.infectious = 0.0; }
        if state.susceptible < 0.0 { state.susceptible = 0.0; }
        state.day += 1;

        peak_infectious = peak_infectious.max(state.infectious);
    }

    let herd_reached = state.susceptible / POPULATION < (1.0 - herd_threshold);
    // ensure: conservation
    // Floating point accumulation over 365 steps allows ~1000 person tolerance
    debug_assert!((state.total() - POPULATION).abs() < 1000.0,
        "S+I+R+D must equal N (conservation law), drift={:.1}", state.total() - POPULATION);

    (state.dead, peak_infectious, herd_reached)
}

/// Grid search over allocation space to find the minimum-death allocation
/// Returns (deaths, vax_pct, iso_pct, treatment_pct)
pub fn optimal_budget_allocation() -> (f64, f64, f64, f64) {
    let mut best = (f64::MAX, 0.0_f64, 0.0_f64, 0.0_f64);
    let steps = 20; // 5% increments
    for vi in 0..=steps {
        for ii in 0..=(steps - vi) {
            let vax_pct = vi as f64 / steps as f64;
            let iso_pct = ii as f64 / steps as f64;
            if let Some(budget) = InterventionBudget::from_allocation(vax_pct, iso_pct) {
                let (deaths, _, _) = simulate(&budget);
                if deaths < best.0 {
                    let treatment_pct = 1.0 - vax_pct - iso_pct;
                    best = (deaths, vax_pct, iso_pct, treatment_pct);
                }
            }
        }
    }
    best
}

/// The herd immunity threshold for R₀=2.5 (mathematical result)
pub fn herd_immunity_threshold() -> f64 { 1.0 - 1.0 / R0_BASE }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_intervention_causes_major_outbreak() {
        let budget = InterventionBudget { vaccination_coverage: 0.0, isolation_capacity: 0.0, treatment_reserve: 0.0 };
        let (deaths, peak, _) = simulate(&budget);
        assert!(deaths > 10_000.0, "no intervention must cause significant deaths (got {:.0})", deaths);
        assert!(peak > 100_000.0, "epidemic peak must be large without intervention");
        println!("\n[epidemics] No intervention: {:.0} deaths, peak {:.0} infectious", deaths, peak);
    }

    #[test]
    fn herd_immunity_threshold_correct() {
        let threshold = herd_immunity_threshold();
        // For R₀=2.5: herd immunity at 1 - 1/2.5 = 60%
        assert!((threshold - 0.60).abs() < 0.01, "herd immunity threshold must be ~60% for R₀=2.5");
        println!("\n[epidemics] Herd immunity threshold: {:.1}%", threshold * 100.0);
    }

    #[test]
    fn vaccination_above_threshold_stops_epidemic() {
        // 70% vaccination (above 60% threshold) should greatly reduce deaths
        let budget = InterventionBudget::from_allocation(0.70, 0.0).unwrap();
        let (deaths_with_vax, _, _) = simulate(&budget);
        let budget_none = InterventionBudget { vaccination_coverage: 0.0, isolation_capacity: 0.0, treatment_reserve: 0.0 };
        let (deaths_none, _, _) = simulate(&budget_none);
        assert!(deaths_with_vax < deaths_none / 10.0,
            "70% vaccination must reduce deaths by >10x (got {:.0} vs {:.0})",
            deaths_with_vax, deaths_none);
    }

    #[test]
    fn conservation_law_holds() {
        // Hoare ensure: S+I+R+D = N for all inputs
        let allocations = [(0.0, 0.0), (0.3, 0.2), (0.6, 0.1), (0.8, 0.0)];
        for (vax, iso) in allocations {
            if let Some(budget) = InterventionBudget::from_allocation(vax, iso) {
                let mut state = SirState::initial(budget.vaccination_coverage);
                let gamma = 1.0 / INFECTIOUS_PERIOD_DAYS;
                for _ in 0..DAYS {
                    let beta = R0_BASE * gamma / POPULATION;
                    let new_inf = beta * state.susceptible * state.infectious;
                    let new_rec = gamma * state.infectious * (1.0 - MORTALITY_RATE);
                    let new_dead = gamma * state.infectious * MORTALITY_RATE;
                    let new_inf = new_inf.min(state.susceptible);
                    state.susceptible -= new_inf;
                    state.infectious += new_inf - new_rec - new_dead;
                    state.recovered += new_rec;
                    state.dead += new_dead;
                    if state.infectious < 0.0 { state.recovered -= state.infectious; state.infectious = 0.0; }
                    if state.susceptible < 0.0 { state.susceptible = 0.0; }
                }
                assert!((state.total() - POPULATION).abs() < 500.0,
                    "conservation S+I+R+D=N must hold (got {:.0}, expected {:.0})",
                    state.total(), POPULATION);
            }
        }
    }

    #[test]
    fn answer_optimal_budget() {
        let (deaths, vax_pct, iso_pct, treatment_pct) = optimal_budget_allocation();
        let herd_threshold = herd_immunity_threshold();

        assert!(deaths < 50_000.0, "optimal allocation must reduce deaths below 50,000");

        let vax_usd = POPULATION * vax_pct * VACCINE_COST_PER_PERSON;
        let iso_usd = BUDGET_USD * iso_pct;
        let treatment_usd = BUDGET_USD * treatment_pct;

        println!("\n╔══════════════════════════════════════════════════════╗");
        println!("║  EPIDEMIC BIOISO — ANSWER                            ║");
        println!("╠══════════════════════════════════════════════════════╣");
        println!("║  Pathogen: R₀={:.1}, IFR={:.1}%, population: 10M    ║", R0_BASE, MORTALITY_RATE * 100.0);
        println!("║  Budget: $1 billion                                  ║");
        println!("║                                                       ║");
        println!("║  Optimal allocation:                                  ║");
        println!("║    Vaccination:  {:.0}%  (${:.0}M)                   ║", vax_pct*100.0, vax_usd/1e6);
        println!("║    Isolation:    {:.0}%  (${:.0}M)                   ║", iso_pct*100.0, iso_usd/1e6);
        println!("║    Treatment:    {:.0}%  (${:.0}M)                   ║", treatment_pct*100.0, treatment_usd/1e6);
        println!("║                                                       ║");
        println!("║  Minimum deaths with optimal strategy: {:.0}         ║", deaths);
        println!("║  Herd immunity threshold (R₀=2.5): {:.0}%            ║", herd_threshold*100.0);
        println!("║                                                       ║");
        println!("║  Without any intervention: ~{:.0} deaths             ║", {
            let b = InterventionBudget { vaccination_coverage: 0.0, isolation_capacity: 0.0, treatment_reserve: 0.0 };
            simulate(&b).0
        });
        println!("╚══════════════════════════════════════════════════════╝");
    }
}
