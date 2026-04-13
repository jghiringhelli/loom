//! Circadian — cross-cutting C axis of the CEMS runtime.
//!
//! The Circadian layer gates signal processing and mutation scheduling based on
//! temporal patterns, mirroring biological circadian rhythms. In living organisms,
//! the same environmental signal produces different responses depending on the
//! time of day — immune activity peaks at night, metabolic rate varies with the
//! solar cycle, etc.
//!
//! # Responsibilities
//!
//! 1. **Temporal gating**: allow or suppress signals/mutations based on cron-like
//!    schedule expressions. A mutation proposed at 3 AM (low traffic) should be
//!    treated differently than one at peak load.
//!
//! 2. **SNR gate with Kalman pre-filter**: suppress low-signal-to-noise telemetry
//!    before it reaches the drift engine. A Kalman filter tracks the running mean
//!    and variance of each metric; signals within the noise band are gated out.
//!
//! # Schedule expressions
//!
//! Schedules use a cron-inspired syntax with five positional fields:
//!
//! ```text
//! minute  hour  day-of-month  month  day-of-week
//! 0-59    0-23  1-31          1-12   0-6 (0=Sun)
//! ```
//!
//! Each field accepts:
//! - `*` — any value
//! - `N` — exact value
//! - `N-M` — inclusive range
//! - `*/N` — step (every N units)
//! - `N,M,...` — list of values
//!
//! # Example
//!
//! ```rust,ignore
//! let c = Circadian::new();
//! // Allow mutations only outside business hours (before 9am or after 5pm, any weekday)
//! c.add_gate("off_hours", "* 0-8,17-23 * * 1-5", CircadianAction::Allow);
//! // Suppress all mutation proposals on Sunday
//! c.add_gate("no_sunday", "* * * * 0", CircadianAction::Suppress);
//!
//! let now: DateTime = ...; // current wall time
//! assert_eq!(c.evaluate("my_entity", &now), CircadianVerdict::Allow);
//! ```
//!
//! See [`ADR-0011`](../../docs/adrs/ADR-0011-ceks-runtime-architecture.md) §C-axis.

use std::collections::HashMap;

use crate::runtime::signal::{EntityId, MetricName, Timestamp};

// ── Cron field ────────────────────────────────────────────────────────────────

/// Parsed representation of a single cron field.
#[derive(Debug, Clone, PartialEq)]
enum CronField {
    /// `*` — matches any value.
    Any,
    /// `N` — exact match.
    Exact(u32),
    /// `N-M` — inclusive range.
    Range(u32, u32),
    /// `*/N` — step from 0.
    Step(u32),
    /// `N,M,...` — explicit list.
    List(Vec<u32>),
}

impl CronField {
    fn parse(s: &str) -> Result<Self, String> {
        if s == "*" {
            return Ok(CronField::Any);
        }
        if let Some(step) = s.strip_prefix("*/") {
            let n: u32 = step.parse().map_err(|_| format!("invalid step: {s}"))?;
            return Ok(CronField::Step(n));
        }
        if s.contains(',') {
            let values: Result<Vec<u32>, _> = s.split(',').map(|v| v.parse::<u32>()).collect();
            return values
                .map(CronField::List)
                .map_err(|_| format!("invalid list: {s}"));
        }
        if let Some((lo, hi)) = s.split_once('-') {
            let lo: u32 = lo.parse().map_err(|_| format!("invalid range lo: {s}"))?;
            let hi: u32 = hi.parse().map_err(|_| format!("invalid range hi: {s}"))?;
            return Ok(CronField::Range(lo, hi));
        }
        s.parse::<u32>()
            .map(CronField::Exact)
            .map_err(|_| format!("invalid cron field: {s}"))
    }

    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Exact(n) => *n == value,
            CronField::Range(lo, hi) => value >= *lo && value <= *hi,
            CronField::Step(n) => *n > 0 && value % n == 0,
            CronField::List(vs) => vs.contains(&value),
        }
    }
}

// ── Schedule ──────────────────────────────────────────────────────────────────

/// A parsed five-field cron schedule: `minute hour dom month dow`.
#[derive(Debug, Clone)]
pub struct Schedule {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
    raw: String,
}

