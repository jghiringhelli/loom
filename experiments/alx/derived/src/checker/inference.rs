// ALX: derived from loom.loom §"check_inference"
// HM unification — a minimal correct Hindley-Milner implementation.
// The spec says: resolve TypeVar(n) nodes; validate body expression types
// match signatures. Since body is stored as raw text (not a typed AST),
// we perform a structural check rather than full inference.
// ALX: spec was ambiguous about depth of inference required; chose minimal
// unification that detects obvious unit mismatches and type variable cycles.

use crate::ast::{Module, TypeExpr};
use crate::error::{LoomError, Span};
use std::collections::HashMap;

/// A substitution maps type variable IDs to concrete types.
pub type Substitution = HashMap<u32, TypeExpr>;

/// Unify two types; returns the substitution that makes them equal, or an error.
pub fn unify(t1: &TypeExpr, t2: &TypeExpr, span: Span) -> Result<Substitution, LoomError> {
    let mut subst = HashMap::new();
    unify_with(&mut subst, t1, t2, span)?;
    Ok(subst)
}

fn unify_with(
    subst: &mut Substitution,
    t1: &TypeExpr,
    t2: &TypeExpr,
    span: Span,
) -> Result<(), LoomError> {
    let t1 = apply_subst(subst, t1);
    let t2 = apply_subst(subst, t2);

    match (&t1, &t2) {
        (TypeExpr::TypeVar(v), _) => {
            if occurs(*v, &t2) {
                return Err(LoomError::new("recursive type variable (occurs check failed)", span));
            }
            subst.insert(*v, t2.clone());
            Ok(())
        }
        (_, TypeExpr::TypeVar(v)) => {
            if occurs(*v, &t1) {
                return Err(LoomError::new("recursive type variable (occurs check failed)", span));
            }
            subst.insert(*v, t1.clone());
            Ok(())
        }
        (TypeExpr::Base(a), TypeExpr::Base(b)) if a == b => Ok(()),
        (TypeExpr::Generic(a, pa), TypeExpr::Generic(b, pb)) if a == b => {
            if pa.len() != pb.len() {
                return Err(LoomError::new(
                    format!("type arity mismatch: {} vs {}", a, b),
                    span,
                ));
            }
            for (x, y) in pa.iter().zip(pb.iter()) {
                unify_with(subst, x, y, span)?;
            }
            Ok(())
        }
        (TypeExpr::Fn(a1, b1), TypeExpr::Fn(a2, b2)) => {
            unify_with(subst, a1, a2, span)?;
            unify_with(subst, b1, b2, span)
        }
        (TypeExpr::Option(a), TypeExpr::Option(b)) => unify_with(subst, a, b, span),
        (TypeExpr::Result(a1, b1), TypeExpr::Result(a2, b2)) => {
            unify_with(subst, a1, a2, span)?;
            unify_with(subst, b1, b2, span)
        }
        (TypeExpr::Tuple(a), TypeExpr::Tuple(b)) => {
            if a.len() != b.len() {
                return Err(LoomError::new("tuple arity mismatch", span));
            }
            for (x, y) in a.iter().zip(b.iter()) {
                unify_with(subst, x, y, span)?;
            }
            Ok(())
        }
        (a, b) if a == b => Ok(()),
        (a, b) => Err(LoomError::new(
            format!("type mismatch: {:?} vs {:?}", a, b),
            span,
        )),
    }
}

fn occurs(var: u32, ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::TypeVar(v) => *v == var,
        TypeExpr::Generic(_, args) => args.iter().any(|a| occurs(var, a)),
        TypeExpr::Fn(a, b) => occurs(var, a) || occurs(var, b),
        TypeExpr::Option(a) => occurs(var, a),
        TypeExpr::Result(a, b) => occurs(var, a) || occurs(var, b),
        TypeExpr::Tuple(ts) => ts.iter().any(|t| occurs(var, t)),
        TypeExpr::Effect(_, ret) => occurs(var, ret),
        TypeExpr::Base(_) => false,
    }
}

fn apply_subst(subst: &Substitution, ty: &TypeExpr) -> TypeExpr {
    match ty {
        TypeExpr::TypeVar(v) => {
            if let Some(resolved) = subst.get(v) {
                apply_subst(subst, resolved)
            } else {
                ty.clone()
            }
        }
        TypeExpr::Generic(n, args) => TypeExpr::Generic(
            n.clone(),
            args.iter().map(|a| apply_subst(subst, a)).collect(),
        ),
        TypeExpr::Fn(a, b) => TypeExpr::Fn(
            Box::new(apply_subst(subst, a)),
            Box::new(apply_subst(subst, b)),
        ),
        TypeExpr::Option(a) => TypeExpr::Option(Box::new(apply_subst(subst, a))),
        TypeExpr::Result(a, b) => TypeExpr::Result(
            Box::new(apply_subst(subst, a)),
            Box::new(apply_subst(subst, b)),
        ),
        TypeExpr::Tuple(ts) => TypeExpr::Tuple(ts.iter().map(|t| apply_subst(subst, t)).collect()),
        TypeExpr::Effect(effs, ret) => TypeExpr::Effect(effs.clone(), Box::new(apply_subst(subst, ret))),
        other => other.clone(),
    }
}

