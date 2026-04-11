//! Module-level and ecosystem item AST nodes (lifecycle, session, effects, stores, etc.)
use super::*;
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleDef {
    /// The type this lifecycle applies to (e.g., "Connection").
    pub type_name: String,
    /// Ordered list of states (e.g., ["Disconnected", "Connected", "Authenticated"]).
    pub states: Vec<String>,
    /// M69: Optional cell-cycle checkpoints (Hartwell).
    pub checkpoints: Vec<CheckpointDef>,
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
    Eventually {
        type_name: String,
        target_state: String,
        span: Span,
    },
    /// `never: <state> transitions to <state>` — forbidden transition.
    Never {
        from_state: String,
        to_state: String,
        span: Span,
    },
    /// `precedes: <state> before <state>` — ordering constraint.
    Precedes {
        first: String,
        second: String,
        span: Span,
    },
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

// ── M98: Session Types (Honda 1993) ──────────────────────────────────────────

/// M98: Session type definition.
///
/// Describes the complete communication protocol between two roles.
/// Milner (1980) Pi-calculus; Honda (1993) session types; Gay & Hole (2005) subtyping.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionDef {
    pub name: String,
    /// Role definitions (typically two: client/server, buyer/seller, etc.)
    pub roles: Vec<SessionRole>,
    /// Duality declaration: role_a <-> role_b
    pub duality: Option<(String, String)>,
    pub span: Span,
}

/// One named role in a session type (e.g. client, server).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRole {
    pub name: String,
    pub steps: Vec<SessionStep>,
    pub span: Span,
}

/// A single communication step within a session role.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStep {
    /// `send: Type` — this role sends a message of the given type.
    Send(TypeExpr),
    /// `recv: Type` — this role receives a message of the given type.
    Recv(TypeExpr),
}

// ── M99: Algebraic Effect Handlers (Plotkin & Pretnar 2009) ──────────────────

/// M99: Effect definition.
///
/// Declares a named effect with typed operations.
/// Moggi (1991) monads; Plotkin & Pretnar (2009) algebraic effects; Leijen (2017) Koka.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectDef {
    pub name: String,
    /// Optional type parameters (e.g. `State<S>` → `["S"]`).
    pub type_params: Vec<String>,
    pub operations: Vec<EffectOperation>,
    pub span: Span,
}

/// A single operation declared inside an effect definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectOperation {
    pub name: String,
    pub input: TypeExpr,
    pub output: TypeExpr,
    pub span: Span,
}

/// `handle … with … end` block — intercept and dispatch effect operations.
///
/// Used inside function bodies to provide implementations for declared effects.
#[derive(Debug, Clone, PartialEq)]
pub struct HandleBlock {
    /// Name of the computation being handled (the expression argument).
    pub computation: String,
    pub handlers: Vec<EffectHandler>,
    pub span: Span,
}

/// A single handler case inside a `handle … with` block.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectHandler {
    /// Qualified operation name, e.g. `"Log.emit"` or `"State.get"`.
    pub effect_op: String,
    /// Bound parameter names (including the continuation).
    pub params: Vec<String>,
    /// The continuation variable name (e.g. `"k"`).
    pub continuation: String,
    pub span: Span,
}

// ── M78-M82: Biosemiotic signal infrastructure ────────────────────────────────

/// M80: Umwelt block — perceptual world declaration (Uexküll 1909).
/// Default: omnisensory (no umwelt = being receives any typed signal).
/// If present: restricts detectable signal types. The perceptual world is
/// a purposeful limitation, not a default constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct UmweltBlock {
    /// Signal types this being can detect. If empty and blind_to is also empty,
    /// the block is a no-op (equivalent to no umwelt declaration).
    pub detects: Vec<String>,
    /// Signal types explicitly excluded from this being's perceptual world.
    pub blind_to: Vec<String>,
    pub span: Span,
}

/// M82: Resonance block — cross-channel correlation discovery.
///
/// Models signal relationships that escape single-channel human perception.
#[derive(Debug, Clone, PartialEq)]
pub struct ResonanceBlock {
    /// Each entry: (signal_type_a, signal_type_b, optional correlation fn name)
    pub correlations: Vec<CorrelationPair>,
    pub span: Span,
}

/// A declared cross-channel correlation pair.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationPair {
    pub signal_a: String,
    pub signal_b: String,
    /// Optional declared correlation function name.
    pub via: Option<String>,
    pub span: Span,
}

/// M81: Sense declaration — a named signal channel, potentially beyond human perception.
/// Mantis shrimp model: any measurable physical quantity can be a first-class signal.
/// Examples: electromagnetic spectrum bands, acoustic ranges, chemical gradients,
/// quantum states, gravitational waves, magnetic field intensity.
///
/// M83 extends this with SI dimension symbols and derived unit formulas,
/// grounding every sense in the SI system of units.
#[derive(Debug, Clone, PartialEq)]
pub struct SenseDef {
    pub name: String,
    /// Named sub-channels within this sense dimension.
    pub channels: Vec<String>,
    /// Optional physical range description (e.g. "1e-12m to 1e3m").
    pub range: Option<String>,
    /// Optional unit declaration (e.g. "Hz", "nm", "mol/L").
    pub unit: Option<String>,
    /// M83: SI base dimension symbol (e.g. L, M, T, I, Theta, N, J).
    pub dimension: Option<String>,
    /// M83: Dimensional formula for derived units (e.g. M_L_T_neg2 for force).
    pub derived: Option<String>,
    pub span: Span,
}

