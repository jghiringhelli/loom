//! Ganglion — Tier 2 synthesis engine (R5).
//!
//! Named after a biological *ganglion*: a cluster of nerve cells that processes
//! signals semi-autonomously between the peripheral and central nervous systems.
//!
//! When Tier 1 (Polycephalum) fails to converge after a configurable number of
//! cycles, the Ganglion sends the signal corpus to a locally-running micro-LLM
//! via the [Ollama](https://ollama.com) HTTP API and parses the proposed `.loom`
//! mutations from the response.
//!
//! # Configuration
//!
//! | Field | Default | Description |
//! |---|---|---|
//! | `base_url` | `http://localhost:11434` | Ollama API base URL |
//! | `model` | `"phi3"` | Model name to use |
//! | `tier1_fail_threshold` | `3` | Consecutive Tier 1 zero-proposal cycles before escalation |
//! | `timeout_secs` | `30` | HTTP request timeout |
//!
//! # No-network tests
//!
//! All tests in this module use a `MockOllamaClient` (injected via a trait) so
//! that the Ganglion engine itself can be tested without a running Ollama instance.

use crate::runtime::{
    drift::DriftEvent,
    mutation::MutationProposal,
    signal::EntityId,
    store::SignalStore,
};

// ── Ollama HTTP client ────────────────────────────────────────────────────────

/// Response from the Ollama `/api/generate` endpoint.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OllamaResponse {
    /// The generated text.
    pub response: String,
    /// Whether the generation completed.
    #[serde(default)]
    pub done: bool,
}

/// Request body for the Ollama `/api/generate` endpoint.
#[derive(Debug, Clone, serde::Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

/// Trait allowing tests to inject a mock HTTP client.
pub trait OllamaClient: Send + Sync {
    /// Send a prompt to the model and return the raw response text.
    fn generate(&self, model: &str, prompt: &str) -> Result<String, String>;

    /// Check that the Ollama service is reachable.
    fn health_check(&self) -> bool;
}

// ── Claude-backed Tier 2 client ───────────────────────────────────────────────

/// Anthropic Messages API response (minimal — only what Ganglion needs).
#[derive(Debug, serde::Deserialize)]
struct ClaudeGanglionResponse {
    content: Vec<ClaudeGanglionBlock>,
}

#[derive(Debug, serde::Deserialize)]
struct ClaudeGanglionBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct ClaudeGanglionRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeGanglionMessage<'a>>,
}

#[derive(Debug, serde::Serialize)]
struct ClaudeGanglionMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Claude-backed implementation of [`OllamaClient`].
///
/// Allows Tier 2 (Ganglion) to use the Anthropic API when Ollama is not
/// available.  Configured via environment variables:
///
/// | Env var | Default | Description |
/// |---|---|---|
/// | `CLAUDE_API_KEY` | — | Required |
/// | `CLAUDE_BASE_URL` | `https://api.anthropic.com/v1` | API base URL |
/// | `GANGLION_CLAUDE_MODEL` | `claude-3-haiku-20240307` | Model to use (cheapest) |
///
/// The `model` argument passed to [`generate`] is ignored — the client always
/// uses its own configured model (Ollama model names are not valid here).
pub struct ClaudeGanglionClient {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl ClaudeGanglionClient {
    /// Create a client from environment variables.  Returns `None` if
    /// `CLAUDE_API_KEY` is not set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("CLAUDE_API_KEY").ok()?;
        let base_url = std::env::var("CLAUDE_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".into());
        let model = std::env::var("GANGLION_CLAUDE_MODEL")
            .unwrap_or_else(|_| "claude-3-haiku-20240307".into());
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self { base_url, api_key, model, client })
    }
}

