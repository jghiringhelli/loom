//! Being and ecosystem emitters — `being:` / `ecosystem:` blocks → Rust structs + impl.

use super::{pascal_case, to_snake_case, RustEmitter};
use crate::ast::*;

impl RustEmitter {
    /// Emit all ecosystem definitions as Rust submodules.
    pub(super) fn emit_ecosystem(&self, eco: &EcosystemDef) -> String {
        let mut out = String::new();
        let mod_name = to_snake_case(&eco.name);

        out.push_str(&format!("// Ecosystem: {}\n", eco.name));
        if let Some(telos) = &eco.telos {
            out.push_str(&format!("// telos: {:?}\n", telos));
        }
        if !eco.members.is_empty() {
            out.push_str(&format!("// members: {}\n", eco.members.join(", ")));
        }
        out.push_str(&format!("pub mod {} {{\n", mod_name));
        out.push_str("    use super::*;\n");

        for sig in &eco.signals {
            out.push_str(&format!(
                "\n    /// Signal: {} ({} → {})\n",
                sig.name, sig.from, sig.to
            ));
            out.push_str(&format!("    pub struct {} {{\n", sig.name));
            out.push_str(&format!(
                "        pub payload: {}, // {}\n",
                self.payload_to_rust_type(&sig.payload),
                sig.payload
            ));
            out.push_str("    }\n");
        }

        let params: Vec<String> = eco
            .members
            .iter()
            .map(|m| format!("{}: &mut {}", to_snake_case(m), m))
            .collect();
        if let Some(telos) = &eco.telos {
            out.push_str("\n    /// Coordinate the ecosystem: route signals between members.\n");
            out.push_str(&format!("    /// telos: {}\n", telos));
        } else {
            out.push_str("\n    /// Coordinate the ecosystem: route signals between members.\n");
        }
        out.push_str(&format!(
            "    pub fn coordinate({}) {{\n",
            params.join(", ")
        ));
        out.push_str("        todo!(\"implement ecosystem coordination toward telos\")\n");
        out.push_str("    }\n");

        for quorum in &eco.quorum_blocks {
            let signal_snake = to_snake_case(&quorum.signal);
            let threshold_f64: f64 = quorum.threshold.parse().unwrap_or(0.0);
            out.push_str(&format!(
                "\n    /// Quorum sensing: {} at {} population fraction → {}\n",
                quorum.signal, quorum.threshold, quorum.action
            ));
            out.push_str(
                "    /// Bassler (1999): collective behavior emerging from individual signals.\n",
            );
            out.push_str(&format!(
                "    pub fn check_quorum_{}(population_signals: &[f64]) -> bool {{\n",
                signal_snake
            ));
            out.push_str(
                "        let fraction = population_signals.iter().filter(|&&s| s > 0.0).count() as f64\n"
            );
            out.push_str("            / population_signals.len() as f64;\n");
            out.push_str(&format!(
                "        if fraction >= {}_f64 {{\n",
                threshold_f64
            ));
            out.push_str(&format!("            // trigger: {}\n", quorum.action));
            out.push_str(&format!(
                "            todo!(\"implement quorum action: {}\")\n        }}\n",
                quorum.action
            ));
            out.push_str(&format!(
                "        fraction >= {}_f64\n    }}\n",
                threshold_f64
            ));
        }

        out.push_str("}\n");
        out
    }

    /// Map a payload type string to a Rust type.
    pub(super) fn payload_to_rust_type(&self, payload: &str) -> String {
        let base = payload.split('<').next().unwrap_or(payload).trim();
        match base {
            "Float" => "f64".to_string(),
            "Int" => "i64".to_string(),
            "String" | "Str" => "String".to_string(),
            "Bool" => "bool".to_string(),
            other => other.to_string(),
        }
    }

