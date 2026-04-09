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
    // 12. Interface implementation checker
    check_implements(module)?;
    // 13. DI checker
    check_di(module)?;
    Ok(())
}

/// Check dependency injection: all `with dep` references must appear in `requires`.
fn check_di(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Collect declared dep names
    let declared: std::collections::HashSet<&str> = module.requires
        .as_ref()
        .map(|r| r.deps.iter().map(|(n, _)| n.as_str()).collect())
        .unwrap_or_default();

    for item in &module.items {
        if let crate::ast::Item::Fn(f) = item {
            for dep in &f.with_deps {
                if module.requires.is_none() || !declared.contains(dep.as_str()) {
                    errors.push(LoomError::UndeclaredDependency {
                        name: dep.clone(),
                        span: f.span,
                    });
                }
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Verify that every `implements X` declaration provides all methods required by interface X.
fn check_implements(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();
    // Collect all function names in the module
    let fn_names: std::collections::HashSet<&str> = module.items.iter()
        .filter_map(|i| if let crate::ast::Item::Fn(f) = i { Some(f.name.as_str()) } else { None })
        .collect();

    for trait_name in &module.implements {
        // Find the interface definition
        if let Some(iface) = module.interface_defs.iter().find(|i| &i.name == trait_name) {
            for method in &iface.methods {
                if !fn_names.contains(method.name.as_str()) {
                    errors.push(LoomError::new(
                        format!("implements {}: missing method '{}' required by interface", trait_name, method.name),
                        module.span,
                    ));
                }
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
