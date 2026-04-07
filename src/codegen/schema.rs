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
        // Build sensitivity map from flow_labels.
        let sensitivity_map: std::collections::HashMap<String, String> = module.flow_labels.iter()
            .flat_map(|fl| fl.types.iter().map(move |t| (t.clone(), fl.label.clone())))
            .collect();

        let mut defs: Vec<String> = Vec::new();

        for item in &module.items {
            match item {
                Item::Type(td) => {
                    let mut schema = self.emit_type_def(td);
                    if let Some(label) = sensitivity_map.get(&td.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    defs.push(format!("    {:?}: {}", td.name, schema));
                }
                Item::Enum(ed) => {
                    let mut schema = self.emit_enum_def(ed);
                    if let Some(label) = sensitivity_map.get(&ed.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    defs.push(format!("    {:?}: {}", ed.name, schema));
                }
                Item::RefinedType(rt) => {
                    let mut schema = self.emit_refined_type(rt);
                    if let Some(label) = sensitivity_map.get(&rt.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    defs.push(format!("    {:?}: {}", rt.name, schema));
                }
                Item::Fn(_) => {}
                _ => {}
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
            .map(|f| {
                let mut schema = self.type_expr_to_schema(&f.ty);
                // Inject x-privacy extensions when annotations are present.
                if !f.annotations.is_empty() {
                    // Strip trailing `}` and append extension properties.
                    if schema.ends_with('}') {
                        schema.pop();
                        for ann in &f.annotations {
                            schema.push_str(&format!(", \"x-{}\": true", ann.key));
                        }
                        schema.push('}');
                    }
                }
                format!("        {:?}: {}", f.name, schema)
            })
            .collect();
        let required: Vec<String> = td.fields.iter()
            .map(|f| format!("{:?}", f.name))
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
        let constraints = extract_constraints(&rt.predicate);
        if constraints.is_empty() {
            format!(
                "{{\"allOf\":[{}],\"description\":\"Refined type: {}\"}}",
                base, rt.name
            )
        } else {
            // Merge base schema with extracted constraints
            let base_trimmed = base.trim_start_matches('{').trim_end_matches('}');
            let constraint_str = constraints.join(",");
            format!(
                "{{{},{}}}",
                base_trimmed, constraint_str
            )
        }
    }

    // ── Type expressions → inline schema ─────────────────────────────────

    pub fn type_expr_to_schema(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Base(name) => self.base_type_schema(name),
            TypeExpr::Generic(name, params) => {
                // Unit-annotated primitives: Float<usd> → {"type":"number","x-unit":"usd"}
                if (name == "Float" || name == "Int") && params.len() == 1 {
                    if let TypeExpr::Base(unit) = &params[0] {
                        let base = if name == "Int" { "integer" } else { "number" };
                        return format!("{{\"type\":{:?},\"x-unit\":{:?}}}", base, unit.as_str());
                    }
                }
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
            TypeExpr::Dynamic => "{\"type\":\"object\"}".to_string(),
            TypeExpr::TypeVar(_) => "{\"type\":\"object\"}".to_string(),
            // Tensor<rank, shape, unit> — emit as nested JSON Schema array.
            TypeExpr::Tensor { unit, .. } => {
                let items = self.type_expr_to_schema(unit);
                format!("{{\"type\":\"array\",\"items\":{}}}", items)
            }
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

/// Inject an `"x-sensitivity"` extension into a JSON schema string.
fn inject_x_sensitivity(schema: String, label: &str) -> String {
    if schema.ends_with('}') {
        let mut s = schema;
        s.pop();
        s.push_str(&format!(", \"x-sensitivity\": {:?}}}", label));
        s
    } else {
        schema
    }
}

// ── Constraint extraction from refinement predicates ─────────────────────────

/// Extracts JSON Schema constraints from a refinement predicate expression.
///
/// Recognizes patterns like `self >= N`, `self <= N`, `self > N`, `self < N`
/// and emits corresponding `"minimum"`, `"maximum"`, `"exclusiveMinimum"`,
/// `"exclusiveMaximum"` JSON Schema keywords.
fn extract_constraints(predicate: &Expr) -> Vec<String> {
    let mut constraints = Vec::new();
    collect_constraints(predicate, &mut constraints);
    constraints
}

fn collect_constraints(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::BinOp { op, left, right, .. } => {
            match op {
                BinOpKind::And => {
                    collect_constraints(left, out);
                    collect_constraints(right, out);
                }
                BinOpKind::Ge if is_self(left) => {
                    if let Some(n) = extract_int_literal(right) {
                        out.push(format!("\"minimum\":{}", n));
                    }
                }
                BinOpKind::Le if is_self(left) => {
                    if let Some(n) = extract_int_literal(right) {
                        out.push(format!("\"maximum\":{}", n));
                    }
                }
                BinOpKind::Gt if is_self(left) => {
                    if let Some(n) = extract_int_literal(right) {
                        out.push(format!("\"exclusiveMinimum\":{}", n));
                    }
                }
                BinOpKind::Lt if is_self(left) => {
                    if let Some(n) = extract_int_literal(right) {
                        out.push(format!("\"exclusiveMaximum\":{}", n));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn is_self(expr: &Expr) -> bool {
    matches!(expr, Expr::Ident(name) if name == "self")
}

fn extract_int_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Literal(Literal::Int(n)) => Some(*n),
        // Handle unary minus: 0 - n
        Expr::BinOp { op: BinOpKind::Sub, left, right, .. } => {
            if let (Expr::Literal(Literal::Int(0)), Expr::Literal(Literal::Int(n))) =
                (left.as_ref(), right.as_ref())
            {
                Some(-n)
            } else {
                None
            }
        }
        _ => None,
    }
}
