//! NeuroML 2 XML emitter.
//!
//! Converts Loom biological beings and ecosystems into NeuroML 2 XML format,
//! which is the standard for specifying computational neural network models.
//!
//! Mapping:
//! - `being:` with `plasticity:` → `<cell>`
//! - `regulate:` → `<biophysicalProperties>`
//! - `morphogen:` → `<morphology>`
//! - `ecosystem:` → `<network>` with `<population>` + `<projection>`
//! - `plasticity:` (Hebbian/Boltzmann) → `<synapse>`
//!
//! NeuroML 2: https://neuroml.org/neuromlv2

use crate::ast::{BeingDef, EcosystemDef, Module, PlasticityRule};

/// Emits a NeuroML 2 XML document from a Loom [`Module`].
///
/// Only beings that declare at least one `plasticity:` block are emitted
/// as `<cell>` elements — non-plastic beings are silently excluded.
pub struct NeuroMLEmitter;

impl NeuroMLEmitter {
    /// Emit a complete NeuroML 2 XML document from a [`Module`].
    ///
    /// Beings without a `plasticity:` block are excluded from the output.
    /// Ecosystems whose members all lack plasticity will still emit `<network>`
    /// (populations are filtered to plastic beings only).
    pub fn emit(module: &Module) -> String {
        let mut out = String::new();

        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<neuroml xmlns=\"http://www.neuroml.org/schema/neuroml2\"\n");
        out.push_str("         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
        out.push_str("         xsi:schemaLocation=\"http://www.neuroml.org/schema/neuroml2 https://raw.github.com/NeuroML/NeuroML2/development/Schemas/NeuroML2/NeuroML_v2beta4.xsd\"\n");
        out.push_str(&format!("         id=\"{}\">\n\n", module.name));

        // Emit plastic beings as <cell> elements
        let plastic_beings: Vec<&BeingDef> = module
            .being_defs
            .iter()
            .filter(|b| !b.plasticity_blocks.is_empty())
            .collect();

        for being in &plastic_beings {
            out.push_str(&emit_cell(being));
            out.push('\n');
        }

        // Emit ecosystems as <network>
        for eco in &module.ecosystem_defs {
            out.push_str(&emit_network(eco, &plastic_beings));
            out.push('\n');
        }

        out.push_str("</neuroml>\n");
        out
    }
}

fn emit_cell(being: &BeingDef) -> String {
    let mut out = String::new();
    let id = sanitize_id(&being.name);

    out.push_str(&format!("  <cell id=\"{}\">\n", id));

    // morphogen: → <morphology>
    if !being.morphogen_blocks.is_empty() {
        out.push_str(&format!("    <morphology id=\"{}_morphology\">\n", id));
        for morph in &being.morphogen_blocks {
            out.push_str(&format!(
                "      <segment id=\"{}\" threshold=\"{}\"",
                sanitize_id(&morph.signal),
                morph.threshold
            ));
            if !morph.produces.is_empty() {
                out.push_str(&format!(" produces=\"{}\"", morph.produces.join(",")));
            }
            out.push_str("/>\n");
        }
        out.push_str("    </morphology>\n");
    }

    // regulate: → <biophysicalProperties>
    if !being.regulate_blocks.is_empty() {
        out.push_str(&format!(
            "    <biophysicalProperties id=\"{}_biophysics\">\n",
            id
        ));
        out.push_str("      <membraneProperties>\n");
        for reg in &being.regulate_blocks {
            out.push_str(&format!(
                "        <property variable=\"{}\" target=\"{}\"",
                sanitize_id(&reg.variable),
                sanitize_id(&reg.target)
            ));
            if let Some((low, high)) = &reg.bounds {
                out.push_str(&format!(" min=\"{}\" max=\"{}\"", low, high));
            }
            out.push_str("/>\n");
        }
        out.push_str("      </membraneProperties>\n");
        out.push_str("    </biophysicalProperties>\n");
    }

    // plasticity: → <synapse>
    for plasticity in &being.plasticity_blocks {
        let rule_str = match plasticity.rule {
            PlasticityRule::Hebbian => "hebbian",
            PlasticityRule::Boltzmann => "boltzmann",
            PlasticityRule::ReinforcementLearning => "reinforcement_learning",
        };
        out.push_str(&format!(
            "    <synapse id=\"{}_{}_synapse\" trigger=\"{}\" modifies=\"{}\" rule=\"{}\"/>\n",
            id,
            rule_str,
            sanitize_id(&plasticity.trigger),
            sanitize_id(&plasticity.modifies),
            rule_str
        ));
    }

    out.push_str("  </cell>\n");
    out
}

fn emit_network(eco: &EcosystemDef, plastic_beings: &[&BeingDef]) -> String {
    let mut out = String::new();
    let id = sanitize_id(&eco.name);

    out.push_str(&format!("  <network id=\"{}\">\n", id));

    // <population> for each member that has plasticity
    let plastic_names: std::collections::HashSet<&str> =
        plastic_beings.iter().map(|b| b.name.as_str()).collect();

    for member in &eco.members {
        if plastic_names.contains(member.as_str()) {
            out.push_str(&format!(
                "    <population id=\"{}_population\" component=\"{}\"/>\n",
                sanitize_id(member),
                sanitize_id(member)
            ));
        }
    }

    // <projection> for each signal
    for signal in &eco.signals {
        out.push_str(&format!(
            "    <projection id=\"{}\" presynapticPopulation=\"{}_population\" postsynapticPopulation=\"{}_population\" synapse=\"{}\">\n",
            sanitize_id(&signal.name),
            sanitize_id(&signal.from),
            sanitize_id(&signal.to),
            sanitize_id(&signal.payload)
        ));
        out.push_str("    </projection>\n");
    }

    out.push_str("  </network>\n");
    out
}

/// Convert a Loom identifier to a NeuroML-safe ID (no spaces, lowercase-friendly).
fn sanitize_id(s: &str) -> String {
    s.replace(' ', "_")
}