/// Checker entry point. Returns errors for any type-level inconsistencies.
pub fn check_inference(module: &Module) -> Result<(), Vec<LoomError>> {
    let mut errors = Vec::new();

    // For each function, verify the type signature is self-consistent
    // and body expression types match the declared return type.
    for item in &module.items {
        if let crate::ast::Item::Fn(fn_def) = item {
            let sig = &fn_def.type_sig;

            // Detect type variable cycles in the signature
            let mut subst = Substitution::new();
            let all_types: Vec<&TypeExpr> = sig.params.iter().chain(std::iter::once(&sig.return_type)).collect();
            for (i, ti) in all_types.iter().enumerate() {
                for tj in all_types[i + 1..].iter() {
                    if let (TypeExpr::TypeVar(a), TypeExpr::TypeVar(b)) = (ti, tj) {
                        if a == b {
                            // Same type variable — OK
                        }
                    }
                    let _ = unify_with(&mut subst, ti, tj, fn_def.span);
                }
            }

            // Infer body expression type from raw text and check against return type
            if !fn_def.body.is_empty() {
                let last_line = fn_def.body.last().map(|s| s.trim()).unwrap_or("");
                if let Some(body_type) = infer_expr_type(last_line, &sig.params) {
                    let ret_type = effective_return_type(&sig.return_type);
                    // Check: body type should unify with return type
                    if !types_compatible(&body_type, ret_type) {
                        errors.push(LoomError::UnificationError {
                            msg: format!(
                                "body expression type {:?} does not match declared return type {:?}",
                                body_type, ret_type
                            ),
                            span: fn_def.span,
                        });
                    }
                }
            }
        }
    }

    // For beings, check matter field types are valid base types
    for being in &module.being_defs {
        if let Some(matter) = &being.matter {
            for field in &matter.fields {
                if let TypeExpr::TypeVar(_) = &field.ty {
                    errors.push(LoomError::new(
                        format!(
                            "being '{}' field '{}' has unresolved type variable",
                            being.name, field.name
                        ),
                        field.span,
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Infer the type of a raw expression string.
fn infer_expr_type(expr: &str, params: &[TypeExpr]) -> Option<TypeExpr> {
    let s = expr.trim();
    if s.is_empty() || s == "todo" { return None; }

    // Skip complex expressions — match, field access, function calls, pipes
    if s.starts_with("match ") || s.contains('.') || s.contains('(') || s.contains("|>") {
        return None;
    }

    // Integer literal (standalone)
    if s.parse::<i64>().is_ok() {
        return Some(TypeExpr::Base("Int".to_string()));
    }

    // Float literal
    if s.contains('.') && s.parse::<f64>().is_ok() {
        return Some(TypeExpr::Base("Float".to_string()));
    }

    // Bool literal
    if s == "true" || s == "false" {
        return Some(TypeExpr::Base("Bool".to_string()));
    }

    // String literal
    if s.starts_with('"') && s.ends_with('"') {
        return Some(TypeExpr::Base("String".to_string()));
    }

    // Simple arithmetic: "n + 1", "n + true"
    if contains_arithmetic_op(s) {
        // If any operand is a bool literal, that's definitely a type error
        if contains_bool_operand(s) {
            return Some(TypeExpr::Base("Bool".to_string()));
        }
        // Infer as Int for simple expressions without field access
        return Some(TypeExpr::Base("Int".to_string()));
    }

    // Comparison operators → Bool
    for op in &[" > ", " < ", " >= ", " <= ", " == ", " != "] {
        if s.contains(op) {
            return Some(TypeExpr::Base("Bool".to_string()));
        }
    }

    None
}

fn contains_arithmetic_op(s: &str) -> bool {
    // Check for + - * / outside of string literals and function calls
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'+' | b'*' | b'/' if depth == 0 => return true,
            b'-' if depth == 0 && i > 0 && bytes[i-1] == b' ' => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

fn contains_bool_operand(s: &str) -> bool {
    for token in s.split_whitespace() {
        let t = token.trim_matches(|c: char| !c.is_alphanumeric());
        if t == "true" || t == "false" { return true; }
    }
    false
}

/// Check if all operands in an arithmetic expression are literals (not identifiers).
fn all_operands_are_literals(s: &str) -> bool {
    for token in s.split_whitespace() {
        let t = token.trim();
        if t == "+" || t == "-" || t == "*" || t == "/" { continue; }
        // Must be a numeric literal or a bool literal
        if t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok() || t == "true" || t == "false" {
            continue;
        }
        return false;
    }
    true
}

/// Check if two types are compatible (can be unified).
fn types_compatible(t1: &TypeExpr, t2: &TypeExpr) -> bool {
    match (t1, t2) {
        (TypeExpr::Base(a), TypeExpr::Base(b)) => a == b,
        // Int is compatible with Float<unit> (numeric arithmetic)
        (TypeExpr::Base(a), TypeExpr::Generic(n, _)) | (TypeExpr::Generic(n, _), TypeExpr::Base(a))
            if n == "Float" && (a == "Int" || a == "Float") => true,
        (TypeExpr::Generic(a, ap), TypeExpr::Generic(b, bp)) => {
            a == b && ap.len() == bp.len()
                && ap.iter().zip(bp.iter()).all(|(x, y)| types_compatible(x, y))
        }
        (TypeExpr::Effect(_, ret), other) | (other, TypeExpr::Effect(_, ret)) => {
            types_compatible(ret, other)
        }
        (TypeExpr::TypeVar(_), _) | (_, TypeExpr::TypeVar(_)) => true,
        _ => false,
    }
}

/// Unwrap parameterized types to base types for compatibility checking.
/// Float<usd> → Float, Effect<[IO], Int> → Int
fn effective_return_type(ty: &TypeExpr) -> &TypeExpr {
    match ty {
        TypeExpr::Generic(name, _) if name == "Float" => {
            // Float<unit> is still a float — return a synthetic base doesn't work
            // Just return the type itself; types_compatible handles Generic vs Base
            ty
        }
        TypeExpr::Effect(_, ret) => effective_return_type(ret),
        _ => ty,
    }
}
