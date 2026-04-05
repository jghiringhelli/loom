// ALX: derived from loom.loom §"check_infoflow" and language-spec.md §9.5
// Information flow: secret data cannot flow to public outputs without declassification.
// flow secret :: Password means Password cannot appear in a @public output without
// an explicit declassification function.

use crate::ast::{Module, Item, TypeExpr};
use crate::error::LoomError;
use std::collections::{HashMap, HashSet};

/// G4: InfoFlowChecker struct — tests call `InfoFlowChecker::new().check(&module)`.
pub struct InfoFlowChecker;

impl InfoFlowChecker {
    pub fn new() -> Self { InfoFlowChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_infoflow(module)
    }
}

/// Labels: "secret" > "tainted" > "public"
pub fn check_infoflow(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Build label map: type_name -> label
    let mut labels: HashMap<String, String> = HashMap::new();
    for fl in &module.flow_labels {
        for ty_name in &fl.types {
            labels.insert(ty_name.clone(), fl.label.clone());
        }
    }

    let secret_types: HashSet<&String> = labels
        .iter()
        .filter(|(_, v)| v.as_str() == "secret")
        .map(|(k, _)| k)
        .collect();
    let public_types: HashSet<&String> = labels
        .iter()
        .filter(|(_, v)| v.as_str() == "public")
        .map(|(k, _)| k)
        .collect();

    // For each function, if a param is secret and the return type is public, error.
    for item in &module.items {
        if let Item::Fn(f) = item {
            let has_secret_param = f.type_sig.params.iter().any(|p| {
                type_name_of(p)
                    .map(|n| secret_types.contains(&n))
                    .unwrap_or(false)
            });
            let returns_public = type_name_of(&f.type_sig.return_type)
                .map(|n| public_types.contains(&n))
                .unwrap_or(false);

            if has_secret_param && returns_public {
                // Check for declassification hint in function name
                let is_declassification = f.name.contains("declassify")
                    || f.name.contains("sanitize")
                    || f.name.contains("hash")
                    || f.name.contains("anonymize");

                if !is_declassification {
                    errors.push(LoomError::new(
                        format!(
                            "function '{}': secret data flows to public output without declassification",
                            f.name
                        ),
                        f.span,
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn type_name_of(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Base(n) => Some(n.clone()),
        TypeExpr::Generic(n, _) => Some(n.clone()),
        _ => None,
    }
}
