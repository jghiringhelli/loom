//! Tests for the 7 BioISO constructs (M117-M119, propagate, epigenetic duration,
//! signal_attention named lists, ecosystem tipping_points).

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ── Feature 1: propagate: block ───────────────────────────────────────────────

#[test]
fn propagate_block_parses_correctly() {
    let src = r#"
module BioTest

being MolCell
  @mortal @corrigible
  matter:
    energy: Float
  end
  telos: "maximize energy efficiency"
  end
  propagate:
    condition: telos.score > 0.85
    inherits: [matter, telos]
    mutates: energy within quantum_bounds
  end
end

end
"#;
    let m = parse_module(src);
    let b = m.being_defs.first().expect("should have being");
    assert!(
        b.propagate_block.is_some(),
        "propagate_block should be Some"
    );
    let p = b.propagate_block.as_ref().unwrap();
    assert!(
        p.condition.contains("0.85"),
        "condition should contain threshold"
    );
    assert!(
        p.inherits.contains(&"matter".to_string()),
        "inherits should contain matter"
    );
    assert!(
        p.inherits.contains(&"telos".to_string()),
        "inherits should contain telos"
    );
    assert!(!p.mutates.is_empty(), "mutates should be non-empty");
    assert_eq!(p.mutates[0].0, "energy");
}

#[test]
fn propagate_block_with_offspring_type() {
    let src = r#"
module BioTest

being MolCell
  @mortal
  matter:
    energy: Float
  end
  telos: "maximize energy efficiency"
  end
  propagate:
    condition: telos.score > 0.9
    inherits: [matter]
    offspring_type: MutantCell
  end
end

end
"#;
    let m = parse_module(src);
    let b = m.being_defs.first().unwrap();
    let p = b.propagate_block.as_ref().unwrap();
    assert_eq!(p.offspring_type.as_deref(), Some("MutantCell"));
}

// ── Feature 2: epigenetic duration ───────────────────────────────────────────

#[test]
fn epigenetic_duration_parses_correctly() {
    let src = r#"
module EpiTest

being NeuralCell
  @mortal
  matter:
    adaptability: Float
  end
  telos: "maintain homeostasis"
  end
  epigenetic:
    signal: stress_hormone
    modifies: adaptability
    duration: 18.months
  end
end

end
"#;
    let m = parse_module(src);
    let b = m.being_defs.first().expect("should have being");
    assert!(
        !b.epigenetic_blocks.is_empty(),
        "should have epigenetic block"
    );
    let epi = &b.epigenetic_blocks[0];
    assert_eq!(epi.signal, "stress_hormone");
    // duration is parsed from "18.months" tokens
    assert!(epi.duration.is_some(), "duration field should be parsed");
    let dur = epi.duration.as_ref().unwrap();
    assert!(!dur.is_empty(), "duration should not be empty");
}

// ── Feature 3: signal_attention named lists ───────────────────────────────────

#[test]
fn signal_attention_named_prioritize_list() {
    let src = r#"
module SigTest

being Trader
  @mortal
  matter:
    capital: Float
  end
  telos: "maximize returns"
  end
  signal_attention
    prioritize: [price_signal, volume_signal]
    attenuate: [noise_signal]
  end
end

end
"#;
    let m = parse_module(src);
    let b = m.being_defs.first().expect("should have being");
    let sa = b
        .signal_attention
        .as_ref()
        .expect("should have signal_attention");
    assert!(
        sa.prioritize_named.contains(&"price_signal".to_string()),
        "prioritize_named should contain price_signal"
    );
    assert!(
        sa.prioritize_named.contains(&"volume_signal".to_string()),
        "prioritize_named should contain volume_signal"
    );
    assert!(
        sa.attenuate_named.contains(&"noise_signal".to_string()),
        "attenuate_named should contain noise_signal"
    );
}

#[test]
fn signal_attention_telos_relevance() {
    let src = r#"
module SigTest

being Processor
  @mortal
  matter:
    state: Int
  end
  telos: "maximize throughput"
  end
  signal_attention
    prioritize: 0.8
    attenuate: 0.2
    telos_relevance: computed_from_fitness
  end
end

end
"#;
    let m = parse_module(src);
    let b = m.being_defs.first().unwrap();
    let sa = b.signal_attention.as_ref().unwrap();
    assert_eq!(sa.prioritize_above, Some(0.8));
    assert_eq!(sa.attenuate_below, Some(0.2));
    assert!(
        sa.telos_relevance.is_some(),
        "telos_relevance should be parsed"
    );
    let tr = sa.telos_relevance.as_ref().unwrap();
    assert!(tr.contains("computed_from_fitness"));
}

