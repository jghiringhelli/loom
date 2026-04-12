// proof.rs — emitted by: loom compile proof.loom
// Theory: TLA+ / Convergence (Lamport 1994)
// convergence: emits a ConvergenceState tracker.
// Each step must decrease the distance to the target (progress property).

#[derive(Debug, Clone, PartialEq)]
pub enum ConvergenceState {
    Converging,
    Converged,
    Diverging,
    Alarm,
}

pub struct ConvergenceTracker {
    pub state: ConvergenceState,
    pub steps_without_progress: u32,
    pub alarm_threshold: u32,
}

impl ConvergenceTracker {
    pub fn new(alarm_threshold: u32) -> Self {
        Self { state: ConvergenceState::Converging, steps_without_progress: 0, alarm_threshold }
    }

    pub fn update(&mut self, prev_distance: f64, new_distance: f64) {
        if new_distance == 0.0 {
            self.state = ConvergenceState::Converged;
            self.steps_without_progress = 0;
        } else if new_distance < prev_distance {
            self.state = ConvergenceState::Converging;
            self.steps_without_progress = 0;
        } else {
            self.steps_without_progress += 1;
            if self.steps_without_progress >= self.alarm_threshold {
                self.state = ConvergenceState::Alarm;
            } else {
                self.state = ConvergenceState::Diverging;
            }
        }
    }
}

pub fn distributed_step(current: i64, target_value: i64) -> i64 {
    debug_assert!(current >= 0, "require: current >= 0");
    let result = if current < target_value { current + 1 }
                 else if current > target_value { current - 1 }
                 else { current };
    let prev_dist = (current - target_value).abs();
    let new_dist = (result - target_value).abs();
    debug_assert!(new_dist <= prev_dist, "ensure: distance to target non-increasing");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_tracker_detects_progress() {
        let mut tracker = ConvergenceTracker::new(100);
        tracker.update(10.0, 9.0);
        assert_eq!(tracker.state, ConvergenceState::Converging);
    }

    #[test]
    fn convergence_tracker_detects_arrival() {
        let mut tracker = ConvergenceTracker::new(100);
        tracker.update(1.0, 0.0);
        assert_eq!(tracker.state, ConvergenceState::Converged);
    }

    #[test]
    fn convergence_tracker_fires_alarm() {
        let mut tracker = ConvergenceTracker::new(3);
        for _ in 0..3 { tracker.update(5.0, 6.0); } // no progress 3 times
        assert_eq!(tracker.state, ConvergenceState::Alarm);
    }

    #[test]
    fn distributed_step_always_reduces_distance() {
        for current in 0i64..=20 {
            let target = 10i64;
            let next = distributed_step(current, target);
            let dist_before = (current - target).abs();
            let dist_after = (next - target).abs();
            assert!(dist_after <= dist_before,
                "convergence: step must not increase distance (current={}, target={})", current, target);
        }
    }

    #[test]
    fn distributed_step_converges_to_target() {
        let mut current = 0i64;
        let target = 10i64;
        let mut steps = 0;
        while current != target {
            current = distributed_step(current, target);
            steps += 1;
            assert!(steps <= 20, "must converge within 20 steps");
        }
        assert_eq!(current, target);
    }
}
