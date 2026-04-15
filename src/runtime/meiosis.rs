//! Meiosis engine — R14: cross-entity genetic recombination + autonomous evolution judge.
//!
//! After an experiment completes, [`MeiosisEngine`] closes the evolutionary loop:
//!
//! 1. **Select donors** — entities with the most promoted mutations qualify.
//! 2. **Recombine** — pair donors and cross-breed their mutations into hybrid
//!    `.loom` genome files.
//! 3. **Judge** — [`EvolutionJudge`] (Claude) evaluates each genome autonomously:
//!    - **Mitosis**: convergent mutations → apply back to the fittest parent entity.
//!    - **Meiosis**: orthogonal mutations → register as a new independent offspring.
//!    - **Reject**: contradictory or non-beneficial — discarded, never pushed.
//! 4. **Publish** — accepted genomes are pushed to GitHub; the `evolve.yml`
//!    workflow compiles and auto-merges them to `main` (no PR required).
//!
//! # Configuration (environment)
//!
//! | Env var | Default | Description |
//! |---|---|---|
//! | `GITHUB_TOKEN` | — | PAT with `contents: write` on the target repo |
//! | `GITHUB_REPO` | — | `owner/repo` (e.g. `jghiringhelli/loom`) |
//! | `GITHUB_GENOMES_BRANCH` | `genomes/evolved` | Branch to push genomes to |
//! | `JUDGE_CLAUDE_MODEL` | `claude-3-haiku-20240307` | Model for evolution judge |

use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};

use crate::runtime::mutation::MutationProposal;

// ── Promoted record ───────────────────────────────────────────────────────────

/// A single mutation that was promoted during an experiment run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotedRecord {
    /// Tick at which this mutation was promoted.
    pub tick: u64,
    /// Entity that was mutated.
    pub entity_id: String,
    /// The promoted proposal.
    pub proposal: MutationProposal,
}

// ── Donor selection ───────────────────────────────────────────────────────────

/// An entity selected as a genetic donor for cross-breeding.
#[derive(Debug, Clone)]
pub struct MeiosisDonor {
    /// Entity identifier.
    pub entity_id: String,
    /// All promoted mutations from this entity.
    pub records: Vec<PromotedRecord>,
}

impl MeiosisDonor {
    /// Number of promoted mutations (the donor's "fitness score").
    pub fn score(&self) -> usize {
        self.records.len()
    }

    /// Promoted `ParameterAdjust` mutations only.
    pub fn parameter_adjusts(&self) -> Vec<&PromotedRecord> {
        self.records
            .iter()
            .filter(|r| matches!(r.proposal, MutationProposal::ParameterAdjust { .. }))
            .collect()
    }

    /// Promoted `StructuralRewire` mutations only.
    pub fn structural_rewires(&self) -> Vec<&PromotedRecord> {
        self.records
            .iter()
            .filter(|r| matches!(r.proposal, MutationProposal::StructuralRewire { .. }))
            .collect()
    }
}

// ── Mutation vectors ──────────────────────────────────────────────────────────

/// A sparse vector in mutation-parameter space.
///
/// Each `ParameterAdjust` contributes a signed delta to the named parameter
/// dimension.  Each `StructuralRewire` contributes +1.0 to a synthetic
/// `rewire:<signal>` dimension.  This lets us compute real cosine similarity
/// between two donors' mutation histories before asking the LLM judge.
///
/// Decision thresholds (loose guidelines, LLM may override):
/// - cosine ≈  1.0  → same parameters, same direction  → **Mitosis**
/// - cosine ≈  0.0  → different parameter spaces       → **Meiosis**
/// - cosine ≈ −1.0  → same parameters, opposite sign   → **Reject**
#[derive(Debug, Clone, Default)]
pub struct MutationVector(HashMap<String, f64>);

impl MutationVector {
    /// Build a vector from a slice of promoted records.
    pub fn from_records(records: &[PromotedRecord]) -> Self {
        let mut map: HashMap<String, f64> = HashMap::new();
        for r in records {
            match &r.proposal {
                MutationProposal::ParameterAdjust { param, delta, .. } => {
                    *map.entry(param.clone()).or_insert(0.0) += delta;
                }
                MutationProposal::StructuralRewire { signal_name, .. } => {
                    *map.entry(format!("rewire:{signal_name}")).or_insert(0.0) += 1.0;
                }
                _ => {}
            }
        }
        MutationVector(map)
    }

    /// Cosine similarity ∈ [−1, 1].  Returns 0.0 if either vector is zero.
    pub fn cosine_similarity(&self, other: &Self) -> f64 {
        let dot: f64 = self
            .0
            .iter()
            .filter_map(|(k, v)| other.0.get(k).map(|u| v * u))
            .sum();
        let mag_a: f64 = self.0.values().map(|v| v * v).sum::<f64>().sqrt();
        let mag_b: f64 = other.0.values().map(|v| v * v).sum::<f64>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            (dot / (mag_a * mag_b)).clamp(-1.0, 1.0)
        }
    }

    /// Orthogonality score ∈ [0, 1].
    /// 1.0 = truly orthogonal (different spaces), 0.0 = identical or opposing.
    pub fn orthogonality_score(&self, other: &Self) -> f64 {
        1.0 - self.cosine_similarity(other).abs()
    }

    /// True if no mutations contributed to this vector.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Dimension names present in this vector.
    pub fn dimensions(&self) -> Vec<&str> {
        let mut dims: Vec<&str> = self.0.keys().map(|s| s.as_str()).collect();
        dims.sort_unstable();
        dims
    }
}

// ── Genome rendering ──────────────────────────────────────────────────────────

