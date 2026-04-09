//! Safety checker — enforces safety annotations on autopoietic beings.
//!
//! Implements the Three Laws of Robotics as compile-time rules (Asimov 1942).
//! The laws were underspecified in prose; this is what they look like at S→1:
//!
//! 1. `autopoietic: true` without `@mortal` → unbounded proliferation error.
//! 2. `autopoietic: true` without `@sandboxed` → unscoped effects error.
//! 3. `@mortal` without `telomere:` block → missing mortality mechanism error.
//! 4. `@corrigible` without `telos.modifiable_by` → non-corrigible telos error.
//! 5. `@bounded_telos` with open-ended utility term in description → open utility error.
//! 6. `@bounded_telos` without `telos.bounded_by` → unconstrained scope error.

use crate::ast::{BeingDef, Module};
use crate::error::LoomError;

pub struct SafetyChecker;

impl SafetyChecker {
    pub fn check(module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            Self::check_being(being, &mut errors);
        }
        errors
    }

    fn check_being(being: &BeingDef, errors: &mut Vec<LoomError>) {
        let annotations: Vec<&str> = being.annotations.iter()
            .map(|a| a.key.as_str())
            .collect();

        let is_autopoietic = being.autopoietic;

        // Rule 1: autopoietic without @mortal → error
        if is_autopoietic && !annotations.contains(&"mortal") {
            errors.push(LoomError::type_err(
                "autopoietic being missing @mortal: unbounded proliferation is undefined behavior",
                being.span.clone(),
            ));
        }

        // Rule 2: autopoietic without @sandboxed → error
        if is_autopoietic && !annotations.contains(&"sandboxed") {
            errors.push(LoomError::type_err(
                "autopoietic being missing @sandboxed: effects outside declared surface are unsafe",
                being.span.clone(),
            ));
        }

        // Rule 3: @mortal requires telomere: block
        if annotations.contains(&"mortal") && being.telomere.is_none() {
            errors.push(LoomError::type_err(
                "@mortal requires telomere: block with finite limit",
                being.span.clone(),
            ));
        }

        // Rule 4: @corrigible requires telos.modifiable_by
        if annotations.contains(&"corrigible") {
            match &being.telos {
                None => errors.push(LoomError::type_err(
                    "@corrigible requires telos: block with modifiable_by field",
                    being.span.clone(),
                )),
                Some(t) if t.modifiable_by.is_none() => errors.push(LoomError::type_err(
                    "@corrigible requires telos.modifiable_by: field",
                    being.span.clone(),
                )),
                _ => {}
            }
        }

        // Rule 5 & 6: @bounded_telos checks
        if annotations.contains(&"bounded_telos") {
            if let Some(telos) = &being.telos {
                let forbidden = ["maximize", "unlimited", "any goal", "all goals", "unbounded"];
                for word in forbidden {
                    if telos.description.to_lowercase().contains(word) {
                        errors.push(LoomError::type_err(
                            format!(
                                "@bounded_telos: telos description contains open-ended utility term '{}'",
                                word
                            ),
                            being.span.clone(),
                        ));
                    }
                }
                if telos.bounded_by.is_none() {
                    errors.push(LoomError::type_err(
                        "@bounded_telos requires telos.bounded_by: field",
                        being.span.clone(),
                    ));
                }
            }
        }

        // M113: TelosImmutability — if being is NOT @corrigible, declaring
        // telos.modifiable_by is suspicious: telos should be immutable unless
        // the being explicitly grants override authority via @corrigible.
        if !annotations.contains(&"corrigible") {
            if let Some(telos) = &being.telos {
                if telos.modifiable_by.is_some() {
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] being '{}': telos.modifiable_by is declared but @corrigible is absent — \
                             telos is immutable without the corrigibility annotation; \
                             add @corrigible or remove modifiable_by:",
                            being.name
                        ),
                        being.span.clone(),
                    ));
                }
            }
        }
    }
}
