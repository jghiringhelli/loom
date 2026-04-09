//! M88: Stochastic process type checker.
//!
//! Validates that stochastic process declarations are internally consistent and
//! that process/distribution family declarations are coherent.
//!
//! Academic grounding:
//! - Wiener (1923): Brownian motion as a rigorous stochastic process.
//! - Itô (1944): stochastic calculus; Itô's lemma relates GBM to log-normal distributions.
//! - Ornstein & Uhlenbeck (1930): mean-reverting process for physical systems.
//! - Markov (1906): memoryless processes with discrete state spaces.

use crate::ast::*;
use crate::error::LoomError;

pub struct StochasticChecker;

impl StochasticChecker {
    /// Check all function definitions in the module for stochastic process violations.
    pub fn check(module: &Module, errors: &mut Vec<LoomError>) {
        for item in &module.items {
            if let Item::Fn(fd) = item {
                Self::check_fn(fd, errors);
            }
        }
    }

    fn check_fn(fd: &FnDef, errors: &mut Vec<LoomError>) {
        let Some(proc_block) = &fd.stochastic_process else { return };

        match &proc_block.kind {
            StochasticKind::GeometricBrownian => {
                // Rule 1: GBM paths are always positive (log-normal distribution).
                if proc_block.always_positive == Some(false) {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "stochastic: `{}` declares process kind GeometricBrownian with \
                             always_positive: false. GBM paths follow a log-normal distribution \
                             and are always strictly positive. Itô's lemma: dS = μS dt + σS dW.",
                            fd.name
                        ),
                        span: proc_block.span.clone(),
                    });
                }
            }
            StochasticKind::PoissonProcess => {
                // Rule 3: Poisson process must be integer-valued.
                if proc_block.integer_valued == Some(false) {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "stochastic: `{}` declares PoissonProcess with integer_valued: false. \
                             A Poisson process is a count process — it takes only non-negative \
                             integer values. Poisson (1837): P(N(t)=k) = e^(-λt)(λt)^k/k!.",
                            fd.name
                        ),
                        span: proc_block.span.clone(),
                    });
                }
            }
            StochasticKind::MarkovChain => {
                // Rule 4: MarkovChain requires a non-empty states list.
                if proc_block.states.is_empty() {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "stochastic: `{}` declares MarkovChain but provides no states. \
                             A Markov chain requires an explicit finite state space. \
                             Markov (1906): P(Xn+1=s | X0..Xn) = P(Xn+1=s | Xn).",
                            fd.name
                        ),
                        span: proc_block.span.clone(),
                    });
                }
            }
            _ => {}
        }

        // Rule 5: process.kind must be coherent with distribution.family.
        if let Some(dist) = &fd.distribution {
            Self::check_process_distribution_coherence(fd, proc_block, dist, errors);
        }
    }

    /// Check that the process kind and distribution family are mathematically coherent.
    ///
    /// Key violation: GeometricBrownian process with Gaussian distribution —
    /// GBM paths follow a log-normal distribution, not Gaussian.
    /// Itô's lemma: the LOG-RETURN of a GBM is Gaussian, not the price itself.
    fn check_process_distribution_coherence(
        fd: &FnDef,
        proc_block: &StochasticProcessBlock,
        dist: &DistributionBlock,
        errors: &mut Vec<LoomError>,
    ) {
        if matches!(proc_block.kind, StochasticKind::GeometricBrownian) {
            if matches!(dist.family, DistributionFamily::Gaussian { .. }) {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "stochastic: `{}` declares process kind GeometricBrownian but \
                         distribution family Gaussian. GeometricBrownian paths follow \
                         log-normal distributions, not Gaussian. \
                         Use family: GeometricBrownian(drift: ..., volatility: ...) or \
                         Itô's lemma applies: the log-return is Gaussian, not the price.",
                        fd.name
                    ),
                    span: proc_block.span.clone(),
                });
            }
        }
    }
}
