//! BIOISO event logger — append-only TOML event stream per entity.
//!
//! Each entity writes to `{log_dir}/{entity_id}.bioiso`.  Events are TOML
//! `[[event]]` blocks, one per occurrence, making the files human-readable,
//! `grep`-able, and trivially importable into Python/R for analysis.
//!
//! # Aggregation
//!
//! ```sh
//! # All T5 proposals across the colony:
//! grep -h 'type = "t5_proposal"' .bioiso/*.bioiso
//!
//! # All apoptosis events:
//! grep -A4 'type = "apoptosis"' .bioiso/*.bioiso
//!
//! # loom bioiso scan --dir .bioiso/
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Writes BIOISO lifecycle events to per-entity `.bioiso` files.
#[derive(Debug, Clone)]
pub struct BioisoLogger {
    pub log_dir: PathBuf,
}

impl BioisoLogger {
    pub fn new(log_dir: impl Into<PathBuf>) -> Self {
        let dir = log_dir.into();
        let _ = fs::create_dir_all(&dir);
        Self { log_dir: dir }
    }

    pub fn log_seeded(&self, entity_id: &str, tick: u64, telos: &str) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"seeded\"\nentity = {entity_id:?}\ntelos = {telos:?}\n"
            ),
        );
    }

    pub fn log_promoted(
        &self,
        entity_id: &str,
        tick: u64,
        tier: u8,
        mutation_type: &str,
        param: Option<&str>,
        delta: Option<f64>,
        drift: f64,
    ) {
        let mut block = format!(
            "\n[[event]]\ntick = {tick}\ntype = \"promoted\"\nentity = {entity_id:?}\ntier = {tier}\nmutation = {mutation_type:?}\ndrift = {drift:.4}\n"
        );
        if let (Some(p), Some(d)) = (param, delta) {
            block.push_str(&format!("param = {p:?}\ndelta = {d:.6}\n"));
        }
        self.append(entity_id, &block);
    }

    pub fn log_tier_up(
        &self,
        entity_id: &str,
        tick: u64,
        from_tier: u8,
        to_tier: u8,
        reason: &str,
    ) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"tier_up\"\nentity = {entity_id:?}\nfrom_tier = {from_tier}\nto_tier = {to_tier}\nreason = {reason:?}\n"
            ),
        );
    }

    pub fn log_t5_proposal(
        &self,
        entity_id: &str,
        tick: u64,
        mutation_type: &str,
        target: &str,
        reason: &str,
        stagnation: u32,
        accepted: bool,
    ) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"t5_proposal\"\nentity = {entity_id:?}\nmutation = {mutation_type:?}\ntarget = {target:?}\nreason = {reason:?}\nstagnation = {stagnation}\naccepted = {accepted}\n"
            ),
        );
    }

    pub fn log_senescence(&self, entity_id: &str, tick: u64, cause: &str) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"senescence\"\nentity = {entity_id:?}\ncause = {cause:?}\n"
            ),
        );
    }

    pub fn log_apoptosis(&self, entity_id: &str, tick: u64, cause: &str) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"apoptosis\"\nentity = {entity_id:?}\ncause = {cause:?}\n"
            ),
        );
    }

    pub fn log_branch(&self, parent_id: &str, child_id: &str, tick: u64, triggering_metric: &str) {
        let block = format!(
            "\n[[event]]\ntick = {tick}\ntype = \"branch\"\nentity = {child_id:?}\nparent = {parent_id:?}\ntriggering_metric = {triggering_metric:?}\n"
        );
        self.append(parent_id, &block);
        self.append(child_id, &block);
    }

    pub fn log_gate_rejected(
        &self,
        entity_id: &str,
        tick: u64,
        tier: u8,
        mutation_type: &str,
        reason: &str,
    ) {
        self.append(
            entity_id,
            &format!(
                "\n[[event]]\ntick = {tick}\ntype = \"gate_rejected\"\nentity = {entity_id:?}\ntier = {tier}\nmutation = {mutation_type:?}\nreason = {reason:?}\n"
            ),
        );
    }

    fn append(&self, entity_id: &str, toml_block: &str) {
        let safe_id = entity_id.replace(['/', '\\', ':'], "_");
        let path = self.log_dir.join(format!("{safe_id}.bioiso"));
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut f) => {
                let _ = f.write_all(toml_block.as_bytes());
            }
            Err(e) => {
                eprintln!("[bioiso_log] write error {}: {e}", path.display());
            }
        }
    }
}

// ── Scan / aggregate ─────────────────────────────────────────────────────────

/// A single parsed event from a `.bioiso` file.
#[derive(Debug, Clone)]
pub struct BioisoEvent {
    pub entity: String,
    pub tick: u64,
    pub event_type: String,
    pub extra: Vec<(String, String)>,
}

/// Read and parse all `.bioiso` files in `dir`, sorted by tick ascending.
pub fn scan_dir(dir: &Path) -> Vec<BioisoEvent> {
    let mut events = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[bioiso scan] cannot read dir {}: {e}", dir.display());
            return events;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("bioiso") {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        parse_bioiso_file(&content, &mut events);
    }

    events.sort_by_key(|e| e.tick);
    events
}

fn parse_bioiso_file(content: &str, out: &mut Vec<BioisoEvent>) {
    let mut current_tick = 0u64;
    let mut current_type = String::new();
    let mut current_entity = String::new();
    let mut extras: Vec<(String, String)> = Vec::new();
    let mut in_event = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[[event]]" {
            if in_event && !current_type.is_empty() {
                out.push(BioisoEvent {
                    entity: current_entity.clone(),
                    tick: current_tick,
                    event_type: current_type.clone(),
                    extra: std::mem::take(&mut extras),
                });
            }
            in_event = true;
            current_tick = 0;
            current_type.clear();
            current_entity.clear();
            extras.clear();
            continue;
        }
        if !in_event {
            continue;
        }
        if let Some((k, v)) = line.split_once(" = ") {
            let v = v.trim().trim_matches('"');
            match k {
                "tick" => current_tick = v.parse().unwrap_or(0),
                "type" => current_type = v.to_string(),
                "entity" => current_entity = v.to_string(),
                _ => extras.push((k.to_string(), v.to_string())),
            }
        }
    }
    if in_event && !current_type.is_empty() {
        out.push(BioisoEvent {
            entity: current_entity,
            tick: current_tick,
            event_type: current_type,
            extra: extras,
        });
    }
}

/// Print a human-readable aggregate report of scanned events.
pub fn print_scan_report(events: &[BioisoEvent], filter_type: Option<&str>) {
    let filtered: Vec<_> = events
        .iter()
        .filter(|e| filter_type.map_or(true, |t| e.event_type == t))
        .collect();

    if filtered.is_empty() {
        println!("No BIOISO events found.");
        return;
    }

    // Summary counts by type
    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &filtered {
        *type_counts.entry(e.event_type.as_str()).or_insert(0) += 1;
    }
    println!("=== BIOISO Colony Event Summary ===");
    let mut types: Vec<_> = type_counts.iter().collect();
    types.sort_by(|a, b| b.1.cmp(a.1));
    for (t, n) in &types {
        println!("  {t:<20} {n}");
    }
    println!();

    // Timeline
    println!("=== Event Timeline (tick asc) ===");
    for e in &filtered {
        let extra: Vec<String> = e.extra.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let suffix = if extra.is_empty() {
            String::new()
        } else {
            format!("  [{}]", extra.join(" "))
        };
        println!(
            "  tick={:>6}  {:>12}  {:>24}{}",
            e.tick, e.event_type, e.entity, suffix
        );
    }
}