// ── M66: Aspect-Oriented Specification ───────────────────────────────────────

/// Pointcut expression — selects which functions an aspect applies to.
///
/// Inspired by AspectJ (Kiczales et al., 1997) but statically resolved
/// at compile time against declared annotations and effect signatures.
#[derive(Debug, Clone, PartialEq)]
pub enum PointcutExpr {
    /// `fn where @annotation_name` — matches functions with this annotation.
    HasAnnotation(String),
    /// `fn where effect includes EffectName` — matches functions with this effect.
    EffectIncludes(String),
    /// `pointcut_a and pointcut_b` — intersection.
    And(Box<PointcutExpr>, Box<PointcutExpr>),
    /// `pointcut_a or pointcut_b` — union.
    Or(Box<PointcutExpr>, Box<PointcutExpr>),
}

/// An aspect declaration — a named, composable cross-cutting specification.
///
/// Aspects are resolved before other checkers run. Each aspect that matches a
/// function (via its pointcut) injects its before/after/around advice and
/// generates temporal ordering constraints from its `order:` field.
///
/// Key differentiator from AspectJ: aspects are type-checked at compile time.
/// A function annotated `@requires_auth` without SecurityAspect in scope is a
/// compile error, not a runtime surprise.
#[derive(Debug, Clone, PartialEq)]
pub struct AspectDef {
    pub name: String,
    /// Which functions this aspect applies to.
    pub pointcut: Option<PointcutExpr>,
    /// Functions that run before the matched function's body.
    pub before: Vec<String>,
    /// Functions that run after the matched function returns normally.
    pub after: Vec<String>,
    /// Functions that run if the matched function throws/returns Err.
    pub after_throwing: Vec<String>,
    /// Functions that wrap the matched function (full control).
    pub around: Vec<String>,
    /// Recovery function when matched function fails (`on_failure:`).
    pub on_failure: Option<String>,
    /// Maximum retry attempts for `on_failure:` recovery.
    pub max_attempts: Option<u32>,
    /// Explicit ordering relative to other aspects — lower runs first.
    /// Compiler generates temporal `precedes:` constraints from order values.
    pub order: Option<u32>,
    pub span: Span,
}

// ── M66b: Annotation Algebra ──────────────────────────────────────────────────

/// A typed annotation declaration with optional composition.
///
/// Annotation declarations serve three purposes:
/// 1. Define a typed schema for annotation parameters.
/// 2. Compose multiple existing annotations into one (the composed annotation
///    expands to its `meta_annotations` at every usage site).
/// 3. Express `@requires_aspect(X)` constraints — using this annotation on a
///    function without aspect X in scope is a compile error.
///
/// ```loom
/// @separation(owns: [source, target], disjoint: [(source, target)])
/// @timing_safety(constant_time: true)
/// annotation concurrent_transfer(source: String, target: String)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationDecl {
    /// The annotation's name (e.g. `"concurrent_transfer"`).
    pub name: String,
    /// Typed parameters: `(param_name, type_name)`.
    pub params: Vec<(String, String)>,
    /// Meta-annotations — what this annotation expands to at usage sites.
    pub meta_annotations: Vec<Annotation>,
    pub span: Span,
}

// ── M67: Correctness Report ───────────────────────────────────────────────────

/// A proved correctness claim: property name + checker that verified it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvedClaim {
    pub property: String,
    pub checker: String,
    pub span: Span,
}

/// Module-level correctness report — the autopoietic self-certification.
///
/// Generated by the compiler from checker pipeline results; can also be
/// declared manually to express claims that should be verified.
/// An `unverified:` section is required if any declared property could
/// not be checked — honest incompleteness, not silent omission.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectnessReport {
    /// Properties that were proved by their named checker.
    pub proved: Vec<ProvedClaim>,
    /// Properties that could not be checked (with reason).
    pub unverified: Vec<(String, String)>,
    pub span: Span,
}

// ── Definitions ───────────────────────────────────────────────────────────────

/// Separation logic block — O'Hearn, Reynolds, Yang (2001-2002).
///
/// Declares owned resources, disjointness constraints, frame preservation,
/// and an optional proof assertion for a function.
///
/// # Formal semantics
/// The separating conjunction `P * Q` asserts that `P` and `Q` hold for
/// *disjoint* portions of the heap simultaneously.  The frame rule then allows
/// local reasoning: a function that only touches its declared `owns:` resources
/// cannot affect anything outside that footprint, so every caller can rely on
/// unowned state being unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparationBlock {
    /// Resources exclusively owned by this function (`owns: name`).
    pub owns: Vec<String>,
    /// Pairs that must be heap-disjoint (`disjoint: A * B`).
    pub disjoint: Vec<(String, String)>,
    /// Resources that are in scope but not modified (`frame: name`).
    pub frame: Vec<String>,
    /// Optional proof assertion, e.g. `"frame_rule_verified"`.
    pub proof: Option<String>,
    pub span: Span,
}

/// Gradual typing block — Scott, Siek (2006). M59.
#[derive(Debug, Clone, PartialEq)]
pub struct GradualBlock {
    pub input_type: Option<String>,
    pub boundary: Option<String>,
    pub output_type: Option<String>,
    pub on_cast_failure: Option<String>,
    pub blame: Option<String>,
    pub span: Span,
}

