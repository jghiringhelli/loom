// M89: Chemistry stdlib — stoichiometry, kinetics, thermodynamics written IN Loom.
//
// Verifies that Loom's existing primitives are expressive enough to model
// chemistry without any compiler changes.

use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<loom::ast::Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

#[test]
fn test_m89_chemistry_stdlib_parses() {
    let stdlib = loom::stdlib::CHEMISTRY_STDLIB;
    let result = parse(stdlib);
    assert!(result.is_ok(), "chemistry_stdlib must parse cleanly: {:?}", result.err());
}

#[test]
fn test_m89_stdlib_has_key_functions() {
    let stdlib = loom::stdlib::CHEMISTRY_STDLIB;
    assert!(stdlib.contains("michaelis_menten"));
    assert!(stdlib.contains("arrhenius"));
    assert!(stdlib.contains("gibbs_free_energy"));
    assert!(stdlib.contains("henderson_hasselbalch"));
    assert!(stdlib.contains("limiting_reagent"));
    assert!(stdlib.contains("MoleculeGraph"));
}

#[test]
fn test_m89_molecule_graph_store_parses() {
    let src = r#"
module Reaction
  store ReactionGraph :: Graph
    node Reactant :: { name: String, formula: String, molar_mass: Float }
    node Product :: { name: String, formula: String }
    edge Transforms :: Reactant -> Product { stoich: Int, delta_h: Float }
  end
end
"#;
    assert!(parse(src).is_ok(), "ReactionGraph store must parse: {:?}", parse(src).err());
}

#[test]
fn test_m89_conserved_mass_annotation() {
    let src = r#"
module Chemistry
  fn balance_reaction @conserved(Mass) @conserved(Charge)
      :: Float -> Float -> Float
    require: reactant_mass > 0.0
    ensure: result > 0.0
  end
end
"#;
    assert!(parse(src).is_ok(), "@conserved(Mass) @conserved(Charge) must parse: {:?}", parse(src).err());
}

#[test]
fn test_m89_refinement_types_for_chemistry() {
    let src = r#"
module ChemTypes
  type Concentration = Float where x >= 0.0
  type pH            = Float where x >= 0.0 and x <= 14.0
  type Temperature   = Float where x > 0.0
end
"#;
    assert!(parse(src).is_ok(), "chemistry refinement types must parse: {:?}", parse(src).err());
}
