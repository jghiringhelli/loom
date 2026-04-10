/// LPN instruction AST — the data types that represent every
/// valid Loom Protocol Notation instruction across all three tiers.
///
/// **Tier 1** — atomic: one op, one target.
/// **Tier 2** — compound: multiple steps in one instruction.
/// **Tier 3** — experiment: named experiment with key=value params.

// ── Emit targets ─────────────────────────────────────────────────────────────

/// Compilation target for an `EMIT` instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitTarget {
    /// `rust` — emit Rust source (default).
    Rust,
    /// `ts` or `typescript` — emit TypeScript.
    TypeScript,
    /// `wasm` — emit WebAssembly Text (WAT).
    Wasm,
    /// `openapi` — emit OpenAPI 3.0 YAML.
    OpenApi,
    /// `schema` — emit JSON Schema.
    Schema,
}

impl EmitTarget {
    /// Parse an emit target from a lowercase string token.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "rust" | "rs" => Some(Self::Rust),
            "ts" | "typescript" => Some(Self::TypeScript),
            "wasm" | "wat" => Some(Self::Wasm),
            "openapi" | "oas" => Some(Self::OpenApi),
            "schema" | "json-schema" => Some(Self::Schema),
            _ => None,
        }
    }
}

// ── Check kinds ───────────────────────────────────────────────────────────────

/// What the `CHECK` instruction verifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckKind {
    /// Type resolution and compatibility.
    Types,
    /// Effect propagation and consequence tiers.
    Effects,
    /// `require:`/`ensure:` Hoare contracts.
    Contracts,
    /// Privacy label co-occurrence rules.
    Privacy,
    /// Safety checker — `@mortal @corrigible @sandboxed`.
    Safety,
    /// Run all checkers (full pipeline).
    All,
}

impl CheckKind {
    /// Parse a check kind from a lowercase string token.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "types" => Some(Self::Types),
            "effects" => Some(Self::Effects),
            "contracts" => Some(Self::Contracts),
            "privacy" => Some(Self::Privacy),
            "safety" => Some(Self::Safety),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

// ── Verify steps ──────────────────────────────────────────────────────────────

/// A verification step declared in a Tier 2 `VERIFY` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyStep {
    /// Compile the emitted output with `rustc`.
    Compile,
    /// Run the compiled binary.
    Run,
    /// Run only the type checker.
    Types,
    /// Run only the effect checker.
    Effects,
    /// Run only the contract checker.
    Contracts,
}

impl VerifyStep {
    /// Parse a single verify step.  `+`-separated lists are handled by the
    /// caller in [`parse_verify_steps`].
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compile" => Some(Self::Compile),
            "run" => Some(Self::Run),
            "types" => Some(Self::Types),
            "effects" => Some(Self::Effects),
            "contracts" => Some(Self::Contracts),
            _ => None,
        }
    }
}

/// Parse a `+`-separated list of verify steps (e.g. `compile+run+types`).
pub fn parse_verify_steps(s: &str) -> Vec<VerifyStep> {
    s.split('+').filter_map(VerifyStep::from_str).collect()
}

// ── Milestone references ──────────────────────────────────────────────────────

/// A milestone reference: either a single milestone (`M41`) or a
/// contiguous range (`M84-M89`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilestoneRef {
    /// A single milestone, e.g. `M41`.
    Single(u32),
    /// An inclusive range, e.g. `M84-M89`.
    Range(u32, u32),
}

impl MilestoneRef {
    /// Expand the reference into individual milestone numbers.
    pub fn expand(&self) -> Vec<u32> {
        match self {
            Self::Single(n) => vec![*n],
            Self::Range(start, end) => (*start..=*end).collect(),
        }
    }

    /// Parse a milestone reference from a string like `M41` or `M84-M89`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('M');
        if let Some((a, b)) = s.split_once('-') {
            let start: u32 = a.parse().ok()?;
            let end: u32 = b.trim_start_matches('M').parse().ok()?;
            Some(Self::Range(start, end))
        } else {
            let n: u32 = s.parse().ok()?;
            Some(Self::Single(n))
        }
    }
}

/// Parse a bracket-enclosed milestone list: `[M41,M55,M84-M89]`.
pub fn parse_milestone_list(s: &str) -> Vec<MilestoneRef> {
    s.trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|part| MilestoneRef::parse(part.trim()))
        .collect()
}

// ── Experiment params ─────────────────────────────────────────────────────────

/// Parsed key=value parameters for a Tier 3 `ALX` experiment instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentParams {
    /// Number of ALX iterations (`n=7`).
    pub n: Option<u32>,
    /// Target domain name (`domain=biotech`).
    pub domain: Option<String>,
    /// Minimum S_realized coverage threshold (`coverage>=0.95`).
    pub min_coverage: Option<f64>,
    /// Emission target (`emit=rust`).
    pub emit: EmitTarget,
    /// Verification steps (`verify=compile+run`).
    pub verify: Vec<VerifyStep>,
    /// Whether to persist evidence (`evidence=store`).
    pub evidence: bool,
}

impl Default for ExperimentParams {
    fn default() -> Self {
        Self {
            n: None,
            domain: None,
            min_coverage: None,
            emit: EmitTarget::Rust,
            verify: vec![],
            evidence: false,
        }
    }
}

// ── Main instruction type ─────────────────────────────────────────────────────

/// A single LPN instruction — the unit of AI-to-AI communication in Loom.
///
/// Each variant corresponds to one line in a `.lp` file.
/// Comments (lines starting with `#`) and blank lines are not instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum LpnInstruction {
    // ── Tier 1: Atomic declarations ──────────────────────────────────────────

    /// `FN name :: TypeSig` — declare a function signature.
    Fn { name: String, sig: String },

    /// `TYPE name = field:Type …` — declare a product type.
    Type { name: String, body: String },

    /// `ENUM name = | Variant1 | Variant2 of Type` — declare a sum type.
    Enum { name: String, body: String },

    /// `EMIT target Module [FROM file]` — compile and emit a module.
    Emit { target: EmitTarget, module: String, from: Option<String> },

    /// `CHECK kind file` — run the specified checker on a file.
    Check { kind: CheckKind, file: String },

    /// `TEST name (args) -> expected` — verify a function call.
    Test { name: String, args: String, expected: String },

    /// `VERIFY claim file` — verify a correctness claim against a file.
    Verify { claim: String, file: String },

    /// `ADD feature TO module` — add a feature to an existing module.
    Add { feature: String, module: String },

    /// `DEL item FROM file` — remove an item from a file.
    Del { item: String, from: String },

    /// `RENAME from TO to IN file` — rename a symbol in a file.
    Rename { from: String, to: String, in_file: String },

    // ── Tier 2: Compound operations ──────────────────────────────────────────

    /// `IMPL target USING [milestones] EMIT target VERIFY steps`
    /// — implement a module covering specific milestones, then emit and verify.
    Impl {
        target: String,
        milestones: Vec<MilestoneRef>,
        emit: EmitTarget,
        verify: Vec<VerifyStep>,
    },

    /// `REFACTOR file SPLIT AT fn_name` — refactor a source file.
    Refactor { file: String, split_at: String },

    // ── Tier 3: Experiments ───────────────────────────────────────────────────

    /// `ALX n=N domain=D coverage>=C emit=T verify=V evidence=store`
    /// — run an ALX self-applicability experiment.
    Alx(ExperimentParams),

    /// Any other named experiment with key=value params.
    /// e.g. `SCALPER ticks=10000 ou_theta=2.0`
    Experiment { name: String, params: Vec<(String, String)> },
}
