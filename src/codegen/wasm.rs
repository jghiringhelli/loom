//! WebAssembly Text format (WAT) emitter for the Loom compiler.
//!
//! # Supported subset (Phase M3)
//!
//! | Loom construct | WAT output |
//! |---|---|
//! | `fn f :: Int -> Int -> Int` | `(func $f (export "f") (param $p0 i64) (param $p1 i64) (result i64) …)` |
//! | `let x = expr` | `(local $x i64)` declaration + `local.set $x` |
//! | `a + b` (Int) | `local.get $p0 \n local.get $p1 \n i64.add` |
//! | `n > 0` | `local.get $p0 \n i64.const 0 \n i64.gt_s` |
//! | Literals | `i64.const N`, `f64.const F`, `i32.const 0/1` |
//!
//! # Unsupported (returns [`LoomError::WasmUnsupported`])
//!
//! `Effect<…>` types · `enum` types · refined types · `match` expressions ·
//! pipe operator · field access · boolean `and`/`or` operators.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LoomError;

// ── WAT primitive types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum WasmType {
    I64, // Int
    F64, // Float
    I32, // Bool
}

impl WasmType {
    fn from_type_expr(ty: &TypeExpr) -> Option<Self> {
        match ty {
            TypeExpr::Base(name) => match name.as_str() {
                "Int" => Some(WasmType::I64),
                "Float" => Some(WasmType::F64),
                "Bool" => Some(WasmType::I32),
                _ => None,
            },
            _ => None,
        }
    }

    fn wat_name(self) -> &'static str {
        match self {
            WasmType::I64 => "i64",
            WasmType::F64 => "f64",
            WasmType::I32 => "i32",
        }
    }
}

// ── WasmEmitter ───────────────────────────────────────────────────────────────

/// Emits WebAssembly Text format (WAT) from a Loom [`Module`].
pub struct WasmEmitter;

impl WasmEmitter {
    /// Create a new `WasmEmitter`.
    pub fn new() -> Self {
        WasmEmitter
    }

    /// Emit a WAT module string from a [`Module`] AST.
    ///
    /// Returns `Err` if any item uses a construct unsupported by the WASM
    /// back-end (effect types, enums, refined types, match expressions, etc.).
    pub fn emit(&self, module: &Module) -> Result<String, Vec<LoomError>> {
        let mut errors = Vec::new();
        let mut funcs: Vec<String> = Vec::new();

        for item in &module.items {
            match item {
                Item::Fn(fd) => match self.emit_fn(fd) {
                    Ok(wat) => funcs.push(wat),
                    Err(e) => errors.push(e),
                },
                Item::Type(_) => {
                    // Product types have no direct WAT representation in M3 —
                    // they are skipped rather than erroring so that modules that
                    // define types alongside functions can still be compiled.
                }
                Item::Enum(ed) => errors.push(LoomError::WasmUnsupported {
                    feature: format!("enum type `{}`", ed.name),
                    span: ed.span.clone(),
                }),
                Item::RefinedType(rt) => errors.push(LoomError::WasmUnsupported {
                    feature: format!("refined type `{}`", rt.name),
                    span: rt.span.clone(),
                }),
                _ => {}
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let mut out = String::with_capacity(512);
        out.push_str("(module\n");
        for func in funcs {
            out.push_str(&func);
        }
        out.push_str(")\n");
        Ok(out)
    }

    // ── Function emission ─────────────────────────────────────────────────

    fn emit_fn(&self, fd: &FnDef) -> Result<String, LoomError> {
        // Effect-typed return values are unsupported.
        if matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _)) {
            return Err(LoomError::WasmUnsupported {
                feature: format!("effectful function `{}`", fd.name),
                span: fd.span.clone(),
            });
        }

        // Resolve return type.
        let return_type = WasmType::from_type_expr(&fd.type_sig.return_type).ok_or_else(|| {
            LoomError::WasmUnsupported {
                feature: format!("unsupported return type in `{}`", fd.name),
                span: fd.span.clone(),
            }
        })?;

        // Resolve parameter types.
        let param_types: Vec<WasmType> = fd
            .type_sig
            .params
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                WasmType::from_type_expr(ty).ok_or_else(|| LoomError::WasmUnsupported {
                    feature: format!("unsupported type for parameter {} of `{}`", i, fd.name),
                    span: fd.span.clone(),
                })
            })
            .collect::<Result<_, _>>()?;

        // Collect let-bound names to distinguish them from parameter references.
        let let_names = collect_let_names(&fd.body);

        // Collect free variables (parameter references) in order of first use.
        let free_vars = collect_free_vars(&fd.body, &let_names);

        // Build the parameter resolution map: source name → (WAT param index, type).
        let param_map: HashMap<String, (usize, WasmType)> = free_vars
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let ty = param_types.get(i).copied().unwrap_or(WasmType::I64);
                (name.clone(), (i, ty))
            })
            .collect();

        // Build the let-local map: source name → (WAT local name, type).
        // Types default to i64; M1 type inference will refine this.
        let let_map: HashMap<String, WasmType> = let_names
            .iter()
            .map(|name| (name.clone(), WasmType::I64))
            .collect();

        // ── Emit WAT ──────────────────────────────────────────────────────

        let params_str: Vec<String> = param_types
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("(param $p{} {})", i, ty.wat_name()))
            .collect();

        let mut out = String::new();
        out.push_str(&format!(
            "  (func ${name} (export \"{name}\") {params} (result {ret})\n",
            name = fd.name,
            params = params_str.join(" "),
            ret = return_type.wat_name(),
        ));

        // Declare locals for let bindings.
        for name in &let_names {
            let ty = let_map[name];
            out.push_str(&format!("    (local ${} {})\n", name, ty.wat_name()));
        }

        // Emit body instructions.
        let mut instrs: Vec<String> = Vec::new();
        for expr in &fd.body {
            emit_expr(expr, &param_map, &let_map, &mut instrs)?;
        }
        for instr in instrs {
            out.push_str(&format!("    {}\n", instr));
        }

        out.push_str("  )\n");
        Ok(out)
    }
}

