//! Explicit mathematical domain structure codegen.
//!
//! These are emitted because the developer **declared** a specific mathematical
//! object — a Markov chain, a stochastic process, a probability distribution, a
//! graph.  Unlike disciplines, these are not implicitly applied patterns; they are
//! the thing itself.  The developer explicitly asked for a Markov chain; they get
//! one.  Loom's job is to emit the correct, proven implementation.
//!
//! ## Structure map (declaration -> generated artifact)
//!
//! ### Stochastic Processes (fn annotation: `process:`)
//! | Wiener            | BrownianMotion sampler — Wiener 1923       |
//! | GeometricBrownian | GBM price simulation — Black-Scholes 1973  |
//! | OrnsteinUhlenbeck | Mean-reverting process — OU 1930           |
//! | PoissonProcess    | Event counting process — Poisson 1837      |
//! | MarkovChain       | Typed transition matrix — Markov 1906      |
//!
//! ### Probability Distributions (fn annotation: `distribution:`)
//! | Gaussian    | Normal sampler (Box-Muller) — Gauss 1809           |
//! | Poisson     | Poisson sampler (Knuth) — Poisson 1837             |
//! | Uniform     | Uniform sampler — Laplace 1812                     |
//! | Exponential | Memoryless waiting-time sampler                    |
//! | Beta        | Beta sampler — Bayesian prior — Euler 1763         |
//! | Binomial    | Binomial sampler — Bernoulli 1713                  |
//! | Pareto      | Power-law tail sampler — Pareto 1896               |
//! | LogNormal   | Log-normal sampler — Galton 1879                   |
//! | Gamma       | Gamma sampler — Euler 1729                         |
//! | Cauchy      | Heavy-tail (no mean) — Cauchy 1853                 |
//! | Levy        | Stable/anomalous diffusion — Levy 1937             |
//! | Dirichlet   | Probability simplex — Bayesian prior               |
//!
//! ### Graph Structures (store :: Graph)
//! | Graph (directed)   | DAG + topological sort — Kahn 1962              |
//! | Graph (undirected) | Labelled Transition System — Keller 1976        |

use crate::ast::*;
use super::{RustEmitter, to_pascal_case};
use super::template::ts;


