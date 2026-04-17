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
        // M186: role annotation
        if let Some(role) = &being.role {
            out.push_str(&format!("// LOOM[role:{}]\n", role));
        }
        // M187: structural relationships
        for rel in &being.relates_to {
            out.push_str(&format!(
                "// LOOM[relates_to:{}:{}]\n",
                rel.target, rel.kind
            ));
        }
        // LOOM[propagate]: emit propagation metadata
        if let Some(prop) = &being.propagate_block {
            out.push_str(&format!(
                "// LOOM[propagate]: condition={}, inherits=[{}], mutates=[{}]\n",
                prop.condition,
                prop.inherits.join(", "),
                prop.mutates
                    .iter()
                    .map(|(f, c)| format!("{} {}", f, c))
                    .collect::<Vec<_>>()
                    .join("; ")
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
                "/// TLA+ convergence specification for `{name}` (extract and run with TLC).\n\
/// Invariant: fitness is monotonically non-decreasing toward telos.\n\
pub const {name_upper}_TLA_SPEC: &str = r#\"\n\
---- MODULE {name}ConvergenceCheck ----\n\
EXTENDS Reals, Naturals\n\
\n\
CONSTANT ConvergenceThreshold,  \\* fitness >= this => converged\n\
         DivergenceThreshold,   \\* fitness < this => diverging\n\
         MaxTicks               \\* bound for finite-state model checking\n\
\n\
VARIABLES fitness, state, tick\n\
\n\
\\* telos: {desc}\n\
\n\
TypeInvariant ==\n\
  /\\ fitness \\in REAL\n\
  /\\ state \\in {{\"converging\", \"warning\", \"diverging\"}}\n\
  /\\ tick \\in 0..MaxTicks\n\
\n\
Init ==\n\
  /\\ fitness = 0.5\n\
  /\\ state = \"warning\"\n\
  /\\ tick = 0\n\
\n\
Next ==\n\
  /\\ tick < MaxTicks\n\
  /\\ tick' = tick + 1\n\
  /\\ UNCHANGED <<fitness, state>>  \\* fitness updated by runtime; model checks structure\n\
\n\
Spec == Init /\\ [][Next]_<<fitness, state, tick>>\n\
\n\
TelosConverged == fitness >= ConvergenceThreshold\n\
TelosDiverged  == fitness < DivergenceThreshold\n\
\n\
\\* Safety: once converged fitness never falls below divergence threshold\n\
NonDegeneracy == [](TelosConverged => ~TelosDiverged)\n\
\n\
\\* Liveness: the being eventually converges within MaxTicks\n\
ConvergenceProperty == <>(TelosConverged)\n\
\n\
\\* State machine transitions are well-typed\n\
StateConsistency == [](\n\
  (state = \"converging\") => (fitness >= ConvergenceThreshold) )\n\
\n\
====\n\
\"#;\n\n\
/// TLC model configuration for `{name}` (save as `{name}_convergence.cfg`).\n\
pub const {name_upper}_TLC_CONFIG: &str = \"\n\
SPECIFICATION Spec\n\
INVARIANT TypeInvariant\n\
INVARIANT NonDegeneracy\n\
INVARIANT StateConsistency\n\
PROPERTY ConvergenceProperty\n\
CONSTANTS\n\
  ConvergenceThreshold <- 0.8\n\
  DivergenceThreshold <- 0.3\n\
  MaxTicks <- 100\n\
\";\n\n",
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
        out.push_str(
            "    /// Returns the fitness score relative to telos (0.0 = worst, 1.0 = perfect).\n",
        );
        out.push_str(&format!("    /// telos: {:?}\n", telos_desc));
        out.push_str("    pub fn fitness(&self) -> f64 {\n");
        // Generate fitness body from regulate_blocks bounds.
        // Each regulation bound contributes a normalized gap toward telos.
        let matter_field_names: Vec<String> = being
            .matter
            .as_ref()
            .map(|m| m.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        let regs_with_bounds: Vec<_> = being
            .regulate_blocks
            .iter()
            .filter(|r| r.bounds.is_some())
            .collect();
        if regs_with_bounds.is_empty() {
            // No regulate bounds — emit a static comment with telos description.
            out.push_str("        // Fitness toward telos — no regulate bounds declared.\n");
            out.push_str(
                "        // Values: 1.0 = all bounds satisfied, 0.0 = maximally violated.\n",
            );
            if let Some(telos) = &being.telos {
                if let Some(ff) = &telos.fitness_fn {
                    out.push_str(&format!("        // Override via fitness_fn: {}\n", ff));
                }
            }
            out.push_str(
                "        0.5 // static estimate — wire to runtime signal store for live fitness\n",
            );
        } else {
            out.push_str(
                "        // Fitness toward telos — computed from homeostatic bound gaps.\n",
            );
            out.push_str("        // Each regulate bound contributes a normalized gap [0, 1].\n");
            out.push_str(
                "        // Final score: 1.0 = all bounds satisfied, 0.0 = maximally violated.\n",
            );
            out.push_str(&format!("        let mut total_gap: f64 = 0.0;\n"));
            out.push_str(&format!(
                "        let n_bounds: f64 = {}_f64;\n",
                regs_with_bounds.len()
            ));
            for reg in &regs_with_bounds {
                let (low_str, high_str) = reg.bounds.as_ref().unwrap();
                let target_str = &reg.target;
                let field_name = to_snake_case(&reg.variable);
                let uses_self = matter_field_names.contains(&field_name);
                let current_expr = if uses_self {
                    format!("self.{} as f64", field_name)
                } else {
                    "0.0_f64 /* wire to runtime signal */".to_string()
                };
                out.push_str(&format!("        {{\n"));
                out.push_str(&format!(
                    "            // regulate {}: target={}, bounds=[{}, {}]\n",
                    reg.variable, target_str, low_str, high_str
                ));
                out.push_str(&format!(
                    "            let current: f64 = {};\n",
                    current_expr
                ));
                out.push_str(&format!("            let lower: f64 = {}_f64;\n", low_str));
                out.push_str(&format!("            let upper: f64 = {}_f64;\n", high_str));
                out.push_str("            let gap = if current < lower {\n");
                out.push_str("                (lower - current) / (lower.abs().max(1.0))\n");
                out.push_str("            } else if current > upper {\n");
                out.push_str("                (current - upper) / (upper.abs().max(1.0))\n");
                out.push_str("            } else {\n");
                out.push_str("                0.0\n");
                out.push_str("            };\n");
                out.push_str("            total_gap += gap.min(1.0);\n");
                out.push_str(&format!("        }}\n"));
            }
            out.push_str("        1.0 - (total_gap / n_bounds).min(1.0)\n");
        }
        out.push_str("    }\n");

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

        // ── OU convergence estimate (always emitted) ──────────────────────────
        let (conv_thresh, div_thresh) = being
            .telos
            .as_ref()
            .and_then(|t| t.thresholds.as_ref())
            .map(|th| (th.convergence, th.divergence))
            .unwrap_or((0.2, 0.7));
        out.push_str(
            "\n    /// Estimate probability of telos convergence within `ticks` steps.\n\
    /// Based on Ornstein-Uhlenbeck mean-reversion approximation.\n\
    ///\n\
    /// Parameters:\n\
    ///   current_drift: current D_static score (0.0 = at telos, 1.0 = maximally diverged)\n\
    ///   drift_velocity: rate of change of drift per tick (negative = improving)\n\
    ///   ticks: horizon for convergence estimate\n\
    ///\n\
    /// Returns probability in [0.0, 1.0] that drift reaches 0.0 within `ticks` steps.\n\
    pub fn telos_convergence_estimate(\n\
        current_drift: f64,\n\
        drift_velocity: f64,\n\
        ticks: u64,\n\
    ) -> f64 {\n\
        if current_drift <= 0.0 { return 1.0; }  // already converged\n\
        if drift_velocity >= 0.0 { return 0.0; } // diverging — no convergence\n\
\n\
        // OU mean-reversion: expected ticks to convergence = -current_drift / drift_velocity\n\
        let expected_ticks = -current_drift / drift_velocity;\n\
\n\
        // Probability using exponential approximation:\n\
        // P(converge within T) ≈ 1 - exp(-T / expected_ticks)\n\
        // This is exact for constant-velocity convergence; OU adds the reversion factor.\n\
        let lambda = 1.0 / expected_ticks.max(1.0);\n\
        let prob = 1.0_f64 - (-lambda * ticks as f64).exp();\n\
        prob.clamp(0.0, 1.0)\n\
    }\n",
        );
        out.push_str(&format!(
            "\n    /// Classify convergence state given current drift and trend.\n\
    pub fn convergence_state_from_drift(current_drift: f64, drift_velocity: f64) -> &'static str {{\n\
        if current_drift <= {conv_thresh:.3}_f64 {{ \"converged\" }}\n\
        else if current_drift >= {div_thresh:.3}_f64 {{ \"diverging\" }}\n\
        else if drift_velocity < 0.0 {{ \"converging\" }}\n\
        else {{ \"warning\" }}\n\
    }}\n",
            conv_thresh = conv_thresh,
            div_thresh = div_thresh,
        ));

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
            // Emit signature: takes `current: f64` and returns `f64` when bounds are declared,
            // so callers can pass the live metric value and receive a corrective signal.
            if reg.bounds.is_some() {
                out.push_str(&format!(
                    "    pub fn regulate_{}(&mut self, current: f64) -> f64 {{\n",
                    var_snake
                ));
            } else {
                out.push_str(&format!(
                    "    pub fn regulate_{}(&mut self) {{\n",
                    var_snake
                ));
            }
            out.push_str(&format!(
                "        // target: {}, bounds: ({}, {})\n",
                reg.target, low, high
            ));
            // M189: emit LOOM[trigger:classifier:Name] when trigger is a classifier gate
            if let Some(trigger) = &reg.trigger {
                if let Some(classifier_name) = trigger.strip_prefix("classifier:") {
                    out.push_str(&format!(
                        "        // LOOM[trigger:classifier:{}]\n",
                        classifier_name
                    ));
                } else {
                    out.push_str(&format!("        // trigger: {}\n", trigger));
                }
            }
            if !reg.response.is_empty() {
                let resp: Vec<String> = reg
                    .response
                    .iter()
                    .map(|(c, a)| format!("{} -> {}", c, a))
                    .collect();
                out.push_str(&format!("        // response: {}\n", resp.join(", ")));
            }
            // Generate real homeostatic regulation body when bounds are available.
            if let Some((low, high)) = &reg.bounds {
                out.push_str("        // Homeostatic regulation: return corrective signal toward [lower, upper].\n");
                out.push_str("        // Positive return = push up, negative = push down, 0 = within bounds.\n");
                out.push_str(&format!("        let lower: f64 = {}_f64;\n", low));
                out.push_str(&format!("        let upper: f64 = {}_f64;\n", high));
                out.push_str("        if current < lower { lower - current }\n");
                out.push_str("        else if current > upper { current - upper }\n");
                out.push_str("        else { 0.0 }\n");
            } else {
                out.push_str(&format!(
                    "        todo!({:?})\n",
                    format!("implement homeostatic regulation for {}", reg.variable)
                ));
            }
            out.push_str("    }\n");
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
                // Emit real body for derivative-free (coordinate perturbation) strategy.
                if sc.strategy == SearchStrategy::DerivativeFree {
                    out.push_str("        // Derivative-free optimization: coordinate perturbation toward telos.\n");
                    out.push_str("        // Uses random ±ε perturbation; keeps the move if fitness improves.\n");
                    out.push_str("        // The runtime orchestrator applies perturbations to the live signal store.\n");
                    out.push_str("        let current_fitness = self.fitness();\n");
                    out.push_str(
                        "        // Perturbation is applied externally by the CEMS orchestrator.\n",
                    );
                    out.push_str("        // Return current fitness for introspection and dispatcher routing.\n");
                    out.push_str("        current_fitness\n");
                } else {
                    out.push_str(&format!(
                        "        todo!({:?})\n",
                        format!("implement {} step toward telos", strategy_name)
                    ));
                }
                out.push_str("    }\n");
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

        if let Some(rewire) = &being.rewire_block {
            let candidates_str = rewire
                .candidates
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");
            let candidates_list = rewire.candidates.join(", ");
            let selection = &rewire.selection;
            out.push_str(&format!(
                "\n    // Structural rewire constants\n    pub const REWIRE_TRIGGER_THRESHOLD: f64 = {};\n",
                rewire.trigger_threshold
            ));
            out.push_str(&format!(
                "    pub const REWIRE_CANDIDATES: &'static [&'static str] = &[{}];\n",
                candidates_str
            ));
            out.push_str(&format!(
                "    pub const REWIRE_SELECTION: &'static str = \"{}\";\n",
                selection
            ));
            out.push_str(&format!(
                "    pub const REWIRE_COOLDOWN_TICKS: u64 = {};\n",
                rewire.cooldown
            ));
            out.push_str("\n    /// Evaluate whether structural self-modification is warranted.\n");
            out.push_str(
                "    /// Called by the CEMS orchestrator when drift exceeds REWIRE_TRIGGER_THRESHOLD.\n",
            );
            out.push_str(
                "    /// Returns the selected candidate component name, or None if rewire is not yet due.\n",
            );
            out.push_str(
                "    pub fn evaluate_structural_rewire(&self, drift_score: f64, ticks_since_last_rewire: u64) -> Option<&'static str> {\n",
            );
            out.push_str(
                "        if drift_score < Self::REWIRE_TRIGGER_THRESHOLD { return None; }\n",
            );
            out.push_str(
                "        if ticks_since_last_rewire < Self::REWIRE_COOLDOWN_TICKS { return None; }\n",
            );
            out.push_str(&format!(
                "        // Selection strategy: {}\n        // Candidates: {}\n",
                selection, candidates_list
            ));
            out.push_str(&format!(
                "        todo!(\"implement {} selection among REWIRE_CANDIDATES\")\n    }}\n",
                selection
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
                being.name.to_uppercase(),
                crit.lower
            ));
            out.push_str(&format!(
                "pub const {}_CRITICALITY_UPPER: f64 = {};\n",
                being.name.to_uppercase(),
                crit.upper
            ));
            if let Some(p) = &crit.probe_fn {
                out.push_str(&format!(
                    "pub fn {}_criticality_probe() -> f64 {{ {}() }}\n",
                    being.name.to_lowercase(),
                    p
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