// ── Feature 4: ecosystem tipping_points ──────────────────────────────────────

#[test]
fn ecosystem_tipping_points_parse() {
    let src = r#"
module EcoTest

ecosystem AmazonRainforest
  members: [VegetationBeing, HydrologyCycle]
  telos: "maintain biodiversity and carbon cycling"
  coevolution: true
  tipping_points:
    amazon_dieback:
      condition: vegetation_coverage < 0.60
      on_crossing: escalate to human_regulated
  end
end

end
"#;
    let m = parse_module(src);
    assert!(!m.ecosystem_defs.is_empty(), "should have ecosystem");
    let eco = &m.ecosystem_defs[0];
    assert_eq!(eco.name, "AmazonRainforest");
    assert!(eco.coevolution, "coevolution should be true");
    assert!(!eco.tipping_points.is_empty(), "should have tipping points");
    let tp = &eco.tipping_points[0];
    assert_eq!(tp.name, "amazon_dieback");
    assert!(
        tp.condition.contains("vegetation_coverage"),
        "condition should mention vegetation"
    );
}

#[test]
fn ecosystem_collective_telos_metric() {
    let src = r#"
module EcoTest

ecosystem Coral
  members: [CoralPolyp, Zooxanthellae]
  telos: "maintain reef ecosystem"
  collective_telos_metric: mean_telos_score
end

end
"#;
    let m = parse_module(src);
    let eco = &m.ecosystem_defs[0];
    assert_eq!(
        eco.collective_telos_metric.as_deref(),
        Some("mean_telos_score")
    );
}

// ── Feature 5: telos_function top-level ──────────────────────────────────────

#[test]
fn telos_function_parses_correctly() {
    let src = r#"
module TFTest

telos_function RiskAdjustedConvergence
  statement: "converge risk-adjusted PnL toward equilibrium"
  bounded_by: portfolio_constraint
  measured_by: sharpe_ratio_metric
  guides: [signal_attention, resource_allocation]
end

end
"#;
    let m = parse_module(src);
    let tf_item = m
        .items
        .iter()
        .find(|item| matches!(item, Item::TelosFunction(_)));
    assert!(tf_item.is_some(), "should have TelosFunction item");
    if let Some(Item::TelosFunction(tf)) = tf_item {
        assert_eq!(tf.name, "RiskAdjustedConvergence");
        assert!(tf.statement.is_some());
        assert!(tf.statement.as_deref().unwrap().contains("equilibrium"));
        assert_eq!(tf.bounded_by.as_deref(), Some("portfolio_constraint"));
        assert!(tf.guides.contains(&"signal_attention".to_string()));
        assert!(tf.guides.contains(&"resource_allocation".to_string()));
    }
}

// ── Feature 6: entity universal primitive ────────────────────────────────────

#[test]
fn entity_directed_acyclic_parses() {
    let src = r#"
module EntityTest

entity DependencyGraph<Module, Import>
  @directed @acyclic
end

end
"#;
    let m = parse_module(src);
    let ent_item = m.items.iter().find(|item| matches!(item, Item::Entity(_)));
    assert!(ent_item.is_some(), "should have Entity item");
    if let Some(Item::Entity(ent)) = ent_item {
        assert_eq!(ent.name, "DependencyGraph");
        assert_eq!(ent.node_type.as_deref(), Some("Module"));
        assert_eq!(ent.edge_type.as_deref(), Some("Import"));
        assert!(
            ent.annotations.contains(&"directed".to_string()),
            "should have directed annotation"
        );
        assert!(
            ent.annotations.contains(&"acyclic".to_string()),
            "should have acyclic annotation"
        );
    }
}

#[test]
fn entity_without_type_params() {
    let src = r#"
module EntityTest

entity SimpleGraph
  @undirected
end

end
"#;
    let m = parse_module(src);
    let ent_item = m.items.iter().find(|item| matches!(item, Item::Entity(_)));
    assert!(ent_item.is_some(), "should have Entity item");
    if let Some(Item::Entity(ent)) = ent_item {
        assert_eq!(ent.name, "SimpleGraph");
        assert!(ent.node_type.is_none());
        assert!(ent.annotations.contains(&"undirected".to_string()));
    }
}

// ── Feature 7: intent_coordinator ────────────────────────────────────────────

