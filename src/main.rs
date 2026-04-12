//! `loom` CLI — compiles Loom source files to Rust.
//!
//! # Usage
//!
//! ```text
//! loom compile <INPUT> [--output <OUTPUT>] [--check-only]
//! ```

use std::path::PathBuf;
use std::process;

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

// ── Runtime command handlers ──────────────────────────────────────────────────

fn handle_runtime(subcommand: RuntimeCommands) {
    match subcommand {
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

        RuntimeCommands::Rollback { db, entity, checkpoint } => {
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
    }
}
