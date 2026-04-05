//! OpenAPI 3.0.3 emitter for the Loom compiler — full REST semantic inference.
//!
//! Unlike annotation-driven emitters, this derives HTTP verbs, resource paths,
//! path parameters, request bodies, and error responses **directly from the
//! Loom type signatures and function names** — no annotations required.
//!
//! # REST inference rules
//!
//! ## HTTP verb
//!
//! Priority: `@method("X")` annotation → function name prefix → type signature.
//!
//! | Name prefix | Verb |
//! |---|---|
//! | `create`, `add`, `register`, `post`, `insert`, `save` | `POST` |
//! | `update`, `set`, `put`, `replace`, `upsert` | `PUT` |
//! | `patch`, `modify`, `change` | `PATCH` |
//! | `delete`, `remove`, `destroy`, `drop` | `DELETE` |
//! | `get`, `fetch`, `find`, `load`, `read`, `show`, `by` | `GET` |
//! | `list`, `all`, `search`, `query`, `index`, `browse` | `GET` (collection) |
//! | returns `List<T>` | `GET` (collection) |
//! | no params | `GET` |
//! | otherwise | `POST` |
//!
//! ## Resource path
//!
//! Priority: `@path(...)` annotation → inferred from return/param types → fn name suffix → module name.
//!
//! `fn get_order :: Int -> Effect<[IO], Order>` → resource `order` → path `/orders/{id}`
//!
//! ## Path parameters
//!
//! A parameter becomes `{name}` when:
//! - Its inferred name contains `id` (e.g. `user_id`, `id`)
//! - OR the verb is GET/DELETE and it is a primitive scalar (Int or String)
//!
//! ## Request body
//!
//! POST/PUT/PATCH with non-path parameters → JSON request body object schema.
//!
//! ## Responses
//!
//! | Condition | Status |
//! |---|---|
//! | Normal return (unwrapped from Effect) | 200 |
//! | Returns `List<T>` | 200 array |
//! | Has matching `XError` enum with `NotFound` variant | 404 |
//! | Has matching `XError` enum with `InvalidInput`/`Validation` | 400 |
//! | Has matching `XError` enum with `PermissionDenied`/`Unauthorized` | 403 |
//! | Effectful (`Effect<[IO]…>`) | 500 (server error schema) |
//! | `Result<T, E>` return | 200 T + 422 E |

use crate::ast::*;
use crate::codegen::schema::JsonSchemaEmitter;

// ── HTTP verb ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Verb { Get, Post, Put, Patch, Delete }

impl Verb {
    fn as_str(&self) -> &'static str {
        match self {
            Verb::Get    => "get",
            Verb::Post   => "post",
            Verb::Put    => "put",
            Verb::Patch  => "patch",
            Verb::Delete => "delete",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        let n = name.to_lowercase();
        let prefix = n.split('_').next().unwrap_or(&n);
        match prefix {
            "create" | "add" | "register" | "post" | "insert" | "save" | "submit" | "publish" => Some(Verb::Post),
            "update" | "set" | "put" | "replace" | "upsert" | "store" => Some(Verb::Put),
            "patch"  | "modify" | "change" | "edit" => Some(Verb::Patch),
            "delete" | "remove" | "destroy" | "drop" | "purge" | "revoke" => Some(Verb::Delete),
            "get" | "fetch" | "find" | "load" | "read" | "show" | "by" | "lookup" => Some(Verb::Get),
            "list" | "all" | "search" | "query" | "index" | "browse" | "filter" => Some(Verb::Get),
            _ => None,
        }
    }

    fn takes_body(&self) -> bool {
        matches!(self, Verb::Post | Verb::Put | Verb::Patch)
    }
}

// ── REST analysis ─────────────────────────────────────────────────────────────

