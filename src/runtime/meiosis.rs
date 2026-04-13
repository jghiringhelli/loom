//! Meiosis engine — R14: cross-entity genetic recombination + GitHub publication.
//!
//! After an experiment completes, [`MeiosisEngine`] closes the evolutionary loop:
//!
//! 1. **Select donors** — entities with the most promoted mutations qualify.
//! 2. **Recombine** — pair donors and cross-breed their mutations into hybrid
//!    `.loom` genome files (parameter strategies from parent A, structural rewires
//!    from parent B).
//! 3. **Publish** — push rendered genomes to the `genomes/evolved/` branch on
//!    GitHub via the REST API.
//!
//! The GitHub Actions `evolve.yml` workflow then validates each incoming genome:
//! - `loom compile` — syntax + type-check
//! - Gauntlet — if the survival gate passes, open a PR for human review.
//!
//! # Configuration (environment)
//!
//! | Env var | Default | Description |
//! |---|---|---|
//! | `GITHUB_TOKEN` | — | PAT with `contents: write` on the target repo |
//! | `GITHUB_REPO` | — | `owner/repo` (e.g. `PragmaWorks/loom`) |
//! | `GITHUB_GENOMES_BRANCH` | `genomes/evolved` | Branch to push genomes to |

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
    /// Relative path for the GitHub file (e.g. `genomes/evolved/gen1/climate_epidemics.loom`).
    pub filename: String,
    /// Full `.loom` source content.
    pub source: String,
    /// Total number of mutations incorporated from both parents.
    pub mutations_incorporated: usize,
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
    EvolvedGenome { generation, parent_a: parent_a.entity_id.clone(), parent_b: parent_b.map(|b| b.entity_id.clone()), filename, source, mutations_incorporated }
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
    /// Number of genomes successfully pushed to GitHub.
    pub genomes_published: usize,
    /// Per-genome publish results.
    pub publish_results: Vec<PublishResult>,
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

    /// Run the full pipeline: select → recombine → publish.
    ///
    /// If `GITHUB_TOKEN` / `GITHUB_REPO` are unset, genomes are logged to stderr
    /// but the function returns without error.
    pub fn run(&self, promoted: &[PromotedRecord]) -> MeiosisReport {
        let donors = self.select_donors(promoted);
        let genomes = self.recombine(&donors);

        let publisher = GitHubPublisher::from_env();
        let mut results = Vec::new();
        let mut published = 0usize;

        if let Some(pub_client) = publisher {
            for genome in &genomes {
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
            for genome in &genomes {
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
            genomes_rendered: genomes.len(),
            genomes_published: published,
            publish_results: results,
        }
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
        assert_eq!(report.genomes_rendered, 1);
        assert_eq!(report.genomes_published, 0);
        assert!(!report.publish_results[0].success);
    }

    #[test]
    fn run_produces_no_genomes_when_promoted_is_empty() {
        let engine = MeiosisEngine::with_defaults();
        let report = engine.run(&[]);
        assert_eq!(report.donors_selected, 0);
        assert_eq!(report.genomes_rendered, 0);
        assert_eq!(report.genomes_published, 0);
    }
}
