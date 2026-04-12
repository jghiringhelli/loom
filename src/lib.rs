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

pub mod alx;
pub mod ast;
pub mod checker;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod lpn;
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
    let tokens = lexer::Lexer::tokenize(source)?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| vec![e])?;

    for stage in build_checker_pipeline() {
        stage.run(&module)?;
    }

    let smt_errors: Vec<LoomError> = run_smt_verification(&module);
    if !smt_errors.is_empty() {
        return Err(smt_errors);
    }

    Ok(codegen::RustEmitter::new().emit(&module))
}

/// Construct the ordered checker pipeline for the Rust compilation path.
///
/// Each [`CheckerStage`] wraps one checker with its suppression policy.
/// Ordering matters: type inference runs before type checking, which runs
/// before exhaustiveness, which runs before effect checking, etc.
fn build_checker_pipeline() -> Vec<checker::CheckerStage> {
    use checker::{
        AlgebraicChecker, AspectChecker, BoundaryChecker, CanalizationChecker, CategoryChecker,
        CheckerStage, CheckpointChecker, CognitiveMemoryChecker, CriticalityChecker,
        CurryHowardChecker, DegeneracyChecker, DependentChecker, EffectChecker,
        EffectHandlerChecker, EntityChecker, ErrorCorrectionChecker, EvolutionVectorChecker,
        ExhaustivenessChecker, GradualChecker, HgtChecker, InferenceEngine, JournalChecker,
        ManifestChecker, MessagingChecker, MigrationChecker, MinimalChecker,
        NicheConstructionChecker, PathwayChecker, PrivacyChecker, ProbabilisticChecker,
        PropertyChecker, ProvenanceChecker, RandomnessCheckerAdapter, RefinementChecker,
        ResonanceChecker, SafetyCheckerAdapter, ScenarioChecker, SelfCertChecker, SemiosisChecker,
        SenescenceChecker, SeparationChecker, SessionChecker, SideChannelChecker,
        SignalAttentionChecker, StochasticCheckerAdapter, StoreChecker, SymbiosisChecker,
        TeleosCheckerAdapter, TemporalChecker, TensorChecker, TypeChecker, TypestateChecker,
        UmweltChecker, UnitsChecker, UseCaseChecker,
    };
    vec![
        // Core type system
        CheckerStage::hard(InferenceEngine::new()),
        CheckerStage::hard(AspectChecker::new()),
        CheckerStage::hard(TypeChecker::new()),
        CheckerStage::hard(RefinementChecker::new()),
        CheckerStage::hard(ExhaustivenessChecker::new()),
        CheckerStage::hard(EffectChecker::new()),
        CheckerStage::hard(AlgebraicChecker::new()),
        CheckerStage::hard(UnitsChecker::new()),
        // Extended type disciplines
        CheckerStage::hard(TypestateChecker::new()),
        CheckerStage::hard(TemporalChecker::new()),
        CheckerStage::hard(SeparationChecker::new()),
        CheckerStage::hard(GradualChecker::new()),
        CheckerStage::hard(ProbabilisticChecker::new()),
        CheckerStage::hard(DependentChecker::new()),
        CheckerStage::hard(SideChannelChecker::new()),
        CheckerStage::hard(CategoryChecker::new()),
        CheckerStage::hard(CurryHowardChecker::new()),
        CheckerStage::hard(SelfCertChecker::new()),
        CheckerStage::hard(TensorChecker::new()),
        CheckerStage::hard(PrivacyChecker::new()),
        // Biological / domain checkers
        CheckerStage::hard(TeleosCheckerAdapter),
        CheckerStage::hard(SafetyCheckerAdapter),
        CheckerStage::hard(SessionChecker::new()),
        CheckerStage::hard(EffectHandlerChecker::new()),
        CheckerStage::hard(RandomnessCheckerAdapter),
        CheckerStage::hard(StochasticCheckerAdapter),
        CheckerStage::hard(CanalizationChecker::new()),
        CheckerStage::hard(SenescenceChecker::new()),
        CheckerStage::hard(CriticalityChecker::new()),
        CheckerStage::hard(HgtChecker::new()),
        CheckerStage::hard(NicheConstructionChecker::new()),
        CheckerStage::hard(UmweltChecker::new()),
        CheckerStage::hard(ResonanceChecker::new()),
        CheckerStage::hard(PathwayChecker::new()),
        CheckerStage::hard(SymbiosisChecker::new()),
        CheckerStage::hard(ErrorCorrectionChecker::new()),
        CheckerStage::hard(DegeneracyChecker::new()),
        CheckerStage::hard(CheckpointChecker::new()),
        CheckerStage::hard(SemiosisChecker::new()),
        // Store / persistence
        CheckerStage::suppressing(StoreChecker::new(), &["[hint]", "[warn]", "[info]"]),
        // Documentation / contract liveness
        CheckerStage::warn_only(UseCaseChecker::new()),
        CheckerStage::warn_only(ManifestChecker::new()),
        CheckerStage::suppressing(MigrationChecker::new(), &["[warn]", "[info]"]),
        CheckerStage::suppressing(MinimalChecker::new(), &["[info]"]),
        CheckerStage::warn_only(JournalChecker::new()),
        CheckerStage::warn_only(ScenarioChecker::new()),
        CheckerStage::warn_only(PropertyChecker::new()),
        CheckerStage::warn_only(ProvenanceChecker::new()),
        CheckerStage::warn_only(BoundaryChecker::new()),
        // Evolution / memory (M111-M116)
        CheckerStage::suppressing(EvolutionVectorChecker::new(), &["[warn]", ""]),
        CheckerStage::warn_only(CognitiveMemoryChecker::new()),
        // M115: Signal attention filter validation
        CheckerStage::hard(SignalAttentionChecker::new()),
        // M116: Messaging primitive validation
        CheckerStage::warn_only(MessagingChecker::new()),
        // M118: Entity annotation coherence
        CheckerStage::hard(EntityChecker::new()),
    ]
}

/// Run the SMT contract verification bridge and collect any counterexample errors.
///
/// Returns counterexample [`LoomError`]s only — `Skipped` and `Valid` results are silent.
fn run_smt_verification(module: &ast::Module) -> Vec<LoomError> {
    checker::SmtBridgeChecker::check(&module.items)
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
        .collect()
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
    let tokens = lexer::Lexer::tokenize(source).map_err(|es| {
        es.iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
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
    let tokens = lexer::Lexer::tokenize(source).map_err(|es| {
        es.iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
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
    let tokens = lexer::Lexer::tokenize(source).map_err(|es| {
        es.iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
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
    let tokens = lexer::Lexer::tokenize(source).map_err(|es| {
        es.iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let module = parser::Parser::new(&tokens)
        .parse_module()
        .map_err(|e| format!("{}", e))?;
    Ok(codegen::MermaidEmitter::new().emit_flow(&module))
}