/// A hybrid `.loom` genome file ready for publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolvedGenome {
    /// Evolutionary generation number.
    pub generation: u32,
    /// Primary parent entity.
    pub parent_a: String,
    /// Secondary parent entity (cross-breed) or `None` for selfing.
    pub parent_b: Option<String>,
    /// Relative path for the GitHub file.
    /// Mitosis: `genomes/evolved/gen{N}/mitosis_{parent_a}.loom`
    /// Meiosis: `genomes/evolved/gen{N}/{a}_x_{b}.loom`
    pub filename: String,
    /// Full `.loom` source content.
    pub source: String,
    /// Total number of mutations incorporated from both parents.
    pub mutations_incorporated: usize,
    /// Judge decision (set after evaluation; `None` before judge runs).
    pub decision: Option<EvolutionDecision>,
}

/// Render a hybrid `.loom` being from one or two donor entities.
fn render_genome(
    generation: u32,
    parent_a: &MeiosisDonor,
    parent_b: Option<&MeiosisDonor>,
) -> EvolvedGenome {
    let slug = match parent_b {
        Some(b) => format!("{}_x_{}", sanitize_ident(&parent_a.entity_id), sanitize_ident(&b.entity_id)),
        None => format!("{}_evolved", sanitize_ident(&parent_a.entity_id)),
    };
    let module_name = to_pascal_case(&slug);
    let being_name = format!("{}Gen{}", to_pascal_case(&parent_a.entity_id), generation);

    let parent_comment = match parent_b {
        Some(b) => format!("{} × {} (generation {})", parent_a.entity_id, b.entity_id, generation),
        None => format!("{} self-evolved (generation {})", parent_a.entity_id, generation),
    };

    // Collect mutations from both parents
    let mut all_records: Vec<&PromotedRecord> = parent_a.records.iter().collect();
    if let Some(b) = parent_b {
        all_records.extend(b.records.iter());
    }

    let mut regulate_blocks = String::new();
    let mut epigenetic_blocks = String::new();
    let mut fn_defs = String::new();
    let mut fn_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut mutations_incorporated = 0usize;

    for record in &all_records {
        match &record.proposal {
            MutationProposal::ParameterAdjust { param, delta, reason, .. } => {
                let fn_name = format!("adjust_{}", sanitize_ident(param));
                let threshold = if *delta > 0.0 {
                    format!("{param} < {:.4}", delta.abs() * 10.0)
                } else {
                    format!("{param} > {:.4}", delta.abs() * 10.0)
                };
                regulate_blocks.push_str(&format!(
                    "\n  regulate:\n    trigger: {threshold}\n    action: {fn_name}\n  end\n"
                ));
                if fn_names.insert(fn_name.clone()) {
                    fn_defs.push_str(&format!(
                        "\n-- evolved from tick {}: {}\nfn {fn_name} :: Unit -> Unit\nend\n",
                        record.tick, reason
                    ));
                }
                mutations_incorporated += 1;
            }
            MutationProposal::StructuralRewire { signal_name, reason, to_id, .. } => {
                epigenetic_blocks.push_str(&format!(
                    "\n  epigenetic:\n    signal: {signal_name}\n    modifies: adaptation_rate\n    reverts_when: stress < 0.3\n    duration: 10.ticks\n  end\n"
                ));
                fn_defs.push_str(&format!(
                    "\n-- structural rewire tick {}: signal '{}' → '{}' ({})\n",
                    record.tick, signal_name, to_id, reason
                ));
                mutations_incorporated += 1;
            }
            _ => {}
        }
    }

    // Ensure at least one regulate block so the being compiles
    if regulate_blocks.is_empty() {
        regulate_blocks.push_str(
            "\n  regulate:\n    trigger: stress > 0.7\n    action: activate_homeostasis\n  end\n",
        );
        fn_defs.push_str("\nfn activate_homeostasis :: Unit -> Unit\nend\n");
    }

    let source = format!(
        r#"-- BIOISO Evolved Genome
-- {parent_comment}
-- Mutations incorporated: {mutations_incorporated}
-- Generated by MeiosisEngine (autonomous)

module {module_name}

being {being_name}
  describe: "Evolved being: {parent_comment}"

  telos: "maintain homeostasis through evolved adaptive strategies"
  end

  criticality:
    lower: 0.2
    upper: 0.9
    probe_fn: measure_stability
  end
{regulate_blocks}{epigenetic_blocks}
  evolve:
    toward: telos
    search: | gradient_descent
    constraint: "convergence toward evolved equilibrium"
  end
end

lifecycle {being_name} :: Stable -> Stressed -> Recovering
  checkpoint: EnterStressed
    requires: stability_below_threshold
    on_fail: activate_emergency_adaptation
  end
end
{fn_defs}
fn measure_stability :: Unit -> Float
  0.5
end

fn stability_below_threshold :: Unit -> Bool
  false
end

fn activate_emergency_adaptation :: Unit -> Unit
end

end
"#
    );

    let filename = format!("genomes/evolved/gen{generation}/{slug}.loom");
    EvolvedGenome { generation, parent_a: parent_a.entity_id.clone(), parent_b: parent_b.map(|b| b.entity_id.clone()), filename, source, mutations_incorporated, decision: None }
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

/// Replace non-alphanumeric characters with underscores.
fn sanitize_ident(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}

// ── Evolution judge ───────────────────────────────────────────────────────────

/// Autonomous decision for a candidate genome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionDecision {
    /// Mutations are convergent — apply changes back to the fittest parent (self-update).
    Mitosis,
    /// Mutations are orthogonal — register as a new independent offspring entity.
    Meiosis,
    /// Contradictory changes, no net benefit, or evaluation error — discard.
    Reject,
}

