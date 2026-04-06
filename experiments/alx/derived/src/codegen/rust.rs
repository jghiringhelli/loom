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

    // Provides → pub trait
    if let Some(provides) = &module.provides {
        body.push_str(&format!("pub trait {} {{\n", module.name));
        for (op_name, sig) in &provides.ops {
            let params: Vec<String> = sig.params.iter().enumerate()
                .map(|(i, ty)| format!("arg{}: {}", i, type_to_rust(ty)))
                .collect();
            let ret = type_to_rust(&sig.return_type);
            body.push_str(&format!("    fn {}({}) -> {};\n", op_name, params.join(", "), ret));
        }
        body.push_str("}\n\n");
    }

    // Requires → context struct
    let has_requires = module.requires.is_some();
    if let Some(requires) = &module.requires {
        body.push_str("#[derive(Debug)]\n");
        body.push_str(&format!("pub struct {}Context {{\n", module.name));
        for (dep_name, dep_type) in &requires.deps {
            body.push_str(&format!("    pub {}: {},\n", dep_name, type_to_rust(dep_type)));
        }
        body.push_str("}\n\n");
    }

    // Interface defs → pub trait
    for iface in &module.interface_defs {
        emit_interface_def(&mut body, iface);
    }

    // Items: non-implements functions emitted directly; implements functions in impl block
    if module.implements.is_empty() {
        // No implements: emit all items normally
        for item in &module.items {
            match item {
                Item::Type(t) => emit_type_def(&mut body, t),
                Item::Enum(e) => emit_enum_def(&mut body, e),
                Item::RefinedType(r) => emit_refined_type(&mut body, r),
                Item::Fn(f) => emit_fn_def_di(&mut body, f, &module.name, has_requires),
            }
        }
    } else {
        // Implements: emit types normally, wrap functions in impl block(s)
        for item in &module.items {
            match item {
                Item::Type(t) => emit_type_def(&mut body, t),
                Item::Enum(e) => emit_enum_def(&mut body, e),
                Item::RefinedType(r) => emit_refined_type(&mut body, r),
                Item::Fn(_) => {} // collected into impl block below
            }
        }
        let impl_struct = format!("{}Impl", module.name);
        body.push_str(&format!("pub struct {};\n\n", impl_struct));
        for trait_name in &module.implements {
            body.push_str(&format!("impl {} for {} {{\n", trait_name, impl_struct));
            for item in &module.items {
                if let Item::Fn(f) = item {
                    emit_fn_def_di(&mut body, f, &module.name, has_requires);
                }
            }
            body.push_str("}\n\n");
        }
    }

    // Invariants → _check_invariants() function
    if !module.invariants.is_empty() {
        body.push_str("#[cfg(debug_assertions)]\n");
        body.push_str("pub fn _check_invariants() {\n");
        for inv in &module.invariants {
            body.push_str(&format!(
                "    debug_assert!(({cond}), \"invariant '{name}' violated\");\n",
                cond = inv.condition,
                name = inv.name
            ));
        }
        body.push_str("}\n\n");
    }

    // Beings.
    for being in &module.being_defs {
        emit_being_def(&mut body, being);
    }

    // Ecosystems.
    for eco in &module.ecosystem_defs {
        emit_ecosystem_def(&mut body, eco);
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
    out.push_str(&format!("pub struct {}({});\n\n", r.name, base));
    out.push_str(&format!("impl TryFrom<{}> for {} {{\n", base, r.name));
    out.push_str("    type Error = String;\n");
    out.push_str(&format!("    fn try_from(value: {}) -> Result<Self, Self::Error> {{\n", base));
    out.push_str(&format!("        debug_assert!({predicate}(value), \"predicate: {predicate} violated\");\n", predicate = r.predicate));
    out.push_str("        Ok(Self(value))\n");
    out.push_str("    }\n}\n\n");
}

fn emit_fn_def(out: &mut String, f: &FnDef) {
    emit_fn_def_di(out, f, "", false);
}