impl OllamaClient for ClaudeGanglionClient {
    fn generate(&self, _model: &str, prompt: &str) -> Result<String, String> {
        let url = format!("{}/messages", self.base_url);
        let body = ClaudeGanglionRequest {
            model: &self.model,
            max_tokens: 512,
            messages: vec![ClaudeGanglionMessage { role: "user", content: prompt }],
        };
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("Ganglion Claude error {status}: {text}"));
        }

        let cr: ClaudeGanglionResponse = resp.json().map_err(|e| e.to_string())?;
        let text = cr
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");
        Ok(text)
    }

    /// Returns `true` immediately (no network call) — the presence of an API
    /// key is sufficient to declare the backend healthy.
    fn health_check(&self) -> bool {
        true
    }
}

// ── Ollama HTTP client ────────────────────────────────────────────────────────

/// Real blocking HTTP client backed by `reqwest`.
pub struct ReqwestOllamaClient {
    base_url: String,
    timeout_secs: u64,
    client: reqwest::blocking::Client,
    /// Separate short-timeout client used only for health checks.
    /// Allows fast failure when Ollama is not running (e.g. in tests / CI).
    health_client: reqwest::blocking::Client,
}

impl ReqwestOllamaClient {
    /// Create a client targeting `base_url` (default: `http://localhost:11434`).
    pub fn new(base_url: impl Into<String>, timeout_secs: u64) -> Self {
        let health_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .connect_timeout(std::time::Duration::from_millis(300))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.into(),
            timeout_secs,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("failed to build reqwest client"),
            health_client,
        }
    }
}

impl OllamaClient for ReqwestOllamaClient {
    fn generate(&self, model: &str, prompt: &str) -> Result<String, String> {
        let url = format!("{}/api/generate", self.base_url);
        let body = OllamaRequest { model, prompt, stream: false };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let ollama_resp: OllamaResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(ollama_resp.response)
    }

    fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.health_client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// ── Signal corpus serializer ──────────────────────────────────────────────────

/// Serialise the signal corpus (entity state + drift history + telos bounds)
/// into a structured prompt block for the LLM.
pub fn serialize_corpus(
    entity_id: &str,
    drift_event: &DriftEvent,
    store: &SignalStore,
    recent_n: usize,
) -> String {
    let signals = store
        .signals_for_entity(entity_id, recent_n)
        .unwrap_or_default();
    let bounds = store
        .telos_bounds_for_entity(entity_id)
        .unwrap_or_default();

    let mut corpus = String::new();
    corpus.push_str(&format!("# Entity: {entity_id}\n"));
    corpus.push_str(&format!(
        "# Drift event: metric='{}' score={:.3} ts={}\n",
        drift_event.triggering_metric, drift_event.score, drift_event.ts
    ));

    if !bounds.is_empty() {
        corpus.push_str("# Telos bounds:\n");
        for b in &bounds {
            corpus.push_str(&format!(
                "#   metric={} min={:?} max={:?} target={:?}\n",
                b.metric, b.min, b.max, b.target
            ));
        }
    }

    if !signals.is_empty() {
        corpus.push_str(&format!("# Last {} signals (newest first):\n", signals.len()));
        for s in &signals {
            corpus.push_str(&format!(
                "#   {} = {} @ {}\n",
                s.metric, s.value, s.timestamp
            ));
        }
    }
    corpus
}

// ── Prompt builder ────────────────────────────────────────────────────────────

/// Build the full LLM prompt from the signal corpus.
///
/// The prompt instructs the model to respond with a JSON array of
/// `MutationProposal` objects.
pub fn build_prompt(entity_id: &EntityId, corpus: &str) -> String {
    format!(
        r#"{corpus}

You are the Ganglion synthesis engine for entity '{entity_id}'.
The entity's telos is drifting. Propose mutations to restore convergence.

Respond with ONLY a JSON array of mutation proposals.  Each proposal must be
one of these shapes:
  {{"kind":"parameter_adjust","entity_id":"...","param":"...","delta":0.0,"reason":"..."}}
  {{"kind":"entity_clone","source_id":"...","new_id":"...","reason":"..."}}
  {{"kind":"entity_rollback","entity_id":"...","checkpoint_id":0,"reason":"..."}}
  {{"kind":"entity_prune","entity_id":"...","reason":"..."}}
  {{"kind":"structural_rewire","from_id":"...","to_id":"...","signal_name":"...","reason":"..."}}

Return ONLY the JSON array.  No explanation. No markdown fences.
"#,
        corpus = corpus,
        entity_id = entity_id
    )
}

/// Parse a JSON array of `MutationProposal`s from LLM response text.
///
/// Extracts the first `[...]` block found in the text (tolerates surrounding prose).
pub fn parse_proposals(text: &str) -> Vec<MutationProposal> {
    let start = text.find('[');
    let end = text.rfind(']');
    match (start, end) {
        (Some(s), Some(e)) if e >= s => {
            let json_slice = &text[s..=e];
            serde_json::from_str(json_slice).unwrap_or_default()
        }
        _ => vec![],
    }
}

// ── Ganglion engine ───────────────────────────────────────────────────────────

/// Configuration for the Ganglion engine.
#[derive(Debug, Clone)]
pub struct GanglionConfig {
    /// Ollama API base URL.
    pub base_url: String,
    /// Model name.
    pub model: String,
    /// Number of consecutive Tier 1 zero-proposal cycles before Ganglion fires.
    pub tier1_fail_threshold: usize,
    /// HTTP timeout in seconds.
    pub timeout_secs: u64,
    /// Number of recent signals to include in the corpus.
    pub corpus_lookback: usize,
}

impl Default for GanglionConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".into(),
            model: "phi3".into(),
            tier1_fail_threshold: 3,
            timeout_secs: 30,
            corpus_lookback: 20,
        }
    }
}

