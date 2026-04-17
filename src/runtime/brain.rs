//! Mammal Brain — Tier 3 synthesis engine (R6).
//!
//! Named after the mammalian cortex: the highest-level synthesis tier, called
//! only when Tiers 1 and 2 fail to converge.  Uses the Claude API (or any
//! OpenAI-compatible chat completions endpoint) to propose system-level mutations
//! from the full genome (`.loom` source), entity signals, and mutation history.
//!
//! # Cost guard
//!
//! To prevent runaway spend, the engine enforces a **call budget** expressed as
//! a maximum number of API calls per sliding hour window.  When the budget is
//! exhausted, [`MammalBrain::evaluate`] returns an empty vec rather than calling
//! the API.
//!
//! # Configuration (from environment)
//!
//! | Env var | Default | Description |
//! |---|---|---|
//! | `CLAUDE_API_KEY` | — | Required; API key for the Claude endpoint |
//! | `CLAUDE_BASE_URL` | `https://api.anthropic.com/v1` | API base URL |
//! | `CLAUDE_MODEL` | `claude-3-5-haiku-20241022` | Model to use |
//! | `BIOISO_MAX_TIER3_CALLS_PER_HOUR` | `10` | Cost guard limit |
//!
//! # No-network tests
//!
//! All unit tests inject a `MockClaudeClient` via the `ClaudeClient` trait.

use crate::runtime::{
    drift::DriftEvent,
    ganglion::parse_proposals,
    mutation::MutationProposal,
    signal::{now_ms, EntityId},
    store::SignalStore,
};

// ── Claude client trait + types ───────────────────────────────────────────────

/// Request body compatible with the Anthropic Messages API.
#[derive(Debug, Clone, serde::Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// The content block returned by the Claude API.
#[derive(Debug, Clone, serde::Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

/// Response from the Claude API.
#[derive(Debug, Clone, serde::Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

/// Trait allowing tests to inject a mock HTTP client.
pub trait ClaudeClient: Send + Sync {
    /// Send a system + user prompt pair and return the assistant's text.
    fn complete(&self, system: &str, user: &str) -> Result<String, String>;
}

/// Real blocking HTTP client for the Anthropic Messages API.
pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl AnthropicClient {
    /// Create a client from explicit parameters (prefer [`from_env`] in production).
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// Create a client from environment variables.  Returns `None` if
    /// neither `CLAUDE_API_KEY` nor `ANTHROPIC_API_KEY` is set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CLAUDE_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok()?;
        let base_url = std::env::var("CLAUDE_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".into());
        let model = std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into());
        Some(Self::new(base_url, api_key, model, 60))
    }
}

impl ClaudeClient for AnthropicClient {
    fn complete(&self, system: &str, user: &str) -> Result<String, String> {
        let url = format!("{}/messages", self.base_url);
        let body = ClaudeRequest {
            model: &self.model,
            max_tokens: 2048,
            messages: vec![ClaudeMessage {
                role: "user",
                content: user,
            }],
        };
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("x-system", system)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("Claude API error {status}: {text}"));
        }

        let claude_resp: ClaudeResponse = resp.json().map_err(|e| e.to_string())?;
        let text = claude_resp
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");
        Ok(text)
    }
}

// ── System-level prompt builder ───────────────────────────────────────────────

/// Build the system prompt for the Mammal Brain.
pub fn build_system_prompt() -> String {
    r#"You are the Mammal Brain — the highest synthesis tier of the BIOISO
biological computation runtime. You receive:
- The full Loom genome (.loom source) for an entity
- Its recent telemetry signals
- The telos drift history
- Prior mutation proposals and their gate verdicts

Your task: propose system-level mutations that restore convergence.
These may involve structural rewiring, entity cloning for redundancy,
or parameter adjustments that Tier 1 and Tier 2 could not discover.

Respond with ONLY a JSON array of mutation proposals. Each must be one of:
  {"kind":"parameter_adjust","entity_id":"...","param":"...","delta":0.0,"reason":"..."}
  {"kind":"entity_clone","source_id":"...","new_id":"...","reason":"..."}
  {"kind":"entity_rollback","entity_id":"...","checkpoint_id":0,"reason":"..."}
  {"kind":"entity_prune","entity_id":"...","reason":"..."}
  {"kind":"structural_rewire","from_id":"...","to_id":"...","signal_name":"...","reason":"..."}

Return ONLY the JSON array. No explanation. No markdown fences."#
        .into()
}