/// A parametric distribution family with typed parameters.
/// M84 — replaces the free-string model field.
///
/// Academic grounding:
/// - Gaussian: central limit theorem (Laplace 1812, Gauss 1809)
/// - Poisson: rare events in fixed intervals (Poisson 1837)
/// - Beta: probabilities, proportions — bounded [0,1] (Euler 1763)
/// - Dirichlet: probability vectors — sum to 1 (Dirichlet 1831)
/// - Gamma: waiting times, positive reals (Euler 1729)
/// - Exponential: memoryless waiting times (special case of Gamma)
/// - Binomial: count of successes in n trials (Bernoulli 1713)
/// - Pareto: power-law tails, 80/20 rule (Pareto 1896)
/// - Cauchy: heavy tails, NO defined mean or variance (Cauchy 1853)
///   — CLT and LLN do not apply; convergence claims are invalid
/// - Levy: stable distribution, anomalous diffusion (Lévy 1937)
/// - LogNormal: multiplicative processes, finance (Galton 1879)
/// - Uniform: equal probability over range (Laplace 1812)
/// - GeometricBrownian: GBM — multiplicative diffusion (Black-Scholes 1973)
/// - Unknown(String): backward compatibility — free string model name
#[derive(Debug, Clone, PartialEq)]
pub enum DistributionFamily {
    Gaussian { mean: String, std_dev: String },
    Poisson { lambda: String },
    Beta { alpha: String, beta: String },
    Dirichlet { alpha: Vec<String> },
    Gamma { shape: String, scale: String },
    Exponential { lambda: String },
    Binomial { n: String, p: String },
    Pareto { alpha: String, x_min: String },
    Cauchy { location: String, scale: String },
    Levy { location: String, scale: String },
    LogNormal { mean: String, std_dev: String },
    Uniform { low: String, high: String },
    GeometricBrownian { drift: String, volatility: String },
    Unknown(String),
}

/// Probabilistic types distribution block. M84 (replaces M60 thin version).
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionBlock {
    /// The parametric distribution family.
    pub family: DistributionFamily,
    /// Deprecated free-string model field — kept for compatibility, prefer family.
    pub model: String,
    pub mean: Option<String>,
    pub variance: Option<String>,
    pub bounds: Option<String>,
    pub convergence: Option<String>,
    pub stability: Option<String>,
    pub span: Span,
}

/// Side-channel timing safety block. M62.
#[derive(Debug, Clone, PartialEq)]
pub struct TimingSafetyBlock {
    pub constant_time: bool,
    pub leaks_bits: Option<String>,
    pub method: Option<String>,
    pub span: Span,
}

/// Proof annotation for Curry-Howard correspondence. M64.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofAnnotation {
    pub strategy: String,
    pub span: Span,
}

// ── M88: Stochastic Process Types ────────────────────────────────────────────

/// M88: Stochastic process kind.
/// Wiener (1923), Itô (1944), Ornstein-Uhlenbeck (1930), Markov (1906).
#[derive(Debug, Clone, PartialEq)]
pub enum StochasticKind {
    /// Standard Brownian motion. Continuous paths. Martingale property.
    Wiener,
    /// GBM — always positive, used for asset prices. Black-Scholes (1973).
    GeometricBrownian,
    /// Mean-reverting process. Ornstein-Uhlenbeck (1930).
    OrnsteinUhlenbeck,
    /// Count process. Integer-valued. Events at rate λ. Poisson (1837).
    PoissonProcess,
    /// Discrete state, memoryless. Markov (1906).
    MarkovChain,
    /// Unrecognized kind name — forward compatible.
    Unknown(String),
}

/// M88: Stochastic process annotation block.
///
/// Declares the mathematical process type governing a function's probabilistic
/// behaviour, with verifiable properties checked at compile time.
#[derive(Debug, Clone, PartialEq)]
pub struct StochasticProcessBlock {
    pub kind: StochasticKind,
    /// GBM: paths are always > 0 (log-normal distribution).
    pub always_positive: Option<bool>,
    /// Whether the process satisfies the martingale property.
    pub martingale: Option<bool>,
    /// OU: process reverts toward a long-run mean.
    pub mean_reverting: Option<bool>,
    /// OU: the long-run equilibrium value.
    pub long_run_mean: Option<String>,
    /// Poisson: event arrival rate λ.
    pub rate: Option<String>,
    /// Poisson: process takes only integer values.
    pub integer_valued: Option<bool>,
    /// MarkovChain: explicit state names.
    pub states: Vec<String>,
    pub span: Span,
}

// ── M92: Store declarations ───────────────────────────────────────────────────

/// Store kind — the data model algebra.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreKind {
    Relational,
    KeyValue,
    Graph,
    Document,
    Columnar,
    Snowflake,
    Hypercube,
    TimeSeries,
    Vector,
    InMemory(Box<StoreKind>),
    FlatFile,
    /// Distributed MapReduce store (Dean & Ghemawat 2004).
    Distributed,
    /// Kafka-style partitioned append-only distributed log (Kreps 2011).
    DistributedLog,
}

/// A store declaration — first-class persistence with typed schema.
///
/// Each store kind has a distinct data algebra with academically grounded
/// query strategies. The `store:` construct makes polyglot persistence
/// a compile-time concern rather than a runtime configuration problem.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreDef {
    pub name: String,
    pub kind: StoreKind,
    /// Schema entries — tables, nodes, edges, collections, etc.
    pub schema: Vec<StoreSchemaEntry>,
    /// Optional store-level configuration (ttl, retention, index, etc.)
    pub config: Vec<StoreConfigEntry>,
    pub span: Span,
}

