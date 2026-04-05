// ALX: derived from loom.loom §"Pipeline: Safety checker (M55)"
// SafetyChecker — autopoietic safety rules. Runs after TeleosChecker.
// These are compile errors, not warnings. The Three Laws as a type system.

use crate::ast::Module;
use crate::error::LoomError;

/// G4: SafetyChecker struct — tests call `SafetyChecker::check(&module)` (static, no new()).
pub struct SafetyChecker;

impl SafetyChecker {
    pub fn check(module: &Module) -> Vec<LoomError> {
        check_safety(module).err().unwrap_or_default()
    }
}

pub fn check_safety(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    for being in &module.being_defs {
        let ann_keys: Vec<&str> = being.annotations.iter().map(|a| a.key.as_str()).collect();
        let is_autopoietic = being.autopoietic;
        let has_mortal = ann_keys.contains(&"mortal");
        let has_corrigible = ann_keys.contains(&"corrigible");
        let has_sandboxed = ann_keys.contains(&"sandboxed");
        let has_bounded_telos = ann_keys.contains(&"bounded_telos");

        // Rule 1: autopoietic: true without @mortal → error
        if is_autopoietic && !has_mortal {
            errors.push(LoomError::new(
                format!(
                    "being '{}': autopoietic being missing @mortal: \
                     unbounded proliferation is undefined behavior",
                    being.name
                ),
                being.span,
            ));
        }

        // Rule 2: autopoietic: true without @sandboxed → error
        if is_autopoietic && !has_sandboxed {
            errors.push(LoomError::new(
                format!(
                    "being '{}': autopoietic being missing @sandboxed: \
                     effects outside declared surface are unsafe",
                    being.name
                ),
                being.span,
            ));
        }

        // Rule 3: @mortal without telomere: block → error
        if has_mortal && being.telomere.is_none() {
            errors.push(LoomError::new(
                format!(
                    "being '{}': @mortal requires telomere: block with finite limit",
                    being.name
                ),
                being.span,
            ));
        }

        // Rule 4: @corrigible without telos.modifiable_by → error
        if has_corrigible {
            let has_modifiable_by = being
                .telos
                .as_ref()
                .and_then(|t| t.modifiable_by.as_ref())
                .is_some();
            if !has_modifiable_by {
                errors.push(LoomError::new(
                    format!(
                        "being '{}': @corrigible requires telos.modifiable_by: field",
                        being.name
                    ),
                    being.span,
                ));
            }
        }

        // Rule 5: @bounded_telos with open-ended utility terms in telos description → error
        if has_bounded_telos {
            if let Some(telos) = &being.telos {
                let desc_lower = telos.description.to_lowercase();
                let open_ended = ["maximize", "unlimited", "any", "all"];
                for term in &open_ended {
                    if desc_lower.contains(term) {
                        errors.push(LoomError::new(
                            format!(
                                "being '{}': @bounded_telos: telos description contains \
                                 open-ended utility term '{}'",
                                being.name, term
                            ),
                            being.span,
                        ));
                        break;
                    }
                }

                // Rule 6: @bounded_telos without telos.bounded_by → error
                if telos.bounded_by.is_none() {
                    errors.push(LoomError::new(
                        format!(
                            "being '{}': @bounded_telos requires telos.bounded_by: field",
                            being.name
                        ),
                        being.span,
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