impl Schedule {
    /// Parse a five-field cron expression.
    ///
    /// Returns `Err` with a description of the parsing failure.
    pub fn parse(expr: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!(
                "expected 5 cron fields, got {}: '{expr}'",
                parts.len()
            ));
        }
        Ok(Self {
            minute: CronField::parse(parts[0])?,
            hour: CronField::parse(parts[1])?,
            day_of_month: CronField::parse(parts[2])?,
            month: CronField::parse(parts[3])?,
            day_of_week: CronField::parse(parts[4])?,
            raw: expr.to_string(),
        })
    }

    /// Return `true` if the schedule matches the given [`WallTime`].
    pub fn matches(&self, t: &WallTime) -> bool {
        self.minute.matches(t.minute)
            && self.hour.matches(t.hour)
            && self.day_of_month.matches(t.day_of_month)
            && self.month.matches(t.month)
            && self.day_of_week.matches(t.day_of_week)
    }

    /// The original expression string.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

// ── WallTime ──────────────────────────────────────────────────────────────────

/// Decomposed wall-clock time for cron matching.
///
/// All fields use human-readable ranges (months 1–12, days 1–31).
/// `day_of_week` is 0 (Sunday) through 6 (Saturday).
#[derive(Debug, Clone, PartialEq)]
pub struct WallTime {
    pub minute: u32,
    pub hour: u32,
    pub day_of_month: u32,
    pub month: u32,
    /// 0 = Sunday, 1 = Monday … 6 = Saturday
    pub day_of_week: u32,
}

impl WallTime {
    /// Decompose a Unix timestamp (milliseconds) into wall-clock fields (UTC).
    ///
    /// Uses a simple Gregorian calendar computation — no external dependency.
    pub fn from_unix_ms(ms: Timestamp) -> Self {
        let secs = ms / 1_000;
        let minute = ((secs % 3_600) / 60) as u32;
        let hour = ((secs % 86_400) / 3_600) as u32;

        // Days since Unix epoch (1970-01-01, Thursday = day_of_week 4).
        let days = secs / 86_400;
        let day_of_week = ((days + 4) % 7) as u32; // 0=Sun

        // Gregorian date from days since epoch.
        let (year, month, day) = days_to_ymd(days);
        let _ = year;

        Self { minute, hour, day_of_month: day, month, day_of_week }
    }
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Algorithm: proleptic Gregorian calendar using the civil calendar epoch mapping.
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    // Shift epoch to 1 Mar 2000 for simpler leap-year arithmetic.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month prime [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

// ── Circadian gate ────────────────────────────────────────────────────────────

/// Action that a matched gate applies.
#[derive(Debug, Clone, PartialEq)]
pub enum CircadianAction {
    /// Signals and mutations matching this gate are allowed to proceed.
    Allow,
    /// Signals and mutations matching this gate are suppressed (not propagated).
    Suppress,
    /// Signals are admitted but mutations are deferred (queued for later).
    DeferMutations,
}

/// A named temporal gate entry.
#[derive(Debug, Clone)]
pub struct CircadianGate {
    pub name: String,
    pub schedule: Schedule,
    pub action: CircadianAction,
    /// Optional scope: entity id or `None` = applies to all entities.
    pub entity_scope: Option<EntityId>,
}

/// Verdict returned by [`Circadian::evaluate`].
#[derive(Debug, Clone, PartialEq)]
pub enum CircadianVerdict {
    /// No gate matched — allow with full pipeline.
    Allow,
    /// A `Suppress` gate matched — drop the signal/mutation silently.
    Suppress,
    /// A `DeferMutations` gate matched — admit signal, defer any proposed mutations.
    DeferMutations,
}

// ── Kalman pre-filter ─────────────────────────────────────────────────────────

/// One-dimensional Kalman filter tracking a metric's running estimate.
///
/// Used as the SNR gate: signals within `snr_threshold` standard deviations of the
/// running estimate are considered noise and suppressed.
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    /// Current state estimate (mean).
    pub estimate: f64,
    /// Estimate error covariance.
    pub error_covariance: f64,
    /// Process noise (how much the true value can change per step).
    process_noise: f64,
    /// Observation noise variance.
    observation_noise: f64,
    /// Total observations ingested. The first observation bootstraps the filter
    /// and is never classified as noise — we have no prior to compare against.
    observation_count: u64,
}

impl KalmanFilter {
    /// Create a new filter.
    ///
    /// - `initial_estimate`: initial guess for the metric value
    /// - `process_noise`: Q — expected variance of natural drift per observation
    /// - `observation_noise`: R — sensor/measurement variance
    pub fn new(initial_estimate: f64, process_noise: f64, observation_noise: f64) -> Self {
        Self {
            estimate: initial_estimate,
            error_covariance: 1.0,
            process_noise,
            observation_noise,
            observation_count: 0,
        }
    }