// ═══════════════════════════════════════════════════════════════════════════
// STOCHASTIC PROCESSES
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct stochastic process emitter from a `process:` annotation.
    pub(super) fn emit_stochastic_process(
        &self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String,
    ) {
        match &sp.kind {
            StochasticKind::Wiener            => self.emit_wiener_process(fn_name, out),
            StochasticKind::GeometricBrownian => self.emit_gbm(fn_name, sp, out),
            StochasticKind::OrnsteinUhlenbeck => self.emit_ou_process(fn_name, sp, out),
            StochasticKind::PoissonProcess    => self.emit_poisson_process(fn_name, sp, out),
            StochasticKind::MarkovChain       => self.emit_markov_transition_matrix(fn_name, &sp.states, out),
            StochasticKind::Unknown(k)        => {
                out.push_str(&format!(
                    "// LOOM[structure:stochastic:Unknown]: process kind '{k}' not yet generated\n\n"
                ));
            }
        }
    }

    /// Standard Brownian motion (Wiener 1923).
    /// W(t+dt) = W(t) + sqrt(dt)*N(0,1). Martingale. E[W_t]=0, Var[W_t]=t.
    fn emit_wiener_process(&self, fn_name: &str, out: &mut String) {
        let n = to_pascal_case(fn_name);
        out.push_str(&format!(
            "// LOOM[structure:Wiener]: {fn_name} — Brownian motion (Wiener 1923)\n\
             // E[W_t]=0, Var[W_t]=t. Martingale. Continuous paths. Ecosystem: rand, statrs\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}WienerProcess {{\n    pub t: f64,\n    pub value: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}WienerProcess {{\n    \
pub fn new() -> Self {{ Self {{ t: 0.0, value: 0.0 }} }}\n    \
/// Euler-Maruyama: W(t+dt) = W(t) + sqrt(dt)*z, z ~ N(0,1).\n    \
pub fn step(&mut self, dt: f64, z: f64) {{ self.t += dt; self.value += dt.sqrt() * z; }}\n}}\n\n"
        ));
    }

    /// Geometric Brownian Motion (Black-Scholes 1973).
    /// dS = mu*S*dt + sigma*S*dW. Always positive. Log-normal increments.
    fn emit_gbm(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let mu = sp.long_run_mean.as_deref().unwrap_or("0.05");
        out.push_str(&format!(
            "// LOOM[structure:GBM]: {fn_name} — Geometric Brownian Motion (Black-Scholes 1973)\n\
             // dS = mu*S*dt + sigma*S*dW. Always positive. Log-normal. mu={mu}\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}GBM {{\n    pub mu: f64,\n    pub sigma: f64,\n    pub price: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}GBM {{\n    \
pub fn new(price: f64) -> Self {{ Self {{ mu: {mu}, sigma: 0.2, price }} }}\n    \
/// S(t+dt) = S(t)*exp((mu-0.5*sigma^2)*dt + sigma*sqrt(dt)*z).\n    \
pub fn step(&mut self, dt: f64, z: f64) {{\n        \
self.price *= ((self.mu - 0.5*self.sigma*self.sigma)*dt + self.sigma*dt.sqrt()*z).exp();\n    }}\n    \
pub fn assert_positive(&self) {{ debug_assert!(self.price > 0.0, \"GBM price must be > 0\"); }}\n}}\n\n"
        ));
    }

    /// Ornstein-Uhlenbeck mean-reverting process (OU 1930).
    /// dX = theta*(mu - X)*dt + sigma*dW. Stationary Gaussian.
    fn emit_ou_process(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let mu = sp.long_run_mean.as_deref().unwrap_or("0.0");
        out.push_str(&format!(
            "// LOOM[structure:OU]: {fn_name} — Ornstein-Uhlenbeck (1930)\n\
             // dX = theta*(mu-X)*dt + sigma*dW. Mean-reverting to {mu}. Stationary Gaussian.\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}OUProcess {{\n    pub theta: f64,\n    pub mu: f64,\n    pub sigma: f64,\n    pub value: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}OUProcess {{\n    \
pub fn new() -> Self {{ Self {{ theta: 1.0, mu: {mu}, sigma: 0.1, value: 0.0 }} }}\n    \
pub fn step(&mut self, dt: f64, z: f64) {{\n        \
self.value += self.theta*(self.mu - self.value)*dt + self.sigma*dt.sqrt()*z;\n    }}\n}}\n\n"
        ));
    }

    /// Poisson process (Poisson 1837). N(t) ~ Poisson(lambda*t). Integer-valued.
    fn emit_poisson_process(&self, fn_name: &str, sp: &StochasticProcessBlock, out: &mut String) {
        let n = to_pascal_case(fn_name);
        let rate = sp.rate.as_deref().unwrap_or("1.0");
        out.push_str(&format!(
            "// LOOM[structure:PoissonProcess]: {fn_name} — Poisson process (Poisson 1837)\n\
             // N(t)~Poisson(lambda*t). Integer-valued. Inter-arrival~Exp(lambda). rate={rate}\n\n"
        ));
        out.push_str(&format!(
            "#[derive(Debug, Clone)]\npub struct {n}PoissonProcess {{\n    pub lambda: f64,\n    pub count: u64,\n    pub t: f64,\n}}\n"
        ));
        out.push_str(&format!(
            "impl {n}PoissonProcess {{\n    \
pub fn new() -> Self {{ Self {{ lambda: {rate}, count: 0, t: 0.0 }} }}\n    \
/// Advance by dt. Provide arrivals from rand_distr::Poisson(lambda*dt).\n    \
pub fn step(&mut self, dt: f64, arrivals: u64) {{ self.t += dt; self.count += arrivals; }}\n}}\n\n"
        ));
    }

    /// Markov chain TransitionMatrix<S> (Markov 1906).
    /// Memoryless discrete-state chain. P(X_{n+1}|X_n).
    pub(super) fn emit_markov_transition_matrix(
        &self, fn_name: &str, states: &[String], out: &mut String,
    ) {
        let n = to_pascal_case(fn_name);
        let states_enum = states.iter()
            .map(|s| format!("    {},", to_pascal_case(s)))
            .collect::<Vec<_>>().join("\n");
        out.push_str(&ts(
            r#"
// LOOM[structure:Markov]: {fn_name} — TransitionMatrix (Markov 1906)
// P(X_{n+1}|X_n): memoryless, discrete-state chain.
// Ecosystem: ndarray (dense), petgraph (sparse), statrs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum {N}States {
{states}
}
#[derive(Debug, Clone, Default)]
pub struct {N}TransitionMatrix {
    transitions: std::collections::HashMap<({N}States, {N}States), f64>,
}
impl {N}TransitionMatrix {
    pub fn set(&mut self, from: {N}States, to: {N}States, prob: f64) {
        debug_assert!((0.0..=1.0).contains(&prob), "prob must be in [0,1]");
        self.transitions.insert((from, to), prob);
    }
    pub fn next_states(&self, state: {N}States) -> Vec<({N}States, f64)> {
        self.transitions.iter()
            .filter_map(|(&(f, t), &p)| if f == state { Some((t, p)) } else { None })
            .collect()
    }
    /// Verify all outgoing probs from each state sum to 1.0 (stochastic matrix).
    pub fn validate(&self) -> bool {
        use std::collections::HashMap;
        let mut sums: HashMap<{N}States, f64> = HashMap::new();
        for (&(from, _), &p) in &self.transitions { *sums.entry(from).or_default() += p; }
        sums.values().all(|&s| (s - 1.0).abs() < 1e-9)
    }
}"#,
            &[("N", &n), ("fn_name", fn_name), ("states", &states_enum)],
        ));
        out.push_str("\n\n");
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// PROBABILITY DISTRIBUTIONS
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct distribution sampler from a `distribution:` annotation.
    pub(super) fn emit_distribution_sampler(
        &self, fn_name: &str, db: &DistributionBlock, out: &mut String,
    ) {
        let n = to_pascal_case(fn_name);
        match &db.family {
            DistributionFamily::Gaussian { mean, std_dev } => {
                out.push_str(&format!(
                    "// LOOM[structure:Gaussian]: {fn_name} — Normal distribution (Gauss 1809)\n\
                     // X ~ N(mu={mean}, sigma={std_dev}). Ecosystem: rand_distr::Normal\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GaussianSampler {{\n    pub mean: f64,\n    pub std_dev: f64,\n}}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GaussianSampler {{\n    \
pub fn new() -> Self {{ Self {{ mean: {mean}, std_dev: {std_dev} }} }}\n    \
/// Box-Muller transform. z1, z2 ~ U(0,1). Returns one N(0,1) sample.\n    \
pub fn sample_box_muller(&self, z1: f64, z2: f64) -> f64 {{\n        \
let n01 = (-2.0*z1.ln()).sqrt() * (2.0*std::f64::consts::PI*z2).cos();\n        \
self.mean + self.std_dev * n01\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Poisson { lambda } => {
                out.push_str(&format!(
                    "// LOOM[structure:PoissonDist]: {fn_name} — Poisson distribution (Poisson 1837)\n\
                     // X ~ Poisson(lambda={lambda}). Integer-valued. Ecosystem: rand_distr::Poisson\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}PoissonSampler {{ pub lambda: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}PoissonSampler {{\n    \
pub fn new() -> Self {{ Self {{ lambda: {lambda} }} }}\n    \
/// Knuth algorithm for small lambda. For large lambda use Gaussian approx.\n    \
pub fn sample_knuth(&self, uniform_samples: &[f64]) -> u64 {{\n        \
let limit = (-self.lambda).exp();\n        \
let mut prod = 1.0; let mut k = 0u64;\n        \
for &u in uniform_samples {{ prod *= u; k += 1; if prod < limit {{ break; }} }}\n        \
k.saturating_sub(1)\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Uniform { low, high } => {
                out.push_str(&format!(
                    "// LOOM[structure:Uniform]: {fn_name} — Uniform distribution (Laplace 1812)\n\
                     // X ~ U({low}, {high}). Ecosystem: rand::Rng::gen_range\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}UniformSampler {{ pub low: f64, pub high: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}UniformSampler {{\n    \
pub fn new() -> Self {{ Self {{ low: {low}, high: {high} }} }}\n    \
pub fn sample(&self, u: f64) -> f64 {{ debug_assert!((0.0..=1.0).contains(&u)); self.low + (self.high - self.low) * u }}\n}}\n\n"
                ));
            }
            DistributionFamily::Exponential { lambda } => {
                out.push_str(&format!(
                    "// LOOM[structure:Exponential]: {fn_name} — Exponential distribution\n\
                     // X ~ Exp(lambda={lambda}). Memoryless. Inter-arrival times. Ecosystem: rand_distr::Exp\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}ExpSampler {{ pub lambda: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}ExpSampler {{\n    \
pub fn new() -> Self {{ Self {{ lambda: {lambda} }} }}\n    \
/// Inverse CDF: X = -ln(U)/lambda, U ~ U(0,1).\n    \
pub fn sample(&self, u: f64) -> f64 {{ debug_assert!(u > 0.0 && u < 1.0); -u.ln() / self.lambda }}\n}}\n\n"
                ));
            }
            DistributionFamily::Beta { alpha, beta } => {
                out.push_str(&format!(
                    "// LOOM[structure:Beta]: {fn_name} — Beta distribution (Euler 1763)\n\
                     // X ~ Beta(alpha={alpha}, beta={beta}). Bounded [0,1]. Bayesian prior.\n\
                     // Ecosystem: rand_distr::Beta, statrs::Beta\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}BetaSampler {{ pub alpha: f64, pub beta: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}BetaSampler {{\n    \
pub fn new() -> Self {{ Self {{ alpha: {alpha}, beta: {beta} }} }}\n    \
pub fn mean(&self) -> f64 {{ self.alpha / (self.alpha + self.beta) }}\n    \
pub fn variance(&self) -> f64 {{\n        \
let s = self.alpha + self.beta;\n        \
self.alpha * self.beta / (s * s * (s + 1.0))\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Binomial { n: bin_n, p: bin_p } => {
                let struct_name = format!("{n}BinomialSampler");
                out.push_str(&format!(
                    "// LOOM[structure:Binomial]: {fn_name} — Binomial distribution (Bernoulli 1713)\n\
// X ~ Bin(n={bin_n}, p={bin_p}). Count of successes in n trials.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {struct_name} {{ pub n: u64, pub p: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {struct_name} {{\n    \
pub fn new() -> Self {{ Self {{ n: {bin_n}, p: {bin_p} }} }}\n    \
pub fn mean(&self) -> f64 {{ self.n as f64 * self.p }}\n    \
pub fn variance(&self) -> f64 {{ self.n as f64 * self.p * (1.0 - self.p) }}\n}}\n\n"
                ));
            }
            DistributionFamily::Pareto { alpha, x_min } => {
                out.push_str(&format!(
                    "// LOOM[structure:Pareto]: {fn_name} — Pareto power-law (Pareto 1896)\n\
                     // X ~ Pareto(alpha={alpha}, x_min={x_min}). 80/20 rule. Heavy tail.\n\
                     // WARNING: Mean infinite if alpha <= 1. Variance infinite if alpha <= 2.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}ParetoSampler {{ pub alpha: f64, pub x_min: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}ParetoSampler {{\n    \
pub fn new() -> Self {{ Self {{ alpha: {alpha}, x_min: {x_min} }} }}\n    \
pub fn sample(&self, u: f64) -> f64 {{ self.x_min / (1.0 - u).powf(1.0 / self.alpha) }}\n}}\n\n"
                ));
            }
            DistributionFamily::LogNormal { mean, std_dev } => {
                out.push_str(&format!(
                    "// LOOM[structure:LogNormal]: {fn_name} — Log-Normal (Galton 1879)\n\
                     // ln(X) ~ N(mu={mean}, sigma={std_dev}). Always positive. Multiplicative processes.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}LogNormalSampler {{ pub mu: f64, pub sigma: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}LogNormalSampler {{\n    \
pub fn new() -> Self {{ Self {{ mu: {mean}, sigma: {std_dev} }} }}\n    \
pub fn sample(&self, z: f64) -> f64 {{ (self.mu + self.sigma * z).exp() }}\n    \
pub fn median(&self) -> f64 {{ self.mu.exp() }}\n}}\n\n"
                ));
            }
            DistributionFamily::GeometricBrownian { drift, volatility } => {
                out.push_str(&format!(
                    "// LOOM[structure:GBMDist]: {fn_name} — GBM distribution (Black-Scholes 1973)\n\
                     // drift={drift}, volatility={volatility}\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GBMDist {{ pub drift: f64, pub volatility: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GBMDist {{\n    \
pub fn new() -> Self {{ Self {{ drift: {drift}, volatility: {volatility} }} }}\n}}\n\n"
                ));
            }
            DistributionFamily::Gamma { shape, scale } => {
                out.push_str(&format!(
                    "// LOOM[structure:Gamma]: {fn_name} — Gamma distribution (Euler 1729)\n\
                     // X ~ Gamma(k={shape}, theta={scale}). Waiting times, positive reals.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}GammaSampler {{ pub shape: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}GammaSampler {{\n    \
pub fn new() -> Self {{ Self {{ shape: {shape}, scale: {scale} }} }}\n    \
pub fn mean(&self) -> f64 {{ self.shape * self.scale }}\n    \
pub fn variance(&self) -> f64 {{ self.shape * self.scale * self.scale }}\n}}\n\n"
                ));
            }
            DistributionFamily::Cauchy { location, scale } => {
                out.push_str(&format!(
                    "// LOOM[structure:Cauchy]: {fn_name} — Cauchy distribution (Cauchy 1853)\n\
                     // WARNING: NO defined mean or variance. CLT and LLN do NOT apply.\n\
                     // location={location}, scale={scale}. Heavy-tailed. Do not use for averaging.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}CauchySampler {{ pub location: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}CauchySampler {{\n    \
pub fn new() -> Self {{ Self {{ location: {location}, scale: {scale} }} }}\n    \
// Inverse CDF: X = location + scale*tan(pi*(u - 0.5))\n    \
pub fn sample(&self, u: f64) -> f64 {{\n        \
self.location + self.scale * (std::f64::consts::PI * (u - 0.5)).tan()\n    }}\n}}\n\n"
                ));
            }
            DistributionFamily::Levy { location, scale } => {
                out.push_str(&format!(
                    "// LOOM[structure:Levy]: {fn_name} — Levy distribution (Levy 1937)\n\
                     // Stable distribution. Anomalous diffusion. location={location}, scale={scale}.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}LevySampler {{ pub location: f64, pub scale: f64 }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}LevySampler {{ pub fn new() -> Self {{ Self {{ location: {location}, scale: {scale} }} }} }}\n\n"
                ));
            }
            DistributionFamily::Dirichlet { alpha } => {
                let a_str = alpha.join(", ");
                out.push_str(&format!(
                    "// LOOM[structure:Dirichlet]: {fn_name} — Dirichlet distribution (Dirichlet 1831)\n\
                     // Probability simplex. alpha=[{a_str}]. Bayesian prior for categorical.\n\n"
                ));
                out.push_str(&format!(
                    "#[derive(Debug, Clone)]\npub struct {n}DirichletSampler {{ pub alpha: Vec<f64> }}\n"
                ));
                out.push_str(&format!(
                    "impl {n}DirichletSampler {{\n    \
pub fn new() -> Self {{ Self {{ alpha: vec![{a_str}] }} }}\n    \
pub fn concentration_sum(&self) -> f64 {{ self.alpha.iter().sum() }}\n}}\n\n"
                ));
            }
            DistributionFamily::Unknown(name) => {
                out.push_str(&format!(
                    "// LOOM[structure:distribution:Unknown]: '{name}' distribution not yet generated\n\n"
                ));
            }
        }
    }
}


// ═══════════════════════════════════════════════════════════════════════════
// GRAPH STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// DAG wrapper with Kahn topological sort (Kahn 1962). For directed Graph stores.
    pub(super) fn emit_dag_wrapper(&self, store_name: &str, out: &mut String) {
        let n = to_pascal_case(store_name);
        out.push_str(&ts(
            r#"
// LOOM[structure:DAG]: {name} — Directed Acyclic Graph (Kahn 1962)
// Topological sort via Kahn's algorithm. Ecosystem: petgraph
#[derive(Debug, Clone, Default)]
pub struct {N}Dag {
    nodes: std::collections::HashMap<String, Vec<String>>,
}
impl {N}Dag {
    pub fn new() -> Self { Self::default() }
    pub fn add_node(&mut self, id: impl Into<String>) {
        self.nodes.entry(id.into()).or_default();
    }
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.nodes.entry(from.into()).or_default().push(to.into());
    }
    /// Kahn's algorithm: returns None if cycle detected (invariant: DAG must be acyclic).
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        use std::collections::{HashMap, VecDeque};
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for id in self.nodes.keys() { in_degree.insert(id, 0); }
        for children in self.nodes.values() {
            for c in children { *in_degree.entry(c).or_default() += 1; }
        }
        let mut queue: VecDeque<&str> = in_degree.iter()
            .filter_map(|(&n, &d)| if d == 0 { Some(n) } else { None }).collect();
        let mut result = Vec::new();
        while let Some(n) = queue.pop_front() {
            result.push(n.to_owned());
            if let Some(children) = self.nodes.get(n) {
                for c in children {
                    let d = in_degree.entry(c).or_default();
                    *d -= 1;
                    if *d == 0 { queue.push_back(c); }
                }
            }
        }
        if result.len() == self.nodes.len() { Some(result) } else { None }
    }
}"#,
            &[("N", &n), ("name", store_name)],
        ));
        out.push_str("\n\n");
    }

    /// LTS (Labelled Transition System) for general/undirected graphs (Keller 1976).
    pub(super) fn emit_lts_graph(&self, store_name: &str, out: &mut String) {
        let n = to_pascal_case(store_name);
        out.push_str(&ts(
            r#"
// LOOM[structure:LTS]: {name} — Labelled Transition System (Keller 1976)
// State + action-labelled transitions. Ecosystem: petgraph, roaring
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct {N}State(pub String);
#[derive(Debug, Clone, Default)]
pub struct {N}Lts {
    transitions: Vec<({N}State, String, {N}State)>,
}
impl {N}Lts {
    pub fn add_transition(&mut self, from: {N}State, label: impl Into<String>, to: {N}State) {
        self.transitions.push((from, label.into(), to));
    }
    pub fn successors(&self, state: &{N}State) -> Vec<(&str, &{N}State)> {
        self.transitions.iter()
            .filter_map(|(f, l, t)| if f == state { Some((l.as_str(), t)) } else { None })
            .collect()
    }
    /// Reachability: BFS from initial state.
    pub fn reachable(&self, initial: &{N}State) -> std::collections::HashSet<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial.0.clone());
        while let Some(s) = queue.pop_front() {
            if visited.insert(s.clone()) {
                let state = {N}State(s.clone());
                for (_, next) in self.successors(&state) { queue.push_back(next.0.clone()); }
            }
        }
        visited
    }
}"#,
            &[("N", &n), ("name", store_name)],
        ));
        out.push_str("\n\n");
    }
}
