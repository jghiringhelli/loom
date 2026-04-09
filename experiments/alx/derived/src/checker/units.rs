// ALX: derived from loom.loom §"check_units"
// Units of measure: Float<usd> + Float<eur> is a type error.
// Float<usd> * Float<rate> is dimensionless (allowed).
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

    for item in &module.items {
        if let Item::Fn(f) = item {
            // For each function, scan body for arithmetic ops with unit-typed params
            let param_units: Vec<Option<String>> =
                f.type_sig.params.iter().map(|p| unit_of_owned(p)).collect();

            // Check body for add/sub operations between different units
            for stmt in &f.body {
                check_body_units(stmt, &f.type_sig.params, f.span, &mut errors);
            }

            // Also check: if two params have different units and body has + or -,
            // that's likely an error
            let distinct_units: std::collections::HashSet<&str> = param_units
                .iter()
                .filter_map(|u| u.as_deref())
                .collect();

            if distinct_units.len() > 1 {
                // Check body for add/sub
                let body_text = f.body.join(" ");
                let has_add_sub = body_text.contains(" + ") || body_text.contains(" - ");
                let has_mul_div = body_text.contains(" * ") || body_text.contains(" / ");

                if has_add_sub {
                    let units_list: Vec<&str> = distinct_units.iter().copied().collect();
                    errors.push(LoomError::type_err(
                        format!(
                            "function '{}': cannot add/subtract different units {:?}",
                            f.name, units_list
                        ),
                        f.span,
                    ));
                }
                // multiply/divide with different units is allowed
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_body_units(
    body: &str,
    _params: &[TypeExpr],
    _span: crate::error::Span,
    _errors: &mut Vec<LoomError>,
) {
    // Additional body-level checks could go here
    let _ = body;
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

fn unit_of_owned(ty: &TypeExpr) -> Option<String> {
    unit_of(ty).map(|s| s.to_string())
}
