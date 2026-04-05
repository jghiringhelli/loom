//! JSON Schema (draft 2020-12) emitter for the Loom compiler.
//!
//! Translates Loom type/enum/refined-type definitions into a JSON Schema document
//! containing `$defs` — one entry per named type in the module.
//!
//! # Mapping summary
//!
//! | Loom construct | JSON Schema |
//! |---|---|
//! | `type Point = x: Float, y: Float end` | `{"type":"object","properties":{"x":{"type":"number"},...},"required":["x","y"]}` |
//! | `enum Color = \| Red \| Green end` | `{"oneOf":[{"const":"Red"},{"const":"Green"}]}` |
//! | `enum Shape = \| Circle of Float end` | `{"oneOf":[{"type":"object","properties":{"tag":{"const":"Circle"},"value":{"type":"number"}}},...]}` |
//! | `type Email = String where pred end` | `{"type":"string","description":"Refined Email"}` |
//! | `Int` | `{"type":"integer"}` |
//! | `Float` | `{"type":"number"}` |
//! | `String` | `{"type":"string"}` |
//! | `Bool` | `{"type":"boolean"}` |
//! | `Option<T>` | `{"oneOf":[<T-schema>,{"type":"null"}]}` |
//! | `Result<T,E>` | `{"oneOf":[{"type":"object","properties":{"ok":<T>}},{"type":"object","properties":{"err":<E>}}]}` |
//! | `List<T>` | `{"type":"array","items":<T-schema>}` |
//! | `Map<K,V>` | `{"type":"object","additionalProperties":<V-schema>}` |

use crate::ast::*;

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Stateless JSON Schema emitter.
pub struct JsonSchemaEmitter;

impl JsonSchemaEmitter {
    pub fn new() -> Self { JsonSchemaEmitter }

    /// Emit a full JSON Schema document for all types in a [`Module`].
    ///
    /// Returns a pretty-printed JSON string with a `$defs` section containing
    /// one schema per type/enum/refined-type definition.
    pub fn emit(&self, module: &Module) -> String {
        let mut defs: Vec<String> = Vec::new();

        for item in &module.items {
            match item {
                Item::Type(td)        => defs.push(format!("    {:?}: {}", td.name, self.emit_type_def(td))),
                Item::Enum(ed)        => defs.push(format!("    {:?}: {}", ed.name, self.emit_enum_def(ed))),
                Item::RefinedType(rt) => defs.push(format!("    {:?}: {}", rt.name, self.emit_refined_type(rt))),
                Item::Fn(_)           => {}
            }
        }

        let description = module.describe.as_deref().unwrap_or(&module.name);

        if defs.is_empty() {
            return format!(
                "{{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"$id\": {:?},\n  \"description\": {:?},\n  \"$defs\": {{}}\n}}",
                module.name, description
            );
        }

        format!(
            "{{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"$id\": {:?},\n  \"description\": {:?},\n  \"$defs\": {{\n{}\n  }}\n}}",
            module.name,
            description,
            defs.join(",\n")
        )
    }

    // ── Type definitions ──────────────────────────────────────────────────

    pub fn emit_type_def_pub(&self, td: &TypeDef) -> String { self.emit_type_def(td) }
    pub fn emit_enum_def_pub(&self, ed: &EnumDef) -> String { self.emit_enum_def(ed) }
    pub fn emit_refined_type_pub(&self, rt: &RefinedType) -> String { self.emit_refined_type(rt) }

    fn emit_type_def(&self, td: &TypeDef) -> String {
        let props: Vec<String> = td.fields.iter()
            .map(|(name, ty)| format!("        {:?}: {}", name, self.type_expr_to_schema(ty)))
            .collect();
        let required: Vec<String> = td.fields.iter()
            .map(|(name, _)| format!("{:?}", name))
            .collect();
        format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            props.join(", "),
            required.join(", ")
        )
    }

    fn emit_enum_def(&self, ed: &EnumDef) -> String {
        let variants: Vec<String> = ed.variants.iter().map(|v| {
            match &v.payload {
                None => format!("{{\"const\":{:?}}}", v.name),
                Some(ty) => format!(
                    "{{\"type\":\"object\",\"properties\":{{\"tag\":{{\"const\":{:?}}},\"value\":{}}},\"required\":[\"tag\",\"value\"]}}",
                    v.name,
                    self.type_expr_to_schema(ty)
                ),
            }
        }).collect();

        if variants.len() == 1 {
            variants.into_iter().next().unwrap()
        } else {
            format!("{{\"oneOf\":[{}]}}", variants.join(", "))
        }
    }

    fn emit_refined_type(&self, rt: &RefinedType) -> String {
        let base = self.type_expr_to_schema(&rt.base_type);
        // Embed the base schema and add a description noting it is refined.
        format!(
            "{{\"allOf\":[{}],\"description\":\"Refined type: {}\"}}",
            base, rt.name
        )
    }

    // ── Type expressions → inline schema ─────────────────────────────────

    pub fn type_expr_to_schema(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Base(name) => self.base_type_schema(name),
            TypeExpr::Generic(name, params) => {
                let ps: Vec<String> = params.iter().map(|p| self.type_expr_to_schema(p)).collect();
                match name.as_str() {
                    "List" if ps.len() == 1 =>
                        format!("{{\"type\":\"array\",\"items\":{}}}", ps[0]),
                    "Map" if ps.len() == 2 =>
                        format!("{{\"type\":\"object\",\"additionalProperties\":{}}}", ps[1]),
                    "Set" if ps.len() == 1 =>
                        format!("{{\"type\":\"array\",\"uniqueItems\":true,\"items\":{}}}", ps[0]),
                    _ =>
                        format!("{{\"$ref\":\"#/$defs/{}\"}}", name),
                }
            }
            TypeExpr::Option(inner) => format!(
                "{{\"oneOf\":[{},{{\"type\":\"null\"}}]}}",
                self.type_expr_to_schema(inner)
            ),
            TypeExpr::Result(ok, err) => format!(
                "{{\"oneOf\":[{{\"type\":\"object\",\"properties\":{{\"ok\":{}}},\"required\":[\"ok\"]}},{{\"type\":\"object\",\"properties\":{{\"err\":{}}},\"required\":[\"err\"]}}]}}",
                self.type_expr_to_schema(ok),
                self.type_expr_to_schema(err)
            ),
            TypeExpr::Tuple(elems) => {
                let items: Vec<String> = elems.iter().map(|e| self.type_expr_to_schema(e)).collect();
                format!("{{\"type\":\"array\",\"prefixItems\":[{}],\"minItems\":{},\"maxItems\":{}}}",
                    items.join(", "), items.len(), items.len())
            }
            TypeExpr::Effect(_, inner) => self.type_expr_to_schema(inner),
            TypeExpr::TypeVar(_) => "{\"type\":\"object\"}".to_string(),
        }
    }

    fn base_type_schema(&self, name: &str) -> String {
        match name {
            "Int"  => "{\"type\":\"integer\"}".to_string(),
            "Float" => "{\"type\":\"number\"}".to_string(),
            "String" | "Str" => "{\"type\":\"string\"}".to_string(),
            "Bool"  => "{\"type\":\"boolean\"}".to_string(),
            "Unit"  => "{\"type\":\"null\"}".to_string(),
            other   => format!("{{\"$ref\":\"#/$defs/{}\"}}", other),
        }
    }
}
