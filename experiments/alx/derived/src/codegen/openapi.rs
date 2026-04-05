// ALX: derived from loom.loom §"emit_openapi" and language-spec.md §11 (OpenAPI REST Inference)
// - interface methods → paths
// - TypeDef → components/schemas
// - @exactly-once → POST promoted to PUT
// - @idempotent → x-idempotent: true extension
// - BeingDef → x-beings extension
// - EcosystemDef → x-ecosystems extension

use crate::ast::*;

/// G3: OpenApiEmitter struct — tests call `OpenApiEmitter::new().emit(&module)`.
pub struct OpenApiEmitter;

impl OpenApiEmitter {
    pub fn new() -> Self { OpenApiEmitter }
    pub fn emit(&self, module: &Module) -> String {
        emit_openapi(module)
    }
}

pub fn emit_openapi(module: &Module) -> String {
    let mut out = String::new();

    out.push_str("openapi: \"3.0.0\"\n");
    out.push_str("info:\n");
    out.push_str(&format!("  title: \"{}\"\n", module.name));
    if let Some(desc) = &module.describe {
        out.push_str(&format!("  description: \"{}\"\n", desc));
    }
    out.push_str("  version: \"1.0.0\"\n");

    // x-lifecycle extensions
    if !module.lifecycle_defs.is_empty() {
        out.push_str("  x-lifecycle:\n");
        for lc in &module.lifecycle_defs {
            out.push_str(&format!(
                "    {}: [{}]\n",
                lc.type_name,
                lc.states.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")
            ));
        }
    }

    // x-security-labels from flow labels
    if !module.flow_labels.is_empty() {
        out.push_str("  x-security-labels:\n");
        for fl in &module.flow_labels {
            out.push_str(&format!(
                "    {}: [{}]\n",
                fl.label,
                fl.types.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ")
            ));
        }
    }

    // Paths from functions
    out.push_str("paths:\n");
    let mut has_paths = false;
    for item in &module.items {
        if let Item::Fn(f) = item {
            let (method, path) = infer_http_method_and_path(f, module);
            out.push_str(&format!("  {}:\n", path));
            out.push_str(&format!("    {}:\n", method));
            if let Some(desc) = &f.describe {
                out.push_str(&format!("      description: \"{}\"\n", desc));
            }
            out.push_str("      operationId: \"");
            out.push_str(&f.name);
            out.push_str("\"\n");

            // Annotations
            for ann in &f.annotations {
                match ann.key.as_str() {
                    "idempotent" => out.push_str("      x-idempotent: true\n"),
                    "exactly-once" => out.push_str("      x-exactly-once: true\n"),
                    _ => {}
                }
            }

            // Request body for POST/PUT/PATCH
            if matches!(method.as_str(), "post" | "put" | "patch") && !f.type_sig.params.is_empty() {
                out.push_str("      requestBody:\n");
                out.push_str("        required: true\n");
                out.push_str("        content:\n");
                out.push_str("          application/json:\n");
                out.push_str("            schema:\n");
                out.push_str(&format!(
                    "              $ref: '#/components/schemas/{}'\n",
                    param_schema_name(f)
                ));
            }

            // Responses
            let status = if method == "post" { "201" } else { "200" };
            out.push_str("      responses:\n");
            out.push_str(&format!("        \"{}\":\n", status));
            out.push_str("          description: Success\n");
            out.push_str("          content:\n");
            out.push_str("            application/json:\n");
            out.push_str("              schema:\n");
            out.push_str(&format!(
                "                $ref: '#/components/schemas/{}'\n",
                return_schema_name(f)
            ));

            has_paths = true;
        }
    }
    if !has_paths {
        out.push_str("  {}\n");
    }

    // Components/schemas from types
    out.push_str("components:\n  schemas:\n");
    let mut has_schemas = false;
    for item in &module.items {
        match item {
            Item::Type(t) => {
                emit_openapi_type_schema(&mut out, t);
                has_schemas = true;
            }
            Item::Enum(e) => {
                emit_openapi_enum_schema(&mut out, e);
                has_schemas = true;
            }
            _ => {}
        }
    }
    if !has_schemas {
        out.push_str("    {}\n");
    }

    // x-beings extension
    if !module.being_defs.is_empty() {
        out.push_str("x-beings:\n");
        for being in &module.being_defs {
            out.push_str(&format!("  {}:\n", being.name));
            if let Some(telos) = &being.telos {
                out.push_str(&format!("    x-telos: \"{}\"\n", telos.description));
            }
            if being.autopoietic {
                out.push_str("    x-autopoietic: true\n");
            }
            if !being.regulate_blocks.is_empty() {
                out.push_str("    x-homeostasis:\n");
                for reg in &being.regulate_blocks {
                    out.push_str(&format!("      {}: {}\n", reg.variable, reg.target));
                }
            }
        }
    }

    // x-ecosystems extension
    if !module.ecosystem_defs.is_empty() {
        out.push_str("x-ecosystems:\n");
        for eco in &module.ecosystem_defs {
            out.push_str(&format!("  {}:\n", eco.name));
            if let Some(telos) = &eco.telos {
                out.push_str(&format!("    x-telos: \"{}\"\n", telos));
            }
            out.push_str(&format!(
                "    members: [{}]\n",
                eco.members.iter().map(|m| format!("\"{}\"", m)).collect::<Vec<_>>().join(", ")
            ));
        }
    }

    out
}

