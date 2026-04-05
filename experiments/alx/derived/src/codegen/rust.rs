// ALX: derived from loom.loom §"emit_rust"
// Rust codegen rules (verbatim from spec):
// - TypeDef → pub struct with #[derive(Debug, Clone)]
// - EnumDef → pub enum with variants
// - RefinedType → newtype struct with TryFrom impl
// - FnDef → pub fn with require:/ensure: as debug_assert!
// - LifecycleDef → phantom type structs
// - FlowLabel → doc comments
// - BeingDef → pub struct (matter fields) + impl block with fitness/regulate/evolve/
//              epigenetic/morphogen/telomere/crispr/plasticity methods
// - autopoietic: true → second impl block with is_autopoietic() + verify_closure()
// - EcosystemDef → pub mod with signal structs + coordinate() + check_quorum()
// - @exactly-once → doc comment
// - @idempotent → doc comment

use crate::ast::*;

/// G3: RustEmitter struct — tests call `RustEmitter::new().emit(&module)`.
pub struct RustEmitter;

impl RustEmitter {
    pub fn new() -> Self { RustEmitter }
    pub fn emit(&self, module: &Module) -> String {
        emit_rust(module)
    }
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_lowercase().next().unwrap());
    }
    out
}

pub fn emit_rust(module: &Module) -> String {
    let mut out = String::new();

    // File-level attributes and imports (match real compiler output).
    out.push_str("#![allow(unused)]\n");
    out.push_str("use std::convert::TryFrom;\n\n");

    // Module-level doc comments from describe:.
    if let Some(desc) = &module.describe {
        for line in desc.lines() {
            out.push_str(&format!("/// {}\n", line));
        }
    }
    for ann in &module.annotations {
        out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value));
    }

    // Wrap everything in pub mod <snake_name> { ... }
    let mod_name = to_snake_case(&module.name);
    out.push_str(&format!("pub mod {} {{\n", mod_name));
    out.push_str("    use super::*;\n");

    // Information-flow labels as comments.
    if !module.flow_labels.is_empty() {
        out.push_str("    // information-flow labels:\n");
        for fl in &module.flow_labels {
            out.push_str(&format!("    //   {}: {}\n", fl.label, fl.types.join(", ")));
        }
    }

    // Module imports → use super::snake_module::*;
    for imp in &module.imports {
        out.push_str(&format!("    use super::{}::*;\n", to_snake_case(imp)));
    }

    // Render body into a buffer first to detect needed stdlib imports.
    let mut body = String::new();

    // Lifecycle phantom state types.
    for lc in &module.lifecycle_defs {
        emit_lifecycle(&mut body, lc);
    }

    // Flow labels (already emitted as comments above).
    // Items.
    for item in &module.items {
        match item {
            Item::Type(t) => emit_type_def(&mut body, t),
            Item::Enum(e) => emit_enum_def(&mut body, e),
            Item::RefinedType(r) => emit_refined_type(&mut body, r),
            Item::Fn(f) => emit_fn_def(&mut body, f),
        }
    }

    // Invariants.
    for inv in &module.invariants {
        body.push_str(&format!("// invariant {}: {}\n", inv.name, inv.condition));
    }

    // Beings.
    for being in &module.being_defs {
        emit_being_def(&mut body, being);
    }

    // Ecosystems.
    for eco in &module.ecosystem_defs {
        emit_ecosystem_def(&mut body, eco);
    }

    // Interface defs → pub trait
    for iface in &module.interface_defs {
        emit_interface_def(&mut body, iface);
    }

    // Test blocks → #[cfg(test)] mod tests { ... }
    if !module.test_defs.is_empty() {
        body.push_str("#[cfg(test)]\nmod tests {\n    use super::*;\n\n");
        for tst in &module.test_defs {
            emit_test(&mut body, tst);
        }
        body.push_str("}\n\n");
    }

    // Inject stdlib collection imports when body uses them.
    if body.contains("HashMap") {
        out.push_str("    use std::collections::HashMap;\n");
    }
    if body.contains("HashSet") {
        out.push_str("    use std::collections::HashSet;\n");
    }
    if !body.is_empty() {
        out.push('\n');
    }

    // Indent body lines inside the module.
    for line in body.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }

    out.push_str("}\n");
    out
}

fn emit_lifecycle(out: &mut String, lc: &LifecycleDef) {
    out.push_str(&format!("// Lifecycle: {} states\n", lc.type_name));
    for state in &lc.states {
        out.push_str(&format!(
            "#[derive(Debug, Clone, PartialEq)]\npub struct {};\n",
            state
        ));
    }
    out.push('\n');
}

