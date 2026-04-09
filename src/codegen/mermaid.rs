//! M108: Mermaid diagram emitter for Loom AST.
//!
//! Emits relationship-memory artifacts directly from program structure.
//! Diagrams cannot drift from code because they ARE the code.
//! Closes the GS Diagram-emitting property.
//!
//! Academic lineage:
//! - C4 model (Simon Brown 2018) → architecture context/container diagrams
//! - Mermaid (Sveidqvist 2019) → plain-text diagram DSL
//! - Honda (1993) session types → sequence diagram source of truth
//! - Loom lifecycle (M7) → state machine source of truth

use crate::ast::*;

/// M108: Mermaid diagram emitter.
///
/// Derives four diagram types directly from parsed AST, eliminating
/// diagram drift: the diagram IS the code.
pub struct MermaidEmitter;

impl MermaidEmitter {
    /// Create a new Mermaid emitter.
    pub fn new() -> Self {
        MermaidEmitter
    }

    /// Emit a C4 container diagram from being and fn structure.
    ///
    /// Each `being:` block → Container node; each `fn` → Component node.
    pub fn emit_c4(&self, module: &Module) -> String {
        let mut out = String::from("```mermaid\nC4Container\n");
        out.push_str(&format!("  title C4 Container — {}\n", module.name));
        for being in &module.being_defs {
            let desc = being
                .telos
                .as_ref()
                .map(|t| t.description.as_str())
                .unwrap_or("no telos");
            out.push_str(&format!(
                "  Container({}, \"{}\", \"Loom Being\", \"{}\")\n",
                being.name, being.name, desc
            ));
        }
        for item in &module.items {
            if let Item::Fn(fd) = item {
                out.push_str(&format!(
                    "  Component({}, \"{}\", \"fn\")\n",
                    fd.name, fd.name
                ));
            }
        }
        out.push_str("```\n");
        out
    }

    /// Emit a sequence diagram from session type declarations.
    ///
    /// Each role → `participant` line; Send steps → `->>` arrows derived
    /// from the `duality:` declaration.
    pub fn emit_sequence(&self, module: &Module) -> String {
        let mut out = String::from("```mermaid\nsequenceDiagram\n");
        for item in &module.items {
            if let Item::Session(sd) = item {
                for role in &sd.roles {
                    out.push_str(&format!("  participant {}\n", role.name));
                }
                if let Some((dual_a, dual_b)) = &sd.duality {
                    let steps_a = sd
                        .roles
                        .iter()
                        .find(|r| r.name == *dual_a)
                        .map(|r| &r.steps);
                    let steps_b = sd
                        .roles
                        .iter()
                        .find(|r| r.name == *dual_b)
                        .map(|r| &r.steps);
                    if let (Some(sa), Some(sb)) = (steps_a, steps_b) {
                        let max = sa.len().max(sb.len());
                        for i in 0..max {
                            if let Some(SessionStep::Send(te)) = sa.get(i) {
                                out.push_str(&format!(
                                    "  {}->>{}: {}\n",
                                    dual_a,
                                    dual_b,
                                    type_expr_str(te)
                                ));
                            }
                            if let Some(SessionStep::Send(te)) = sb.get(i) {
                                out.push_str(&format!(
                                    "  {}->>{}: {}\n",
                                    dual_b,
                                    dual_a,
                                    type_expr_str(te)
                                ));
                            }
                        }
                    }
                }
            }
        }
        out.push_str("```\n");
        out
    }

    /// Emit a state diagram from lifecycle declarations.
    ///
    /// Each `lifecycle T :: S1 -> S2 -> S3` → adjacent `S1 --> S2` transitions
    /// in `stateDiagram-v2` syntax.
    pub fn emit_state(&self, module: &Module) -> String {
        let mut out = String::from("```mermaid\nstateDiagram-v2\n");
        for lc in &module.lifecycle_defs {
            for window in lc.states.windows(2) {
                out.push_str(&format!("  {} --> {}\n", window[0], window[1]));
            }
        }
        out.push_str("```\n");
        out
    }

    /// Emit a flow diagram from fn declarations.
    ///
    /// Top-level `fn` items → `flowchart TD` nodes with sequential edges
    /// from `Start` through each function to `End`.
    pub fn emit_flow(&self, module: &Module) -> String {
        let mut out = String::from("```mermaid\nflowchart TD\n");
        out.push_str("  Start([Start])\n");
        let mut prev = "Start".to_string();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                let node_id = fn_node_id(&fd.name);
                out.push_str(&format!("  {}[{}]\n", node_id, fd.name));
                out.push_str(&format!("  {} --> {}\n", prev, node_id));
                prev = node_id;
            }
        }
        out.push_str("  End([End])\n");
        out.push_str(&format!("  {} --> End\n", prev));
        out.push_str("```\n");
        out
    }
}

/// Format a `TypeExpr` as a human-readable label for diagram arrows.
fn type_expr_str(te: &TypeExpr) -> String {
    match te {
        TypeExpr::Base(n) => n.clone(),
        TypeExpr::Generic(name, params) => {
            let ps: Vec<String> = params.iter().map(type_expr_str).collect();
            format!("{}<{}>", name, ps.join(", "))
        }
        TypeExpr::Option(inner) => format!("Option<{}>", type_expr_str(inner)),
        TypeExpr::Result(ok, err) => {
            format!("Result<{}, {}>", type_expr_str(ok), type_expr_str(err))
        }
        TypeExpr::Tuple(elems) => {
            let es: Vec<String> = elems.iter().map(type_expr_str).collect();
            format!("({})", es.join(", "))
        }
        TypeExpr::Effect(_, inner) => format!("Effect<{}>", type_expr_str(inner)),
        TypeExpr::Dynamic => "?".to_string(),
        _ => "Type".to_string(),
    }
}

/// Convert a function name into a valid Mermaid node ID (alphanumeric + underscore only).
fn fn_node_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