struct RestOp {
    verb: Verb,
    path: String,
    path_params: Vec<(String, String)>,   // (param_name, json_type)
    request_body: Option<String>,          // JSON fragment
    responses: Vec<(u16, String, String)>, // (status, description, schema_json)
    operation_id: String,
    summary: String,
    tags: Vec<String>,
    is_async: bool,
    // Algebraic properties
    is_idempotent: bool,
    is_commutative: bool,
    is_associative: bool,
    is_at_most_once: bool,
    is_exactly_once: bool,
    is_monotonic: bool,
}

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Stateless OpenAPI 3.0.3 emitter with full REST semantic inference.
pub struct OpenApiEmitter;

impl OpenApiEmitter {
    pub fn new() -> Self { OpenApiEmitter }

    /// Emit a complete OpenAPI 3.0.3 JSON document for a [`Module`].
    pub fn emit(&self, module: &Module) -> String {
        let schema_emitter = JsonSchemaEmitter::new();

        let title = &module.name;
        let description = module.describe.as_deref().unwrap_or(title);

        // Collect type names defined in this module (for resource detection).
        let type_names: Vec<String> = module.items.iter().filter_map(|i| match i {
            Item::Type(td) => Some(td.name.clone()),
            _ => None,
        }).collect();

        // Collect error enums (XError, XException patterns) for response inference.
        let error_enums: Vec<&EnumDef> = module.items.iter().filter_map(|i| {
            if let Item::Enum(ed) = i {
                if ed.name.ends_with("Error") || ed.name.ends_with("Exception") || ed.name.ends_with("Fault") {
                    Some(ed)
                } else { None }
            } else { None }
        }).collect();

        // Analyse each fn.
        let fns: Vec<&FnDef> = module.items.iter().filter_map(|i| {
            if let Item::Fn(fd) = i { Some(fd) } else { None }
        }).collect();

        let mut ops: Vec<RestOp> = fns.iter()
            .map(|fd| self.analyse(fd, module, &type_names, &error_enums))
            .collect();

        // Merge same-path operations (e.g. GET + DELETE on /orders/{id}).
        let mut path_groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (i, op) in ops.iter().enumerate() {
            if let Some(g) = path_groups.iter_mut().find(|(p, _)| p == &op.path) {
                g.1.push(i);
            } else {
                path_groups.push((op.path.clone(), vec![i]));
            }
        }

        // ── paths JSON ────────────────────────────────────────────────────
        let paths_entries: Vec<String> = path_groups.iter().map(|(path, indices)| {
            let method_entries: Vec<String> = indices.iter().map(|&i| {
                let op = &ops[i];
                format!("      {:?}: {}", op.verb.as_str(), self.render_operation(op))
            }).collect();
            format!("    {:?}: {{\n{}\n    }}", path, method_entries.join(",\n"))
        }).collect();

        let paths_section = if paths_entries.is_empty() {
            "  \"paths\": {}".to_string()
        } else {
            format!("  \"paths\": {{\n{}\n  }}", paths_entries.join(",\n"))
        };

        // ── components/schemas ────────────────────────────────────────────
        // Build a sensitivity map for x-sensitivity injection.
        let sensitivity_map: std::collections::HashMap<String, String> = module.flow_labels.iter()
            .flat_map(|fl| fl.types.iter().map(move |t| (t.clone(), fl.label.clone())))
            .collect();

        let mut schemas: Vec<String> = Vec::new();
        for item in &module.items {
            match item {
                Item::Type(td) => {
                    let mut schema = schema_emitter.emit_type_def_pub(td);
                    if let Some(label) = sensitivity_map.get(&td.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    schemas.push(format!("      {:?}: {}", td.name, schema));
                }
                Item::Enum(ed) => {
                    let mut schema = schema_emitter.emit_enum_def_pub(ed);
                    if let Some(label) = sensitivity_map.get(&ed.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    schemas.push(format!("      {:?}: {}", ed.name, schema));
                }
                Item::RefinedType(rt) => {
                    let mut schema = schema_emitter.emit_refined_type_pub(rt);
                    if let Some(label) = sensitivity_map.get(&rt.name) {
                        schema = inject_x_sensitivity(schema, label);
                    }
                    schemas.push(format!("      {:?}: {}", rt.name, schema));
                }
                Item::Fn(_) => {}
            }
        }

        // ── x-data-protection extension ───────────────────────────────────
        let mut pii_fields: Vec<String> = Vec::new();
        let mut hipaa_fields: Vec<String> = Vec::new();
        let mut pci_fields: Vec<String> = Vec::new();
        for item in &module.items {
            if let Item::Type(td) = item {
                for f in &td.fields {
                    let has = |key: &str| f.annotations.iter().any(|a| a.key == key);
                    if has("pii")   { pii_fields.push(format!("{}.{}", td.name, f.name)); }
                    if has("hipaa") { hipaa_fields.push(format!("{}.{}", td.name, f.name)); }
                    if has("pci")   { pci_fields.push(format!("{}.{}", td.name, f.name)); }
                }
            }
        }
        let data_protection_ext = if pii_fields.is_empty() && hipaa_fields.is_empty() && pci_fields.is_empty() {
            String::new()
        } else {
            let pii_json: Vec<String> = pii_fields.iter().map(|s| format!("{:?}", s)).collect();
            let hipaa_json: Vec<String> = hipaa_fields.iter().map(|s| format!("{:?}", s)).collect();
            let pci_json: Vec<String> = pci_fields.iter().map(|s| format!("{:?}", s)).collect();
            format!(
                ",\n  \"x-data-protection\": {{\"pii-fields\": [{}], \"hipaa-fields\": [{}], \"pci-fields\": [{}]}}",
                pii_json.join(", "), hipaa_json.join(", "), pci_json.join(", ")
            )
        };

        let schemas_section = if schemas.is_empty() {
            "  \"components\": {\"schemas\": {}}".to_string()
        } else {
            format!("  \"components\": {{\n    \"schemas\": {{\n{}\n    }}\n  }}", schemas.join(",\n"))
        };

        // Build x-lifecycle extension when lifecycle_defs are present.
        let lifecycle_ext = if module.lifecycle_defs.is_empty() {
            String::new()
        } else {
            let entries: Vec<String> = module.lifecycle_defs.iter().map(|lc| {
                let states_json: Vec<String> = lc.states.iter().map(|s| format!("{:?}", s)).collect();
                let transitions_json: Vec<String> = lc.states.windows(2).map(|w| {
                    format!("[{:?},{:?}]", w[0], w[1])
                }).collect();
                format!(
                    "      {:?}: {{\"states\": [{}], \"transitions\": [{}]}}",
                    lc.type_name,
                    states_json.join(", "),
                    transitions_json.join(", ")
                )
            }).collect();
            format!(",\n    \"x-lifecycle\": {{\n{}\n    }}", entries.join(",\n"))
        };

        // Build x-security-labels extension when flow_labels are present.
        let security_labels_ext = if module.flow_labels.is_empty() {
            String::new()
        } else {
            let entries: Vec<String> = module.flow_labels.iter().map(|fl| {
                let types_json: Vec<String> = fl.types.iter().map(|t| format!("{:?}", t)).collect();
                format!("    {:?}: [{}]", fl.label, types_json.join(", "))
            }).collect();
            format!(",\n  \"x-security-labels\": {{\n{}\n  }}", entries.join(",\n"))
        };

        // Build x-beings extension when being_defs are present.
        let being_ext = if module.being_defs.is_empty() {
            String::new()
        } else {
            let entries: Vec<String> = module.being_defs.iter().map(|being| {
                let telos_str = being.telos.as_ref().map(|t| t.description.as_str()).unwrap_or("");
                let matter_fields: Vec<String> = being.matter.as_ref()
                    .map(|m| m.fields.iter().map(|f| format!("{:?}", f.name)).collect())
                    .unwrap_or_default();
                let form_types: Vec<String> = being.form.as_ref()
                    .map(|f| {
                        let mut names: Vec<String> = f.types.iter().map(|t| format!("{:?}", t.name)).collect();
                        names.extend(f.enums.iter().map(|e| format!("{:?}", e.name)));
                        names
                    })
                    .unwrap_or_default();
                let regulate_entries: Vec<String> = being.regulate_blocks.iter().map(|reg| {
                    let bounds_str = reg.bounds.as_ref()
                        .map(|(l, h)| format!("{{\"low\": {:?}, \"high\": {:?}}}", l, h))
                        .unwrap_or_else(|| "null".to_string());
                    format!("{{\"variable\": {:?}, \"target\": {:?}, \"bounds\": {}}}", reg.variable, reg.target, bounds_str)
                }).collect();
                let mut parts = vec![
                    format!("\"x-being\": true"),
                    format!("\"x-telos\": {:?}", telos_str),
                ];
                if !form_types.is_empty() {
                    parts.push(format!("\"x-form\": [{}]", form_types.join(", ")));
                }
                if !matter_fields.is_empty() {
                    parts.push(format!("\"x-matter\": [{}]", matter_fields.join(", ")));
                }
                if let Some(evolve) = &being.evolve_block {
                    parts.push(format!("\"x-evolve-constraint\": {:?}", evolve.constraint));
                }
                if !regulate_entries.is_empty() {
                    parts.push(format!("\"x-regulate\": [{}]", regulate_entries.join(", ")));
                }
                format!("      {:?}: {{{}}}", being.name, parts.join(", "))
            }).collect();
            format!(",\n  \"x-beings\": {{\n{}\n  }}", entries.join(",\n"))
        };

        // Add being schemas to components/schemas.
        for being in &module.being_defs {
            let telos_str = being.telos.as_ref().map(|t| t.description.as_str()).unwrap_or("");
            let mut parts = vec![
                format!("\"x-being\": true"),
                format!("\"x-telos\": {:?}", telos_str),
            ];
            if let Some(matter) = &being.matter {
                let props: Vec<String> = matter.fields.iter()
                    .map(|f| format!("{:?}: {{\"type\": \"string\"}}", f.name))
                    .collect();
                if !props.is_empty() {
                    parts.push(format!("\"properties\": {{{}}}", props.join(", ")));
                }
            }
            schemas.push(format!("      {:?}: {{\"type\": \"object\", {}}}", being.name, parts.join(", ")));
        }

        let schemas_section = if schemas.is_empty() {
            "  \"components\": {\"schemas\": {}}".to_string()
        } else {
            format!("  \"components\": {{\n    \"schemas\": {{\n{}\n    }}\n  }}", schemas.join(",\n"))
        };

        // Build x-ecosystems extension when ecosystem_defs are present.
        let ecosystem_ext = if module.ecosystem_defs.is_empty() {
            String::new()
        } else {
            let entries: Vec<String> = module.ecosystem_defs.iter().map(|eco| {
                let telos_str = eco.telos.as_deref().unwrap_or("");
                let members_json: Vec<String> = eco.members.iter().map(|m| format!("{:?}", m)).collect();
                let signals_json: Vec<String> = eco.signals.iter().map(|sig| {
                    format!(
                        "{{\"name\": {:?}, \"from\": {:?}, \"to\": {:?}, \"payload\": {:?}}}",
                        sig.name, sig.from, sig.to, sig.payload
                    )
                }).collect();
                format!(
                    "    {:?}: {{\"x-telos\": {:?}, \"x-members\": [{}], \"x-signals\": [{}]}}",
                    eco.name,
                    telos_str,
                    members_json.join(", "),
                    signals_json.join(", ")
                )
            }).collect();
            format!(",\n  \"x-ecosystems\": {{\n{}\n  }}", entries.join(",\n"))
        };

        format!(
            "{{\n  \"openapi\": \"3.0.3\",\n  \"info\": {{\n    \"title\": {:?},\n    \"description\": {:?},\n    \"version\": \"1.0.0\"{}\n  }},\n{},\n{}{}{}{}{}\n}}",
            title, description, lifecycle_ext, paths_section, schemas_section, data_protection_ext, security_labels_ext, being_ext, ecosystem_ext
        )
    }

    // ── REST analysis ─────────────────────────────────────────────────────

    fn analyse(
        &self,
        fd: &FnDef,
        module: &Module,
        type_names: &[String],
        error_enums: &[&EnumDef],
    ) -> RestOp {
        let schema_emitter = JsonSchemaEmitter::new();

        // ── verb ─────────────────────────────────────────────────────────
        let verb = annotation_value(&fd.annotations, "method")
            .and_then(|m| match m.to_lowercase().as_str() {
                "get"    => Some(Verb::Get),
                "post"   => Some(Verb::Post),
                "put"    => Some(Verb::Put),
                "patch"  => Some(Verb::Patch),
                "delete" => Some(Verb::Delete),
                _ => None,
            })
            .or_else(|| Verb::from_name(&fd.name))
            .unwrap_or_else(|| {
                if fd.type_sig.params.is_empty() { Verb::Get }
                else if returns_list(&fd.type_sig.return_type) { Verb::Get }
                else { Verb::Post }
            });

        // @idempotent forces POST → PUT (idempotent mutations should be PUT).
        let is_idempotent = has_annotation(&fd.annotations, "idempotent");
        let verb = if is_idempotent && verb == Verb::Post { Verb::Put } else { verb };

        let is_commutative  = has_annotation(&fd.annotations, "commutative");
        let is_associative  = has_annotation(&fd.annotations, "associative");
        let is_at_most_once = has_annotation(&fd.annotations, "at-most-once");
        let is_exactly_once = has_annotation(&fd.annotations, "exactly-once");
        let is_monotonic    = has_annotation(&fd.annotations, "monotonic");

        // ── resource ─────────────────────────────────────────────────────
        let resource = annotation_value(&fd.annotations, "resource")
            .map(|s| s.to_string())
            .or_else(|| infer_resource_from_fn(fd, type_names))
            .or_else(|| infer_resource_from_module(&module.name, type_names))
            .unwrap_or_else(|| to_kebab_case(&module.name));

        let is_collection = returns_list(&fd.type_sig.return_type)
            || fd.name.starts_with("list")
            || fd.name.starts_with("all")
            || fd.name.starts_with("search");

        // ── parameter names ───────────────────────────────────────────────
        let param_names = op_param_names(fd);
        let params_with_types: Vec<(String, &TypeExpr)> = param_names.iter()
            .zip(fd.type_sig.params.iter())
            .map(|(n, t)| (n.clone(), t))
            .collect();

        // ── path params ───────────────────────────────────────────────────
        let path_params: Vec<(String, String)> = params_with_types.iter()
            .filter(|(name, ty)| is_path_param(name, ty, &verb))
            .map(|(name, ty)| (name.clone(), schema_emitter.type_expr_to_schema(ty)))
            .collect();

        // ── path ─────────────────────────────────────────────────────────
        let path = annotation_value(&fd.annotations, "path")
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let base = format!("/{}", pluralize(&resource));
                if path_params.is_empty() {
                    base
                } else if path_params.len() == 1 {
                    format!("{}/{{{}}}", base, path_params[0].0)
                } else {
                    let pp: Vec<String> = path_params.iter().map(|(n, _)| format!("{{{}}}", n)).collect();
                    format!("{}/{}", base, pp.join("/"))
                }
            });

        // ── request body ──────────────────────────────────────────────────
        let path_param_names: std::collections::HashSet<String> =
            path_params.iter().map(|(n, _)| n.clone()).collect();

        let body_params: Vec<(&String, &TypeExpr)> = params_with_types.iter()
            .filter(|(name, _)| !path_param_names.contains(name))
            .map(|(n, t)| (n, *t))
            .collect();

        let request_body = if verb.takes_body() && !body_params.is_empty() {
            let props: Vec<String> = body_params.iter()
                .map(|(name, ty)| format!("            {:?}: {}", name, schema_emitter.type_expr_to_schema(ty)))
                .collect();
            let required: Vec<String> = body_params.iter()
                .map(|(name, _)| format!("{:?}", name))
                .collect();
            Some(format!(
                "{{\n        \"required\": true,\n        \"content\": {{\n          \"application/json\": {{\n            \"schema\": {{\n              \"type\": \"object\",\n              \"properties\": {{{}}},\n              \"required\": [{}]\n            }}\n          }}\n        }}\n      }}",
                props.join(", "),
                required.join(", ")
            ))
        } else {
            None
        };

        // ── responses ─────────────────────────────────────────────────────
        let is_async = matches!(fd.type_sig.return_type.as_ref(), TypeExpr::Effect(_, _));
        let inner_ret = unwrap_effect(&fd.type_sig.return_type);
        let success_status = if verb == Verb::Post { 201 } else { 200 };

        let mut responses: Vec<(u16, String, String)> = Vec::new();

        // Handle Result<T, E> specially — split into 200 and 422.
        if let TypeExpr::Result(ok, err) = inner_ret {
            responses.push((success_status, "Success".to_string(), schema_emitter.type_expr_to_schema(ok)));
            responses.push((422, "Unprocessable entity".to_string(), schema_emitter.type_expr_to_schema(err)));
        } else {
            responses.push((success_status, "Success".to_string(), schema_emitter.type_expr_to_schema(inner_ret)));
        }

        // Error responses from matching XError enums.
        let resource_pascal = snake_to_pascal(&resource);
        let error_enum_name = format!("{}Error", resource_pascal);
        if let Some(err_enum) = error_enums.iter().find(|e| e.name == error_enum_name || e.name.contains(&resource_pascal)) {
            for status in error_status_codes(err_enum) {
                responses.push(status);
            }
        } else if !error_enums.is_empty() {
            // Any error enum in the module — attach generically.
            for err_enum in error_enums {
                for status in error_status_codes(err_enum) {
                    if !responses.iter().any(|(s, _, _)| *s == status.0) {
                        responses.push(status);
                    }
                }
            }
        }

        // 500 for effectful operations.
        if is_async {
            responses.push((500, "Internal server error".to_string(), "{\"type\":\"object\",\"properties\":{\"message\":{\"type\":\"string\"}}}".to_string()));
        }

        // ── tags ──────────────────────────────────────────────────────────
        let tag = annotation_value(&fd.annotations, "tag")
            .map(|t| t.to_string())
            .unwrap_or_else(|| snake_to_pascal(&resource));

        RestOp {
            verb,
            path,
            path_params,
            request_body,
            responses,
            operation_id: fd.name.clone(),
            summary: fd.describe.clone().unwrap_or_else(|| fd.name.clone()),
            tags: vec![tag],
            is_async,
            is_idempotent,
            is_commutative,
            is_associative,
            is_at_most_once,
            is_exactly_once,
            is_monotonic,
        }
    }

