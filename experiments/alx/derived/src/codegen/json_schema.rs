// ALX: derived from loom.loom §"emit_json_schema"
// JSON Schema draft-07 emitter.
// TypeDef and EnumDef → $schema draft-07 objects.

use crate::ast::*;

/// G3: JsonSchemaEmitter struct — tests call `JsonSchemaEmitter::new().emit(&module)`.
pub struct JsonSchemaEmitter;

impl JsonSchemaEmitter {
    pub fn new() -> Self { JsonSchemaEmitter }
    pub fn emit(&self, module: &Module) -> String {
        emit_json_schema(module)
    }
}

pub fn emit_json_schema(module: &Module) -> String {
    let mut schemas = serde_json::Map::new();

    for item in &module.items {
        match item {
            Item::Type(t) => {
                schemas.insert(t.name.clone(), type_def_to_schema(t));
            }
            Item::Enum(e) => {
                schemas.insert(e.name.clone(), enum_def_to_schema(e));
            }
            Item::RefinedType(r) => {
                schemas.insert(r.name.clone(), refined_to_schema(r));
            }
            _ => {}
        }
    }

    let root = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": module.name,
        "definitions": schemas,
    });

    serde_json::to_string_pretty(&root).unwrap_or_default()
}

fn type_def_to_schema(t: &TypeDef) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for field in &t.fields {
        let mut schema = type_expr_to_schema(&field.ty);
        // Privacy annotations
        let ann_keys: Vec<&str> = field.annotations.iter().map(|a| a.key.as_str()).collect();
        if ann_keys.contains(&"pci") {
            schema["x-pci"] = serde_json::json!(true);
            schema["x-never-log"] = serde_json::json!(true);
        }
        if ann_keys.contains(&"hipaa") {
            schema["x-hipaa"] = serde_json::json!(true);
        }
        if ann_keys.contains(&"pii") {
            schema["x-pii"] = serde_json::json!(true);
        }
        if ann_keys.contains(&"never-log") {
            schema["x-never-log"] = serde_json::json!(true);
        }
        properties.insert(field.name.clone(), schema);
        required.push(serde_json::json!(field.name));
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn enum_def_to_schema(e: &EnumDef) -> serde_json::Value {
    // Simple string enum (unit variants only) or oneOf with payloads
    let all_unit = e.variants.iter().all(|v| v.payload.is_none());
    if all_unit {
        let values: Vec<serde_json::Value> = e.variants.iter()
            .map(|v| serde_json::json!(v.name))
            .collect();
        serde_json::json!({ "type": "string", "enum": values })
    } else {
        let one_of: Vec<serde_json::Value> = e.variants.iter().map(|v| {
            match &v.payload {
                None => serde_json::json!({ "const": v.name }),
                Some(ty) => serde_json::json!({
                    "type": "object",
                    "properties": {
                        v.name.clone(): type_expr_to_schema(ty)
                    },
                    "required": [v.name]
                }),
            }
        }).collect();
        serde_json::json!({ "oneOf": one_of })
    }
}

fn refined_to_schema(r: &RefinedType) -> serde_json::Value {
    let mut schema = type_expr_to_schema(&r.base_type);
    schema["description"] = serde_json::json!(format!("Refined: {}", r.predicate));
    schema
}

fn type_expr_to_schema(ty: &TypeExpr) -> serde_json::Value {
    match ty {
        TypeExpr::Base(n) => match n.as_str() {
            "Int" => serde_json::json!({ "type": "integer", "format": "int64" }),
            "Float" => serde_json::json!({ "type": "number" }),
            "String" => serde_json::json!({ "type": "string" }),
            "Bool" => serde_json::json!({ "type": "boolean" }),
            "Unit" => serde_json::json!({ "type": "null" }),
            _ => serde_json::json!({ "$ref": format!("#/definitions/{}", n) }),
        },
        TypeExpr::Generic(n, args) => match n.as_str() {
            "List" => serde_json::json!({
                "type": "array",
                "items": type_expr_to_schema(&args[0]),
            }),
            "Option" => serde_json::json!({
                "oneOf": [type_expr_to_schema(&args[0]), { "type": "null" }]
            }),
            "Float" if args.len() == 1 => {
                if let TypeExpr::Base(unit) = &args[0] {
                    serde_json::json!({ "type": "number", "x-unit": unit })
                } else {
                    serde_json::json!({ "type": "number" })
                }
            }
            _ => serde_json::json!({ "$ref": format!("#/definitions/{}", n) }),
        },
        TypeExpr::Option(inner) => serde_json::json!({
            "oneOf": [type_expr_to_schema(inner), { "type": "null" }]
        }),
        TypeExpr::Tuple(types) => serde_json::json!({
            "type": "array",
            "prefixItems": types.iter().map(type_expr_to_schema).collect::<Vec<_>>(),
            "items": false,
        }),
        _ => serde_json::json!({ "type": "object" }),
    }
}