// ── Expression emission ───────────────────────────────────────────────────────

fn emit_expr(
    expr: &Expr,
    params: &HashMap<String, (usize, WasmType)>,
    lets: &HashMap<String, WasmType>,
    out: &mut Vec<String>,
) -> Result<(), LoomError> {
    match expr {
        Expr::Literal(lit) => out.push(emit_literal(lit)),

        Expr::Ident(name) => {
            if let Some((idx, _)) = params.get(name) {
                out.push(format!("local.get $p{}", idx));
            } else if lets.contains_key(name) {
                out.push(format!("local.get ${}", name));
            } else {
                out.push(format!(";; TODO: unresolved identifier `{}`", name));
            }
        }

        Expr::BinOp {
            op,
            left,
            right,
            span,
        } => {
            emit_expr(left, params, lets, out)?;
            emit_expr(right, params, lets, out)?;
            out.push(emit_binop(op, span)?);
        }

        Expr::Let { name, value, .. } => {
            emit_expr(value, params, lets, out)?;
            out.push(format!("local.set ${}", name));
        }

        Expr::Call { func, args, span } => {
            for arg in args {
                emit_expr(arg, params, lets, out)?;
            }
            match func.as_ref() {
                Expr::Ident(fn_name) => out.push(format!("call ${}", fn_name)),
                _ => {
                    return Err(LoomError::WasmUnsupported {
                        feature: "indirect or higher-order function call".to_string(),
                        span: span.clone(),
                    })
                }
            }
        }

        Expr::Match { span, .. } => {
            return Err(LoomError::WasmUnsupported {
                feature: "match expression".to_string(),
                span: span.clone(),
            })
        }

        Expr::Pipe { span, .. } => {
            return Err(LoomError::WasmUnsupported {
                feature: "pipe operator (|>)".to_string(),
                span: span.clone(),
            })
        }

        Expr::FieldAccess { span, .. } => {
            return Err(LoomError::WasmUnsupported {
                feature: "field access".to_string(),
                span: span.clone(),
            })
        }

        Expr::InlineRust(_) => {
            return Err(LoomError::WasmUnsupported {
                feature: "inline rust block".to_string(),
                span: Span::synthetic(),
            })
        }

        Expr::As(_, _) => {
            return Err(LoomError::WasmUnsupported {
                feature: "as coercion".to_string(),
                span: Span::synthetic(),
            })
        }

        Expr::Lambda { .. } | Expr::ForIn { .. } => {
            return Err(LoomError::WasmUnsupported {
                feature: "lambda / for-in (use inline rust)".to_string(),
                span: Span::synthetic(),
            })
        }

        Expr::Tuple(_, _) | Expr::Try(_, _) => {
            return Err(LoomError::WasmUnsupported {
                feature: "tuple / try operator".to_string(),
                span: Span::synthetic(),
            })
        }
    }
    Ok(())
}

fn emit_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => format!("i64.const {}", n),
        Literal::Float(f) => format!("f64.const {}", f),
        Literal::Bool(b) => format!("i32.const {}", if *b { 1 } else { 0 }),
        Literal::Str(_) => ";; TODO: string literals not supported in WASM M3".to_string(),
        Literal::Unit => ";; unit value (nop)".to_string(),
    }
}

fn emit_binop(op: &BinOpKind, span: &Span) -> Result<String, LoomError> {
    // Default to i64 (integer) operations.  Float detection requires M1
    // type inference; until then, float operations require explicit Float
    // type in the signature for the caller to opt in via compile flags.
    Ok(match op {
        BinOpKind::Add => "i64.add",
        BinOpKind::Sub => "i64.sub",
        BinOpKind::Mul => "i64.mul",
        BinOpKind::Div => "i64.div_s",
        BinOpKind::Eq => "i64.eq",
        BinOpKind::Ne => "i64.ne",
        BinOpKind::Lt => "i64.lt_s",
        BinOpKind::Le => "i64.le_s",
        BinOpKind::Gt => "i64.gt_s",
        BinOpKind::Ge => "i64.ge_s",
        BinOpKind::And | BinOpKind::Or => {
            return Err(LoomError::WasmUnsupported {
                feature: format!("boolean operator `{:?}` (requires i32 operands)", op),
                span: span.clone(),
            })
        }
    }
    .to_string())
}