/// Tier 2 synthesis engine.
pub struct Ganglion {
    /// Configuration.
    pub config: GanglionConfig,
    /// The Ollama client (real or mock).
    client: Box<dyn OllamaClient>,
    /// Consecutive Tier 1 failure counter per entity.
    tier1_fail_counts: std::collections::HashMap<EntityId, usize>,
    /// Cached health check result — avoids blocking every tick when Ollama is down.
    /// `None` means not yet checked; `Some((result, checked_at_ms))` is the cache.
    health_cache: Option<(bool, u64)>,
    /// How long (ms) to honour a cached health result before re-checking.
    health_cache_ttl_ms: u64,
}

impl Ganglion {
    /// Create a Ganglion, auto-selecting the backend:
    ///
    /// 1. If `CLAUDE_API_KEY` is set → use [`ClaudeGanglionClient`] (cheapest
    ///    Claude model, no Ollama required).
    /// 2. Otherwise → use [`ReqwestOllamaClient`] pointing at `config.base_url`.
    ///
    /// The Claude backend is preferred when available because it is always
    /// reachable, whereas Ollama requires a local install.  Switch back to
    /// Ollama by unsetting `CLAUDE_API_KEY` and setting `OLLAMA_BASE_URL`.
    pub fn new(config: GanglionConfig) -> Self {
        let client: Box<dyn OllamaClient> = if let Some(c) = ClaudeGanglionClient::from_env() {
            Box::new(c)
        } else {
            Box::new(ReqwestOllamaClient::new(config.base_url.clone(), config.timeout_secs))
        };
        Self {
            config,
            client,
            tier1_fail_counts: std::collections::HashMap::new(),
            health_cache: None,
            health_cache_ttl_ms: 60_000,
        }
    }

    /// Create a Ganglion with a custom (mock) client.  Used in tests.
    pub fn with_client(config: GanglionConfig, client: Box<dyn OllamaClient>) -> Self {
        Self {
            config,
            client,
            tier1_fail_counts: std::collections::HashMap::new(),
            health_cache: None,
            health_cache_ttl_ms: 60_000,
        }
    }