fn emit_openapi_type_schema(out: &mut String, t: &TypeDef) {
    out.push_str(&format!("    {}:\n", t.name));
    out.push_str("      type: object\n");
    if !t.fields.is_empty() {
        out.push_str("      properties:\n");
        for field in &t.fields {
            out.push_str(&format!("        {}:\n", field.name));
            let (schema_type, format) = type_to_json_schema_type(&field.ty);
            out.push_str(&format!("          type: {}\n", schema_type));
            if let Some(fmt) = format {
                out.push_str(&format!("          format: {}\n", fmt));
            }
            // Privacy annotations
            for ann in &field.annotations {
                match ann.key.as_str() {
                    "pci" => out.push_str("          x-pci: true\n"),
                    "hipaa" => out.push_str("          x-hipaa: true\n"),
                    "never-log" => out.push_str("          x-never-log: true\n"),
                    "pii" => out.push_str("          x-pii: true\n"),
                    _ => {}
                }
            }
        }
    }
}

fn emit_openapi_enum_schema(out: &mut String, e: &EnumDef) {
    out.push_str(&format!("    {}:\n", e.name));
    out.push_str("      type: string\n");
    out.push_str(&format!(
        "      enum: [{}]\n",
        e.variants.iter().map(|v| format!("\"{}\"", v.name)).collect::<Vec<_>>().join(", ")
    ));
}

fn type_to_json_schema_type(ty: &TypeExpr) -> (String, Option<String>) {
    match ty {
        TypeExpr::Base(n) => match n.as_str() {
            "Int" => ("integer".into(), Some("int64".into())),
            "Float" => ("number".into(), Some("double".into())),
            "String" => ("string".into(), None),
            "Bool" => ("boolean".into(), None),
            _ => ("object".into(), None),
        },
        TypeExpr::Generic(n, _) => match n.as_str() {
            "List" => ("array".into(), None),
            "Float" => ("number".into(), None),
            _ => ("object".into(), None),
        },
        _ => ("object".into(), None),
    }
}