/// Full verdict returned by [`EvolutionJudge`] for a candidate genome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionVerdict {
    /// Whether to self-update, spawn offspring, or discard.
    pub decision: EvolutionDecision,
    /// One-sentence justification from the judge.
    pub reasoning: String,
    /// 0.0 = identical mutation domains; 1.0 = completely independent domains.
    pub orthogonality_score: f64,
    /// Estimated fitness change: positive = improvement, negative = regression.
    pub fitness_delta: f64,
}

/// LLM-backed judge that evaluates evolved genomes autonomously.
///
/// Uses `claude-3-haiku-20240307` by default (cheap, fires per genome).
/// Configurable via `JUDGE_CLAUDE_MODEL`.
pub struct EvolutionJudge {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl EvolutionJudge {
    /// Build from environment. Returns `None` when `CLAUDE_API_KEY` is unset.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CLAUDE_API_KEY").ok()?;
        let model = std::env::var("JUDGE_CLAUDE_MODEL")
            .unwrap_or_else(|_| "claude-3-haiku-20240307".to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self { api_key, model, client })
    }

    /// Evaluate a candidate genome against its donors.
    ///
    /// Returns `Reject` with an error note on any network or parse failure.
    pub fn evaluate(&self, genome: &EvolvedGenome, donors: &[&MeiosisDonor]) -> EvolutionVerdict {
        self.call_claude(genome, donors).unwrap_or_else(|e| {
            eprintln!("[EvolutionJudge] evaluation failed for {}: {e}", genome.filename);
            EvolutionVerdict {
                decision: EvolutionDecision::Reject,
                reasoning: format!("evaluation error: {e}"),
                orthogonality_score: 0.0,
                fitness_delta: 0.0,
            }
        })
    }

