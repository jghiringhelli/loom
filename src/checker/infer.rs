//! Hindley-Milner type inference engine for the Loom compiler.
//!
//! # Architecture
//!
//! The engine runs as a pipeline pass between parsing and the symbol-resolution
//! [`super::TypeChecker`].  It validates that each function's body type is
//! consistent with its declared signature.
//!
//! ## Algorithm (simplified HM for explicitly-typed functions)
//!
//! 1. Build a **function registry** mapping `fn_name → FnTypeSignature`.
//! 2. For each function:
//!    a. Identify free variables (parameter references) in the body by scanning
//!       for identifiers that are not bound by `let`.  Assign them the declared
//!       parameter types from the signature (positional matching).
//!    b. Walk the body, inferring the type of every expression.  Unification
//!       constraints are solved in-place via a [`Substitution`].
//!    c. Unify the inferred body type with the declared return type.
//!       A failure emits [`LoomError::UnificationError`].
//!
//! ## Limitations (Phase M1)
//!
//! - Polymorphism / let-generalisation is not yet implemented.
//! - Field access and pipe operator return fresh type variables (unconstrained).
//! - Match arm types are individually inferred but cross-arm unification is
//!   deferred to M1.5.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::LoomError;

// ── Substitution ──────────────────────────────────────────────────────────────

/// A mapping from type-variable IDs to their resolved types.
///
/// The substitution is grown monotonically: once a variable is bound it is
/// never rebound to a different type.
#[derive(Default)]
pub struct Substitution(HashMap<u32, TypeExpr>);

impl Substitution {
    /// Apply the substitution to `ty`, chasing variable chains until a
    /// non-variable type (or an unbound variable) is reached.
    pub fn apply(&self, ty: &TypeExpr) -> TypeExpr {
        match ty {
            TypeExpr::TypeVar(v) => match self.0.get(v) {
                Some(bound) => self.apply(bound),
                None => ty.clone(),
            },
            TypeExpr::Base(_) => ty.clone(),
            TypeExpr::Generic(name, params) => {
                TypeExpr::Generic(name.clone(), params.iter().map(|p| self.apply(p)).collect())
            }
            TypeExpr::Effect(effs, inner) => {
                TypeExpr::Effect(effs.clone(), Box::new(self.apply(inner)))
            }
            TypeExpr::Option(inner) => TypeExpr::Option(Box::new(self.apply(inner))),
            TypeExpr::Result(ok, err) => {
                TypeExpr::Result(Box::new(self.apply(ok)), Box::new(self.apply(err)))
            }
            TypeExpr::Tuple(elems) => {
                TypeExpr::Tuple(elems.iter().map(|e| self.apply(e)).collect())
            }
            TypeExpr::Dynamic => ty.clone(),
            // Tensor — apply substitution to the unit type.
            TypeExpr::Tensor {
                rank,
                shape,
                unit,
                span,
            } => TypeExpr::Tensor {
                rank: *rank,
                shape: shape.clone(),
                unit: Box::new(self.apply(unit)),
                span: span.clone(),
            },
        }
    }

    /// Bind type variable `v` to `ty`.
    fn bind(&mut self, v: u32, ty: TypeExpr) {
        self.0.insert(v, ty);
    }
}

// ── Unification ───────────────────────────────────────────────────────────────