    // ── Render an operation to JSON ───────────────────────────────────────

    fn render_operation(&self, op: &RestOp) -> String {
        let mut parts: Vec<String> = Vec::new();
        parts.push(format!("        \"operationId\": {:?}", op.operation_id));
        parts.push(format!("        \"summary\": {:?}", op.summary));
        parts.push(format!("        \"tags\": [{}]",
            op.tags.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(", ")));

        if op.is_async {
            parts.push("        \"x-loom-async\": true".to_string());
        }

        // Algebraic property extensions.
        if op.is_idempotent    { parts.push("        \"x-idempotent\": true".to_string()); }
        if op.is_commutative   { parts.push("        \"x-commutative\": true".to_string()); }
        if op.is_associative   { parts.push("        \"x-associative\": true".to_string()); }
        if op.is_monotonic     { parts.push("        \"x-monotonic\": true".to_string()); }
        if op.is_at_most_once  {
            parts.push("        \"x-at-most-once\": true".to_string());
            parts.push("        \"x-retry-policy\": \"never\"".to_string());
        }
        if op.is_exactly_once  {
            parts.push("        \"x-exactly-once\": true".to_string());
            parts.push("        \"x-retry-policy\": \"never\"".to_string());
        }

        // Path parameters.
        if !op.path_params.is_empty() {
            let pp: Vec<String> = op.path_params.iter().map(|(name, schema)| {
                format!(
                    "{{\n            \"name\": {:?},\n            \"in\": \"path\",\n            \"required\": true,\n            \"schema\": {}\n          }}",
                    name, schema
                )
            }).collect();
            parts.push(format!("        \"parameters\": [\n          {}\n        ]", pp.join(",\n          ")));
        }

        // Request body.
        if let Some(rb) = &op.request_body {
            parts.push(format!("        \"requestBody\": {}", rb));
        }

        // Responses.
        let resp_entries: Vec<String> = op.responses.iter().map(|(status, desc, schema)| {
            format!(
                "          {:?}: {{\n            \"description\": {:?},\n            \"content\": {{\n              \"application/json\": {{\n                \"schema\": {}\n              }}\n            }}\n          }}",
                status.to_string(), desc, schema
            )
        }).collect();
        parts.push(format!("        \"responses\": {{\n{}\n        }}", resp_entries.join(",\n")));

        format!("{{\n{}\n      }}", parts.join(",\n"))
    }
}

