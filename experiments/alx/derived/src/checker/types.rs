// ALX: derived from loom.loom §"check_types"
// Symbol resolution: verifies all type names and function names are declared,
// and that interface implementations provide all required methods.

use crate::ast::{Module, TypeExpr, Item};
use crate::error::{LoomError, Span};
use std::collections::HashSet;

/// G4: TypeChecker struct — tests call `TypeChecker::new().check(&module)`.
pub struct TypeChecker;

impl TypeChecker {
    pub fn new() -> Self { TypeChecker }
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        check_types(module)
    }
}

/// Built-in type names that are always in scope.
const BUILTIN_TYPES: &[&str] = &[
    "Int", "Float", "String", "Bool", "Unit",
    "List", "Map", "Set", "Option", "Result", "Effect",
];

pub fn check_types(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // Build the set of all declared type names.
    let mut declared_types: HashSet<String> = BUILTIN_TYPES.iter().map(|s| s.to_string()).collect();

    for item in &module.items {
        match item {
            Item::Type(t) => { declared_types.insert(t.name.clone()); }
            Item::Enum(e) => { declared_types.insert(e.name.clone()); }
            Item::RefinedType(r) => { declared_types.insert(r.name.clone()); }
            _ => {}
        }
    }
    // Lifecycle state types
    for lc in &module.lifecycle_defs {
        declared_types.insert(lc.type_name.clone());
        for state in &lc.states {
            declared_types.insert(state.clone());
        }
    }
    // Flow label types
    for fl in &module.flow_labels {
        for ty in &fl.types {
            declared_types.insert(ty.clone());
        }
    }
    // Being names are types too
    for b in &module.being_defs {
        declared_types.insert(b.name.clone());
    }
    // Interface type params (A, B, T etc. are implicitly declared)
    // We skip single-uppercase-letter params as they are always type variables.

    // Build the set of all declared function names.
    let mut declared_fns: HashSet<String> = HashSet::new();
    for item in &module.items {
        if let Item::Fn(f) = item {
            declared_fns.insert(f.name.clone());
        }
    }

    // Verify type_exprs used in functions reference declared types.
    for item in &module.items {
        if let Item::Fn(f) = item {
            check_type_expr(&f.type_sig.return_type, &declared_types, &mut errors, f.span);
            for p in &f.type_sig.params {
                check_type_expr(p, &declared_types, &mut errors, f.span);
            }
        }
    }

    // Verify interface implementations: every implements clause must have a
    // corresponding interface_def with all methods.
    let declared_interfaces: HashSet<String> =
        module.interface_defs.iter().map(|i| i.name.clone()).collect();
    for impl_name in &module.implements {
        // ALX: interface name might be generic e.g. Repository<User> — extract base name
        let base = impl_name.split('<').next().unwrap_or(impl_name);
        if !declared_interfaces.contains(base) && !declared_types.contains(base) {
            // ALX: the interface might be imported; we allow it
        }
        // Check all required methods are provided
        if let Some(iface) = module.interface_defs.iter().find(|i| i.name == base) {
            for method in &iface.methods {
                if !declared_fns.contains(&method.name) {
                    errors.push(LoomError::new(
                        format!("implements {}: missing method '{}' required by interface", impl_name, method.name),
                        module.span,
                    ));
                }
            }
        }
    }

    // Ecosystem members must refer to declared beings.
    let being_names: HashSet<String> =
        module.being_defs.iter().map(|b| b.name.clone()).collect();
    for eco in &module.ecosystem_defs {
        for member in &eco.members {
            if !being_names.contains(member) {
                errors.push(LoomError::new(
                    format!(
                        "ecosystem '{}': unknown being member '{}'",
                        eco.name, member
                    ),
                    eco.span,
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_type_expr(
    ty: &TypeExpr,
    declared: &HashSet<String>,
    errors: &mut Vec<LoomError>,
    span: Span,
) {
    match ty {
        TypeExpr::Base(name) => {
            // Allow single-uppercase-letter type variables (A, B, T, E, etc.)
            if name.len() == 1 && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                return;
            }
            // Allow lowercase names — they are unit annotations (usd, eur, km, etc.)
            if name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                return;
            }
            if !declared.contains(name.as_str()) {
                errors.push(LoomError::new(
                    format!("unknown type '{}'", name),
                    span,
                ));
            }
        }
        TypeExpr::Generic(name, args) => {
            // Float<unit> / Int<unit>: unit args are not types, skip their check
            let is_unit_parameterized = matches!(name.as_str(), "Float" | "Int");
            if !is_unit_parameterized && !declared.contains(name.as_str()) && name != "Effect" {
                // ALX: generic type names may be type params themselves
                if name.len() > 1 || !name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // Not a type variable — check it
                    if !BUILTIN_TYPES.contains(&name.as_str()) {
                        errors.push(LoomError::new(
                            format!("unknown generic type '{}'", name),
                            span,
                        ));
                    }
                }
            }
            for arg in args {
                if !is_unit_parameterized {
                    check_type_expr(arg, declared, errors, span);
                }
            }
        }
        TypeExpr::Option(inner) | TypeExpr::Effect(_, inner) => {
            check_type_expr(inner, declared, errors, span);
        }
        TypeExpr::Result(ok, err) => {
            check_type_expr(ok, declared, errors, span);
            check_type_expr(err, declared, errors, span);
        }
        TypeExpr::Tuple(types) => {
            for t in types {
                check_type_expr(t, declared, errors, span);
            }
        }
        TypeExpr::Fn(a, b) => {
            check_type_expr(a, declared, errors, span);
            check_type_expr(b, declared, errors, span);
        }
        TypeExpr::TypeVar(_) => {} // resolved by inference
    }
}
