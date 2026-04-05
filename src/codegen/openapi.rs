//! OpenAPI 3.0.3 emitter for the Loom compiler.
//!
//! Translates a Loom [`Module`] into an OpenAPI 3.0.3 JSON document.
//!
//! # Path / method inference
//!
//! For each `fn` in the module (or in `provides:` if present), an operation is
//! emitted.  The path and HTTP method are controlled by annotations:
//!
//! | Annotation | Effect |
//! |---|---|
//! | `@path("/route")` | Sets the path (default: `/<module-name>/<fn-name>`) |
//! | `@method("GET")` | Sets the HTTP method (default: `post`) |
//! | `@tag("Users")` | Adds the operation to a tag group |
//!
//! Functions with zero parameters emit a GET with no request body.
//! Functions returning `Effect<[…], T>` have their response marked `async: true`
//! in the extension field `x-loom-async`.
//!
//! # Components / schemas
//!
//! All `type`, `enum`, and refined-type definitions in the module are emitted
//! into `components/schemas` using the JSON Schema emitter.

use crate::ast::*;
use crate::codegen::schema::JsonSchemaEmitter;

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Stateless OpenAPI 3.0.3 emitter.
pub struct OpenApiEmitter;

impl OpenApiEmitter {
    pub fn new() -> Self { OpenApiEmitter }

    /// Emit a complete OpenAPI 3.0.3 JSON document for a [`Module`].
    pub fn emit(&self, module: &Module) -> String {
        let schema_emitter = JsonSchemaEmitter::new();

        let title = &module.name;
        let description = module.describe.as_deref().unwrap_or(title);

        // ── paths ─────────────────────────────────────────────────────────
        let fns: Vec<&FnDef> = module.items.iter().filter_map(|i| {
            if let Item::Fn(fd) = i { Some(fd) } else { None }
        }).collect();

        // Group operations by path (multiple methods on same path are merged).
        let mut path_map: Vec<(String, String, String)> = Vec::new(); // (path, method, op_json)
        for fd in &fns {
            let (path, method, op) = self.emit_operation(fd, module);
            path_map.push((path, method, op));
        }

        // Merge same paths.
        let mut merged_paths: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for (path, method, op) in path_map {
            if let Some(entry) = merged_paths.iter_mut().find(|(p, _)| p == &path) {
                entry.1.push((method, op));
            } else {
                merged_paths.push((path, vec![(method, op)]));
            }
        }

        let paths_json: Vec<String> = merged_paths.iter().map(|(path, ops)| {
            let ops_json: Vec<String> = ops.iter()
                .map(|(method, op)| format!("      {:?}: {}", method, op))
                .collect();
            format!("    {:?}: {{\n{}\n    }}", path, ops_json.join(",\n"))
        }).collect();

        // ── components/schemas ────────────────────────────────────────────
        let mut schemas: Vec<String> = Vec::new();
        for item in &module.items {
            match item {
                Item::Type(td) => schemas.push(format!(
                    "      {:?}: {}",
                    td.name,
                    schema_emitter.emit_type_def_pub(td)
                )),
                Item::Enum(ed) => schemas.push(format!(
                    "      {:?}: {}",
                    ed.name,
                    schema_emitter.emit_enum_def_pub(ed)
                )),
                Item::RefinedType(rt) => schemas.push(format!(
                    "      {:?}: {}",
                    rt.name,
                    schema_emitter.emit_refined_type_pub(rt)
                )),
                Item::Fn(_) => {}
            }
        }

        // ── assemble document ─────────────────────────────────────────────
        let paths_section = if paths_json.is_empty() {
            "  \"paths\": {}".to_string()
        } else {
            format!("  \"paths\": {{\n{}\n  }}", paths_json.join(",\n"))
        };

        let schemas_section = if schemas.is_empty() {
            "  \"components\": {\"schemas\": {}}".to_string()
        } else {
            format!("  \"components\": {{\n    \"schemas\": {{\n{}\n    }}\n  }}", schemas.join(",\n"))
        };

        format!(
            "{{\n  \"openapi\": \"3.0.3\",\n  \"info\": {{\n    \"title\": {:?},\n    \"description\": {:?},\n    \"version\": \"1.0.0\"\n  }},\n{},\n{}\n}}",
            title,
            description,
            paths_section,
            schemas_section
        )
    }

    // ── Operation emission ────────────────────────────────────────────────

    fn emit_operation(&self, fd: &FnDef, module: &Module) -> (String, String, String) {
        let schema_emitter = JsonSchemaEmitter::new();

        // Resolve path + method from annotations.
        let module_slug = to_kebab_case(&module.name);
        let fn_slug = to_kebab_case(&fd.name);
        let default_path = format!("/{}/{}", module_slug, fn_slug);

        let path = annotation_value(&fd.annotations, "path")
            .map(|v| v.to_string())
            .unwrap_or(default_path);

        let has_params = !fd.type_sig.params.is_empty();
        let default_method = if has_params { "post" } else { "get" };
        let method = annotation_value(&fd.annotations, "method")
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|| default_method.to_string());

        let tag = annotation_value(&fd.annotations, "tag");
        let description = fd.describe.as_deref().unwrap_or(&fd.name);