/// Unify `t1` and `t2` under the current `subst`, extending it if needed.
///
/// Returns `Err(UnificationError)` when types cannot be made equal.
pub fn unify(
    t1: &TypeExpr,
    t2: &TypeExpr,
    subst: &mut Substitution,
    span: Span,
) -> Result<(), LoomError> {
    let t1 = subst.apply(t1);
    let t2 = subst.apply(t2);

    match (&t1, &t2) {
        // Structurally identical after substitution — nothing to do.
        _ if t1 == t2 => Ok(()),

        // Left side is a variable — bind it.
        (TypeExpr::TypeVar(v), other) => {
            let v = *v;
            if occurs(v, other) {
                return Err(LoomError::UnificationError {
                    msg: format!(
                        "occurs check failed: type variable ?{} appears in `{:?}`",
                        v, other
                    ),
                    span,
                });
            }
            subst.bind(v, other.clone());
            Ok(())
        }

        // Right side is a variable — bind it.
        (other, TypeExpr::TypeVar(v)) => {
            let v = *v;
            if occurs(v, other) {
                return Err(LoomError::UnificationError {
                    msg: format!(
                        "occurs check failed: type variable ?{} appears in `{:?}`",
                        v, other
                    ),
                    span,
                });
            }
            subst.bind(v, other.clone());
            Ok(())
        }

        // Both base types — must be the same name.
        (TypeExpr::Base(a), TypeExpr::Base(b)) => {
            if a == b {
                Ok(())
            } else {
                Err(LoomError::UnificationError {
                    msg: format!("type mismatch: expected `{}`, found `{}`", a, b),
                    span,
                })
            }
        }

        // Generic types — name and arity must match, then unify element-wise.
        (TypeExpr::Generic(n1, p1), TypeExpr::Generic(n2, p2))
            if n1 == n2 && p1.len() == p2.len() =>
        {
            for (a, b) in p1.iter().zip(p2.iter()) {
                unify(a, b, subst, span.clone())?;
            }
            Ok(())
        }

        // Effect types — unify the inner return types.
        (TypeExpr::Effect(_, inner1), TypeExpr::Effect(_, inner2)) => {
            unify(inner1, inner2, subst, span)
        }

        // Option types.
        (TypeExpr::Option(a), TypeExpr::Option(b)) => unify(a, b, subst, span),

        // Result types.
        (TypeExpr::Result(ok1, err1), TypeExpr::Result(ok2, err2)) => {
            unify(ok1, ok2, subst, span.clone())?;
            unify(err1, err2, subst, span)
        }

        // Tuple types — arity must match, then element-wise.
        (TypeExpr::Tuple(a), TypeExpr::Tuple(b)) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b.iter()) {
                unify(x, y, subst, span.clone())?;
            }
            Ok(())
        }

        // Dynamic type is compatible with anything.
        (TypeExpr::Dynamic, _) | (_, TypeExpr::Dynamic) => Ok(()),

        // All other combinations are mismatches.
        _ => Err(LoomError::UnificationError {
            msg: format!("type mismatch: `{:?}` vs `{:?}`", t1, t2),
            span,
        }),
    }
}

/// Returns `true` if type variable `v` appears anywhere in `ty`.
///
/// Used by the occurs check to prevent infinite types.
pub fn occurs(v: u32, ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::TypeVar(u) => *u == v,
        TypeExpr::Base(_) => false,
        TypeExpr::Generic(_, params) => params.iter().any(|p| occurs(v, p)),
        TypeExpr::Effect(_, inner) => occurs(v, inner),
        TypeExpr::Option(inner) => occurs(v, inner),
        TypeExpr::Result(ok, err) => occurs(v, ok) || occurs(v, err),
        TypeExpr::Tuple(elems) => elems.iter().any(|e| occurs(v, e)),
        TypeExpr::Dynamic => false,
        // Tensor — check within the unit type.
        TypeExpr::Tensor { unit, .. } => occurs(v, unit),
    }
}

// ── Type variable generator ───────────────────────────────────────────────────

/// Generator that produces fresh, unique type-variable IDs.
struct TyVarGen {
    next: u32,
}

impl TyVarGen {
    fn new() -> Self {
        TyVarGen { next: 0 }
    }

    fn fresh(&mut self) -> TypeExpr {
        let v = self.next;
        self.next += 1;
        TypeExpr::TypeVar(v)
    }
}

// ── Type environment ──────────────────────────────────────────────────────────

/// Maps in-scope names to their types.
type TypeEnv = HashMap<String, TypeExpr>;

// ── InferenceEngine ───────────────────────────────────────────────────────────

/// Hindley-Milner inference engine.
///
/// Validates that function bodies are type-consistent with their declared
/// signatures.  Runs before the symbol-resolution [`super::TypeChecker`].
pub struct InferenceEngine;

impl InferenceEngine {
    /// Create a new `InferenceEngine`.
    pub fn new() -> Self {
        InferenceEngine
    }