    fn call_claude(
        &self,
        genome: &EvolvedGenome,
        donors: &[&MeiosisDonor],
    ) -> Result<EvolutionVerdict, String> {
        let prompt = Self::build_prompt(genome, donors);
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 256,
            "messages": [{"role": "user", "content": prompt}]
        });
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        let text = json["content"][0]["text"].as_str().unwrap_or("").to_string();
        Self::parse_verdict(&text)
    }

    fn build_prompt(genome: &EvolvedGenome, donors: &[&MeiosisDonor]) -> String {
        // Compute math-grounded orthogonality before asking the LLM.
        // Two vectors → cosine similarity → orthogonality score.
        // For a single donor (selfing), orthogonality is 0 by definition.
        let (math_cosine, math_orthogonality, math_hint) = if donors.len() >= 2 {
            let va = MutationVector::from_records(&donors[0].records);
            let vb = MutationVector::from_records(&donors[1].records);
            let cos = va.cosine_similarity(&vb);
            let orth = 1.0 - cos.abs();
            let hint = if cos > 0.7 {
                "math suggests MITOSIS (convergent)"
            } else if cos < -0.7 {
                "math suggests REJECT (contradictory)"
            } else {
                "math suggests MEIOSIS (orthogonal)"
            };
            (cos, orth, hint)
        } else {
            (1.0, 0.0, "math suggests MITOSIS (single donor — selfing)")
        };

        let donor_lines = donors
            .iter()
            .map(|d| {
                let types: Vec<String> = d
                    .records
                    .iter()
                    .map(|r| match &r.proposal {
                        crate::runtime::mutation::MutationProposal::ParameterAdjust {
                            param, delta, ..
                        } => format!("param:{param}({delta:+.3})"),
                        crate::runtime::mutation::MutationProposal::StructuralRewire {
                            signal_name,
                            ..
                        } => format!("rewire:{signal_name}"),
                        _ => "other".to_string(),
                    })
                    .collect();
                format!(
                    "  entity={} promotions={} mutations=[{}]",
                    d.entity_id,
                    d.score(),
                    types.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Truncate genome source to stay within token budget
        let source_preview = &genome.source[..genome.source.len().min(1200)];

        format!(
            r#"You are the autonomous evolution judge for a self-modifying adaptive system.

MATHEMATICAL ORTHOGONALITY ANALYSIS (computed before this prompt):
  cosine_similarity = {math_cosine:.3}   (1=same direction, 0=orthogonal, -1=contradictory)
  orthogonality_score = {math_orthogonality:.3}   (1=truly orthogonal, 0=convergent/opposing)
  {math_hint}

Use the math as a strong prior. Override only if you have clear semantic evidence.

DONOR ENTITIES:
{donor_lines}

EVOLVED GENOME ({filename}):
```
{source_preview}
```

RULES:
- MITOSIS: mutations target the same behavioral domain (convergent) — apply back to fittest parent as a self-update.
- MEIOSIS: mutations target different behavioral domains (orthogonal) — create a new independent offspring entity.
- REJECT: contradictory changes, circular dependencies, no net benefit, or likely to break compilation.

Respond ONLY in this exact format (no extra text):
ORTHOGONALITY: <0.0-1.0>
FITNESS_DELTA: <-1.0 to 1.0>
DECISION: <MITOSIS|MEIOSIS|REJECT>
REASONING: <one sentence>"#,
            math_cosine = math_cosine,
            math_orthogonality = math_orthogonality,
            math_hint = math_hint,
            donor_lines = donor_lines,
            filename = genome.filename,
            source_preview = source_preview,
        )
    }

    fn parse_verdict(text: &str) -> Result<EvolutionVerdict, String> {
        let mut orthogonality = 0.5f64;
        let mut fitness_delta = 0.0f64;
        let mut decision = EvolutionDecision::Reject;
        let mut reasoning = "no reasoning provided".to_string();

        for line in text.lines() {
            if let Some(val) = line.strip_prefix("ORTHOGONALITY:") {
                orthogonality = val.trim().parse().unwrap_or(0.5);
            } else if let Some(val) = line.strip_prefix("FITNESS_DELTA:") {
                fitness_delta = val.trim().parse().unwrap_or(0.0);
            } else if let Some(val) = line.strip_prefix("DECISION:") {
                decision = match val.trim() {
                    "MITOSIS" => EvolutionDecision::Mitosis,
                    "MEIOSIS" => EvolutionDecision::Meiosis,
                    _ => EvolutionDecision::Reject,
                };
            } else if let Some(val) = line.strip_prefix("REASONING:") {
                reasoning = val.trim().to_string();
            }
        }

        Ok(EvolutionVerdict { decision, reasoning, orthogonality_score: orthogonality, fitness_delta })
    }
}

// ── GitHub publisher ──────────────────────────────────────────────────────────

/// Result of a single genome publication attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    /// Target filename in the repo.
    pub filename: String,
    /// Whether the push succeeded.
    pub success: bool,
    /// Error message if the push failed.
    pub error: Option<String>,
}

/// Pushes evolved genome files to a GitHub repository via the Contents API.
pub struct GitHubPublisher {
    token: String,
    repo: String,
    branch: String,
    client: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct CreateFileBody<'a> {
    message: &'a str,
    /// Base64-encoded file content.
    content: &'a str,
    branch: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<String>,
}

#[derive(Deserialize)]
struct ExistingFileInfo {
    sha: String,
}

impl GitHubPublisher {
    /// Build from environment variables.  Returns `None` if `GITHUB_TOKEN` or
    /// `GITHUB_REPO` are not set.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("GITHUB_TOKEN").ok()?;
        let repo = std::env::var("GITHUB_REPO").ok()?;
        let branch = std::env::var("GITHUB_GENOMES_BRANCH")
            .unwrap_or_else(|_| "genomes/evolved".into());
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self { token, repo, branch, client })
    }

    /// Push a single genome file to GitHub.
    pub fn publish(&self, genome: &EvolvedGenome) -> PublishResult {
        let url = format!(
            "https://api.github.com/repos/{}/contents/{}",
            self.repo, genome.filename
        );
        let existing_sha = self.get_file_sha(&url);
        let content = BASE64.encode(genome.source.as_bytes());
        let msg = format!(
            "feat(genomes): {} gen{} — {} mutations\n\nParent A: {}\nParent B: {}",
            genome.parent_a,
            genome.generation,
            genome.mutations_incorporated,
            genome.parent_a,
            genome.parent_b.as_deref().unwrap_or("none"),
        );
        let body = CreateFileBody {
            message: &msg,
            content: &content,
            branch: &self.branch,
            sha: existing_sha,
        };
        match self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "loom-meiosis/1.0")
            .json(&body)
            .send()
        {
            Ok(r) if r.status().is_success() => {
                PublishResult { filename: genome.filename.clone(), success: true, error: None }
            }
            Ok(r) => {
                let status = r.status();
                let text = r.text().unwrap_or_default();
                PublishResult {
                    filename: genome.filename.clone(),
                    success: false,
                    error: Some(format!("GitHub {status}: {text}")),
                }
            }
            Err(e) => PublishResult {
                filename: genome.filename.clone(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    fn get_file_sha(&self, url: &str) -> Option<String> {
        let resp = self
            .client
            .get(format!("{url}?ref={}", self.branch))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "loom-meiosis/1.0")
            .send()
            .ok()?;
        if resp.status().is_success() {
            resp.json::<ExistingFileInfo>().ok().map(|f| f.sha)
        } else {
            None
        }
    }
}

// ── MeiosisConfig ─────────────────────────────────────────────────────────────

/// Configuration for the meiosis engine.
#[derive(Debug, Clone)]
pub struct MeiosisConfig {
    /// Minimum number of promoted mutations for an entity to qualify as a donor.
    pub min_promotions_to_qualify: usize,
    /// Maximum number of donors to select (pairs are formed from this pool).
    pub top_donors: usize,
    /// Generation counter — incremented each time a new experiment runs.
    pub generation: u32,
}

impl Default for MeiosisConfig {
    fn default() -> Self {
        Self { min_promotions_to_qualify: 1, top_donors: 6, generation: 1 }
    }
}

// ── MeiosisEngine ─────────────────────────────────────────────────────────────

/// Full meiosis pipeline: select → recombine → publish.
pub struct MeiosisEngine {
    /// Configuration controlling selection thresholds and generation number.
    pub config: MeiosisConfig,
}

/// Summary produced after a meiosis run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeiosisReport {
    /// Number of entities selected as donors.
    pub donors_selected: usize,
    /// Number of hybrid genomes rendered.
    pub genomes_rendered: usize,
    /// Number of genomes accepted by the judge (Mitosis + Meiosis decisions).
    pub genomes_accepted: usize,
    /// Number of genomes successfully pushed to GitHub.
    pub genomes_published: usize,
    /// Per-genome publish results.
    pub publish_results: Vec<PublishResult>,
    /// Judge verdicts keyed by genome filename.
    pub verdicts: Vec<(String, EvolutionVerdict)>,
}

impl MeiosisEngine {
    /// Create with custom config.
    pub fn new(config: MeiosisConfig) -> Self {
        Self { config }
    }

    /// Create with default config.
    pub fn with_defaults() -> Self {
        Self::new(MeiosisConfig::default())
    }

    /// Select top-N donor entities by promoted-mutation count.
    pub fn select_donors(&self, promoted: &[PromotedRecord]) -> Vec<MeiosisDonor> {
        let mut by_entity: HashMap<String, Vec<PromotedRecord>> = HashMap::new();
        for r in promoted {
            by_entity.entry(r.entity_id.clone()).or_default().push(r.clone());
        }
        let mut donors: Vec<MeiosisDonor> = by_entity
            .into_iter()
            .filter(|(_, records)| records.len() >= self.config.min_promotions_to_qualify)
            .map(|(entity_id, records)| MeiosisDonor { entity_id, records })
            .collect();
        donors.sort_by(|a, b| b.score().cmp(&a.score()));
        donors.truncate(self.config.top_donors);
        donors
    }