        let is_async = matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _));

        // ── request body ──────────────────────────────────────────────────
        let param_names = op_param_names(fd);
        let request_body_json = if has_params && method != "get" && method != "head" {
            let props: Vec<String> = fd.type_sig.params.iter()
                .zip(param_names.iter())
                .map(|(ty, name)| format!("          {:?}: {}", name, schema_emitter.type_expr_to_schema(ty)))
                .collect();
            let required: Vec<String> = param_names.iter()
                .map(|n| format!("{:?}", n))
                .collect();
            Some(format!(
                "{{\n        \"required\": true,\n        \"content\": {{\n          \"application/json\": {{\n            \"schema\": {{\n              \"type\": \"object\",\n              \"properties\": {{{}}},\n              \"required\": [{}]\n            }}\n          }}\n        }}\n      }}",
                props.join(", "),
                required.join(", ")
            ))
        } else if has_params {
            // GET with params → query parameters list (simplified: just names)
            None
        } else {
            None
        };

        // ── response schema ───────────────────────────────────────────────
        let ret_ty = match fd.type_sig.return_type.as_ref() {
            TypeExpr::Effect(_, inner) => inner.as_ref(),
            ty => ty,
        };
        let response_schema = schema_emitter.type_expr_to_schema(ret_ty);

        // ── operation JSON ────────────────────────────────────────────────
        let mut op_parts: Vec<String> = Vec::new();
        op_parts.push(format!("        \"operationId\": {:?}", fd.name));
        op_parts.push(format!("        \"summary\": {:?}", description));
        if let Some(t) = tag {
            op_parts.push(format!("        \"tags\": [{:?}]", t));
        }
        if is_async {
            op_parts.push("        \"x-loom-async\": true".to_string());
        }
        if let Some(rb) = request_body_json {
            op_parts.push(format!("        \"requestBody\": {}", rb));
        }
        op_parts.push(format!(
            "        \"responses\": {{\n          \"200\": {{\n            \"description\": \"Success\",\n            \"content\": {{\n              \"application/json\": {{\n                \"schema\": {}\n              }}\n            }}\n          }}\n        }}",
            response_schema
        ));

        let op_json = format!("{{\n{}\n      }}", op_parts.join(",\n"));
        (path, method, op_json)
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn annotation_value<'a>(annotations: &'a [Annotation], key: &str) -> Option<&'a str> {
    annotations.iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
        .filter(|v| !v.is_empty())
}

fn to_kebab_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

fn op_param_names(fd: &FnDef) -> Vec<String> {
    use std::collections::HashSet;
    let max = fd.type_sig.params.len();
    if max == 0 { return vec![]; }

    let mut let_bound: HashSet<String> = HashSet::new();
    for expr in &fd.body {
        collect_lets(expr, &mut let_bound);
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut ordered: Vec<String> = Vec::new();

    let all: Vec<&Expr> = fd.body.iter()
        .chain(fd.requires.iter().map(|c| &c.expr))
        .chain(fd.ensures.iter().map(|c| &c.expr))
        .collect();

    for expr in all {
        scan_idents(expr, &let_bound, &mut seen, &mut ordered);
        if ordered.len() >= max { break; }
    }

    (0..max)
        .map(|i| ordered.get(i).cloned().unwrap_or_else(|| format!("arg{}", i)))
        .collect()
}

fn collect_lets(expr: &Expr, out: &mut std::collections::HashSet<String>) {
    if let Expr::Let { name, value, .. } = expr {
        out.insert(name.clone());
        collect_lets(value, out);
    }
}

fn scan_idents(
    expr: &Expr,
    let_bound: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
    ordered: &mut Vec<String>,
) {
    const BUILTINS: &[&str] = &["todo", "map", "filter", "fold", "true", "false"];
    match expr {
        Expr::Ident(name) => {
            if !let_bound.contains(name) && !BUILTINS.contains(&name.as_str()) && seen.insert(name.clone()) {
                ordered.push(name.clone());
            }
        }
        Expr::BinOp { left, right, .. } => {
            scan_idents(left, let_bound, seen, ordered);
            scan_idents(right, let_bound, seen, ordered);
        }
        Expr::Call { func, args, .. } => {
            scan_idents(func, let_bound, seen, ordered);
            for a in args { scan_idents(a, let_bound, seen, ordered); }
        }
        Expr::Let { value, .. } => scan_idents(value, let_bound, seen, ordered),
        Expr::FieldAccess { object, .. } => scan_idents(object, let_bound, seen, ordered),
        Expr::Pipe { left, right, .. } => {
            scan_idents(left, let_bound, seen, ordered);
            scan_idents(right, let_bound, seen, ordered);
        }
        Expr::Lambda { body, .. } => scan_idents(body, let_bound, seen, ordered),
        Expr::Match { subject, arms, .. } => {
            scan_idents(subject, let_bound, seen, ordered);
            for arm in arms { scan_idents(&arm.body, let_bound, seen, ordered); }
        }
        Expr::Tuple(elems, _) => {
            for e in elems { scan_idents(e, let_bound, seen, ordered); }
        }
        Expr::Try(inner, _) | Expr::As(inner, _) => scan_idents(inner, let_bound, seen, ordered),
        Expr::ForIn { iter, body, .. } => {
            scan_idents(iter, let_bound, seen, ordered);
            scan_idents(body, let_bound, seen, ordered);
        }
        Expr::Literal(_) | Expr::InlineRust(_) => {}
    }
}