/// Infer HTTP method and path from function name.
/// ALX: derived from language-spec.md §11 (OpenAPI REST Inference table).
fn infer_http_method_and_path(f: &FnDef, module: &Module) -> (String, String) {
    let name = f.name.to_lowercase();

    // @method override
    for ann in &f.annotations {
        if ann.key == "method" {
            let m = ann.value.trim_matches('"').to_lowercase();
            let path = infer_path(f, module);
            return (m, path);
        }
    }

    // @exactly-once on POST → PUT (language-spec.md §11: @idempotent on POST → PUT)
    let has_exactly_once = f.annotations.iter().any(|a| a.key == "exactly-once");
    let has_idempotent = f.annotations.iter().any(|a| a.key == "idempotent");

    let verb = if name.starts_with("create") || name.starts_with("add")
        || name.starts_with("register") || name.starts_with("insert")
        || name.starts_with("save") || name.starts_with("post")
    {
        if has_idempotent || has_exactly_once { "put" } else { "post" }
    } else if name.starts_with("update") || name.starts_with("set")
        || name.starts_with("put") || name.starts_with("replace")
        || name.starts_with("upsert")
    {
        "put"
    } else if name.starts_with("patch") || name.starts_with("modify")
        || name.starts_with("change")
    {
        "patch"
    } else if name.starts_with("delete") || name.starts_with("remove")
        || name.starts_with("destroy") || name.starts_with("drop")
    {
        "delete"
    } else {
        "get"
    };

    let path = infer_path(f, module);
    (verb.into(), path)
}

fn infer_path(f: &FnDef, module: &Module) -> String {
    // @path override
    for ann in &f.annotations {
        if ann.key == "path" {
            return ann.value.trim_matches('"').to_string();
        }
    }

    // Infer resource name from return type
    let resource = infer_resource_name(f, module);
    let base = format!("/{}", resource.to_lowercase());

    // If has Int param with id-like name → {id}
    let has_id_param = f.type_sig.params.iter().any(|p| matches!(p, TypeExpr::Base(n) if n == "Int"));
    if has_id_param {
        format!("{}/{{id}}", base)
    } else {
        base
    }
}

fn infer_resource_name(f: &FnDef, module: &Module) -> String {
    // From return type
    let ret = match &f.type_sig.return_type {
        TypeExpr::Base(n) if n != "Unit" && n != "Bool" && n != "String" && n != "Int" && n != "Float" => {
            return n.clone();
        }
        TypeExpr::Generic(n, _) if n != "List" && n != "Option" && n != "Result" => {
            return n.clone();
        }
        TypeExpr::Generic(_, args) if !args.is_empty() => {
            if let TypeExpr::Base(n) = &args[0] {
                return n.clone();
            }
            String::new()
        }
        TypeExpr::Effect(_, inner) => {
            return infer_resource_from_type(inner);
        }
        _ => String::new(),
    };

    if !ret.is_empty() {
        return ret;
    }

    // Fall back to module name
    module.name.trim_end_matches("Service")
        .trim_end_matches("Controller")
        .trim_end_matches("Handler")
        .to_string()
}

fn infer_resource_from_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(n) => n.clone(),
        TypeExpr::Generic(_, args) if !args.is_empty() => infer_resource_from_type(&args[0]),
        TypeExpr::Option(inner) | TypeExpr::Effect(_, inner) => infer_resource_from_type(inner),
        _ => "resource".into(),
    }
}

fn param_schema_name(f: &FnDef) -> String {
    f.type_sig.params.first()
        .and_then(|p| match p {
            TypeExpr::Base(n) => Some(n.clone()),
            TypeExpr::Generic(n, _) => Some(n.clone()),
            _ => None,
        })
        .unwrap_or_else(|| format!("{}Request", f.name))
}

fn return_schema_name(f: &FnDef) -> String {
    match &f.type_sig.return_type {
        TypeExpr::Base(n) => n.clone(),
        TypeExpr::Generic(n, args) if (n == "Option" || n == "List") && !args.is_empty() => {
            match &args[0] {
                TypeExpr::Base(n) => n.clone(),
                _ => n.clone(),
            }
        }
        TypeExpr::Effect(_, inner) => return_schema_name_from_type(inner),
        _ => format!("{}Response", f.name),
    }
}

fn return_schema_name_from_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(n) => n.clone(),
        TypeExpr::Generic(_, args) if !args.is_empty() => return_schema_name_from_type(&args[0]),
        TypeExpr::Option(inner) => return_schema_name_from_type(inner),
        _ => "Response".into(),
    }
}
