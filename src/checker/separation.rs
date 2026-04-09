//! Separation logic checker for the Loom compiler.
//!
//! Validates separation logic blocks declared with `separation:` inside function
//! definitions.  Based on O'Hearn, Reynolds, and Yang (2001–2002): *Local
//! Reasoning about Programs that Alter Data Structures*.
//!
//! ## Checks performed
//!
//! 1. **Disjointness requires ownership** — every resource appearing in a
//!    `disjoint: A * B` clause must have been declared by an `owns: A` and
//!    `owns: B` clause in the same block.  You cannot prove disjointness for a
//!    resource you have not claimed to own.
//!
//! 2. **Frame requires ownership** — every resource in a `frame: X` clause must
//!    appear in an `owns: X` clause.  The frame rule only applies to resources
//!    whose footprint is known.
//!
//! These checks are conservative structural checks.  Full heap-level separation
//! verification requires an SMT solver integration (deferred to the `smt` feature).

use std::collections::HashSet;

use crate::ast::*;
use crate::error::LoomError;

/// Separation logic checker.
///
/// Validates structural consistency of `separation:` blocks in functions.
pub struct SeparationChecker;

impl SeparationChecker {
    /// Create a new `SeparationChecker`.
    pub fn new() -> Self {
        SeparationChecker
    }

    /// Check all function separation blocks in the module.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fn_def) = item {
                if let Some(sep) = &fn_def.separation {
                    self.check_separation_block(fn_def, sep, &mut errors);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate one separation block for structural consistency.
    fn check_separation_block(
        &self,
        fn_def: &FnDef,
        sep: &SeparationBlock,
        errors: &mut Vec<LoomError>,
    ) {
        let owned: HashSet<&str> = sep.owns.iter().map(|s| s.as_str()).collect();

        self.check_disjoint_pairs(fn_def, sep, &owned, errors);
        self.check_frame_resources(fn_def, sep, &owned, errors);
    }

    /// Every resource in a `disjoint: A * B` clause must be declared in `owns:`.
    fn check_disjoint_pairs(
        &self,
        fn_def: &FnDef,
        sep: &SeparationBlock,
        owned: &HashSet<&str>,
        errors: &mut Vec<LoomError>,
    ) {
        for (left, right) in &sep.disjoint {
            for resource in [left.as_str(), right.as_str()] {
                if !owned.contains(resource) {
                    errors.push(LoomError::TypeError {
                        msg: format!(
                            "separation violation in `{}`: `disjoint` references `{}` \
                             but it is not declared in `owns:`",
                            fn_def.name, resource
                        ),
                        span: sep.span.clone(),
                    });
                }
            }
        }
    }

    /// Every resource in a `frame: X` clause must be declared in `owns:`.
    fn check_frame_resources(
        &self,
        fn_def: &FnDef,
        sep: &SeparationBlock,
        owned: &HashSet<&str>,
        errors: &mut Vec<LoomError>,
    ) {
        for resource in &sep.frame {
            if !owned.contains(resource.as_str()) {
                errors.push(LoomError::TypeError {
                    msg: format!(
                        "separation violation in `{}`: `frame` references `{}` \
                         but it is not declared in `owns:`",
                        fn_def.name, resource
                    ),
                    span: sep.span.clone(),
                });
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser;

    fn parse_module(src: &str) -> Module {
        let tokens = Lexer::tokenize(src).unwrap();
        parser::Parser::new(&tokens).parse_module().unwrap()
    }

    #[test]
    fn valid_separation_block_passes() {
        let module = parse_module(
            "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\nowns: b\ndisjoint: a * b\nend\nend\nend",
        );
        let checker = SeparationChecker::new();
        assert!(checker.check(&module).is_ok());
    }

    #[test]
    fn disjoint_undeclared_resource_fails() {
        let module = parse_module(
            "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\ndisjoint: a * b\nend\nend\nend",
        );
        let checker = SeparationChecker::new();
        let result = checker.check(&module);
        assert!(result.is_err());
        let msg = result.unwrap_err()[0].to_string();
        assert!(msg.contains("b"), "expected 'b' in error: {}", msg);
    }

    #[test]
    fn frame_undeclared_resource_fails() {
        let module = parse_module(
            "module T\nfn f :: Unit -> Unit\nseparation:\nowns: a\nframe: ghost\nend\nend\nend",
        );
        let checker = SeparationChecker::new();
        let result = checker.check(&module);
        assert!(result.is_err());
        let msg = result.unwrap_err()[0].to_string();
        assert!(msg.contains("ghost"), "expected 'ghost' in error: {}", msg);
    }
}
