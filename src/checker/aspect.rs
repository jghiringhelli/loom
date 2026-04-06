//! Stage 2b: Aspect Checker (M66 AOP + M66b Annotation Algebra + M67 Correctness Report)
//!
//! Verifies:
//! 1. All advice function names in `aspect` blocks refer to functions declared in the module.
//! 2. `order:` values are unique across all aspects in the same module.
//! 3. `@requires_aspect(X)` annotations on functions have a corresponding aspect `X` in scope.
//! 4. Annotation declarations (M66b) are well-formed (no duplicate param names).
//! 5. Correctness reports (M67) are syntactically coherent.

use crate::ast::{Item, Module, PointcutExpr, Span};
use crate::error::LoomError;
use std::collections::HashSet;

/// Checker for AOP aspects, annotation algebra, and correctness reports.
pub struct AspectChecker;

impl AspectChecker {
    /// Create a new [`AspectChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Run all aspect-oriented checks over the compiled module.
    ///
    /// # Errors
    /// Returns a list of [`LoomError`] if any check fails.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors: Vec<LoomError> = Vec::new();

        // Build the set of function names declared in this module (for advice validation).
        let declared_fns: HashSet<&str> = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();

        // Check aspects.
        self.check_aspects(module, &declared_fns, &mut errors);

        // Check annotation declarations.
        self.check_annotation_decls(module, &mut errors);

        // Check correctness reports.
        self.check_correctness_reports(module, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate all `aspect` blocks in the module.
    fn check_aspects(
        &self,
        module: &Module,
        declared_fns: &HashSet<&str>,
        errors: &mut Vec<LoomError>,
    ) {
        let mut seen_orders: HashSet<u32> = HashSet::new();

        // Collect all aspect names for @requires_aspect validation.
        let aspect_names: HashSet<&str> = module
            .aspect_defs
            .iter()
            .map(|a| a.name.as_str())
            .collect();

        for aspect in &module.aspect_defs {
            // Validate order uniqueness.
            if let Some(ord) = aspect.order {
                if !seen_orders.insert(ord) {
                    errors.push(LoomError::type_err(
                        format!(
                            "duplicate aspect order `{}` — each aspect must have a unique order value",
                            ord
                        ),
                        aspect.span.clone(),
                    ));
                }
            }

            // Validate all advice functions exist.
            for advice_fn in aspect
                .before
                .iter()
                .chain(&aspect.after)
                .chain(&aspect.after_throwing)
                .chain(&aspect.around)
            {
                if !declared_fns.contains(advice_fn.as_str()) {
                    errors.push(LoomError::type_err(
                        format!(
                            "aspect `{}`: advice function `{}` is not declared in this module",
                            aspect.name, advice_fn
                        ),
                        aspect.span.clone(),
                    ));
                }
            }

            if let Some(on_fail) = &aspect.on_failure {
                if !declared_fns.contains(on_fail.as_str()) {
                    errors.push(LoomError::type_err(
                        format!(
                            "aspect `{}`: on_failure handler `{}` is not declared in this module",
                            aspect.name, on_fail
                        ),
                        aspect.span.clone(),
                    ));
                }
            }

            // Validate pointcut references if present.
            if let Some(pointcut) = &aspect.pointcut {
                self.check_pointcut(pointcut, &aspect.name, errors);
            }
        }

        // Check @requires_aspect annotations on functions.
        // Annotation has `key` and `value` — @requires_aspect("X") → key="requires_aspect", value="X".
        for item in &module.items {
            if let Item::Fn(fn_def) = item {
                for ann in &fn_def.annotations {
                    if ann.key == "requires_aspect" && !ann.value.is_empty() {
                        let required = ann.value.trim_matches('"');
                        if !aspect_names.contains(required) {
                            errors.push(LoomError::type_err(
                                format!(
                                    "function `{}` requires aspect `{}` but no such aspect is declared in this module",
                                    fn_def.name, required
                                ),
                                fn_def.span.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Recursively validate a pointcut expression.
    fn check_pointcut(
        &self,
        pointcut: &PointcutExpr,
        _aspect_name: &str,
        errors: &mut Vec<LoomError>,
    ) {
        match pointcut {
            PointcutExpr::HasAnnotation(_) | PointcutExpr::EffectIncludes(_) => {
                // Leaf nodes are always valid syntactically.
            }
            PointcutExpr::And(left, right) | PointcutExpr::Or(left, right) => {
                self.check_pointcut(left, _aspect_name, errors);
                self.check_pointcut(right, _aspect_name, errors);
            }
        }
    }

    /// Validate annotation declarations (M66b).
    fn check_annotation_decls(&self, module: &Module, errors: &mut Vec<LoomError>) {
        for item in &module.items {
            if let Item::AnnotationDecl(decl) = item {
                // Check for duplicate parameter names.
                let mut param_names: HashSet<&str> = HashSet::new();
                for (param_name, _) in &decl.params {
                    if !param_names.insert(param_name.as_str()) {
                        errors.push(LoomError::type_err(
                            format!(
                                "annotation `{}`: duplicate parameter name `{}`",
                                decl.name, param_name
                            ),
                            decl.span.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Validate correctness reports (M67).
    fn check_correctness_reports(&self, module: &Module, errors: &mut Vec<LoomError>) {
        // Count reports — at most one per module is allowed.
        let report_count = module
            .items
            .iter()
            .filter(|i| matches!(i, Item::CorrectnessReport(_)))
            .count();

        if report_count > 1 {
            errors.push(LoomError::type_err(
                format!(
                    "module `{}` has {} correctness_report blocks; at most one is allowed",
                    module.name, report_count
                ),
                Span::synthetic(),
            ));
        }

        for item in &module.items {
            if let Item::CorrectnessReport(report) = item {
                // Check for duplicate proved claims.
                let mut seen: HashSet<&str> = HashSet::new();
                for claim in &report.proved {
                    if !seen.insert(claim.property.as_str()) {
                        errors.push(LoomError::type_err(
                            format!("duplicate proved claim `{}`", claim.property),
                            claim.span.clone(),
                        ));
                    }
                }
            }
        }
    }
}

