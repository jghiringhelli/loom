// ALX: derived from loom.loom §"check_privacy" and language-spec.md §9.2
// PCI/HIPAA co-occurrence rules:
// @pci fields must have @encrypt-at-rest AND @never-log
// @hipaa fields must have @encrypt-at-rest

use crate::ast::{Module, Item};
use crate::error::LoomError;

/// G4: PrivacyChecker struct — tests call `PrivacyChecker::new().check(&module)`.
pub struct PrivacyChecker;

impl PrivacyChecker {
    pub fn new() -> Self { PrivacyChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_privacy(module)
    }
}

pub fn check_privacy(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Check all TypeDef fields.
    for item in &module.items {
        if let Item::Type(t) = item {
            for field in &t.fields {
                check_field_annotations(&t.name, &field.name, &field.annotations, &mut errors, field.span);
            }
        }
    }

    // Check being matter fields.
    for being in &module.being_defs {
        if let Some(matter) = &being.matter {
            for field in &matter.fields {
                check_field_annotations(
                    &being.name, &field.name, &field.annotations, &mut errors, field.span,
                );
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_field_annotations(
    type_name: &str,
    field_name: &str,
    annotations: &[crate::ast::Annotation],
    errors: &mut Vec<LoomError>,
    span: crate::error::Span,
) {
    let keys: std::collections::HashSet<&str> = annotations.iter().map(|a| a.key.as_str()).collect();

    // @pci requires @encrypt-at-rest AND @never-log
    if keys.contains("pci") {
        if !keys.contains("encrypt-at-rest") {
            errors.push(LoomError::new(
                format!(
                    "{}::{}: @pci field must also be @encrypt-at-rest",
                    type_name, field_name
                ),
                span,
            ));
        }
        if !keys.contains("never-log") {
            errors.push(LoomError::new(
                format!(
                    "{}::{}: @pci field must also be @never-log",
                    type_name, field_name
                ),
                span,
            ));
        }
    }

    // @hipaa requires @encrypt-at-rest
    if keys.contains("hipaa") && !keys.contains("encrypt-at-rest") {
        errors.push(LoomError::new(
            format!(
                "{}::{}: @hipaa field must also be @encrypt-at-rest",
                type_name, field_name
            ),
            span,
        ));
    }
}