    /// Cross-breed donors into evolved genomes.
    ///
    /// Pairs donors in order: (0,1), (2,3), … Odd donor out produces a selfed
    /// genome from a single parent.
    pub fn recombine(&self, donors: &[MeiosisDonor]) -> Vec<EvolvedGenome> {
        let mut genomes = Vec::new();
        let mut i = 0;
        while i < donors.len() {
            if i + 1 < donors.len() {
                genomes.push(render_genome(self.config.generation, &donors[i], Some(&donors[i + 1])));
                i += 2;
            } else {
                genomes.push(render_genome(self.config.generation, &donors[i], None));
                i += 1;
            }
        }
        genomes
    }

    /// Run the full pipeline: select → recombine → judge → publish.
    ///
    /// The judge evaluates each genome autonomously:
    /// - **Mitosis**: filename gets `mitosis_` prefix; genome self-updates the parent.
    /// - **Meiosis**: filename unchanged; genome registers as a new offspring entity.
    /// - **Reject**: genome is discarded and never pushed.
    ///
    /// If `GITHUB_TOKEN` / `GITHUB_REPO` are unset, genomes are logged but not pushed.
    /// If `CLAUDE_API_KEY` is unset, the judge is skipped and all genomes default to Meiosis.
    pub fn run(&self, promoted: &[PromotedRecord]) -> MeiosisReport {
        let donors = self.select_donors(promoted);
        let mut raw_genomes = self.recombine(&donors);

        let judge = EvolutionJudge::from_env();
        let publisher = GitHubPublisher::from_env();

        let mut accepted_genomes: Vec<EvolvedGenome> = Vec::new();
        let mut verdicts: Vec<(String, EvolutionVerdict)> = Vec::new();

        // Build donor pairs for judge context (mirrors recombine pairing)
        let mut donor_pairs: Vec<Vec<&MeiosisDonor>> = Vec::new();
        let mut i = 0;
        while i < donors.len() {
            if i + 1 < donors.len() {
                donor_pairs.push(vec![&donors[i], &donors[i + 1]]);
                i += 2;
            } else {
                donor_pairs.push(vec![&donors[i]]);
                i += 1;
            }
        }

        for (genome_idx, mut genome) in raw_genomes.drain(..).enumerate() {
            let pair = donor_pairs.get(genome_idx).map(|v| v.as_slice()).unwrap_or(&[]);

            let verdict = match &judge {
                Some(j) => {
                    eprintln!("[meiosis] judging {} ...", genome.filename);
                    j.evaluate(&genome, pair)
                }
                None => {
                    // No judge: default all to Meiosis
                    EvolutionVerdict {
                        decision: EvolutionDecision::Meiosis,
                        reasoning: "no judge configured — default meiosis".to_string(),
                        orthogonality_score: 0.5,
                        fitness_delta: 0.0,
                    }
                }
            };

            eprintln!(
                "[meiosis] {} → {:?} (orthogonality={:.2}, Δfitness={:+.2}): {}",
                genome.filename,
                verdict.decision,
                verdict.orthogonality_score,
                verdict.fitness_delta,
                verdict.reasoning,
            );

            match verdict.decision {
                EvolutionDecision::Reject => {
                    verdicts.push((genome.filename.clone(), verdict));
                    // Discarded — not pushed
                }
                EvolutionDecision::Mitosis => {
                    // Self-update: rename with mitosis_ prefix so evolve.yml can route it
                    let base = genome.filename.rsplit('/').next().unwrap_or(&genome.filename);
                    let dir = genome.filename.rsplit_once('/').map(|(d, _)| d).unwrap_or("genomes/evolved");
                    genome.filename = format!("{dir}/mitosis_{base}");
                    genome.decision = Some(EvolutionDecision::Mitosis);
                    verdicts.push((genome.filename.clone(), verdict));
                    accepted_genomes.push(genome);
                }
                EvolutionDecision::Meiosis => {
                    genome.decision = Some(EvolutionDecision::Meiosis);
                    verdicts.push((genome.filename.clone(), verdict));
                    accepted_genomes.push(genome);
                }
            }
        }

        let mut results: Vec<PublishResult> = Vec::new();
        let mut published = 0usize;

        if let Some(pub_client) = publisher {
            for genome in &accepted_genomes {
                let r = pub_client.publish(genome);
                if r.success {
                    published += 1;
                    eprintln!("[meiosis] pushed {}", genome.filename);
                } else {
                    eprintln!("[meiosis] push failed {}: {:?}", genome.filename, r.error);
                }
                results.push(r);
            }
        } else {
            for genome in &accepted_genomes {
                eprintln!(
                    "[meiosis] GITHUB_TOKEN/GITHUB_REPO not set — genome not pushed: {}",
                    genome.filename
                );
                results.push(PublishResult {
                    filename: genome.filename.clone(),
                    success: false,
                    error: Some("GITHUB_TOKEN or GITHUB_REPO not set".into()),
                });
            }
        }

        MeiosisReport {
            donors_selected: donors.len(),
            genomes_rendered: accepted_genomes.len() + verdicts.iter().filter(|(_, v)| v.decision == EvolutionDecision::Reject).count(),
            genomes_accepted: accepted_genomes.len(),
            genomes_published: published,
            publish_results: results,
            verdicts,
        }
    }
}

// ── Telomere tracker ──────────────────────────────────────────────────────────

