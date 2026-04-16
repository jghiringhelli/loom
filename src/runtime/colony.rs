//! Mycelium — cross-cutting M axis of the CEMS runtime.
//!
//! The Mycelium layer handles colony coordination between Loom entity instances,
//! mirroring the role of mycorrhizal fungal networks in forest ecosystems: invisible
//! infrastructure that allows otherwise isolated organisms to share nutrients (signals),
//! warning chemicals (security events), and learned adaptations (Core memories).
//!
//! # Responsibilities
//!
//! 1. **Gossip protocol**: periodic push of epigenome Core snapshots to known peers
//!    via HTTP. Drives convergence of institutional memory across the colony.
//!
//! 2. **Stigmergy (ACO-inspired)**: pheromone trail markers on strategies that
//!    succeeded. Each colony member reads pheromone strengths before selecting
//!    mutation strategies. Trails evaporate over time (simulated annealing variant).
//!
//! 3. **Offline cache**: when a peer is unreachable, outbound messages are queued
//!    locally. On reconnect the queue is flushed before normal gossip resumes.
//!
//! 4. **Hibernation**: when metabolic load (signal rate) drops below a threshold,
//!    the colony member reduces gossip frequency to save resources.
//!
//! 5. **Catch-up resync**: on reconnect, pull the peer's current snapshot to fill
//!    the gap created by the offline period.
//!
//! # Stigmergy model
//!
//! Each `(strategy_key, colony_id)` pair accumulates a pheromone strength in [0, 1].
//! A successful mutation deposits pheromone (additive, capped at 1.0).
//! Every tick, all trails evaporate by a configurable `evaporation_rate`.
//! The Reflex stage reads pheromone strengths as a prior when selecting from
//! multiple equally-scoring mutation proposals.
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md) §M-axis.

use std::collections::{HashMap, VecDeque};

use crate::runtime::signal::Timestamp;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default pheromone evaporation rate per tick (fraction of current strength removed).
pub const DEFAULT_EVAPORATION_RATE: f64 = 0.05;

/// Maximum pheromone strength (saturates at this value).
pub const MAX_PHEROMONE: f64 = 1.0;

/// Default offline queue capacity per peer (oldest dropped when exceeded).
pub const OFFLINE_QUEUE_CAPACITY: usize = 1_000;

/// Default hibernation threshold: signals per tick below this → hibernate.
pub const DEFAULT_HIBERNATION_THRESHOLD: f64 = 1.0;

// ── Peer state ────────────────────────────────────────────────────────────────

/// Connectivity state of a colony peer.
#[derive(Debug, Clone, PartialEq)]
pub enum PeerStatus {
    /// Peer is reachable and actively exchanging gossip.
    Online,
    /// Peer is unreachable; messages are queued in the offline cache.
    Offline,
    /// Peer has been silent longer than the hibernation window.
    Hibernating,
}

/// A registered colony peer.
#[derive(Debug, Clone)]
pub struct ColonyPeer {
    /// Unique identifier for this peer (typically entity_id of the remote Runtime).
    pub peer_id: String,
    /// HTTP base URL for gossip push (e.g., `"http://192.168.1.10:8080"`).
    pub base_url: String,
    /// Current connectivity status.
    pub status: PeerStatus,
    /// Timestamp of the last successful gossip exchange.
    pub last_seen_ms: Timestamp,
    /// Queued outbound gossip payloads (used when offline).
    pub offline_queue: VecDeque<GossipMessage>,
}

impl ColonyPeer {
    fn new(peer_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            peer_id: peer_id.into(),
            base_url: base_url.into(),
            status: PeerStatus::Online,
            last_seen_ms: 0,
            offline_queue: VecDeque::with_capacity(OFFLINE_QUEUE_CAPACITY),
        }
    }

    /// Enqueue a message for when the peer comes back online.
    ///
    /// Evicts the oldest entry when the queue is full.
    pub fn enqueue_offline(&mut self, msg: GossipMessage) {
        if self.offline_queue.len() >= OFFLINE_QUEUE_CAPACITY {
            self.offline_queue.pop_front();
        }
        self.offline_queue.push_back(msg);
    }

    /// Drain the offline queue, returning all queued messages in order.
    pub fn drain_offline_queue(&mut self) -> Vec<GossipMessage> {
        self.offline_queue.drain(..).collect()
    }
}