#[test]
fn intent_coordinator_parses_correctly() {
    let src = r#"
module IntentTest

intent_coordinator TradeStrategyCoordinator
  telomere: 90.days
  governance: ai_proposes
  signals: [market_data, user_feedback, risk_metrics]
  min_confidence: 0.85
  rollback_on: "intent_score drops below 0.7"
  audit_path: "logs/intent_changes.json"
end

end
"#;
    let m = parse_module(src);
    let ic_item = m
        .items
        .iter()
        .find(|item| matches!(item, Item::IntentCoordinator(_)));
    assert!(ic_item.is_some(), "should have IntentCoordinator item");
    if let Some(Item::IntentCoordinator(ic)) = ic_item {
        assert_eq!(ic.name, "TradeStrategyCoordinator");
        assert_eq!(ic.telomere_days, Some(90));
        assert_eq!(ic.governance_class, GovernanceClass::AiProposes);
        assert_eq!(ic.signals.len(), 3);
        assert!(ic.signals.iter().any(|s| s.name == "market_data"));
        assert_eq!(ic.min_confidence, Some(0.85));
        assert!(ic.rollback_on.is_some());
        assert!(ic.audit_path.is_some());
        assert_eq!(ic.audit_path.as_deref(), Some("logs/intent_changes.json"));
    }
}

// ── M120-M125: entity 3-dimension checker tests ───────────────────────────────

fn parse_expect_err(src: &str) -> Vec<String> {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    use loom::checker::{EntityChecker, LoomChecker};
    EntityChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect()
}

// ── Dimension 1: Structural rules ────────────────────────────────────────────

#[test]
fn entity_directed_undirected_conflict_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @directed @undirected end
end
"#,
    );
    assert!(!errors.is_empty(), "should reject @directed @undirected");
    assert!(errors[0].contains("mutually exclusive"));
}

#[test]
fn entity_acyclic_without_directionality_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @acyclic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @acyclic without direction"
    );
    assert!(errors[0].contains("@acyclic requires"));
}

#[test]
fn entity_layered_without_directed_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @layered end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @layered without @directed"
    );
    assert!(errors[0].contains("@layered requires @directed"));
}

#[test]
fn entity_hierarchical_without_directed_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @hierarchical @acyclic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @hierarchical without @directed"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("@hierarchical requires @directed")));
}

#[test]
fn entity_hierarchical_without_acyclic_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @directed @hierarchical end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @hierarchical without @acyclic"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("@hierarchical requires @acyclic")));
}

#[test]
fn entity_weighted_without_edge_type_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Node> @directed @weighted end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @weighted without edge type"
    );
    assert!(errors[0].contains("@weighted requires an edge type"));
}

#[test]
fn entity_valid_dag_accepted() {
    let errors = parse_expect_err(
        r#"
module T
entity DepGraph<Module, Import> @directed @acyclic end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid DAG should produce no errors: {:?}",
        errors
    );
}

#[test]
fn entity_valid_hierarchical_tree_accepted() {
    let errors = parse_expect_err(
        r#"
module T
entity OrgTree<Person, Reports> @directed @acyclic @hierarchical end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid tree should produce no errors: {:?}",
        errors
    );
}

// ── Dimension 2: Formal constraint rules ──────────────────────────────────────

#[test]
fn entity_stochastic_deterministic_conflict_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<State, Float> @directed @stochastic @deterministic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @stochastic @deterministic"
    );
    assert!(errors.iter().any(|e| e.contains("mutually exclusive")));
}

#[test]
fn entity_stochastic_without_edge_type_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<State> @directed @stochastic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @stochastic without edge type"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("@stochastic requires an edge type")));
}

#[test]
fn entity_learnable_without_weighted_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Node, Edge> @directed @learnable end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @learnable without @weighted"
    );
    assert!(errors[0].contains("@learnable requires @weighted"));
}

#[test]
fn entity_causal_without_directed_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Event, Rel> @causal @acyclic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @causal without @directed"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("@causal requires @directed")));
}

#[test]
fn entity_causal_without_acyclic_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Event, Rel> @directed @causal end
end
"#,
    );
    assert!(!errors.is_empty(), "should reject @causal without @acyclic");
    assert!(errors
        .iter()
        .any(|e| e.contains("@causal requires @acyclic")));
}

#[test]
fn entity_temporal_without_directed_rejected() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Event, Duration> @temporal end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @temporal without @directed"
    );
    assert!(errors[0].contains("@temporal requires @directed"));
}

#[test]
fn entity_valid_markov_chain_accepted() {
    let errors = parse_expect_err(
        r#"
module T
entity Weather<State, Float> @directed @stochastic @finite end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid Markov chain should produce no errors: {:?}",
        errors
    );
}

