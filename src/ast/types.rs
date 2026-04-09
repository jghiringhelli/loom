//! Type system AST nodes — type expressions, function definitions, type declarations.
use super::*;
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
    /// Optional separation logic block (`separation: owns: ... disjoint: ... end`).
    pub separation: Option<SeparationBlock>,
    /// Gradual typing block (`gradual:` clause). M59.
    pub gradual: Option<GradualBlock>,
    /// Probabilistic distribution block (`distribution:` clause). M60.
    pub distribution: Option<DistributionBlock>,
    /// Side-channel timing safety block (`timing_safety:` clause). M62.
    pub timing_safety: Option<TimingSafetyBlock>,
    /// Termination claim (`termination:` clause). M61.
    pub termination: Option<String>,
    /// Curry-Howard proof annotations (`proof:` clauses). M64.
    pub proofs: Vec<ProofAnnotation>,
    /// M68: Degeneracy block (Edelman) — primary and fallback implementations.
    pub degenerate: Option<DegenerateBlock>,
    /// M88: Stochastic process annotation block (`process:` clause).
    pub stochastic_process: Option<StochasticProcessBlock>,
    /// M99: Optional algebraic effect handler block (`handle … with … end`).
    pub handle_block: Option<HandleBlock>,
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


/// Product type (struct / record).
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
    /// M73: Optional error correction handler (on_violation).
    pub on_violation: Option<String>,
    /// M73: Optional repair function.
    pub repair_fn: Option<String>,
    pub span: Span,
}

/// Proposition (dependent type claim). M61.
#[derive(Debug, Clone, PartialEq)]
pub struct PropositionDef {
    pub name: String,
    pub base_type: TypeExpr,
    pub predicate: Option<Expr>,
    pub span: Span,
}

/// A law declaration inside a functor or monad. M63.
#[derive(Debug, Clone, PartialEq)]
pub struct LawDecl {
    pub name: String,
    pub span: Span,
}

/// Functor definition — category theory. M63.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctorDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub laws: Vec<LawDecl>,
    pub span: Span,
}

/// Monad definition — category theory. M63.
#[derive(Debug, Clone, PartialEq)]
pub struct MonadDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub laws: Vec<LawDecl>,
    pub span: Span,
}

/// A single field in a compilation certificate. M65.
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateField {
    pub name: String,
    pub value: String,
    pub span: Span,
}

/// Compilation certificate definition. M65.
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateDef {
    pub fields: Vec<CertificateField>,
    pub span: Span,
}



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
    /// Gradual / dynamic type (written `?` in source). M59.
    Dynamic,
    /// Inference variable introduced by the HM engine. Never produced by the
    /// parser; fully resolved before code generation.
    TypeVar(u32),
    /// M87: Tensor type — multi-dimensional typed array.
    ///
    /// `Tensor<rank, shape, unit>` where:
    /// - `rank`: 0=scalar, 1=vector, 2=matrix, 3+=higher-order
    /// - `shape`: dimension sizes, can be symbolic (e.g. "N", "D") or numeric ("3")
    /// - `unit`: Kennedy unit type (e.g. `Float<Pa>`, `Float`)
    ///
    /// Grounded in differential geometry, quantum mechanics (state vectors),
    /// ML (weight matrices), physics (stress/strain/metric tensors).
    Tensor {
        rank: usize,
        shape: Vec<String>,
        unit: Box<TypeExpr>,
        span: Span,
    },
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

