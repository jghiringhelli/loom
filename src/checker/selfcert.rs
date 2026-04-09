//! Self-certifying compilation checker for the Loom compiler. M65.
use crate::ast::*;
use crate::error::LoomError;

pub struct SelfCertChecker;

impl SelfCertChecker {
    pub fn new() -> Self {
        SelfCertChecker
    }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Certificate(cert) = item {
                self.check_certificate(cert, &mut errors);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_certificate(&self, cert: &CertificateDef, errors: &mut Vec<LoomError>) {
        let valid_values = [
            "proven", "verified", "partial", "assumed", "checked", "passed",
        ];
        let known_fields = [
            "type_safety",
            "memory_safety",
            "termination",
            "purity",
            "effect_safety",
            "privacy",
            "timing_safety",
            "separation",
        ];
        for field in &cert.fields {
            if field.name.is_empty() || field.value.is_empty() {
                errors.push(LoomError::TypeError {
                    msg: format!("self-cert: certificate field has empty name or value"),
                    span: field.span.clone(),
                });
                continue;
            }
            if known_fields.contains(&field.name.as_str())
                && !valid_values.contains(&field.value.as_str())
            {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "self-cert: certificate field `{}` has unknown value `{}`; \
                         valid values are: proven, verified, partial, assumed, checked, passed",
                        field.name, field.value
                    ),
                    span: field.span.clone(),
                });
            }
        }
    }
}