// ── Resource inference ────────────────────────────────────────────────────────

/// Try to infer the resource name from a function's signature.
///
/// Injects `"x-sensitivity": "<label>"` into a JSON schema string.
/// Inserts before the closing `}` of the outermost object.
fn inject_x_sensitivity(schema: String, label: &str) -> String {
    if let Some(pos) = schema.rfind('}') {
        let mut out = schema.clone();
        let insert = format!(", \"x-sensitivity\": \"{}\"", label);
        out.insert_str(pos, &insert);
        out
    } else {
        schema
    }
}

/// Checks (in order):
/// 1. Return type name (unwrapped from List<T> / Effect<_, T> / Option<T>)
/// 2. Non-primitive parameter types
/// 3. Function name suffix after common verb prefixes
fn infer_resource_from_fn(fd: &FnDef, type_names: &[String]) -> Option<String> {
    // From return type.
    if let Some(name) = type_name_from_expr(&fd.type_sig.return_type, type_names) {
        return Some(to_kebab_case(&name));
    }
    // From param types.
    for ty in &fd.type_sig.params {
        if let Some(name) = type_name_from_expr(ty, type_names) {
            return Some(to_kebab_case(&name));
        }
    }
    // From fn name suffix: `create_order` → `order`.
    for prefix in &["create", "add", "get", "fetch", "find", "update", "patch", "delete", "remove", "list", "search", "load", "save", "upsert", "register"] {
        if fd.name.starts_with(prefix) {
            let rest = &fd.name[prefix.len()..];
            let rest = rest.trim_start_matches('_');
            if !rest.is_empty() {
                return Some(rest.replace('_', "-"));
            }
        }
    }
    None
}