/// A schema entry in a store declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreSchemaEntry {
    /// `table Name ... end` — relational table
    Table {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `node Name :: { ... }` — graph node type
    Node {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `edge Name :: Source -> Target { ... }` — graph edge type
    Edge {
        name: String,
        source: String,
        target: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `key: Type` — key-value key declaration
    KeyType { ty: TypeExpr, span: Span },
    /// `value: Type` — key-value value declaration
    ValueType { ty: TypeExpr, span: Span },
    /// `event Name :: { ... }` — time series event
    Event {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `embedding :: { ... }` — vector embedding
    EmbeddingEntry { fields: Vec<FieldDef>, span: Span },
    /// `fact Name :: { ... }` — OLAP fact table
    Fact {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `dimension Name :: { ... }` — OLAP dimension
    DimensionEntry {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `schema Name :: { ... }` — document collection schema
    Collection {
        name: String,
        fields: Vec<FieldDef>,
        span: Span,
    },
    /// `mapreduce Name ... end` — MapReduce job (M97)
    MapReduceJob(MapReduceDef),
    /// `consumer Name :: offset: value` — DistributedLog consumer (M97)
    LogConsumer(LogConsumerDef),
}

/// MapReduce job declaration inside a Distributed store.
///
/// Dean & Ghemawat (2004): map emits (key,value) pairs; shuffle groups by key;
/// reduce aggregates per key. Signatures stored as raw strings for flexibility.
#[derive(Debug, Clone, PartialEq)]
pub struct MapReduceDef {
    pub name: String,
    /// `map :: InputType -> [(KeyType, ValueType)]`
    pub map_sig: String,
    /// `reduce :: KeyType -> [ValueType] -> (KeyType, OutputType)`
    pub reduce_sig: String,
    /// `combine :: KeyType -> [ValueType] -> ValueType`  (optional local combiner)
    pub combine_sig: Option<String>,
    pub span: Span,
}

/// Consumer declaration for a DistributedLog store (M97).
#[derive(Debug, Clone, PartialEq)]
pub struct LogConsumerDef {
    pub name: String,
    /// Offset position: `"earliest"`, `"latest"`, or a timestamp string.
    pub offset: String,
    pub span: Span,
}

/// Key-value configuration entry in a store.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreConfigEntry {
    pub key: String,
    pub value: String,
    pub span: Span,
}

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

// ── M100: SMT Contract Verification Bridge (Hoare 1969 → Dijkstra WP → Z3) ────

/// SMT verification status for a function's contracts.
///
/// Lineage: Hoare (1969) axiomatic semantics → Dijkstra (1975) weakest
/// precondition calculus → Dafny (2009) → Loom `require:`/`ensure:` → M100 discharge.
#[derive(Debug, Clone, PartialEq)]
pub enum SmtStatus {
    /// The contract was proved unsatisfiable (postcondition holds for all preconditions).
    Proved,
    /// The solver found a counterexample — spec is contradictory or wrong.
    Counterexample(String),
    /// The solver could not determine satisfiability within its budget.
    Unknown,
    /// Z3 is not available; verification was skipped.
    Skipped,
}

/// SMT verification result for a single function's contracts.
#[derive(Debug, Clone, PartialEq)]
pub struct SmtVerification {
    /// The function whose contracts are being verified.
    pub function: String,
    /// SMT-LIB2 translation of the precondition expression.
    pub precondition: String,
    /// SMT-LIB2 translation of the postcondition expression.
    pub postcondition: String,
    /// Result of the SMT check.
    pub status: SmtStatus,
}

// ── M116: Messaging Primitive — typed inter-being communication contract ───────
//
// Formalises SyncRequest/AsyncMessage/Stream/EventBus/RPC/MessageBroker as
// first-class Loom constructs with compiler-verified delivery guarantees.
// Lineage: session types (Honda 1993) → capability types → Loom `messaging_primitive`.

/// The interaction pattern of a messaging primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum MessagingPattern {
    RequestResponse,
    PublishSubscribe,
    PointToPoint,
    ProducerConsumer,
    Bidirectional,
    /// M136: Continuous data stream with backpressure (Reactive Streams spec).
    Stream,
}

/// A top-level messaging primitive declaration.
///
/// Declares how a being communicates with other beings or external systems,
/// including delivery guarantees and interaction pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct MessagingPrimitiveDef {
    /// Primitive name (e.g. `SyncRequest`, `OrderEventBus`).
    pub name: String,
    /// Interaction pattern (request/response, pub/sub, etc.).
    pub pattern: Option<MessagingPattern>,
    /// Declared delivery guarantees (e.g. `@exactly-once`, `@at_least_once`).
    pub guarantees: Vec<String>,
    /// Whether a timeout declaration is mandatory for this primitive.
    pub timeout_mandatory: bool,
    pub span: Span,
}

// ── M117: TelosFunctionDef ───────────────────────────────────────────────────

/// M117: Top-level telos function declaration — telos as typed function, not just string.
///
/// Bridges Peirce semiotics (interpretant as function), Barbieri code biology (propagation
/// carries the interpretant), and Loom's type system (TelosMetric is a typed function).
#[derive(Debug, Clone, PartialEq)]
pub struct TelosFunctionDef {
    pub name: String,
    /// Human-readable statement (e.g. "converge risk-adjusted PnL toward equilibrium").
    pub statement: Option<String>,
    /// Formal constraint (e.g. a function reference).
    pub bounded_by: Option<String>,
    /// Typed metric function signature (e.g. "BeingState -> SignalSet -> Float").
    pub measured_by: Option<String>,
    /// Convergence thresholds.
    pub thresholds: Option<TelosThresholds>,
    /// Decision axes this telos guides.
    pub guides: Vec<String>,
    pub span: Span,
}

// ── M118: EntityDef ──────────────────────────────────────────────────────────

/// M118: Universal graph/network primitive — entity<N, E, Annotations>.
///
/// All computation structures are instances of entity:
/// - MarkovChain = entity<State, Transition, @stochastic @finite>
/// - DAG = entity<Node, Edge, @directed @acyclic>
/// - NeuralNet = entity<Neuron, Weight, @directed @weighted @learnable>
/// - KnowledgeGraph = entity<Concept, Relation, @undirected @semantic>
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDef {
    pub name: String,
    /// Node type parameter (e.g. "State", "Neuron", "ClimateRegion").
    pub node_type: Option<String>,
    /// Edge type parameter (e.g. "Transition", "Weight", "Coupling").
    pub edge_type: Option<String>,
    /// All annotations combined: structural + semantic + verification + behavior.
    pub annotations: Vec<String>,
    /// Optional describe string.
    pub describe: Option<String>,
    /// Optional alias: what well-known structure this is an instance of.
    pub alias_of: Option<String>,
    pub span: Span,
}

// ── M119: IntentCoordinatorDef ───────────────────────────────────────────────

/// Governance class for intent changes (Part IX — Intent Vivo with Governance).
#[derive(Debug, Clone, PartialEq)]
pub enum GovernanceClass {
    Automatic,
    AiProposes,
    HumanOnly,
    Blocked,
}

/// A signal source for intent inference.
#[derive(Debug, Clone, PartialEq)]
pub struct IntentSignalSource {
    pub name: String,
    pub trust_level: Option<String>,
}

/// M119: Intent Coordinator — living intent with human governance.
///
/// The third mode between static production code and full BIOISO:
/// intent that can adapt based on usage behavior and market context,
/// subject to governance gates that classify each change by required approval level.
#[derive(Debug, Clone, PartialEq)]
pub struct IntentCoordinatorDef {
    pub name: String,
    /// Telomere on the coordinator (e.g. 90 days before re-evaluation).
    pub telomere_days: Option<u64>,
    /// Default governance class for changes.
    pub governance_class: GovernanceClass,
    /// Signal sources that feed the coordinator.
    pub signals: Vec<IntentSignalSource>,
    /// Rollback condition.
    pub rollback_on: Option<String>,
    /// Minimum confidence score to propose a change (0.0–1.0).
    pub min_confidence: Option<f64>,
    /// Audit trail: emit changes to this path.
    pub audit_path: Option<String>,
    pub span: Span,
}

// ── M141: DisciplineDecl ─────────────────────────────────────────────────────

/// M141: Explicit forced-discipline declaration.
///
/// Bridges Loom's architectural disciplines (DI, CQRS, EventSourcing, CircuitBreaker, Saga)
/// with the type system: a `discipline D for T` block is a compiler-verified promise that
/// code generated from type `T` will conform to discipline `D`'s structural contracts.
///
/// Syntax:
/// ```loom
/// discipline CQRS for OrderStore end
/// discipline EventSourcing for OrderStore events: [OrderCreated, OrderShipped] end
/// discipline DependencyInjection for PricingEngine binds: [IPriceRepo, IRiskCalc] end
/// discipline CircuitBreaker for PaymentService max_attempts: 3 timeout_ms: 500 end
/// discipline Saga for CheckoutFlow steps: [ValidateOrder, ProcessPayment] end
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DisciplineDecl {
    /// The discipline kind.
    pub kind: DisciplineKind,
    /// The target type/store/module this discipline applies to.
    pub target: String,
    /// Keyword arguments (string key → string value) for parameterised disciplines.
    pub params: Vec<(String, DisciplineParam)>,
    pub span: Span,
}

/// Typed parameter values for a discipline declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum DisciplineParam {
    /// A single identifier or string value.
    Scalar(String),
    /// A list of identifiers (e.g. `events: [Created, Updated]`).
    List(Vec<String>),
    /// A numeric value (e.g. `max_attempts: 3`).
    Number(i64),
}

/// The kind of forced architectural discipline.
#[derive(Debug, Clone, PartialEq)]
pub enum DisciplineKind {
    /// Command/Query Responsibility Segregation (Young 2010, Meyer CQS).
    Cqrs,
    /// Event Sourcing — state = fold of events (Fowler 2005, Evans DDD).
    EventSourcing,
    /// Dependency Injection container — port-based DI (Martin 2003).
    DependencyInjection,
    /// Circuit breaker — fault tolerance pattern (Nygard 2007, Fowler).
    CircuitBreaker,
    /// Saga — compensating long-running transactions (Garcia-Molina 1987).
    Saga,
    /// Unit of Work — batch persistence with commit/rollback (Fowler PoEAA).
    UnitOfWork,
}

// ── M155: ChainDef ───────────────────────────────────────────────────────────

/// M155: Discrete-time Markov chain as a first-class module-level item.
///
/// Syntax:
/// ```loom
/// chain Weather
///   states: [Sunny, Cloudy, Rainy]
///   transitions:
///     Sunny -> Cloudy: 0.3
///     Sunny -> Rainy: 0.1
///     Cloudy -> Sunny: 0.4
///     Cloudy -> Rainy: 0.2
///     Rainy -> Sunny: 0.15
///     Rainy -> Cloudy: 0.35
///   end
/// end
/// ```
///
/// Emits a `WeatherTransitionMatrix` struct with the transitions pre-initialized
/// and a `validate()` method that asserts row-stochastic property.
/// Reference: Markov (1906) — P(X_{n+1}|X_n).
#[derive(Debug, Clone, PartialEq)]
pub struct ChainDef {
    pub name: String,
    /// Named states in the Markov chain (form the State enum).
    pub states: Vec<String>,
    /// Transitions as (from, to, probability).
    pub transitions: Vec<(String, String, f64)>,
    pub span: Span,
}

// ── M156: DagDef ─────────────────────────────────────────────────────────────

/// M156: Directed Acyclic Graph as a first-class module-level item.
///
/// Syntax:
/// ```loom
/// dag Pipeline
///   nodes: [Ingest, Transform, Validate, Load]
///   edges: [Ingest -> Transform, Transform -> Validate, Validate -> Load]
/// end
/// ```
///
/// Emits a `{Name}Node` enum, a `{Name}Dag` struct with typed nodes/edges,
/// Kahn topological sort, and cycle detection.
/// Reference: Kahn (1962) — BFS-based topological ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct DagDef {
    pub name: String,
    /// Declared node names (form the Node enum variants).
    pub nodes: Vec<String>,
    /// Declared edges as (from_node, to_node).
    pub edges: Vec<(String, String)>,
    pub span: Span,
}

