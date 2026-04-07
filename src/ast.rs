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
    /// Aspect declarations (AOP, M66).
    pub aspect_defs: Vec<AspectDef>,
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
    /// M79: The signal type this being interprets as directional input (Peirce semiosis).
    /// A being is organized to interpret this sign toward its telos.
    pub sign: Option<String>,
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
    /// M70: Canalization block (Waddington).
    pub canalization: Option<CanalizationBlock>,
    /// M74: Senescence block (Campisi).
    pub senescence: Option<SenescenceBlock>,
    /// M76: Criticality bounds (Langton).
    pub criticality: Option<CriticalityBlock>,
    /// M80: Umwelt block — perceptual world declaration (Uexküll 1909).
    /// Default: omnisensory (no umwelt = being receives any typed signal).
    /// If present: restricts detectable signal types.
    pub umwelt: Option<UmweltBlock>,
    /// M82: Resonance block — cross-channel correlation discovery.
    pub resonance: Option<ResonanceBlock>,
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

// ── M68: Degeneracy (Edelman) ────────────────────────────────────────────────
/// Multiple structurally distinct components can perform the same function.
/// Edelman (1987): degeneracy is the ability of structurally different elements to
/// perform the same function — distinct from redundancy.
#[derive(Debug, Clone, PartialEq)]
pub struct DegenerateBlock {
    pub primary: String,
    pub fallback: String,
    pub equivalence_proof: Option<String>,
    pub span: Span,
}

// ── M69: Cell Cycle Checkpoints (Hartwell) ───────────────────────────────────
/// Named checkpoint within a lifecycle transition.
/// Hartwell (1974): checkpoints pause the cell cycle until conditions are met.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointDef {
    pub name: String,
    pub requires: String,
    pub on_fail: String,
    pub span: Span,
}

// ── M70: Canalization (Waddington) ───────────────────────────────────────────
/// Developmental channel converging on a phenotype despite perturbations.
/// Waddington (1942): canalization is the tendency to produce the same phenotype
/// regardless of genetic or environmental perturbations.
#[derive(Debug, Clone, PartialEq)]
pub struct CanalizationBlock {
    pub toward: String,
    pub despite: Vec<String>,
    pub convergence_proof: Option<String>,
    pub span: Span,
}

// ── M71: Metabolic Pathways (Krebs) ──────────────────────────────────────────
/// One step in a metabolic pathway.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayStep {
    pub from: String,
    pub via: String,
    pub to: String,
    pub span: Span,
}

/// Named metabolic pathway (Krebs cycle as archetype).
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayDef {
    pub name: String,
    pub steps: Vec<PathwayStep>,
    pub compensate: Option<String>,
    pub span: Span,
}

// ── M74: Senescence (Campisi) ────────────────────────────────────────────────
/// Irreversible growth arrest with secretory phenotype.
/// Campisi (2001): senescent cells cease dividing but remain metabolically active.
#[derive(Debug, Clone, PartialEq)]
pub struct SenescenceBlock {
    pub onset: String,
    pub degradation: String,
    pub sasp: Option<String>,
    pub span: Span,
}

// ── M75: HGT ─────────────────────────────────────────────────────────────────
/// Horizontal gene transfer: adopt an interface from another module.
#[derive(Debug, Clone, PartialEq)]
pub struct AdoptDecl {
    pub interface: String,
    pub from_module: String,
    pub span: Span,
}

// ── M76: Criticality Bounds (Langton) ────────────────────────────────────────
/// Edge-of-chaos bounds: system must operate between ordered and chaotic regimes.
/// Langton (1990): maximal computation occurs at the phase transition.
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalityBlock {
    pub lower: f64,
    pub upper: f64,
    pub probe_fn: Option<String>,
    pub span: Span,
}

// ── M77: Niche Construction (Odling-Smee) ────────────────────────────────────
/// Organisms modify their environment, feeding back on selection pressures.
/// Odling-Smee (1988): niche construction is a second inheritance system.
#[derive(Debug, Clone, PartialEq)]
pub struct NicheConstructionDef {
    pub modifies: String,
    pub affects: Vec<String>,
    pub probe_fn: Option<String>,
    pub span: Span,
}

/// Top-level item in a module body.
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
    SymbioticImport { module: String, kind: String, span: Span },
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
    Table { name: String, fields: Vec<FieldDef>, span: Span },
    /// `node Name :: { ... }` — graph node type
    Node { name: String, fields: Vec<FieldDef>, span: Span },
    /// `edge Name :: Source -> Target { ... }` — graph edge type
    Edge { name: String, source: String, target: String, fields: Vec<FieldDef>, span: Span },
    /// `key: Type` — key-value key declaration
    KeyType { ty: TypeExpr, span: Span },
    /// `value: Type` — key-value value declaration
    ValueType { ty: TypeExpr, span: Span },
    /// `event Name :: { ... }` — time series event
    Event { name: String, fields: Vec<FieldDef>, span: Span },
    /// `embedding :: { ... }` — vector embedding
    EmbeddingEntry { fields: Vec<FieldDef>, span: Span },
    /// `fact Name :: { ... }` — OLAP fact table
    Fact { name: String, fields: Vec<FieldDef>, span: Span },
    /// `dimension Name :: { ... }` — OLAP dimension
    DimensionEntry { name: String, fields: Vec<FieldDef>, span: Span },
    /// `schema Name :: { ... }` — document collection schema
    Collection { name: String, fields: Vec<FieldDef>, span: Span },
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
