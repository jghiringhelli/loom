//! Being-related AST nodes — the biological entity and all its sub-blocks.
#![allow(clippy::large_enum_variant)]
use super::*;
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
///
/// Supports two syntaxes:
/// - Classic: `regulate varname  target: ... bounds: (...) response: | cond -> action end`
/// - Trigger/action: `regulate:  trigger: condition_expr  action: fn_name end`
#[derive(Debug, Clone, PartialEq)]
pub struct RegulateBlock {
    pub variable: String,
    pub target: String,
    pub bounds: Option<(String, String)>,
    pub response: Vec<(String, String)>,
    /// M114: How much this regulation contributes to telos convergence (0.0–1.0).
    pub telos_contribution: Option<f64>,
    /// Trigger/action syntax: the condition expression that activates this rule.
    pub trigger: Option<String>,
    /// Trigger/action syntax: the function to call when trigger fires.
    pub action: Option<String>,
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
    /// M112: Typed metric function — `compute :: BeingState -> SignalSet -> Float`.
    /// Turns telos from a string label into a measurable convergence function.
    pub metric: Option<String>,
    /// M112: Convergence thresholds for telos evaluation.
    pub thresholds: Option<TelosThresholds>,
    /// M112: Which decision axes this telos guides (signal attention, propagation, etc.).
    pub guides: Vec<String>,
    pub span: Span,
}

/// M112: Thresholds controlling telos-driven lifecycle transitions.
///
/// Invariant enforced by checker: divergence < warning ≤ convergence ≤ propagation.
/// All values in [0.0, 1.0].
#[derive(Debug, Clone, PartialEq)]
pub struct TelosThresholds {
    /// Being flourishes above this score; eligible for propagation signal.
    pub convergence: f64,
    /// Below `convergence`, above `warning`: being under stress, activates repair.
    pub warning: Option<f64>,
    /// Below this: apoptosis trigger.
    pub divergence: f64,
    /// Above this: eligible to propagate.  Defaults to `convergence` if absent.
    pub propagation: Option<f64>,
}

/// M115: Declares how a being filters incoming signals based on telos relevance.
///
/// Signals above `prioritize_above` are given full attention;
/// signals below `attenuate_below` are damped but not ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalAttentionBlock {
    /// Signals with telos_relevance > this threshold receive priority processing.
    pub prioritize_above: Option<f64>,
    /// Signals with telos_relevance < this threshold are attenuated.
    pub attenuate_below: Option<f64>,
    /// Named signals that receive priority processing (Uexküll Umwelt declaration).
    pub prioritize_named: Vec<String>,
    /// Named signals that are attenuated/damped.
    pub attenuate_named: Vec<String>,
    /// Telos relevance: "computed from X" — the function that weights signals by telos.
    pub telos_relevance: Option<String>,
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

