// proof.rs — emitted by: loom compile proof.loom
// Theory: Waddington Canalization (Waddington 1942)
// A canalizing system converges back to its developmental channel after perturbations.
// The canalize: block emits a regulate loop that enforces channel boundaries.

#[derive(Debug, Clone)]
pub struct CanalizingSystem {
    pub current_state: f64,
    pub perturbation: f64,
    pub resilience: f64,
    pub generation: u32,
}

/// The developmental channel: state must remain in [0.4, 0.6]
pub const CHANNEL_MIN: f64 = 0.4;
pub const CHANNEL_MAX: f64 = 0.6;
pub const CONVERGE_RATE: f64 = 0.3;

impl CanalizingSystem {
    pub fn new(initial_state: f64, resilience: f64) -> Self {
        Self {
            current_state: initial_state.clamp(0.0, 1.0),
            perturbation: 0.0,
            resilience,
            generation: 0,
        }
    }

    pub fn is_in_channel(&self) -> bool {
        self.current_state >= CHANNEL_MIN && self.current_state <= CHANNEL_MAX
    }

    /// Apply external perturbation (environmental disturbance)
    pub fn perturb(&mut self, magnitude: f64) {
        self.perturbation = magnitude;
        self.current_state = (self.current_state + magnitude).clamp(0.0, 1.0);
    }

    /// Canalization regulation: return to channel
    pub fn regulate(&mut self) {
        if !self.is_in_channel() {
            let target = 0.5; // channel center
            let correction = (target - self.current_state) * CONVERGE_RATE * self.resilience;
            self.current_state = (self.current_state + correction).clamp(0.0, 1.0);
        }
        // Epigenetic: increase resilience under persistent perturbation
        if self.perturbation > 0.1 {
            self.resilience = (self.resilience + 0.05).min(1.0);
        } else {
            self.perturbation = 0.0; // revert epigenetic change
        }
        self.generation += 1;
    }

    pub fn tick(&mut self) {
        self.regulate();
    }
}

pub fn channel_fitness(sys: &CanalizingSystem) -> f64 {
    debug_assert!(sys.current_state >= 0.0 && sys.current_state <= 1.0);
    let result = 1.0 - (sys.current_state - 0.5).abs();
    debug_assert!(result >= 0.0);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_starts_in_channel() {
        let sys = CanalizingSystem::new(0.5, 0.8);
        assert!(sys.is_in_channel());
    }

    #[test]
    fn perturbation_knocks_out_of_channel() {
        let mut sys = CanalizingSystem::new(0.5, 0.8);
        sys.perturb(0.3); // pushes to 0.8
        assert!(!sys.is_in_channel(), "large perturbation must knock system out of channel");
    }

    #[test]
    fn canalization_restores_channel_after_perturbation() {
        let mut sys = CanalizingSystem::new(0.5, 0.8);
        sys.perturb(0.3);
        assert!(!sys.is_in_channel());
        // Regulate repeatedly until back in channel
        for _ in 0..20 {
            sys.tick();
            if sys.is_in_channel() { break; }
        }
        assert!(sys.is_in_channel(), "canalization must restore system to channel");
    }

    #[test]
    fn fitness_higher_in_channel_than_out() {
        let in_channel = CanalizingSystem::new(0.5, 0.8);
        let out_of_channel = CanalizingSystem::new(0.9, 0.8);
        assert!(
            channel_fitness(&in_channel) > channel_fitness(&out_of_channel),
            "fitness must be higher inside the developmental channel"
        );
    }

    #[test]
    fn resilience_increases_under_persistent_perturbation() {
        let mut sys = CanalizingSystem::new(0.5, 0.5);
        let initial_resilience = sys.resilience;
        sys.perturbation = 0.5; // persistent perturbation
        sys.regulate();
        assert!(
            sys.resilience > initial_resilience,
            "epigenetic: resilience must increase under persistent perturbation"
        );
    }
}
