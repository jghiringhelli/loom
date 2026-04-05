//! Information flow checker.
//!
//! Prevents @secret data from flowing to @public outputs without
//! explicit declassification. Prevents @tainted data from flowing
//! to database/system calls without sanitization.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::LoomError;

/// Information flow checker.
///
/// Inspects each function's parameter and return types against the module-level
/// `flow` declarations and reports violations where sensitive data would leak.
pub struct InfoFlowChecker;

impl InfoFlowChecker {
    pub fn new() -> Self {
        InfoFlowChecker
    }

    /// Check `module` for information flow violations.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let label_map = build_label_map(module);
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &label_map, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_fn(
        &self,
        fd: &FnDef,
        label_map: &HashMap<String, String>,
        errors: &mut Vec<LoomError>,
    ) {
        let fn_name_lower = fd.name.to_lowercase();

        // Determine param labels from type names.
        for param_ty in &fd.type_sig.params {
            let param_type_name = extract_base_name(param_ty);
            let param_label = param_type_name
                .as_deref()
                .and_then(|n| label_map.get(n))
                .map(String::as_str);

            // Check: secret param flowing to public return.
            if param_label == Some("secret") {
                let return_label = extract_base_name(&fd.type_sig.return_type)
                    .as_deref()
                    .and_then(|n| label_map.get(n))
                    .map(String::as_str);

                let return_is_public = matches!(return_label, Some("public") | None);

                if return_is_public
                    && !is_declassification_fn(&fn_name_lower)
                {
                    let param_name = param_type_name
                        .as_deref()
                        .unwrap_or("unknown")
                        .to_string();
                    let return_name = extract_base_name_str(&fd.type_sig.return_type);
                    errors.push(LoomError::type_err(
                        format!(
                            "information flow violation: function '{}' takes @secret '{}' but returns @public '{}' \
                             — use declassify() or rename to indicate intentional exposure",
                            fd.name, param_name, return_name
                        ),
                        fd.span.clone(),
                    ));
                }
            }

            // Check: tainted param flowing to DB operation.
            if param_label == Some("tainted") && is_db_operation(&fn_name_lower) {
                let param_name = param_type_name
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_string();
                errors.push(LoomError::type_err(
                    format!(
                        "information flow violation: @tainted input '{}' flows to DB operation '{}' \
                         — sanitize input first",
                        param_name, fd.name
                    ),
                    fd.span.clone(),
                ));
            }
        }
    }
}

/// Build a map from type name → security label from the module's `flow_labels`.
fn build_label_map(module: &Module) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for fl in &module.flow_labels {
        for type_name in &fl.types {
            map.insert(type_name.clone(), fl.label.clone());
        }
    }
    map
}

/// Extract the base type name from a TypeExpr, if it is a Base or Effect(_, Base) type.
fn extract_base_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Base(name) => Some(name.clone()),
        TypeExpr::Effect(_, inner) => extract_base_name(inner),
        TypeExpr::Generic(name, _) => Some(name.clone()),
        TypeExpr::Option(inner) => extract_base_name(inner),
        TypeExpr::Result(ok, _) => extract_base_name(ok),
        _ => None,
    }
}

/// Like extract_base_name but returns a display string even for unknown types.
fn extract_base_name_str(ty: &TypeExpr) -> String {
    extract_base_name(ty).unwrap_or_else(|| "unknown".to_string())
}

/// Returns `true` if the function name suggests it is an intentional
/// declassification/transformation (hashing, encrypting, masking, etc.).
fn is_declassification_fn(name_lower: &str) -> bool {
    name_lower.contains("hash")
        || name_lower.contains("encrypt")
        || name_lower.contains("mask")
        || name_lower.contains("sanitize")
        || name_lower.contains("declassify")
        || name_lower.contains("redact")
        || name_lower.contains("anonymize")
}

/// Returns `true` if the function name suggests it performs a DB/query operation.
fn is_db_operation(name_lower: &str) -> bool {
    name_lower.contains("query")
        || name_lower.contains("find")
        || name_lower.contains("get")
        || name_lower.contains("fetch")
        || name_lower.contains("search")
        || name_lower.contains("insert")
        || name_lower.contains("update")
        || name_lower.contains("delete")
        || name_lower.contains("select")
}