// ── M159: PipelineDef ──────────────────────────────────────────────────────────

/// M159: Named sequential data-transformation pipeline as a first-class module item.
///
/// Syntax:
/// ```loom
/// pipeline DataCleaner
///   step normalize :: String -> String
///   step trim :: String -> String
///   step validate :: String -> Bool
/// end
/// ```
///
/// Emits:
/// - `{Name}Pipeline` struct (unit struct, zero-cost abstraction)
/// - `impl {Name}Pipeline { pub fn process(&self, input: {InputType}) -> {OutputType} { ... } }`
/// - One `pub fn {step_name}(&self, input: {In}) -> {Out}` per step (stub, `todo!()`)
/// - `LOOM[pipeline:step]` audit comment on each step
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineDef {
    pub name: String,
    /// Each step: (step_name, input_type, output_type)
    pub steps: Vec<PipelineStep>,
    pub span: Span,
}

/// A single named transformation step in a pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStep {
    pub name: String,
    /// Loom type name for the input (e.g. `"String"`, `"Int"`)
    pub input_ty: String,
    /// Loom type name for the output (e.g. `"String"`, `"Bool"`)
    pub output_ty: String,
    pub span: Span,
}

// ── M160: SagaDef ─────────────────────────────────────────────────────────────

/// M160: Named distributed transaction saga — first-class module-level item.
///
/// A saga is a sequence of forward steps, each paired with an optional compensating
/// transaction for rollback on failure (Garcia-Molina 1987, "SAGAS").
///
/// Syntax:
/// ```loom
/// saga OrderSaga
///   step reserve :: Order -> Reservation
///   step charge :: Reservation -> Payment
///   compensate charge :: Payment -> Unit
///   step fulfill :: Payment -> Fulfillment
///   compensate fulfill :: Fulfillment -> Unit
/// end
/// ```
///
/// Emits:
/// - `{Name}Saga` struct (unit struct)
/// - `{Name}SagaError` enum with one variant per step
/// - `impl {Name}Saga { pub fn execute(&self, input: {In}) -> Result<{Out}, {Name}SagaError> }`
/// - Per-step `fn` stub + per-compensate `fn` stub (both `todo!()`)
/// - `LOOM[saga:step]` + `LOOM[saga:compensate]` audit comments
#[derive(Debug, Clone, PartialEq)]
pub struct SagaDef {
    pub name: String,
    pub steps: Vec<SagaStep>,
    pub span: Span,
}

