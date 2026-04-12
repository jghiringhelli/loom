//! Type and enum emitters — struct / enum / refined types / functors / monads.

use super::RustEmitter;
use crate::ast::*;

impl RustEmitter {
    /// Emit a product type as a `#[derive(…)] pub struct`.
    pub(super) fn emit_type_def(&self, td: &TypeDef) -> String {
        let has_pii = td
            .fields
            .iter()
            .any(|f| f.annotations.iter().any(|a| a.key == "pii"));
        let mut out = String::new();
        if has_pii {
            out.push_str("// loom: module contains PII fields — handle with care\n");
        }
        let fields: Vec<String> = td
            .fields
            .iter()
            .map(|f| {
                let mut field_out = String::new();
                for ann in &f.annotations {
                    match ann.key.as_str() {
                        "pii" => field_out.push_str("    #[cfg_attr(loom_runtime, loom_pii)]\n"),
                        "secret" => {
                            field_out.push_str("    #[cfg_attr(loom_runtime, loom_secret)]\n")
                        }
                        "encrypt-at-rest" => field_out
                            .push_str("    #[cfg_attr(loom_runtime, loom_encrypt_at_rest)]\n"),
                        "never-log" => {
                            field_out.push_str(&format!("    // NEVER LOG: {}\n", f.name))
                        }
                        _ => {}
                    }
                }
                field_out.push_str(&format!(
                    "    pub {}: {},",
                    f.name,
                    self.emit_type_expr(&f.ty)
                ));
                field_out
            })
            .collect();
        out.push_str(&format!(
            "#[derive(Debug, Clone, PartialEq)]\npub struct {} {{\n{}\n}}\n",
            td.name,
            fields.join("\n")
        ));
        out
    }

    /// Emit a sum type as a `#[derive(…)] pub enum`.
    pub(super) fn emit_enum_def(&self, ed: &EnumDef) -> String {
        let variants: Vec<String> = ed
            .variants
            .iter()
            .map(|v| match &v.payload {
                Some(ty) => format!("    {}({}),", v.name, self.emit_type_expr(ty)),
                None => format!("    {},", v.name),
            })
            .collect();
        format!(
            "#[derive(Debug, Clone, PartialEq)]\npub enum {} {{\n{}\n}}\n",
            ed.name,
            variants.join("\n")
        )
    }

    /// Emit a refined type as a newtype + `TryFrom` with the predicate as the guard.
    ///
    /// The `self` reference in the predicate is replaced with `value` to match
    /// the `TryFrom` parameter name.
    pub(super) fn emit_refined_type(&self, rt: &RefinedType) -> String {
        let base = self.emit_type_expr(&rt.base_type);
        let pred = self.emit_predicate(&rt.predicate);
        let mut out = format!(
            "#[derive(Debug, Clone, PartialEq)]\n\
             pub struct {name}({base});\n\n\
             impl TryFrom<{base}> for {name} {{\n\
             \x20\x20\x20\x20type Error = String;\n\
             \x20\x20\x20\x20fn try_from(value: {base}) -> Result<Self, Self::Error> {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20if !({pred}) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20return Err(format!(\"refined type invariant violated for {name}: {{:?}}\", value));\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Ok({name}(value))\n\
             \x20\x20\x20\x20}}\n\
             }}\n",
            name = rt.name,
            base = base,
            pred = pred,
        );
        if let Some(ov) = &rt.on_violation {
            out.push_str(&format!("// on_violation: {}\n", ov));
        }
        if let Some(rf) = &rt.repair_fn {
            out.push_str(&format!("// repair_fn: {}\n", rf));
        }
        out
    }

    pub(super) fn emit_proposition(&self, prop: &PropositionDef) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "// proposition: {} = {:?}\n",
            prop.name, prop.base_type
        ));
        let base = match &prop.base_type {
            TypeExpr::Base(n) => self.map_base_type(n),
            other => self.emit_type_expr(other),
        };
        out.push_str(&format!("pub type {} = {};\n", prop.name, base));
        out
    }

    pub(super) fn emit_functor(&self, f: &FunctorDef) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "// Functor: {} — category theory (Mac Lane 1971)\n",
            f.name
        ));
        let type_params = if f.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", f.type_params.join(", "))
        };
        out.push_str(&format!("pub trait Functor{}{} {{\n", f.name, type_params));
        for law in &f.laws {
            out.push_str(&format!("    // law: {}\n", law.name));
        }
        out.push_str("}\n");
        out
    }

    pub(super) fn emit_monad(&self, m: &MonadDef) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "// Monad: {} — category theory (Mac Lane 1971)\n",
            m.name
        ));
        let type_params = if m.type_params.is_empty() {
            String::new()
        } else {
            format!("<{}>", m.type_params.join(", "))
        };
        out.push_str(&format!("pub trait Monad{}{} {{\n", m.name, type_params));
        for law in &m.laws {
            out.push_str(&format!("    // law: {}\n", law.name));
        }
        out.push_str("}\n");
        out
    }

    pub(super) fn emit_certificate(&self, cert: &CertificateDef) -> String {
        let mut out = String::new();
        out.push_str("// Self-certifying compilation certificate (Necula 1997):\n");
        for field in &cert.fields {
            out.push_str(&format!("//   {}: {}\n", field.name, field.value));
        }
        out
    }

    /// Format a pointcut expression as a human-readable string for doc comments.
    pub(super) fn fmt_pointcut(pc: &PointcutExpr) -> String {
        match pc {
            PointcutExpr::HasAnnotation(ann) => format!("fn where @{}", ann),
            PointcutExpr::EffectIncludes(eff) => format!("fn where effect includes {}", eff),
            PointcutExpr::And(l, r) => {
                format!("{} and {}", Self::fmt_pointcut(l), Self::fmt_pointcut(r))
            }
            PointcutExpr::Or(l, r) => {
                format!("{} or {}", Self::fmt_pointcut(l), Self::fmt_pointcut(r))
            }
        }
    }
}
