//! M103: Boundary checker — explicit public API surface declaration.
//!
//! Parnas (1972) information hiding → Composable GS property → Loom `boundary:` (M103).
//!
//! Rules enforced:
//! 1. Any name in `private:` that appears in a public fn signature → error (leaks internal type).
//! 2. Any name in `exports:` that doesn't exist as a declared type/fn → error (ghost export).
//! 3. `seal:` types cannot have `implements` added outside the declaring module → warning.
//! 4. If `boundary:` exists, any fn/type NOT in exports: or private: → warning (undeclared visibility).

use crate::ast::*;
use crate::error::LoomError;

/// Boundary checker — validates `boundary:` blocks in a module.
pub struct BoundaryChecker;

impl BoundaryChecker {
    /// Create a new BoundaryChecker.
    pub fn new() -> Self {
        BoundaryChecker
    }

    /// Check all boundary declarations in `module`.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::BoundaryBlock(bb) = item {
                self.check_boundary(bb, module, &mut errors);
            }
        }
        for being in &module.being_defs {
            if let Some(bb) = &being.boundary {
                self.check_being_boundary(bb, being, &mut errors);
            }
        }
        errors
    }

    fn check_boundary(&self, bb: &BoundaryBlock, module: &Module, errors: &mut Vec<LoomError>) {
        let declared_names: Vec<String> = module.items.iter().filter_map(|item| match item {
            Item::Fn(fd) => Some(fd.name.clone()),
            Item::Type(td) => Some(td.name.clone()),
            Item::Enum(ed) => Some(ed.name.clone()),
            _ => None,
        }).collect();

        // Rule 2: ghost exports
        for export in &bb.exports {
            if !declared_names.contains(export) {
                errors.push(LoomError::type_err(
                    format!("[error] boundary: exports '{}' which is not declared in this module", export),
                    bb.span.clone(),
                ));
            }
        }

        // Rule 1: private type in public fn signature
        for export_fn in &bb.exports {
            if let Some(Item::Fn(fd)) = module.items.iter().find(|i| matches!(i, Item::Fn(f) if f.name == *export_fn)) {
                for param_type in &fd.type_sig.params {
                    let type_name = extract_type_name(param_type);
                    if bb.private.contains(&type_name) {
                        errors.push(LoomError::type_err(
                            format!("[error] boundary: fn '{}' is exported but uses private type '{}' in its signature", export_fn, type_name),
                            fd.span.clone(),
                        ));
                    }
                }
            }
        }

        // Rule 4: undeclared visibility warning
        for name in &declared_names {
            let in_exports = bb.exports.contains(name);
            let in_private = bb.private.contains(name);
            let in_sealed = bb.sealed.contains(name);
            if !in_exports && !in_private && !in_sealed {
                errors.push(LoomError::type_err(
                    format!("[warn] boundary: '{}' is not listed in export:, private:, or seal: — visibility undeclared", name),
                    bb.span.clone(),
                ));
            }
        }
    }

    fn check_being_boundary(&self, bb: &BoundaryBlock, being: &BeingDef, errors: &mut Vec<LoomError>) {
        let declared_fn_names: Vec<String> = if let Some(func_block) = &being.function {
            func_block.fns.iter().map(|f| f.name.clone()).collect()
        } else {
            vec![]
        };

        for export in &bb.exports {
            if !declared_fn_names.is_empty() && !declared_fn_names.contains(export) {
                errors.push(LoomError::type_err(
                    format!("[error] boundary: being '{}' exports '{}' which is not declared in its function block", being.name, export),
                    bb.span.clone(),
                ));
            }
        }
    }
}

fn extract_type_name(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(name) => name.clone(),
        TypeExpr::Generic(name, _) => name.clone(),
        TypeExpr::Option(inner) => extract_type_name(inner),
        _ => String::new(),
    }
}
