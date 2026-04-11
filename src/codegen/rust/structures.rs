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

use super::template::ts;
use super::{to_pascal_case, RustEmitter};
use crate::ast::*;

/// Ensure a numeric string literal is a valid Rust f64 literal (has a decimal point).
/// `"0"` → `"0.0"`, `"1"` → `"1.0"`, `"0.5"` → `"0.5"` (unchanged).
fn ensure_float_lit(s: &str) -> String {
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.to_string()
    } else {
        format!("{s}.0")
    }
}

/// Convert a PascalCase or camelCase name to UPPER_SNAKE_CASE.
fn to_upper_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

/// Map a Loom scalar type name to its Rust equivalent (for pipeline step signatures).
fn loom_type_to_rust(ty: &str) -> &str {
    match ty {
        "Int" | "Integer" | "Nat" | "Index" | "Count" => "i64",
        "Float" | "Double" | "Real" => "f64",
        "Bool" | "Boolean" => "bool",
        "String" | "Str" | "Text" => "String",
        _ => ty,
    }
}

/// Map a Loom type + raw value pair to `(rust_type, rust_value)` strings.
fn map_const_type_value<'a>(ty: &'a str, value: &'a str) -> (&'a str, std::borrow::Cow<'a, str>) {
    match ty {
        "Int" | "Integer" | "Nat" | "Index" | "Count" => ("i64", std::borrow::Cow::Borrowed(value)),
        "Float" | "Double" | "Real" => {
            if value.contains('.') {
                ("f64", std::borrow::Cow::Borrowed(value))
            } else {
                ("f64", std::borrow::Cow::Owned(format!("{value}.0")))
            }
        }
        "Bool" | "Boolean" => ("bool", std::borrow::Cow::Borrowed(value)),
        "String" | "Str" | "Text" => ("&str", std::borrow::Cow::Borrowed(value)),
        // Infer from value when type is omitted
        _ => {
            if value.starts_with('"') {
                ("&str", std::borrow::Cow::Borrowed(value))
            } else if value.contains('.') {
                ("f64", std::borrow::Cow::Borrowed(value))
            } else if value == "true" || value == "false" {
                ("bool", std::borrow::Cow::Borrowed(value))
            } else {
                ("i64", std::borrow::Cow::Borrowed(value))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct stochastic process emitter from a `process:` annotation.
    pub(super) fn emit_stochastic_process(
        &self,
        fn_name: &str,
        sp: &StochasticProcessBlock,
        out: &mut String,
    ) {
        match &sp.kind {
            StochasticKind::Wiener => self.emit_wiener_process(fn_name, out),
            StochasticKind::GeometricBrownian => self.emit_gbm(fn_name, sp, out),
            StochasticKind::OrnsteinUhlenbeck => self.emit_ou_process(fn_name, sp, out),
            StochasticKind::PoissonProcess => self.emit_poisson_process(fn_name, sp, out),
            StochasticKind::MarkovChain => {
                self.emit_markov_transition_matrix(fn_name, &sp.states, out)
            }
            StochasticKind::Unknown(k) => {
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
        let mu = ensure_float_lit(sp.long_run_mean.as_deref().unwrap_or("0.0"));
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
        &self,
        fn_name: &str,
        states: &[String],
        out: &mut String,
    ) {
        let n = to_pascal_case(fn_name);
        let states_enum = states
            .iter()
            .map(|s| format!("    {},", to_pascal_case(s)))
            .collect::<Vec<_>>()
            .join("\n");
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

    /// M155: Emit a top-level Markov chain item (`chain Name ... end`).
    ///
    /// Produces `{Name}State` enum, `{Name}TransitionMatrix` struct with
    /// transitions pre-initialized from the `chain` declaration, and a
    /// `validate()` assertion on the row-stochastic property.
    pub(super) fn emit_chain_item(&self, chain: &ChainDef, out: &mut String) {
        let n = to_pascal_case(&chain.name);
        let states_enum = chain
            .states
            .iter()
            .map(|s| format!("    {},", to_pascal_case(s)))
            .collect::<Vec<_>>()
            .join("\n");

        let init_transitions = chain
            .transitions
            .iter()
            .map(|(from, to, prob)| {
                format!(
                    "        m.set({n}State::{f}, {n}State::{t}, {p:.9});",
                    n = n,
                    f = to_pascal_case(from),
                    t = to_pascal_case(to),
                    p = prob
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        out.push_str(&format!(
            "// LOOM[chain:Markov]: {name} — TransitionMatrix (Markov 1906, M155)\n\
             // P(X_{{n+1}}|X_n): memoryless discrete-state chain.\n\
             // Ecosystem: ndarray (dense), petgraph (sparse), statrs\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
             pub enum {n}State {{\n{states}\n}}\n\
             #[derive(Debug, Clone, Default)]\n\
             pub struct {n}TransitionMatrix {{\n\
             \x20\x20\x20\x20transitions: std::collections::HashMap<({n}State, {n}State), f64>,\n\
             }}\n\
             impl {n}TransitionMatrix {{\n\
             \x20\x20\x20\x20/// Create a matrix pre-initialized from the `chain {name}` declaration.\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut m = Self::default();\n\
             {inits}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20m\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20pub fn set(&mut self, from: {n}State, to: {n}State, prob: f64) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20debug_assert!((0.0..=1.0).contains(&prob), \"prob must be in [0,1]\");\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.transitions.insert((from, to), prob);\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20pub fn next_states(&self, state: {n}State) -> Vec<({n}State, f64)> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.transitions.iter()\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20.filter_map(|(&(f, t), &p)| if f == state {{ Some((t, p)) }} else {{ None }})\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20.collect()\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20/// Verify row-stochastic property: outgoing probs sum to 1.0 per state.\n\
             \x20\x20\x20\x20pub fn validate(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::collections::HashMap;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut sums: HashMap<{n}State, f64> = HashMap::new();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20for (&(from, _), &p) in &self.transitions {{ *sums.entry(from).or_default() += p; }}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20sums.values().all(|&s| (s - 1.0).abs() < 1e-9)\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n",
            name = chain.name,
            n = n,
            states = states_enum,
            inits = init_transitions,
        ));
    }

    /// M156: Emit a top-level DAG item (`dag Name nodes: [...] edges: [...] end`).
    ///
    /// Produces `{Name}Node` enum from declared nodes, and a `{Name}DagItem` struct
    /// with typed `add_edge(from: {Name}Node, to: {Name}Node)` and pre-initialized
    /// edges in `new()`, plus Kahn topological sort.
    pub(super) fn emit_dag_item(&self, dag: &DagDef, out: &mut String) {
        let n = to_pascal_case(&dag.name);

        // Node enum
        let node_variants = dag
            .nodes
            .iter()
            .map(|s| format!("    {},", to_pascal_case(s)))
            .collect::<Vec<_>>()
            .join("\n");

        // Pre-initialized edges in new()
        let init_edges = dag
            .edges
            .iter()
            .map(|(from, to)| {
                format!(
                    "        g.add_typed_edge({n}Node::{f}, {n}Node::{t});",
                    n = n,
                    f = to_pascal_case(from),
                    t = to_pascal_case(to)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        out.push_str(&format!(
            "// LOOM[dag:item]: {name} — Directed Acyclic Graph (Kahn 1962, M156)\n\
             // Ecosystem: petgraph. Kahn's algorithm: topological_sort() → None if cycle.\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
             pub enum {n}Node {{\n{nodes}\n}}\n\
             #[derive(Debug, Clone, Default)]\n\
             pub struct {n}DagItem {{\n\
             \x20\x20\x20\x20adjacency: std::collections::HashMap<{n}Node, Vec<{n}Node>>,\n\
             }}\n\
             impl {n}DagItem {{\n\
             \x20\x20\x20\x20/// Create a DAG pre-initialized from the `dag {name}` declaration.\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut g = Self::default();\n\
             {inits}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20g\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20/// Add a typed directed edge from `from` to `to`.\n\
             \x20\x20\x20\x20pub fn add_typed_edge(&mut self, from: {n}Node, to: {n}Node) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.adjacency.entry(from).or_default().push(to);\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20/// Successors of a given node.\n\
             \x20\x20\x20\x20pub fn successors(&self, node: {n}Node) -> &[{n}Node] {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.adjacency.get(&node).map(Vec::as_slice).unwrap_or(&[])\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20/// Kahn's algorithm: topological sort. Returns None if a cycle is detected.\n\
             \x20\x20\x20\x20pub fn topological_sort(&self) -> Option<Vec<{n}Node>> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::collections::{{HashMap, VecDeque}};\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut in_degree: HashMap<{n}Node, usize> = HashMap::new();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20for (&node, children) in &self.adjacency {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20in_degree.entry(node).or_default();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20for &c in children {{ *in_degree.entry(c).or_default() += 1; }}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut queue: VecDeque<{n}Node> = in_degree.iter()\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20.filter_map(|(&n, &d)| if d == 0 {{ Some(n) }} else {{ None }}).collect();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut result = Vec::new();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20while let Some(node) = queue.pop_front() {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20result.push(node);\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20for &c in self.adjacency.get(&node).unwrap_or(&vec![]) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20let d = in_degree.entry(c).or_default();\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20*d -= 1;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20if *d == 0 {{ queue.push_back(c); }}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20if result.len() == in_degree.len() {{ Some(result) }} else {{ None }}\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n",
            name = dag.name,
            n = n,
            nodes = node_variants,
            inits = init_edges,
        ));
    }

    /// M157: Emit a top-level `const` item.
    ///
    /// Maps Loom type annotations to Rust types and converts the constant name
    /// to UPPER_SNAKE_CASE. String values become `&'static str`.
    ///
    /// ```loom
    /// const MaxRetries: Int = 3          → pub const MAX_RETRIES: i64 = 3;
    /// const Timeout: Float = 30.0        → pub const TIMEOUT: f64 = 30.0;
    /// const ServiceName: String = "api"  → pub const SERVICE_NAME: &str = "api";
    /// ```
    pub(super) fn emit_const_def(&self, cd: &ConstDef, out: &mut String) {
        let rust_name = to_upper_snake(&cd.name);
        let (rust_type, rust_value) = map_const_type_value(&cd.ty, &cd.value);
        out.push_str(&format!(
            "// LOOM[const:item]: {name} — named constant (M157)\n\
             pub const {rust_name}: {rust_type} = {rust_value};\n\n",
            name = cd.name,
        ));
    }

    /// M159: Emit a top-level `pipeline` item as a named processing chain.
    pub(super) fn emit_pipeline_def(&self, pd: &PipelineDef, out: &mut String) {
        let struct_name = format!("{}Pipeline", pd.name);

        out.push_str(&format!(
            "// LOOM[pipeline:item]: {name} — sequential transformation pipeline (M159)\n\
             pub struct {struct_name};\n\n\
             impl {struct_name} {{\n",
            name = pd.name,
        ));

        for step in &pd.steps {
            let rust_in = loom_type_to_rust(&step.input_ty);
            let rust_out = loom_type_to_rust(&step.output_ty);
            out.push_str(&format!(
                "    // LOOM[pipeline:step]: {step_name} — {in_ty} \u{2192} {out_ty}\n\
                     pub fn {step_name}(&self, input: {rust_in}) -> {rust_out} {{\n\
                 \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement {step_name}\")\n\
                 \x20\x20\x20\x20}}\n",
                step_name = step.name,
                in_ty = step.input_ty,
                out_ty = step.output_ty,
            ));
        }

        if let Some(first) = pd.steps.first() {
            let rust_in = loom_type_to_rust(&first.input_ty);
            let final_step = pd.steps.last().unwrap();
            let rust_out = loom_type_to_rust(&final_step.output_ty);

            let chain = if pd.steps.len() == 1 {
                format!("self.{}(input)", first.name)
            } else {
                let mut chain = format!("self.{}(input)", first.name);
                for step in pd.steps.iter().skip(1) {
                    chain = format!("self.{}({})", step.name, chain);
                }
                chain
            };

            out.push_str(&format!(
                "\n    /// Run all pipeline steps in declaration order.\n\
                 \x20\x20\x20\x20pub fn process(&self, input: {rust_in}) -> {rust_out} {{\n\
                 \x20\x20\x20\x20\x20\x20\x20\x20{chain}\n\
                 \x20\x20\x20\x20}}\n",
            ));
        }

        out.push_str("}\n\n");
    }

    /// M160: Emit a top-level `saga` item as a distributed transaction coordinator.
    ///
    /// Reference: Garcia-Molina & Salem, "SAGAS" (SIGMOD 1987).
    pub(super) fn emit_saga_def(&self, sd: &SagaDef, out: &mut String) {
        // sd.name is the user-provided name, e.g. "OrderSaga" or "Deploy".
        // Struct name = sd.name (user already decides whether to include "Saga" suffix).
        // Error enum = {Name}Error (avoids "OrderSagaSagaError" double-suffix).
        let struct_name = &sd.name;
        let error_name = format!("{}Error", sd.name);

        let error_variants: String = sd
            .steps
            .iter()
            .map(|s| {
                format!(
                    "    // LOOM[saga:error]: step {} failed\n    {step}Failed,\n",
                    s.name,
                    step = to_pascal_case(&s.name),
                )
            })
            .collect();

        out.push_str(&format!(
            "// LOOM[saga:item]: {name} — distributed transaction saga (M160, Garcia-Molina 1987)\n\
             pub struct {struct_name};\n\n\
             #[derive(Debug)]\n\
             pub enum {error_name} {{\n\
             {error_variants}\
             }}\n\n\
             impl {struct_name} {{\n",
            name = sd.name,
        ));

        for step in &sd.steps {
            let rust_in = loom_type_to_rust(&step.input_ty);
            let rust_out = loom_type_to_rust(&step.output_ty);
            out.push_str(&format!(
                "    // LOOM[saga:step]: {step_name} — forward transaction\n\
                     pub fn {step_name}(&self, input: {rust_in}) -> Result<{rust_out}, {error_name}> {{\n\
                 \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement {step_name}\")\n\
                 \x20\x20\x20\x20}}\n",
                step_name = step.name,
                error_name = error_name,
            ));

            if let Some(comp) = &step.compensate {
                let comp_in = loom_type_to_rust(&comp.input_ty);
                let comp_out = loom_type_to_rust(&comp.output_ty);
                out.push_str(&format!(
                    "    // LOOM[saga:compensate]: {step_name} — compensating transaction (rollback)\n\
                         pub fn {step_name}_compensate(&self, input: {comp_in}) -> {comp_out} {{\n\
                     \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement {step_name} compensation\")\n\
                     \x20\x20\x20\x20}}\n",
                    step_name = comp.step_name,
                    comp_out = comp_out,
                ));
            }
        }

        if let Some(first) = sd.steps.first() {
            let rust_in = loom_type_to_rust(&first.input_ty);
            let last = sd.steps.last().unwrap();
            let rust_out = loom_type_to_rust(&last.output_ty);

            let mut body_lines = vec![format!(
                "        let step0 = self.{}(input)?;",
                first.name
            )];
            for (i, step) in sd.steps.iter().enumerate().skip(1) {
                body_lines.push(format!(
                    "        let step{i} = self.{}(step{})?;",
                    step.name,
                    i - 1
                ));
            }
            let last_var = if sd.steps.len() == 1 {
                "step0".to_string()
            } else {
                format!("step{}", sd.steps.len() - 1)
            };
            body_lines.push(format!("        Ok({last_var})"));
            let body = body_lines.join("\n");

            out.push_str(&format!(
                "\n    /// Execute the full saga. On failure, invoke compensating transactions.\n\
                 \x20\x20\x20\x20pub fn execute(&self, input: {rust_in}) -> Result<{rust_out}, {error_name}> {{\n\
                 {body}\n\
                 \x20\x20\x20\x20}}\n",
                error_name = error_name,
            ));
        }

        out.push_str("}\n\n");
    }

    /// M161: Emit `{Name}Event` struct + `{Name}EventHandler` trait.
    ///
    /// Domain events carry immutable typed payload. The handler trait decouples
    /// the event from its consumers (Observer / EventBus pattern).
    pub(super) fn emit_event_def(&self, ed: &EventDef, out: &mut String) {
        let name = &ed.name;
        out.push_str(&format!(
            "// LOOM[event:domain]: {name} — M161 domain event\n\
             #[derive(Debug, Clone, PartialEq)]\n\
             pub struct {name}Event {{\n"
        ));
        for (field, ty) in &ed.fields {
            let rust_ty = loom_type_to_rust(ty);
            out.push_str(&format!("    pub {field}: {rust_ty},\n"));
        }
        out.push_str("}\n\n");
        out.push_str(&format!(
            "pub trait {name}EventHandler {{\n\
             \x20\x20\x20\x20fn handle(&self, event: &{name}Event);\n\
             }}\n\n"
        ));
    }

    /// M162: Emit `{Name}Command` struct + `{Name}Handler` trait (CQRS command side).
    pub(super) fn emit_command_def(&self, cd: &CommandDef, out: &mut String) {
        let name = &cd.name;
        out.push_str(&format!(
            "// LOOM[command:cqrs]: {name} — M162 CQRS command\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Command {{\n"
        ));
        for (field, ty) in &cd.fields {
            let rust_ty = loom_type_to_rust(ty);
            out.push_str(&format!("    pub {field}: {rust_ty},\n"));
        }
        out.push_str("}\n\n");
        out.push_str(&format!(
            "pub trait {name}Handler {{\n\
             \x20\x20\x20\x20fn handle(&self, cmd: {name}Command) -> Result<(), String>;\n\
             }}\n\n"
        ));
    }

    /// M162: Emit `{Name}Query` struct + `{Name}QueryHandler<R>` trait (CQRS query side).
    pub(super) fn emit_query_def(&self, qd: &QueryDef, out: &mut String) {
        let name = &qd.name;
        out.push_str(&format!(
            "// LOOM[query:cqrs]: {name} — M162 CQRS query\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Query {{\n"
        ));
        for (field, ty) in &qd.fields {
            let rust_ty = loom_type_to_rust(ty);
            out.push_str(&format!("    pub {field}: {rust_ty},\n"));
        }
        out.push_str("}\n\n");
        out.push_str(&format!(
            "pub trait {name}QueryHandler<R> {{\n\
             \x20\x20\x20\x20fn handle(&self, query: {name}Query) -> R;\n\
             }}\n\n"
        ));
    }

    /// M163: Emit circuit breaker struct + state enum + impl (Nygard 2007).
    pub(super) fn emit_circuit_breaker_def(&self, cb: &CircuitBreakerDef, out: &mut String) {
        let name = &cb.name;
        let threshold = cb.threshold;
        let timeout = cb.timeout;
        out.push_str(&format!(
            "// LOOM[circuit_breaker:resilience]: {name} — M163 (Nygard 2007, \"Release It!\")\n\
             #[derive(Debug, Clone, PartialEq)]\n\
             pub enum {name}CircuitState {{ Closed, Open, HalfOpen }}\n\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}CircuitBreaker {{\n\
             \x20\x20\x20\x20pub failure_threshold: u32,\n\
             \x20\x20\x20\x20pub timeout_secs: u64,\n\
             \x20\x20\x20\x20pub state: {name}CircuitState,\n\
             \x20\x20\x20\x20failures: u32,\n\
             }}\n\n\
             impl {name}CircuitBreaker {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20failure_threshold: {threshold},\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20timeout_secs: {timeout},\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20state: {name}CircuitState::Closed,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20failures: 0,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[circuit_breaker:call]: wraps a remote call, tracks failures\n\
             \x20\x20\x20\x20pub fn call<F, T>(&mut self, f: F) -> Result<T, String>\n\
             \x20\x20\x20\x20where\n\
             \x20\x20\x20\x20\x20\x20\x20\x20F: FnOnce() -> Result<T, String>,\n\
             \x20\x20\x20\x20{{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement circuit breaker call logic\")\n\
             \x20\x20\x20\x20}}\n"
        ));
        if !cb.fallback.is_empty() {
            let fb = &cb.fallback;
            out.push_str(&format!(
                "\n\x20\x20\x20\x20// LOOM[circuit_breaker:fallback]: {fb} — invoked when circuit is open\n\
                 \x20\x20\x20\x20pub fn fallback_{fb}(&self) -> Result<(), String> {{\n\
                 \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement fallback: {fb}\")\n\
                 \x20\x20\x20\x20}}\n"
            ));
        }
        out.push_str("}\n\n");
    }

    /// M164: Emit `{Name}Policy` struct + `execute<F,T,E>()` (exponential backoff retry).
    pub(super) fn emit_retry_def(&self, rd: &RetryDef, out: &mut String) {
        let name = &rd.name;
        let max_attempts = rd.max_attempts;
        let base_delay = rd.base_delay;
        let multiplier = rd.multiplier;
        out.push_str(&format!(
            "// LOOM[retry:resilience]: {name} — M164 exponential backoff (Tanenbaum 2007)\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Policy {{\n\
             \x20\x20\x20\x20pub max_attempts: u32,\n\
             \x20\x20\x20\x20pub base_delay_ms: u64,\n\
             \x20\x20\x20\x20pub multiplier: u32,\n\
             }}\n\n\
             impl {name}Policy {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ max_attempts: {max_attempts}, base_delay_ms: {base_delay}, multiplier: {multiplier} }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[retry:execute]: wraps a fallible call with exponential backoff\n\
             \x20\x20\x20\x20pub fn execute<F, T, E>(&self, f: F) -> Result<T, E>\n\
             \x20\x20\x20\x20where\n\
             \x20\x20\x20\x20\x20\x20\x20\x20F: Fn() -> Result<T, E>,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20E: std::fmt::Debug,\n\
             \x20\x20\x20\x20{{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement retry with exponential backoff\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M165: Emit `{Name}RateLimiter` struct + `allow()` method (token bucket, Anderson 1990).
    pub(super) fn emit_rate_limiter_def(&self, rl: &RateLimiterDef, out: &mut String) {
        let name = &rl.name;
        let requests = rl.requests;
        let per = rl.per;
        let burst = rl.burst;
        out.push_str(&format!(
            "// LOOM[rate_limiter:resilience]: {name} — M165 token bucket (Anderson 1990)\n\
             // requests: {requests} per {per}s, burst: {burst}\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}RateLimiter {{\n\
             \x20\x20\x20\x20pub requests_per_window: u64,\n\
             \x20\x20\x20\x20pub window_secs: u64,\n\
             \x20\x20\x20\x20pub burst_capacity: u64,\n\
             }}\n\n\
             impl {name}RateLimiter {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ requests_per_window: {requests}, window_secs: {per}, burst_capacity: {burst} }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[rate_limiter:allow]: token bucket admission check\n\
             \x20\x20\x20\x20pub fn allow(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement token bucket allow()\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M166: Emit `{Name}Cache<K,V>` generic struct + get/set/evict methods (TTL-aware cache).
    pub(super) fn emit_cache_def(&self, cd: &CacheDef, out: &mut String) {
        let name = &cd.name;
        let key = &cd.key_type;
        let val = &cd.value_type;
        let ttl = cd.ttl;
        out.push_str(&format!(
            "// LOOM[cache:performance]: {name} — M166 typed cache with TTL={ttl}s\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Cache<K = {key}, V = {val}> {{\n\
             \x20\x20\x20\x20pub ttl_secs: u64,\n\
             \x20\x20\x20\x20_phantom: std::marker::PhantomData<(K, V)>,\n\
             }}\n\n\
             impl<K, V> {name}Cache<K, V>\n\
             where\n\
             \x20\x20\x20\x20K: std::hash::Hash + Eq + Clone,\n\
             \x20\x20\x20\x20V: Clone,\n\
             {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ ttl_secs: {ttl}, _phantom: std::marker::PhantomData }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[cache:get]: retrieve value by key (None if missing or expired)\n\
             \x20\x20\x20\x20pub fn get(&self, _key: &K) -> Option<V> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement cache get\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[cache:set]: insert a key-value pair, resetting TTL\n\
             \x20\x20\x20\x20pub fn set(&mut self, _key: K, _value: V) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement cache set\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[cache:evict]: remove expired entries\n\
             \x20\x20\x20\x20pub fn evict(&mut self) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement cache evict\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M167: Emit `{Name}Bulkhead` struct + `execute<F,T,E>()` (Nygard 2007 Release It!).
    pub(super) fn emit_bulkhead_def(&self, bd: &BulkheadDef, out: &mut String) {
        let name = &bd.name;
        let max_concurrent = bd.max_concurrent;
        let queue_size = bd.queue_size;
        out.push_str(&format!(
            "// LOOM[bulkhead:resilience]: {name} — M167 isolation (Nygard 2007 Release It!)\n\
             // max_concurrent: {max_concurrent}, queue_size: {queue_size}\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Bulkhead {{\n\
             \x20\x20\x20\x20pub max_concurrent: u64,\n\
             \x20\x20\x20\x20pub queue_size: u64,\n\
             }}\n\n\
             impl {name}Bulkhead {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ max_concurrent: {max_concurrent}, queue_size: {queue_size} }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[bulkhead:execute]: run f() if a slot is available\n\
             \x20\x20\x20\x20pub fn execute<F, T, E>(&self, f: F) -> Result<T, E>\n\
             \x20\x20\x20\x20where\n\
             \x20\x20\x20\x20\x20\x20\x20\x20F: FnOnce() -> Result<T, E>,\n\
             \x20\x20\x20\x20{{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement bulkhead execute\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[bulkhead:available]: true if a concurrent slot is free\n\
             \x20\x20\x20\x20pub fn available(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement bulkhead available\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M168: Emit `{Name}Timeout` struct + `execute<F,T>()` deadline wrapper.
    pub(super) fn emit_timeout_def(&self, td: &TimeoutDef, out: &mut String) {
        let name = &td.name;
        let duration = td.duration;
        let unit = &td.unit;
        out.push_str(&format!(
            "// LOOM[timeout:resilience]: {name} — M168 deadline enforcement\n\
             // deadline: {duration}{unit}\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Timeout {{\n\
             \x20\x20\x20\x20pub duration: u64,\n\
             \x20\x20\x20\x20pub unit: &'static str,\n\
             }}\n\n\
             impl {name}Timeout {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ duration: {duration}, unit: \"{unit}\" }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[timeout:execute]: run f() or return Err on deadline exceeded\n\
             \x20\x20\x20\x20pub fn execute<F, T>(&self, f: F) -> Result<T, String>\n\
             \x20\x20\x20\x20where\n\
             \x20\x20\x20\x20\x20\x20\x20\x20F: FnOnce() -> T,\n\
             \x20\x20\x20\x20{{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement timeout execute\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M169: Emit `{Name}Fallback<T>` struct + `get() -> T` static fallback.
    pub(super) fn emit_fallback_item_def(&self, fd: &FallbackItemDef, out: &mut String) {
        let name = &fd.name;
        let value = &fd.value;
        let value_display = if value.is_empty() { "default" } else { value.as_str() };
        out.push_str(&format!(
            "// LOOM[fallback:resilience]: {name} — M169 static fallback value\n\
             // fallback value: \"{value_display}\"\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Fallback<T = String> {{\n\
             \x20\x20\x20\x20pub value: T,\n\
             }}\n\n\
             impl {name}Fallback {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ value: \"{value}\".to_string() }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[fallback:get]: return the static fallback value\n\
             \x20\x20\x20\x20pub fn get(&self) -> &String {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20&self.value\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M170: Emit `{Name}Observer<T>` struct + subscribe/notify/get (GoF Observer).
    pub(super) fn emit_observer_def(&self, od: &ObserverDef, out: &mut String) {
        let name = &od.name;
        let ty = &od.observed_type;
        out.push_str(&format!(
            "// LOOM[observer:behavioral]: {name} — M170 observable value (GoF Observer)\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Observer<T = {ty}> {{\n\
             \x20\x20\x20\x20pub value: T,\n\
             }}\n\n\
             impl<T: Clone> {name}Observer<T> {{\n\
             \x20\x20\x20\x20pub fn new(initial: T) -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ value: initial }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[observer:get]: read the current observed value\n\
             \x20\x20\x20\x20pub fn get(&self) -> &T {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20&self.value\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[observer:notify]: update value and notify subscribers\n\
             \x20\x20\x20\x20pub fn notify(&mut self, new_value: T) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.value = new_value;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"notify subscribers\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[observer:subscribe]: register a callback\n\
             \x20\x20\x20\x20pub fn subscribe<F: Fn(&T)>(&mut self, _callback: F) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"register subscriber\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M171: Emit `{Name}Pool<T>` struct + acquire/release (object pool pattern).
    pub(super) fn emit_pool_def(&self, pd: &PoolDef, out: &mut String) {
        let name = &pd.name;
        let size = pd.size;
        out.push_str(&format!(
            "// LOOM[pool:performance]: {name} — M171 object pool (Gamma et al. 1994)\n\
             // capacity: {size}\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Pool<T> {{\n\
             \x20\x20\x20\x20pub capacity: usize,\n\
             \x20\x20\x20\x20_phantom: std::marker::PhantomData<T>,\n\
             }}\n\n\
             impl<T> {name}Pool<T> {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ capacity: {size}, _phantom: std::marker::PhantomData }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[pool:acquire]: get an item from the pool\n\
             \x20\x20\x20\x20pub fn acquire(&mut self) -> Option<T> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement pool acquire\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[pool:release]: return an item to the pool\n\
             \x20\x20\x20\x20pub fn release(&mut self, _item: T) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement pool release\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M172: Emit `{Name}Scheduler` struct + run/stop methods.
    pub(super) fn emit_scheduler_def(&self, sd: &SchedulerDef, out: &mut String) {
        let name = &sd.name;
        let interval = sd.interval;
        let unit = &sd.unit;
        out.push_str(&format!(
            "// LOOM[scheduler:behavioral]: {name} — M172 periodic scheduler\n\
             // interval: {interval}{unit}\n\
             #[derive(Debug, Clone)]\n\
             pub struct {name}Scheduler {{\n\
             \x20\x20\x20\x20pub interval: u64,\n\
             \x20\x20\x20\x20pub unit: &'static str,\n\
             }}\n\n\
             impl {name}Scheduler {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ interval: {interval}, unit: \"{unit}\" }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[scheduler:run]: start the periodic task\n\
             \x20\x20\x20\x20pub fn run<F: Fn()>(&self, _task: F) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement scheduler run\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[scheduler:stop]: halt the periodic task\n\
             \x20\x20\x20\x20pub fn stop(&mut self) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement scheduler stop\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M173: Emit `{Name}Queue<T>` — FIFO/LIFO named queue.
    pub(super) fn emit_queue_def(&self, qd: &QueueDef, out: &mut String) {
        let name = &qd.name;
        let capacity = qd.capacity;
        let kind = &qd.kind;
        let cap_comment = if capacity == 0 {
            "unbounded".to_string()
        } else {
            format!("capacity: {capacity}")
        };
        out.push_str(&format!(
            "// LOOM[queue:concurrency]: {name} — M173 {kind} queue ({cap_comment})\n\
             #[derive(Debug)]\n\
             pub struct {name}Queue<T> {{\n\
             \x20\x20\x20\x20inner: std::collections::VecDeque<T>,\n\
             \x20\x20\x20\x20pub capacity: usize,\n\
             }}\n\n\
             impl<T> {name}Queue<T> {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ inner: std::collections::VecDeque::new(), capacity: {capacity} }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[queue:enqueue]: add item\n\
             \x20\x20\x20\x20pub fn enqueue(&mut self, item: T) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20if self.capacity > 0 && self.inner.len() >= self.capacity {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20return false;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.inner.push_back(item);\n\
             \x20\x20\x20\x20\x20\x20\x20\x20true\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[queue:dequeue]: remove and return next item\n\
             \x20\x20\x20\x20pub fn dequeue(&mut self) -> Option<T> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.inner.pop_front()\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[queue:is_empty]: true when no items\n\
             \x20\x20\x20\x20pub fn is_empty(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.inner.is_empty()\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M174: Emit `{Name}Lock` — named mutex-style lock.
    pub(super) fn emit_lock_def(&self, ld: &LockDef, out: &mut String) {
        let name = &ld.name;
        out.push_str(&format!(
            "// LOOM[lock:concurrency]: {name} — M174 named mutex-style lock\n\
             #[derive(Debug)]\n\
             pub struct {name}Lock {{\n\
             \x20\x20\x20\x20locked: std::sync::atomic::AtomicBool,\n\
             }}\n\n\
             impl {name}Lock {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ locked: std::sync::atomic::AtomicBool::new(false) }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[lock:acquire]: acquire the lock; returns false if already held\n\
             \x20\x20\x20\x20pub fn acquire(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.locked\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20.is_ok()\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[lock:release]: release the lock\n\
             \x20\x20\x20\x20pub fn release(&self) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.locked.store(false, Ordering::Release);\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[lock:is_locked]: true when currently held\n\
             \x20\x20\x20\x20pub fn is_locked(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.locked.load(Ordering::Relaxed)\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M175: Emit `{Name}Channel<T>` — typed MPSC channel.
    pub(super) fn emit_channel_def(&self, cd: &ChannelDef, out: &mut String) {
        let name = &cd.name;
        let ty = &cd.element_type;
        let capacity = cd.capacity;
        let cap_comment = if capacity == 0 {
            "unbounded".to_string()
        } else {
            format!("capacity: {capacity}")
        };
        out.push_str(&format!(
            "// LOOM[channel:concurrency]: {name} — M175 MPSC channel ({cap_comment})\n\
             // element type: {ty}\n\
             #[derive(Debug)]\n\
             pub struct {name}Channel<T = {ty}> {{\n\
             \x20\x20\x20\x20pub capacity: usize,\n\
             \x20\x20\x20\x20_phantom: std::marker::PhantomData<T>,\n\
             }}\n\n\
             impl<T> {name}Channel<T> {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ capacity: {capacity}, _phantom: std::marker::PhantomData }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[channel:send]: send a value into the channel\n\
             \x20\x20\x20\x20pub fn send(&self, _value: T) -> Result<(), String> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement channel send\")\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[channel:recv]: receive the next value\n\
             \x20\x20\x20\x20pub fn recv(&self) -> Option<T> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20todo!(\"implement channel recv\")\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M176: Emit `{Name}Semaphore` — counting semaphore.
    pub(super) fn emit_semaphore_def(&self, sd: &SemaphoreDef, out: &mut String) {
        let name = &sd.name;
        let permits = sd.permits;
        out.push_str(&format!(
            "// LOOM[semaphore:concurrency]: {name} — M176 counting semaphore (permits: {permits})\n\
             #[derive(Debug)]\n\
             pub struct {name}Semaphore {{\n\
             \x20\x20\x20\x20count: std::sync::atomic::AtomicUsize,\n\
             }}\n\n\
             impl {name}Semaphore {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ count: std::sync::atomic::AtomicUsize::new({permits}) }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[semaphore:wait]: acquire a permit; returns false if none available\n\
             \x20\x20\x20\x20pub fn wait(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let mut current = self.count.load(Ordering::Acquire);\n\
             \x20\x20\x20\x20\x20\x20\x20\x20loop {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20if current == 0 {{ return false; }}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20match self.count.compare_exchange_weak(current, current - 1, Ordering::AcqRel, Ordering::Acquire) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20Ok(_) => return true,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20Err(c) => current = c,\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[semaphore:signal]: release a permit\n\
             \x20\x20\x20\x20pub fn signal(&self) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.count.fetch_add(1, Ordering::Release);\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[semaphore:count]: current available permits\n\
             \x20\x20\x20\x20pub fn count(&self) -> usize {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.count.load(Ordering::Relaxed)\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M177: Emit `{Name}Actor<M>` — lightweight actor with mailbox.
    pub(super) fn emit_actor_def(&self, ad: &ActorDef, out: &mut String) {
        let name = &ad.name;
        let msg = &ad.message_type;
        out.push_str(&format!(
            "// LOOM[actor:concurrency]: {name} — M177 lightweight actor (message: {msg})\n\
             #[derive(Debug)]\n\
             pub struct {name}Actor<M = {msg}> {{\n\
             \x20\x20\x20\x20mailbox: std::collections::VecDeque<M>,\n\
             }}\n\n\
             impl<M> {name}Actor<M> {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ mailbox: std::collections::VecDeque::new() }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[actor:send]: enqueue a message into the mailbox\n\
             \x20\x20\x20\x20pub fn send(&mut self, message: M) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.mailbox.push_back(message);\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[actor:receive]: dequeue the next message\n\
             \x20\x20\x20\x20pub fn receive(&mut self) -> Option<M> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.mailbox.pop_front()\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[actor:pending]: number of queued messages\n\
             \x20\x20\x20\x20pub fn pending(&self) -> usize {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.mailbox.len()\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }

    /// M178: Emit `{Name}Barrier` — N-thread synchronization barrier.
    pub(super) fn emit_barrier_def(&self, bd: &BarrierDef, out: &mut String) {
        let name = &bd.name;
        let count = bd.count;
        out.push_str(&format!(
            "// LOOM[barrier:concurrency]: {name} — M178 synchronization barrier (count: {count})\n\
             #[derive(Debug)]\n\
             pub struct {name}Barrier {{\n\
             \x20\x20\x20\x20pub count: usize,\n\
             \x20\x20\x20\x20arrived: std::sync::atomic::AtomicUsize,\n\
             }}\n\n\
             impl {name}Barrier {{\n\
             \x20\x20\x20\x20pub fn new() -> Self {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Self {{ count: {count}, arrived: std::sync::atomic::AtomicUsize::new(0) }}\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[barrier:wait]: signal arrival and wait until all count threads arrive\n\
             \x20\x20\x20\x20pub fn wait(&self) -> bool {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20let prev = self.arrived.fetch_add(1, Ordering::AcqRel);\n\
             \x20\x20\x20\x20\x20\x20\x20\x20prev + 1 >= self.count\n\
             \x20\x20\x20\x20}}\n\n\
             \x20\x20\x20\x20// LOOM[barrier:reset]: reset arrival counter\n\
             \x20\x20\x20\x20pub fn reset(&self) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20use std::sync::atomic::Ordering;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20self.arrived.store(0, Ordering::Release);\n\
             \x20\x20\x20\x20}}\n\
             }}\n\n"
        ));
    }
}
// ═══════════════════════════════════════════════════════════════════════════

impl RustEmitter {
    /// Dispatch to the correct distribution sampler from a `distribution:` annotation.
    pub(super) fn emit_distribution_sampler(
        &self,
        fn_name: &str,
        db: &DistributionBlock,
        out: &mut String,
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
pub fn new() -> Self {{ Self {{ mean: {}, std_dev: {} }} }}\n    \
/// Box-Muller transform. z1, z2 ~ U(0,1). Returns one N(0,1) sample.\n    \
pub fn sample_box_muller(&self, z1: f64, z2: f64) -> f64 {{\n        \
let n01 = (-2.0*z1.ln()).sqrt() * (2.0*std::f64::consts::PI*z2).cos();\n        \
self.mean + self.std_dev * n01\n    }}\n}}\n\n",
                    ensure_float_lit(mean),
                    ensure_float_lit(std_dev)
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
pub fn new() -> Self {{ Self {{ location: {}, scale: {} }} }}\n    \
// Inverse CDF: X = location + scale*tan(pi*(u - 0.5))\n    \
pub fn sample(&self, u: f64) -> f64 {{\n        \
self.location + self.scale * (std::f64::consts::PI * (u - 0.5)).tan()\n    }}\n}}\n\n",
                    ensure_float_lit(location),
                    ensure_float_lit(scale)
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
                    "impl {n}LevySampler {{ pub fn new() -> Self {{ Self {{ location: {}, scale: {} }} }} }}\n\n",
                        ensure_float_lit(location), ensure_float_lit(scale)
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
