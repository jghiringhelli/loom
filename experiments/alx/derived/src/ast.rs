// ALX: derived from loom.loom §"AST: Type expressions" through §"Top-level module"
// All AST node types mirror the Loom spec exactly.

// Re-export Span so tests can write `loom::ast::Span`.
pub use crate::error::Span;

// ── Type expressions ──────────────────────────────────────────────────────────

/// A type expression in the Loom AST.
/// TypeVar is only produced by HM inference, never by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A bare name: Int, Float, String, Bool, Unit, or a user type name.
    Base(String),
    /// A generic application: List<T>, Option<T>, Result<T,E>, Float<usd>, etc.
    /// The inner Vec holds type arguments.
    Generic(String, Vec<TypeExpr>),
    /// An effectful type: Effect<[E1,E2], T>
    /// First element is the effect list (stored as strings), second is return type.
    Effect(Vec<String>, Box<TypeExpr>),
    /// Option shorthand: Option<T>
    Option(Box<TypeExpr>),
    /// Result shorthand: Result<T, E>
    Result(Box<TypeExpr>, Box<TypeExpr>),
    /// Tuple type: (A, B) or (A, B, C)
    Tuple(Vec<TypeExpr>),
    /// A type variable produced by HM inference — never parsed.
    TypeVar(u32),
    /// Function type: A -> B
    Fn(Box<TypeExpr>, Box<TypeExpr>),
}