    /// Update with a new observation. Returns the updated estimate.
    pub fn update(&mut self, observation: f64) -> f64 {
        // Predict.
        let predicted_covariance = self.error_covariance + self.process_noise;

        // Update (Kalman gain).
        let gain = predicted_covariance / (predicted_covariance + self.observation_noise);
        self.estimate += gain * (observation - self.estimate);
        self.error_covariance = (1.0 - gain) * predicted_covariance;
        self.observation_count += 1;

        self.estimate
    }

    /// Returns `true` if `observation` is within `threshold` standard deviations of the
    /// current estimate. The first observation always returns `false` — it bootstraps
    /// the filter and there is no prior distribution to compare against.
    pub fn is_noise(&self, observation: f64, threshold: f64) -> bool {
        if self.observation_count == 0 {
            return false;
        }
        let std_dev = self.error_covariance.sqrt();
        (observation - self.estimate).abs() < threshold * std_dev
    }
}

// ── Circadian ─────────────────────────────────────────────────────────────────

/// The Circadian layer — temporal gating and SNR pre-filtering.
///
/// Lives on [`Runtime`](super::Runtime) as the `circadian` field.
/// The orchestration loop calls [`Circadian::evaluate`] before passing each
/// signal to the drift engine, and [`Circadian::evaluate_mutation`] before
/// passing each proposal to the gate stage.
pub struct Circadian {
    gates: Vec<CircadianGate>,
    /// SNR Kalman filters per (entity_id, metric) pair.
    kalman: HashMap<(EntityId, MetricName), KalmanFilter>,
    /// Default SNR threshold (number of std deviations to consider noise).
    pub snr_threshold: f64,
    /// Default Kalman process noise (Q).
    pub process_noise: f64,
    /// Default Kalman observation noise (R).
    pub observation_noise: f64,
}

impl Circadian {
    /// Create a new Circadian layer with sensible defaults.
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            kalman: HashMap::new(),
            snr_threshold: 1.5,
            process_noise: 0.01,
            observation_noise: 0.1,
        }
    }

    /// Register a temporal gate.
    ///
    /// Gates are evaluated in order — first match wins.
    pub fn add_gate(
        &mut self,
        name: impl Into<String>,
        schedule_expr: &str,
        action: CircadianAction,
        entity_scope: Option<EntityId>,
    ) -> Result<(), String> {
        let schedule = Schedule::parse(schedule_expr)?;
        self.gates.push(CircadianGate {
            name: name.into(),
            schedule,
            action,
            entity_scope,
        });
        Ok(())
    }

    /// Remove a gate by name. Returns `true` if it existed.
    pub fn remove_gate(&mut self, name: &str) -> bool {
        let before = self.gates.len();
        self.gates.retain(|g| g.name != name);
        self.gates.len() < before
    }

    /// Evaluate the current wall time against all registered gates for an entity.
    ///
    /// Returns the first matching verdict, or `Allow` when no gate matches.
    pub fn evaluate(&self, entity_id: &str, now: &WallTime) -> CircadianVerdict {
        for gate in &self.gates {
            if let Some(scope) = &gate.entity_scope {
                if scope != entity_id {
                    continue;
                }
            }
            if gate.schedule.matches(now) {
                return match gate.action {
                    CircadianAction::Allow => CircadianVerdict::Allow,
                    CircadianAction::Suppress => CircadianVerdict::Suppress,
                    CircadianAction::DeferMutations => CircadianVerdict::DeferMutations,
                };
            }
        }
        CircadianVerdict::Allow
    }

    /// Run the Kalman SNR gate for a signal value.
    ///
    /// Returns `true` when the signal is within the noise band and should be suppressed.
    /// Updates the Kalman filter for the `(entity_id, metric)` pair.
    pub fn is_noise_signal(
        &mut self,
        entity_id: &str,
        metric: &str,
        value: f64,
    ) -> bool {
        let filter = self
            .kalman
            .entry((entity_id.to_string(), metric.to_string()))
            .or_insert_with(|| {
                KalmanFilter::new(value, self.process_noise, self.observation_noise)
            });
        let is_noise = filter.is_noise(value, self.snr_threshold);
        filter.update(value);
        is_noise
    }

    /// Kalman estimate for a metric on an entity, if available.
    pub fn kalman_estimate(&self, entity_id: &str, metric: &str) -> Option<f64> {
        self.kalman
            .get(&(entity_id.to_string(), metric.to_string()))
            .map(|f| f.estimate)
    }

    /// All registered gates (read-only).
    pub fn gates(&self) -> &[CircadianGate] {
        &self.gates
    }
}

