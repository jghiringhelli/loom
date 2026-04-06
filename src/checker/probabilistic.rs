//! Probabilistic types checker for the Loom compiler. M60.
use crate::ast::*;
use crate::error::LoomError;

pub struct ProbabilisticChecker;

impl ProbabilisticChecker {
    pub fn new() -> Self { ProbabilisticChecker }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut errors);
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    fn has_annotation(fd: &FnDef, key: &str) -> bool {
        fd.annotations.iter().any(|a| a.key == key)
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        if Self::has_annotation(fd, "probabilistic") && fd.distribution.is_none() {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "probabilistic: function `{}` is annotated `@probabilistic` but lacks a `distribution:` block",
                    fd.name
                ),
                span: fd.span.clone(),
            });
        }
        if let Some(dist) = &fd.distribution {
            if dist.convergence.is_some() {
                let ret = fd.type_sig.return_type.as_ref();
                let is_numeric = matches!(ret, TypeExpr::Base(n) if matches!(n.as_str(), "Int" | "Float" | "f64" | "f32" | "i64" | "i32"));
                if !is_numeric {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "probabilistic: function `{}` declares `convergence:` but its return type is not numeric \
                             — convergence guarantees require a numeric return type",
                            fd.name
                        ),
                        span: dist.span.clone(),
                    });
                }
            }
        }
    }
}
