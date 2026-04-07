//! Loom compiler library.
//!
//! Exposes all pipeline stages and the high-level [`compile`] function that
//! orchestrates the full lexer → parser → type-checker → effect-checker →
//! code-generator pipeline.
//!
//! # Quick start
//!
//! ```rust,ignore
//! match loom::compile(source) {
//!     Ok(rust_src) => println!("{}", rust_src),
//!     Err(errors)  => errors.iter().for_each(|e| eprintln!("{}", e)),
//! }
//! ```

#![allow(missing_docs)] // Phase 1: docs are on public items; fields documented in Phase 2

pub mod ast;
pub mod checker;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod project;
pub mod stdlib;

pub use error::LoomError;

// ── Public pipeline entry point ───────────────────────────────────────────────

/// Compile a Loom source string to a Rust source string.
///
/// Runs the full pipeline:
///
/// 1. **Lexer** — tokenise `source` into `(Token, Span)` pairs.
/// 2. **Parser** — parse the token stream into an [`ast::Module`].
/// 3. **Type inference** — HM unification, validates body types match signatures.
/// 4. **Type checker** — validate symbols and patterns.
/// 5. **Exhaustiveness checker** — verify all `match` arms are exhaustive.
/// 6. **Effect checker** — validate effect declarations.
/// 7. **Code generator** — emit Rust source.
///
/// Returns `Ok(rust_source)` on success.  On failure, returns all accumulated
/// [`LoomError`]s so the caller can display the complete diagnostic list.
pub fn compile(source: &str) -> Result<String, Vec<LoomError>> {
    // ── Stage 1: lex ──────────────────────────────────────────────────────
    let tokens = lexer::Lexer::tokenize(source)?;

    // ── Stage 2: parse ────────────────────────────────────────────────────
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;

    // ── Stage 3: type inference ───────────────────────────────────────────
    checker::InferenceEngine::new().check(&module)?;

    // ── Stage 2b: aspect-oriented specification check ─────────────────────
    checker::AspectChecker::new().check(&module)?;

    // ── Stage 4: type check ───────────────────────────────────────────────
    checker::TypeChecker::new().check(&module)?;

    // ── Stage 4b: refinement type check ──────────────────────────────────
    checker::RefinementChecker::new().check(&module)?;

    // ── Stage 5: exhaustiveness check ────────────────────────────────────
    checker::ExhaustivenessChecker::new().check(&module)?;

    // ── Stage 6: effect check ─────────────────────────────────────────────
    checker::EffectChecker::new().check(&module)?;

    // ── Stage 7: algebraic property check ────────────────────────────────
    checker::AlgebraicChecker::new().check(&module)?;

    // ── Stage 8: units of measure check ──────────────────────────────────
    checker::UnitsChecker::new().check(&module)?;

    // ── Stage 9b: typestate check ─────────────────────────────────────────
    checker::TypestateChecker::new().check(&module)?;

    // ── Stage 9b2: temporal logic check ───────────────────────────────────
    checker::TemporalChecker::new().check(&module)?;

    // ── Stage 9b3: separation logic check ─────────────────────────────────
    checker::SeparationChecker::new().check(&module)?;

    // ── Stage 9f: gradual typing check ───────────────────────────────────────
    checker::GradualChecker::new().check(&module)?;

    // ── Stage 9g: probabilistic types check ──────────────────────────────────
    checker::ProbabilisticChecker::new().check(&module)?;

    // ── Stage 9h: dependent types check ──────────────────────────────────────
    checker::DependentChecker::new().check(&module)?;

    // ── Stage 9i: side-channel timing check ──────────────────────────────────
    checker::SideChannelChecker::new().check(&module)?;

    // ── Stage 9j: category theory check ──────────────────────────────────────
    checker::CategoryChecker::new().check(&module)?;

    // ── Stage 9k: Curry-Howard correspondence check ───────────────────────────
    checker::CurryHowardChecker::new().check(&module)?;

    // ── Stage 9l: self-certifying compilation check ───────────────────────────
    checker::SelfCertChecker::new().check(&module)?;

    // ── Stage 9o: store declaration check ────────────────────────────────────
    // Hints and warnings (prefixed with [hint]/[warn]/[info]) are informational;
    // only hard structural errors (missing key, missing embedding, duplicate PK)
    // block compilation.
    let store_errors: Vec<LoomError> = checker::StoreChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| {
            let msg = format!("{}", e);
            !msg.contains("[hint]") && !msg.contains("[warn]") && !msg.contains("[info]")
        })
        .collect();
    if !store_errors.is_empty() {
        return Err(store_errors);
    }

    // ── Stage 9p: tensor type check ──────────────────────────────────────────
    checker::TensorChecker::new().check(&module)?;

    // ── Stage 9c: privacy check ───────────────────────────────────────────
    checker::PrivacyChecker::new().check(&module)?;

    // ── Stage 9d: teleological check ─────────────────────────────────────
    checker::check_teleos(&module).map_err(|es| es)?;

    // ── Stage 9e: safety check ────────────────────────────────────────────
    let safety_errors = checker::SafetyChecker::check(&module);
    if !safety_errors.is_empty() {
        return Err(safety_errors);
    }

    // ── Stage 9m: session type duality check (M98) ────────────────────────
    let session_errors = checker::SessionChecker::new().check(&module);
    if !session_errors.is_empty() {
        return Err(session_errors);
    }

    // ── Stage 9n: algebraic effect handler exhaustiveness (M99) ──────────
    let effect_handler_errors = checker::EffectHandlerChecker::new().check(&module);
    if !effect_handler_errors.is_empty() {
        return Err(effect_handler_errors);
    }

    // ── Stage 9q: use-case triple-derivation check (M110) ────────────────
    let uc_errors: Vec<LoomError> = checker::UseCaseChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !uc_errors.is_empty() {
        return Err(uc_errors);
    }

    // ── Stage 9o: randomness quality check (M85) ─────────────────────────
    let mut randomness_errors = Vec::new();
    checker::RandomnessChecker::check(&module, &mut randomness_errors);
    if !randomness_errors.is_empty() {
        return Err(randomness_errors);
    }

    // ── Stage 9p: stochastic process check (M88) ─────────────────────────
    let mut stochastic_errors = Vec::new();
    checker::StochasticChecker::check(&module, &mut stochastic_errors);
    if !stochastic_errors.is_empty() {
        return Err(stochastic_errors);
    }

    // ── Stage 9q: SMT contract verification bridge (M100) ────────────────
    // Counterexamples are hard errors; Skipped/Unknown are informational.
    let smt_results = checker::SmtBridgeChecker::check(&module.items);
    let smt_errors: Vec<LoomError> = smt_results
        .into_iter()
        .filter_map(|v| match &v.status {
            ast::SmtStatus::Counterexample(msg) => Some(LoomError::parse(
                format!(
                    "fn '{}': SMT counterexample found — spec is contradictory: {}",
                    v.function, msg
                ),
                ast::Span::synthetic(),
            )),
            _ => None,
        })
        .collect();
    if !smt_errors.is_empty() {
        return Err(smt_errors);
    }

    // ── Stage 9r: manifest documentation liveness check (M101) ──────────
    // Missing files are hard errors; unknown-symbol warnings are filtered out.
    let manifest_errors: Vec<LoomError> = checker::ManifestChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !manifest_errors.is_empty() {
        return Err(manifest_errors);
    }

    // ── Stage 9s: migration evolution contract check (M106) ───────────────
    let migration_errors: Vec<LoomError> = checker::MigrationChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| {
            let msg = format!("{}", e);
            !msg.contains("[warn]") && !msg.contains("[info]")
        })
        .collect();
    if !migration_errors.is_empty() {
        return Err(migration_errors);
    }

    // ── Stage 9t: minimal dead-declaration check (M107) ───────────────────
    let minimal_errors: Vec<LoomError> = checker::MinimalChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| {
            let msg = format!("{}", e);
            !msg.contains("[info]")
        })
        .collect();
    if !minimal_errors.is_empty() {
        return Err(minimal_errors);
    }

    // ── Stage 9u: journal episodic memory check (M104) ────────────────────
    let journal_errors: Vec<LoomError> = checker::JournalChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !journal_errors.is_empty() {
        return Err(journal_errors);
    }

    // ── Stage 9v: scenario executable acceptance criteria check (M105) ────
    let scenario_errors: Vec<LoomError> = checker::ScenarioChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !scenario_errors.is_empty() {
        return Err(scenario_errors);
    }

    // ── Stage 9w: property-based test block check (M109) ──────────────────
    let property_errors: Vec<LoomError> = checker::PropertyChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !property_errors.is_empty() {
        return Err(property_errors);
    }

    // ── Stage 9w: property-based test validation (M109) ───────────────────

    // ── Stage 9x: provenance lineage check (M102) ─────────────────────────
    let provenance_errors: Vec<LoomError> = checker::ProvenanceChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !provenance_errors.is_empty() {
        return Err(provenance_errors);
    }

    // ── Stage 9y: boundary information hiding check (M103) ────────────────
    let boundary_errors: Vec<LoomError> = checker::BoundaryChecker::new()
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    if !boundary_errors.is_empty() {
        return Err(boundary_errors);
    }

    // ── Stage 9z: evolution vector semantics (M111) ────────────────────────
    // Detects duplicate and related migration patterns across beings.
    // Warns only — does not block compilation. Hard errors from MigrationChecker
    // (chain breaks, cycles) have already fired in stage 9v.
    let _evolution_warnings: Vec<LoomError> = checker::EvolutionVectorChecker::new()
        .check(&module);
    // Warnings are informational — they do not block codegen.

    // ── Stage 9: code generation ──────────────────────────────────────────
    Ok(codegen::RustEmitter::new().emit(&module))
}