impl Default for Circadian {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(min: u32, hour: u32, dom: u32, month: u32, dow: u32) -> WallTime {
        WallTime { minute: min, hour, day_of_month: dom, month, day_of_week: dow }
    }

    // ── Schedule parsing ──────────────────────────────────────────────────────

    #[test]
    fn schedule_any_matches_all_values() {
        let s = Schedule::parse("* * * * *").unwrap();
        assert!(s.matches(&wall(0, 0, 1, 1, 0)));
        assert!(s.matches(&wall(59, 23, 31, 12, 6)));
    }

    #[test]
    fn schedule_exact_matches_only_that_value() {
        let s = Schedule::parse("30 14 * * *").unwrap();
        assert!(s.matches(&wall(30, 14, 15, 6, 3)));
        assert!(!s.matches(&wall(29, 14, 15, 6, 3)));
        assert!(!s.matches(&wall(30, 13, 15, 6, 3)));
    }

    #[test]
    fn schedule_range_matches_inclusive() {
        let s = Schedule::parse("* 9-17 * * *").unwrap();
        assert!(s.matches(&wall(0, 9, 1, 1, 0)));
        assert!(s.matches(&wall(0, 17, 1, 1, 0)));
        assert!(!s.matches(&wall(0, 8, 1, 1, 0)));
        assert!(!s.matches(&wall(0, 18, 1, 1, 0)));
    }

    #[test]
    fn schedule_step_matches_multiples() {
        let s = Schedule::parse("*/15 * * * *").unwrap();
        assert!(s.matches(&wall(0, 12, 1, 1, 0)));
        assert!(s.matches(&wall(15, 12, 1, 1, 0)));
        assert!(s.matches(&wall(30, 12, 1, 1, 0)));
        assert!(s.matches(&wall(45, 12, 1, 1, 0)));
        assert!(!s.matches(&wall(10, 12, 1, 1, 0)));
    }

    #[test]
    fn schedule_list_matches_any_in_list() {
        let s = Schedule::parse("* * * * 1,3,5").unwrap();
        assert!(s.matches(&wall(0, 0, 1, 1, 1))); // Mon
        assert!(s.matches(&wall(0, 0, 1, 1, 3))); // Wed
        assert!(s.matches(&wall(0, 0, 1, 1, 5))); // Fri
        assert!(!s.matches(&wall(0, 0, 1, 1, 0))); // Sun
        assert!(!s.matches(&wall(0, 0, 1, 1, 6))); // Sat
    }

    #[test]
    fn schedule_parse_error_on_wrong_field_count() {
        assert!(Schedule::parse("* * *").is_err());
        assert!(Schedule::parse("* * * * * *").is_err());
    }

    #[test]
    fn schedule_parse_error_on_invalid_value() {
        assert!(Schedule::parse("abc * * * *").is_err());
    }

    // ── WallTime from Unix ms ─────────────────────────────────────────────────

    #[test]
    fn wall_time_from_unix_ms_epoch_is_thursday() {
        // Unix epoch = 1970-01-01 00:00:00 UTC = Thursday (4)
        let t = WallTime::from_unix_ms(0);
        assert_eq!(t.minute, 0);
        assert_eq!(t.hour, 0);
        assert_eq!(t.day_of_month, 1);
        assert_eq!(t.month, 1);
        assert_eq!(t.day_of_week, 4); // Thursday
    }

    #[test]
    fn wall_time_from_unix_ms_known_date() {
        // 2024-03-14 15:09:26 UTC = Pi Day
        // 1710428966 seconds
        let ms: Timestamp = 1_710_428_966_000;
        let t = WallTime::from_unix_ms(ms);
        assert_eq!(t.hour, 15);
        assert_eq!(t.minute, 9);
        assert_eq!(t.month, 3);
        assert_eq!(t.day_of_month, 14);
        // 2024-03-14 is a Thursday
        assert_eq!(t.day_of_week, 4);
    }

    // ── Circadian gating ──────────────────────────────────────────────────────

    #[test]
    fn no_gates_always_allows() {
        let c = Circadian::new();
        assert_eq!(c.evaluate("e1", &wall(0, 3, 1, 1, 0)), CircadianVerdict::Allow);
    }

