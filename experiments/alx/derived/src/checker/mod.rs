// ALX: derived from loom.loom §"Pipeline: Checkers"
// Checkers run in the order specified by the spec. Any error short-circuits.

pub mod inference;
pub mod types;
pub mod exhaustiveness;
pub mod effects;
pub mod algebraic;
pub mod units;
pub mod typestate;
pub mod privacy;
pub mod infoflow;
pub mod teleos;
pub mod safety;

// G4: Re-export checker structs and check_teleos so tests can import from loom::checker::*
pub use effects::EffectChecker;
pub use privacy::PrivacyChecker;
pub use safety::SafetyChecker;
pub use infoflow::InfoFlowChecker;
pub use typestate::TypestateChecker;
pub use units::UnitsChecker;
pub use teleos::check_teleos;

use crate::ast::Module;
use crate::error::LoomError;

/// Run all checkers in the order mandated by loom.loom.
/// Returns on the first checker that reports errors.
pub fn run_all(module: &Module) -> Result<(), Vec<LoomError>> {
    // 1. HM type inference
    inference::check_inference(module)?;
    // 2. Symbol resolution
    types::check_types(module)?;
    // 3. Pattern match exhaustiveness
    exhaustiveness::check_exhaustiveness(module)?;
    // 4. Effect tracking
    effects::check_effects(module)?;
    // 5. Algebraic properties
    algebraic::check_algebraic(module)?;
    // 6. Units of measure
    units::check_units(module)?;
    // 7. Typestate protocols
    typestate::check_typestate(module)?;
    // 8. Privacy labels
    privacy::check_privacy(module)?;
    // 9. Information flow
    infoflow::check_infoflow(module)?;
    // 10. Teleological checker
    teleos::check_teleos(module)?;
    // 11. Safety checker (M55)
    safety::check_safety(module)?;
    Ok(())
}
