//! Runtime emitter — generates the BIOISO bootstrap `main.rs` from a Loom module.
//!
//! The emitted code, when compiled with `loom` as a dependency, creates a
//! `Runtime`, spawns all `being:` entities, registers `telos:` bounds, registers
//! `signal` channels from `ecosystem:` blocks, and starts the supervision loop.

use crate::ast::{BeingDef, EcosystemDef, Module};

/// Emits a BIOISO runtime bootstrap (`main.rs`) from a Loom module.
pub struct RuntimeEmitter;

impl RuntimeEmitter {
    /// Create a new emitter.
    pub fn new() -> Self {
        Self
    }

    /// Emit the complete bootstrap source for `module`.
    pub fn emit(&self, module: &Module) -> String {
        let mut out = String::new();
        out.push_str(HEADER);
        out.push_str("\nfn main() {\n");
        out.push_str(
            "    let mut rt = loom::runtime::Runtime::new(\"./signals.db\")\
             .expect(\"failed to open signal store\");\n\n",
        );

        for being in &module.being_defs {
            out.push_str(&self.emit_spawn(being));
        }

        for eco in &module.ecosystem_defs {
            out.push_str(&self.emit_ecosystem_signals(eco));
        }

        out.push_str(
            "\n    // ── Startup summary ────────────────────────────────────────────\n\
             let entities = rt.entities().expect(\"store query failed\");\n\
             println!(\"[BIOISO] {} entities registered\", entities.len());\n\
             for e in &entities {\n\
             println!(\"  · {} ({}) state: {}\", e.name, e.id, e.state);\n\
             }\n",
        );
        out.push_str(
            "\n    // ── Evolution loop placeholder (Phase R7) ──────────────────────\n\
             // TODO: replace with orchestrator::start(&mut rt);\n\
             println!(\"[BIOISO] Evolution loop not yet wired (Phase R7).\");\n",
        );
        out.push_str("}\n");
        out
    }

    /// Emit a `spawn_entity` call for one `being:`.
    fn emit_spawn(&self, being: &BeingDef) -> String {
        let mut out = String::new();
        let telos_json = self.telos_to_json(being);
        let limit = self.telomere_limit(being);
        let exhaustion = self.on_exhaustion(being);
        out.push_str(&format!(
            "    // Being: {name}\n\
             rt.spawn_entity(\"{name}\", \"{name}\", r#\"{telos}\"#, {limit}, {exhaustion})\n\
             .expect(\"failed to spawn '{name}'\");\n",
            name = being.name,
            telos = telos_json,
            limit = limit,
            exhaustion = exhaustion,
        ));
        if let Some(telos) = &being.telos {
            if let Some(th) = &telos.thresholds {
                out.push_str(&format!(
                    "    rt.set_telos_bounds(\"{name}\", \"telos_score\",\
                     Some({div:.4}), None, Some({conv:.4}))\n\
                     .expect(\"failed to set telos bounds for '{name}'\");\n",
                    name = being.name,
                    div = th.divergence,
                    conv = th.convergence,
                ));
            }
        }
        out.push('\n');
        out
    }

    /// Emit signal channel comments from an ecosystem.
    fn emit_ecosystem_signals(&self, eco: &EcosystemDef) -> String {
        if eco.signals.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        out.push_str(&format!("    // Ecosystem: {} signals\n", eco.name));
        for sig in &eco.signals {
            out.push_str(&format!(
                "    // signal {} :: {} -> {} ({})\n",
                sig.name, sig.from, sig.to, sig.payload
            ));
        }
        out.push('\n');
        out
    }

    fn telos_to_json(&self, being: &BeingDef) -> String {
        match &being.telos {
            None => "{}".into(),
            Some(t) => {
                let mut fields = vec![
                    format!("\"description\":\"{}\"", escape_json(&t.description)),
                ];
                if let Some(m) = &t.metric {
                    fields.push(format!("\"metric\":\"{}\"", escape_json(m)));
                }
                if let Some(th) = &t.thresholds {
                    fields.push(format!(
                        "\"convergence\":{},\"divergence\":{}",
                        th.convergence, th.divergence
                    ));
                }
                if let Some(mb) = &t.modifiable_by {
                    fields.push(format!("\"modifiable_by\":\"{}\"", escape_json(mb)));
                }
                format!("{{{}}}", fields.join(","))
            }
        }
    }

