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
    // Build OpenAPI 3.0.3 document as JSON

    // Info
    let mut info = serde_json::json!({
        "title": module.name,
        "version": "1.0.0"
    });
    if let Some(desc) = &module.describe {
        info["description"] = serde_json::json!(desc);
    }

    // Paths
    let mut paths = serde_json::Map::new();
    for item in &module.items {
        if let Item::Fn(f) = item {
            let (method, path) = infer_http_method_and_path(f, module);

            let has_effects = !f.effect_tiers.is_empty()
                || matches!(f.type_sig.return_type, TypeExpr::Effect(_, _));

            let mut operation = serde_json::json!({
                "operationId": f.name,
            });
            if let Some(desc) = &f.describe {
                operation["description"] = serde_json::json!(desc);
                operation["summary"] = serde_json::json!(desc);
            }

            // Annotations
            for ann in &f.annotations {
                match ann.key.as_str() {
                    "idempotent" => { operation["x-idempotent"] = serde_json::json!(true); }
                    "exactly-once" => { operation["x-exactly-once"] = serde_json::json!(true); }
                    "at-most-once" => {
                        operation["x-at-most-once"] = serde_json::json!(true);
                        operation["x-retry-policy"] = serde_json::json!("never");
                    }
                    "commutative" => { operation["x-commutative"] = serde_json::json!(true); }
                    "associative" => { operation["x-associative"] = serde_json::json!(true); }
                    _ => {}
                }
            }

            if has_effects {
                operation["x-loom-async"] = serde_json::json!(true);
            }

            // Request body for non-GET methods with params
            if matches!(method.as_str(), "post" | "put" | "patch") && !f.type_sig.params.is_empty() {
                operation["requestBody"] = serde_json::json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": type_to_openapi_schema(&f.type_sig.params[0])
                        }
                    }
                });
            }

            // Responses
            let status = if method == "post" { "201" } else { "200" };
            let ret_schema = type_to_openapi_schema(&f.type_sig.return_type);
            operation["responses"] = serde_json::json!({
                status: {
                    "description": "Success",
                    "content": {
                        "application/json": {
                            "schema": ret_schema
                        }
                    }
                }
            });

            let path_entry = paths.entry(path).or_insert_with(|| serde_json::json!({}));
            path_entry[method] = operation;
        }
    }

    // Components/schemas from types
    let mut schemas = serde_json::Map::new();
    for item in &module.items {
        match item {
            Item::Type(t) => {
                let mut props = serde_json::Map::new();
                let mut required = Vec::new();
                for field in &t.fields {
                    props.insert(field.name.clone(), type_to_openapi_schema(&field.ty));
                    required.push(serde_json::json!(field.name));
                }
                schemas.insert(t.name.clone(), serde_json::json!({
                    "type": "object",
                    "properties": props,
                    "required": required,
                }));
            }
            Item::Enum(e) => {
                let values: Vec<serde_json::Value> = e.variants.iter()
                    .map(|v| serde_json::json!(v.name))
                    .collect();
                schemas.insert(e.name.clone(), serde_json::json!({
                    "type": "string",
                    "enum": values,
                }));
            }
            _ => {}
        }
    }

    let mut doc = serde_json::json!({
        "openapi": "3.0.3",
        "info": info,
        "paths": paths,
        "components": {
            "schemas": schemas
        }
    });

    // x-beings extension
    if !module.being_defs.is_empty() {
        let beings: serde_json::Map<String, serde_json::Value> = module.being_defs.iter().map(|b| {
            let mut being_ext = serde_json::Map::new();
            if let Some(telos) = &b.telos {
                being_ext.insert("x-telos".to_string(), serde_json::json!(telos.description));
            }
            if b.autopoietic {
                being_ext.insert("x-autopoietic".to_string(), serde_json::json!(true));
            }
            (b.name.clone(), serde_json::Value::Object(being_ext))
        }).collect();
        doc["x-beings"] = serde_json::Value::Object(beings);
    }

    // x-ecosystems extension
    if !module.ecosystem_defs.is_empty() {
        let ecosystems: serde_json::Map<String, serde_json::Value> = module.ecosystem_defs.iter().map(|eco| {
            let mut eco_ext = serde_json::Map::new();
            eco_ext.insert("members".to_string(), serde_json::json!(eco.members));
            let signals: Vec<serde_json::Value> = eco.signals.iter().map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "from": s.from,
                    "to": s.to,
                    "payload": s.payload
                })
            }).collect();
            eco_ext.insert("signals".to_string(), serde_json::json!(signals));
            if let Some(telos) = &eco.telos {
                eco_ext.insert("telos".to_string(), serde_json::json!(telos));
            }
            (eco.name.clone(), serde_json::Value::Object(eco_ext))
        }).collect();
        doc["x-ecosystems"] = serde_json::Value::Object(ecosystems);
    }

    // x-security-labels extension for flow labels
    if !module.flow_labels.is_empty() {
        let mut labels_map = serde_json::Map::new();
        for fl in &module.flow_labels {
            labels_map.insert(
                fl.label.clone(),
                serde_json::json!(fl.types),
            );
        }
        doc["x-security-labels"] = serde_json::Value::Object(labels_map);
    }

    // x-lifecycle extension for lifecycle defs
    if !module.lifecycle_defs.is_empty() {
        let lifecycles: serde_json::Map<String, serde_json::Value> = module.lifecycle_defs.iter().map(|lc| {
            let mut transitions = Vec::new();
            for i in 0..lc.states.len().saturating_sub(1) {
                transitions.push(serde_json::json!({
                    "from": lc.states[i],
                    "to": lc.states[i + 1]
                }));
            }
            (lc.type_name.clone(), serde_json::json!({
                "states": lc.states,
                "transitions": transitions
            }))
        }).collect();
        doc["x-lifecycle"] = serde_json::Value::Object(lifecycles);
    }

    // x-data-protection extension for PII/privacy fields
    {
        let mut pii_fields: Vec<serde_json::Value> = Vec::new();
        for item in &module.items {
            if let Item::Type(t) = item {
                for field in &t.fields {
                    let ann_keys: Vec<&str> = field.annotations.iter().map(|a| a.key.as_str()).collect();
                    if ann_keys.contains(&"pii") || ann_keys.contains(&"gdpr") || ann_keys.contains(&"hipaa") || ann_keys.contains(&"pci") {
                        let mut entry = serde_json::Map::new();
                        entry.insert("field".to_string(), serde_json::json!(format!("{}.{}", t.name, field.name)));
                        for ann in &field.annotations {
                            entry.insert(format!("x-{}", ann.key), serde_json::json!(true));
                        }
                        pii_fields.push(serde_json::Value::Object(entry));
                    }
                }
            }
        }
        if !pii_fields.is_empty() {
            doc["x-data-protection"] = serde_json::json!(pii_fields);
        }
    }

    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn type_to_openapi_schema(ty: &TypeExpr) -> serde_json::Value {
    match ty {
        TypeExpr::Base(n) => match n.as_str() {
            "Int" => serde_json::json!({"type": "integer", "format": "int64"}),
            "Float" => serde_json::json!({"type": "number"}),
            "String" => serde_json::json!({"type": "string"}),
            "Bool" => serde_json::json!({"type": "boolean"}),
            "Unit" => serde_json::json!({"type": "null"}),
            _ => serde_json::json!({"$ref": format!("#/components/schemas/{}", n)}),
        },
        TypeExpr::Generic(n, args) => match n.as_str() {
            "List" if !args.is_empty() => serde_json::json!({
                "type": "array",
                "items": type_to_openapi_schema(&args[0])
            }),
            "Option" if !args.is_empty() => serde_json::json!({
                "oneOf": [type_to_openapi_schema(&args[0]), {"type": "null"}]
            }),
            _ => serde_json::json!({"$ref": format!("#/components/schemas/{}", n)}),
        },
        TypeExpr::Option(inner) => serde_json::json!({
            "oneOf": [type_to_openapi_schema(inner), {"type": "null"}]
        }),
        TypeExpr::Effect(_, ret) => type_to_openapi_schema(ret),
        _ => serde_json::json!({"type": "object"}),
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
    } else if !f.type_sig.params.is_empty() {
        // Default: functions with params → POST (unless @idempotent → PUT)
        if has_idempotent || has_exactly_once { "put" } else { "post" }
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
