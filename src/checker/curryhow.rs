//! Curry-Howard correspondence checker for the Loom compiler. M64.
use crate::ast::*;
use crate::error::LoomError;

pub struct CurryHowardChecker;

impl CurryHowardChecker {
    pub fn new() -> Self {
        CurryHowardChecker
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

    fn has_recursive_call(name: &str, exprs: &[Expr]) -> bool {
        exprs.iter().any(|e| Self::expr_has_recursive_call(name, e))
    }

    fn expr_has_recursive_call(name: &str, expr: &Expr) -> bool {
        match expr {
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(n) = func.as_ref() {
                    if n == name {
                        return true;
                    }
                }
                args.iter().any(|a| Self::expr_has_recursive_call(name, a))
            }
            Expr::Let { value, .. } => Self::expr_has_recursive_call(name, value),
            Expr::Pipe { left, right, .. } => {
                Self::expr_has_recursive_call(name, left)
                    || Self::expr_has_recursive_call(name, right)
            }
            Expr::Match { subject, arms, .. } => {
                Self::expr_has_recursive_call(name, subject)
                    || arms
                        .iter()
                        .any(|a| Self::expr_has_recursive_call(name, &a.body))
            }
            _ => false,
        }
    }

    fn has_match_expr(exprs: &[Expr]) -> bool {
        exprs.iter().any(|e| Self::expr_has_match(e))
    }

    fn expr_has_match(expr: &Expr) -> bool {
        matches!(expr, Expr::Match { .. })
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        for proof in &fd.proofs {
            match proof.strategy.as_str() {
                "structural_recursion" => {
                    if !Self::has_recursive_call(&fd.name, &fd.body) {
                        errors.push(LoomError::TypeError {
                            msg: format!(
                                "curry-howard: function `{}` declares `proof: structural_recursion` \
                                 but no recursive call to `{}` found in body",
                                fd.name, fd.name
                            ),
                            span: proof.span.clone(),
                        });
                    }
                }
                "totality" => {
                    if !Self::has_match_expr(&fd.body) {
                        errors.push(LoomError::TypeError {
                            msg: format!(
                                "curry-howard: function `{}` declares `proof: totality` \
                                 but no match expression found in body",
                                fd.name
                            ),
                            span: proof.span.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
}