    fn telomere_limit(&self, being: &BeingDef) -> String {
        match &being.telomere {
            Some(t) => format!("Some({})", t.limit),
            None => "None".into(),
        }
    }

    fn on_exhaustion(&self, being: &BeingDef) -> String {
        match &being.telomere {
            Some(t) => format!("Some(\"{}\".into())", escape_json(&t.on_exhaustion)),
            None => "None".into(),
        }
    }
}

impl Default for RuntimeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

const HEADER: &str =
    "// ── BIOISO Runtime Bootstrap ── generated by loom compile_runtime() ────────\n\
     // Add to Cargo.toml: loom = \"0.2.0\"\n\
     // Run: cargo run\n\
     // ──────────────────────────────────────────────────────────────────────────\n";

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Module, Span, TelosDef, TelosThresholds, TelomereBlock};

    fn make_being(name: &str) -> BeingDef {
        BeingDef {
            name: name.into(),
            describe: None,
            annotations: vec![],
            matter: None,
            form: None,
            function: None,
            telos: Some(TelosDef {
                description: "maintain temperature below 2C".into(),
                fitness_fn: None,
                modifiable_by: Some("human_operator".into()),
                bounded_by: None,
                sign: None,
                metric: Some("temperature_delta".into()),
                thresholds: Some(TelosThresholds {
                    convergence: 0.8,
                    warning: Some(0.5),
                    divergence: 0.2,
                    propagation: None,
                }),
                guides: vec![],
                span: Span::synthetic(),
            }),
            regulate_blocks: vec![],
            evolve_block: None,
            epigenetic_blocks: vec![],
            morphogen_blocks: vec![],
            telomere: Some(TelomereBlock {
                limit: 50,
                on_exhaustion: "graceful_shutdown".into(),
                span: Span::synthetic(),
            }),
            autopoietic: false,
            crispr_blocks: vec![],
            plasticity_blocks: vec![],
            canalization: None,
            senescence: None,
            criticality: None,
            umwelt: None,
            resonance: None,
            manifest: None,
            migrations: vec![],
            journal: None,
            scenarios: vec![],
            boundary: None,
            cognitive_memory: None,
            signal_attention: None,
            role: None,
            relates_to: vec![],
            propagate_block: None,
            span: Span::synthetic(),
        }
    }

    fn make_module(being: BeingDef) -> Module {
        Module {
            name: "test".into(),
            describe: None,
            domains: vec![],
            annotations: vec![],
            imports: vec![],
            spec: None,
            interface_defs: vec![],
            implements: vec![],
            provides: None,
            requires: None,
            invariants: vec![],
            test_defs: vec![],
            lifecycle_defs: vec![],
            temporal_defs: vec![],
            being_defs: vec![being],
            ecosystem_defs: vec![],
            flow_labels: vec![],
            aspect_defs: vec![],
            items: vec![],
            span: Span::synthetic(),
        }
    }

    #[test]
    fn emit_contains_spawn_for_being() {
        let being = make_being("ClimateModel");
        let code = RuntimeEmitter::new().emit_spawn(&being);
        assert!(code.contains("spawn_entity"));
        assert!(code.contains("ClimateModel"));
        assert!(code.contains("Some(50)"));
        assert!(code.contains("graceful_shutdown"));
    }

    #[test]
    fn emit_contains_telos_bounds_when_thresholds_declared() {
        let being = make_being("ClimateModel");
        let code = RuntimeEmitter::new().emit_spawn(&being);
        assert!(code.contains("set_telos_bounds"));
        assert!(code.contains("0.2000"));
        assert!(code.contains("0.8000"));
    }

    #[test]
    fn telos_to_json_includes_description_and_metric() {
        let being = make_being("Foo");
        let json = RuntimeEmitter::new().telos_to_json(&being);
        assert!(json.contains("maintain temperature below 2C"));
        assert!(json.contains("temperature_delta"));
        assert!(json.contains("human_operator"));
    }

    #[test]
    fn emit_full_contains_header_and_main() {
        let module = make_module(make_being("EpidemicModel"));
        let output = RuntimeEmitter::new().emit(&module);
        assert!(output.contains("BIOISO Runtime Bootstrap"));
        assert!(output.contains("fn main()"));
        assert!(output.contains("spawn_entity"));
        assert!(output.contains("Runtime::new"));
    }

    #[test]
    fn escape_json_handles_special_chars() {
        assert_eq!(escape_json(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape_json(r"path\file"), r"path\\file");
    }
}