    /// Emit a being definition as a Rust struct + impl with fitness/regulate/evolve methods.
    pub(super) fn emit_being(&self, being: &BeingDef) -> String {
        let mut out = String::new();
        let telos_desc = being
            .telos
            .as_ref()
            .map(|t| t.description.as_str())
            .unwrap_or("");

        out.push_str(&format!("// Being: {}\n", being.name));
        if let Some(telos) = &being.telos {
            out.push_str(&format!("// telos: {:?}\n", telos.description));
        }
        if let Some(desc) = &being.describe {
            out.push_str(&format!("/// {}\n", desc));
        }
        // LOOM[propagate]: emit propagation metadata
        if let Some(prop) = &being.propagate_block {
            out.push_str(&format!(
                "// LOOM[propagate]: condition={}, inherits=[{}], mutates=[{}]\n",
                prop.condition,
                prop.inherits.join(", "),
                prop.mutates.iter().map(|(f, c)| format!("{} {}", f, c)).collect::<Vec<_>>().join("; ")
            ));
            if let Some(ot) = &prop.offspring_type {
                out.push_str(&format!("// LOOM[propagate]: offspring_type={}\n", ot));
            }
        }

        // ── Telos convergence: emit threshold constants + convergence state ──
        if let Some(telos) = &being.telos {
            if let Some(thresholds) = &telos.thresholds {
                let name_upper = being.name.to_uppercase();
                out.push_str(&format!(
                    "pub const {name_upper}_CONVERGENCE_THRESHOLD: f64  = {:.3};\n\
pub const {name_upper}_WARNING_THRESHOLD:     f64  = {:.3};\n\
pub const {name_upper}_DIVERGENCE_THRESHOLD:  f64  = {:.3};\n\n",
                    thresholds.convergence,
                    thresholds.warning.unwrap_or(thresholds.divergence),
                    thresholds.divergence,
                ));
                let name_pascal = &being.name;
                out.push_str(&format!(
                    "/// Telos convergence state for `{name_pascal}` (Aristotle/Varela 1972).\n\
/// Determined by comparing `fitness()` score against declared thresholds.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub enum {name_pascal}ConvergenceState {{\n\
    /// fitness >= {conv:.3}: being is converging toward telos.\n\
    Converging,\n\
    /// warning <= fitness < {conv:.3}: under stress, homeostasis active.\n\
    Warning,\n\
    /// fitness < {div:.3}: diverging, apoptosis candidate.\n\
    Diverging,\n\
}}\n\n",
                    conv = thresholds.convergence,
                    div = thresholds.divergence,
                ));
            }

            // TLA+ convergence spec embedded as a const string for external verification.
            let desc_safe = telos.description.replace('"', "'");
            let name_upper = being.name.to_uppercase();
            out.push_str(&format!(
                "/// TLA+ convergence specification for `{}` (extract and run with TLC).\n\
/// Invariant: fitness is monotonically non-decreasing toward telos.\n\
pub const {name_upper}_TLA_SPEC: &str = r#\"\n\
---- MODULE {name}ConvergenceCheck ----\n\
EXTENDS Reals\n\
\n\
CONSTANT ConvergenceThreshold, DivergenceThreshold\n\
VARIABLES fitness, state\n\
\n\
(* telos: {desc} *)\n\
TypeInvariant ==\n\
  /\\ fitness \\in REAL\n\
  /\\ state \\in {{\"converging\", \"warning\", \"diverging\"}}\n\
\n\
TelosConverged == fitness >= ConvergenceThreshold\n\
TelosDiverged  == fitness < DivergenceThreshold\n\
\n\
(* Liveness: the being eventually converges *)\n\
ConvergenceProperty == []<>TelosConverged\n\
\n\
(* Safety: once converged, fitness never drops below divergence *)\n\
NonDegeneracy == [](TelosConverged => ~TelosDiverged)\n\
\n\
====\n\
\"#;\n\n",
                being.name,
                name = being.name,
                desc = desc_safe,
            ));
        }

        out.push_str("#[derive(Debug, Clone)]\n");
        out.push_str(&format!("pub struct {} {{\n", being.name));
        if let Some(matter) = &being.matter {
            for field in &matter.fields {
                out.push_str(&format!(
                    "    pub {}: {},\n",
                    field.name,
                    self.emit_type_expr(&field.ty)
                ));
            }
        }
        if being.telomere.is_some() {
            out.push_str("    pub telomere_count: u64,\n");
        }
        out.push_str("}\n\n");

        out.push_str(&format!("impl {} {{\n", being.name));

        // Fitness method: if thresholds are declared, emit convergence_state() too.
        let fitness_todo = if let Some(t) = &being.telos {
            if let Some(ff) = &t.fitness_fn {
                format!("implement fitness: {}", ff)
            } else {
                format!("implement fitness toward telos: {}", t.description)
            }
        } else {
            "implement fitness".to_string()
        };
        out.push_str(
            "    /// Returns the fitness score relative to telos (0.0 = worst, 1.0 = perfect).\n",
        );
        out.push_str(&format!("    /// telos: {:?}\n", telos_desc));
        out.push_str(&format!(
            "    pub fn fitness(&self) -> f64 {{\n        todo!({:?})\n    }}\n",
            fitness_todo
        ));

        // Convergence state helper (only when thresholds are declared).
        if let Some(telos) = &being.telos {
            if telos.thresholds.is_some() {
                let name_upper = being.name.to_uppercase();
                out.push_str(&format!(
                    "\n    /// Classify the current convergence state against telos thresholds.\n\
    pub fn convergence_state(&self) -> {}ConvergenceState {{\n\
        let f = self.fitness();\n\
        if f >= {name_upper}_CONVERGENCE_THRESHOLD {{\n\
            {}ConvergenceState::Converging\n\
        }} else if f >= {name_upper}_WARNING_THRESHOLD {{\n\
            {}ConvergenceState::Warning\n\
        }} else {{\n\
            {}ConvergenceState::Diverging\n\
        }}\n\
    }}\n",
                    being.name, being.name, being.name, being.name,
                ));
            }
        }

        for reg in &being.regulate_blocks {
            let var_snake = to_snake_case(&reg.variable);
            let (low, high) = reg
                .bounds
                .as_ref()
                .map(|(l, h)| (l.as_str(), h.as_str()))
                .unwrap_or(("?", "?"));
            out.push_str(&format!(
                "\n    /// Homeostatic regulation: {} → target {} within [{}, {}]\n",
                reg.variable, reg.target, low, high
            ));
            out.push_str(&format!(
                "    pub fn regulate_{}(&mut self) {{\n",
                var_snake
            ));
            out.push_str(&format!(
                "        // target: {}, bounds: ({}, {})\n",
                reg.target, low, high
            ));
            if !reg.response.is_empty() {
                let resp: Vec<String> = reg
                    .response
                    .iter()
                    .map(|(c, a)| format!("{} -> {}", c, a))
                    .collect();
                out.push_str(&format!("        // response: {}\n", resp.join(", ")));
            }
            out.push_str(&format!(
                "        todo!({:?})\n    }}\n",
                format!("implement homeostatic regulation for {}", reg.variable)
            ));
        }

        if let Some(evolve) = &being.evolve_block {
            for sc in &evolve.search_cases {
                let method = strategy_rust_method(&sc.strategy);
                let strategy_name = strategy_rust_label(&sc.strategy);
                let step_comment = strategy_rust_step_comment(&sc.strategy);
                out.push_str(&format!("\n    /// Search strategy: {}\n", strategy_name));
                if !sc.when.trim().is_empty() {
                    out.push_str(&format!("    /// Condition: when {}\n", sc.when));
                }
                out.push_str("    /// Part of directed evolution toward telos. E[distance_to_telos] non-increasing.\n");
                out.push_str(&format!("    pub fn {}(&mut self) -> f64 {{\n", method));
                out.push_str(&format!("        // {}\n", step_comment));
                out.push_str(&format!("        // constraint: {}\n", evolve.constraint));
                out.push_str(&format!(
                    "        todo!({:?})\n    }}\n",
                    format!("implement {} step toward telos", strategy_name)
                ));
            }

            let strategy_list: Vec<&str> = evolve
                .search_cases
                .iter()
                .map(|sc| strategy_rust_label(&sc.strategy))
                .collect();
            let default_method = evolve
                .search_cases
                .first()
                .map(|sc| strategy_rust_method(&sc.strategy))
                .unwrap_or("evolve_step_impl");
            out.push_str("\n    /// Select and apply the appropriate search strategy based on current landscape.\n");
            out.push_str(
                "    /// Directed evolution: E[distance_to_telos] must be non-increasing.\n",
            );
            out.push_str("    pub fn evolve_step(&mut self) -> f64 {\n");
            out.push_str("        // dispatcher: select strategy based on landscape topology\n");
            if !strategy_list.is_empty() {
                out.push_str(&format!(
                    "        // strategies available: {}\n",
                    strategy_list.join(", ")
                ));
            }
            out.push_str(&format!(
                "        self.{}()  // default to first strategy\n    }}\n",
                default_method
            ));
        }

        for epi in &being.epigenetic_blocks {
            let signal_snake = to_snake_case(&epi.signal);
            let reverts_str = epi.reverts_when.as_deref().unwrap_or("never");
            out.push_str(&format!(
                "\n    /// Epigenetic modulation: {} → modifies {}\n    /// Waddington landscape: behavioral change without structural change.\n    /// Reverts when: {}\n",
                epi.signal, epi.modifies, reverts_str
            ));
            out.push_str(&format!(
                "    pub fn apply_epigenetic_{}(&mut self, signal_strength: f64) {{\n",
                signal_snake
            ));
            out.push_str(&format!("        // modifies: {}\n", epi.modifies));
            out.push_str(&format!("        // reverts_when: {}\n", reverts_str));
            out.push_str(&format!(
                "        todo!({:?})\n    }}\n",
                format!("implement epigenetic modulation of {}", epi.modifies)
            ));
        }

        for morph in &being.morphogen_blocks {
            let signal_snake = to_snake_case(&morph.signal);
            let produces_str = morph.produces.join(", ");
            let threshold_val: f64 = morph.threshold.parse().unwrap_or(0.5);
            out.push_str(&format!(
                "\n    /// Morphogenetic differentiation: {} above {} → produces {}\n    /// Turing (1952): local activation + lateral inhibition.\n",
                morph.signal, morph.threshold, produces_str
            ));
            out.push_str(&format!("    pub fn differentiate_{}(&self, signal_level: f64) -> Option<Vec<Box<dyn std::any::Any>>> {{\n", signal_snake));
            out.push_str(&format!(
                "        if signal_level >= {}_f64 {{\n",
                threshold_val
            ));
            out.push_str(&format!("            // produces: {}\n", produces_str));
            out.push_str(&format!(
                "            todo!({:?})\n        }} else {{\n            None\n        }}\n    }}\n",
                format!("implement differentiation: produce {}", produces_str)
            ));
        }

        if let Some(tel) = &being.telomere {
            out.push_str(&format!(
                "\n    /// Telomere countdown: {} replications maximum.\n    /// on_exhaustion: {}\n    /// Hayflick (1961): finite replication limit as a design invariant.\n",
                tel.limit, tel.on_exhaustion
            ));
            out.push_str(&format!(
                "    pub fn replicate(&mut self) -> Result<(), &'static str> {{\n        if self.telomere_count >= {} {{\n            // on_exhaustion: {}\n            return Err(\"telomere exhausted: {}\");\n        }}\n        self.telomere_count += 1;\n        Ok(())\n    }}\n",
                tel.limit, tel.on_exhaustion, tel.on_exhaustion
            ));
        }

        for crispr in &being.crispr_blocks {
            let guide_snake = to_snake_case(&crispr.guide);
            out.push_str(&format!(
                "\n    /// CRISPR-directed modification: {} targets {} → {}\n",
                crispr.guide, crispr.target, crispr.replace
            ));
            out.push_str("    /// Doudna/Charpentier (2012): targeted form editing under guided correction.\n");
            out.push_str(&format!(
                "    pub fn edit_{}(&mut self, guide: {}) -> Result<(), &'static str> {{\n",
                guide_snake, crispr.guide
            ));
            out.push_str(&format!("        // target: {}\n", crispr.target));
            out.push_str(&format!("        // replace: {}\n", crispr.replace));
            out.push_str(&format!(
                "        todo!(\"implement CRISPR edit: {} replaces {} with {}\")\n    }}\n",
                crispr.guide, crispr.target, crispr.replace
            ));
        }

        for plasticity in &being.plasticity_blocks {
            let modifies_snake = to_snake_case(&plasticity.modifies);
            let rule_name = match plasticity.rule {
                PlasticityRule::Hebbian => "Hebbian",
                PlasticityRule::Boltzmann => "Boltzmann",
                PlasticityRule::ReinforcementLearning => "ReinforcementLearning",
            };
            let rule_description = match plasticity.rule {
                PlasticityRule::Hebbian => "co-activation strengthens the connection weight",
                PlasticityRule::Boltzmann => "energy minimization via thermal equilibration",
                PlasticityRule::ReinforcementLearning => {
                    "reward signal updates weight toward policy optimum"
                }
            };
            out.push_str(&format!(
                "\n    /// Plasticity: {} → updates {} via {}\n",
                plasticity.trigger, plasticity.modifies, rule_name
            ));
            out.push_str("    /// Hebb (1949): neurons that fire together wire together.\n");
            out.push_str(&format!(
                "    pub fn update_{}(&mut self, trigger_strength: f64) {{\n",
                modifies_snake
            ));
            out.push_str(&format!(
                "        // rule: {} — {}\n",
                rule_name, rule_description
            ));
            out.push_str(&format!("        // modifies: {}\n", plasticity.modifies));
            out.push_str(&format!(
                "        todo!(\"implement {} plasticity for {}\")\n    }}\n",
                rule_name, plasticity.modifies
            ));
        }

        out.push_str("}\n");

        if let Some(can) = &being.canalization {
            let name = pascal_case(&being.name);
            let toward = &can.toward;
            out.push_str(&format!(
                "// LOOM[canalize:{name}]: Waddington (1942) — developmental channel toward {toward}\n"
            ));
            out.push_str(&format!(
                "pub struct {name}Canalization;\nimpl {name}Canalization {{\n    pub const TOWARD: &'static str = \"{toward}\";\n"
            ));
            if !can.despite.is_empty() {
                let despite_list: String = can
                    .despite
                    .iter()
                    .map(|d| format!("\"{}\"", d))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "    pub const DESPITE: &'static [&'static str] = &[{despite_list}];\n"
                ));
                out.push_str(
                    "    pub fn is_canalized(perturbation: &str) -> bool {\n        Self::DESPITE.contains(&perturbation)\n    }\n",
                );
            }
            if let Some(cp) = &can.convergence_proof {
                out.push_str(&format!("    // convergence_proof: {cp}\n"));
            }
            out.push_str("}\n");
        }
        if let Some(sen) = &being.senescence {
            out.push_str(&format!(
                "// senescence: onset: {}, degradation: {}\n",
                sen.onset, sen.degradation
            ));
            if let Some(s) = &sen.sasp {
                out.push_str(&format!("//   sasp: {}\n", s));
            }
        }
        if let Some(crit) = &being.criticality {
            out.push_str(&format!(
                "// LOOM[criticality:{}]: tipping point bounds [lower={}, upper={}]\n",
                being.name, crit.lower, crit.upper
            ));
            if let Some(p) = &crit.probe_fn {
                out.push_str(&format!("// LOOM[criticality:probe]: {}\n", p));
            }
            out.push_str(&format!(
                "pub const {}_CRITICALITY_LOWER: f64 = {};\n",
                being.name.to_uppercase(), crit.lower
            ));
            out.push_str(&format!(
                "pub const {}_CRITICALITY_UPPER: f64 = {};\n",
                being.name.to_uppercase(), crit.upper
            ));
            if let Some(p) = &crit.probe_fn {
                out.push_str(&format!(
                    "pub fn {}_criticality_probe() -> f64 {{ {}() }}\n",
                    being.name.to_lowercase(), p
                ));
            }
        }
        if being.autopoietic {
            out.push_str(&format!("\nimpl {} {{\n", being.name));
            out.push_str("    /// Autopoietic system: operationally closed, self-producing, boundary-maintaining.\n");
            out.push_str("    /// Maturana/Varela (1972): the living system that produces and maintains itself.\n");
            out.push_str(
                "    /// Organizational properties: telos (purpose) + regulate (homeostasis) +\n",
            );
            out.push_str("    /// evolve (self-modification) + matter (boundary substrate).\n");
            out.push_str("    pub fn is_autopoietic() -> bool { true }\n\n");
            out.push_str(
                "    /// Verify operational closure: all autopoietic components are functional.\n",
            );
            out.push_str("    pub fn verify_closure(&self) -> bool {\n");
            out.push_str("        // operational closure requires all four layers to be non-trivially implemented\n");
            out.push_str("        false // todo: implement verification\n");
            out.push_str("    }\n");
            out.push_str("}\n");
        }

        for scenario in &being.scenarios {
            let fn_name = format!("scenario_{}", to_snake_case(&scenario.name));
            out.push_str("\n#[test]\n");
            out.push_str(&format!("#[doc = \"Scenario: {}\"]\n", scenario.name));
            out.push_str(&format!("fn {}() {{\n", fn_name));
            out.push_str(&format!("    // given: {}\n", scenario.given));
            out.push_str(&format!("    // when: {}\n", scenario.when));
            out.push_str(&format!("    // then: {}\n", scenario.then));
            if let Some((count, unit)) = &scenario.within {
                out.push_str(&format!("    // within: {} {}\n", count, unit));
            }
            out.push_str(&format!(
                "    todo!({:?})\n}}\n",
                format!("scenario: {} — implement test body", scenario.name)
            ));
        }

        out
    }
}