// ── TypeScript pipeline entry point ──────────────────────────────────────────

/// Compile a Loom source string to a JSON Schema document.
///
/// Emits a JSON Schema draft 2020-12 document with `$defs` for every type
/// definition in the module.
pub fn compile_json_schema(source: &str) -> Result<String, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;
    checker::TypeChecker::new().check(&module)?;
    Ok(codegen::JsonSchemaEmitter::new().emit(&module))
}

/// Compile a Loom source string to an OpenAPI 3.0.3 JSON document.
///
/// Emits paths/operations from functions, components/schemas from types.
pub fn compile_openapi(source: &str) -> Result<String, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;
    checker::TypeChecker::new().check(&module)?;
    checker::AlgebraicChecker::new().check(&module)?;
    Ok(codegen::OpenApiEmitter::new().emit(&module))
}
///
/// Runs the full lex → parse → type-check pipeline, then emits TypeScript.
pub fn compile_typescript(source: &str) -> Result<String, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;
    checker::InferenceEngine::new().check(&module)?;
    checker::AspectChecker::new().check(&module)?;
    checker::TypeChecker::new().check(&module)?;
    checker::ExhaustivenessChecker::new().check(&module)?;
    checker::EffectChecker::new().check(&module)?;
    checker::AlgebraicChecker::new().check(&module)?;
    checker::UnitsChecker::new().check(&module)?;
    Ok(codegen::TypeScriptEmitter::new().emit(&module))
}