fn emit_type_def(out: &mut String, t: &TypeDef) {
    // Derive macro
    out.push_str(
        "#[derive(Debug, Clone)]\n"
    );
    // Generic params
    let params = if t.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", t.type_params.join(", "))
    };
    out.push_str(&format!("pub struct {}{} {{\n", t.name, params));
    for field in &t.fields {
        // Privacy annotations as doc comments
        for ann in &field.annotations {
            out.push_str(&format!("    // @{}\n", ann.key));
        }
        let ty_str = type_to_rust(&field.ty);
        out.push_str(&format!("    pub {}: {},\n", rust_field_name(&field.name), ty_str));
    }
    out.push_str("}\n\n");
}

fn emit_enum_def(out: &mut String, e: &EnumDef) {
    out.push_str(
        "#[derive(Debug, Clone, PartialEq)]\n"
    );
    let params = if e.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", e.type_params.join(", "))
    };
    out.push_str(&format!("pub enum {}{} {{\n", e.name, params));
    for v in &e.variants {
        match &v.payload {
            None => out.push_str(&format!("    {},\n", v.name)),
            Some(ty) => out.push_str(&format!("    {}({}),\n", v.name, type_to_rust(ty))),
        }
    }
    out.push_str("}\n\n");
}

fn emit_refined_type(out: &mut String, r: &RefinedType) {
    let base = type_to_rust(&r.base_type);
    out.push_str("#[derive(Debug, Clone)]\n");
    out.push_str(&format!("pub struct {}(pub {});\n\n", r.name, base));
    out.push_str(&format!("impl TryFrom<{}> for {} {{\n", base, r.name));
    out.push_str("    type Error = String;\n");
    out.push_str(&format!("    fn try_from(value: {}) -> Result<Self, Self::Error> {{\n", base));
    out.push_str(&format!("        // Predicate: {}\n", r.predicate));
    out.push_str("        // ALX: predicate is raw text; full validation requires expression eval\n");
    out.push_str("        Ok(Self(value))\n");
    out.push_str("    }\n}\n\n");
}

fn emit_fn_def(out: &mut String, f: &FnDef) {
    // Doc comment
    if let Some(desc) = &f.describe {
        out.push_str(&format!("/// {}\n", desc));
    }
    // Annotation doc comments
    for ann in &f.annotations {
        match ann.key.as_str() {
            "exactly-once" => out.push_str("/// @exactly-once: this function must execute exactly once\n"),
            "idempotent" => out.push_str("/// @idempotent: f(f(x)) = f(x)\n"),
            "commutative" => out.push_str("/// @commutative: f(a,b) = f(b,a)\n"),
            "deprecated" => {
                out.push_str(&format!("/// @deprecated: {}\n", ann.value));
                out.push_str(&format!("#[deprecated(note = \"{}\")]\n", ann.value));
            }
            _ => out.push_str(&format!("/// @{}: {}\n", ann.key, ann.value)),
        }
    }

    // Generic params
    let type_params = if f.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", f.type_params.join(", "))
    };

    // Parameters
    let params: Vec<String> = f
        .type_sig
        .params
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("p{}: {}", i, type_to_rust(ty)))
        .collect();
    let ret = type_to_rust(&f.type_sig.return_type);
    out.push_str(&format!(
        "pub fn {}{}({}) -> {} {{\n",
        f.name,
        type_params,
        params.join(", "),
        ret
    ));

    // require: → debug_assert!
    for req in &f.requires {
        out.push_str(&format!("    debug_assert!({}, \"require: {}\");\n", req.expr, req.expr));
    }

    // Body
    if let Some(inline) = &f.inline_body {
        out.push_str(&format!("    {}\n", inline));
    } else if f.body.is_empty() {
        out.push_str("    todo!()\n");
    } else {
        out.push_str("    // body:\n");
        for line in &f.body {
            out.push_str(&format!("    // {}\n", line));
        }
        out.push_str("    todo!()\n");
    }

    // ensure: → debug_assert!
    for ens in &f.ensures {
        out.push_str(&format!(
            "    debug_assert!({{}}, \"ensure: {}\");\n",
            ens.expr
        ));
    }

    out.push_str("}\n\n");
}