/// Build the user prompt from the full genome and signal corpus.
pub fn build_user_prompt(
    entity_id: &EntityId,
    genome: Option<&str>,
    drift_event: &DriftEvent,
    store: &SignalStore,
    recent_n: usize,
) -> String {
    let signals = store
        .signals_for_entity(entity_id, recent_n)
        .unwrap_or_default();
    let bounds = store.telos_bounds_for_entity(entity_id).unwrap_or_default();

    let mut prompt = String::new();
    prompt.push_str(&format!("# Entity: {entity_id}\n"));
    prompt.push_str(&format!(
        "# Drift: metric='{}' score={:.3} (CRITICAL — Tier 1 and Tier 2 failed to converge)\n\n",
        drift_event.triggering_metric, drift_event.score
    ));

    if let Some(src) = genome {
        prompt.push_str("# Genome (.loom source):\n```\n");
        prompt.push_str(src);
        prompt.push_str("\n```\n\n");
    }

    if !bounds.is_empty() {
        prompt.push_str("# Telos bounds:\n");
        for b in &bounds {
            prompt.push_str(&format!(
                "  metric={} min={:?} max={:?} target={:?}\n",
                b.metric, b.min, b.max, b.target
            ));
        }
        prompt.push('\n');
    }

    if !signals.is_empty() {
        prompt.push_str(&format!("# Last {n} signals:\n", n = signals.len()));
        for s in &signals {
            prompt.push_str(&format!("  {} = {} @ {}\n", s.metric, s.value, s.timestamp));
        }
    }

    prompt
}

// ── Cost guard ────────────────────────────────────────────────────────────────

/// Sliding-window call budget for Tier 3 API calls.
pub struct CostGuard {
    /// Maximum calls per hour.
    pub max_calls_per_hour: usize,
    /// Timestamps (ms) of recent calls within the window.
    call_log: Vec<u64>,
}

impl CostGuard {
    /// Create a guard with the given hourly call limit.
    pub fn new(max_calls_per_hour: usize) -> Self {
        Self {
            max_calls_per_hour,
            call_log: Vec::new(),
        }
    }

    /// Create a guard using `BIOISO_MAX_TIER3_CALLS_PER_HOUR` env var, default 200.
    pub fn from_env() -> Self {
        let limit = std::env::var("BIOISO_MAX_TIER3_CALLS_PER_HOUR")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);
        Self::new(limit)
    }

    /// Check whether a call is permitted. Returns `true` if under budget.
    pub fn is_permitted(&mut self) -> bool {
        let now = now_ms();
        let one_hour_ago = now.saturating_sub(3_600_000);
        self.call_log.retain(|&ts| ts > one_hour_ago);
        self.call_log.len() < self.max_calls_per_hour
    }

    /// Record a call. Must be called immediately after a successful API call.
    pub fn record_call(&mut self) {
        self.call_log.push(now_ms());
    }

    /// Remaining calls allowed in the current window.
    pub fn remaining(&mut self) -> usize {
        let now = now_ms();
        let one_hour_ago = now.saturating_sub(3_600_000);
        self.call_log.retain(|&ts| ts > one_hour_ago);
        self.max_calls_per_hour.saturating_sub(self.call_log.len())
    }
}

// ── MammalBrain engine ────────────────────────────────────────────────────────

/// Tier 3 synthesis engine.
pub struct MammalBrain {
    client: Box<dyn ClaudeClient>,
    /// The hourly call budget enforcer.
    pub cost_guard: CostGuard,
    /// Number of recent signals to include in the user prompt.
    pub corpus_lookback: usize,
}

impl MammalBrain {
    /// Create a Mammal Brain backed by `AnthropicClient::from_env()`.
    ///
    /// Returns `None` if `CLAUDE_API_KEY` is not set.
    pub fn from_env() -> Option<Self> {
        let client = AnthropicClient::from_env()?;
        Some(Self {
            client: Box::new(client),
            cost_guard: CostGuard::from_env(),
            corpus_lookback: 30,
        })
    }

    /// Create a Mammal Brain with a custom (mock) client. Used in tests.
    pub fn with_client(client: Box<dyn ClaudeClient>, max_calls_per_hour: usize) -> Self {
        Self {
            client,
            cost_guard: CostGuard::new(max_calls_per_hour),
            corpus_lookback: 30,
        }
    }

