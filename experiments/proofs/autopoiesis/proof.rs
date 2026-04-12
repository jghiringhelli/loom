// proof.rs — emitted by: loom compile proof.loom
// Theory: Autopoiesis (Maturana & Varela 1972)
// A BIOISO being with autopoietic:true emits a Rust struct with:
//   - A boundary (matter fields)
//   - Self-regulation (homeostatic trigger/action pairs)
//   - Self-production capability (propagate or repair)
//   - Finite lifespan (telomere counter)

/// Autopoietic being: self-bounded, self-produced, self-regulated.
#[derive(Debug, Clone)]
pub struct MinimalCell {
    pub membrane_integrity: f64,
    pub energy_reserve: f64,
    pub generation: u32,
}

impl MinimalCell {
    pub fn new(membrane_integrity: f64, energy_reserve: f64) -> Self {
        debug_assert!(membrane_integrity >= 0.0 && membrane_integrity <= 1.0);
        debug_assert!(energy_reserve >= 0.0 && energy_reserve <= 1.0);
        Self { membrane_integrity, energy_reserve, generation: 0 }
    }

    // ── Condition 3: self-regulation (homeostatic loops) ─────────────────────

    /// Regulation loop 1: low energy triggers harvesting
    pub fn regulate_energy(&mut self) {
        if self.energy_reserve < 0.2 {
            self.harvest_energy();
        }
    }

    /// Regulation loop 2: damaged membrane triggers repair
    pub fn regulate_membrane(&mut self) {
        if self.membrane_integrity < 0.4 {
            self.repair_membrane();
        }
    }

    fn harvest_energy(&mut self) {
        self.energy_reserve = (self.energy_reserve + 0.3).min(1.0);
    }

    fn repair_membrane(&mut self) {
        self.membrane_integrity = (self.membrane_integrity + 0.2).min(1.0);
    }

    // ── Condition 2: self-production ─────────────────────────────────────────

    /// Produce offspring if viable. Returns Some(child) if propagation succeeds.
    pub fn propagate(&self) -> Option<MinimalCell> {
        if self.membrane_integrity > 0.8 && self.energy_reserve > 0.7 {
            Some(MinimalCell::new(
                self.membrane_integrity * 0.95,
                self.energy_reserve * 0.5,
            ))
        } else {
            None
        }
    }

    // ── Telomere: finite lifespan ─────────────────────────────────────────────

    pub fn is_senescent(&self) -> bool {
        self.generation >= 500 || self.membrane_integrity < 0.05
    }

    pub fn tick(&mut self) {
        self.generation += 1;
        // Natural decay
        self.energy_reserve = (self.energy_reserve - 0.01).max(0.0);
        self.membrane_integrity = (self.membrane_integrity - 0.005).max(0.0);
        // Self-regulate
        self.regulate_energy();
        self.regulate_membrane();
    }
}

pub fn survival_score(cell: &MinimalCell) -> f64 {
    debug_assert!(cell.membrane_integrity >= 0.0);
    debug_assert!(cell.energy_reserve >= 0.0);
    let result = cell.membrane_integrity * 0.6 + cell.energy_reserve * 0.4;
    debug_assert!(result >= 0.0);
    debug_assert!(result <= 1.0);
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autopoiesis_condition_1_boundary_exists() {
        let cell = MinimalCell::new(0.9, 0.8);
        assert!(cell.membrane_integrity > 0.0, "Boundary must exist");
    }

    #[test]
    fn autopoiesis_condition_2_self_production() {
        let cell = MinimalCell::new(0.9, 0.9);
        let offspring = cell.propagate();
        assert!(offspring.is_some(), "Viable cell must be able to self-produce");
    }

    #[test]
    fn autopoiesis_condition_3_self_regulation() {
        let mut cell = MinimalCell::new(0.9, 0.1); // low energy
        cell.regulate_energy();
        assert!(cell.energy_reserve > 0.1, "Regulation must restore energy");
    }

    #[test]
    fn telomere_limits_lifespan() {
        let mut cell = MinimalCell::new(0.9, 0.9);
        cell.generation = 500;
        assert!(cell.is_senescent(), "Cell must senesce at telomere limit");
    }

    #[test]
    fn survival_score_bounded_0_to_1() {
        let cell = MinimalCell::new(0.7, 0.6);
        let score = survival_score(&cell);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn cell_maintains_viability_through_regulation() {
        let mut cell = MinimalCell::new(0.9, 0.9);
        for _ in 0..50 {
            cell.tick();
        }
        // After 50 ticks with regulation, cell should still be viable
        assert!(survival_score(&cell) > 0.3, "Autopoietic cell must maintain itself");
    }
}