// ── WASM pipeline entry point ─────────────────────────────────────────────────

/// Compile a Loom source to a Mesa Python ABM simulation.
///
/// Runs lex → parse → type-check → teleos-check, then emits a Mesa
/// agent-based simulation. Each `being:` becomes an `Agent` class;
/// each `ecosystem:` becomes a `Model` class.
pub fn compile_simulation(source: &str) -> Result<String, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;
    checker::TypeChecker::new().check(&module)?;
    checker::check_teleos(&module).map_err(|es| es)?;
    Ok(codegen::SimulationEmitter::new().emit(&module))
}

/// Compile a Loom source string to a NeuroML 2 XML document.
///
/// Only `being:` blocks that declare at least one `plasticity:` block are
/// emitted as `<cell>` elements; `ecosystem:` blocks emit as `<network>`.
pub fn compile_neuroml(source: &str) -> Result<String, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;
    checker::TypeChecker::new().check(&module)?;
    Ok(codegen::NeuroMLEmitter::emit(&module))
}

///
/// Runs the lex → parse → inference → type-check → exhaustiveness-check
/// pipeline, then emits WAT instead of Rust.  Only the M3 supported subset
/// is accepted; any unsupported construct (effect types, enums, refined types,
/// match expressions) returns a [`LoomError::WasmUnsupported`] error.
///
/// Returns `Ok(wat_source)` on success.  On failure, returns all accumulated
/// [`LoomError`]s.
pub fn compile_wasm(source: &str) -> Result<String, Vec<LoomError>> {
    // ── Stage 1: lex ──────────────────────────────────────────────────────
    let tokens = lexer::Lexer::tokenize(source)?;

    // ── Stage 2: parse ────────────────────────────────────────────────────
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;

    // ── Stage 3: type inference ───────────────────────────────────────────
    checker::InferenceEngine::new().check(&module)?;

    // ── Stage 4: type check ───────────────────────────────────────────────
    checker::TypeChecker::new().check(&module)?;

    // ── Stage 5: exhaustiveness check ────────────────────────────────────
    checker::ExhaustivenessChecker::new().check(&module)?;

    // ── Stage 6: WASM code generation ────────────────────────────────────
    codegen::WasmEmitter::new().emit(&module)
}