/// A single step in a saga, with an optional compensating transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct SagaStep {
    pub name: String,
    pub input_ty: String,
    pub output_ty: String,
    /// Compensating transaction, if declared. Re-uses the same name + `_compensate` suffix.
    pub compensate: Option<SagaCompensate>,
    pub span: Span,
}

/// A compensating transaction for a saga step.
#[derive(Debug, Clone, PartialEq)]
pub struct SagaCompensate {
    pub step_name: String,
    pub input_ty: String,
    pub output_ty: String,
    pub span: Span,
}

/// M157: Named constant as a first-class module-level item.
///
/// Syntax:
/// ```loom
/// const MaxRetries: Int = 3
/// const Timeout: Float = 30.0
/// const ServiceName: String = "api-gateway"
/// ```
///
/// Emits `pub const UPPER_SNAKE: RustType = value;`
/// with a `LOOM[const:item]` audit comment.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: String,
    /// Loom type annotation (e.g. `"Int"`, `"Float"`, `"String"`).
    pub ty: String,
    /// Raw value token as a string (e.g. `"3"`, `"30.0"`, `"\"api-gateway\""`).
    pub value: String,
    pub span: Span,
}

// ── M161: EventDef ────────────────────────────────────────────────────────────

/// M161: Named domain event — first-class module-level item.
///
/// A domain event captures something significant that happened in the system.
/// Each event carries typed payload fields declared inline.
///
/// Syntax:
/// ```loom
/// event UserRegistered
///   user_id: Int
///   email: String
///   at: String
/// end
/// ```
///
/// Emits:
/// - `#[derive(Debug, Clone, PartialEq)]` struct `{Name}Event` with typed pub fields
/// - `pub trait {Name}EventHandler { fn handle(&self, event: &{Name}Event); }`
/// - `LOOM[event:domain]` audit comment
#[derive(Debug, Clone, PartialEq)]
pub struct EventDef {
    pub name: String,
    /// Payload fields: (field_name, loom_type_name)
    pub fields: Vec<(String, String)>,
    pub span: Span,
}

