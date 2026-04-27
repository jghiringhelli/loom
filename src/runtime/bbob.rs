//! BBOB (Black-Box Optimization Benchmarking) canonical function implementations.
//!
//! Implements four functions from the COCO benchmark suite (Hansen et al., 2021)
//! used in the GECCO BBOB workshop. Selected for the BIOISO T5 empirical experiment:
//!
//! | ID  | Name                  | Why selected                                        |
//! |-----|-----------------------|-----------------------------------------------------|
//! | f1  | Sphere                | Unimodal baseline; T1-T4 dominate; T5 should match |
//! | f2  | Separable Ellipsoid   | Ill-conditioned; tests T4 sample efficiency         |
//! | f15 | Rastrigin             | ~10^n local optima; T5 structural rewire is needed  |
//! | f24 | Lunacek bi-Rastrigin  | Bimodal basin; inter-basin jump requires T5         |
//!
//! All functions are defined on R^n with global optimum at f* = 0 (after shifting).
//! Fitness is normalized: `nf = (f(x) - 0) / max(1.0, f(x_init))`.
//!
//! Reference: Hansen N., Finck S., Ros R., Auger A. (2009).
//! "Real-parameter black-box optimization benchmarking 2009: Noiseless functions definitions."
//! INRIA Research Report RR-6829.

use std::f64::consts::PI;

// ── Internal PRNG (no external rand crate) ────────────────────────────────────

/// 64-bit LCG — fast, portable, reproducible.
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Sample from U[-range, range].
    pub fn uniform(&mut self, range: f64) -> f64 {
        self.next_f64() * 2.0 * range - range
    }

    /// Sample from N(0, sigma) using Box-Muller.
    pub fn normal(&mut self, sigma: f64) -> f64 {
        let u1 = self.next_f64().max(1e-300);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos() * sigma
    }
}

// ── Coordinate transformation helpers ────────────────────────────────────────

/// Generate a random DIM×DIM orthogonal matrix via Gram-Schmidt on LCG vectors.
/// Returns as a flat row-major Vec<f64> of length DIM*DIM.
pub fn random_rotation(dim: usize, rng: &mut Lcg) -> Vec<f64> {
    // Build DIM random vectors and orthonormalize them.
    let mut basis = vec![0.0f64; dim * dim];
    for i in 0..dim {
        for j in 0..dim {
            basis[i * dim + j] = rng.normal(1.0);
        }
        // Gram-Schmidt: subtract projections of previous basis vectors.
        for k in 0..i {
            let dot: f64 = (0..dim)
                .map(|j| basis[i * dim + j] * basis[k * dim + j])
                .sum();
            for j in 0..dim {
                basis[i * dim + j] -= dot * basis[k * dim + j];
            }
        }
        // Normalize.
        let norm: f64 = (0..dim)
            .map(|j| basis[i * dim + j].powi(2))
            .sum::<f64>()
            .sqrt();
        if norm > 1e-12 {
            for j in 0..dim {
                basis[i * dim + j] /= norm;
            }
        }
    }
    basis
}

/// Apply rotation matrix (row-major, DIM×DIM) to vector x → y = R * x.
pub fn rotate(r: &[f64], x: &[f64], dim: usize) -> Vec<f64> {
    (0..dim)
        .map(|i| (0..dim).map(|j| r[i * dim + j] * x[j]).sum())
        .collect()
}

/// Apply T_osz transformation (BBOB oscillation): x → sign(x) * exp(|x|^0.1 * ...)
/// Used by f15 and f24 to introduce asymmetry around the optimum.
fn t_osz(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let s = if x > 0.0 { 1.0 } else { -1.0 };
    let x_hat = if x != 0.0 { x.abs().ln() } else { 0.0 };
    let c1 = if x > 0.0 { 10.0 } else { 5.5 };
    let c2 = if x > 0.0 { 7.9 } else { 3.1 };
    s * (x_hat + 0.049 * (c1 * x_hat).sin() + (c2 * x_hat).sin()).exp()
}

// ── BBOB Functions ────────────────────────────────────────────────────────────

/// f1: Sphere — unimodal, symmetric.
/// z = R * (x - x_opt); f(z) = ||z||²
pub fn f1_sphere(z: &[f64]) -> f64 {
    z.iter().map(|xi| xi * xi).sum()
}

