// proof.rs — emitted by: loom compile proof.loom
// Theory: Hayflick Limit (Hayflick & Moorhead 1961)
// @mortal beings have a telomere counter that triggers senescence.
// Division is only possible while telomere_length > threshold.

#[derive(Debug, Clone)]
pub struct HayflickCell {
    pub dna_damage: f64,
    pub telomere_length: f64,
    pub metabolic_rate: f64,
    pub generation: u32,
}

/// Maximum divisions before senescence (Hayflick number for human somatic cells).
pub const HAYFLICK_LIMIT: u32 = 50;

impl HayflickCell {
    pub fn new() -> Self {
        Self {
            dna_damage: 0.0,
            telomere_length: 1.0,
            metabolic_rate: 1.0,
            generation: 0,
        }
    }

    /// Is the cell senescent? (telomere exhausted or Hayflick limit reached)
    pub fn is_senescent(&self) -> bool {
        self.telomere_length < 0.05 || self.generation >= HAYFLICK_LIMIT
    }

    /// Attempt division. Returns offspring if viable, None if senescent.
    pub fn divide(&mut self) -> Option<HayflickCell> {
        if self.is_senescent() {
            return None;
        }
        if self.telomere_length <= 0.1 || self.dna_damage >= 0.8 {
            return None;
        }
        // Telomere shortens with each division
        self.telomere_length -= 0.02;
        self.dna_damage += 0.005;
        self.generation += 1;

        Some(HayflickCell {
            dna_damage: self.dna_damage + 0.005,
            telomere_length: self.telomere_length,
            metabolic_rate: self.metabolic_rate * 0.99,
            generation: self.generation,
        })
    }

    /// DNA repair: only available while telomeres are long enough
    pub fn regulate(&mut self) {
        if self.dna_damage > 0.3 && self.telomere_length > 0.2 {
            self.dna_damage = (self.dna_damage - 0.1).max(0.0);
        }
    }
}

pub fn replication_fitness(cell: &HayflickCell) -> f64 {
    debug_assert!(cell.telomere_length >= 0.0);
    debug_assert!(cell.dna_damage >= 0.0);
    let result = cell.telomere_length * 0.7 + (1.0 - cell.dna_damage) * 0.3;
    debug_assert!(result >= 0.0);
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_divides_while_viable() {
        let mut cell = HayflickCell::new();
        let offspring = cell.divide();
        assert!(offspring.is_some(), "Healthy cell must be able to divide");
        assert_eq!(cell.generation, 1);
    }

    #[test]
    fn telomere_shortens_each_division() {
        let mut cell = HayflickCell::new();
        let initial = cell.telomere_length;
        cell.divide();
        assert!(cell.telomere_length < initial, "Telomere must shorten on division");
    }

    #[test]
    fn hayflick_limit_stops_division() {
        let mut cell = HayflickCell::new();
        cell.generation = HAYFLICK_LIMIT;
        assert!(cell.is_senescent(), "Cell must senesce at Hayflick limit");
        let offspring = cell.divide();
        assert!(offspring.is_none(), "Senescent cell cannot divide");
    }

    #[test]
    fn cell_reaches_senescence_through_division() {
        let mut cell = HayflickCell::new();
        let mut divisions = 0;
        while cell.divide().is_some() {
            divisions += 1;
            if divisions > 100 { break; } // safety
        }
        assert!(
            divisions <= HAYFLICK_LIMIT as usize,
            "Divisions ({}) must not exceed Hayflick limit ({})", divisions, HAYFLICK_LIMIT
        );
    }

    #[test]
    fn fitness_declines_with_age() {
        let young = HayflickCell::new();
        let mut old = HayflickCell::new();
        old.telomere_length = 0.1;
        old.dna_damage = 0.6;
        assert!(
            replication_fitness(&young) > replication_fitness(&old),
            "Young cell must have higher fitness than old cell"
        );
    }
}