fn infer_resource_from_module(module_name: &str, type_names: &[String]) -> Option<String> {
    // Strip common suffixes: Service, Controller, Handler, Manager, Api
    for suffix in &["Service", "Controller", "Handler", "Manager", "Api", "Resource"] {
        if module_name.ends_with(suffix) {
            let base = &module_name[..module_name.len() - suffix.len()];
            if !base.is_empty() {
                return Some(to_kebab_case(base));
            }
        }
    }
    // If module name matches a type name directly.
    let kebab = to_kebab_case(module_name);
    if type_names.iter().any(|t| to_kebab_case(t) == kebab) {
        return Some(kebab);
    }
    None
}

/// Extract the innermost named user type from a TypeExpr.
fn type_name_from_expr(ty: &TypeExpr, type_names: &[String]) -> Option<String> {
    match ty {
        TypeExpr::Base(name) => {
            if type_names.contains(name) { Some(name.clone()) } else { None }
        }
        TypeExpr::Generic(name, params) if name == "List" || name == "Set" => {
            params.first().and_then(|p| type_name_from_expr(p, type_names))
        }
        TypeExpr::Option(inner) => type_name_from_expr(inner, type_names),
        TypeExpr::Effect(_, inner) => type_name_from_expr(inner, type_names),
        TypeExpr::Result(ok, _) => type_name_from_expr(ok, type_names),
        _ => None,
    }
}

