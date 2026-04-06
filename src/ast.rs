//! Abstract Syntax Tree (AST) node types for the Loom language.
//!
//! All nodes carry a [`Span`] for accurate error reporting and source-map
//! generation.  The AST is a close, lossless representation of the source —
//! no desugaring is performed here.

use std::fmt;

// ── Source position ──────────────────────────────────────────────────────────

/// Byte-offset span into the source string (`start` inclusive, `end` exclusive).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Construct a new `Span` from byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// A synthetic span used for generated or placeholder nodes.
    pub fn synthetic() -> Self {
        Span { start: 0, end: 0 }
    }

    /// Merge two spans into one covering both.
    pub fn merge(a: &Span, b: &Span) -> Self {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

// ── Top-level structure ───────────────────────────────────────────────────────

/// A compiled Loom module — the top-level compilation unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name as written in source (e.g. `PricingEngine`).
    pub name: String,
    /// Optional human-readable description (`describe: "..."`).
    pub describe: Option<String>,
    /// Audit annotations (`@since`, `@decision`, `@deprecated`, `@author`).
    pub annotations: Vec<Annotation>,
    /// Compile-time module imports (`import ModuleName`).
    pub imports: Vec<String>,
    /// Optional spec name this module implements (`spec PricingSpec`).
    pub spec: Option<String>,
    /// Named interface declarations (`interface Foo fn ... end`).
    pub interface_defs: Vec<InterfaceDef>,
    /// Interfaces this module explicitly implements (`implements Foo`).
    pub implements: Vec<String>,
    /// Capabilities the module exposes to callers.
    pub provides: Option<Provides>,
    /// Capabilities the module requires from its environment (DI surface).
    pub requires: Option<Requires>,
    /// Structural invariants declared at module level (`invariant name :: cond`).
    pub invariants: Vec<Invariant>,
    /// Inline test definitions (`test name :: body_expr`).
    pub test_defs: Vec<TestDef>,
    /// Lifecycle (typestate) declarations for this module.
    pub lifecycle_defs: Vec<LifecycleDef>,
    /// Temporal logic property blocks for this module.
    pub temporal_defs: Vec<TemporalDef>,
    /// Being (Aristotelian four-causes) declarations for this module.
    pub being_defs: Vec<BeingDef>,
    /// Ecosystem (multi-being composition) declarations for this module.
    pub ecosystem_defs: Vec<EcosystemDef>,
    /// Information flow label declarations (`flow secret :: TypeA, TypeB`).
    pub flow_labels: Vec<FlowLabel>,
    /// Top-level definitions in declaration order.
    pub items: Vec<Item>,
    pub span: Span,
}

/// A typestate/lifecycle declaration.
///
/// Declares a type that progresses through named states in order.
/// Functions that take/return `TypeName<State>` must respect the declared transitions.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleDef {
    /// The type this lifecycle applies to (e.g., "Connection").
    pub type_name: String,
    /// Ordered list of states (e.g., ["Disconnected", "Connected", "Authenticated"]).
    pub states: Vec<String>,
    pub span: Span,
}

/// A temporal logic property block.
///
/// Declares LTL properties over a lifecycle's state space:
/// `always`, `eventually`, `never`, `precedes`.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalDef {
    /// Name of this temporal property block (e.g., "PaymentRules").
    pub name: String,
    /// Individual temporal properties declared in this block.
    pub properties: Vec<TemporalProperty>,
    pub span: Span,
}

/// A single temporal property within a `temporal` block.
#[derive(Debug, Clone, PartialEq)]
pub enum TemporalProperty {
    /// `always: <predicate>` — holds in every reachable state.
    Always { predicate: Expr, span: Span },
    /// `eventually: <type> reaches <state>` — some future state is reached.
    Eventually { type_name: String, target_state: String, span: Span },
    /// `never: <state> transitions to <state>` — forbidden transition.
    Never { from_state: String, to_state: String, span: Span },
    /// `precedes: <state> before <state>` — ordering constraint.
    Precedes { first: String, second: String, span: Span },
}

/// An information-flow label declaration (`flow secret :: TypeA, TypeB`).
///
/// Reserved for a future privacy/taint-tracking pass; currently parsed as a
/// stub so the AST compiles.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowLabel {
    /// The label name (e.g., `"secret"`, `"public"`).
    pub label: String,
    /// Type names that carry this label.
    pub types: Vec<String>,
    pub span: Span,
}

// ── Being (Aristotelian four causes) ─────────────────────────────────────────

/// A search strategy for directed evolution toward telos.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchStrategy {
    GradientDescent,
    StochasticGradient,
    SimulatedAnnealing,
    DerivativeFree,
    Mcmc,
}

/// A single search case: when this landscape condition holds, use this strategy.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchCase {
    pub strategy: SearchStrategy,
    pub when: String,
}

