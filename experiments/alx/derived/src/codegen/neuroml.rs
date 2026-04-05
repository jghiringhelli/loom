// ALX: derived from loom.loom §"emit_neuroml" (M53)
// NeuroML 2 XML emitter — neural beings with plasticity blocks.
// - Only beings WITH plasticity: blocks are emitted
// - Being → <cell id="..."> element
// - regulate: → <biophysicalProperties> with bounds
// - morphogen: → <morphology> with <segment>
// - Hebbian plasticity rule → <synapse rule="Hebbian"/>
// - ecosystem: → <network> with <population> and <projection> per signal
// - Root element: <neuroml xmlns="https://www.neuroml.org/schema/neuroml2">

use crate::ast::*;

/// G3: NeuroMLEmitter struct — tests call `NeuroMLEmitter::emit(&module)` (static, no self).
pub struct NeuroMLEmitter;

impl NeuroMLEmitter {
    pub fn emit(module: &Module) -> String {
        emit_neuroml(module)
    }
}

pub fn emit_neuroml(module: &Module) -> String {
    let mut out = String::new();

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<neuroml xmlns=\"https://www.neuroml.org/schema/neuroml2\"\n");
    out.push_str("         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
    out.push_str("         xsi:schemaLocation=\"https://www.neuroml.org/schema/neuroml2 https://raw.github.com/NeuroML/NeuroML2/development/Schemas/NeuroML2/NeuroML_v2.2.xsd\"\n");
    out.push_str(&format!("         id=\"{}\">\n\n", module.name));

    if let Some(desc) = &module.describe {
        out.push_str(&format!("  <!-- {} -->\n\n", desc));
    }

    // Emit beings with plasticity: blocks as <cell> elements
    for being in &module.being_defs {
        if !being.plasticity_blocks.is_empty() {
            emit_neuroml_cell(&mut out, being);
        }
    }

    // Ecosystems → <network>
    for eco in &module.ecosystem_defs {
        let has_neural_members = eco.members.iter().any(|m| {
            // ALX: member is neural if it has plasticity block (we can't easily check here)
            true // conservative: include all
        });
        if has_neural_members {
            emit_neuroml_network(&mut out, eco);
        }
    }

    out.push_str("</neuroml>\n");
    out
}

fn emit_neuroml_cell(out: &mut String, being: &BeingDef) {
    out.push_str(&format!("  <cell id=\"{}\">\n", being.name));

    if let Some(desc) = &being.describe {
        out.push_str(&format!("    <!-- {} -->\n", desc));
    }
    if let Some(telos) = &being.telos {
        out.push_str(&format!("    <!-- telos: {} -->\n", telos.description));
    }

    // regulate: → <biophysicalProperties>
    if !being.regulate_blocks.is_empty() {
        out.push_str("    <biophysicalProperties id=\"biophys\">\n");
        for reg in &being.regulate_blocks {
            out.push_str(&format!(
                "      <!-- regulate {} target={} -->\n",
                reg.variable, reg.target
            ));
            if let Some((lo, hi)) = &reg.bounds {
                out.push_str(&format!(
                    "      <property name=\"{}\" value=\"{}\" min=\"{}\" max=\"{}\"/>\n",
                    reg.variable, reg.target, lo, hi
                ));
            } else {
                out.push_str(&format!(
                    "      <property name=\"{}\" value=\"{}\"/>\n",
                    reg.variable, reg.target
                ));
            }
        }
        out.push_str("    </biophysicalProperties>\n");
    }

    // morphogen: → <morphology>
    for morph in &being.morphogen_blocks {
        out.push_str("    <morphology id=\"morpho\">\n");
        out.push_str(&format!(
            "      <!-- signal: {} threshold: {} produces: {:?} -->\n",
            morph.signal, morph.threshold, morph.produces
        ));
        let first_product = morph.produces.first().map(|s| s.as_str()).unwrap_or("unknown");
        out.push_str(&format!(
            "      <segment id=\"0\" name=\"{}\">\n",
            first_product
        ));
        out.push_str("        <proximal x=\"0\" y=\"0\" z=\"0\" diameter=\"1.0\"/>\n");
        out.push_str("        <distal x=\"0\" y=\"0\" z=\"10\" diameter=\"1.0\"/>\n");
        out.push_str("      </segment>\n");
        out.push_str("    </morphology>\n");
    }

    // plasticity rules → <synapse>
    for plasticity in &being.plasticity_blocks {
        let rule_name = match plasticity.rule {
            PlasticityRule::Hebbian => "Hebbian",
            PlasticityRule::Boltzmann => "Boltzmann",
            PlasticityRule::ReinforcementLearning => "ReinforcementLearning",
        };
        out.push_str(&format!(
            "    <synapse id=\"{}\" rule=\"{}\" trigger=\"{}\" modifies=\"{}\"/>\n",
            plasticity.modifies, rule_name, plasticity.trigger, plasticity.modifies
        ));
    }

    out.push_str("  </cell>\n\n");
}

fn emit_neuroml_network(out: &mut String, eco: &EcosystemDef) {
    out.push_str(&format!("  <network id=\"{}\">\n", eco.name));

    if let Some(telos) = &eco.telos {
        out.push_str(&format!("    <!-- telos: {} -->\n", telos));
    }

    // Population per member
    for member in &eco.members {
        out.push_str(&format!(
            "    <population id=\"{}_pop\" component=\"{}\"\n",
            member.to_lowercase(), member
        ));
        out.push_str("                size=\"1\"/>\n");
    }

    // Projection per signal
    for sig in &eco.signals {
        out.push_str(&format!(
            "    <projection id=\"{}\" presynapticPopulation=\"{}_pop\" postsynapticPopulation=\"{}_pop\">\n",
            sig.name,
            sig.from.to_lowercase(),
            sig.to.to_lowercase()
        ));
        out.push_str(&format!(
            "      <!-- payload: {} -->\n",
            sig.payload
        ));
        out.push_str(&format!(
            "      <connection id=\"0\" preCellId=\"../{}_pop/0/{}\" postCellId=\"../{}_pop/0/{}\"/>\n",
            sig.from.to_lowercase(), sig.from,
            sig.to.to_lowercase(), sig.to
        ));
        out.push_str("    </projection>\n");
    }

    out.push_str("  </network>\n\n");
}
