//! Tensor type checker. M87.
//!
//! Validates tensor type expressions: rank consistency with shape,
//! unit compatibility, and contraction rules.
//!
//! Grounded in differential geometry, quantum mechanics (state vectors),
//! ML (weight matrices), and physics (stress/strain/metric tensors).

use crate::ast::*;
use crate::error::LoomError;

/// Validates `Tensor<rank, shape, unit>` type expressions throughout a module.
///
/// Rules enforced:
/// 1. `rank` must equal `shape.len()` for rank ≥ 1. Rank 0 requires an empty shape `[]`.
/// 2. Rank > 8 is flagged as unusual (likely a mistake).
/// 3. The `unit` type expression is validated by the type checker (runs separately).
pub struct TensorChecker;

impl TensorChecker {
    /// Create a new tensor checker.
    pub fn new() -> Self {
        TensorChecker
    }

    /// Validate all tensor type expressions in `module`.
    ///
    /// Returns accumulated errors for any rank/shape mismatches found.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            self.check_item(item, &mut errors);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_item(&self, item: &Item, errors: &mut Vec<LoomError>) {
        match item {
            Item::Type(td) => {
                for field in &td.fields {
                    self.check_type_expr(&field.ty, &field.span, errors);
                }
            }
            Item::Fn(fd) => {
                for param in &fd.type_sig.params {
                    self.check_type_expr(param, &fd.span, errors);
                }
                self.check_type_expr(&fd.type_sig.return_type, &fd.span, errors);
            }
            _ => {}
        }
    }

    fn check_type_expr(&self, ty: &TypeExpr, span: &Span, errors: &mut Vec<LoomError>) {
        match ty {
            TypeExpr::Tensor { rank, shape, unit, span: tensor_span } => {
                // Rule 1: rank must match shape length.
                if *rank != shape.len() {
                    errors.push(LoomError::type_err(
                        format!(
                            "tensor rank {} does not match shape length {} (shape: [{}])",
                            rank,
                            shape.len(),
                            shape.join(", ")
                        ),
                        tensor_span.clone(),
                    ));
                }
                // Rule 2: rank > 8 is unusual.
                if *rank > 8 {
                    errors.push(LoomError::type_err(
                        format!(
                            "tensor rank {} is unusually high (> 8) — verify this is intentional",
                            rank
                        ),
                        tensor_span.clone(),
                    ));
                }
                // Recurse into the unit type.
                self.check_type_expr(unit, tensor_span, errors);
            }
            TypeExpr::Generic(_, params) => {
                for p in params {
                    self.check_type_expr(p, span, errors);
                }
            }
            TypeExpr::Option(inner) | TypeExpr::Effect(_, inner) => {
                self.check_type_expr(inner, span, errors);
            }
            TypeExpr::Result(ok, err) => {
                self.check_type_expr(ok, span, errors);
                self.check_type_expr(err, span, errors);
            }
            TypeExpr::Tuple(elems) => {
                for e in elems {
                    self.check_type_expr(e, span, errors);
                }
            }
            TypeExpr::Base(_) | TypeExpr::Dynamic | TypeExpr::TypeVar(_) => {}
        }
    }
}