impl TypeExpr {
    /// Return true if this type carries a unit label (e.g. Float<usd>).
    pub fn unit_label(&self) -> Option<&str> {
        match self {
            TypeExpr::Generic(name, args) if name == "Float" && args.len() == 1 => {
                if let TypeExpr::Base(unit) = &args[0] {
                    Some(unit.as_str())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// ── Fields and annotations ────────────────────────────────────────────────────

/// A key-value annotation: @key or @key("value") or @key(N).
/// G6: Annotation has only key and value — no span field.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub key: String,
    pub value: String,
}

impl Annotation {
    pub fn bare(key: impl Into<String>) -> Self {
        Annotation { key: key.into(), value: String::new() }
    }
    pub fn with_value(key: impl Into<String>, value: impl Into<String>) -> Self {
        Annotation { key: key.into(), value: value.into() }
    }
}

/// A field in a product type or matter block.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

// ── Core definitions ──────────────────────────────────────────────────────────

/// Product type (struct).
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// A variant of a sum type.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    /// None = unit variant; Some = payload type.
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

/// Sum type (enum).
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// Refined type: `type Email = String where valid_email end`
#[derive(Debug, Clone)]
pub struct RefinedType {
    pub name: String,
    pub base_type: TypeExpr,
    pub predicate: String,
    pub span: Span,
}

/// Function type signature (curried).
#[derive(Debug, Clone)]
pub struct FnTypeSignature {
    pub params: Vec<TypeExpr>,
    pub return_type: TypeExpr,
}

/// A require:/ensure: contract clause.
#[derive(Debug, Clone)]
pub struct Contract {
    pub expr: String,
    pub span: Span,
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub describe: Option<String>,
    pub annotations: Vec<Annotation>,
    pub type_params: Vec<String>,
    pub type_sig: FnTypeSignature,
    /// Effect names declared in the Effect<[...]> position.
    pub effect_tiers: Vec<String>,
    pub requires: Vec<Contract>,
    pub ensures: Vec<Contract>,
    pub with_deps: Vec<String>,
    /// Body lines (raw text after the signature, before `end`).
    pub body: Vec<String>,
    pub span: Span,
}

// ── Semantic constructs ───────────────────────────────────────────────────────

/// lifecycle Payment :: Pending -> Completed -> Failed
#[derive(Debug, Clone)]
pub struct LifecycleDef {
    pub type_name: String,
    pub states: Vec<String>,
    pub span: Span,
}

/// flow secret :: Password, Token
#[derive(Debug, Clone)]
pub struct FlowLabel {
    pub label: String,
    pub types: Vec<String>,
    pub span: Span,
}

// ── Biological constructs (M41–M55) ──────────────────────────────────────────

/// Search strategy for evolve: block.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    GradientDescent,
    StochasticGradient,
    SimulatedAnnealing,
    DerivativeFree,
    Mcmc,
}

/// A single search case in evolve:
#[derive(Debug, Clone)]
pub struct SearchCase {
    pub strategy: SearchStrategy,
    pub when: String,
}

/// evolve: block — directed search toward telos.
#[derive(Debug, Clone)]
pub struct EvolveBlock {
    pub search_cases: Vec<SearchCase>,
    /// Must contain "decreasing", "non-increasing", or "converg".
    pub constraint: String,
    pub span: Span,
}

/// regulate: block — homeostatic regulation.
#[derive(Debug, Clone)]
pub struct RegulateBlock {
    pub variable: String,
    pub target: String,
    pub bounds: Option<(String, String)>,
    pub response: Vec<String>,
    pub span: Span,
}

/// telos: block — final cause (required on every being:).
#[derive(Debug, Clone)]
pub struct TelosDef {
    pub description: String,
    pub fitness_fn: Option<String>,
    pub modifiable_by: Option<String>,
    pub bounded_by: Option<String>,
    pub span: Span,
}

/// matter: block — fields/state.
#[derive(Debug, Clone)]
pub struct MatterBlock {
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// form: block — nested type definitions.
#[derive(Debug, Clone)]
pub struct FormBlock {
    pub types: Vec<TypeDef>,
    pub enums: Vec<EnumDef>,
    pub span: Span,
}

/// function: block — method signatures.
#[derive(Debug, Clone)]
pub struct FunctionBlock {
    pub fns: Vec<FnDef>,
    pub span: Span,
}

/// epigenetic: block — Waddington 1957.
/// signal: the trigger signal; modifies: the field being modulated.
#[derive(Debug, Clone)]
pub struct EpigeneticBlock {
    pub signal: String,
    pub modifies: String,
    pub reverts_when: Option<String>,
    pub span: Span,
}

/// morphogen: block — Turing 1952 reaction-diffusion.
/// signal: the morphogen signal; threshold: concentration threshold (String);
/// produces: list of differentiated structure names.
#[derive(Debug, Clone)]
pub struct MorphogenBlock {
    pub signal: String,
    pub threshold: String,
    pub produces: Vec<String>,
    pub span: Span,
}

/// telomere: block — Hayflick 1961 finite replication.
#[derive(Debug, Clone)]
pub struct TelomereBlock {
    pub limit: i64,
    pub on_exhaustion: String,
    pub span: Span,
}

/// crispr: block — Doudna/Charpentier 2012 targeted modification.
/// No preserve field — tests construct without it.
#[derive(Debug, Clone)]
pub struct CrisprBlock {
    pub target: String,
    pub replace: String,
    pub guide: String,
    pub span: Span,
}

/// M49: quorum: block on EcosystemDef. threshold is a String (e.g. "0.6").
#[derive(Debug, Clone)]
pub struct QuorumBlock {
    pub signal: String,
    pub threshold: String,
    pub action: String,
    pub span: Span,
}

/// M50: Plasticity strategy — Hebb (1949) synaptic weight adjustment.
/// Tests use PlasticityRule as the enum (renamed from PlasticityStrategy).
#[derive(Debug, Clone, PartialEq)]
pub enum PlasticityRule {
    Hebbian,
    Boltzmann,
    ReinforcementLearning,
}

/// plasticity: block — Hebb 1949. Flat structure: one trigger/modifies/rule per block.
#[derive(Debug, Clone)]
pub struct PlasticityBlock {
    pub trigger: String,
    pub modifies: String,
    pub rule: PlasticityRule,
    pub span: Span,
}

/// A `being:` block — Aristotle's four causes as a first-class construct.
#[derive(Debug, Clone)]
pub struct BeingDef {
    pub name: String,
    pub describe: Option<String>,
    pub annotations: Vec<Annotation>,
    pub matter: Option<MatterBlock>,
    pub form: Option<FormBlock>,
    pub function: Option<FunctionBlock>,
    pub telos: Option<TelosDef>,
    pub regulate_blocks: Vec<RegulateBlock>,
    pub evolve_block: Option<EvolveBlock>,
    pub epigenetic_blocks: Vec<EpigeneticBlock>,   // G5: Vec not Option, _blocks suffix
    pub morphogen_blocks: Vec<MorphogenBlock>,      // G5: Vec not Option, _blocks suffix
    pub telomere: Option<TelomereBlock>,            // exactly one telomere per being
    pub crispr_blocks: Vec<CrisprBlock>,            // G5: Vec not Option, _blocks suffix
    pub plasticity_blocks: Vec<PlasticityBlock>,    // G5: Vec not Option, _blocks suffix
    /// M51: Maturana/Varela 1972 — self-producing system.
    pub autopoietic: bool,
    pub span: Span,
}

/// A signal channel in an ecosystem.
#[derive(Debug, Clone)]
pub struct SignalDef {
    pub name: String,
    pub from: String,
    pub to: String,
    pub payload: String,
    pub span: Span,
}

/// An `ecosystem:` block — multi-being composition.
#[derive(Debug, Clone)]
pub struct EcosystemDef {
    pub name: String,
    pub describe: Option<String>,
    pub members: Vec<String>,
    pub signals: Vec<SignalDef>,
    pub telos: Option<String>,
    pub quorum_blocks: Vec<QuorumBlock>,   // tests use Vec not Option
    pub span: Span,
}

// ── Interface / provides / requires ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Provides {
    pub interfaces: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Requires {
    pub dependencies: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Invariant {
    pub name: String,
    pub condition: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TestDef {
    pub name: String,
    pub body: String,
    pub span: Span,
}

// ── Top-level item ────────────────────────────────────────────────────────────

/// An item inside a module.
#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDef),
    Type(TypeDef),
    Enum(EnumDef),
    RefinedType(RefinedType),
}

// ── Module ────────────────────────────────────────────────────────────────────

/// The compilation unit. Every Loom source file is one module.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub describe: Option<String>,
    pub annotations: Vec<Annotation>,
    pub imports: Vec<String>,
    pub spec: Option<String>,
    pub interface_defs: Vec<InterfaceDef>,
    pub implements: Vec<String>,
    pub provides: Option<Provides>,
    pub requires: Option<Requires>,
    pub invariants: Vec<Invariant>,
    pub test_defs: Vec<TestDef>,
    pub lifecycle_defs: Vec<LifecycleDef>,
    pub flow_labels: Vec<FlowLabel>,
    pub being_defs: Vec<BeingDef>,
    pub ecosystem_defs: Vec<EcosystemDef>,
    pub items: Vec<Item>,
    pub span: Span,
}

impl Module {
    pub fn new(name: String, span: Span) -> Self {
        Module {
            name,
            describe: None,
            annotations: Vec::new(),
            imports: Vec::new(),
            spec: None,
            interface_defs: Vec::new(),
            implements: Vec::new(),
            provides: None,
            requires: None,
            invariants: Vec::new(),
            test_defs: Vec::new(),
            lifecycle_defs: Vec::new(),
            flow_labels: Vec::new(),
            being_defs: Vec::new(),
            ecosystem_defs: Vec::new(),
            items: Vec::new(),
            span,
        }
    }
}