// ── Gossip message ────────────────────────────────────────────────────────────

/// A gossip payload sent between colony peers.
///
/// Contains a snapshot of Core memories that the sender wants to propagate.
/// The receiver merges these into its own Epigenome using
/// [`Mycelium::merge_gossip`].
#[derive(Debug, Clone)]
pub struct GossipMessage {
    /// Entity that originated the gossip.
    pub sender_id: String,
    /// Serialised Core memory entries (JSON array of `CoreEntry`-compatible objects).
    pub core_snapshot: String,
    /// Wall-clock timestamp of the snapshot.
    pub ts: Timestamp,
}

// ── Stigmergy ─────────────────────────────────────────────────────────────────

/// A pheromone trail entry for a (strategy, colony) pair.
#[derive(Debug, Clone)]
pub struct PheromoneTrail {
    /// Strategy key — identifies the mutation strategy (e.g., `"reduce_batch_size"`).
    pub strategy_key: String,
    /// Current pheromone strength [0.0, 1.0].
    pub strength: f64,
    /// Timestamp of the last deposit.
    pub last_deposit_ms: Timestamp,
    /// Total successful deposits (for diagnostics).
    pub deposit_count: u64,
}

// ── Hibernation state ─────────────────────────────────────────────────────────

/// Metabolic load tracker for hibernation decisions.
#[derive(Debug, Clone)]
pub struct MetabolicLoad {
    /// Moving average of signals per tick.
    pub signals_per_tick: f64,
    /// Timestamp of the last load sample.
    pub last_sample_ms: Timestamp,
    /// Count of signals seen in the current tick window.
    tick_count: u64,
}

impl MetabolicLoad {
    fn new() -> Self {
        Self {
            signals_per_tick: 0.0,
            last_sample_ms: 0,
            tick_count: 0,
        }
    }

    /// Record a signal observation.
    pub fn record_signal(&mut self) {
        self.tick_count += 1;
    }

    /// Advance the tick and update the moving average.
    ///
    /// Uses exponential moving average with `alpha = 0.2`.
    pub fn tick(&mut self, now: Timestamp) {
        const ALPHA: f64 = 0.2;
        self.signals_per_tick =
            ALPHA * self.tick_count as f64 + (1.0 - ALPHA) * self.signals_per_tick;
        self.tick_count = 0;
        self.last_sample_ms = now;
    }
}

// ── Mycelium ──────────────────────────────────────────────────────────────────

/// The Mycelium layer — colony coordination, stigmergy, and offline resilience.
///
/// Lives on [`Runtime`](super::Runtime) as the `mycelium` field.
/// The orchestration loop calls [`Mycelium::tick`] once per orchestration cycle
/// to advance evaporation and hibernation tracking.
pub struct Mycelium {
    /// Registered peers, keyed by peer_id.
    peers: HashMap<String, ColonyPeer>,
    /// Pheromone trails, keyed by strategy_key.
    trails: HashMap<String, PheromoneTrail>,
    /// Metabolic load tracker for this colony member.
    pub load: MetabolicLoad,
    /// Evaporation rate per tick.
    pub evaporation_rate: f64,
    /// Signals-per-tick threshold below which hibernation is triggered.
    pub hibernation_threshold: f64,
    /// Whether this colony member is currently hibernating.
    pub hibernating: bool,
    /// Merged gossip entries received from peers (queue for Epigenome absorption).
    inbound_gossip: VecDeque<GossipMessage>,
}