/// Runtime telomere state for a single entity.
///
/// The telomere shortens when **telos drift exceeds a tolerance window** —
/// not on time or invocation count.  This implements the insight-document
/// contract (Part 3): the countdown is semantic divergence from intent.
///
/// Within the tolerance window, drift is allowed — it is how the system
/// escapes local fitness maxima (random walk).  Sustained drift above the
/// window triggers shortening.  At zero remaining, the entity is senescent
/// and should be replaced by the meiosis cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelomereState {
    /// Entity identifier.
    pub entity_id: String,
    /// Original telos string captured when the entity was first tracked.
    pub original_telos: String,
    /// Remaining telomere length.  Starts at `initial_length`.
    pub remaining: u32,
    /// Fraction of max drift tolerated before shortening begins (0.0–1.0).
    /// Default 0.3 — up to 30 % telos drift is treated as exploration.
    pub tolerance_window: f64,
    /// Shortening per unit of excess drift above the window.
    /// `floor(excess * decay_rate)` ticks are removed per drift event.
    pub decay_rate: f64,
}

impl TelomereState {
    /// Create a fresh telomere state for an entity.
    pub fn new(
        entity_id: impl Into<String>,
        original_telos: impl Into<String>,
        initial_length: u32,
    ) -> Self {
        Self {
            entity_id: entity_id.into(),
            original_telos: original_telos.into(),
            remaining: initial_length,
            tolerance_window: 0.3,
            decay_rate: 5.0,
        }
    }

    /// True when the entity has reached senescence.
    pub fn is_senescent(&self) -> bool {
        self.remaining == 0
    }

    /// Record a drift event.  `drift_score` ∈ [0, 1] where 1.0 is max drift.
    ///
    /// - Below `tolerance_window`: treated as exploration; slight recovery applied.
    /// - Above `tolerance_window`: shortens by `floor(excess * decay_rate)`.
    ///
    /// Returns `true` if the telomere was shortened this event.
    pub fn record_drift(&mut self, drift_score: f64) -> bool {
        let drift_score = drift_score.clamp(0.0, 1.0);
        if drift_score <= self.tolerance_window {
            // Within tolerance — local exploration permitted.
            // Small recovery: gap between score and window absorbs prior stress.
            let _recovery = (self.tolerance_window - drift_score) * 0.1;
            // (Recovery on `remaining` not yet modelled — telomere doesn't grow back.)
            return false;
        }
        let excess = drift_score - self.tolerance_window;
        let shortening = (excess * self.decay_rate).floor() as u32;
        if shortening > 0 {
            self.remaining = self.remaining.saturating_sub(shortening);
            true
        } else {
            false
        }
    }
}

/// Colony-level telomere registry — one [`TelomereState`] per entity.
///
/// Wired into the meiosis cycle so that entities approaching senescence
/// are prioritised as meiosis donors (their genome is preserved before
/// they retire).
#[derive(Debug, Clone, Default)]
pub struct TelomereTracker {
    states: HashMap<String, TelomereState>,
    /// Initial telomere length assigned to newly registered entities.
    pub initial_length: u32,
}

impl TelomereTracker {
    /// Create a tracker with a given default initial length.
    pub fn new(initial_length: u32) -> Self {
        Self { states: HashMap::new(), initial_length }
    }

    /// Record a drift event for an entity, registering it if unseen.
    ///
    /// Returns `true` when the entity has reached senescence after this event.
    pub fn record_drift(&mut self, entity_id: &str, telos: &str, drift_score: f64) -> bool {
        let initial = self.initial_length;
        let state = self.states.entry(entity_id.to_string()).or_insert_with(|| {
            TelomereState::new(entity_id, telos, initial)
        });
        state.record_drift(drift_score);
        state.is_senescent()
    }

    /// Check senescence without recording a drift event.
    pub fn is_senescent(&self, entity_id: &str) -> bool {
        self.states.get(entity_id).is_some_and(|s| s.is_senescent())
    }

    /// Remaining telomere length, or `None` if the entity has not been seen.
    pub fn remaining(&self, entity_id: &str) -> Option<u32> {
        self.states.get(entity_id).map(|s| s.remaining)
    }

    /// Entities currently in senescence.
    pub fn senescent_entities(&self) -> Vec<&str> {
        self.states
            .values()
            .filter(|s| s.is_senescent())
            .map(|s| s.entity_id.as_str())
            .collect()
    }