fn emit_test(out: &mut String, tst: &TestDef) {
    // Test blocks: detect `inline { ... }` and emit verbatim, otherwise emit as comment
    out.push_str("    #[test]\n");
    out.push_str(&format!("    fn {}() {{\n", rust_ident(&tst.name)));
    // If body starts with "inline {", strip the wrapper and emit verbatim
    let body = tst.body.trim();
    if let Some(rest) = body.strip_prefix("inline {") {
        let inner = rest.trim_end_matches('}').trim();
        out.push_str(&format!("        {}\n", inner));
    } else if !body.is_empty() {
        out.push_str(&format!("        {}\n", body));
    }
    out.push_str("    }\n\n");
}

fn emit_interface_def(out: &mut String, iface: &InterfaceDef) {
    let params = if iface.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", iface.type_params.join(", "))
    };
    out.push_str(&format!("pub trait {}{} {{\n", iface.name, params));
    for method in &iface.methods {
        let params: Vec<String> = method.type_sig.params.iter().enumerate()
            .map(|(i, ty)| format!("p{}: {}", i, type_to_rust(ty)))
            .collect();
        let ret = type_to_rust(&method.type_sig.return_type);
        out.push_str(&format!("    fn {}({}) -> {};\n", method.name, params.join(", "), ret));
    }
    out.push_str("}\n\n");
}