/// f2: Separable Ellipsoid — unimodal, ill-conditioned (condition number 10^6).
/// z = x - x_opt (no rotation — separable version); f(z) = Σ 10^(6i/(n-1)) * T_osz(z_i)²
pub fn f2_ellipsoid(z: &[f64]) -> f64 {
    let n = z.len();
    z.iter()
        .enumerate()
        .map(|(i, xi)| {
            let exp = if n > 1 {
                6.0 * i as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let t = t_osz(*xi);
            10f64.powf(exp) * t * t
        })
        .sum()
}

/// f15: Rastrigin — multimodal, ~10^n local optima on a regular grid.
/// Uses T_osz and T_asy transformations before the Rastrigin template.
/// z = R2 * T_asy(T_osz(R1 * (x - x_opt))); f(z) = 10n + Σ [z_i² - 10cos(2π z_i)]
pub fn f15_rastrigin(z: &[f64]) -> f64 {
    let n = z.len() as f64;
    let sq_sum: f64 = z.iter().map(|xi| xi * xi).sum();
    let cos_sum: f64 = z.iter().map(|xi| (2.0 * PI * xi).cos()).sum();
    10.0 * n + sq_sum - 10.0 * cos_sum
}

/// f24: Lunacek bi-Rastrigin — bimodal basin structure with Rastrigin oscillation.
/// The global optimum is in one basin; the second basin is a deep local attractor.
/// T1-T4 cannot reliably identify which basin contains the global optimum.
///
/// f(x) = min(||x - mu_0||², s * ||x - mu_1||²) + 10 * (n - Σ cos(2π(x - mu_0)))
pub fn f24_lunacek(x: &[f64], x_opt: &[f64]) -> f64 {
    let n = x.len();
    let mu0 = 2.5;
    let s = 1.0 - 1.0 / (2.0 * ((n as f64 + 20.0).sqrt()) - 8.2);
    let mu1 = -((mu0 * mu0 - 1.0) / s).sqrt();

    // Shifted inputs.
    let z: Vec<f64> = x.iter().zip(x_opt).map(|(xi, xo)| xi - xo + mu0).collect();

    let sphere1: f64 = z.iter().map(|zi| (zi - mu0) * (zi - mu0)).sum();
    let sphere2: f64 = s * z.iter().map(|zi| (zi - mu1) * (zi - mu1)).sum::<f64>();

    let n_f = n as f64;
    let cos_sum: f64 = z
        .iter()
        .map(|zi| (2.0 * PI * (zi - mu0)).cos())
        .sum::<f64>();
    let rastrigin = 10.0 * (n_f - cos_sum);

    sphere1.min(sphere2) + rastrigin
}

// ── Domain specs for the experiment harness ───────────────────────────────────

/// Which BBOB function to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbobFn {
    Sphere,
    Ellipsoid,
    Rastrigin,
    Lunacek,
}

impl BbobFn {
    pub fn name(self) -> &'static str {
        match self {
            BbobFn::Sphere => "f1_sphere",
            BbobFn::Ellipsoid => "f2_ellipsoid",
            BbobFn::Rastrigin => "f15_rastrigin",
            BbobFn::Lunacek => "f24_lunacek",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            BbobFn::Sphere => "f1",
            BbobFn::Ellipsoid => "f2",
            BbobFn::Rastrigin => "f15",
            BbobFn::Lunacek => "f24",
        }
    }

    /// Is this a multimodal function where T5 structural rewire should help?
    pub fn is_multimodal(self) -> bool {
        matches!(self, BbobFn::Rastrigin | BbobFn::Lunacek)
    }

    /// Evaluate raw fitness at x (shifted to z = Rx).
    pub fn evaluate(self, z: &[f64], x_opt: &[f64]) -> f64 {
        match self {
            BbobFn::Sphere => f1_sphere(z),
            BbobFn::Ellipsoid => f2_ellipsoid(z),
            BbobFn::Rastrigin => f15_rastrigin(z),
            BbobFn::Lunacek => f24_lunacek(z, x_opt),
        }
    }

    /// All four BBOB functions.
    pub fn all() -> [BbobFn; 4] {
        [
            BbobFn::Sphere,
            BbobFn::Ellipsoid,
            BbobFn::Rastrigin,
            BbobFn::Lunacek,
        ]
    }
}