// ── M162: CommandDef / QueryDef ───────────────────────────────────────────────

/// M162: Named CQRS command — first-class module-level item.
///
/// Commands represent intent to change state. They carry payload fields
/// and produce a handler trait that returns `Result<(), String>`.
///
/// Syntax:
/// ```loom
/// command PlaceOrder
///   order_id: Int
///   amount: Float
/// end
/// ```
///
/// Emits:
/// - `#[derive(Debug, Clone)]` struct `{Name}Command` with typed pub fields
/// - `pub trait {Name}Handler { fn handle(&self, cmd: {Name}Command) -> Result<(), String>; }`
/// - `LOOM[command:cqrs]` audit comment
#[derive(Debug, Clone, PartialEq)]
pub struct CommandDef {
    pub name: String,
    /// Payload fields: (field_name, loom_type_name)
    pub fields: Vec<(String, String)>,
    pub span: Span,
}

/// M162: Named CQRS query — first-class module-level item.
///
/// Queries read state without side effects. They carry criteria fields
/// and produce a generic handler trait parameterised over the return type.
///
/// Syntax:
/// ```loom
/// query GetOrder
///   order_id: Int
/// end
/// ```
///
/// Emits:
/// - `#[derive(Debug, Clone)]` struct `{Name}Query` with typed pub fields
/// - `pub trait {Name}QueryHandler<R> { fn handle(&self, query: {Name}Query) -> R; }`
/// - `LOOM[query:cqrs]` audit comment
#[derive(Debug, Clone, PartialEq)]
pub struct QueryDef {
    pub name: String,
    /// Criteria fields: (field_name, loom_type_name)
    pub fields: Vec<(String, String)>,
    pub span: Span,
}

// ── M163: CircuitBreakerDef ───────────────────────────────────────────────────

/// M163: Named circuit breaker — first-class module-level resilience item.
///
/// Implements the Circuit Breaker pattern (Nygard 2007, "Release It!").
/// A circuit breaker wraps remote calls and opens after repeated failures,
/// preventing cascading failures across service boundaries.
///
/// Syntax:
/// ```loom
/// circuit_breaker PaymentGateway
///   threshold: 5
///   timeout: 30
///   fallback: use_cache
/// end
/// ```
///
/// Emits:
/// - `{Name}CircuitState` enum: `Closed`, `Open`, `HalfOpen`
/// - `{Name}CircuitBreaker` struct with `failure_threshold`, `timeout_secs`, `state`
/// - `impl {Name}CircuitBreaker` with `new()`, `call<F,T>()`, `fallback_{fallback}()`
/// - `LOOM[circuit_breaker:resilience]` + M163 audit comment
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitBreakerDef {
    pub name: String,
    /// Number of consecutive failures before opening (default: 5).
    pub threshold: u32,
    /// Seconds before attempting half-open (default: 30).
    pub timeout: u64,
    /// Fallback function name (snake_case). Empty string = no fallback.
    pub fallback: String,
    pub span: Span,
}

// ── M164: RetryDef ────────────────────────────────────────────────────────────

/// M164: Named retry policy — first-class module-level resilience item.
///
/// Implements exponential backoff retry (Tanenbaum & Van Steen, "Distributed Systems").
///
/// Syntax:
/// ```loom
/// retry PaymentRetry
///   max_attempts: 3
///   base_delay: 100
///   multiplier: 2
///   on: NetworkError
/// end
/// ```
///
/// Emits:
/// - `{Name}Policy` struct with `max_attempts`, `base_delay_ms`, `multiplier`
/// - `impl {Name}Policy` with `new()` + `execute<F,T,E>()` stub
/// - `LOOM[retry:resilience]` + M164 audit comment
#[derive(Debug, Clone, PartialEq)]
pub struct RetryDef {
    pub name: String,
    /// Maximum number of attempts (default: 3).
    pub max_attempts: u32,
    /// Base delay in milliseconds (default: 100).
    pub base_delay: u64,
    /// Backoff multiplier (default: 2).
    pub multiplier: u32,
    /// Error type to retry on (optional; empty = retry any error).
    pub on_error: String,
    pub span: Span,
}

// ── M165: rate_limiter (token bucket) ──────────────────────────────────────────

/// `rate_limiter Name requests: N per: N burst: N end`
/// Generates a token-bucket `{Name}RateLimiter` struct with `allow()` method.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimiterDef {
    pub name: String,
    /// Max requests allowed in the window (default: 100).
    pub requests: u64,
    /// Window size in seconds (default: 60).
    pub per: u64,
    /// Burst capacity — extra tokens above rate (default: 0 = no burst).
    pub burst: u64,
    pub span: Span,
}

// ── M166: cache (typed, TTL-aware) ─────────────────────────────────────────────