fn emit_being_def(out: &mut String, being: &BeingDef) {
    if let Some(desc) = &being.describe {
        out.push_str(&format!("/// {}\n", desc));
    }
    // telos as doc comment
    if let Some(telos) = &being.telos {
        out.push_str(&format!("/// telos: {}\n", telos.description));
    }
    // Annotations as doc comments
    for ann in &being.annotations {
        out.push_str(&format!("/// @{}\n", ann.key));
    }

    // pub struct with matter fields
    out.push_str(
        "#[derive(Debug, Clone)]\n"
    );
    out.push_str(&format!("pub struct {} {{\n", being.name));

    if let Some(matter) = &being.matter {
        for field in &matter.fields {
            for ann in &field.annotations {
                out.push_str(&format!("    // @{}\n", ann.key));
            }
            out.push_str(&format!(
                "    pub {}: {},\n",
                rust_field_name(&field.name),
                type_to_rust(&field.ty)
            ));
        }
    }

    // telomere: → telomere_count field
    if being.telomere.is_some() {
        out.push_str("    pub telomere_count: u64,\n");
    }

    out.push_str("}\n\n");

    // impl block
    out.push_str(&format!("impl {} {{\n", being.name));

    // fitness() method
    if let Some(telos) = &being.telos {
        out.push_str("    /// Evaluate fitness toward telos.\n");
        if let Some(fitness_fn) = &telos.fitness_fn {
            out.push_str(&format!("    /// fitness: {}\n", fitness_fn));
        }
        out.push_str("    pub fn fitness(&self) -> f64 {\n");
        out.push_str("        todo!(\"implement fitness function\")\n");
        out.push_str("    }\n\n");
    }

    // regulate() methods
    for reg in &being.regulate_blocks {
        out.push_str(&format!("    /// Homeostatic regulation of {}.\n", reg.variable));
        out.push_str(&format!("    pub fn regulate_{}(&self) {{\n", rust_ident(&reg.variable)));
        if let Some((lo, hi)) = &reg.bounds {
            out.push_str(&format!(
                "        debug_assert!(!({lo} > {hi}), \"inverted bounds: {} > {}\");\n",
                lo, hi,
                lo = lo,
                hi = hi
            ));
            out.push_str(&format!(
                "        // target: {}\n        // bounds: ({}, {})\n",
                reg.target, lo, hi
            ));
        }
        for resp in &reg.response {
            out.push_str(&format!("        // response: {}\n", resp));
        }
        out.push_str("        todo!()\n    }\n\n");
    }

    // evolve() method
    if let Some(evolve) = &being.evolve_block {
        out.push_str("    /// Directed search toward telos.\n");
        out.push_str(&format!("    /// constraint: {}\n", evolve.constraint));
        out.push_str("    pub fn evolve(&mut self) {\n");
        for case in &evolve.search_cases {
            let strategy = match case.strategy {
                SearchStrategy::GradientDescent => "gradient_descent",
                SearchStrategy::StochasticGradient => "stochastic_gradient",
                SearchStrategy::SimulatedAnnealing => "simulated_annealing",
                SearchStrategy::DerivativeFree => "derivative_free",
                SearchStrategy::Mcmc => "mcmc",
            };
            out.push_str(&format!("        // {} when {}\n", strategy, case.when));
        }
        out.push_str("        todo!()\n    }\n\n");
    }

    // epigenetic method
    for epi in &being.epigenetic_blocks {
        out.push_str(&format!(
            "    /// Epigenetic modifier: signal '{}' modifies '{}'.\n",
            epi.signal, epi.modifies
        ));
        out.push_str(&format!(
            "    pub fn apply_epigenetic_{}(&mut self, signal_strength: f64) {{\n",
            rust_ident(&epi.signal)
        ));
        out.push_str(&format!("        // modifies: {}\n", epi.modifies));
        if let Some(rev) = &epi.reverts_when {
            out.push_str(&format!("        // reverts_when: {}\n", rev));
        }
        out.push_str("        todo!()\n    }\n\n");
    }

    // morphogen method
    for morph in &being.morphogen_blocks {
        let produces_str = morph.produces.join(", ");
        out.push_str(&format!(
            "    /// Reaction-diffusion: signal '{}' produces [{}].\n",
            morph.signal, produces_str
        ));
        out.push_str(&format!(
            "    pub fn differentiate_{}(&self) {{\n",
            rust_ident(&morph.signal)
        ));
        out.push_str(&format!(
            "        // signal: {}, threshold: {}, produces: {:?}\n",
            morph.signal, morph.threshold, morph.produces
        ));
        out.push_str("        todo!()\n    }\n\n");
    }

    // telomere — replicate() method
    if let Some(tel) = &being.telomere {
        out.push_str("    /// Replicate with finite Hayflick limit.\n");
        out.push_str("    pub fn replicate(&mut self) -> Option<Self> where Self: Clone {\n");
        out.push_str(&format!(
            "        if self.telomere_count >= {} {{\n",
            tel.limit
        ));
        out.push_str(&format!("            // on_exhaustion: {}\n", tel.on_exhaustion));
        out.push_str("            return None;\n        }\n");
        out.push_str("        self.telomere_count += 1;\n");
        out.push_str("        Some(self.clone())\n    }\n\n");
    }

    // crispr — edit method
    for crispr in &being.crispr_blocks {
        out.push_str(&format!(
            "    /// CRISPR-guided self-modification: {}.\n",
            crispr.guide
        ));
        out.push_str(&format!(
            "    pub fn edit_{}(&mut self) {{\n",
            rust_ident(&crispr.guide)
        ));
        out.push_str(&format!(
            "        // target: {}, replace with: {}\n",
            crispr.target, crispr.replace
        ));
        out.push_str("        todo!()\n    }\n\n");
    }

    // plasticity — update method per plasticity block
    for plasticity in &being.plasticity_blocks {
        out.push_str(&format!(
            "    /// Plasticity rule ({:?}) modifies {}.\n",
            plasticity.rule, plasticity.modifies
        ));
        out.push_str(&format!(
            "    pub fn update_{}(&mut self) {{\n",
            rust_ident(&plasticity.modifies)
        ));
        out.push_str(&format!("        // trigger: {}\n", plasticity.trigger));
        out.push_str("        todo!()\n    }\n\n");
    }

    // function: block methods
    if let Some(func) = &being.function {
        for f in &func.fns {
            let params: Vec<String> = f
                .type_sig
                .params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("p{}: {}", i, type_to_rust(ty)))
                .collect();
            let ret = type_to_rust(&f.type_sig.return_type);
            out.push_str(&format!(
                "    pub fn {}({}) -> {} {{\n        todo!()\n    }}\n\n",
                f.name,
                params.join(", "),
                ret
            ));
        }
    }

    out.push_str("}\n\n");

    // autopoietic: true → second impl block
    if being.autopoietic {
        out.push_str(&format!("impl {} {{\n", being.name));
        out.push_str("    /// Returns true if this being is autopoietic (self-producing).\n");
        out.push_str("    pub fn is_autopoietic() -> bool { true }\n\n");
        out.push_str("    /// Verify the system is operationally closed.\n");
        out.push_str("    pub fn verify_closure(&self) -> bool {\n");
        out.push_str("        todo!(\"verify operational closure\")\n");
        out.push_str("    }\n}\n\n");
    }
}

