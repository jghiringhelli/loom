//! M115: Signal attention checker.
//!
//! Validates `signal_attention:` blocks inside `being:` definitions.
//!
//! Rules:
//! - `prioritize_above` must be in [0.0, 1.0] if present.
//! - `attenuate_below` must be in [0.0, 1.0] if present.
//! - `prioritize_above` must be strictly greater than `attenuate_below` when
//!   both are declared — otherwise the attention window is semantically inverted
//!   (everything is simultaneously prioritised and attenuated).

use crate::ast::Module;
use crate::error::LoomError;

pub struct SignalAttentionChecker;

impl SignalAttentionChecker {
    /// Construct a new [`SignalAttentionChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `signal_attention:` blocks in the module.
    ///
    /// # Returns
    /// A vec of [`LoomError`] — empty on success.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for being in &module.being_defs {
            if let Some(sa) = &being.signal_attention {
                let in_range = |v: f64| (0.0..=1.0).contains(&v);

                if let Some(prio) = sa.prioritize_above {
                    if !in_range(prio) {
                        errors.push(LoomError::type_err(
                            format!(
                                "signal_attention in being '{}': prioritize_above {:.3} out of range [0.0, 1.0]",
                                being.name, prio
                            ),
                            being.span.clone(),
                        ));
                    }
                }
                if let Some(att) = sa.attenuate_below {
                    if !in_range(att) {
                        errors.push(LoomError::type_err(
                            format!(
                                "signal_attention in being '{}': attenuate_below {:.3} out of range [0.0, 1.0]",
                                being.name, att
                            ),
                            being.span.clone(),
                        ));
                    }
                }
                // Ordering invariant: prioritize_above > attenuate_below
                if let (Some(prio), Some(att)) = (sa.prioritize_above, sa.attenuate_below) {
                    if prio <= att {
                        errors.push(LoomError::type_err(
                            format!(
                                "signal_attention in being '{}': prioritize_above ({:.3}) must be \
                                 strictly greater than attenuate_below ({:.3}) — \
                                 inverted window means all signals are simultaneously boosted and suppressed",
                                being.name, prio, att
                            ),
                            being.span.clone(),
                        ));
                    }
                }
            }
        }
        errors
    }
}
