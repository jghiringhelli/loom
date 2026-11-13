//! M109: Property-based test checker.
//!
//! Validates `property:` blocks in the Loom pipeline.
//!
//! Rules:
//! 1. `samples: 0` → compile error (zero samples is meaningless).
//! 2. `invariant:` that does not reference `var_name` → warning.

use crate::ast::*;
use crate::error::LoomError;

/// M109: Property-based test checker.
///
/// Validates structural correctness of `property:` blocks.
/// QuickCheck (Claessen & Hughes 2000) lineage.
pub struct PropertyChecker;

impl PropertyChecker {
    /// Create a new property checker.
    pub fn new() -> Self {
        PropertyChecker
    }

    /// Check all `property:` blocks in `module`.
    ///
    /// Returns accumulated errors — the pipeline receives all errors at once.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Property(pb) = item {
                self.check_property(pb, &mut errors);
            }
        }
        errors
    }

    fn check_property(&self, pb: &PropertyBlock, errors: &mut Vec<LoomError>) {
        // Rule 1: samples: 0 is a hard error — zero samples produces no evidence.
        if pb.samples == 0 {
            errors.push(LoomError::parse(
                format!("property '{}': samples must be > 0, got 0", pb.name),
                pb.span.clone(),
            ));
        }

        // Rule 2: invariant must reference the quantified variable.
        if !pb.invariant.contains(&pb.var_name) {
            errors.push(LoomError::parse(
                format!(
                    "[warn] property '{}': invariant expression does not reference '{}' — \
                     the quantified variable appears unused",
                    pb.name, pb.var_name
                ),
                pb.span.clone(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_module() -> Module {
        let tokens = crate::lexer::Lexer::tokenize("module T\nend").unwrap();
        crate::parser::Parser::new(&tokens).parse_module().unwrap()
    }

    fn block(name: &str, var: &str, inv: &str, samples: u64) -> Module {
        let mut m = empty_module();
        m.items.push(Item::Property(PropertyBlock {
            name: name.into(),
            var_name: var.into(),
            var_type: "Int".into(),
            invariant: inv.into(),
            shrink: true,
            samples,
            span: Span::new(0, 0),
        }));
        m
    }

    fn msgs(errs: &[LoomError]) -> Vec<String> {
        errs.iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn invariant_referencing_var_is_clean() {
        let m = block("p", "x", "x = x", 10);
        assert!(PropertyChecker::new().check(&m).is_empty());
    }

    #[test]
    fn invariant_not_referencing_var_warns_exactly_once() {
        let m = block("p", "x", "1 = 1", 10);
        let errs = PropertyChecker::new().check(&m);
        assert_eq!(errs.len(), 1);
        let s = msgs(&errs).remove(0);
        assert!(s.contains("[warn] property 'p'"), "{s}");
        assert!(s.contains("does not reference 'x'"), "{s}");
    }

    #[test]
    fn zero_samples_is_error_with_message() {
        let m = block("p", "x", "x = x", 0);
        let errs = PropertyChecker::new().check(&m);
        assert_eq!(errs.len(), 1);
        assert!(msgs(&errs)[0].contains("property 'p': samples must be > 0, got 0"));
    }

    #[test]
    fn zero_samples_and_unused_var_yield_both_diagnostics() {
        let m = block("p", "x", "1 = 1", 0);
        assert_eq!(PropertyChecker::new().check(&m).len(), 2);
    }
}
