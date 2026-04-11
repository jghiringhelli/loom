//! Abstract Syntax Tree (AST) node types for the Loom language.
//!
//! All nodes carry a [Span] for accurate error reporting and source-map
//! generation.  The AST is a close, lossless representation of the source —
//! no desugaring is performed here.
//!
//! Submodules:
//! - [eing]       — biological entity and its sub-blocks
//! - [xpressions] — expression, pattern, literal, binary operators
//! - [items]       — module-level items (lifecycle, session, effects, stores)
//! - [	ypes]       — type expressions and function definitions

use std::fmt;

mod being;
mod expressions;
mod items;
mod types;

pub use being::*;
pub use expressions::*;
pub use items::*;
pub use types::*;

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
    /// Aspect declarations (AOP, M66).
    pub aspect_defs: Vec<AspectDef>,
    /// Top-level definitions in declaration order.
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Fn(FnDef),
    Type(TypeDef),
    Enum(EnumDef),
    RefinedType(RefinedType),
    /// Proposition (dependent type). M61.
    Proposition(PropositionDef),
    /// Functor declaration. M63.
    Functor(FunctorDef),
    /// Monad declaration. M63.
    Monad(MonadDef),
    /// Self-certifying compilation certificate. M65.
    Certificate(CertificateDef),
    /// Typed annotation declaration — M66b.
    AnnotationDecl(AnnotationDecl),
    /// Module correctness report — M67.
    CorrectnessReport(CorrectnessReport),
    /// Metabolic pathway — M71.
    Pathway(PathwayDef),
    /// Symbiotic import — M72.
    SymbioticImport {
        module: String,
        kind: String,
        span: Span,
    },
    /// Horizontal gene transfer adopt — M75.
    Adopt(AdoptDecl),
    /// Niche construction — M77.
    NicheConstruction(NicheConstructionDef),
    /// Sense declaration — a named signal channel. M81.
    Sense(SenseDef),
    /// Store declaration — first-class persistence schema. M92.
    Store(StoreDef),
    /// Type alias — M87. `type Name = TypeExpr` (no field list).
    TypeAlias(String, TypeExpr, Span),
    /// Session type definition — M98. Honda (1993).
    Session(SessionDef),
    /// Algebraic effect definition — M99. Plotkin & Pretnar (2009).
    Effect(EffectDef),
    /// Use-case triple-derivation block — M110. Jacobson (1992).
    UseCase(UseCaseBlock),
    /// Property-based test declaration — M109.
    /// QuickCheck (Claessen & Hughes 2000) → fast-check → Hypothesis → Loom `property:`.
    Property(PropertyBlock),
    /// Boundary block — module-level API surface declaration. M103.
    BoundaryBlock(BoundaryBlock),
    /// M116: Messaging primitive — typed inter-being communication contract.
    MessagingPrimitive(MessagingPrimitiveDef),
    /// M117: Top-level telos function declaration.
    TelosFunction(TelosFunctionDef),
    /// M118: Universal graph/network entity primitive.
    Entity(EntityDef),
    /// M119: Intent coordinator — living intent with human governance.
    IntentCoordinator(IntentCoordinatorDef),
    /// M141: Explicit forced-discipline declaration.
    Discipline(DisciplineDecl),
    /// M155: Discrete-time Markov chain — first-class module-level item.
    Chain(ChainDef),
    /// M156: Directed Acyclic Graph — first-class module-level item.
    Dag(DagDef),
    /// M157: Named constant — first-class module-level item.
    Const(ConstDef),
    /// M159: Named sequential data-transformation pipeline — first-class module-level item.
    Pipeline(PipelineDef),
    /// M160: Distributed transaction saga — first-class module-level item.
    Saga(SagaDef),
    /// M161: Named domain event — first-class module-level item.
    Event(EventDef),
    /// M162: Named CQRS command — first-class module-level item.
    Command(CommandDef),
    /// M162: Named CQRS query — first-class module-level item.
    Query(QueryDef),
    /// M163: Named circuit breaker — first-class module-level resilience item.
    CircuitBreaker(CircuitBreakerDef),
    /// M164: Named retry policy — first-class module-level resilience item.
    Retry(RetryDef),
    /// M165: Named rate limiter (token bucket) — first-class module-level item.
    RateLimiter(RateLimiterDef),
    /// M166: Named cache with TTL — first-class module-level item.
    Cache(CacheDef),
    /// M167: Named bulkhead (concurrency isolation) — first-class module-level item.
    Bulkhead(BulkheadDef),
    /// M168: Named timeout (deadline enforcement) — first-class module-level item.
    Timeout(TimeoutDef),
    /// M169: Named fallback value — first-class module-level item.
    FallbackItem(FallbackItemDef),
}