/// Irreversible state transition — tipping point in the ecosystem.
///
/// When a threshold condition is crossed, the system undergoes a regime shift.
/// Scheffer (2009): critical transitions in nature and society.
#[derive(Debug, Clone, PartialEq)]
pub struct TippingPoint {
    /// Name (e.g. "amazon_dieback").
    pub name: String,
    /// Condition string (e.g. "vegetation_coverage < 0.60").
    pub condition: String,
    /// Action on crossing (e.g. "escalate to human_regulated").
    pub on_crossing: String,
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
    /// collective_telos_metric: a function or expression combining all member telos scores.
    pub collective_telos_metric: Option<String>,
    /// tipping_points: irreversible state transitions in the ecosystem.
    pub tipping_points: Vec<TippingPoint>,
    /// coevolution: true means beings in this ecosystem co-evolve (fitness is relative).
    pub coevolution: bool,
    /// coupling: how beings are physically/mathematically coupled.
    pub coupling: Option<String>,
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
    /// Optional duration before auto-revert (e.g. "18.months", "90.days").
    pub duration: Option<String>,
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

// ── M106: Migration — Interface Evolution Contract ───────────────────────────

/// A migration block — declares how a being's public interface changes between versions.
///
/// Liskov Substitution Principle (1987) → semantic versioning (Preston-Werner 2011)
/// → API evolution contracts → Loom `migration:` (M106).
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationBlock {
    /// Version transition name (e.g. "v1_to_v2").
    pub name: String,
    /// Field name + old type (stored as debug-format token string).
    pub from_field: String,
    /// Field name + new type (stored as debug-format token string).
    pub to_field: String,
    /// Adapter function string (e.g. "fn v1 -> Duration::from_seconds(v1)").
    pub adapter: Option<String>,
    /// Whether this is a breaking change (default: true).
    pub breaking: bool,
    pub span: Span,
}

/// Biological propagation — the being can reproduce when telos score is sufficient.
/// Barbieri (2003): propagation copies both matter (information) AND telos (code/interpretant).
/// Without propagation, beings cannot evolve populations; with it, natural selection emerges.
#[derive(Debug, Clone, PartialEq)]
pub struct PropagateBlock {
    /// Condition string for eligibility (e.g. "telos.score > 0.85").
    pub condition: String,
    /// Matter fields the offspring inherits (e.g. ["matter", "telos", "epigenetic_memory"]).
    pub inherits: Vec<String>,
    /// Mutations: (field_path, constraint_string) e.g. ("bond_angles", "within quantum_mechanical_bounds").
    pub mutates: Vec<(String, String)>,
    /// Optional: what type of being the offspring is (defaults to same type).
    pub offspring_type: Option<String>,
    pub span: Span,
}

/// M187: A declared structural relationship from this being to another — `relates_to: Name kind: K`.
///
/// Kinds mirror M72 symbiosis: mutualistic | commensal | parasitic.
/// Emitted as a `// LOOM[relates_to:Name:kind]` doc comment alongside the being.
#[derive(Debug, Clone, PartialEq)]
pub struct RelatesTo {
    pub target: String,
    pub kind: String,
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
    /// M101: Documentation liveness manifest block.
    pub manifest: Option<ManifestBlock>,
    /// M106: Migration blocks — interface evolution contracts.
    pub migrations: Vec<MigrationBlock>,
    /// M104: Journal block — episodic memory primitive (Tulving 1972).
    pub journal: Option<JournalBlock>,
    /// M105: Scenario blocks — executable acceptance criteria (Beck 2002 / BDD).
    pub scenarios: Vec<ScenarioBlock>,
    /// M103: Boundary block — public API surface declaration.
    pub boundary: Option<BoundaryBlock>,
    /// M112: Cognitive memory block — lightweight hippocampal layer.
    pub cognitive_memory: Option<CognitiveMemoryBlock>,
    /// M115: Signal attention filter — prioritize/attenuate by telos relevance.
    pub signal_attention: Option<SignalAttentionBlock>,
    /// M186: Role classification tag — M185 taxonomy. Values: sensor | effector | regulator |
    /// integrator | memory | classifier.  Emitted as a `// LOOM[role:X]` comment.
    pub role: Option<String>,
    /// M187: Structural relationships to other beings — `relates_to: Name kind: mutualistic`.
    pub relates_to: Vec<RelatesTo>,
    /// Biological propagation — offspring when telos score exceeds threshold.
    pub propagate_block: Option<PropagateBlock>,
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

// ── M104: Journal — Episodic Memory Primitive (Tulving 1972) ─────────────────

/// What a being records in its episodic journal.
/// Tulving (1972) episodic vs semantic memory → Squire (1987) declarative/procedural
/// distinction → GS Five Memory Types → Loom `journal:` (M104).
#[derive(Debug, Clone, PartialEq)]
pub enum JournalRecord {
    /// `record: every evolve_step` — log each adaptation.
    EvolveStep,
    /// `record: every telos_progress` — log progress toward telos.
    TelosProgress,
    /// `record: every state_transition` — log lifecycle state changes.
    StateTransition,
    /// `record: every regulation_trigger` — log when regulate: bounds fire.
    RegulationTrigger,
    /// `record: every <custom>` — user-defined record event.
    Custom(String),
}

/// Episodic memory block — records what the being experienced and why.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalBlock {
    /// Events to record.
    pub records: Vec<JournalRecord>,
    /// Ring-buffer size (`keep: last N`).
    pub keep_last: Option<u64>,
    /// Output path template (`emit: "path/file.log"`).
    pub emit_path: Option<String>,
    pub span: Span,
}

// ── M105: Scenario — Executable Acceptance Criteria (Beck 2002 / BDD) ────────

/// Given/When/Then executable acceptance criterion for a being.
/// Beck (2002) TDD → Cucumber BDD (2008) Given/When/Then → GS Executable property
/// → Loom `scenario:` (M105).
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioBlock {
    /// Scenario name (e.g. `trade_executes_on_signal`).
    pub name: String,
    /// Precondition string (e.g. `"market_signal == BullishCrossover"`).
    pub given: String,
    /// Trigger string (e.g. `"being.sense() detects market_signal"`).
    pub when: String,
    /// Assertion string, mirrors `ensure:` syntax (e.g. `"position_size > 0"`).
    pub then: String,
    /// Optional deadline: (count, unit) e.g. `(3, "lifecycle_ticks")`.
    pub within: Option<(u64, String)>,
    pub span: Span,
}

// ── M110: Use Case Triple Derivation (Jacobson 1992 → Beck 2003 → GS) ─────────

/// A single verifiable acceptance criterion within a `usecase:` block.
///
/// Each criterion derives a `#[test]` stub at code-generation time.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceCriterion {
    /// Snake-case test identifier (e.g. `"can_register_valid_user"`).
    pub name: String,
    /// Human-readable description of the criterion.
    pub description: String,
}