// ── Error response codes from enum variants ────────────────────────────────

fn error_status_codes(ed: &EnumDef) -> Vec<(u16, String, String)> {
    let mut out = Vec::new();
    let schema = format!("{{\"$ref\":\"#/components/schemas/{}\"}}", ed.name);
    let mut have_400 = false;
    let mut have_403 = false;
    let mut have_404 = false;

    for v in &ed.variants {
        let lower = v.name.to_lowercase();
        if !have_404 && (lower.contains("notfound") || lower.contains("not_found") || lower.contains("missing")) {
            out.push((404u16, "Not found".to_string(), schema.clone()));
            have_404 = true;
        }
        if !have_403 && (lower.contains("permission") || lower.contains("denied") || lower.contains("unauthorized") || lower.contains("forbidden")) {
            out.push((403u16, "Forbidden".to_string(), schema.clone()));
            have_403 = true;
        }
        if !have_400 && (lower.contains("invalid") || lower.contains("validation") || lower.contains("bad") || lower.contains("malformed")) {
            out.push((400u16, "Bad request".to_string(), schema.clone()));
            have_400 = true;
        }
    }
    out
}

// ── Path parameter detection ──────────────────────────────────────────────────

fn is_path_param(name: &str, ty: &TypeExpr, verb: &Verb) -> bool {
    let is_scalar = matches!(ty, TypeExpr::Base(n) if n == "Int" || n == "String" || n == "Str");
    let name_is_id = name == "id" || name.ends_with("_id") || name.ends_with("Id");
    // GET/DELETE scalars with id-ish names → path params
    // GET/DELETE with single scalar → path param even without id name
    if matches!(verb, Verb::Get | Verb::Delete) && is_scalar && name_is_id {
        return true;
    }
    // Any param with explicit "id" in the name → path param
    name_is_id && is_scalar
}