/// Directed evolution block.
#[derive(Debug, Clone, PartialEq)]
pub struct EvolveBlock {
    pub search_cases: Vec<SearchCase>,
    pub constraint: String,
    pub span: Span,
}

/// Homeostatic regulation block.
#[derive(Debug, Clone, PartialEq)]
pub struct RegulateBlock {
    pub variable: String,
    pub target: String,
    pub bounds: Option<(String, String)>,
    pub response: Vec<(String, String)>,
    pub span: Span,
}

/// Telos definition — the final cause.
#[derive(Debug, Clone, PartialEq)]
pub struct TelosDef {
    pub description: String,
    pub fitness_fn: Option<String>,
    /// Required by `@corrigible`: the authority that may modify this being's telos.
    pub modifiable_by: Option<String>,
    /// Required by `@bounded_telos`: the operational scope that constrains this being.
    pub bounded_by: Option<String>,
    pub span: Span,
}

/// Material cause block.
#[derive(Debug, Clone, PartialEq)]
pub struct MatterBlock {
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// Formal cause block.
#[derive(Debug, Clone, PartialEq)]
pub struct FormBlock {
    pub types: Vec<TypeDef>,
    pub enums: Vec<EnumDef>,
    pub span: Span,
}

/// Efficient cause block.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionBlock {
    pub fns: Vec<FnDef>,
    pub span: Span,
}

/// A signal passing between beings in an ecosystem.
/// Derived from Honda's session types (1993): protocols as structured communication.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalDef {
    pub name: String,
    /// The being sending this signal.
    pub from: String,
    /// The being receiving this signal.
    pub to: String,
    /// The payload type (as a type expression string).
    pub payload: String,
    pub span: Span,
}

/// An ecosystem — a composition of beings with inter-being signaling.
///
/// Expresses how multiple goal-directed entities interact through
/// structured communication channels (session-typed signals).
#[derive(Debug, Clone, PartialEq)]
pub struct EcosystemDef {
    pub name: String,
    pub describe: Option<String>,
    /// The beings participating in this ecosystem.
    pub members: Vec<String>,
    /// Signals flowing between beings.
    pub signals: Vec<SignalDef>,
    /// The combined telos of the ecosystem (emergent purpose).
    pub telos: Option<String>,
    pub quorum_blocks: Vec<QuorumBlock>,
    pub span: Span,
}

/// Epigenetic modulation — behavioral change without structural change.
/// Waddington (1957): the developmental landscape where environment
/// channels phenotype without altering genotype.
#[derive(Debug, Clone, PartialEq)]
pub struct EpigeneticBlock {
    /// The environmental signal that triggers modulation.
    pub signal: String,
    /// The field path being modulated (e.g. "metabolism.rate").
    pub modifies: String,
    /// Condition under which the modulation reverts.
    pub reverts_when: Option<String>,
    pub span: Span,
}

/// Morphogenetic signal — differentiation via threshold crossing.
/// Turing (1952): reaction-diffusion systems produce spatial patterns
/// from homogeneous initial conditions via local activation + lateral inhibition.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphogenBlock {
    /// The morphogenetic signal type name.
    pub signal: String,
    /// Threshold above which differentiation occurs (as string, e.g. "0.8").
    pub threshold: String,
    /// Being types produced when threshold is crossed.
    pub produces: Vec<String>,
    pub span: Span,
}

/// Telomere countdown — finite lifecycle limit.
/// Hayflick (1961): normal human cells divide ~50 times before senescence.
#[derive(Debug, Clone, PartialEq)]
pub struct TelomereBlock {
    /// Maximum number of replications/evolutions before exhaustion.
    pub limit: u64,
    /// Behavior when limit is reached.
    pub on_exhaustion: String,
    pub span: Span,
}

/// A biological being — a self-maintaining, goal-directed entity.
#[derive(Debug, Clone, PartialEq)]
pub struct BeingDef {
    pub name: String,
    pub describe: Option<String>,
    /// Safety and capability annotations (`@mortal`, `@corrigible`, `@sandboxed`,
    /// `@transparent`, `@bounded_telos`).
    pub annotations: Vec<Annotation>,
    pub matter: Option<MatterBlock>,
    pub form: Option<FormBlock>,
    pub function: Option<FunctionBlock>,
    pub telos: Option<TelosDef>,
    pub regulate_blocks: Vec<RegulateBlock>,
    pub evolve_block: Option<EvolveBlock>,
    pub epigenetic_blocks: Vec<EpigeneticBlock>,
    pub morphogen_blocks: Vec<MorphogenBlock>,
    pub telomere: Option<TelomereBlock>,
    /// Whether this being declares itself autopoietic (Maturana/Varela 1972).
    /// Requires: telos + at least one regulate block + evolve block + matter.
    pub autopoietic: bool,
    pub crispr_blocks: Vec<CrisprBlock>,
    pub plasticity_blocks: Vec<PlasticityBlock>,
    pub span: Span,
}