    /// Record that Tier 1 produced zero proposals for `entity_id`.
    ///
    /// Returns `true` when the fail count reaches `tier1_fail_threshold`,
    /// indicating Ganglion should fire.
    pub fn record_tier1_miss(&mut self, entity_id: &EntityId) -> bool {
        let count = self.tier1_fail_counts.entry(entity_id.clone()).or_insert(0);
        *count += 1;
        *count >= self.config.tier1_fail_threshold
    }

    /// Reset the Tier 1 miss counter when Tier 1 succeeds.
    pub fn reset_tier1_miss(&mut self, entity_id: &EntityId) {
        self.tier1_fail_counts.remove(entity_id);
    }

    /// Current Tier 1 miss count for an entity.
    pub fn tier1_miss_count(&self, entity_id: &EntityId) -> usize {
        *self.tier1_fail_counts.get(entity_id).unwrap_or(&0)
    }

    /// Evaluate a drift event: build corpus, call Ollama, parse proposals.
    ///
    /// Returns an empty vec if the Ollama service is unreachable or returns
    /// an unparseable response.
    pub fn evaluate(
        &mut self,
        event: &DriftEvent,
        store: &SignalStore,
    ) -> Vec<MutationProposal> {
        if !self.cached_health_check() {
            return vec![];
        }

        let corpus =
            serialize_corpus(&event.entity_id, event, store, self.config.corpus_lookback);
        let prompt = build_prompt(&event.entity_id, &corpus);

        match self.client.generate(&self.config.model, &prompt) {
            Ok(text) => parse_proposals(&text),
            Err(_) => vec![],
        }
    }

    /// Evaluate using a pre-built corpus string (avoids double-borrow of `store`).
    ///
    /// The caller is responsible for building the corpus via [`serialize_corpus`].
    pub fn evaluate_with_corpus(
        &mut self,
        event: &DriftEvent,
        corpus: &str,
    ) -> Vec<MutationProposal> {
        if !self.cached_health_check() {
            return vec![];
        }

        let prompt = build_prompt(&event.entity_id, corpus);

        match self.client.generate(&self.config.model, &prompt) {
            Ok(text) => parse_proposals(&text),
            Err(_) => vec![],
        }
    }