    /// Evaluate a drift event. Returns proposals or empty vec if cost guard
    /// is exhausted or the API call fails.
    pub fn evaluate(
        &mut self,
        event: &DriftEvent,
        store: &SignalStore,
        genome: Option<&str>,
    ) -> Vec<MutationProposal> {
        if !self.cost_guard.is_permitted() {
            return vec![];
        }

        let system = build_system_prompt();
        let user = build_user_prompt(&event.entity_id, genome, event, store, self.corpus_lookback);

        match self.client.complete(&system, &user) {
            Ok(text) => {
                self.cost_guard.record_call();
                let proposals = parse_proposals(&text);
                if proposals.is_empty() {
                    eprintln!(
                        "[T3] API call succeeded but yielded no proposals for `{}` \
                         (response len={})",
                        event.entity_id,
                        text.len()
                    );
                }
                proposals
            }
            Err(e) => {
                eprintln!("[T3] API error for `{}`: {e}", event.entity_id);
                vec![]
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{drift::DriftEvent, store::SignalStore};

    fn mem_store() -> SignalStore {
        SignalStore::new(":memory:").unwrap()
    }

    fn make_event(entity_id: &str, score: f64) -> DriftEvent {
        DriftEvent {
            entity_id: entity_id.into(),
            triggering_metric: "carbon_stock".into(),
            score,
            ts: 2_000_000,
            entity_aggregate_score: None,
            velocity: 0.0,
        }
    }

    struct MockClaudeClient {
        response: String,
    }

    impl ClaudeClient for MockClaudeClient {
        fn complete(&self, _system: &str, _user: &str) -> Result<String, String> {
            Ok(self.response.clone())
        }
    }

    fn brain_with_response(resp: &str, limit: usize) -> MammalBrain {
        MammalBrain::with_client(
            Box::new(MockClaudeClient {
                response: resp.into(),
            }),
            limit,
        )
    }

    // ── CostGuard ─────────────────────────────────────────────────────────────

    #[test]
    fn cost_guard_allows_calls_within_budget() {
        let mut guard = CostGuard::new(3);
        assert!(guard.is_permitted());
        guard.record_call();
        assert!(guard.is_permitted());
        guard.record_call();
        assert!(guard.is_permitted());
        guard.record_call();
        // now at limit
        assert!(!guard.is_permitted());
    }

    #[test]
    fn cost_guard_remaining_decreases_with_calls() {
        let mut guard = CostGuard::new(5);
        assert_eq!(guard.remaining(), 5);
        guard.record_call();
        guard.record_call();
        assert_eq!(guard.remaining(), 3);
    }

    #[test]
    fn cost_guard_never_negative_remaining() {
        let mut guard = CostGuard::new(1);
        guard.record_call();
        guard.record_call(); // over budget
        assert_eq!(guard.remaining(), 0);
    }

    // ── MammalBrain::evaluate ─────────────────────────────────────────────────

    #[test]
    fn evaluate_returns_proposals_from_mock_client() {
        let store = mem_store();
        store.register_entity("soil", "SoilModel", "{}", 0).unwrap();
        let mut brain = brain_with_response(
            r#"[{"kind":"parameter_adjust","entity_id":"soil","param":"carbon_input","delta":5.0,"reason":"cortex: replenish carbon"}]"#,
            10,
        );
        let event = make_event("soil", 0.9);
        let proposals = brain.evaluate(&event, &store, None);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(
            proposals[0],
            MutationProposal::ParameterAdjust { .. }
        ));
    }

    #[test]
    fn evaluate_returns_empty_when_cost_guard_exhausted() {
        let store = mem_store();
        let mut brain = brain_with_response("[]", 0); // limit = 0
        let event = make_event("x", 0.95);
        let proposals = brain.evaluate(&event, &store, None);
        assert!(proposals.is_empty(), "should be blocked by cost guard");
    }

    #[test]
    fn evaluate_decrements_cost_guard_on_call() {
        let store = mem_store();
        let mut brain = brain_with_response("[]", 3);
        let event = make_event("x", 0.8);
        brain.evaluate(&event, &store, None);
        brain.evaluate(&event, &store, None);
        assert_eq!(brain.cost_guard.remaining(), 1);
    }

    // ── build_user_prompt ─────────────────────────────────────────────────────

    #[test]
    fn build_user_prompt_includes_genome_when_provided() {
        let store = mem_store();
        let event = make_event("soil", 0.8);
        let genome = "module SoilModel\nbeing Soil\n  telos: \"sequester carbon\"\n  end\nend\nend";
        let prompt = build_user_prompt(&"soil".into(), Some(genome), &event, &store, 5);
        assert!(prompt.contains("Genome"));
        assert!(prompt.contains("sequester carbon"));
    }

    #[test]
    fn build_user_prompt_skips_genome_section_when_none() {
        let store = mem_store();
        let event = make_event("soil", 0.8);
        let prompt = build_user_prompt(&"soil".into(), None, &event, &store, 5);
        assert!(!prompt.contains("Genome"));
        assert!(prompt.contains("soil"));
    }

    // ── build_system_prompt ───────────────────────────────────────────────────

    #[test]
    fn system_prompt_contains_all_mutation_kinds() {
        let prompt = build_system_prompt();
        assert!(prompt.contains("parameter_adjust"));
        assert!(prompt.contains("entity_clone"));
        assert!(prompt.contains("entity_rollback"));
        assert!(prompt.contains("entity_prune"));
        assert!(prompt.contains("structural_rewire"));
    }
}