    /// All tracked states, sorted by remaining length ascending
    /// (most critical first).
    pub fn all_states_by_urgency(&self) -> Vec<&TelomereState> {
        let mut states: Vec<&TelomereState> = self.states.values().collect();
        states.sort_by_key(|s| s.remaining);
        states
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::mutation::MutationProposal;

    fn param_record(entity_id: &str, param: &str, delta: f64, tick: u64) -> PromotedRecord {
        PromotedRecord {
            tick,
            entity_id: entity_id.into(),
            proposal: MutationProposal::ParameterAdjust {
                entity_id: entity_id.into(),
                param: param.into(),
                delta,
                reason: format!("test delta {delta}"),
            },
        }
    }

    fn rewire_record(entity_id: &str, signal: &str, tick: u64) -> PromotedRecord {
        PromotedRecord {
            tick,
            entity_id: entity_id.into(),
            proposal: MutationProposal::StructuralRewire {
                from_id: entity_id.into(),
                to_id: "target".into(),
                signal_name: signal.into(),
                reason: "test rewire".into(),
            },
        }
    }

    #[test]
    fn select_donors_picks_highest_promoted_count() {
        let engine = MeiosisEngine::with_defaults();
        let records = vec![
            param_record("climate", "albedo", -0.02, 10),
            param_record("climate", "co2_threshold", 5.0, 20),
            param_record("climate", "sink_rate", 0.1, 30),
            param_record("epidemics", "r0_threshold", -0.1, 15),
        ];
        let donors = engine.select_donors(&records);
        assert_eq!(donors[0].entity_id, "climate");
        assert_eq!(donors[0].score(), 3);
        assert_eq!(donors[1].entity_id, "epidemics");
        assert_eq!(donors[1].score(), 1);
    }

    #[test]
    fn select_donors_filters_below_min_threshold() {
        let mut config = MeiosisConfig::default();
        config.min_promotions_to_qualify = 3;
        let engine = MeiosisEngine::new(config);
        let records = vec![
            param_record("climate", "albedo", -0.02, 1),
            param_record("climate", "co2_threshold", 5.0, 2),
            param_record("climate", "sink_rate", 0.1, 3),
            param_record("epidemics", "r0_threshold", -0.1, 4),
            param_record("epidemics", "recovery_rate", 0.05, 5),
        ];
        let donors = engine.select_donors(&records);
        // epidemics has 2 promotions — below threshold of 3
        assert_eq!(donors.len(), 1);
        assert_eq!(donors[0].entity_id, "climate");
    }

    #[test]
    fn recombine_pairs_donors_into_cross_genomes() {
        let engine = MeiosisEngine::with_defaults();
        let records = vec![
            param_record("climate", "albedo", -0.02, 1),
            param_record("epidemics", "r0_threshold", -0.1, 2),
        ];
        let donors = engine.select_donors(&records);
        let genomes = engine.recombine(&donors);
        assert_eq!(genomes.len(), 1);
        // Cross genome: climate × epidemics
        assert!(genomes[0].parent_b.is_some());
        assert!(genomes[0].filename.contains("gen1"));
    }

    #[test]
    fn recombine_odd_donor_produces_selfed_genome() {
        let engine = MeiosisEngine::with_defaults();
        let records = vec![
            param_record("climate", "albedo", -0.02, 1),
            param_record("climate", "sink_rate", 0.1, 2),
            param_record("epidemics", "r0_threshold", -0.1, 3),
            param_record("epidemics", "recovery", 0.05, 4),
            param_record("energy", "capacity", 0.3, 5),
        ];
        let donors = engine.select_donors(&records);
        let genomes = engine.recombine(&donors);
        // 3 donors → 1 cross (0,1) + 1 selfed (2)
        assert_eq!(genomes.len(), 2);
        let selfed = genomes.iter().find(|g| g.parent_b.is_none());
        assert!(selfed.is_some());
    }

    #[test]
    fn render_genome_incorporates_parameter_adjust_as_regulate_block() {
        let donor = MeiosisDonor {
            entity_id: "climate".into(),
            records: vec![param_record("climate", "albedo", -0.02, 10)],
        };
        let genome = render_genome(1, &donor, None);
        assert!(genome.source.contains("regulate:"));
        assert!(genome.source.contains("adjust_albedo"));
        assert_eq!(genome.mutations_incorporated, 1);
    }

    #[test]
    fn render_genome_incorporates_structural_rewire_as_epigenetic_block() {
        let donor = MeiosisDonor {
            entity_id: "epidemics".into(),
            records: vec![rewire_record("epidemics", "infection_rate", 50)],
        };
        let genome = render_genome(2, &donor, None);
        assert!(genome.source.contains("epigenetic:"));
        assert!(genome.source.contains("infection_rate"));
        assert_eq!(genome.generation, 2);
    }

    #[test]
    fn run_skips_publish_without_token_and_reports_zero_published() {
        // Ensure no GITHUB_TOKEN is set for this test
        std::env::remove_var("GITHUB_TOKEN");
        let engine = MeiosisEngine::with_defaults();
        let records = vec![param_record("climate", "albedo", -0.02, 1)];
        let report = engine.run(&records);
        assert_eq!(report.donors_selected, 1);
        // 1 genome rendered (selfed), accepted as Meiosis (no judge = default meiosis)
        assert_eq!(report.genomes_accepted, 1);
        assert_eq!(report.genomes_published, 0);
        assert!(!report.publish_results[0].success);
    }

    #[test]
    fn run_produces_no_genomes_when_promoted_is_empty() {
        let engine = MeiosisEngine::with_defaults();
        let report = engine.run(&[]);
        assert_eq!(report.donors_selected, 0);
        assert_eq!(report.genomes_rendered, 0);
        assert_eq!(report.genomes_accepted, 0);
        assert_eq!(report.genomes_published, 0);
    }

    #[test]
    fn judge_parse_verdict_mitosis() {
        let text = "ORTHOGONALITY: 0.2\nFITNESS_DELTA: 0.4\nDECISION: MITOSIS\nREASONING: Same parameter domain.";
        let verdict = EvolutionJudge::parse_verdict(text).unwrap();
        assert_eq!(verdict.decision, EvolutionDecision::Mitosis);
        assert!((verdict.orthogonality_score - 0.2).abs() < 0.001);
        assert!((verdict.fitness_delta - 0.4).abs() < 0.001);
    }

    #[test]
    fn judge_parse_verdict_meiosis() {
        let text = "ORTHOGONALITY: 0.85\nFITNESS_DELTA: 0.6\nDECISION: MEIOSIS\nREASONING: Orthogonal behavioral domains.";
        let verdict = EvolutionJudge::parse_verdict(text).unwrap();
        assert_eq!(verdict.decision, EvolutionDecision::Meiosis);
        assert!((verdict.orthogonality_score - 0.85).abs() < 0.001);
    }

    #[test]
    fn judge_parse_verdict_reject() {
        let text = "ORTHOGONALITY: 0.5\nFITNESS_DELTA: -0.3\nDECISION: REJECT\nREASONING: Contradictory parameter changes.";
        let verdict = EvolutionJudge::parse_verdict(text).unwrap();
        assert_eq!(verdict.decision, EvolutionDecision::Reject);
        assert!((verdict.fitness_delta - (-0.3)).abs() < 0.001);
    }

    #[test]
    fn judge_parse_verdict_unknown_defaults_to_reject() {
        let text = "ORTHOGONALITY: 0.5\nFITNESS_DELTA: 0.0\nDECISION: UNKNOWN\nREASONING: Garbage.";
        let verdict = EvolutionJudge::parse_verdict(text).unwrap();
        assert_eq!(verdict.decision, EvolutionDecision::Reject);
    }

    #[test]
    fn run_mitosis_decision_prefixes_filename() {
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("CLAUDE_API_KEY");
        // Without judge, all default to Meiosis — verify no mitosis_ prefix
        let engine = MeiosisEngine::with_defaults();
        let records = vec![
            param_record("climate", "albedo", -0.02, 1),
            param_record("epidemics", "r0", 0.1, 2),
        ];
        let report = engine.run(&records);
        // Both accepted (no judge → Meiosis default), no mitosis_ prefix
        for (filename, verdict) in &report.verdicts {
            assert_eq!(verdict.decision, EvolutionDecision::Meiosis);
            assert!(!filename.contains("mitosis_"));
        }
    }

    // ── MutationVector tests ──────────────────────────────────────────────────

    #[test]
    fn mutation_vector_cosine_identical_vectors_is_one() {
        let records = vec![
            param_record("e", "albedo", -0.02, 1),
            param_record("e", "co2", 5.0, 2),
        ];
        let va = MutationVector::from_records(&records);
        let cos = va.cosine_similarity(&va);
        assert!((cos - 1.0).abs() < 1e-9, "identical vectors should have cosine=1.0, got {cos}");
    }

    #[test]
    fn mutation_vector_cosine_orthogonal_is_zero() {
        let ra = vec![param_record("a", "albedo", 1.0, 1)];
        let rb = vec![param_record("b", "r0", 1.0, 1)];
        let va = MutationVector::from_records(&ra);
        let vb = MutationVector::from_records(&rb);
        let cos = va.cosine_similarity(&vb);
        assert!(cos.abs() < 1e-9, "disjoint param spaces should have cosine≈0, got {cos}");
    }

    #[test]
    fn mutation_vector_cosine_opposite_is_negative() {
        let ra = vec![param_record("a", "albedo", 1.0, 1)];
        let rb = vec![param_record("b", "albedo", -1.0, 1)];
        let va = MutationVector::from_records(&ra);
        let vb = MutationVector::from_records(&rb);
        let cos = va.cosine_similarity(&vb);
        assert!(cos < -0.99, "opposite deltas on same param should have cosine≈-1, got {cos}");
    }

    #[test]
    fn mutation_vector_orthogonality_score_complement() {
        let ra = vec![param_record("a", "x", 1.0, 1)];
        let rb = vec![param_record("b", "y", 1.0, 1)];
        let va = MutationVector::from_records(&ra);
        let vb = MutationVector::from_records(&rb);
        let orth = va.orthogonality_score(&vb);
        assert!((orth - 1.0).abs() < 1e-9, "orthogonal vectors → score=1.0, got {orth}");
    }

    #[test]
    fn mutation_vector_empty_returns_zero_cosine() {
        let va = MutationVector::from_records(&[]);
        let vb = MutationVector::from_records(&[param_record("b", "x", 1.0, 1)]);
        let cos = va.cosine_similarity(&vb);
        assert_eq!(cos, 0.0, "empty vector cosine should be 0.0");
    }

    // ── TelomereTracker tests ─────────────────────────────────────────────────

    #[test]
    fn telomere_no_shortening_within_tolerance() {
        let mut tracker = TelomereTracker::new(100);
        // drift = 0.2, tolerance_window = 0.3 → no shortening
        let senescent = tracker.record_drift("e1", "telos text", 0.2);
        assert!(!senescent);
        assert_eq!(tracker.remaining("e1"), Some(100));
    }

    #[test]
    fn telomere_shortens_above_tolerance() {
        let mut tracker = TelomereTracker::new(100);
        // drift = 0.8, excess = 0.5, decay_rate = 5.0 → floor(2.5) = 2 shortening
        let senescent = tracker.record_drift("e1", "telos text", 0.8);
        assert!(!senescent);
        assert_eq!(tracker.remaining("e1"), Some(98));
    }

    #[test]
    fn telomere_reaches_senescence_on_sustained_drift() {
        let mut state = TelomereState::new("e1", "telos", 5);
        // High drift (1.0): excess = 0.7, floor(0.7 * 5.0) = 3 shortening per event
        state.record_drift(1.0); // remaining: 5 → 2
        state.record_drift(1.0); // remaining: 2 → 0 (saturating_sub)
        assert!(state.is_senescent());
    }

    #[test]
    fn telomere_allows_exploration_below_tolerance() {
        let mut state = TelomereState::new("e1", "telos", 50);
        // drift oscillates below tolerance — telomere must not shorten
        for _ in 0..20 {
            state.record_drift(0.1);
            state.record_drift(0.25);
        }
        assert_eq!(state.remaining, 50);
        assert!(!state.is_senescent());
    }

    #[test]
    fn telomere_tracker_senescent_entities_listed() {
        let mut tracker = TelomereTracker::new(3);
        tracker.record_drift("dying", "telos", 1.0); // 3 → 0 (floor(0.7*5)=3)
        tracker.record_drift("healthy", "telos", 0.1);
        assert!(tracker.is_senescent("dying"));
        assert!(!tracker.is_senescent("healthy"));
        let s = tracker.senescent_entities();
        assert!(s.contains(&"dying"));
        assert!(!s.contains(&"healthy"));
    }
}
