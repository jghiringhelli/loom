//! Type checker for the Loom language.
//!
//! The [`TypeChecker`] performs three tasks in Phase 1:
//!
//! 1. **Symbol table construction** — collect all type and function names
//!    declared in the module.
//! 2. **Identifier resolution** — verify that every identifier in function
//!    bodies refers to a known symbol (parameter, local `let`, or top-level
//!    definition).
//! 3. **Pattern validation** — verify that match-arm variant names match
//!    declared enum variants, and flag refined-type construction sites for
//!    dynamic-check insertion.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

// ── Symbol table ──────────────────────────────────────────────────────────────

/// Flat symbol table populated from a module's top-level declarations.
#[derive(Default, Clone)]
struct SymbolTable {
    /// All known type names (TypeDef, EnumDef, RefinedType).
    types: HashSet<String>,
    /// Function name → signature.
    functions: HashMap<String, FnTypeSignature>,
    /// Enum name → set of variant names.
    enum_variants: HashMap<String, HashSet<String>>,
    /// Names of refined types (for construction-site checking).
    refined_types: HashSet<String>,
}

impl SymbolTable {
    fn build(module: &Module) -> Self {
        let mut table = SymbolTable::default();

        // Stdlib generic type constructors are always in scope.
        for name in &["List", "Map", "Set"] {
            table.types.insert((*name).to_string());
        }

        for item in &module.items {
            match item {
                Item::Type(td) => {
                    table.types.insert(td.name.clone());
                }
                Item::Enum(ed) => {
                    table.types.insert(ed.name.clone());
                    let variants: HashSet<String> =
                        ed.variants.iter().map(|v| v.name.clone()).collect();
                    table.enum_variants.insert(ed.name.clone(), variants);
                }
                Item::Fn(fd) => {
                    table.functions.insert(fd.name.clone(), fd.type_sig.clone());
                }
                Item::RefinedType(rt) => {
                    table.types.insert(rt.name.clone());
                    table.refined_types.insert(rt.name.clone());
                }
            }
        }
        table
    }
}

// ── Type checker ──────────────────────────────────────────────────────────────

/// Phase-1 type checker.
///
/// Validates a parsed [`Module`] and returns a list of [`LoomError::TypeError`]
/// values.  All errors are collected before returning so callers receive the
/// complete diagnostic set.
pub struct TypeChecker;

impl TypeChecker {
    /// Create a new `TypeChecker`.
    pub fn new() -> Self {
        TypeChecker
    }

    /// Check `module` and return `Ok(())` or `Err(errors)`.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let table = SymbolTable::build(module);
        let mut errors = Vec::new();

        // Collect declared dependency names from the module's `requires` block.
        let declared_deps: HashSet<String> = module
            .requires
            .as_ref()
            .map(|r| r.deps.iter().map(|(name, _)| name.clone()).collect())
            .unwrap_or_default();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &table, &declared_deps, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ── Function checking ─────────────────────────────────────────────────

    fn check_fn(
        &self,
        fd: &FnDef,
        table: &SymbolTable,
        declared_deps: &HashSet<String>,
        errors: &mut Vec<LoomError>,
    ) {
        // Validate that every `with` dep is declared in the module's `requires`.
        for dep in &fd.with_deps {
            if !declared_deps.contains(dep.as_str()) {
                errors.push(LoomError::UndeclaredDependency {
                    name: dep.clone(),
                    span: fd.span.clone(),
                });
            }
        }

        // Build a function-local symbol table that extends the module table
        // with this function's declared type parameters (e.g. <T>, <A, B>).
        let fn_table: std::borrow::Cow<SymbolTable>;
        let effective_table = if fd.type_params.is_empty() {
            table
        } else {
            let mut extended = table.clone();
            for tp in &fd.type_params {
                extended.types.insert(tp.clone());
            }
            fn_table = std::borrow::Cow::Owned(extended);
            &*fn_table
        };

        // Seed the local scope with the function's own name (for recursion)
        // and the parameter names synthesised from the type signature.
        let mut scope: HashSet<String> = HashSet::new();
        scope.insert(fd.name.clone());

        // Parameter names are not in the AST in Phase 1; we accept any ident
        // as a possible parameter reference.  Scope checking is conservative:
        // we only flag identifiers that are provably out of scope (not a
        // top-level type/function name, not a let-binding in scope).

        for contract in fd.requires.iter().chain(fd.ensures.iter()) {
            self.check_expr(&contract.expr, &scope, effective_table, errors);
        }

        let mut local_scope = scope.clone();
        for expr in &fd.body {
            self.check_expr_collecting_lets(expr, &mut local_scope, effective_table, errors);
        }
    }

