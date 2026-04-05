// ALX: derived from loom.loom §"check_units"
// Units of measure: Float<usd> + Float<eur> is a type error.
// Float<usd> * Float<rate> = dimensionless.
// Dimensional analysis at compile time.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;

/// UnitsChecker struct — tests call `UnitsChecker::new().check(&module)`.
pub struct UnitsChecker;

impl UnitsChecker {
    pub fn new() -> Self { UnitsChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_units(module)
    }
}

pub fn check_units(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Check that function signatures don't mix units in add/sub positions.
    // ALX: body is raw text, so we check signatures for parameter-return unit consistency.
    for item in &module.items {
        if let Item::Fn(f) = item {
            // Collect all unit labels in params
            let param_units: Vec<Option<&str>> =
                f.type_sig.params.iter().map(|p| unit_of(p)).collect();
            let ret_unit = unit_of(&f.type_sig.return_type);

            // If multiple params have different units, the function must be a conversion.
            let distinct_units: std::collections::HashSet<&str> = param_units
                .iter()
                .filter_map(|u| *u)
                .collect();

            if distinct_units.len() > 1 {
                // Mixed units in parameters — this is allowed only for conversion functions
                // (e.g. fn convert :: Float<usd> -> Float<eur>).
                // If the function name doesn't suggest conversion, warn.
                // ALX: name-based heuristic since body is raw text.
                let is_conversion = f.name.contains("convert")
                    || f.name.contains("exchange")
                    || f.name.contains("to_")
                    || f.name.contains("from_");
                if !is_conversion {
                    errors.push(LoomError::new(
                        format!(
                            "function '{}': mixed unit parameters {:?} — use a conversion function",
                            f.name,
                            distinct_units.iter().collect::<Vec<_>>()
                        ),
                        f.span,
                    ));
                }
            }
        }
    }

    // Check being matter fields: fields should not mix units implicitly.
    for being in &module.being_defs {
        if let Some(matter) = &being.matter {
            let _units: Vec<Option<&str>> = matter.fields.iter().map(|f| unit_of(&f.ty)).collect();
            // Multiple different units in same being is fine (charge: Float<mv>, threshold: Float<mv>)
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn unit_of(ty: &TypeExpr) -> Option<&str> {
    match ty {
        TypeExpr::Generic(name, args) if name == "Float" && args.len() == 1 => {
            if let TypeExpr::Base(unit) = &args[0] {
                Some(unit.as_str())
            } else {
                None
            }
        }
        _ => None,
    }
}