    /// Check Ollama health, using a cached result to avoid blocking every tick.
    ///
    /// When Ollama is unreachable, the first check blocks for `timeout_secs` but
    /// subsequent calls within `health_cache_ttl_ms` return immediately.
    fn cached_health_check(&mut self) -> bool {
        let now = crate::runtime::signal::now_ms();
        if let Some((result, checked_at)) = self.health_cache {
            if now.saturating_sub(checked_at) < self.health_cache_ttl_ms {
                return result;
            }
        }
        let result = self.client.health_check();
        self.health_cache = Some((result, now));
        result
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

    fn make_event(entity_id: &str, metric: &str, score: f64) -> DriftEvent {
        DriftEvent {
            entity_id: entity_id.into(),
            triggering_metric: metric.into(),
            score,
            ts: 1_000_000,
        }
    }

    // ── Mock client ───────────────────────────────────────────────────────────

    struct MockOllamaClient {
        response: String,
        healthy: bool,
    }

    impl OllamaClient for MockOllamaClient {
        fn generate(&self, _model: &str, _prompt: &str) -> Result<String, String> {
            if self.healthy {
                Ok(self.response.clone())
            } else {
                Err("service unavailable".into())
            }
        }
        fn health_check(&self) -> bool {
            self.healthy
        }
    }

    fn ganglion_with_response(response: &str) -> Ganglion {
        Ganglion::with_client(
            GanglionConfig::default(),
            Box::new(MockOllamaClient {
                response: response.into(),
                healthy: true,
            }),
        )
    }

    fn ganglion_unreachable() -> Ganglion {
        Ganglion::with_client(
            GanglionConfig::default(),
            Box::new(MockOllamaClient {
                response: String::new(),
                healthy: false,
            }),
        )
    }

    // ── parse_proposals ───────────────────────────────────────────────────────

    #[test]
    fn parse_proposals_extracts_parameter_adjust_from_json_array() {
        let text = r#"[{"kind":"parameter_adjust","entity_id":"climate","param":"albedo","delta":-0.02,"reason":"reduce drift"}]"#;
        let proposals = parse_proposals(text);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], MutationProposal::ParameterAdjust { .. }));
    }

    #[test]
    fn parse_proposals_tolerates_surrounding_prose() {
        let text =
            "Sure, here are my proposals:\n[{\"kind\":\"entity_prune\",\"entity_id\":\"x\",\"reason\":\"gone\"}]\nDone.";
        let proposals = parse_proposals(text);
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn parse_proposals_returns_empty_on_invalid_json() {
        let text = "I cannot propose anything specific.";
        let proposals = parse_proposals(text);
        assert!(proposals.is_empty());
    }

    // ── serialize_corpus ──────────────────────────────────────────────────────

    #[test]
    fn serialize_corpus_includes_entity_id_and_metric() {
        let store = mem_store();
        let event = make_event("climate_1", "temperature", 0.8);
        let corpus = serialize_corpus("climate_1", &event, &store, 5);
        assert!(corpus.contains("climate_1"));
        assert!(corpus.contains("temperature"));
        assert!(corpus.contains("0.800"));
    }

    #[test]
    fn serialize_corpus_includes_bounds_when_set() {
        let store = mem_store();
        store.register_entity("e1", "E", "{}", 0).unwrap();
        store.set_telos_bounds("e1", "temp", Some(0.0), Some(4.0), Some(2.0)).unwrap();
        let event = make_event("e1", "temp", 0.9);
        let corpus = serialize_corpus("e1", &event, &store, 5);
        assert!(corpus.contains("temp"));
        assert!(corpus.contains("Some(4.0)") || corpus.contains("4.0"));
    }

    // ── Ganglion::evaluate ────────────────────────────────────────────────────

    #[test]
    fn ganglion_returns_proposals_from_mock_client() {
        let store = mem_store();
        store.register_entity("climate", "ClimateModel", "{}", 0).unwrap();

        let mut ganglion = ganglion_with_response(
            r#"[{"kind":"parameter_adjust","entity_id":"climate","param":"albedo","delta":-0.01,"reason":"test"}]"#,
        );
        let event = make_event("climate", "temperature", 0.8);
        let proposals = ganglion.evaluate(&event, &store);
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn ganglion_returns_empty_when_unreachable() {
        let store = mem_store();
        let mut ganglion = ganglion_unreachable();
        let event = make_event("climate", "temperature", 0.9);
        let proposals = ganglion.evaluate(&event, &store);
        assert!(proposals.is_empty());
    }

    // ── Escalation counter ────────────────────────────────────────────────────

    #[test]
    fn escalation_triggers_after_threshold_misses() {
        let mut ganglion = ganglion_with_response("[]");
        ganglion.config.tier1_fail_threshold = 3;
        let id: EntityId = "e1".into();
        assert!(!ganglion.record_tier1_miss(&id));
        assert!(!ganglion.record_tier1_miss(&id));
        let triggered = ganglion.record_tier1_miss(&id);
        assert!(triggered, "should escalate at threshold 3");
    }

    #[test]
    fn reset_clears_miss_counter() {
        let mut ganglion = ganglion_with_response("[]");
        let id: EntityId = "e2".into();
        ganglion.record_tier1_miss(&id);
        ganglion.record_tier1_miss(&id);
        ganglion.reset_tier1_miss(&id);
        assert_eq!(ganglion.tier1_miss_count(&id), 0);
    }

    // ── build_prompt ──────────────────────────────────────────────────────────

    #[test]
    fn build_prompt_includes_entity_id_and_corpus() {
        let corpus = "# Entity: planet_earth\n# Drift: temp=0.9";
        let prompt = build_prompt(&"planet_earth".into(), corpus);
        assert!(prompt.contains("planet_earth"));
        assert!(prompt.contains("parameter_adjust"));
        assert!(prompt.contains("JSON array"));
    }
}