/// A use-case block — Jacobson (1992) use cases expressed as a GS triple-derivation source.
///
/// One block simultaneously generates:
/// 1. **Implementation contract** — `require:`/`ensure:` Hoare-style comments.
/// 2. **Test stubs** — `#[test] fn uc_…()` stubs for each acceptance criterion.
/// 3. **Documentation** — OpenAPI description + user-facing doc comment.
#[derive(Debug, Clone, PartialEq)]
pub struct UseCaseBlock {
    /// Use-case name in PascalCase (e.g. `"RegisterUser"`).
    pub name: String,
    /// The actor initiating this use case (e.g. `"ExternalUser"`).
    pub actor: String,
    /// Precondition expression as a free-form string (e.g. `"not user_exists(email)"`).
    pub precondition: String,
    /// Trigger description (e.g. `"POST /users with CreateUserRequest"`).
    pub trigger: String,
    /// Postcondition expression (e.g. `"user_count == prior(user_count) + 1"`).
    pub postcondition: String,
    /// Verifiable acceptance criteria — each becomes a test stub.
    pub acceptance: Vec<AcceptanceCriterion>,
    pub span: Span,
}

// ── M109: Property-Based Testing (QuickCheck 2000 → fast-check → Hypothesis) ──

/// A property-based test declaration.
///
/// Declares a universally quantified invariant: for all x in T, invariant holds.
/// QuickCheck (Claessen & Hughes 2000) → fast-check (JS) → Hypothesis (Python)
/// → Loom `property:` (M109).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBlock {
    pub name: String,
    /// The universally quantified variable name (e.g. "x").
    pub var_name: String,
    /// The type of the quantified variable (e.g. "String").
    pub var_type: String,
    /// The invariant expression as a raw string (e.g. "decode(encode(x)) = x").
    pub invariant: String,
    /// Whether to enable counterexample shrinking (default: true).
    pub shrink: bool,
    /// Number of random samples to generate (default: 100).
    pub samples: u64,
    pub span: Span,
}