fn emit_ecosystem_def(out: &mut String, eco: &EcosystemDef) {
    if let Some(desc) = &eco.describe {
        out.push_str(&format!("/// {}\n", desc));
    }
    if let Some(telos) = &eco.telos {
        out.push_str(&format!("/// telos: {}\n", telos));
    }
    out.push_str(&format!("pub mod {} {{\n", rust_ident(&eco.name)));
    out.push_str("    use super::*;\n\n");

    // Signal structs
    for sig in &eco.signals {
        out.push_str(&format!("    #[derive(Debug, Clone)]\n"));
        out.push_str(&format!("    pub struct {}Signal {{\n", sig.name));
        out.push_str(&format!("        pub from: String,\n"));
        out.push_str(&format!("        pub to: String,\n"));
        out.push_str(&format!("        // payload: {}\n", sig.payload));
        out.push_str("    }\n\n");
    }

    // coordinate() function
    out.push_str("    /// Coordinate all member beings.\n");
    out.push_str("    pub fn coordinate() {\n");
    for member in &eco.members {
        out.push_str(&format!("        // coordinate: {}\n", member));
    }
    out.push_str("        todo!()\n    }\n\n");

    // check_quorum() function
    if let Some(quorum) = eco.quorum_blocks.first() {
        out.push_str(&format!(
            "    /// Check quorum (threshold: {}) for signal '{}'.\n",
            quorum.threshold,
            quorum.signal
        ));
        out.push_str("    pub fn check_quorum(population_size: usize, active: usize) -> bool {\n");
        out.push_str(&format!(
            "        // threshold: {}\n        active > 0 && population_size > 0\n",
            quorum.threshold
        ));
        out.push_str("    }\n\n");
    } else {
        out.push_str("    pub fn check_quorum(_population_size: usize, _active: usize) -> bool { false }\n\n");
    }

    out.push_str("}\n\n");
}

// ── Type rendering helpers ───────────────────────────────────────────────────

pub fn type_to_rust(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Base(n) => match n.as_str() {
            "Int" => "i64".into(),
            "Float" => "f64".into(),
            "String" => "String".into(),
            "Bool" => "bool".into(),
            "Unit" => "()".into(),
            other => other.to_string(),
        },
        TypeExpr::Generic(n, args) => {
            match n.as_str() {
                "List" => format!("Vec<{}>", args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
                "Option" => format!("Option<{}>", args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
                "Result" => format!("Result<{}>", args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
                "Map" => format!("HashMap<{}>", args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
                "Set" => format!("HashSet<{}>", args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
                "Float" if args.len() == 1 => {
                    // Unit-parameterised float — use newtype
                    if let TypeExpr::Base(unit) = &args[0] {
                        capitalize(unit)
                    } else {
                        "f64".into()
                    }
                }
                _ => format!("{}<{}>", n, args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
            }
        }
        TypeExpr::Effect(_, ret) => type_to_rust(ret),
        TypeExpr::Option(inner) => format!("Option<{}>", type_to_rust(inner)),
        TypeExpr::Result(ok, err) => format!("Result<{}, {}>", type_to_rust(ok), type_to_rust(err)),
        TypeExpr::Tuple(types) => format!("({})", types.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")),
        TypeExpr::Fn(a, b) => format!("impl Fn({}) -> {}", type_to_rust(a), type_to_rust(b)),
        TypeExpr::TypeVar(n) => format!("T{}", n),
    }
}

fn rust_field_name(name: &str) -> String {
    // Convert kebab-case or hyphenated to snake_case
    name.replace('-', "_")
}

/// Check if the module uses a given generic type name (e.g. "Map", "Set").
fn module_uses_collection(module: &Module, type_name: &str) -> bool {
    for item in &module.items {
        if let Item::Fn(f) = item {
            if type_expr_uses(f.type_sig.return_type.clone(), type_name) { return true; }
            for p in &f.type_sig.params {
                if type_expr_uses(p.clone(), type_name) { return true; }
            }
        }
        if let Item::Type(t) = item {
            for field in &t.fields {
                if type_expr_uses(field.ty.clone(), type_name) { return true; }
            }
        }
    }
    false
}

fn type_expr_uses(ty: TypeExpr, name: &str) -> bool {
    match ty {
        TypeExpr::Generic(n, args) => {
            if n == name { return true; }
            args.into_iter().any(|a| type_expr_uses(a, name))
        }
        TypeExpr::Option(inner) | TypeExpr::Effect(_, inner) => type_expr_uses(*inner, name),
        TypeExpr::Result(ok, err) => type_expr_uses(*ok, name) || type_expr_uses(*err, name),
        TypeExpr::Tuple(ts) => ts.into_iter().any(|t| type_expr_uses(t, name)),
        TypeExpr::Fn(a, b) => type_expr_uses(*a, name) || type_expr_uses(*b, name),
        _ => false,
    }
}

pub fn rust_ident(name: &str) -> String {
    name.replace('-', "_").replace(' ', "_").to_lowercase()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