    #[test]
    fn suppress_gate_matches_midnight_hour() {
        let mut c = Circadian::new();
        c.add_gate("quiet", "* 0-5 * * *", CircadianAction::Suppress, None).unwrap();
        assert_eq!(c.evaluate("e1", &wall(30, 3, 1, 1, 0)), CircadianVerdict::Suppress);
        assert_eq!(c.evaluate("e1", &wall(0, 9, 1, 1, 0)), CircadianVerdict::Allow);
    }

    #[test]
    fn defer_mutations_gate_during_peak_hours() {
        let mut c = Circadian::new();
        c.add_gate("peak", "* 9-17 * * 1-5", CircadianAction::DeferMutations, None).unwrap();
        assert_eq!(
            c.evaluate("e1", &wall(0, 12, 10, 6, 3)),
            CircadianVerdict::DeferMutations
        );
        assert_eq!(
            c.evaluate("e1", &wall(0, 12, 10, 6, 0)), // Sunday
            CircadianVerdict::Allow
        );
    }

    #[test]
    fn entity_scoped_gate_ignores_other_entities() {
        let mut c = Circadian::new();
        c.add_gate(
            "e2_only",
            "* * * * *",
            CircadianAction::Suppress,
            Some("e2".to_string()),
        )
        .unwrap();
        assert_eq!(c.evaluate("e1", &wall(0, 0, 1, 1, 0)), CircadianVerdict::Allow);
        assert_eq!(c.evaluate("e2", &wall(0, 0, 1, 1, 0)), CircadianVerdict::Suppress);
    }

    #[test]
    fn first_matching_gate_wins() {
        let mut c = Circadian::new();
        c.add_gate("allow", "* * * * *", CircadianAction::Allow, None).unwrap();
        c.add_gate("suppress", "* * * * *", CircadianAction::Suppress, None).unwrap();
        assert_eq!(c.evaluate("e1", &wall(0, 0, 1, 1, 0)), CircadianVerdict::Allow);
    }

    #[test]
    fn remove_gate_stops_matching() {
        let mut c = Circadian::new();
        c.add_gate("block", "* * * * *", CircadianAction::Suppress, None).unwrap();
        assert_eq!(c.evaluate("e1", &wall(0, 0, 1, 1, 0)), CircadianVerdict::Suppress);
        assert!(c.remove_gate("block"));
        assert_eq!(c.evaluate("e1", &wall(0, 0, 1, 1, 0)), CircadianVerdict::Allow);
    }

    // ── Kalman SNR gate ───────────────────────────────────────────────────────

    #[test]
    fn kalman_first_observation_never_noise() {
        let mut c = Circadian::new();
        // First observation bootstraps the filter — not noise by definition.
        assert!(!c.is_noise_signal("e1", "cpu", 0.5));
    }

    #[test]
    fn kalman_small_deviation_is_noise() {
        let mut c = Circadian::new();
        c.snr_threshold = 2.0;
        // Warm up the filter with stable observations.
        for _ in 0..20 {
            c.is_noise_signal("e1", "cpu", 1.0);
        }
        // A tiny deviation should be within 2 standard deviations.
        assert!(c.is_noise_signal("e1", "cpu", 1.001));
    }

    #[test]
    fn kalman_large_deviation_is_not_noise() {
        let mut c = Circadian::new();
        c.snr_threshold = 2.0;
        for _ in 0..20 {
            c.is_noise_signal("e1", "cpu", 1.0);
        }
        // A large spike is clearly signal.
        assert!(!c.is_noise_signal("e1", "cpu", 100.0));
    }

    #[test]
    fn kalman_estimate_converges_toward_observations() {
        let mut c = Circadian::new();
        for _ in 0..50 {
            c.is_noise_signal("e1", "temp", 25.0);
        }
        let est = c.kalman_estimate("e1", "temp").unwrap();
        assert!((est - 25.0).abs() < 0.5);
    }

    #[test]
    fn kalman_different_metrics_tracked_independently() {
        let mut c = Circadian::new();
        for _ in 0..10 {
            c.is_noise_signal("e1", "cpu", 0.3);
            c.is_noise_signal("e1", "mem", 0.9);
        }
        let cpu = c.kalman_estimate("e1", "cpu").unwrap();
        let mem = c.kalman_estimate("e1", "mem").unwrap();
        assert!((cpu - 0.3).abs() < 0.1);
        assert!((mem - 0.9).abs() < 0.1);
    }
}
