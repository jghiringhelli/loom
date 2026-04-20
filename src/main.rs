//! `loom` CLI — compiles Loom source files to Rust.
//!
//! # Usage
//!
//! ```text
//! loom compile <INPUT> [--output <OUTPUT>] [--check-only]
//! ```

use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use clap::{Parser, Subcommand};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "loom",
    version,
    about = "Loom language compiler — transpiles Loom source to Rust",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a Loom source file to one of the supported targets.
    Compile {
        /// Path to the `.loom` source file.
        input: PathBuf,

        /// Path for the generated output file.
        /// Defaults to the input path with the extension replaced by the
        /// target-appropriate extension.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Run all compiler checks but do not write output.
        #[arg(long)]
        check_only: bool,

        /// Compilation target (default: rust).
        ///
        /// Supported targets:
        ///   rust       — Rust source (.rs)
        ///   typescript — TypeScript source (.ts)
        ///   wasm       — WebAssembly text format (.wat)
        ///   openapi    — OpenAPI 3.0 YAML (.openapi.yaml)
        ///   json-schema — JSON Schema (.schema.json)
        ///   mermaid-c4  — Mermaid C4 context diagram (.c4.md)
        ///   mermaid-sequence — Mermaid sequence diagram (.seq.md)
        ///   mermaid-state    — Mermaid state diagram (.state.md)
        ///   mermaid-flow     — Mermaid flow diagram (.flow.md)
        ///   simulation       — Python simulation scaffold (.py)
        ///   neuroml          — NeuroML 2 XML document (.nml.xml)
        #[arg(long, default_value = "rust")]
        target: String,
    },

    /// Build a multi-module project from a `loom.toml` manifest.
    Build {
        /// Path to the `loom.toml` project manifest.
        #[arg(default_value = "loom.toml")]
        manifest: PathBuf,
    },

    /// BIOISO runtime commands — run and monitor live evolving entities.
    Runtime {
        #[command(subcommand)]
        subcommand: RuntimeCommands,
    },

    /// Verify .loom specifications (--tla for TLA+ model checking).
    ///
    /// Parses the given `.loom` file, extracts all beings with a `telos:` block,
    /// and writes a TLA+ spec (`<name>_convergence.tla`) and TLC config
    /// (`<name>_convergence.cfg`) for each one.  If `tlc` is on PATH it is
    /// invoked automatically; otherwise the command to run manually is printed.
    Verify {
        /// Path to the `.loom` source file.
        input: PathBuf,

        /// Emit TLA+ specs and attempt TLC model checking.
        #[arg(long)]
        tla: bool,
    },

    /// Execute a Loom Protocol Notation (`.lp`) instruction file.
    ///
    /// LPN is a minimal AI-to-AI wire format for orchestrating the Loom
    /// compiler pipeline.  Each non-blank, non-comment line is one
    /// instruction:
    ///
    ///   EMIT rust PaymentAPI FROM examples/02-payment-api.loom
    ///   CHECK all examples/02-payment-api.loom
    ///   IMPL ScalpingAgent USING [M41,M55,M84-M89] EMIT rust VERIFY compile
    ///   ALX n=7 domain=biotech coverage>=0.95 evidence=store
    Lpn {
        /// Path to the `.lp` instruction file.
        input: PathBuf,

        /// Base directory for resolving relative file paths in instructions.
        /// Defaults to the directory containing the input file.
        #[arg(long)]
        base_dir: Option<PathBuf>,

        /// Output format: `human` (default) or `json`.
        #[arg(long, default_value = "human")]
        format: String,
    },
}

// ── Runtime subcommands ────────────────────────────────────────────────────────

/// Subcommands under `loom runtime`.
#[derive(Subcommand)]
enum RuntimeCommands {
    /// Start the BIOISO evolution daemon (runs until Ctrl-C).
    ///
    /// Opens the signal store at `--db`, creates an Orchestrator, and runs the
    /// evolution loop on the configured tick interval.  Press Ctrl-C to stop.
    Start {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Tick interval in milliseconds (default: 5000).
        #[arg(long, default_value = "5000")]
        tick_ms: u64,
    },

    /// Show the current status of all entities in a BIOISO store.
    Status {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,
    },

    /// Show recent signals and drift events from a BIOISO store.
    Log {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Number of recent signals to show per entity.
        #[arg(long, default_value = "10")]
        n: usize,
    },

    /// Roll back an entity to a saved checkpoint.
    Rollback {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Entity ID to roll back.
        entity: String,

        /// Checkpoint ID to restore to.
        checkpoint: i64,
    },