// ── M102: Provenance — Data Lineage Tracking (W3C PROV-DM 2013) ──────────────

/// Data lineage label — tracks the origin and transformation chain of a value.
///
/// W3C PROV-DM (2013) data provenance model → Buneman (2001) "Why and Where"
/// provenance → Loom `@provenance` annotation (M102).
#[derive(Debug, Clone, PartialEq)]
pub struct ProvenanceLabel {
    /// Origin identifier (e.g. "sensor:temperature", "api:exchange").
    pub source: String,
    /// Chain of transformations applied to the value.
    pub transformation: Vec<String>,
    /// Confidence level 0.0–1.0. None = unspecified.
    pub confidence: Option<f64>,
    /// Whether to record a timestamp when the value is produced.
    pub timestamp: bool,
}

// ── M103: Boundary — Explicit Public API Surface Declaration ─────────────────

/// A boundary block — declares exactly which types and functions are public API.
///
/// Everything not listed is private by default. The compiler enforces that no
/// internal type leaks through a public function signature.
/// Parnas (1972) information hiding → Composable GS property → Loom `boundary:` (M103).
#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryBlock {
    /// Names explicitly exported as public API.
    pub exports: Vec<String>,
    /// Names explicitly declared private (must not appear in public signatures).
    pub private: Vec<String>,
    /// Names exported but not extendable outside the declaring module.
    pub sealed: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestArtifact {
    /// Relative or absolute path to the documentation file.
    pub path: String,
    /// Symbol names in the module that this artifact documents.
    pub reflects: Vec<String>,
    /// Freshness requirement as a string, e.g. `"within 1"`.
    pub freshness: Option<String>,
    /// Condition under which this artifact is required, e.g. `"PublicApi"`.
    pub required_when: Option<String>,
}

/// A `manifest:` block inside a `being:` declaration.
///
/// Declares the documentation artifacts that must exist and be current for
/// the being to satisfy the Self-describing GS property.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestBlock {
    pub artifacts: Vec<ManifestArtifact>,
    pub span: Span,
}

// ── M112: Cognitive Memory — Lightweight Hippocampal Layer ───────────────────

/// The five cognitive

// -- M112: Cognitive Memory -- Lightweight Hippocampal Layer

/// The five cognitive memory types.
///
/// Mirrors Chronicle's memory model but self-contained in Loom.
/// Each type has a different decay rate and default storage tier.
///
/// - Episodic: What happened. Fed by journal: entries. Decays fast.
/// - Semantic: Currently true facts. Fed by regulate: violations. Decays slowly.
/// - Procedural: How to evolve. Fed by migration: steps. Never decays.
/// - Architectural: Why built this way. Fed by manifest:. Never decays.
/// - Insight: Cross-being patterns. Fed by M111 clusters. Never decays.
#[derive(Debug, Clone, PartialEq)]
pub enum CognitiveMemoryType {
    Episodic,
    Semantic,
    Procedural,
    Architectural,
    Insight,
}

/// Storage tier for cognitive memories.
///
/// Memories promote: Buffer -> Working -> Core based on access frequency.
/// Core memories never decay.
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTier {
    Buffer,
    Working,
    Core,
}

/// M112: Cognitive memory block inside a being.
///
/// The CognitiveMemoryChecker validates that declared types match the being's blocks:
/// episodic requires journal:, procedural requires migration:, architectural requires manifest:.
#[derive(Debug, Clone, PartialEq)]
pub struct CognitiveMemoryBlock {
    /// Which memory types this being participates in.
    pub memory_types: Vec<CognitiveMemoryType>,
    /// Decay rate override for episodic/semantic memories (0.0 = permanent).
    pub decay_rate: Option<f64>,
    /// Storage tier override.
    pub tier: Option<MemoryTier>,
    pub span: Span,
}