/// `cache Name key: Type value: Type ttl: N end`
/// Generates a typed `{Name}Cache<K,V>` generic struct with get/set/evict methods.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheDef {
    pub name: String,
    /// Key type (default: "String").
    pub key_type: String,
    /// Value type (default: "String").
    pub value_type: String,
    /// Time-to-live in seconds (default: 300).
    pub ttl: u64,
    pub span: Span,
}

// ── M167: bulkhead (concurrency isolation) ─────────────────────────────────────

/// `bulkhead Name max_concurrent: N queue_size: N end`
/// Generates a `{Name}Bulkhead` struct with `execute()` and capacity tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct BulkheadDef {
    pub name: String,
    /// Maximum concurrent executions allowed (default: 10).
    pub max_concurrent: u64,
    /// Queue size for waiting requests (default: 0 = no queue).
    pub queue_size: u64,
    pub span: Span,
}

// ── M168: timeout (deadline enforcement) ───────────────────────────────────────

/// `timeout Name duration: N unit: ms|s|min end`
/// Generates a `{Name}Timeout` struct with `execute<F,T>()` deadline wrapper.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeoutDef {
    pub name: String,
    /// Deadline duration value (default: 30).
    pub duration: u64,
    /// Time unit: "ms", "s", or "min" (default: "s").
    pub unit: String,
    pub span: Span,
}

// ── M169: fallback item (static/dynamic fallback value) ────────────────────────

/// `fallback Name value: "literal" end`
/// Generates a `{Name}Fallback<T>` struct with `get() -> T` method.
#[derive(Debug, Clone, PartialEq)]
pub struct FallbackItemDef {
    pub name: String,
    /// Static fallback value as a string literal (default: "").
    pub value: String,
    pub span: Span,
}

// ── M170: observer (GoF Observable) ────────────────────────────────────────────

/// `observer Name type: T end`
/// Generates a `{Name}Observer<T>` struct with subscribe/notify/get.
#[derive(Debug, Clone, PartialEq)]
pub struct ObserverDef {
    pub name: String,
    /// Observed value type (default: "String").
    pub observed_type: String,
    pub span: Span,
}

// ── M171: pool (object/connection pool) ────────────────────────────────────────

/// `pool Name size: N end`
/// Generates a `{Name}Pool<T>` struct with acquire/release.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolDef {
    pub name: String,
    /// Pool capacity (default: 10).
    pub size: u64,
    pub span: Span,
}

// ── M172: scheduler (periodic task) ────────────────────────────────────────────

/// `scheduler Name interval: N unit: ms|s|min end`
/// Generates a `{Name}Scheduler` struct with run/stop methods.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulerDef {
    pub name: String,
    /// Repeat interval value (default: 1).
    pub interval: u64,
    /// Time unit: "ms", "s", or "min" (default: "s").
    pub unit: String,
    pub span: Span,
}

// ── M173: QueueDef ────────────────────────────────────────────────────────────

/// `queue Name capacity: N kind: fifo|lifo end`
///
/// First-class FIFO/LIFO named queue item.
#[derive(Debug, Clone, PartialEq)]
pub struct QueueDef {
    pub name: String,
    /// Maximum capacity (0 = unbounded, default: 0).
    pub capacity: u64,
    /// Queue discipline: "fifo" or "lifo" (default: "fifo").
    pub kind: String,
    pub span: Span,
}

// ── M174: LockDef ─────────────────────────────────────────────────────────────

/// `lock Name end`
///
/// First-class named mutex-style lock item.
#[derive(Debug, Clone, PartialEq)]
pub struct LockDef {
    pub name: String,
    pub span: Span,
}

// ── M175: ChannelDef ──────────────────────────────────────────────────────────

/// `channel Name type: T capacity: N end`
///
/// First-class typed MPSC channel item.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelDef {
    pub name: String,
    /// Element type (default: "String").
    pub element_type: String,
    /// Bounded capacity (0 = unbounded, default: 0).
    pub capacity: u64,
    pub span: Span,
}

// ── M176: SemaphoreDef ────────────────────────────────────────────────────────

/// `semaphore Name permits: N end`
///
/// First-class counting semaphore item.
#[derive(Debug, Clone, PartialEq)]
pub struct SemaphoreDef {
    pub name: String,
    /// Initial permit count (default: 1).
    pub permits: u64,
    pub span: Span,
}

// ── M177: ActorDef ────────────────────────────────────────────────────────────

/// `actor Name type: M end`
///
/// First-class lightweight actor with mailbox item.
#[derive(Debug, Clone, PartialEq)]
pub struct ActorDef {
    pub name: String,
    /// Message type (default: "String").
    pub message_type: String,
    pub span: Span,
}

// ── M178: BarrierDef ──────────────────────────────────────────────────────────

/// `barrier Name count: N end`
///
/// First-class N-thread synchronization barrier item.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierDef {
    pub name: String,
    /// Number of threads to synchronize (default: 2).
    pub count: u64,
    pub span: Span,
}

// ── M179: EventBusDef ─────────────────────────────────────────────────────────

/// `event_bus Name [element_type: T] end`
///
/// First-class pub/sub event dispatcher item.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBusDef {
    pub name: String,
    /// Element type carried by events (default: "String").
    pub element_type: String,
    pub span: Span,
}

// ── M180: StateMachineDef ─────────────────────────────────────────────────────

/// `state_machine Name [initial: S] end`
///
/// First-class finite state machine item.
#[derive(Debug, Clone, PartialEq)]
pub struct StateMachineDef {
    pub name: String,
    /// Name of the initial state (default: "Initial").
    pub initial_state: String,
    pub span: Span,
}