    /// Spawn a new BIOISO entity, optionally inheriting epigenome from a parent.
    ///
    /// Reads the `.loom` source file (for metadata), registers the entity in the
    /// signal store, and optionally copies the parent's Core memories so the child
    /// warm-starts with inherited priors.
    ///
    /// Example:
    ///   loom runtime spawn my-entity --db bioiso.db --name "ClimateChild" \
    ///        --telos '{"target":1.5}' --inherit parent-entity
    Spawn {
        /// Entity ID to register.
        entity_id: String,

        /// Human-readable display name for the entity.
        #[arg(long)]
        name: Option<String>,

        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Telos JSON string (e.g. `{"target":1.5}`).
        /// Defaults to `{}` if not provided.
        #[arg(long, default_value = "{}")]
        telos: String,

        /// Parent entity ID to inherit epigenome from.
        /// If supplied, the new entity copies the parent's Semantic, Procedural,
        /// and Declarative Core memories as warm-start priors.
        #[arg(long)]
        inherit: Option<String>,

        /// Maximum number of divisions (telomere length).
        /// If omitted the entity has no senescence limit.
        #[arg(long)]
        telomere_limit: Option<u32>,
    },

    /// Seed the signal store with all 11 pre-configured BIOISO domain entities.
    ///
    /// Each entity is registered with its expert-calibrated telos bounds and
    /// baseline signals injected.  Already-registered entities are skipped
    /// (idempotent — safe to run on every deploy).
    ///
    /// Example:
    ///   loom runtime seed --db bioiso.db
    Seed {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Only seed a specific entity ID instead of all 11.
        #[arg(long)]
        only: Option<String>,
    },

    /// Run an autonomous experiment: inject signals, evolve entities, auto-branch.
    ///
    /// The experiment driver injects realistic domain-specific telemetry every tick,
    /// drives the full CEMS cycle (Membrane → drift → T1/T2/T3 proposals → gate →
    /// canary → promote/rollback → epigenome distillation → mycelium gossip), and
    /// automatically branches child entities when stable mutations accumulate.
    ///
    /// Progress is printed every `--summary-interval` ticks.  A final JSON summary is
    /// written to stdout (and optionally to `--log-path` as JSON lines).
    ///
    /// Example:
    ///   loom runtime experiment --db bioiso.db --ticks 500 --seed 42
    Experiment {
        /// Path to the BIOISO SQLite store (default: `bioiso.db`).
        #[arg(long, default_value = "bioiso.db")]
        db: String,

        /// Total number of ticks to simulate.
        #[arg(long, default_value_t = 500)]
        ticks: u64,

        /// Pseudo-random seed for the signal simulator.  42 = reproducible.
        #[arg(long, default_value_t = 42)]
        seed: u64,

        /// Milliseconds between ticks.  0 = maximum speed (no sleep).
        #[arg(long, default_value_t = 100)]
        tick_ms: u64,

        /// Print a progress summary every N ticks.
        #[arg(long, default_value_t = 10)]
        summary_interval: u64,

        /// Minimum stable mutations on a parent before spawning a branch.
        #[arg(long, default_value_t = 3)]
        branch_threshold: u32,

        /// Maximum child branches per parent entity over the run.
        #[arg(long, default_value_t = 2)]
        max_branches: u32,

        /// Restrict simulation to comma-separated entity IDs (empty = all).
        #[arg(long, default_value = "")]
        domains: String,

        /// Write per-tick JSON-lines to this file (optional).
        #[arg(long, default_value = "")]
        log_path: String,

        /// Maximum total living entities (branching suppressed above this cap).
        /// Overrides the `MAX_ENTITY_COUNT` environment variable.
        #[arg(long, default_value_t = 50)]
        max_entities: usize,

        /// Write telomere audit events as JSONL to this file (optional).
        #[arg(long, default_value = "")]
        telomere_log: String,

        /// Write experiment manifest (bioiso.toml) to this path (optional).
        #[arg(long, default_value = "")]
        manifest_path: String,
    },

    /// Pretty-print a BIOISO experiment JSONL log produced by `loom runtime experiment`.
    ///
    /// Reads the JSON-lines file written by `--log-path` and displays a human-readable
    /// summary of each tick: drift scores, proposals, promotions, tier activations,
    /// and branching events.
    ///
    /// Example:
    ///   loom runtime logs --path /data/experiment.jsonl --last 100
    Logs {
        /// Path to the JSON-lines experiment log.
        #[arg(long, default_value = "/data/experiment.jsonl")]
        path: String,

        /// Only show the last N ticks (0 = all).
        #[arg(long, default_value_t = 0)]
        last: usize,

        /// Filter to a specific entity ID (empty = all entities).
        #[arg(long, default_value = "")]
        entity: String,

        /// Only show ticks where Tier 3 (Claude) was activated.
        #[arg(long, default_value_t = false)]
        t3_only: bool,
    },
}