// ── Free-variable and let-name collection ─────────────────────────────────────

/// Collect all `let`-bound names from a list of expressions (in definition order).
fn collect_let_names(body: &[Expr]) -> Vec<String> {
    let mut names = Vec::new();
    for expr in body {
        collect_let_names_in(expr, &mut names);
    }
    names
}

fn collect_let_names_in(expr: &Expr, names: &mut Vec<String>) {
    match expr {
        Expr::Let { name, value, .. } => {
            if !names.contains(name) {
                names.push(name.clone());
            }
            collect_let_names_in(value, names);
        }
        Expr::BinOp { left, right, .. } => {
            collect_let_names_in(left, names);
            collect_let_names_in(right, names);
        }
        Expr::Call { func, args, .. } => {
            collect_let_names_in(func, names);
            for arg in args {
                collect_let_names_in(arg, names);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_let_names_in(left, names);
            collect_let_names_in(right, names);
        }
        Expr::Match { subject, arms, .. } => {
            collect_let_names_in(subject, names);
            for arm in arms {
                collect_let_names_in(&arm.body, names);
            }
        }
        Expr::FieldAccess { object, .. } => collect_let_names_in(object, names),
        Expr::Literal(_) | Expr::Ident(_) => {}
        Expr::InlineRust(_) => {} // opaque
        Expr::As(inner, _) => collect_let_names_in(inner, names),
        Expr::Lambda { body, .. } => collect_let_names_in(body, names),
        Expr::ForIn { iter, body, .. } => {
            collect_let_names_in(iter, names);
            collect_let_names_in(body, names);
        }
        Expr::Tuple(elems, _) => elems.iter().for_each(|e| collect_let_names_in(e, names)),
        Expr::Try(inner, _) => collect_let_names_in(inner, names),
    }
}

/// Collect free-variable references (parameter names) in first-appearance order.
///
/// "Free" means: used as a bare identifier but not bound by any `let` in the body.
/// Function names that appear as the target of a `Call` are excluded.
fn collect_free_vars(body: &[Expr], let_names: &[String]) -> Vec<String> {
    let let_set: HashSet<&str> = let_names.iter().map(String::as_str).collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();
    for expr in body {
        collect_free_vars_in(expr, &let_set, &mut seen, &mut ordered);
    }
    ordered
}

fn collect_free_vars_in(
    expr: &Expr,
    let_bound: &HashSet<&str>,
    seen: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) {
    match expr {
        Expr::Ident(name) => {
            if !let_bound.contains(name.as_str()) && !seen.contains(name) {
                seen.insert(name.clone());
                ordered.push(name.clone());
            }
        }
        Expr::Let { value, .. } => collect_free_vars_in(value, let_bound, seen, ordered),
        Expr::BinOp { left, right, .. } => {
            collect_free_vars_in(left, let_bound, seen, ordered);
            collect_free_vars_in(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            // Skip a bare-ident func — it's a call target, not a value reference.
            if !matches!(func.as_ref(), Expr::Ident(_)) {
                collect_free_vars_in(func, let_bound, seen, ordered);
            }
            for arg in args {
                collect_free_vars_in(arg, let_bound, seen, ordered);
            }
        }
        Expr::Pipe { left, right, .. } => {
            collect_free_vars_in(left, let_bound, seen, ordered);
            collect_free_vars_in(right, let_bound, seen, ordered);
        }
        Expr::FieldAccess { object, .. } => collect_free_vars_in(object, let_bound, seen, ordered),
        Expr::Match { subject, arms, .. } => {
            collect_free_vars_in(subject, let_bound, seen, ordered);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_free_vars_in(guard, let_bound, seen, ordered);
                }
                collect_free_vars_in(&arm.body, let_bound, seen, ordered);
            }
        }
        Expr::Literal(_) => {}
        Expr::InlineRust(_) => {} // opaque — no free variables
        Expr::As(inner, _) => collect_free_vars_in(inner, let_bound, seen, ordered),
        Expr::Lambda { params, body, .. } => {
            let mut ext = let_bound.clone();
            for (name, _) in params {
                ext.insert(name.as_str());
            }
            collect_free_vars_in(body, &ext, seen, ordered);
        }
        Expr::ForIn {
            var, iter, body, ..
        } => {
            collect_free_vars_in(iter, let_bound, seen, ordered);
            let mut ext = let_bound.clone();
            ext.insert(var.as_str());
            collect_free_vars_in(body, &ext, seen, ordered);
        }
        Expr::Tuple(elems, _) => {
            elems
                .iter()
                .for_each(|e| collect_free_vars_in(e, let_bound, seen, ordered));
        }
        Expr::Try(inner, _) => collect_free_vars_in(inner, let_bound, seen, ordered),
    }
}
