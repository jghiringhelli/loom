//! Category theory checker for the Loom compiler. M63.
use crate::ast::*;
use crate::error::LoomError;

pub struct CategoryChecker;

impl CategoryChecker {
    pub fn new() -> Self {
        CategoryChecker
    }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            match item {
                Item::Functor(f) => self.check_functor(f, &mut errors),
                Item::Monad(m) => self.check_monad(m, &mut errors),
                _ => {}
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_functor(&self, f: &FunctorDef, errors: &mut Vec<LoomError>) {
        if f.laws.len() < 2 {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "category: functor `{}` must declare at least 2 laws (identity, composition), \
                     but declares {}",
                    f.name,
                    f.laws.len()
                ),
                span: f.span.clone(),
            });
        }
    }

    fn check_monad(&self, m: &MonadDef, errors: &mut Vec<LoomError>) {
        if m.laws.len() < 3 {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "category: monad `{}` must declare 3 laws (left_identity, right_identity, associativity), \
                     but declares {}",
                    m.name, m.laws.len()
                ),
                span: m.span.clone(),
            });
        }
        let required = ["left_identity", "right_identity", "associativity"];
        for req in &required {
            if !m.laws.iter().any(|l| l.name == *req) {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "category: monad `{}` is missing required law `{}`",
                        m.name, req
                    ),
                    span: m.span.clone(),
                });
            }
        }
    }
}