/// CRISPR-directed self-modification — targeted form editing.
/// Doudna/Charpentier (2012): guide RNA directs Cas9 to cut a specific
/// genomic sequence for replacement. In Loom: a guide type directs
/// targeted replacement of a form field — the being modifies its own spec.
/// This is the deepest construct: form: is no longer static.
#[derive(Debug, Clone, PartialEq)]
pub struct CrisprBlock {
    /// The field path to target (e.g. "Genome.error_sequence").
    pub target: String,
    /// The replacement type/value (as string).
    pub replace: String,
    /// The guide mechanism (type name, e.g. "CasProtein").
    pub guide: String,
    pub span: Span,
}

/// Neural plasticity — experience-driven form modification.
/// Hebb (1949): synaptic weights strengthen with co-activation.
/// Boltzmann (1877): energy-based learning via thermal equilibration.
/// In Loom: a trigger signal modifies a form field (weight/connection),
/// implementing adaptive structural change through experience.
#[derive(Debug, Clone, PartialEq)]
pub struct PlasticityBlock {
    /// The experience signal that triggers weight update.
    pub trigger: String,
    /// The form field being modified (e.g. "SynapticWeight").
    pub modifies: String,
    /// The learning rule.
    pub rule: PlasticityRule,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlasticityRule {
    Hebbian,
    Boltzmann,
    ReinforcementLearning,
}

/// Quorum sensing — threshold-triggered collective behavior.
/// Bassler (1999): bacteria coordinate via autoinducer concentration.
/// At threshold, individual behavior gives way to collective action.
/// In Loom: when enough ecosystem members signal a state, emergent
/// collective behavior is triggered.
#[derive(Debug, Clone, PartialEq)]
pub struct QuorumBlock {
    /// The signal type being accumulated (e.g. "AHL").
    pub signal: String,
    /// Threshold as fraction of population (0.0–1.0, e.g. "0.6" = 60%).
    pub threshold: String,
    /// The collective action triggered at threshold.
    pub action: String,
    pub span: Span,
}

/// A named interface definition — a typed capability contract.
///
/// Emitted as a Rust `pub trait`.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    pub name: String,
    /// Method signatures (name + type sig, no bodies).
    pub methods: Vec<(String, FnTypeSignature)>,
    pub span: Span,
}

/// A module-level structural invariant.
///
/// Emitted as a `debug_assert!` inside `_check_invariants()`.
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    pub name: String,
    pub condition: Expr,
    pub span: Span,
}

/// An in-language unit test block.
///
/// Emitted as `#[test] fn name() { body }` inside `#[cfg(test)] mod tests`.
#[derive(Debug, Clone, PartialEq)]
pub struct TestDef {
    pub name: String,
    pub body: Expr,
    pub span: Span,
}

/// Top-level item in a module body.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Fn(FnDef),
    Type(TypeDef),
    Enum(EnumDef),
    RefinedType(RefinedType),
}

// ── Definitions ───────────────────────────────────────────────────────────────

/// Function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    /// Optional human-readable description (`describe: "..."`).
    pub describe: Option<String>,
    /// Audit annotations (`@since`, `@decision`, `@deprecated`, `@pure`, `@author`).
    pub annotations: Vec<Annotation>,
    /// User-declared type parameters (e.g. `<A, B>` in `fn map<A, B>`).
    pub type_params: Vec<String>,
    /// Full type signature (parameter types + return type).
    pub type_sig: FnTypeSignature,
    /// Consequence tiers for declared effects: `[(effect_name, tier)]`.
    /// Populated when `effect [IO@reversible, DB@irreversible]` syntax is used.
    pub effect_tiers: Vec<(String, ConsequenceTier)>,
    /// Pre-conditions (`require:` clauses).
    pub requires: Vec<Contract>,
    /// Post-conditions (`ensure:` clauses).
    pub ensures: Vec<Contract>,
    /// Effect / dependency injections (`with` clause names).
    pub with_deps: Vec<String>,
    /// Body expressions; the last one is the return value.
    pub body: Vec<Expr>,
    pub span: Span,
}

/// Consequence tier for an effectful operation — classifies side-effect severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsequenceTier {
    /// No side effects; referentially transparent.
    Pure,
    /// Side effects that can be undone (e.g. a DB transaction).
    Reversible,
    /// Side effects that cannot be undone (e.g. sending an email).
    Irreversible,
}

/// Audit annotation — key/value metadata embedded in the Loom source.
///
/// Examples: `@since("1.0")`, `@decision("use UUIDs for ids")`,
/// `@deprecated("use charge_v2")`, `@pure`.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub key: String,
    pub value: String,
}

