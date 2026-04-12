//! M190: OWL/JSON-LD ontology export for Loom modules.
//!
//! Traverses `module.domains`, `being.role`, and `being.relates_to` and emits
//! a JSON-LD document conforming to the OWL 2 Web Ontology Language vocabulary.
//!
//! ## Design
//! - Each `being:` → `owl:Class`
//! - Each `relates_to: Target kind: K` → `owl:ObjectProperty` with domain/range
//! - Each `role: R` on a being → `rdfs:comment` annotation (roles are not
//!   first-class OWL types; they annotate the class)
//! - Each `domain: D` on the module → `rdfs:isDefinedBy` subject IRI segment
//!
//! ## Output format
//! Produces a single JSON object with `@context`, `@graph`, and module metadata.
//! The base IRI is `https://loom.lang/{module_name}#`.
//!
//! ## Academic lineage
//! - OWL 2 Web Ontology Language (W3C 2012)
//! - JSON-LD 1.1 (W3C 2020)
//! - Formal concept analysis → class hierarchy
//! - Description logics (Baader et al. 2003) → expressivity baseline

use crate::ast::Module;

/// M190: OWL/JSON-LD ontology emitter.
///
/// Derives a machine-readable ontology directly from parsed AST, eliminating
/// ontology drift: the ontology IS the code.
pub struct OwlEmitter;

impl OwlEmitter {
    /// Create a new OWL emitter.
    pub fn new() -> Self {
        OwlEmitter
    }

    /// Emit a JSON-LD/OWL ontology document from a parsed module.
    ///
    /// Returns a pretty-printed JSON string.
    pub fn emit(&self, module: &Module) -> String {
        let base_iri = format!("https://loom.lang/{}#", module.name);
        let ontology_iri = format!("https://loom.lang/{}", module.name);

        let mut graph: Vec<String> = Vec::new();

        // Ontology node
        graph.push(self.emit_ontology_node(module, &ontology_iri));

        // Being classes
        for being in &module.being_defs {
            graph.push(self.emit_class_node(being, &base_iri));
        }

        // Object properties from relates_to declarations
        for being in &module.being_defs {
            for rel in &being.relates_to {
                graph.push(self.emit_object_property(&being.name, rel, &base_iri));
            }
        }

        let graph_items = graph.join(",\n    ");
        let domains_comment = if module.domains.is_empty() {
            String::new()
        } else {
            format!(
                "  \"loom:domains\": {},\n",
                json_string_array(&module.domains)
            )
        };

        format!(
            "{{\n  \"@context\": {context},\n{domains_comment}  \"@graph\": [\n    {graph}\n  ]\n}}",
            context = self.emit_context(&base_iri),
            domains_comment = domains_comment,
            graph = graph_items,
        )
    }

    /// Emit the `@context` block.
    fn emit_context(&self, base_iri: &str) -> String {
        format!(
            "{{\n    \"@base\": \"{base}\",\n    \"owl\": \"http://www.w3.org/2002/07/owl#\",\n    \"rdfs\": \"http://www.w3.org/2000/01/rdf-schema#\",\n    \"xsd\": \"http://www.w3.org/2001/XMLSchema#\",\n    \"loom\": \"https://loom.lang/vocab#\"\n  }}",
            base = base_iri,
        )
    }

    /// Emit the ontology declaration node.
    fn emit_ontology_node(&self, module: &Module, ontology_iri: &str) -> String {
        let label = json_escape(&module.name);
        let comment = module
            .describe
            .as_deref()
            .map(|d| format!(",\n      \"rdfs:comment\": \"{}\"", json_escape(d)))
            .unwrap_or_default();
        format!(
            "{{\n      \"@id\": \"{iri}\",\n      \"@type\": \"owl:Ontology\",\n      \"rdfs:label\": \"{label}\"{comment}\n    }}",
            iri = ontology_iri,
            label = label,
            comment = comment,
        )
    }

    /// Emit an `owl:Class` node for a being.
    fn emit_class_node(&self, being: &crate::ast::BeingDef, base_iri: &str) -> String {
        let class_iri = format!("{}{}", base_iri, being.name);
        let telos_comment = being
            .telos
            .as_ref()
            .map(|t| {
                format!(
                    ",\n      \"rdfs:comment\": \"{}\"",
                    json_escape(&t.description)
                )
            })
            .unwrap_or_default();
        let role_annotation = being
            .role
            .as_deref()
            .map(|r| format!(",\n      \"loom:role\": \"{}\"", json_escape(r)))
            .unwrap_or_default();
        let describe_annotation = being
            .describe
            .as_deref()
            .map(|d| format!(",\n      \"rdfs:label\": \"{}\"", json_escape(d)))
            .unwrap_or_default();
        format!(
            "{{\n      \"@id\": \"{iri}\",\n      \"@type\": \"owl:Class\"{telos}{role}{describe}\n    }}",
            iri = class_iri,
            telos = telos_comment,
            role = role_annotation,
            describe = describe_annotation,
        )
    }

    /// Emit an `owl:ObjectProperty` node from a `relates_to` declaration.
    fn emit_object_property(
        &self,
        source_name: &str,
        rel: &crate::ast::RelatesTo,
        base_iri: &str,
    ) -> String {
        // Property IRI convention: {base}{Source}_{kind}_{Target}
        let prop_iri = format!("{}{}_{}_{}", base_iri, source_name, rel.kind, rel.target);
        let domain_iri = format!("{}{}", base_iri, source_name);
        let range_iri = format!("{}{}", base_iri, rel.target);
        format!(
            "{{\n      \"@id\": \"{prop}\",\n      \"@type\": \"owl:ObjectProperty\",\n      \"rdfs:domain\": {{\"@id\": \"{domain}\"}},\n      \"rdfs:range\": {{\"@id\": \"{range}\"}},\n      \"loom:kind\": \"{kind}\"\n    }}",
            prop = prop_iri,
            domain = domain_iri,
            range = range_iri,
            kind = rel.kind,
        )
    }
}

impl Default for OwlEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for safe embedding in JSON.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Format a `Vec<String>` as a JSON array of strings.
fn json_string_array(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", json_escape(s)))
        .collect();
    format!("[{}]", quoted.join(", "))
}