/// Parse a Loom source string and return the AST module.
///
/// Runs only lex + parse — no type checking or code generation.
/// Useful for testing parser behaviour in isolation.
pub fn parse(source: &str) -> Result<ast::Module, Vec<LoomError>> {
    let tokens = lexer::Lexer::tokenize(source)?;
    parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])
}

// ── M108: Mermaid diagram emission ───────────────────────────────────────────

/// Emit a Mermaid C4 container diagram from being/fn structure.
///
/// Runs lex + parse only; no semantic checks required for diagram emission.
/// Diagrams cannot drift from code because they ARE derived from the code.
/// C4 model (Simon Brown 2018) + Mermaid (Sveidqvist 2019).
pub fn compile_mermaid_c4(source: &str) -> Result<String, String> {
    let tokens = lexer::Lexer::tokenize(source)
        .map_err(|es| es.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("; "))?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| format!("{}", e))?;
    Ok(codegen::MermaidEmitter::new().emit_c4(&module))
}

/// Emit a Mermaid sequence diagram from session type declarations.
///
/// Runs lex + parse only. Each session role → participant; Send steps with
/// duality declarations → `->>` arrows. Honda (1993) session types.
pub fn compile_mermaid_sequence(source: &str) -> Result<String, String> {
    let tokens = lexer::Lexer::tokenize(source)
        .map_err(|es| es.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("; "))?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| format!("{}", e))?;
    Ok(codegen::MermaidEmitter::new().emit_sequence(&module))
}

/// Emit a Mermaid state diagram from lifecycle declarations.
///
/// Runs lex + parse only. Each `lifecycle T :: S1 -> S2 -> S3` becomes
/// adjacent `S1 --> S2 --> S3` transitions in `stateDiagram-v2` syntax.
pub fn compile_mermaid_state(source: &str) -> Result<String, String> {
    let tokens = lexer::Lexer::tokenize(source)
        .map_err(|es| es.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("; "))?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| format!("{}", e))?;
    Ok(codegen::MermaidEmitter::new().emit_state(&module))
}

/// Emit a Mermaid flow diagram from fn declarations.
///
/// Runs lex + parse only. Top-level `fn` items → `flowchart TD` nodes
/// with sequential edges from Start through each function to End.
pub fn compile_mermaid_flow(source: &str) -> Result<String, String> {
    let tokens = lexer::Lexer::tokenize(source)
        .map_err(|es| es.iter().map(|e| format!("{}", e)).collect::<Vec<_>>().join("; "))?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| format!("{}", e))?;
    Ok(codegen::MermaidEmitter::new().emit_flow(&module))
}
