//! Probabilistic types checker for the Loom compiler. M60/M84.
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
            self.check_distribution_family(&dist.family, dist.convergence.as_deref(), errors, &dist.span);
        }
    }

    /// Validate distribution family parameter ranges and convergence claims.
    fn check_distribution_family(
        &self,
        family: &DistributionFamily,
        convergence: Option<&str>,
        errors: &mut Vec<LoomError>,
        span: &Span,
    ) {
        match family {
            DistributionFamily::Cauchy { .. } | DistributionFamily::Levy { .. } => {
                if let Some(conv) = convergence {
                    let conv_lower = conv.to_lowercase();
                    if conv_lower.contains("central_limit")
                        || conv_lower.contains("law_of_large")
                        || conv_lower.contains("clt")
                    {
                        errors.push(LoomError::TypeError {
                            msg: format!(
                                "distribution: Cauchy/Lévy distributions have no defined mean or variance \
                                 — the central limit theorem and law of large numbers do not apply. \
                                 Remove the convergence claim or use a finite-variance distribution."
                            ),
                            span: span.clone(),
                        });
                    }
                }
            }
            DistributionFamily::Beta { alpha, beta } => {
                if let Ok(a) = alpha.parse::<f64>() {
                    if a <= 0.0 {
                        errors.push(LoomError::TypeError {
                            msg: format!("distribution: Beta alpha parameter must be > 0, got {}", a),
                            span: span.clone(),
                        });
                    }
                }
                if let Ok(b) = beta.parse::<f64>() {
                    if b <= 0.0 {
                        errors.push(LoomError::TypeError {
                            msg: format!("distribution: Beta beta parameter must be > 0, got {}", b),
                            span: span.clone(),
                        });
                    }
                }
            }
            DistributionFamily::Binomial { p, .. } => {
                if let Ok(prob) = p.parse::<f64>() {
                    if prob < 0.0 || prob > 1.0 {
                        errors.push(LoomError::TypeError {
                            msg: format!("distribution: Binomial p must be in [0, 1], got {}", prob),
                            span: span.clone(),
                        });
                    }
                }
            }
            DistributionFamily::Gaussian { std_dev, .. } => {
                if let Ok(s) = std_dev.parse::<f64>() {
                    if s <= 0.0 {
                        errors.push(LoomError::TypeError {
                            msg: format!("distribution: Gaussian std_dev must be > 0, got {}", s),
                            span: span.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

