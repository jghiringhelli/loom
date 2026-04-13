//! MutationSampler — biased stochastic delta generator for mitotic parameter mutation.
//!
//! When a single entity mutates its own parameters (mitosis), the delta applied to
//! each parameter must balance two forces:
//!
//! - **Guidance force**: Telos as an attractor — pushes the parameter toward its
//!   declared target, proportional to how far we are drifting.
//! - **Stochastic noise**: Controlled randomness — maintains diversity and prevents
//!   premature convergence to local optima.
//!
//! These two forces are combined as:
//!
//! ```text
//! delta = guidance_force + noise(distribution, temperature)
//! ```
//!
//! where `temperature = σ_base × relative_telomere_length` — young entities explore
//! widely, old entities exploit conservatively (simulated annealing for free).
//!
//! # Distributions
//!
//! | Mode | Distribution | Biological analog |
//! |------|-------------|-------------------|
//! | `Gaussian` | N(0, σ) | Small neutral mutations |
//! | `Cauchy` | Cauchy(0, γ) | Heavy-tailed; occasional large jumps |
//! | `Levy` | Lévy-stable (β=1.5) | Punctuated equilibrium |
//! | `Adaptive` | Selected dynamically | Context-sensitive; default |
//!
//! # Adaptive mode
//!
//! In `Adaptive` mode the sampler tracks a rolling acceptance window (survival scores
//! from Stage 5 Simulation). When the acceptance rate drops, σ is halved to become
//! more conservative. When rate is high, σ expands to explore. The Lévy mode is
//! engaged when the entity is stuck (low acceptance despite high drift).
//!
//! # Pure-Rust PRNG
//!
//! No external dependencies. Uses xorshift64 seeded from a provided seed (typically
//! derived from `now_ms() ^ entity_id.hash()`). Box-Muller transform for Gaussian;
//! tan(π(U−0.5)) for Cauchy; Mantegna algorithm for Lévy-stable (β=1.5).
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md) §Sampler.

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default base standard deviation for the stochastic component.
pub const DEFAULT_SIGMA_BASE: f64 = 0.05;

/// Default learning rate for the guidance force.
pub const DEFAULT_LEARNING_RATE: f64 = 0.4;

/// Probability of engaging Lévy flight in Adaptive mode when stuck.
pub const LEVY_STUCK_PROBABILITY: f64 = 0.15;

/// Adaptive mode: expand σ when acceptance rate exceeds this.
pub const ACCEPTANCE_HIGH_THRESHOLD: f64 = 0.75;

/// Adaptive mode: shrink σ when acceptance rate drops below this.
pub const ACCEPTANCE_LOW_THRESHOLD: f64 = 0.25;

/// Acceptance tracking window size.
pub const ACCEPTANCE_WINDOW: usize = 20;

/// Lévy-stable exponent β = 1.5 (Mantegna algorithm pre-computed σ_u).
///
/// σ_u = [Γ(1+β)·sin(πβ/2) / (Γ((1+β)/2)·β·2^((β-1)/2))]^(1/β)
/// For β=1.5: σ_u ≈ 0.7082
const LEVY_SIGMA_U: f64 = 0.7082;
/// 1/β for Lévy (β=1.5).
const LEVY_INV_BETA: f64 = 1.0 / 1.5;

// ── Sampling mode ─────────────────────────────────────────────────────────────

/// Which distribution the stochastic component draws from.
#[derive(Debug, Clone, PartialEq)]
pub enum SamplingMode {
    /// Small neutral mutations; default exploration near the current value.
    Gaussian,
    /// Heavy-tailed; occasional large jumps without Lévy overhead.
    Cauchy,
    /// Lévy-stable (β=1.5) — punctuated equilibrium; rare but very large steps.
    Levy,
    /// Context-sensitive: selects Gaussian normally, escalates to Lévy when stuck.
    Adaptive,
}

// ── PRNG (xorshift64) ─────────────────────────────────────────────────────────

