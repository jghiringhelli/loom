// ALX: derived from loom.loom §"check_effects"
// Effect tracking: every effectful call must be in an Effect context.
// Effects propagate transitively through the call chain.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;

/// G4: EffectChecker struct — tests call `EffectChecker::new().check(&module)`.
pub struct EffectChecker;

impl EffectChecker {
    pub fn new() -> Self { EffectChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_effects(module)
    }
}

pub fn check_effects(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Consequence tier ordering: Irreversible > Reversible > (none)
    // @exactly-once requires Effect return type.
    for item in &module.items {
        if let Item::Fn(f) = item {
            let has_exactly_once = f.annotations.iter().any(|a| a.key == "exactly-once");
            let returns_effect = matches!(f.type_sig.return_type, TypeExpr::Effect(_, _));

            if has_exactly_once && !returns_effect {
                errors.push(LoomError::new(
                    format!(
                        "function '{}': @exactly-once requires an Effect return type",
                        f.name
                    ),
                    f.span,
                ));
            }
        }
    }

    // Check that functions in function: blocks of beings declare Effect types
    // when they perform side effects.
    // ALX: full transitive effect propagation requires a call graph; skipped for now.
    // This is a known limitation.

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
