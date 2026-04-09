//! `LoomChecker` — the common interface for all semantic-analysis passes.
//!
//! Every checker implements [`LoomChecker`] by returning `Vec<LoomError>`.
//! An empty vec means success. Warnings use `[warn]`/`[info]`/`[hint]`
//! prefixes; the [`CheckerStage`] wrapper filters them before propagating
//! errors to the caller.
//!
//! # Design rationale (ADR-0004)
//! The pipeline in `compile()` was a 263-line series of hardcoded checker
//! calls with no common interface. Extracting this trait gives us:
//! - **DI**: swap implementations in tests without touching `compile()`
//! - **Composability**: build sub-pipelines (e.g. wasm, typescript)
//! - **Auditability**: the pipeline is a data structure, not control flow

use crate::ast::Module;
use crate::error::LoomError;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Common interface for every Loom semantic-analysis pass.
///
/// A checker receives the parsed [`Module`] and returns all diagnostic
/// messages it found. An empty vec is success. Warnings and informational
/// messages are conventionally prefixed with `[warn]`, `[info]`, or `[hint]`
/// so callers can filter them without a separate `is_warning` predicate.
pub trait LoomChecker: Send + Sync {
    /// Run this checker against `module` and return all diagnostics.
    ///
    /// # Returns
    /// `Vec<LoomError>` — empty on success. Non-empty means at least one
    /// diagnostic was produced (may be filtered by [`CheckerStage`]).
    fn check_module(&self, module: &Module) -> Vec<LoomError>;
}

// ── Pipeline stage wrapper ────────────────────────────────────────────────────

/// A single stage in the [`compile`](crate::compile) checker pipeline.
///
/// Wraps a [`LoomChecker`] with a set of diagnostic prefixes to suppress
/// (e.g. `[warn]`, `[info]`) before deciding whether to fail the pipeline.
pub struct CheckerStage {
    /// The checker to run.
    pub checker: Box<dyn LoomChecker>,
    /// Diagnostic message prefixes that should NOT block compilation.
    /// E.g. `&["[warn]"]`, `&["[hint]", "[warn]", "[info]"]`.
    pub suppress: &'static [&'static str],
}

impl CheckerStage {
    /// Stage that fails on any error (no suppression).
    pub fn hard(checker: impl LoomChecker + 'static) -> Self {
        Self {
            checker: Box::new(checker),
            suppress: &[],
        }
    }

    /// Stage that suppresses `[warn]` prefixed messages.
    pub fn warn_only(checker: impl LoomChecker + 'static) -> Self {
        Self {
            checker: Box::new(checker),
            suppress: &["[warn]"],
        }
    }