// ── Entry point ────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            check_only,
            target,
        } => {
            // Read the source file.
            let source = match std::fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", input.display(), e);
                    process::exit(1);
                }
            };

            // Determine compile function and default extension from target.
            // Returns (extension, compile_result).
            // Mermaid targets return Result<String, String> — wrap into the common error shape.
            type CompileResult = Result<String, Vec<loom::error::LoomError>>;

            fn mermaid_err(msg: String) -> Vec<loom::error::LoomError> {
                vec![loom::error::LoomError::CodegenError {
                    msg,
                    span: loom::ast::Span::synthetic(),
                }]
            }

            let (default_ext, compile_result): (&str, CompileResult) = match target.as_str() {
                "typescript" | "ts" => ("ts", loom::compile_typescript(&source)),
                "wasm" => ("wat", loom::compile_wasm(&source)),
                "openapi" | "openapi3" => ("openapi.yaml", loom::compile_openapi(&source)),
                "json-schema" | "schema" => ("schema.json", loom::compile_json_schema(&source)),
                "mermaid-c4" | "c4" => (
                    "c4.md",
                    loom::compile_mermaid_c4(&source).map_err(mermaid_err),
                ),
                "mermaid-sequence" | "sequence" => (
                    "seq.md",
                    loom::compile_mermaid_sequence(&source).map_err(mermaid_err),
                ),
                "mermaid-state" | "state" => (
                    "state.md",
                    loom::compile_mermaid_state(&source).map_err(mermaid_err),
                ),
                "mermaid-flow" | "flow" => (
                    "flow.md",
                    loom::compile_mermaid_flow(&source).map_err(mermaid_err),
                ),
                "simulation" | "sim" => ("sim.py", loom::compile_simulation(&source)),
                "neuroml" | "nml" => ("nml.xml", loom::compile_neuroml(&source)),
                _ => ("rs", loom::compile(&source)),
            };

            match compile_result {
                Ok(output_src) => {
                    if check_only {
                        println!("ok — no errors");
                        return;
                    }

                    // Determine the output path.
                    let out_path = output.unwrap_or_else(|| input.with_extension(default_ext));

                    if let Err(e) = std::fs::write(&out_path, &output_src) {
                        eprintln!("error: could not write `{}`: {}", out_path.display(), e);
                        process::exit(1);
                    }

                    println!("compiled `{}` → `{}`", input.display(), out_path.display());
                }

                Err(errors) => {
                    // Print each error in `file:offset: kind: message` format.
                    for err in &errors {
                        let span = err.span();
                        eprintln!(
                            "{}:{}:{}: {}: {}",
                            input.display(),
                            span.start,
                            span.end,
                            err.kind(),
                            err,
                        );
                    }
                    process::exit(1);
                }
            }
        }

        Commands::Verify { input, tla } => {
            if !tla {
                eprintln!("error: specify a verification mode; currently supported: --tla");
                process::exit(1);
            }

            let source = match std::fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", input.display(), e);
                    process::exit(1);
                }
            };

            // Compile to Rust so we can inspect the emitted TLA+ consts.
            // We re-emit the specs ourselves from the parse tree via the public
            // compile_tla_specs helper — falling back to the generated Rust source
            // if that helper is not yet wired.  For now we call compile() and
            // extract the `_TLA_SPEC` / `_TLC_CONFIG` const bodies from the output.
            let rust_src = match loom::compile(&source) {
                Ok(s) => s,
                Err(errors) => {
                    for err in &errors {
                        let span = err.span();
                        eprintln!(
                            "{}:{}:{}: {}: {}",
                            input.display(),
                            span.start,
                            span.end,
                            err.kind(),
                            err,
                        );
                    }
                    process::exit(1);
                }
            };

            // Extract (name, spec_body, cfg_body) triples from the Rust source.
            // Pattern: pub const <NAME>_TLA_SPEC: &str = r#"<body>"#;
            //          pub const <NAME>_TLC_CONFIG: &str = "<body>";
            let specs = extract_tla_specs(&rust_src);

            if specs.is_empty() {
                println!(
                    "no beings with telos: blocks found in `{}`",
                    input.display()
                );
                return;
            }

            let out_dir = input
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf();

            // Detect tlc on PATH once.
            let tlc_available = which_tlc();

            let mut any_fail = false;
            for (being_name, spec_body, cfg_body) in &specs {
                let tla_path = out_dir.join(format!("{being_name}_convergence.tla"));
                let cfg_path = out_dir.join(format!("{being_name}_convergence.cfg"));

                if let Err(e) = std::fs::write(&tla_path, spec_body) {
                    eprintln!("error: could not write `{}`: {e}", tla_path.display());
                    any_fail = true;
                    continue;
                }
                if let Err(e) = std::fs::write(&cfg_path, cfg_body) {
                    eprintln!("error: could not write `{}`: {e}", cfg_path.display());
                    any_fail = true;
                    continue;
                }

                println!(
                    "wrote `{}` and `{}`",
                    tla_path.display(),
                    cfg_path.display()
                );

                if let Some(ref tlc) = tlc_available {
                    // Run TLC: tlc <name>_convergence.tla -config <name>_convergence.cfg
                    let status = std::process::Command::new(tlc)
                        .arg(tla_path.to_str().unwrap_or_default())
                        .arg("-config")
                        .arg(cfg_path.to_str().unwrap_or_default())
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            println!("  TLC: PASS — {being_name}ConvergenceCheck");
                        }
                        Ok(_) => {
                            eprintln!("  TLC: FAIL — {being_name}ConvergenceCheck");
                            any_fail = true;
                        }
                        Err(e) => {
                            eprintln!("  TLC: error running `{tlc}`: {e}");
                            any_fail = true;
                        }
                    }
                } else {
                    println!(
                        "  TLA+ specs written. Run: java -cp tla2tools.jar tlc2.TLC {}",
                        tla_path.display()
                    );
                }
            }

            if any_fail {
                process::exit(1);
            }
        }

        Commands::Build { manifest } => {
            let toml_src = match std::fs::read_to_string(&manifest) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", manifest.display(), e);
                    process::exit(1);
                }
            };

            let parsed = match loom::project::ProjectManifest::from_str(&toml_src) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };

            // Resolve module paths relative to the manifest directory.
            let base = manifest
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let module_paths: Vec<String> = parsed
                .modules
                .iter()
                .map(|m| base.join(m).to_string_lossy().into_owned())
                .collect();
            let output_dir = base.join(&parsed.output).to_string_lossy().into_owned();
            let refs: Vec<&str> = module_paths.iter().map(|s| s.as_str()).collect();

            match loom::project::build_project(&refs, &output_dir) {
                Ok(()) => println!("build ok — {} module(s) compiled", refs.len()),
                Err(errors) => {
                    for err in &errors {
                        eprintln!("{}: {}", err.kind(), err);
                    }
                    process::exit(1);
                }
            }
        }

        Commands::Runtime { subcommand } => {
            handle_runtime(subcommand);
        }

        Commands::Lpn {
            input,
            base_dir,
            format,
        } => {
            let source = match std::fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", input.display(), e);
                    process::exit(1);
                }
            };

            let base = base_dir.unwrap_or_else(|| {
                input
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf()
            });

            let (instrs, parse_errs) = loom::lpn::LpnParser::parse_str_lenient(&source);

            for e in &parse_errs {
                eprintln!("lpn: {e}");
            }

            let executor = loom::lpn::LpnExecutor::new(base);
            let results = executor.execute_all(&instrs);

            let mut exit_code = 0i32;
            for r in &results {
                match &r.status {
                    loom::lpn::LpnStatus::Ok => {
                        if format == "json" {
                            println!(
                                r#"{{"status":"ok","instruction":{:?},"ms":{}}}"#,
                                r.instruction, r.duration_ms
                            );
                        } else {
                            println!("  ✅ {} ({}ms)", r.instruction, r.duration_ms);
                        }
                    }
                    loom::lpn::LpnStatus::Err(msg) => {
                        exit_code = 1;
                        if format == "json" {
                            println!(
                                r#"{{"status":"err","instruction":{:?},"error":{:?}}}"#,
                                r.instruction, msg
                            );
                        } else {
                            eprintln!("  ❌ {}\n     {}", r.instruction, msg);
                        }
                    }
                    loom::lpn::LpnStatus::Skipped(reason) => {
                        if format != "json" {
                            println!("  ⏭  {} — {}", r.instruction, reason);
                        }
                    }
                }
            }

            let ok = results
                .iter()
                .filter(|r| r.status == loom::lpn::LpnStatus::Ok)
                .count();
            let err = results
                .iter()
                .filter(|r| matches!(r.status, loom::lpn::LpnStatus::Err(_)))
                .count();
            println!(
                "\nlpn: {} ok, {} failed, {} skipped",
                ok,
                err,
                results
                    .iter()
                    .filter(|r| matches!(r.status, loom::lpn::LpnStatus::Skipped(_)))
                    .count()
            );

            if !parse_errs.is_empty() || exit_code != 0 {
                process::exit(1);
            }
        }
    }
}