// ── Strategy helpers (only used by emit_being) ────────────────────────────────

fn strategy_rust_method(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent => "evolve_gradient_descent",
        SearchStrategy::StochasticGradient => "evolve_stochastic_gradient",
        SearchStrategy::SimulatedAnnealing => "evolve_simulated_annealing",
        SearchStrategy::DerivativeFree => "evolve_derivative_free",
        SearchStrategy::Mcmc => "evolve_mcmc",
    }
}

fn strategy_rust_label(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent => "gradient_descent",
        SearchStrategy::StochasticGradient => "stochastic_gradient",
        SearchStrategy::SimulatedAnnealing => "simulated_annealing",
        SearchStrategy::DerivativeFree => "derivative_free",
        SearchStrategy::Mcmc => "mcmc",
    }
}

fn strategy_rust_step_comment(strategy: &SearchStrategy) -> &'static str {
    match strategy {
        SearchStrategy::GradientDescent => {
            "gradient descent step: adjust parameters along negative gradient"
        }
        SearchStrategy::StochasticGradient => "stochastic gradient step: noisy gradient estimation",
        SearchStrategy::SimulatedAnnealing => {
            "simulated annealing step: probabilistic uphill acceptance"
        }
        SearchStrategy::DerivativeFree => {
            "derivative-free step: explore without gradient information"
        }
        SearchStrategy::Mcmc => "MCMC step: sample from posterior landscape",
    }
}