    /// Stage with custom suppression prefixes.
    pub fn suppressing(
        checker: impl LoomChecker + 'static,
        suppress: &'static [&'static str],
    ) -> Self {
        Self {
            checker: Box::new(checker),
            suppress,
        }
    }

    /// Run this stage. Returns `Err` only if hard (non-suppressed) errors exist.
    pub fn run(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let errors: Vec<LoomError> = self
            .checker
            .check_module(module)
            .into_iter()
            .filter(|e| {
                let msg = e.to_string();
                self.suppress.iter().all(|prefix| !msg.contains(prefix))
            })
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ── Blanket impls — `Result<(), Vec<LoomError>>` checkers ────────────────────

macro_rules! impl_result_checker {
    ($($T:ty),+ $(,)?) => {
        $(
            impl LoomChecker for $T {
                fn check_module(&self, module: &Module) -> Vec<LoomError> {
                    self.check(module).err().unwrap_or_default()
                }
            }
        )+
    };
}

macro_rules! impl_vec_checker {
    ($($T:ty),+ $(,)?) => {
        $(
            impl LoomChecker for $T {
                fn check_module(&self, module: &Module) -> Vec<LoomError> {
                    self.check(module)
                }
            }
        )+
    };
}

use super::{
    AlgebraicChecker, AspectChecker, BoundaryChecker, CanalizationChecker, CategoryChecker,
    CheckpointChecker, CognitiveMemoryChecker, ConservationChecker, CriticalityChecker,
    CurryHowardChecker, DegeneracyChecker, DependentChecker, EffectChecker, EffectHandlerChecker,
    ErrorCorrectionChecker, EvolutionVectorChecker, ExhaustivenessChecker, GradualChecker,
    HgtChecker, InferenceEngine, JournalChecker, ManifestChecker, MessagingChecker,
    MigrationChecker, MinimalChecker, NicheConstructionChecker, PathwayChecker, PrivacyChecker,
    ProbabilisticChecker, PropertyChecker, ProvenanceChecker, RefinementChecker, ResonanceChecker,
    ScenarioChecker, SelfCertChecker, SemiosisChecker, SenescenceChecker, SeparationChecker,
    SessionChecker, SideChannelChecker, SignalAttentionChecker, StochasticChecker, StoreChecker,
    SymbiosisChecker, TemporalChecker, TensorChecker, TypeChecker, TypestateChecker, UmweltChecker,
    UnitsChecker, UseCaseChecker,
};

// Result<(), Vec<LoomError>> checkers
impl_result_checker!(
    AlgebraicChecker,
    AspectChecker,
    CanalizationChecker,
    CategoryChecker,
    CheckpointChecker,
    CriticalityChecker,
    CurryHowardChecker,
    DegeneracyChecker,
    DependentChecker,
    EffectChecker,
    ErrorCorrectionChecker,
    ExhaustivenessChecker,
    GradualChecker,
    HgtChecker,
    InferenceEngine,
    NicheConstructionChecker,
    PathwayChecker,
    PrivacyChecker,
    ProbabilisticChecker,
    RefinementChecker,
    SelfCertChecker,
    SenescenceChecker,
    SeparationChecker,
    SideChannelChecker,
    SymbiosisChecker,
    TensorChecker,
    TemporalChecker,
    TypeChecker,
    TypestateChecker,
    UmweltChecker,
    UnitsChecker,
);

// Vec<LoomError> checkers
impl_vec_checker!(
    BoundaryChecker,
    CognitiveMemoryChecker,
    ConservationChecker,
    EffectHandlerChecker,
    EvolutionVectorChecker,
    JournalChecker,
    ManifestChecker,
    MessagingChecker,
    MigrationChecker,
    MinimalChecker,
    PropertyChecker,
    ProvenanceChecker,
    ResonanceChecker,
    ScenarioChecker,
    SemiosisChecker,
    SessionChecker,
    SignalAttentionChecker,
    StoreChecker,
    UseCaseChecker,
);

// ── Outlier adapters ──────────────────────────────────────────────────────────

// StochasticChecker uses `check(module, &mut errors)` — adapt to trait.
impl LoomChecker for StochasticChecker {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        StochasticChecker::check(module, &mut errors);
        errors
    }
}

/// Adapter wrapping the unit-struct `StochasticChecker` for use in the pipeline.
pub struct StochasticCheckerAdapter;
impl LoomChecker for StochasticCheckerAdapter {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        StochasticChecker::check(module, &mut errors);
        errors
    }
}

/// Adapter for `safety::SafetyChecker::check(module)` (no `self`).
pub struct SafetyCheckerAdapter;
impl LoomChecker for SafetyCheckerAdapter {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        super::SafetyChecker::check(module)
    }
}

/// Adapter for `teleos::check(module)` (free function, no `self`).
pub struct TeleosCheckerAdapter;
impl LoomChecker for TeleosCheckerAdapter {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        super::check_teleos(module).err().unwrap_or_default()
    }
}

/// Adapter for `randomness::RandomnessChecker::check(module, &mut errors)`.
pub struct RandomnessCheckerAdapter;
impl LoomChecker for RandomnessCheckerAdapter {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        super::randomness::RandomnessChecker::check(module, &mut errors);
        errors
    }
}
