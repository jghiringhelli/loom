//! Exhaustiveness checker for `match` expressions.
//!
//! The [`ExhaustivenessChecker`] pass runs after the [`super::TypeChecker`]
//! and verifies that every `match` expression on a known sum type covers all
//! declared variants.
//!
//! ## Algorithm
//!
//! Because Phase 1 has no type inference, the checker cannot inspect the
//! *type* of the match scrutinee.  Instead it uses the *patterns* themselves:
//! if any arm carries a `Pattern::Variant` whose name belongs to a known enum,
//! the checker resolves the enum and ensures every variant is covered.
//!
//! An arm is considered a **total cover** when its pattern is `Wildcard` or
//! `Ident` (a variable binding) **and** it has no guard.  A guard makes the
//! arm conditional, so it never counts as covering anything unconditionally.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

// ── EnumRegistry ─────────────────────────────────────────────────────────────

/// Lightweight index built from a module's `EnumDef` items.
struct EnumRegistry {
    /// Enum name → ordered list of variant names.
    variants_of: HashMap<String, Vec<String>>,
    /// Variant name → enum name (for reverse lookup).
    enum_of_variant: HashMap<String, String>,
}

impl EnumRegistry {
    fn build(module: &Module) -> Self {
        let mut reg = EnumRegistry {
            variants_of: HashMap::new(),
            enum_of_variant: HashMap::new(),
        };

        // Pre-seed stdlib sum types so Option/Result patterns are checked.
        for (enum_name, vs) in &[
            ("Option", vec!["Some", "None"]),
            ("Result", vec!["Ok", "Err"]),
        ] {
            let names: Vec<String> = vs.iter().map(|s| s.to_string()).collect();
            for name in &names {
                reg.enum_of_variant.insert(name.clone(), enum_name.to_string());
            }
            reg.variants_of.insert(enum_name.to_string(), names);
        }

        for item in &module.items {
            if let Item::Enum(ed) = item {
                let names: Vec<String> = ed.variants.iter().map(|v| v.name.clone()).collect();
                for name in &names {
                    reg.enum_of_variant.insert(name.clone(), ed.name.clone());
                }
                reg.variants_of.insert(ed.name.clone(), names);
            }
        }
        reg
    }

    /// Return the enum name that owns `variant_name`, if any.
    fn enum_of(&self, variant_name: &str) -> Option<&str> {
        self.enum_of_variant.get(variant_name).map(String::as_str)
    }

    /// Return all variant names for `enum_name`, if known.
    fn variants(&self, enum_name: &str) -> Option<&[String]> {
        self.variants_of.get(enum_name).map(Vec::as_slice)
    }
}

// ── ExhaustivenessChecker ─────────────────────────────────────────────────────

/// Checks that every `match` on a known sum type covers all variants.
pub struct ExhaustivenessChecker;

impl ExhaustivenessChecker {
    /// Create a new `ExhaustivenessChecker`.
    pub fn new() -> Self {
        ExhaustivenessChecker
    }

    /// Check `module` and return `Ok(())` or `Err(errors)`.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let registry = EnumRegistry::build(module);
        let mut errors = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                for expr in &fd.body {
                    self.check_expr(expr, &registry, &mut errors);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ── Expression traversal ──────────────────────────────────────────────

    fn check_expr(&self, expr: &Expr, reg: &EnumRegistry, errors: &mut Vec<LoomError>) {
        match expr {
            Expr::Match { subject, arms, span } => {
                self.check_match(arms, span, reg, errors);
                self.check_expr(subject, reg, errors);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard, reg, errors);
                    }
                    self.check_expr(&arm.body, reg, errors);
                }
            }
            Expr::Let { value, .. } => self.check_expr(value, reg, errors),
            Expr::Call { func, args, .. } => {
                self.check_expr(func, reg, errors);
                for arg in args {
                    self.check_expr(arg, reg, errors);
                }
            }
            Expr::Pipe { left, right, .. } => {
                self.check_expr(left, reg, errors);
                self.check_expr(right, reg, errors);
            }
            Expr::BinOp { left, right, .. } => {
                self.check_expr(left, reg, errors);
                self.check_expr(right, reg, errors);
            }
            Expr::FieldAccess { object, .. } => self.check_expr(object, reg, errors),
            Expr::Literal(_) | Expr::Ident(_) => {}
            Expr::InlineRust(_) => {} // opaque — no match expressions to check
            Expr::As(inner, _) => self.check_expr(inner, reg, errors),
            Expr::Lambda { body, .. } => self.check_expr(body, reg, errors),
            Expr::ForIn { iter, body, .. } => {
                self.check_expr(iter, reg, errors);
                self.check_expr(body, reg, errors);
            }
            Expr::Tuple(elems, _) => {
                for e in elems { self.check_expr(e, reg, errors); }
            }
            Expr::Try(inner, _) => self.check_expr(inner, reg, errors),
        }
    }

    // ── Core exhaustiveness logic ─────────────────────────────────────────

    fn check_match(
        &self,
        arms: &[MatchArm],
        span: &Span,
        reg: &EnumRegistry,
        errors: &mut Vec<LoomError>,
    ) {
        // Determine the enum being matched by scanning for the first arm that
        // uses a known variant name.
        let enum_name = self.resolve_enum(arms, reg);
        let enum_name = match enum_name {
            Some(n) => n,
            None => return, // Not matching on a known enum — nothing to check.
        };

        let all_variants = match reg.variants(enum_name) {
            Some(v) => v,
            None => return,
        };

        // A guard-free Wildcard or Ident arm is a total cover — no further
        // variant analysis is needed.
        if arms.iter().any(|arm| arm.guard.is_none() && is_total_pattern(&arm.pattern)) {
            return;
        }

        // Collect the set of variants covered by guard-free arms.
        let covered: HashSet<&str> = arms
            .iter()
            .filter(|arm| arm.guard.is_none())
            .filter_map(|arm| top_level_variant_name(&arm.pattern))
            .collect();

        let mut missing: Vec<String> = all_variants
            .iter()
            .filter(|v| !covered.contains(v.as_str()))
            .cloned()
            .collect();

        if !missing.is_empty() {
            missing.sort(); // Deterministic ordering for error messages.
            errors.push(LoomError::NonExhaustiveMatch {
                missing,
                span: span.clone(),
            });
        }
    }

    /// Find the first arm that uses a known variant, return the owning enum name.
    fn resolve_enum<'reg>(
        &self,
        arms: &[MatchArm],
        reg: &'reg EnumRegistry,
    ) -> Option<&'reg str> {
        arms.iter().find_map(|arm| {
            top_level_variant_name(&arm.pattern)
                .and_then(|v| reg.enum_of(v))
        })
    }
}

// ── Pattern helpers ───────────────────────────────────────────────────────────

/// Returns the top-level variant name from a pattern, if it is a `Variant`.
fn top_level_variant_name(pat: &Pattern) -> Option<&str> {
    match pat {
        Pattern::Variant(name, _) => Some(name.as_str()),
        _ => None,
    }
}

/// Returns `true` if the pattern unconditionally covers any value.
///
/// Only `Wildcard` and `Ident` (variable binding) are total covers.
/// `Variant` patterns cover only their specific variant.
/// `Literal` patterns are never total.
fn is_total_pattern(pat: &Pattern) -> bool {
    matches!(pat, Pattern::Wildcard | Pattern::Ident(_))
}