/// A field in a product type definition, with optional privacy annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
    /// Privacy and compliance annotations (`@pii`, `@gdpr`, `@hipaa`, `@pci`, etc.)
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// Product type (record / struct).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// Sum type (enum / union).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// A single variant of an enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    /// Payload type, if any (`| B of T`).
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

/// Refined (constrained) type — a base type plus a compile-time predicate.
///
/// At construction sites a `TryFrom` impl is generated that asserts the
/// predicate at runtime via `debug_assert!`.
#[derive(Debug, Clone, PartialEq)]
pub struct RefinedType {
    pub name: String,
    pub base_type: TypeExpr,
    /// The predicate expression that must hold for any value of this type.
    pub predicate: Expr,
    pub span: Span,
}

// ── Type expressions ──────────────────────────────────────────────────────────

/// Type expression — the right-hand side of a type annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Primitive or user-defined type name (e.g. `Int`, `OrderLine`).
    Base(String),
    /// Parameterised generic type (e.g. `List<T>`, `Map<K, V>`).
    Generic(String, Vec<TypeExpr>),
    /// Effectful type — `Effect<[E1, E2, ...], ReturnType>`.
    Effect(Vec<String>, Box<TypeExpr>),
    /// Optional value — `Option<T>`.
    Option(Box<TypeExpr>),
    /// Fallible value — `Result<T, E>`.
    Result(Box<TypeExpr>, Box<TypeExpr>),
    /// Unnamed tuple — `(A, B, C)`.
    Tuple(Vec<TypeExpr>),
    /// Inference variable introduced by the HM engine. Never produced by the
    /// parser; fully resolved before code generation.
    TypeVar(u32),
}

/// Full function type signature, possibly curried.
///
/// For a curried function `A -> B -> C`, `params = [A, B]` and
/// `return_type = C`.
#[derive(Debug, Clone, PartialEq)]
pub struct FnTypeSignature {
    /// Parameter types in left-to-right order.
    pub params: Vec<TypeExpr>,
    pub return_type: Box<TypeExpr>,
}

// ── Contracts ─────────────────────────────────────────────────────────────────

/// A `require` or `ensure` contract — a boolean expression that must hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    pub expr: Expr,
    pub span: Span,
}

// ── Expressions ───────────────────────────────────────────────────────────────

/// Expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Let binding: `let name = value`.
    Let {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    /// Match expression: `match subject | Pattern -> body end`.
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Function call: `func(arg1, arg2, ...)`.
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Pipe operator: `left |> right`.
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Literal value.
    Literal(Literal),
    /// Identifier reference.
    Ident(String),
    /// Field access: `object.field`.
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Binary operation: `left op right`.
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Raw Rust code block — `inline { rust_code }`.
    ///
    /// The content is emitted verbatim into the Rust output. The type checker,
    /// inference engine, and effect checker treat this node as opaque.
    InlineRust(String),
    /// Type coercion: `expr as Type` — explicit numeric widening or narrowing.
    As(Box<Expr>, TypeExpr),
    /// Lambda (anonymous function): `|param, param| body`.
    ///
    /// Each parameter is `(name, optional_type_annotation)`.
    Lambda {
        params: Vec<(String, Option<TypeExpr>)>,
        body: Box<Expr>,
        span: Span,
    },
    /// For-in loop: `for VAR in ITER { BODY }` — yields `()`.
    ForIn {
        var: String,
        iter: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    /// Tuple construction: `(expr, expr, ...)`.
    Tuple(Vec<Expr>, Span),
    /// Try / propagate operator: `expr?` — maps to Rust's `?`.
    Try(Box<Expr>, Span),
}

/// Match arm: a single branch of a `match` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    /// Optional guard condition (`if cond`).
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

// ── Patterns ──────────────────────────────────────────────────────────────────

/// Pattern used in match arms.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Enum variant pattern, optionally binding sub-patterns.
    Variant(String, Vec<Pattern>),
    /// Variable binding.
    Ident(String),
    /// Wildcard — matches anything, binds nothing.
    Wildcard,
    /// Literal pattern.
    Literal(Literal),
}

// ── Literals ─────────────────────────────────────────────────────────────────

/// Literal value node.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Unit,
}

// ── Binary operators ──────────────────────────────────────────────────────────

/// Binary operator kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

// ── Interface declarations ────────────────────────────────────────────────────

/// What a module exposes as its public interface.
#[derive(Debug, Clone, PartialEq)]
pub struct Provides {
    /// List of `(operation_name, type_signature)` pairs.
    pub ops: Vec<(String, FnTypeSignature)>,
}

/// What a module requires from its environment (dependency injection surface).
#[derive(Debug, Clone, PartialEq)]
pub struct Requires {
    /// List of `(capability_name, type)` pairs.
    pub deps: Vec<(String, TypeExpr)>,
}