/// Minimal xorshift64 PRNG — zero external dependencies, deterministic from seed.
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        // Seed must be non-zero.
        Self { state: if seed == 0 { 0xDEAD_BEEF_CAFE_1337 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform sample in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal N(0,1) via Box-Muller transform.
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Standard Cauchy via inverse CDF: tan(π(U − 0.5)).
    fn next_cauchy(&mut self) -> f64 {
        let u = self.next_f64();
        (std::f64::consts::PI * (u - 0.5)).tan()
    }

    /// Lévy-stable sample via Mantegna algorithm (β=1.5).
    ///
    /// step = σ_u·N(0,1) / |N(0,1)|^(1/β)
    fn next_levy(&mut self) -> f64 {
        let u = self.next_normal() * LEVY_SIGMA_U;
        let v = self.next_normal().abs().max(1e-15);
        u / v.powf(LEVY_INV_BETA)
    }
}

// ── Acceptance tracker ────────────────────────────────────────────────────────

/// Rolling acceptance rate tracker (survival scores from Stage 5 Simulation).
///
/// Drives adaptive σ adjustment: high acceptance → expand, low → shrink.
#[derive(Debug, Clone)]
pub struct AcceptanceTracker {
    window: VecDeque<bool>,
    capacity: usize,
    /// Number of accepted proposals in the current window.
    accepted: usize,
}

impl AcceptanceTracker {
    pub fn new(capacity: usize) -> Self {
        Self { window: VecDeque::with_capacity(capacity), capacity, accepted: 0 }
    }

    /// Record a simulation result.
    ///
    /// `passed` is `true` if Stage 5 survival_score ≥ threshold.
    pub fn record(&mut self, passed: bool) {
        if self.window.len() == self.capacity {
            if let Some(evicted) = self.window.pop_front() {
                if evicted {
                    self.accepted -= 1;
                }
            }
        }
        self.window.push_back(passed);
        if passed {
            self.accepted += 1;
        }
    }

    /// Rolling acceptance rate [0, 1]. Returns `None` when the window is empty.
    pub fn rate(&self) -> Option<f64> {
        if self.window.is_empty() {
            return None;
        }
        Some(self.accepted as f64 / self.window.len() as f64)
    }

    /// Number of outcomes recorded so far.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }
}

// ── MutationSampler ───────────────────────────────────────────────────────────

/// Biased stochastic delta generator for mitotic parameter mutation.
///
/// Produces the `delta` field of a [`MutationProposal::ParameterAdjust`].
/// Consumed by Polycephalum (Tier 1) and the Meiotic Pool (Stage 5) when
/// generating candidates for isolation testing.
pub struct MutationSampler {
    rng: Xorshift64,
    /// Base standard deviation — the noise amplitude before telomere scaling.
    pub sigma_base: f64,
    /// Learning rate: scales the guidance force magnitude.
    pub learning_rate: f64,
    /// Which distribution the stochastic noise draws from.
    pub mode: SamplingMode,
    /// Acceptance rate tracker (updated from Stage 5 Simulation outcomes).
    pub acceptance: AcceptanceTracker,
    /// Current σ multiplier (adaptive mode only); starts at 1.0.
    sigma_multiplier: f64,
}

impl MutationSampler {
    /// Create a sampler with default parameters, seeded from the entity id.
    pub fn for_entity(entity_id: &str) -> Self {
        let seed = hash_string(entity_id);
        Self::with_seed(seed)
    }

    /// Create a sampler with a specific seed (useful for deterministic tests).
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: Xorshift64::new(seed),
            sigma_base: DEFAULT_SIGMA_BASE,
            learning_rate: DEFAULT_LEARNING_RATE,
            mode: SamplingMode::Adaptive,
            acceptance: AcceptanceTracker::new(ACCEPTANCE_WINDOW),
            sigma_multiplier: 1.0,
        }
    }

    /// Sample a parameter delta for one telos dimension.
    ///
    /// # Parameters
    ///
    /// - `current`: current observed mean of the metric (from Epigenome Working tier)
    /// - `target`: declared telos target for this metric
    /// - `bounds`: (min, max) hard bounds from the telos declaration
    /// - `drift_score`: current normalised drift [0, 1]; higher = more off-target
    /// - `relative_telomere`: telomere fraction [0, 1]; 1.0 = juvenile, 0.0 = senescent
    ///
    /// # Returns
    ///
    /// A delta that, when added to `current`, moves the parameter in the direction
    /// of `target`. Clamped so that `current + delta` stays within `bounds`.
    pub fn sample(
        &mut self,
        current: f64,
        target: f64,
        bounds: (f64, f64),
        drift_score: f64,
        relative_telomere: f64,
    ) -> f64 {
        let temperature = self.effective_temperature(relative_telomere);
        let guidance = self.guidance_force(current, target, bounds, drift_score);
        let noise = self.draw_noise(temperature, drift_score);
        let raw_delta = guidance + noise;
        clamp_to_bounds(current, raw_delta, bounds)
    }

    /// Record a Stage 5 simulation outcome to adapt σ (Adaptive mode only).
    pub fn record_outcome(&mut self, passed: bool) {
        self.acceptance.record(passed);
        if self.mode != SamplingMode::Adaptive {
            return;
        }
        if let Some(rate) = self.acceptance.rate() {
            if rate > ACCEPTANCE_HIGH_THRESHOLD {
                // Doing well — expand σ to explore more.
                self.sigma_multiplier = (self.sigma_multiplier * 1.2).min(4.0);
            } else if rate < ACCEPTANCE_LOW_THRESHOLD {
                // Struggling — shrink σ to be more conservative.
                self.sigma_multiplier = (self.sigma_multiplier * 0.5).max(0.1);
            }
        }
    }

    /// Current acceptance rate, if enough history is available.
    pub fn acceptance_rate(&self) -> Option<f64> {
        self.acceptance.rate()
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Effective temperature: σ_base × telomere × adaptive_multiplier.
    fn effective_temperature(&self, relative_telomere: f64) -> f64 {
        let telomere_factor = relative_telomere.clamp(0.0, 1.0);
        self.sigma_base * telomere_factor.max(0.05) * self.sigma_multiplier
    }

    /// Guidance force: deterministic pull toward telos target.
    ///
    /// Scaled by drift_score so the pull is stronger when far off-target.
    /// Normalised by bounds width to be scale-independent.
    fn guidance_force(
        &self,
        current: f64,
        target: f64,
        bounds: (f64, f64),
        drift_score: f64,
    ) -> f64 {
        let width = (bounds.1 - bounds.0).abs().max(1e-12);
        let direction = target - current; // positive = need to increase
        let normalised = direction / width;
        normalised * drift_score * self.learning_rate
    }

    /// Draw one noise sample from the configured distribution.
    fn draw_noise(&mut self, temperature: f64, drift_score: f64) -> f64 {
        match self.mode {
            SamplingMode::Gaussian => self.rng.next_normal() * temperature,
            SamplingMode::Cauchy => self.rng.next_cauchy() * temperature,
            SamplingMode::Levy => self.rng.next_levy() * temperature,
            SamplingMode::Adaptive => {
                // Escalate to Lévy when acceptance is low AND drift is high.
                let stuck = self
                    .acceptance
                    .rate()
                    .is_some_and(|r| r < ACCEPTANCE_LOW_THRESHOLD);
                let high_drift = drift_score > 0.6;

                if stuck && high_drift && self.rng.next_f64() < LEVY_STUCK_PROBABILITY {
                    self.rng.next_levy() * temperature
                } else {
                    self.rng.next_normal() * temperature
                }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Clamp `current + raw_delta` to `bounds`, returning the clamped delta.
fn clamp_to_bounds(current: f64, raw_delta: f64, bounds: (f64, f64)) -> f64 {
    let proposed = current + raw_delta;
    let clamped = proposed.clamp(bounds.0, bounds.1);
    clamped - current
}

/// Deterministic hash of a string for PRNG seeding.
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sampler() -> MutationSampler {
        MutationSampler::with_seed(42)
    }

    // ── PRNG ──────────────────────────────────────────────────────────────────

    #[test]
    fn xorshift_different_seeds_produce_different_sequences() {
        let mut a = Xorshift64::new(1);
        let mut b = Xorshift64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn xorshift_same_seed_is_deterministic() {
        let mut a = Xorshift64::new(999);
        let mut b = Xorshift64::new(999);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn next_normal_mean_close_to_zero() {
        let mut rng = Xorshift64::new(1234);
        let samples: Vec<f64> = (0..10_000).map(|_| rng.next_normal()).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(mean.abs() < 0.05, "mean {mean:.4} too far from 0");
    }

    #[test]
    fn next_normal_std_dev_close_to_one() {
        let mut rng = Xorshift64::new(5678);
        let samples: Vec<f64> = (0..10_000).map(|_| rng.next_normal()).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        let sd = variance.sqrt();
        assert!((sd - 1.0).abs() < 0.05, "std dev {sd:.4} too far from 1");
    }

    #[test]
    fn levy_samples_have_heavier_tail_than_gaussian() {
        // Lévy tail: expect more samples with |x| > 5 than Gaussian.
        let mut rng = Xorshift64::new(99);
        let n = 10_000;
        let levy_large = (0..n).filter(|_| rng.next_levy().abs() > 5.0).count();
        let gauss_large = (0..n).filter(|_| rng.next_normal().abs() > 5.0).count();
        assert!(
            levy_large > gauss_large,
            "Lévy should have more large samples: levy={levy_large} gauss={gauss_large}"
        );
    }

    // ── Guidance force ────────────────────────────────────────────────────────

    #[test]
    fn guidance_force_points_toward_target() {
        let s = sampler();
        // current=0.8, target=0.3, bounds=(0,1) → direction negative
        let g = s.guidance_force(0.8, 0.3, (0.0, 1.0), 1.0);
        assert!(g < 0.0, "guidance should be negative (need to decrease), got {g}");
    }

    #[test]
    fn guidance_force_zero_when_already_at_target() {
        let s = sampler();
        let g = s.guidance_force(0.5, 0.5, (0.0, 1.0), 1.0);
        assert!(g.abs() < 1e-12);
    }

    #[test]
    fn guidance_force_scales_with_drift_score() {
        let s = sampler();
        let g_low = s.guidance_force(0.8, 0.3, (0.0, 1.0), 0.1);
        let g_high = s.guidance_force(0.8, 0.3, (0.0, 1.0), 0.9);
        assert!(g_high.abs() > g_low.abs(), "higher drift should produce stronger guidance");
    }

    // ── Temperature ───────────────────────────────────────────────────────────

    #[test]
    fn young_entity_has_higher_temperature_than_old() {
        let s = sampler();
        let t_young = s.effective_temperature(0.9);
        let t_old = s.effective_temperature(0.1);
        assert!(t_young > t_old);
    }

    #[test]
    fn senescent_entity_temperature_does_not_reach_zero() {
        let s = sampler();
        let t = s.effective_temperature(0.0);
        assert!(t > 0.0, "temperature must stay positive for exploration: {t}");
    }

    // ── Delta sampling ────────────────────────────────────────────────────────

    #[test]
    fn sample_returns_delta_within_bounds() {
        let mut s = MutationSampler::with_seed(42);
        for _ in 0..1_000 {
            let delta = s.sample(0.5, 0.3, (0.0, 1.0), 0.5, 0.7);
            let proposed = 0.5 + delta;
            assert!(
                proposed >= 0.0 && proposed <= 1.0,
                "proposed {proposed} out of bounds"
            );
        }
    }

    #[test]
    fn sample_with_high_drift_biases_toward_target() {
        let mut s = MutationSampler::with_seed(7);
        s.mode = SamplingMode::Gaussian; // deterministic for this test
        // current=0.9, target=0.2 → guidance is strongly negative
        let deltas: Vec<f64> = (0..100)
            .map(|_| s.sample(0.9, 0.2, (0.0, 1.0), 0.95, 0.8))
            .collect();
        let mean_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;
        assert!(mean_delta < 0.0, "mean delta should be negative (toward target=0.2): {mean_delta}");
    }

    #[test]
    fn sample_cauchy_still_stays_within_bounds() {
        let mut s = MutationSampler::with_seed(123);
        s.mode = SamplingMode::Cauchy;
        for _ in 0..500 {
            let delta = s.sample(50.0, 20.0, (0.0, 100.0), 0.7, 0.5);
            let proposed = 50.0 + delta;
            assert!(
                proposed >= 0.0 && proposed <= 100.0,
                "Cauchy delta {delta} pushed {proposed} out of bounds"
            );
        }
    }

    // ── Acceptance tracker ────────────────────────────────────────────────────

    #[test]
    fn acceptance_rate_reflects_passed_fraction() {
        let mut t = AcceptanceTracker::new(10);
        for _ in 0..8 { t.record(true); }
        for _ in 0..2 { t.record(false); }
        assert!((t.rate().unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn acceptance_rate_none_when_empty() {
        let t = AcceptanceTracker::new(10);
        assert!(t.rate().is_none());
    }

    #[test]
    fn acceptance_tracker_evicts_oldest_outside_window() {
        let mut t = AcceptanceTracker::new(3);
        t.record(true);
        t.record(true);
        t.record(true);
        t.record(false); // evicts first true
        // window is now [true, true, false] → 2/3
        assert!((t.rate().unwrap() - 2.0 / 3.0).abs() < 1e-9);
    }

    // ── Adaptive mode ─────────────────────────────────────────────────────────

    #[test]
    fn adaptive_mode_expands_sigma_multiplier_on_high_acceptance() {
        let mut s = MutationSampler::with_seed(1);
        s.mode = SamplingMode::Adaptive;
        let initial = s.sigma_multiplier;
        // Record many successes.
        for _ in 0..ACCEPTANCE_WINDOW {
            s.record_outcome(true);
        }
        assert!(
            s.sigma_multiplier > initial,
            "σ multiplier should grow on high acceptance: {:.3}",
            s.sigma_multiplier
        );
    }

    #[test]
    fn adaptive_mode_shrinks_sigma_multiplier_on_low_acceptance() {
        let mut s = MutationSampler::with_seed(2);
        s.mode = SamplingMode::Adaptive;
        let initial = s.sigma_multiplier;
        // Record many failures.
        for _ in 0..ACCEPTANCE_WINDOW {
            s.record_outcome(false);
        }
        assert!(
            s.sigma_multiplier < initial,
            "σ multiplier should shrink on low acceptance: {:.3}",
            s.sigma_multiplier
        );
    }

    #[test]
    fn sigma_multiplier_bounded_above_and_below() {
        let mut s = MutationSampler::with_seed(3);
        s.mode = SamplingMode::Adaptive;
        for _ in 0..1_000 { s.record_outcome(true); }
        assert!(s.sigma_multiplier <= 4.0, "upper bound violated: {}", s.sigma_multiplier);
        let mut s2 = MutationSampler::with_seed(4);
        s2.mode = SamplingMode::Adaptive;
        for _ in 0..1_000 { s2.record_outcome(false); }
        assert!(s2.sigma_multiplier >= 0.1, "lower bound violated: {}", s2.sigma_multiplier);
    }

    // ── For entity seeding ────────────────────────────────────────────────────

    #[test]
    fn for_entity_different_ids_produce_different_first_sample() {
        let mut a = MutationSampler::for_entity("entity_alpha");
        let mut b = MutationSampler::for_entity("entity_beta");
        let da = a.sample(0.5, 0.3, (0.0, 1.0), 0.5, 0.5);
        let db = b.sample(0.5, 0.3, (0.0, 1.0), 0.5, 0.5);
        assert_ne!(da, db, "different entity ids should produce different deltas");
    }

    #[test]
    fn for_entity_same_id_is_deterministic_with_same_seed() {
        let mut a = MutationSampler::for_entity("my_entity");
        let mut b = MutationSampler::for_entity("my_entity");
        let da = a.sample(0.5, 0.3, (0.0, 1.0), 0.5, 0.5);
        let db = b.sample(0.5, 0.3, (0.0, 1.0), 0.5, 0.5);
        assert_eq!(da, db);
    }
}
