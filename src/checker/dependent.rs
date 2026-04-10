//! Dependent types checker for the Loom compiler. M61.
use crate::ast::*;
use crate::error::LoomError;

pub struct DependentChecker;

impl DependentChecker {
    pub fn new() -> Self {
        DependentChecker
    }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            match item {
                Item::Proposition(prop) => self.check_proposition(prop, &mut errors),
                Item::Fn(fd) => self.check_fn(fd, &mut errors),
                _ => {}
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_proposition(&self, prop: &PropositionDef, errors: &mut Vec<LoomError>) {
        match &prop.base_type {
            TypeExpr::Base(name) if name.is_empty() => {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "dependent: proposition `{}` has an empty base type",
                        prop.name
                    ),
                    span: prop.span.clone(),
                });
            }
            _ => {}
        }
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        if fd.termination.is_some() {
            let has_pure = fd.annotations.iter().any(|a| a.key == "pure");
            if !has_pure {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "dependent: function `{}` declares a `termination:` claim but is not annotated `@pure` \
                         — termination proofs are only valid for pure functions",
                        fd.name
                    ),
                    span: fd.span.clone(),
                });
            }
        }
    }
}