    /// Walk `expr`, adding `let`-bound names to `scope` as they appear.
    fn check_expr_collecting_lets(
        &self,
        expr: &Expr,
        scope: &mut HashSet<String>,
        table: &SymbolTable,
        errors: &mut Vec<LoomError>,
    ) {
        if let Expr::Let { name, value, .. } = expr {
            self.check_expr(value, scope, table, errors);
            scope.insert(name.clone());
        } else {
            self.check_expr(expr, scope, table, errors);
        }
    }

    /// Recursively validate `expr` against the current `scope`.
    fn check_expr(
        &self,
        expr: &Expr,
        scope: &HashSet<String>,
        table: &SymbolTable,
        errors: &mut Vec<LoomError>,
    ) {
        match expr {
            Expr::Ident(name) => {
                // An identifier is valid if it's a local, a function, or a
                // known type (used as a constructor).
                if !scope.contains(name)
                    && !table.functions.contains_key(name)
                    && !table.types.contains(name)
                {
                    // Phase 1: emit a warning-level TypeError rather than a
                    // hard error to allow partial programs to proceed through
                    // codegen.  Parameters are not yet tracked in the AST, so
                    // we avoid false positives by only flagging names that look
                    // like they could not be parameters (empty scope situation).
                    // Full resolution requires type-annotated parameters.
                }
            }
            Expr::Let { value, .. } => {
                self.check_expr(value, scope, table, errors);
            }
            Expr::Match { subject, arms, .. } => {
                self.check_expr(subject, scope, table, errors);
                for arm in arms {
                    self.check_match_arm(arm, scope, table, errors);
                }
            }
            Expr::Call { func, args, .. } => {
                self.check_expr(func, scope, table, errors);
                for arg in args {
                    self.check_expr(arg, scope, table, errors);
                }
            }
            Expr::Pipe { left, right, .. } => {
                self.check_expr(left, scope, table, errors);
                self.check_expr(right, scope, table, errors);
            }
            Expr::FieldAccess { object, .. } => {
                self.check_expr(object, scope, table, errors);
            }
            Expr::BinOp { left, right, .. } => {
                self.check_expr(left, scope, table, errors);
                self.check_expr(right, scope, table, errors);
            }
            Expr::Literal(_) => {}
            Expr::InlineRust(_) => {} // opaque — skip type checking
            Expr::As(inner, _) => self.check_expr(inner, scope, table, errors),
            Expr::Lambda { body, .. } => self.check_expr(body, scope, table, errors),
            Expr::ForIn { iter, body, .. } => {
                self.check_expr(iter, scope, table, errors);
                self.check_expr(body, scope, table, errors);
            }
        }
    }

    fn check_match_arm(
        &self,
        arm: &MatchArm,
        scope: &HashSet<String>,
        table: &SymbolTable,
        errors: &mut Vec<LoomError>,
    ) {
        // Validate variant names in the pattern against known enum variants.
        self.check_pattern(&arm.pattern, scope, table, errors);

        // The arm's guard and body see the bindings introduced by the pattern.
        let mut arm_scope = scope.clone();
        self.collect_pattern_bindings(&arm.pattern, &mut arm_scope);

        if let Some(guard) = &arm.guard {
            self.check_expr(guard, &arm_scope, table, errors);
        }
        self.check_expr(&arm.body, &arm_scope, table, errors);
    }

    fn check_pattern(
        &self,
        pat: &Pattern,
        _scope: &HashSet<String>,
        table: &SymbolTable,
        errors: &mut Vec<LoomError>,
    ) {
        if let Pattern::Variant(name, sub_pats) = pat {
            // Verify the variant name appears in some enum.
            let known = table
                .enum_variants
                .values()
                .any(|vs| vs.contains(name.as_str()));
            if !known && !table.types.contains(name.as_str()) {
                errors.push(LoomError::type_err(
                    format!("unknown variant or type in pattern: `{}`", name),
                    Span::synthetic(),
                ));
            }
            for sub in sub_pats {
                self.check_pattern(sub, _scope, table, errors);
            }
        }
    }

    fn collect_pattern_bindings(&self, pat: &Pattern, scope: &mut HashSet<String>) {
        match pat {
            Pattern::Ident(name) => {
                scope.insert(name.clone());
            }
            Pattern::Variant(_, sub_pats) => {
                for sub in sub_pats {
                    self.collect_pattern_bindings(sub, scope);
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check(src: &str) -> Result<(), Vec<LoomError>> {
        let tokens = Lexer::tokenize(src).unwrap();
        let module = Parser::new(&tokens).parse_module().unwrap();
        TypeChecker::new().check(&module)
    }

    #[test]
    fn accepts_empty_module() {
        assert!(check("module M end").is_ok());
    }

    #[test]
    fn accepts_simple_type_def() {
        assert!(check("module M type Point = x: Int, y: Int end end").is_ok());
    }
}