// ── TLA+ verification helpers ─────────────────────────────────────────────────

/// Check whether `tlc` is available on PATH; returns the command name if found.
fn which_tlc() -> Option<String> {
    let output = std::process::Command::new("tlc").arg("--version").output();
    if output.is_ok() {
        return Some("tlc".to_string());
    }
    None
}

/// Extract `(being_name, tla_spec, tlc_config)` triples from emitted Rust source.
///
/// Looks for the pattern:
/// ```
/// pub const <NAME>_TLA_SPEC: &str = r#"<body>"#;
/// pub const <NAME>_TLC_CONFIG: &str = "<body>";
/// ```
fn extract_tla_specs(rust_src: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();

    // Find all _TLA_SPEC consts (raw string literals).
    let spec_marker = "_TLA_SPEC: &str = r#\"";

    let mut search = rust_src;
    while let Some(spec_pos) = search.find(spec_marker) {
        // Extract being name: walk backwards from spec_pos to find "pub const ".
        let prefix = &search[..spec_pos];
        let name_start = match prefix.rfind("pub const ") {
            Some(p) => p + "pub const ".len(),
            None => {
                search = &search[spec_pos + spec_marker.len()..];
                continue;
            }
        };
        let raw_name = &prefix[name_start..]; // e.g. "MYENTITY"
                                              // The being name is everything before "_TLA_SPEC" but we want the
                                              // Pascal-case name.  The const is uppercase; recover it by lowercasing.
                                              // We stored being.name (original case) in the format string so the const
                                              // starts with the being name uppercased.  We reconstruct it as stored.
        let being_name_upper = raw_name.trim().to_string();
        // Convert UPPER_SNAKE to the original identifier stored in the spec body.
        // The spec body starts with "---- MODULE <Name>ConvergenceCheck ----".
        // We'll parse it out from the body itself below.

        // Extract the raw-string body.
        let body_start = spec_pos + spec_marker.len();
        let remaining = &search[body_start..];
        let body_end = match remaining.find("\"#;") {
            Some(p) => p,
            None => {
                search = remaining;
                continue;
            }
        };
        let spec_body = remaining[..body_end].to_string();

        // Derive being_name from the MODULE line.
        let being_name = spec_body
            .lines()
            .find(|l| l.contains("MODULE") && l.contains("ConvergenceCheck"))
            .and_then(|l| {
                let idx = l.find("MODULE ")? + "MODULE ".len();
                let rest = &l[idx..];
                let end = rest.find("ConvergenceCheck")?;
                Some(rest[..end].trim().to_string())
            })
            .unwrap_or_else(|| {
                // Fallback: just use lowercased const prefix.
                being_name_upper
                    .trim_end_matches("_TLA_SPEC")
                    .to_lowercase()
            });

        // Now find the matching _TLC_CONFIG const for this being.
        let cfg_const = format!(
            "{}_TLC_CONFIG: &str = \"",
            being_name_upper.trim_end_matches("_TLA_SPEC")
        );
        let cfg_body = if let Some(cfg_pos) = rust_src.find(&cfg_const) {
            let cfg_body_start = cfg_pos + cfg_const.len();
            let cfg_remaining = &rust_src[cfg_body_start..];
            // Config uses a plain string literal terminated by \n";
            if let Some(cfg_end) = cfg_remaining.find("\";") {
                cfg_remaining[..cfg_end].replace("\\n", "\n")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        results.push((being_name, spec_body, cfg_body));

        // Advance search past this spec.
        search = &search[body_start + body_end + "\"#;".len()..];
    }

    results
}

// ── Runtime command handlers ──────────────────────────────────────────────────

fn handle_runtime(subcommand: RuntimeCommands) {
    match subcommand {
        RuntimeCommands::Start { db, tick_ms } => {
            use loom::runtime::orchestrator::{Orchestrator, OrchestratorConfig};
            use std::sync::atomic::{AtomicBool, Ordering};

            let runtime = match loom::runtime::Runtime::new(&db) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };

            let mut config = OrchestratorConfig::default();
            config.tick_interval = std::time::Duration::from_millis(tick_ms);
            if let Ok(v) = std::env::var("T2_MIN_INTERVAL_TICKS") {
                if let Ok(n) = v.parse::<u64>() {
                    config.t2_min_interval_ticks = n;
                }
            }

            let stop = Arc::new(AtomicBool::new(false));

            // The daemon runs until SIGTERM (Railway redeploy) or SIGKILL terminates
            // the process.  For local interactive use, Ctrl-C sends SIGINT which
            // terminates the process via the default OS handler.
            // We never stop on stdin EOF — Railway attaches a pseudo-TTY which would
            // cause immediate EOF otherwise.

            println!(
                "bioiso: starting evolution daemon (store={db}, tick={tick_ms}ms). \
                 Press Ctrl-C or send EOF to stop."
            );

            let mut orch = Orchestrator::new(runtime, config);
            orch.run_loop(&stop);

            println!("bioiso: daemon stopped.");
        }

        RuntimeCommands::Status { db } => {
            let store = match loom::runtime::SignalStore::new(&db) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };
            let entities = store.all_entities().unwrap_or_default();
            if entities.is_empty() {
                println!("no entities registered in `{db}`");
                return;
            }
            println!("{:<20} {:<20} {:<12} {}", "ID", "NAME", "STATE", "BORN AT");
            println!("{}", "-".repeat(64));
            for e in &entities {
                println!("{:<20} {:<20} {:<12} {}", e.id, e.name, e.state, e.born_at);
            }
        }

        RuntimeCommands::Log { db, n } => {
            let store = match loom::runtime::SignalStore::new(&db) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };
            let entities = store.all_entities().unwrap_or_default();
            for e in &entities {
                println!("# {} ({})", e.id, e.name);
                let sigs = store.signals_for_entity(&e.id, n).unwrap_or_default();
                if sigs.is_empty() {
                    println!("  (no signals)");
                } else {
                    for s in &sigs {
                        println!("  {} = {} @ {}", s.metric, s.value, s.timestamp);
                    }
                }
                if let Ok(Some(drift)) = store.latest_drift_score(&e.id) {
                    println!("  drift score: {:.3}", drift);
                }
                println!();
            }
        }

        RuntimeCommands::Rollback {
            db,
            entity,
            checkpoint,
        } => {
            let store = match loom::runtime::SignalStore::new(&db) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };
            let deployer = loom::runtime::deploy::CanaryDeployer::new();
            if deployer.rollback(&entity, checkpoint, &store) {
                println!("rolled back `{entity}` to checkpoint {checkpoint}");
            } else {
                eprintln!("error: rollback failed for entity `{entity}`");
                process::exit(1);
            }
        }

        RuntimeCommands::Spawn {
            entity_id,
            name,
            db,
            telos,
            inherit,
            telomere_limit,
        } => {
            let mut runtime = match loom::runtime::Runtime::new(&db) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };

            let display_name = name.unwrap_or_else(|| entity_id.clone());

            if let Err(e) =
                runtime.spawn_entity(&entity_id, &display_name, &telos, telomere_limit, None)
            {
                eprintln!("error: failed to register entity `{entity_id}`: {e}");
                process::exit(1);
            }

            // Epigenetic inheritance: copy Core memories from parent if requested.
            let inherited_count = if let Some(ref parent_id) = inherit {
                let count = runtime.inherit_epigenome(parent_id, &entity_id);
                if count == 0 {
                    eprintln!(
                        "warning: parent `{parent_id}` has no Core memories to inherit \
                         (entity may not exist or is a cold-start with no epigenome data)"
                    );
                }
                count
            } else {
                0
            };

            // Summarise warm-start params if any were inherited.
            let params = runtime.warm_start_params(&entity_id);

            println!("spawned entity `{entity_id}` ({display_name})");
            if let Some(ref parent_id) = inherit {
                println!("  inherited {inherited_count} Core memories from `{parent_id}`");
                if !params.is_empty() {
                    println!("  warm-start params ({}):", params.len());
                    let mut sorted: Vec<_> = params.iter().collect();
                    sorted.sort_by_key(|(k, _)| k.as_str());
                    for (param, value) in sorted {
                        println!("    {param} = {value:.6}");
                    }
                }
            }
        }

        RuntimeCommands::Seed { db, only } => {
            let mut runtime = match loom::runtime::Runtime::new(&db) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };

            let runner = loom::runtime::BIOISORunner::new();
            let specs = loom::runtime::all_domain_specs();

            let to_seed: Vec<_> = if let Some(ref id) = only {
                specs
                    .iter()
                    .filter(|s| s.entity_id == id.as_str())
                    .collect()
            } else {
                specs.iter().collect()
            };

            if to_seed.is_empty() {
                if let Some(ref id) = only {
                    eprintln!("error: no built-in spec found for entity `{id}`");
                    eprintln!("built-in IDs: climate, epidemics, antibiotic_res, grid_stability,");
                    eprintln!("              soil_carbon, sepsis, flash_crash, nuclear_safety,");
                    eprintln!("              supply_chain, water_basin, urban_heat");
                }
                process::exit(1);
            }

            let mut seeded = 0usize;
            let mut skipped = 0usize;
            for spec in &to_seed {
                // Check if already registered — idempotent.
                let already = runtime
                    .store
                    .all_entities()
                    .unwrap_or_default()
                    .iter()
                    .any(|e| e.id == spec.entity_id);
                if already {
                    println!("  skip  {} (already registered)", spec.entity_id);
                    skipped += 1;
                    continue;
                }
                match runner.spawn_domain(&mut runtime, spec) {
                    Ok(()) => {
                        let bound_count = spec.bounds.len();
                        let sig_count = spec.baseline_signals.len();
                        println!(
                            "  seeded {} ({}) — {bound_count} telos bounds, {sig_count} baseline signals",
                            spec.entity_id, spec.name
                        );
                        seeded += 1;
                    }
                    Err(e) => {
                        eprintln!("  error seeding {}: {e}", spec.entity_id);
                    }
                }
            }
            println!("\nseed complete: {seeded} seeded, {skipped} skipped");
        }

        RuntimeCommands::Experiment {
            db,
            ticks,
            seed,
            tick_ms,
            summary_interval,
            branch_threshold,
            max_branches,
            domains,
            log_path,
            max_entities,
            telomere_log,
            manifest_path,
        } => {
            use loom::runtime::experiment::{ExperimentConfig, ExperimentDriver};
            use loom::runtime::{all_domain_specs, BIOISORunner, Runtime};

            let mut runtime = match Runtime::new(&db) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: could not open store `{db}`: {e}");
                    process::exit(1);
                }
            };

            let runner = BIOISORunner::new();

            // Auto-seed if no entities registered yet.
            {
                let existing = runtime.store.all_entities().unwrap_or_default();
                if existing.is_empty() {
                    eprintln!("info: no entities found — running seed first");
                    for spec in all_domain_specs() {
                        if let Err(e) = runner.spawn_domain(&mut runtime, &spec) {
                            eprintln!("  warn: seed failed for {}: {e}", spec.entity_id);
                        } else {
                            eprintln!("  seeded {}", spec.entity_id);
                        }
                    }
                }
            }

            // Always repopulate in-memory state (gate sources + T1 rules) from specs.
            // spawn_domain only runs on first boot; after a restart the SQLite entities
            // exist but the in-memory supervisor and gate are empty. This is the T1=0 fix.
            runner.repopulate_in_memory(&mut runtime);

            let entity_filter: Vec<String> = if domains.is_empty() {
                Vec::new()
            } else {
                domains
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };

            // MAX_ENTITY_COUNT env var overrides the CLI default (CLI wins if explicitly set).
            let effective_max_entities = std::env::var("MAX_ENTITY_COUNT")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(max_entities);

            let config = ExperimentConfig {
                total_ticks: ticks,
                tick_interval_ms: tick_ms,
                rng_seed: seed,
                entity_filter,
                summary_interval,
                branch_threshold,
                max_branches_per_entity: max_branches,
                autonomous: true,
                log_path: log_path.clone(),
                run_meiosis: std::env::var("GITHUB_TOKEN").is_ok()
                    && std::env::var("GITHUB_REPO").is_ok(),
                meiosis_generation: 1,
                max_entity_count: effective_max_entities,
                telomere_log_path: telomere_log.clone(),
                manifest_path: manifest_path.clone(),
            };

            eprintln!(
                "experiment: ticks={ticks} seed={seed} tick_ms={tick_ms} \
                 branch_threshold={branch_threshold} autonomous=true"
            );
            if !log_path.is_empty() {
                eprintln!("experiment: writing JSON-lines to `{log_path}`");
            }
            if !telomere_log.is_empty() {
                eprintln!("experiment: telomere audit log → `{telomere_log}`");
            }
            if !manifest_path.is_empty() {
                eprintln!("experiment: project manifest → `{manifest_path}`");
            }

            let mut driver = ExperimentDriver::new(runtime, config);
            let summary = driver.run(None);

            println!("\n=== Experiment Complete ===");
            println!("Ticks:           {}", summary.total_ticks);
            println!("Signals injected:{}", summary.total_signals_injected);
            println!("Drift events:    {}", summary.total_drift_events);
            println!("Proposals:       {}", summary.total_proposals);
            println!("Promoted:        {}", summary.total_promoted);
            println!("Rolled back:     {}", summary.total_rolled_back);
            println!("Branches:        {}", summary.branch_decisions.len());
            if let Some(ct) = summary.convergence_tick {
                println!("Convergence:     tick {ct}");
            } else {
                println!("Convergence:     not reached");
            }
            println!("\nFinal entities: {}", summary.entities_final.join(", "));

            if !summary.branch_decisions.is_empty() {
                println!("\nBranch decisions:");
                for b in &summary.branch_decisions {
                    println!(
                        "  tick {:>4}: {} → {} ({})",
                        b.tick, b.parent_id, b.child_id, b.trigger_reason
                    );
                }
            }

            println!("\nTier activations:");
            for tier in ["1", "2", "3"] {
                let count = summary.tier_activations.get(tier).copied().unwrap_or(0);
                println!("  Tier {tier}: {count}");
            }

            println!("\nColony telos alignment (retro score ≥ 0.7 = mandate met):");
            println!("  Mean score: {:.3}", summary.retro_mean_score);
            for s in &summary.retro_stats {
                println!(
                    "  {:<22} score={:.3}  drift_mean={:.3}  within={}/{}",
                    s.entity_id,
                    s.overall_score,
                    s.mean_drift,
                    s.ticks_within_tolerance,
                    s.ticks_tracked,
                );
            }

            // Write JSON summary to stdout
            if let Ok(json) = serde_json::to_string_pretty(&summary) {
                println!("\n--- JSON Summary ---");
                println!("{json}");
            }
        }

        RuntimeCommands::Logs {
            path,
            last,
            entity,
            t3_only,
        } => {
            use loom::runtime::experiment::TickMetrics;

            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{path}`: {e}");
                    process::exit(1);
                }
            };

            let mut ticks: Vec<TickMetrics> = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();

            // Filters
            if t3_only {
                ticks.retain(|t| t.tier_used == Some(3));
            }
            if !entity.is_empty() {
                ticks.retain(|t| {
                    t.drift_scores.contains_key(&entity)
                        || t.entities_branched_this_tick.iter().any(|e| e == &entity)
                });
            }
            if last > 0 && ticks.len() > last {
                let skip = ticks.len() - last;
                ticks.drain(..skip);
            }

            if ticks.is_empty() {
                println!("No matching ticks found in `{path}`.");
                return;
            }

            println!(
                "  {:>6}  {:>5}  {:>5}  {:>5}  {:>4}  {:>4}  {}",
                "TICK", "DRIFT", "PROP", "PROM", "TIER", "BRNCH", "TOP DRIFTERS"
            );
            println!("{}", "-".repeat(80));

            for t in &ticks {
                let drift_count = t.drift_scores.len();
                let tier_str = t
                    .tier_used
                    .map(|u| format!("T{u}"))
                    .unwrap_or_else(|| "-".into());
                let branch_count = t.entities_branched_this_tick.len();

                // Top 3 drift scores
                let mut sorted: Vec<(&String, &f64)> = t.drift_scores.iter().collect();
                sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
                let top: Vec<String> = sorted
                    .iter()
                    .take(3)
                    .map(|(k, v)| format!("{k}={v:.2}"))
                    .collect();

                println!(
                    "  {:>6}  {:>5}  {:>5}  {:>5}  {:>4}  {:>5}  {}",
                    t.tick,
                    drift_count,
                    t.proposals,
                    t.promoted,
                    tier_str,
                    branch_count,
                    top.join("  "),
                );

                if !t.entities_branched_this_tick.is_empty() {
                    println!(
                        "         branched: {}",
                        t.entities_branched_this_tick.join(", ")
                    );
                }
            }

            println!("{}", "-".repeat(80));
            println!(
                "  {} ticks shown{}{}",
                ticks.len(),
                if t3_only { " (T3 only)" } else { "" },
                if !entity.is_empty() {
                    format!(" (entity filter: {entity})")
                } else {
                    String::new()
                }
            );
        }
    }
}