#[test]
fn entity_valid_neural_net_accepted() {
    let errors = parse_expect_err(
        r#"
module T
entity Net<Neuron, Float> @directed @weighted @learnable end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid neural net should produce no errors: {:?}",
        errors
    );
}

#[test]
fn entity_valid_causal_graph_accepted() {
    let errors = parse_expect_err(
        r#"
module T
entity Causes<Event, Strength> @directed @acyclic @causal end
end
"#,
    );
    assert!(
        errors.is_empty(),
        "valid causal graph should produce no errors: {:?}",
        errors
    );
}

// ── Dimension 3: Orthogonality rules ─────────────────────────────────────────

#[test]
fn entity_hierarchical_undirected_orthogonal_conflict() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad @undirected @hierarchical @acyclic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @hierarchical @undirected"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("orthogonally incompatible")));
}

#[test]
fn entity_causal_stochastic_orthogonal_conflict() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Event, Float> @directed @acyclic @causal @stochastic end
end
"#,
    );
    assert!(!errors.is_empty(), "should reject @causal @stochastic");
    assert!(errors
        .iter()
        .any(|e| e.contains("orthogonally incompatible")));
}

#[test]
fn entity_learnable_deterministic_orthogonal_conflict() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Node, Float> @directed @weighted @learnable @deterministic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @learnable @deterministic"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("orthogonally incompatible")));
}

#[test]
fn entity_telos_guided_deterministic_orthogonal_conflict() {
    let errors = parse_expect_err(
        r#"
module T
entity Bad<Node, Edge> @directed @telos_guided @deterministic end
end
"#,
    );
    assert!(
        !errors.is_empty(),
        "should reject @telos_guided @deterministic"
    );
    assert!(errors
        .iter()
        .any(|e| e.contains("orthogonally incompatible")));
}

// ── M123-M125: entity codegen — known type aliases ────────────────────────────

fn compile_entity_emit(src: &str) -> String {
    use loom::codegen::rust::RustEmitter;
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

#[test]
fn entity_codegen_dag_emits_digraph() {
    let out = compile_entity_emit(
        r#"
module T
entity DepGraph<Module, Import> @directed @acyclic end
end
"#,
    );
    assert!(
        out.contains("DiGraph<Module, Import>"),
        "DAG should emit DiGraph: {}",
        out
    );
    assert!(out.contains("pub type DepGraph"), "should emit pub type");
}

#[test]
fn entity_codegen_undirected_emits_ungraph() {
    let out = compile_entity_emit(
        r#"
module T
entity KG<Concept, Rel> @undirected @semantic end
end
"#,
    );
    assert!(
        out.contains("UnGraph<Concept, Rel>"),
        "undirected should emit UnGraph: {}",
        out
    );
}

#[test]
fn entity_codegen_stochastic_emits_guidance_comment() {
    let out = compile_entity_emit(
        r#"
module T
entity Markov<State, Float> @directed @stochastic @finite end
end
"#,
    );
    assert!(
        out.contains("LOOM[stochastic]"),
        "stochastic entity should emit guidance: {}",
        out
    );
}

#[test]
fn entity_codegen_learnable_emits_guidance_comment() {
    let out = compile_entity_emit(
        r#"
module T
entity Net<Neuron, Float> @directed @weighted @learnable end
end
"#,
    );
    assert!(
        out.contains("LOOM[learnable]"),
        "learnable entity should emit guidance: {}",
        out
    );
}

#[test]
fn entity_codegen_alias_of_emits_description() {
    let src = r#"
module T
entity WeatherModel<State, Float>
  @directed @stochastic @finite
  alias_of: MarkovChain
end
end
"#;
    let out = compile_entity_emit(src);
    assert!(
        out.contains("instance of: MarkovChain"),
        "should emit alias comment: {}",
        out
    );
}

#[test]
fn entity_codegen_causal_emits_guidance_comment() {
    let out = compile_entity_emit(
        r#"
module T
entity CauseEffect<Event, Strength> @directed @acyclic @causal end
end
"#,
    );
    assert!(
        out.contains("LOOM[causal]"),
        "causal entity should emit guidance: {}",
        out
    );
}

#[test]
fn intent_coordinator_governance_classes() {
    let src = r#"
module IntentTest

intent_coordinator ImmutableIntent
  governance: blocked
end

end
"#;
    let m = parse_module(src);
    let ic_item = m
        .items
        .iter()
        .find(|item| matches!(item, Item::IntentCoordinator(_)));
    assert!(ic_item.is_some());
    if let Some(Item::IntentCoordinator(ic)) = ic_item {
        assert_eq!(ic.governance_class, GovernanceClass::Blocked);
    }
}
