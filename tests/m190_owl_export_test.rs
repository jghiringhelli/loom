//! M190 tests — `.owl.json` ontology export.
//!
//! The OWL emitter derives a JSON-LD/OWL document from parsed AST:
//! - Module domains → `loom:domains` array
//! - Beings → `owl:Class` nodes
//! - `role:` annotation → `loom:role` on the class
//! - `relates_to:` → `owl:ObjectProperty` nodes with domain/range/kind
//! - `telos:` → `rdfs:comment` on the class

use loom::codegen::OwlEmitter;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> loom::ast::Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn owl(src: &str) -> String {
    let module = parse(src);
    OwlEmitter::new().emit(&module)
}

// ── 1. Minimal module → ontology header ──────────────────────────────────────

#[test]
fn owl_minimal_module_emits_ontology_node() {
    let src = "module BioSys\nend";
    let out = owl(src);
    assert!(
        out.contains("\"@type\": \"owl:Ontology\""),
        "must emit ontology node; got:\n{}", out
    );
    assert!(
        out.contains("BioSys"),
        "module name must appear in output; got:\n{}", out
    );
}

// ── 2. domain: annotation on module ─────────────────────────────────────────

#[test]
fn owl_module_domain_emitted_as_array() {
    let src = "module Climate\n  domain: climate energy\nend";
    let out = owl(src);
    assert!(
        out.contains("\"loom:domains\""),
        "must emit loom:domains field; got:\n{}", out
    );
    assert!(
        out.contains("\"climate\"") && out.contains("\"energy\""),
        "both domain tags must appear; got:\n{}", out
    );
}

// ── 3. Being → owl:Class ─────────────────────────────────────────────────────

#[test]
fn owl_being_emitted_as_class() {
    let src = r#"module Bio
being Cell
  telos: "survive and replicate"
  matter:
    atp: Float
  end
end
end"#;
    let out = owl(src);
    assert!(
        out.contains("\"@type\": \"owl:Class\""),
        "being must emit owl:Class; got:\n{}", out
    );
    assert!(
        out.contains("Cell"),
        "class IRI must contain being name; got:\n{}", out
    );
}

// ── 4. telos → rdfs:comment ──────────────────────────────────────────────────

#[test]
fn owl_telos_emitted_as_rdfs_comment() {
    let src = r#"module Bio
being Neuron
  telos: "transmit electrical signals"
  matter:
    potential: Float
  end
end
end"#;
    let out = owl(src);
    assert!(
        out.contains("\"rdfs:comment\": \"transmit electrical signals\""),
        "telos must appear as rdfs:comment; got:\n{}", out
    );
}

// ── 5. role: → loom:role annotation ─────────────────────────────────────────

#[test]
fn owl_role_emitted_as_loom_role_annotation() {
    let src = r#"module Env
being TemperatureSensor
  role: sensor
  telos: "measure temperature"
  matter:
    reading: Float
  end
end
end"#;
    let out = owl(src);
    assert!(
        out.contains("\"loom:role\": \"sensor\""),
        "role must emit loom:role annotation; got:\n{}", out
    );
}

// ── 6. relates_to: → owl:ObjectProperty ─────────────────────────────────────

#[test]
fn owl_relates_to_emitted_as_object_property() {
    let src = r#"module Eco
being Predator
  relates_to: Prey kind: parasitic
  telos: "hunt prey"
  matter:
    energy: Float
  end
end
being Prey
  telos: "evade predators"
  matter:
    energy: Float
  end
end
end"#;
    let out = owl(src);
    assert!(
        out.contains("\"@type\": \"owl:ObjectProperty\""),
        "relates_to must emit owl:ObjectProperty; got:\n{}", out
    );
    assert!(
        out.contains("Predator") && out.contains("Prey"),
        "domain and range IRIs must reference both beings; got:\n{}", out
    );
    assert!(
        out.contains("\"loom:kind\": \"parasitic\""),
        "relationship kind must be annotated; got:\n{}", out
    );
}

// ── 7. relates_to domain/range IRIs ─────────────────────────────────────────

#[test]
fn owl_object_property_has_domain_and_range() {
    let src = r#"module Eco
being Predator
  relates_to: Prey kind: parasitic
  telos: "hunt prey"
  matter:
    energy: Float
  end
end
being Prey
  telos: "evade predators"
  matter:
    energy: Float
  end
end
end"#;
    let out = owl(src);
    assert!(
        out.contains("\"rdfs:domain\""),
        "object property must have rdfs:domain; got:\n{}", out
    );
    assert!(
        out.contains("\"rdfs:range\""),
        "object property must have rdfs:range; got:\n{}", out
    );
}

// ── 8. Multiple beings → multiple classes ────────────────────────────────────

#[test]
fn owl_multiple_beings_emit_multiple_classes() {
    let src = r#"module Eco
being Predator
  telos: "hunt"
  matter:
    energy: Float
  end
end
being Prey
  telos: "evade"
  matter:
    energy: Float
  end
end
being Decomposer
  telos: "recycle nutrients"
  matter:
    biomass: Float
  end
end
end"#;
    let out = owl(src);
    let class_count = out.matches("\"@type\": \"owl:Class\"").count();
    assert_eq!(class_count, 3, "three beings must produce three owl:Class nodes; got:\n{}", out);
}

// ── 9. JSON structure is valid (basic shape checks) ──────────────────────────

#[test]
fn owl_output_has_context_and_graph() {
    let src = "module Test\nend";
    let out = owl(src);
    assert!(out.contains("\"@context\""), "must have @context; got:\n{}", out);
    assert!(out.contains("\"@graph\""), "must have @graph; got:\n{}", out);
    // Balanced braces (crude but fast sanity check)
    let open = out.chars().filter(|&c| c == '{').count();
    let close = out.chars().filter(|&c| c == '}').count();
    assert_eq!(open, close, "JSON braces must be balanced; got:\n{}", out);
}

// ── 10. Base IRI contains module name ────────────────────────────────────────

#[test]
fn owl_base_iri_contains_module_name() {
    let src = "module ClimateModel\nend";
    let out = owl(src);
    assert!(
        out.contains("https://loom.lang/ClimateModel"),
        "base IRI must contain module name; got:\n{}", out
    );
}