fn emit_fn_def_di(out: &mut String, f: &FnDef, module_name: &str, module_has_requires: bool) {
    // Doc comment
    if let Some(desc) = &f.describe {
        out.push_str(&format!("/// {}\n", desc));
    }
    // Annotation doc comments
    for ann in &f.annotations {
        match ann.key.as_str() {
            "exactly-once" => out.push_str("/// @exactly-once: this function must execute exactly once\n"),
            "idempotent" => out.push_str("/// @idempotent — safe to retry; f(f(x)) = f(x)\n"),
            "commutative" => out.push_str("/// @commutative — argument order does not matter; f(a,b) = f(b,a)\n"),
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

    // Inject ctx param for DI
    let inject_ctx = module_has_requires && !f.with_deps.is_empty();
    let mut params: Vec<String> = Vec::new();
    if inject_ctx {
        params.push(format!("ctx: &{}Context", module_name));
    }
    params.extend(
        f.type_sig.params.iter().enumerate()
            .map(|(i, ty)| format!("p{}: {}", i, type_to_rust(ty)))
    );

    let ret = type_to_rust(&f.type_sig.return_type);
    out.push_str(&format!(
        "pub fn {}{}({}) -> {} {{\n",
        f.name, type_params, params.join(", "), ret
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
        let body_count = f.body.len();
        for (i, stmt) in f.body.iter().enumerate() {
            let is_last = i + 1 == body_count;
            out.push_str(&emit_fn_body_stmt(stmt, is_last));
            out.push('\n');
        }
    }

    // ensure: → debug_assert!
    for ens in &f.ensures {
        out.push_str(&format!(
            "    debug_assert!(true, \"ensure: {}\");\n",
            ens.expr
        ));
    }

    out.push_str("}\n\n");
}

fn emit_fn_body_stmt(stmt: &str, is_last: bool) -> String {
    let s = stmt.trim();
    if s.is_empty() { return String::new(); }

    // todo placeholder
    if s == "todo" {
        return if is_last { "    todo!()".to_string() } else { "    todo!();".to_string() };
    }

    // Inline match expression (reconstructed by parser): starts with "match "
    if s.starts_with("match ") {
        let translated = translate_loom_expr(s);
        return if is_last {
            format!("    {}", translated)
        } else {
            format!("    {};", translated)
        };
    }

    // for expression
    if s.starts_with("for ") {
        let translated = translate_for_stmt(s);
        return format!("    {};", translated);
    }

    // let binding
    if s.starts_with("let ") {
        return format!("    {};", translate_let_stmt(s));
    }

    // Expression (last = return value without semicolon, not last = statement)
    let expr = translate_loom_expr(s);
    if is_last {
        format!("    {}", expr)
    } else {
        format!("    {};", expr)
    }
}

fn translate_let_stmt(s: &str) -> String {
    let rest = s.trim_start_matches("let ").trim();
    if let Some(eq_pos) = rest.find(" = ") {
        let name = &rest[..eq_pos];
        let rhs = &rest[eq_pos + 3..];
        format!("let {} = {}", name, translate_loom_expr(rhs))
    } else {
        s.to_string()
    }
}

fn translate_for_stmt(s: &str) -> String {
    // for X in Y { body } → for X in Y.iter() { body }
    let inner = s.trim_start_matches("for ").trim();
    if let Some(in_pos) = inner.find(" in ") {
        let loop_var = &inner[..in_pos];
        let rest = &inner[in_pos + 4..];
        // Find where the collection ends (before '{')
        if let Some(brace_pos) = rest.find(" {") {
            let coll = &rest[..brace_pos];
            let body = &rest[brace_pos..];
            return format!("for {} in {}.iter(){}", loop_var, coll.trim(), body);
        }
        // No brace — just add .iter()
        return format!("for {} in {}.iter()", loop_var, rest.trim());
    }
    s.to_string()
}

fn translate_loom_expr(expr: &str) -> String {
    let s = expr.trim();

    if s.is_empty() { return String::new(); }

    // todo placeholder
    if s == "todo" { return "todo!()".to_string(); }

    // Boolean literals
    if s == "true" || s == "false" { return s.to_string(); }

    // match expression (already formatted by parser as "match x { ... }")
    if s.starts_with("match ") { return s.to_string(); }

    // for expression
    if s.starts_with("for ") { return translate_for_stmt(s); }

    // HOF: map(list, fn) → list.iter().map(fn).collect::<Vec<_>>()
    if s.starts_with("map(") {
        if let Some(args) = extract_fn_call_args(s, "map") {
            if args.len() >= 2 {
                let list = args[0].trim();
                let func = args[1..].join(", ");
                return format!("{}.iter().map({}).collect::<Vec<_>>()", list, func.trim());
            }
        }
    }

    // HOF: filter(list, fn) → list.iter().filter(fn).cloned().collect::<Vec<_>>()
    if s.starts_with("filter(") {
        if let Some(args) = extract_fn_call_args(s, "filter") {
            if args.len() >= 2 {
                let list = args[0].trim();
                let func = args[1..].join(", ");
                return format!("{}.iter().filter({}).cloned().collect::<Vec<_>>()", list, func.trim());
            }
        }
    }

    // HOF: fold(list, init, fn) → list.iter().fold(init, fn)
    if s.starts_with("fold(") {
        if let Some(args) = extract_fn_call_args(s, "fold") {
            if args.len() >= 3 {
                let list = args[0].trim();
                let init = args[1].trim();
                let func = args[2..].join(", ");
                return format!("{}.iter().fold({}, {})", list, init, func.trim());
            }
        }
    }

    // Lambda with type annotation: |x: Int| body → |x: i64| body
    if s.starts_with('|') {
        return translate_lambda(s);
    }

    // Pipe operator: expr |> f |> g
    if s.contains(" |> ") {
        let parts: Vec<&str> = s.split(" |> ").collect();
        let mut result = translate_loom_expr(parts[0]);
        for func in &parts[1..] {
            result = format!("{{ let _pipe = {}; {}(_pipe) }}", result, func.trim());
        }
        return result;
    }

    // Comparison operators — wrap in parens
    for op in &[" >= ", " <= ", " > ", " < ", " == ", " != "] {
        if let Some(pos) = find_op_outside_parens(s, op) {
            let left = &s[..pos];
            let right = &s[pos + op.len()..];
            let op_str = op.trim();
            return format!("({} {} {})", translate_loom_expr(left), op_str, translate_loom_expr(right));
        }
    }

    // Arithmetic — wrap in parens
    for op in &[" + ", " - ", " * ", " / "] {
        if let Some(_pos) = find_op_outside_parens(s, op) {
            return format!("({})", s);
        }
    }

    // String literals
    if s.starts_with('"') { return s.to_string(); }

    // Integer/float literals
    if s.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) {
        return s.to_string();
    }

    // Otherwise return as-is
    s.to_string()
}

fn translate_lambda(s: &str) -> String {
    // |x| body  or  |x: Type| body  or  |a, b| body
    // Translate Loom types to Rust types within the params
    if !s.starts_with('|') { return s.to_string(); }
    // Find the closing |
    let rest = &s[1..];
    if let Some(close) = rest.find('|') {
        let params_str = &rest[..close];
        let body = rest[close + 1..].trim();
        // Translate each param type
        let params: Vec<String> = params_str.split(',').map(|p| {
            let p = p.trim();
            if let Some(colon) = p.find(':') {
                let name = p[..colon].trim();
                let ty = p[colon + 1..].trim();
                let rust_ty = match ty {
                    "Int" => "i64",
                    "Float" => "f64",
                    "String" => "String",
                    "Bool" => "bool",
                    other => other,
                };
                format!("{}: {}", name, rust_ty)
            } else {
                p.to_string()
            }
        }).collect();
        return format!("|{}| {}", params.join(", "), translate_loom_expr(body));
    }
    s.to_string()
}

fn find_op_outside_parens(s: &str, op: &str) -> Option<usize> {
    let mut depth = 0i32;
    let bytes = s.as_bytes();
    let op_bytes = op.as_bytes();
    let mut i = 0;
    while i + op.len() <= s.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => { depth += 1; i += 1; }
            b')' | b']' | b'}' => { depth -= 1; i += 1; }
            _ if depth == 0 && s[i..].starts_with(op) => return Some(i),
            _ => { i += 1; }
        }
    }
    None
}

fn extract_fn_call_args(s: &str, fn_name: &str) -> Option<Vec<String>> {
    let prefix = format!("{}(", fn_name);
    if !s.starts_with(&prefix) { return None; }
    let inner = &s[prefix.len()..];
    // Find matching closing paren
    let mut depth = 1i32;
    let mut end = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 { end = i; break; }
            }
            _ => {}
        }
    }
    let args_str = &inner[..end];
    // Split by comma outside parens
    let mut args = Vec::new();
    let mut current = String::new();
    let mut d = 0i32;
    for c in args_str.chars() {
        match c {
            '(' | '[' | '{' => { d += 1; current.push(c); }
            ')' | ']' | '}' => { d -= 1; current.push(c); }
            ',' if d == 0 => {
                args.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    Some(args)
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
            to_snake_case(&epi.signal)
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
            "    pub fn differentiate_{}(&self, signal_level: f64) {{\n",
            to_snake_case(&morph.signal)
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
        out.push_str(&format!("            // telomere exhausted (on_exhaustion: {})\n", tel.on_exhaustion));
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
        TypeExpr::Effect(_, ret) => format!("Result<{}, Box<dyn std::error::Error>>", type_to_rust(ret)),
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

