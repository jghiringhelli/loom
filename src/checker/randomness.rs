//! M85: Randomness quality type checker.
//!
//! Validates that randomness sources are used correctly and that pseudo-random
//! generators are not used in security-sensitive contexts.
//!
//! Academic grounding:
//! - Shannon (1948): entropy as a formal measure of unpredictability.
//! - Blum-Blum-Shub (1986): formal definition of a cryptographically secure PRNG.
//! - NIST SP 800-90A (2012): deterministic random bit generator standards.

use crate::ast::*;
use crate::error::LoomError;

/// M85: Randomness quality classification.
///
/// Ordered from weakest to strongest guarantee.
#[derive(Debug, Clone, PartialEq)]
pub enum RandomnessQuality {
    /// `@true_random` — hardware entropy source (TRNG, /dev/random).
    True,
    /// `@crypto_random` — CSPRNG (ChaCha20, Fortuna). Cryptographically indistinguishable.
    Crypto,
    /// `@pseudo_random` — deterministic PRNG (LCG, Mersenne Twister). Reproducible.
    Pseudo,
    /// `@seeded(N)` — explicitly seeded PRNG with documented seed source.
    Seeded(String),
}

pub struct RandomnessChecker;

impl RandomnessChecker {
    /// Check all function definitions in the module for randomness annotation violations.
    pub fn check(module: &Module, errors: &mut Vec<LoomError>) {
        for item in &module.items {
            if let Item::Fn(fd) = item {
                Self::check_fn(fd, errors);
            }
        }
    }

    fn check_fn(fd: &FnDef, errors: &mut Vec<LoomError>) {
        let quality = Self::quality_of(fd);
        let is_auth = Self::has_annotation(fd, "requires_auth");
        let is_pii = Self::has_annotation(fd, "pii");

        if quality == Some(RandomnessQuality::Pseudo) && (is_auth || is_pii) {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "randomness: `{}` is @pseudo_random but used in a security context \
                     (@requires_auth or @pii). Use @crypto_random or @true_random for \
                     cryptographic applications. \
                     NIST SP 800-90A: pseudo-random generators are insufficient for key material.",
                    fd.name
                ),
                span: fd.span.clone(),
            });
        }
    }

    fn quality_of(fd: &FnDef) -> Option<RandomnessQuality> {
        for ann in &fd.annotations {
            match ann.key.as_str() {
                "true_random"   => return Some(RandomnessQuality::True),
                "crypto_random" => return Some(RandomnessQuality::Crypto),
                "pseudo_random" => return Some(RandomnessQuality::Pseudo),
                "seeded"        => return Some(RandomnessQuality::Seeded(ann.value.clone())),
                _ => {}
            }
        }
        None
    }

    fn has_annotation(fd: &FnDef, name: &str) -> bool {
        fd.annotations.iter().any(|a| a.key == name)
    }
}