impl Mycelium {
    /// Create a new Mycelium layer with default parameters.
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            trails: HashMap::new(),
            load: MetabolicLoad::new(),
            evaporation_rate: DEFAULT_EVAPORATION_RATE,
            hibernation_threshold: DEFAULT_HIBERNATION_THRESHOLD,
            hibernating: false,
            inbound_gossip: VecDeque::new(),
        }
    }

    // ── Peer management ───────────────────────────────────────────────────────

    /// Register a colony peer.
    ///
    /// No-op if `peer_id` is already registered.
    pub fn add_peer(&mut self, peer_id: impl Into<String>, base_url: impl Into<String>) {
        let id = peer_id.into();
        self.peers
            .entry(id.clone())
            .or_insert_with(|| ColonyPeer::new(id, base_url));
    }

    /// Remove a peer by id. Returns `true` if it existed.
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        self.peers.remove(peer_id).is_some()
    }

    /// Mark a peer as online and flush its offline queue.
    ///
    /// Returns the drained offline queue — the caller should deliver these
    /// messages to the peer before resuming normal gossip.
    pub fn peer_came_online(&mut self, peer_id: &str, now: Timestamp) -> Vec<GossipMessage> {
        match self.peers.get_mut(peer_id) {
            None => vec![],
            Some(peer) => {
                peer.status = PeerStatus::Online;
                peer.last_seen_ms = now;
                peer.drain_offline_queue()
            }
        }
    }

    /// Mark a peer as offline. Subsequent gossip to this peer will be queued.
    pub fn peer_went_offline(&mut self, peer_id: &str) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.status = PeerStatus::Offline;
        }
    }

    /// Return a view of all registered peers.
    pub fn peers(&self) -> impl Iterator<Item = &ColonyPeer> {
        self.peers.values()
    }

    /// Return the status of a specific peer.
    pub fn peer_status(&self, peer_id: &str) -> Option<&PeerStatus> {
        self.peers.get(peer_id).map(|p| &p.status)
    }

    // ── Gossip ────────────────────────────────────────────────────────────────

    /// Prepare a gossip message for broadcast to all online peers.
    ///
    /// Returns a list of `(peer_id, base_url, GossipMessage)` triples for the
    /// caller to deliver via HTTP. Offline peers receive the message in their
    /// queue instead.
    pub fn prepare_gossip(
        &mut self,
        sender_id: &str,
        core_snapshot: impl Into<String>,
        now: Timestamp,
    ) -> Vec<(String, String, GossipMessage)> {
        let msg = GossipMessage {
            sender_id: sender_id.to_string(),
            core_snapshot: core_snapshot.into(),
            ts: now,
        };
        let mut deliveries = Vec::new();
        for peer in self.peers.values_mut() {
            if peer.status == PeerStatus::Online {
                deliveries.push((peer.peer_id.clone(), peer.base_url.clone(), msg.clone()));
            } else {
                peer.enqueue_offline(msg.clone());
            }
        }
        deliveries
    }

    /// Accept an inbound gossip message from a peer.
    ///
    /// The message is queued for absorption into the Epigenome on the next tick.
    pub fn receive_gossip(&mut self, msg: GossipMessage, now: Timestamp) {
        if let Some(peer) = self.peers.get_mut(&msg.sender_id) {
            peer.last_seen_ms = now;
            peer.status = PeerStatus::Online;
        }
        self.inbound_gossip.push_back(msg);
    }

    /// Drain inbound gossip messages for Epigenome absorption.
    ///
    /// Returns all buffered inbound messages. The caller (orchestrator) passes
    /// each `core_snapshot` to `Epigenome::write_core` with `source = "mycelium"`.
    pub fn drain_inbound(&mut self) -> Vec<GossipMessage> {
        self.inbound_gossip.drain(..).collect()
    }

    // ── Stigmergy ─────────────────────────────────────────────────────────────

    /// Deposit pheromone on a strategy trail (ACO reinforcement).
    ///
    /// Call after a mutation produced a successful outcome. The `amount`
    /// is added to the current strength, capped at [`MAX_PHEROMONE`].
    pub fn deposit_pheromone(
        &mut self,
        strategy_key: impl Into<String>,
        amount: f64,
        now: Timestamp,
    ) {
        let key = strategy_key.into();
        let trail = self
            .trails
            .entry(key.clone())
            .or_insert_with(|| PheromoneTrail {
                strategy_key: key,
                strength: 0.0,
                last_deposit_ms: now,
                deposit_count: 0,
            });
        trail.strength = (trail.strength + amount).min(MAX_PHEROMONE);
        trail.last_deposit_ms = now;
        trail.deposit_count += 1;
    }

    /// Read pheromone strength for a strategy (0.0 if no trail exists).
    pub fn pheromone_strength(&self, strategy_key: &str) -> f64 {
        self.trails.get(strategy_key).map_or(0.0, |t| t.strength)
    }

    /// Return the strategy key with the highest pheromone strength among candidates.
    ///
    /// Returns `None` if `candidates` is empty.
    pub fn strongest_trail<'a>(&self, candidates: &[&'a str]) -> Option<&'a str> {
        candidates
            .iter()
            .max_by(|a, b| {
                self.pheromone_strength(a)
                    .partial_cmp(&self.pheromone_strength(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    // ── Tick ──────────────────────────────────────────────────────────────────

    /// Advance one orchestration tick.
    ///
    /// 1. Evaporates all pheromone trails by `evaporation_rate`.
    /// 2. Updates metabolic load.
    /// 3. Evaluates hibernation state.
    ///
    /// Call once per orchestration cycle.
    pub fn tick(&mut self, now: Timestamp) {
        // Evaporate trails.
        for trail in self.trails.values_mut() {
            trail.strength *= 1.0 - self.evaporation_rate;
        }
        // Remove exhausted trails (< 0.001 is effectively zero).
        self.trails.retain(|_, t| t.strength >= 0.001);

        // Update load.
        self.load.tick(now);

        // Evaluate hibernation.
        self.hibernating = self.load.signals_per_tick < self.hibernation_threshold;
    }

    /// Signal count to feed into metabolic load tracking.
    pub fn record_signal(&mut self) {
        self.load.record_signal();
    }
}

impl Default for Mycelium {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mycelium() -> Mycelium {
        Mycelium::new()
    }

    // ── Peer management ───────────────────────────────────────────────────────

    #[test]
    fn add_peer_registers_as_online() {
        let mut m = mycelium();
        m.add_peer("p1", "http://localhost:8080");
        assert_eq!(m.peer_status("p1"), Some(&PeerStatus::Online));
    }

    #[test]
    fn add_peer_is_idempotent() {
        let mut m = mycelium();
        m.add_peer("p1", "http://a");
        m.add_peer("p1", "http://b"); // second call ignored
        assert_eq!(m.peers().count(), 1);
    }

    #[test]
    fn remove_peer_returns_true_when_exists() {
        let mut m = mycelium();
        m.add_peer("p1", "http://localhost");
        assert!(m.remove_peer("p1"));
        assert_eq!(m.peer_status("p1"), None);
    }

    #[test]
    fn remove_peer_returns_false_when_absent() {
        let mut m = mycelium();
        assert!(!m.remove_peer("nobody"));
    }

    #[test]
    fn peer_went_offline_changes_status() {
        let mut m = mycelium();
        m.add_peer("p1", "http://localhost");
        m.peer_went_offline("p1");
        assert_eq!(m.peer_status("p1"), Some(&PeerStatus::Offline));
    }

    #[test]
    fn peer_came_online_flushes_offline_queue() {
        let mut m = mycelium();
        m.add_peer("p1", "http://localhost");
        m.peer_went_offline("p1");
        m.peers
            .get_mut("p1")
            .unwrap()
            .enqueue_offline(GossipMessage {
                sender_id: "self".into(),
                core_snapshot: "{}".into(),
                ts: 100,
            });
        let flushed = m.peer_came_online("p1", 200);
        assert_eq!(flushed.len(), 1);
        assert_eq!(m.peers.get("p1").unwrap().offline_queue.len(), 0);
        assert_eq!(m.peer_status("p1"), Some(&PeerStatus::Online));
    }

    // ── Gossip ────────────────────────────────────────────────────────────────

    #[test]
    fn prepare_gossip_delivers_to_online_peers() {
        let mut m = mycelium();
        m.add_peer("p1", "http://host1");
        m.add_peer("p2", "http://host2");
        let deliveries = m.prepare_gossip("self", "snapshot", 100);
        assert_eq!(deliveries.len(), 2);
    }

    #[test]
    fn prepare_gossip_queues_for_offline_peers() {
        let mut m = mycelium();
        m.add_peer("p1", "http://host1");
        m.peer_went_offline("p1");
        let deliveries = m.prepare_gossip("self", "snapshot", 100);
        assert_eq!(deliveries.len(), 0);
        assert_eq!(m.peers.get("p1").unwrap().offline_queue.len(), 1);
    }

    #[test]
    fn receive_gossip_queues_for_drain() {
        let mut m = mycelium();
        m.add_peer("sender", "http://sender");
        m.receive_gossip(
            GossipMessage {
                sender_id: "sender".into(),
                core_snapshot: "data".into(),
                ts: 10,
            },
            10,
        );
        let drained = m.drain_inbound();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].core_snapshot, "data");
    }

    #[test]
    fn receive_gossip_marks_peer_online() {
        let mut m = mycelium();
        m.add_peer("p1", "http://p1");
        m.peer_went_offline("p1");
        m.receive_gossip(
            GossipMessage {
                sender_id: "p1".into(),
                core_snapshot: "{}".into(),
                ts: 50,
            },
            50,
        );
        assert_eq!(m.peer_status("p1"), Some(&PeerStatus::Online));
    }

    #[test]
    fn offline_queue_evicts_oldest_at_capacity() {
        let mut peer = ColonyPeer::new("p", "http://p");
        for i in 0..OFFLINE_QUEUE_CAPACITY + 1 {
            peer.enqueue_offline(GossipMessage {
                sender_id: "s".into(),
                core_snapshot: format!("{i}"),
                ts: i as u64,
            });
        }
        assert_eq!(peer.offline_queue.len(), OFFLINE_QUEUE_CAPACITY);
        // oldest (snapshot "0") was evicted; front is now "1"
        assert_eq!(peer.offline_queue.front().unwrap().core_snapshot, "1");
    }

    // ── Stigmergy ─────────────────────────────────────────────────────────────

    #[test]
    fn deposit_pheromone_increases_strength() {
        let mut m = mycelium();
        m.deposit_pheromone("reduce_batch", 0.3, 100);
        assert!((m.pheromone_strength("reduce_batch") - 0.3).abs() < 1e-9);
    }

    #[test]
    fn pheromone_strength_caps_at_max() {
        let mut m = mycelium();
        m.deposit_pheromone("strategy_a", 0.8, 100);
        m.deposit_pheromone("strategy_a", 0.8, 200); // would be 1.6
        assert!((m.pheromone_strength("strategy_a") - MAX_PHEROMONE).abs() < 1e-9);
    }

    #[test]
    fn pheromone_evaporates_on_tick() {
        let mut m = mycelium();
        m.evaporation_rate = 0.5;
        m.deposit_pheromone("strat", 1.0, 0);
        m.tick(1);
        let strength = m.pheromone_strength("strat");
        assert!((strength - 0.5).abs() < 1e-9);
    }

    #[test]
    fn exhausted_trail_is_removed_on_tick() {
        let mut m = mycelium();
        m.evaporation_rate = 1.0; // full evaporation
        m.deposit_pheromone("strat", 1.0, 0);
        m.tick(1);
        assert_eq!(m.pheromone_strength("strat"), 0.0);
        assert!(!m.trails.contains_key("strat"));
    }

    #[test]
    fn strongest_trail_returns_highest_pheromone_candidate() {
        let mut m = mycelium();
        m.deposit_pheromone("a", 0.3, 0);
        m.deposit_pheromone("b", 0.7, 0);
        m.deposit_pheromone("c", 0.5, 0);
        assert_eq!(m.strongest_trail(&["a", "b", "c"]), Some("b"));
    }

    #[test]
    fn strongest_trail_returns_none_for_empty_candidates() {
        let m = mycelium();
        assert_eq!(m.strongest_trail(&[]), None);
    }

    // ── Hibernation ───────────────────────────────────────────────────────────

    #[test]
    fn hibernation_triggered_when_load_below_threshold() {
        let mut m = mycelium();
        m.hibernation_threshold = 5.0;
        // No signals recorded → signals_per_tick stays at 0
        m.tick(1_000);
        assert!(m.hibernating);
    }

    #[test]
    fn hibernation_lifted_when_load_exceeds_threshold() {
        let mut m = mycelium();
        m.hibernation_threshold = 1.0;
        // Record many signals before tick
        for _ in 0..20 {
            m.record_signal();
        }
        m.tick(1_000);
        assert!(!m.hibernating);
    }

    #[test]
    fn metabolic_load_tracks_ema_of_signal_rate() {
        let mut load = MetabolicLoad::new();
        for _ in 0..10 {
            load.record_signal();
        }
        load.tick(1_000); // alpha=0.2 → 0.2 * 10 + 0.8 * 0 = 2.0
        assert!((load.signals_per_tick - 2.0).abs() < 1e-9);
    }
}
