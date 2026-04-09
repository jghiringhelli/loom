// ALX: derived from loom.loom §"emit_wasm"
// WebAssembly text format (WAT) emitter.
// Only pure functions (no effects) are emitted.
// Effect-bearing, refined-type, and match-using functions return WasmUnsupported.

use crate::ast::*;
use crate::error::{LoomError, Span};

pub fn emit_wasm(module: &Module) -> Result<String, Vec<LoomError>> {
    let mut out = String::new();
    let mut errors = Vec::new();

    out.push_str("(module\n");

    for item in &module.items {
        match item {
            Item::Fn(f) => {
                // Check for unsupported features
                let has_effects = !f.effect_tiers.is_empty()
                    || matches!(f.type_sig.return_type, TypeExpr::Effect(_, _));
                if has_effects {
                    errors.push(LoomError::WasmUnsupported {
                        feature: "effectful functions".into(),
                        span: f.span,
                    });
                    continue;
                }
                // Check body for match expressions (simple heuristic)
                let body_str = f.body.join(" ");
                if body_str.contains("match ") || body_str.split_whitespace().any(|w| w == "match") {
                    errors.push(LoomError::WasmUnsupported {
                        feature: "match expressions".into(),
                        span: f.span,
                    });
                    continue;
                }
                emit_wasm_func(&mut out, f);
            }
            Item::RefinedType(r) => {
                errors.push(LoomError::WasmUnsupported {
                    feature: format!("refined types ({})", r.name),
                    span: r.span,
                });
            }
            _ => {}
        }
    }

    out.push_str(")\n");

    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

fn emit_wasm_func(out: &mut String, f: &FnDef) {
    if let Some(desc) = &f.describe {
        out.push_str(&format!("  ;; {}\n", desc));
    }

    // Build param list
    let params: Vec<String> = f
        .type_sig
        .params
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("(param $p{} {})", i, type_to_wasm(ty)))
        .collect();

    let ret = type_to_wasm(&f.type_sig.return_type);

    // Collect let-bound local vars from body
    let mut locals: Vec<(String, String)> = Vec::new();
    for stmt in &f.body {
        let s = stmt.trim();
        if s.starts_with("let ") {
            if let Some(eq) = s.find(" = ") {
                let name = s[4..eq].trim().to_string();
                locals.push((name, ret.clone())); // default to return type
            }
        }
    }

    out.push_str(&format!(
        "  (func ${} {} (result {})\n",
        f.name,
        params.join(" "),
        ret
    ));

    // Declare locals
    for (name, ty) in &locals {
        out.push_str(&format!("    (local ${} {})\n", name, ty));
    }

    // Emit body
    if !f.body.is_empty() {
        for (i, stmt) in f.body.iter().enumerate() {
            let s = stmt.trim();
            let is_last = i + 1 == f.body.len();
            if s.starts_with("let ") {
                if let Some(eq) = s.find(" = ") {
                    let name = s[4..eq].trim();
                    let rhs = s[eq + 3..].trim();
                    emit_wasm_expr(out, rhs);
                    out.push_str(&format!("    local.set ${}\n", name));
                }
            } else {
                emit_wasm_expr(out, s);
            }
        }
    } else {
        out.push_str("    unreachable\n");
    }

    out.push_str("  )\n");

    // Export: simple format
    out.push_str(&format!("  (export \"{}\")\n", f.name));
}

fn emit_wasm_expr(out: &mut String, expr: &str) {
    let s = expr.trim();

    // Simple identifier (local or param)
    if s.chars().all(|c| c.is_alphanumeric() || c == '_') && !s.is_empty() {
        // Check if it's an integer literal
        if s.chars().all(|c| c.is_ascii_digit() || c == '-') {
            out.push_str(&format!("    i64.const {}\n", s));
        } else {
            out.push_str(&format!("    local.get ${}\n", s));
        }
        return;
    }

    // Integer literal like 0
    if let Ok(n) = s.parse::<i64>() {
        out.push_str(&format!("    i64.const {}\n", n));
        return;
    }

    // Arithmetic: "a + b", "a - b", "a * b", "a / b"
    for (op_str, wasm_op) in &[
        (" + ", "i64.add"),
        (" - ", "i64.sub"),
        (" * ", "i64.mul"),
        (" / ", "i64.div_s"),
    ] {
        if let Some(pos) = s.find(op_str) {
            let left = s[..pos].trim();
            let right = s[pos + op_str.len()..].trim();
            emit_wasm_expr(out, left);
            emit_wasm_expr(out, right);
            out.push_str(&format!("    {}\n", wasm_op));
            return;
        }
    }

    // Comparisons: "n > 0", "n < 0", etc.
    for (op_str, wasm_op) in &[
        (" > ", "i64.gt_s"),
        (" < ", "i64.lt_s"),
        (" >= ", "i64.ge_s"),
        (" <= ", "i64.le_s"),
        (" == ", "i64.eq"),
        (" != ", "i64.ne"),
    ] {
        if let Some(pos) = s.find(op_str) {
            let left = s[..pos].trim();
            let right = s[pos + op_str.len()..].trim();
            emit_wasm_expr(out, left);
            emit_wasm_expr(out, right);
            out.push_str(&format!("    {}\n", wasm_op));
            return;
        }
    }

    // Boolean literals
    if s == "true" { out.push_str("    i32.const 1\n"); return; }
    if s == "false" { out.push_str("    i32.const 0\n"); return; }

    // todo / fallback
    out.push_str("    unreachable\n");
}

fn type_to_wasm(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(n) => match n.as_str() {
            "Int" => "i64",
            "Float" => "f64",
            "Bool" => "i32",
            "Unit" => "i32", // ALX: WASM has no void; use i32 0
            _ => "i32",      // All reference types are pointers (i32 in wasm32)
        }.into(),
        TypeExpr::Generic(n, _) => match n.as_str() {
            "Float" => "f64".into(),
            _ => "i32".into(),
        },
        TypeExpr::Effect(_, ret) => type_to_wasm(ret),
        TypeExpr::Option(_) | TypeExpr::Result(_, _) => "i32".into(),
        _ => "i32".into(),
    }
}