// ── Return type helpers ───────────────────────────────────────────────────────

fn returns_list(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Generic(name, _) if name == "List" || name == "Set" => true,
        TypeExpr::Effect(_, inner) => returns_list(inner),
        _ => false,
    }
}

fn unwrap_effect(ty: &TypeExpr) -> &TypeExpr {
    match ty {
        TypeExpr::Effect(_, inner) => inner.as_ref(),
        other => other,
    }
}

// This makes the TypeExpr::Try pattern compile — it doesn't exist but we reference it above.
// The function is only called on Base/Generic/Option/Result/Effect/Tuple.

// ── String utilities ──────────────────────────────────────────────────────────

fn to_kebab_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 { out.push('-'); }
        out.push(ch.to_lowercase().next().unwrap());
    }
    out
}

fn snake_to_pascal(s: &str) -> String {
    s.split(&['-', '_'][..])
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn pluralize(word: &str) -> String {
    if word.is_empty() { return word.to_string(); }
    // Handle kebab compounds: use last segment for pluralization.
    if let Some(pos) = word.rfind('-') {
        let prefix = &word[..=pos];
        let last = &word[pos + 1..];
        return format!("{}{}", prefix, pluralize(last));
    }
    let low = word.to_lowercase();
    if low.ends_with('s') || low.ends_with('x') || low.ends_with('z')
        || low.ends_with("ch") || low.ends_with("sh")
    {
        format!("{}es", word)
    } else if low.ends_with('y') && !matches!(low.chars().rev().nth(1), Some('a'|'e'|'i'|'o'|'u')) {
        format!("{}ies", &word[..word.len()-1])
    } else {
        format!("{}s", word)
    }
}

fn annotation_value<'a>(annotations: &'a [Annotation], key: &str) -> Option<&'a str> {
    annotations.iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
        .filter(|v| !v.is_empty())
}

fn has_annotation(annotations: &[Annotation], key: &str) -> bool {
    annotations.iter().any(|a| a.key == key)
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
        Expr::Tuple(elems, _) => { for e in elems { scan_idents(e, let_bound, seen, ordered); } }
        Expr::Try(inner, _) | Expr::As(inner, _) => scan_idents(inner, let_bound, seen, ordered),
        Expr::ForIn { iter, body, .. } => {
            scan_idents(iter, let_bound, seen, ordered);
            scan_idents(body, let_bound, seen, ordered);
        }
        Expr::Literal(_) | Expr::InlineRust(_) => {}
    }
}