    /// Check `module` and return `Ok(())` or `Err(errors)`.
    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        // Build the function registry (name → signature) for call-site checking.
        let fn_registry: HashMap<String, FnTypeSignature> = module
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Fn(fd) = item {
                    Some((fd.name.clone(), fd.type_sig.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Build refined-type-to-base-type map for subtyping during unification.
        let refined_base_map: HashMap<String, TypeExpr> = module
            .items
            .iter()
            .filter_map(|item| {
                if let Item::RefinedType(rt) = item {
                    Some((rt.name.clone(), rt.base_type.clone()))
                } else {
                    None
                }
            })
            .collect();

        let mut errors: Vec<LoomError> = Vec::new();

        for item in &module.items {
            if let Item::Fn(fd) = item {
                if let Err(mut errs) = self.check_fn(fd, &fn_registry, &refined_base_map) {
                    errors.append(&mut errs);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ── Per-function inference ────────────────────────────────────────────

    fn check_fn(
        &self,
        fd: &FnDef,
        fn_registry: &HashMap<String, FnTypeSignature>,
        refined_base_map: &HashMap<String, TypeExpr>,
    ) -> Result<(), Vec<LoomError>> {
        // Skip functions with Effect<…> return types — the EffectChecker
        // handles those separately.
        if matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _)) {
            return Ok(());
        }

        let mut gen = TyVarGen::new();
        let mut subst = Substitution::default();

        // Identify free variables (parameter references) in the body.
        let let_names = collect_let_names_in_body(&fd.body);
        let free_vars = collect_free_vars_in_body(&fd.body, &let_names);

        // Build the initial type environment, mapping free vars to their
        // declared parameter types **positionally by first appearance**.
        //
        // LIMITATION: Loom's AST has no parameter names — the function
        // signature only carries types, not names.  This means the first
        // identifier in the body that isn't let-bound is assumed to be
        // parameter 0, the second distinct identifier is parameter 1, etc.
        //
        // Consequence: if a multi-parameter function uses its second
        // parameter before its first (e.g. `fn f :: Int -> String` with
        // body `concat(s, n)` where `s` appears before `n`), the type
        // assignment will be reversed.  This is a known Phase-M1
        // limitation — full fix requires adding parameter names to
        // `FnTypeSignature` (planned for Phase 4).
        let mut env: TypeEnv = HashMap::new();
        for (i, var_name) in free_vars.iter().enumerate() {
            let ty = fd
                .type_sig
                .params
                .get(i)
                .cloned()
                .unwrap_or_else(|| gen.fresh());
            // Resolve refined types to their base types for arithmetic/comparison.
            let ty = resolve_refined_type(&ty, refined_base_map);
            env.insert(var_name.clone(), ty);
        }

        // Infer the type of each body expression.  The last expression's type
        // is unified with the declared return type.
        let mut errors: Vec<LoomError> = Vec::new();
        let mut last_ty: Option<TypeExpr> = None;

        for expr in &fd.body {
            match infer_expr(expr, &mut env, &mut subst, &mut gen, fn_registry) {
                Ok(ty) => last_ty = Some(ty),
                Err(e) => errors.push(e),
            }
        }

        // Enforce return-type consistency only for primitive return types.
        //
        // User-defined product/sum types (e.g. `OrderTotal`) require full HM
        // with record-construction inference, which is out of scope for M1.
        // The Rust type system enforces those constraints at the generated-code
        // level.
        if errors.is_empty() {
            if let Some(inferred) = last_ty {
                let declared = fd.type_sig.return_type.as_ref();
                if is_inferrable_primitive(declared) {
                    if let Err(e) = unify(&inferred, declared, &mut subst, fd.span.clone()) {
                        errors.push(e);
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
}

// ── Refined type resolution ───────────────────────────────────────────────────

/// Resolves a refined type name to its base type.
///
/// If `ty` is `Base("PositiveInt")` and `PositiveInt` refines `Int`,
/// returns `Base("Int")`. Chains through multiple levels of refinement.
fn resolve_refined_type(ty: &TypeExpr, refined_base_map: &HashMap<String, TypeExpr>) -> TypeExpr {
    match ty {
        TypeExpr::Base(name) => {
            if let Some(base) = refined_base_map.get(name) {
                resolve_refined_type(base, refined_base_map)
            } else {
                ty.clone()
            }
        }
        _ => ty.clone(),
    }
}

// ── Expression type inference ─────────────────────────────────────────────────

fn infer_expr(
    expr: &Expr,
    env: &mut TypeEnv,
    subst: &mut Substitution,
    gen: &mut TyVarGen,
    fns: &HashMap<String, FnTypeSignature>,
) -> Result<TypeExpr, LoomError> {
    match expr {
        Expr::Literal(lit) => Ok(infer_literal(lit)),

        Expr::Ident(name) => Ok(env.get(name).cloned().unwrap_or_else(|| gen.fresh())),

        Expr::BinOp {
            op,
            left,
            right,
            span,
        } => infer_binop(op, left, right, span, env, subst, gen, fns),

        Expr::Let { name, value, .. } => {
            let ty = infer_expr(value, env, subst, gen, fns)?;
            let resolved = subst.apply(&ty);
            env.insert(name.clone(), resolved.clone());
            Ok(resolved)
        }

        Expr::Call { func, args, span } => infer_call(func, args, span, env, subst, gen, fns),

        Expr::Match {
            subject,
            arms,
            span,
        } => infer_match(subject, arms, span, env, subst, gen, fns),

        // Pipe and field access return fresh TypeVars — resolved in M1.5.
        Expr::Pipe { .. } | Expr::FieldAccess { .. } => Ok(gen.fresh()),

        // InlineRust is opaque — assign a fresh TypeVar, no constraints generated.
        Expr::InlineRust(_) => Ok(gen.fresh()),

        Expr::As(inner, ty) => {
            // Infer the inner expression and unify with the target type.
            let _ = infer_expr(inner, env, subst, gen, fns)?;
            Ok(subst.apply(ty))
        }

        Expr::Lambda { params, body, .. } => {
            // Introduce param bindings as fresh TypeVars, infer body.
            for (name, ty_opt) in params {
                let ty = ty_opt.as_ref().cloned().unwrap_or_else(|| gen.fresh());
                env.insert(name.clone(), ty);
            }
            infer_expr(body, env, subst, gen, fns)
        }

        Expr::ForIn { iter, body, .. } => {
            let _ = infer_expr(iter, env, subst, gen, fns)?;
            let _ = infer_expr(body, env, subst, gen, fns)?;
            Ok(TypeExpr::Base("Unit".to_string()))
        }

        Expr::Tuple(elems, _) => {
            let types: Result<Vec<_>, _> = elems
                .iter()
                .map(|e| infer_expr(e, env, subst, gen, fns))
                .collect();
            Ok(TypeExpr::Tuple(types?))
        }

        Expr::Try(inner, _) => {
            // ? propagates errors; the inner type must be Result<T, E>.
            // Return a fresh TypeVar for now — full Result inference in M12.5.
            let _ = infer_expr(inner, env, subst, gen, fns)?;
            Ok(gen.fresh())
        }
    }
}

fn infer_literal(lit: &Literal) -> TypeExpr {
    match lit {
        Literal::Int(_) => TypeExpr::Base("Int".to_string()),
        Literal::Float(_) => TypeExpr::Base("Float".to_string()),
        Literal::Bool(_) => TypeExpr::Base("Bool".to_string()),
        Literal::Str(_) => TypeExpr::Base("String".to_string()),
        Literal::Unit => TypeExpr::Base("Unit".to_string()),
    }
}

fn infer_binop(
    op: &BinOpKind,
    left: &Expr,
    right: &Expr,
    span: &Span,
    env: &mut TypeEnv,
    subst: &mut Substitution,
    gen: &mut TyVarGen,
    fns: &HashMap<String, FnTypeSignature>,
) -> Result<TypeExpr, LoomError> {
    let t_left = infer_expr(left, env, subst, gen, fns)?;
    let t_right = infer_expr(right, env, subst, gen, fns)?;

    match op {
        // Arithmetic: both operands must have the same type; result is that type.
        BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div => {
            unify(&t_left, &t_right, subst, span.clone())?;
            Ok(subst.apply(&t_left))
        }

        // Comparison: both operands must match; result is Bool.
        BinOpKind::Eq
        | BinOpKind::Ne
        | BinOpKind::Lt
        | BinOpKind::Le
        | BinOpKind::Gt
        | BinOpKind::Ge => {
            unify(&t_left, &t_right, subst, span.clone())?;
            Ok(TypeExpr::Base("Bool".to_string()))
        }

        // Boolean logic: both operands must be Bool; result is Bool.
        BinOpKind::And | BinOpKind::Or => {
            let bool_ty = TypeExpr::Base("Bool".to_string());
            unify(&t_left, &bool_ty, subst, span.clone())?;
            unify(&t_right, &bool_ty, subst, span.clone())?;
            Ok(bool_ty)
        }
    }
}

fn infer_call(
    func: &Expr,
    args: &[Expr],
    span: &Span,
    env: &mut TypeEnv,
    subst: &mut Substitution,
    gen: &mut TyVarGen,
    fns: &HashMap<String, FnTypeSignature>,
) -> Result<TypeExpr, LoomError> {
    // Infer argument types.
    let arg_types: Vec<TypeExpr> = args
        .iter()
        .map(|a| infer_expr(a, env, subst, gen, fns))
        .collect::<Result<_, _>>()?;

    // Named function call.
    if let Expr::Ident(fn_name) = func {
        if let Some(sig) = fns.get(fn_name) {
            // Unify each argument type with the declared parameter type.
            for (arg_ty, param_ty) in arg_types.iter().zip(sig.params.iter()) {
                unify(arg_ty, param_ty, subst, span.clone())?;
            }
            return Ok(*sig.return_type.clone());
        }
    }

    // Unknown function or higher-order call — return a fresh TypeVar.
    Ok(gen.fresh())
}

fn infer_match(
    subject: &Expr,
    arms: &[MatchArm],
    span: &Span,
    env: &mut TypeEnv,
    subst: &mut Substitution,
    gen: &mut TyVarGen,
    fns: &HashMap<String, FnTypeSignature>,
) -> Result<TypeExpr, LoomError> {
    // Infer subject type (unused for now — exhaustiveness checker handles it).
    let _subject_ty = infer_expr(subject, env, subst, gen, fns)?;

    // Infer each arm's body type and unify them all.
    let result_ty = gen.fresh();
    for arm in arms {
        let mut arm_env = env.clone();
        collect_pattern_type_bindings(&arm.pattern, &mut arm_env, gen);

        if let Some(guard) = &arm.guard {
            let guard_ty = infer_expr(guard, &mut arm_env, subst, gen, fns)?;
            let bool_ty = TypeExpr::Base("Bool".to_string());
            unify(&guard_ty, &bool_ty, subst, span.clone())?;
        }

        let body_ty = infer_expr(&arm.body, &mut arm_env, subst, gen, fns)?;
        unify(&body_ty, &result_ty, subst, span.clone())?;
    }

    Ok(subst.apply(&result_ty))
}

/// Add type bindings for pattern-bound variables to the environment.
fn collect_pattern_type_bindings(pat: &Pattern, env: &mut TypeEnv, gen: &mut TyVarGen) {
    match pat {
        Pattern::Ident(name) => {
            env.insert(name.clone(), gen.fresh());
        }
        Pattern::Variant(_, sub_pats) => {
            for sub in sub_pats {
                collect_pattern_type_bindings(sub, env, gen);
            }
        }
        Pattern::Wildcard | Pattern::Literal(_) => {}
    }
}

// ── Free-variable and let-name helpers ───────────────────────────────────────
// (Same logic as in the WASM emitter — kept local to avoid cross-module coupling.)

fn collect_let_names_in_body(body: &[Expr]) -> Vec<String> {
    let mut names = Vec::new();
    for expr in body {
        collect_let_names(expr, &mut names);
    }
    names
}

fn collect_let_names(expr: &Expr, names: &mut Vec<String>) {
    match expr {
        Expr::Let { name, value, .. } => {
            if !names.contains(name) {
                names.push(name.clone());
            }
            collect_let_names(value, names);
        }
        Expr::BinOp { left, right, .. } => {
            collect_let_names(left, names);
            collect_let_names(right, names);
        }
        Expr::Call { func, args, .. } => {
            collect_let_names(func, names);
            for a in args {
                collect_let_names(a, names);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_let_names(left, names);
            collect_let_names(right, names);
        }
        Expr::Match { subject, arms, .. } => {
            collect_let_names(subject, names);
            for arm in arms {
                collect_let_names(&arm.body, names);
            }
        }
        Expr::FieldAccess { object, .. } => collect_let_names(object, names),
        Expr::Ident(_) | Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque
        Expr::As(inner, _) => collect_let_names(inner, names),
        Expr::Lambda { body, .. } => collect_let_names(body, names),
        Expr::ForIn { iter, body, .. } => {
            collect_let_names(iter, names);
            collect_let_names(body, names);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| collect_let_names(e, names)),
        Expr::Try(inner, _) => collect_let_names(inner, names),
    }
}

fn collect_free_vars_in_body(body: &[Expr], let_names: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let let_set: HashSet<&str> = let_names.iter().map(String::as_str).collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();
    for expr in body {
        collect_free_vars(expr, &let_set, &mut seen, &mut ordered);
    }
    ordered
}

fn collect_free_vars(
    expr: &Expr,
    let_bound: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<String>,
    ordered: &mut Vec<String>,
) {
    match expr {
        Expr::Ident(name) => {
            // Skip built-in keyword-like identifiers that are not function parameters.
            const BUILTINS: &[&str] = &["todo", "panic", "unreachable", "unimplemented"];
            if !let_bound.contains(name.as_str())
                && !seen.contains(name)
                && !BUILTINS.contains(&name.as_str())
            {
                seen.insert(name.clone());
                ordered.push(name.clone());
            }
        }
        Expr::Let { value, .. } => collect_free_vars(value, let_bound, seen, ordered),
        Expr::BinOp { left, right, .. } => {
            collect_free_vars(left, let_bound, seen, ordered);
            collect_free_vars(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            if !matches!(func.as_ref(), Expr::Ident(_)) {
                collect_free_vars(func, let_bound, seen, ordered);
            }
            for a in args {
                collect_free_vars(a, let_bound, seen, ordered);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_free_vars(left, let_bound, seen, ordered);
            collect_free_vars(right, let_bound, seen, ordered);
        }
        Expr::Match { subject, arms, .. } => {
            collect_free_vars(subject, let_bound, seen, ordered);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    collect_free_vars(g, let_bound, seen, ordered);
                }
                collect_free_vars(&arm.body, let_bound, seen, ordered);
            }
        }
        Expr::FieldAccess { object, .. } => collect_free_vars(object, let_bound, seen, ordered),
        Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque — no free variables
        Expr::As(inner, _) => collect_free_vars(inner, let_bound, seen, ordered),
        Expr::Lambda { params, body, .. } => {
            // Lambda params are bound — exclude them from outer free-var scan.
            let mut extended: std::collections::HashSet<&str> = let_bound.clone();
            for (name, _) in params {
                extended.insert(name.as_str());
            }
            collect_free_vars(body, &extended, seen, ordered);
        }
        Expr::ForIn {
            var, iter, body, ..
        } => {
            collect_free_vars(iter, let_bound, seen, ordered);
            let mut extended = let_bound.clone();
            extended.insert(var.as_str());
            collect_free_vars(body, &extended, seen, ordered);
        }
        Expr::Tuple(elems, _) => {
            elems
                .iter()
                .for_each(|e| collect_free_vars(e, let_bound, seen, ordered));
        }
        Expr::Try(inner, _) => collect_free_vars(inner, let_bound, seen, ordered),
    }
}

// ── Primitive type predicate ──────────────────────────────────────────────────

/// Returns `true` if `ty` is a primitive type whose values the inference
/// engine can reliably infer from expressions.
///
/// User-defined types (product types, sum types) require full HM with
/// record-construction and variant-construction inference, which is deferred
/// to a later phase.
fn is_inferrable_primitive(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Base(name)
            if matches!(name.as_str(), "Int" | "Float" | "Bool" | "String" | "Unit")
    )
}

// ── Tests for the unifier (internal) ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span::synthetic()
    }

    fn base(name: &str) -> TypeExpr {
        TypeExpr::Base(name.to_string())
    }

    #[test]
    fn unify_identical_base_types_succeeds() {
        let mut s = Substitution::default();
        assert!(unify(&base("Int"), &base("Int"), &mut s, span()).is_ok());
    }

    #[test]
    fn unify_different_base_types_fails() {
        let mut s = Substitution::default();
        assert!(unify(&base("Int"), &base("Bool"), &mut s, span()).is_err());
    }

    #[test]
    fn unify_typevar_with_base_binds_variable() {
        let mut s = Substitution::default();
        let v = TypeExpr::TypeVar(0);
        unify(&v, &base("Int"), &mut s, span()).unwrap();
        assert_eq!(s.apply(&TypeExpr::TypeVar(0)), base("Int"));
    }

    #[test]
    fn unify_typevar_with_itself_succeeds() {
        let mut s = Substitution::default();
        let v = TypeExpr::TypeVar(0);
        assert!(unify(&v, &v, &mut s, span()).is_ok());
    }

    #[test]
    fn occurs_check_detects_infinite_type() {
        // TypeVar(0) appears inside TypeVar(0) → unification error
        let mut s = Substitution::default();
        let v0 = TypeExpr::TypeVar(0);
        let recursive = TypeExpr::Generic("List".to_string(), vec![TypeExpr::TypeVar(0)]);
        let result = unify(&v0, &recursive, &mut s, span());
        assert!(result.is_err(), "occurs check should reject a → List<a>");
        assert!(matches!(
            result.unwrap_err(),
            LoomError::UnificationError { .. }
        ));
    }

    #[test]
    fn chain_substitution_resolves_fully() {
        // TypeVar(0) = TypeVar(1), TypeVar(1) = Int → apply(TypeVar(0)) = Int
        let mut s = Substitution::default();
        s.bind(0, TypeExpr::TypeVar(1));
        s.bind(1, base("Int"));
        assert_eq!(s.apply(&TypeExpr::TypeVar(0)), base("Int"));
    }
}
