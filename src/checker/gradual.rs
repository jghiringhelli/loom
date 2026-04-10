//! Gradual typing checker for the Loom compiler. M59.
use crate::ast::*;
use crate::error::LoomError;

pub struct GradualChecker;

impl GradualChecker {
    pub fn new() -> Self {
        GradualChecker
    }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut errors);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn uses_dynamic_type(ty: &TypeExpr) -> bool {
        match ty {
            TypeExpr::Dynamic => true,
            TypeExpr::Generic(_, params) => params.iter().any(Self::uses_dynamic_type),
            TypeExpr::Effect(_, inner) => Self::uses_dynamic_type(inner),
            TypeExpr::Option(inner) => Self::uses_dynamic_type(inner),
            TypeExpr::Result(ok, err) => {
                Self::uses_dynamic_type(ok) || Self::uses_dynamic_type(err)
            }
            TypeExpr::Tuple(elems) => elems.iter().any(Self::uses_dynamic_type),
            _ => false,
        }
    }

    fn fn_uses_dynamic(fd: &FnDef) -> bool {
        fd.type_sig.params.iter().any(Self::uses_dynamic_type)
            || Self::uses_dynamic_type(&fd.type_sig.return_type)
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        if Self::fn_uses_dynamic(fd) && fd.gradual.is_none() {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "gradual typing: function `{}` uses `?` (dynamic) type without a `gradual:` block; \
                     add a gradual: block to document boundary behaviour",
                    fd.name
                ),
                span: fd.span.clone(),
            });
        }
        if let Some(gradual) = &fd.gradual {
            if gradual.on_cast_failure.is_some() && gradual.blame.is_none() {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "gradual typing: function `{}` declares `on_cast_failure:` but is missing `blame:` \
                         — blame tracking is required when a cast failure handler is declared",
                        fd.name
                    ),
                    span: gradual.span.clone(),
                });
            }
        }
    }
}
