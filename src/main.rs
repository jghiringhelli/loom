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
    /// Compile a Loom source file to Rust or WASM.
    Compile {
        /// Path to the `.loom` source file.
        input: PathBuf,

        /// Path for the generated output file.
        /// Defaults to the input path with the extension replaced by `.rs` or `.wat`.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Run all compiler checks but do not write output.
        #[arg(long)]
        check_only: bool,

        /// Compilation target: `rust` (default) or `wasm` (WAT output).
        #[arg(long, default_value = "rust")]
        target: String,
    },

    /// Build a multi-module project from a `loom.toml` manifest.
    Build {
        /// Path to the `loom.toml` project manifest.
        #[arg(default_value = "loom.toml")]
        manifest: PathBuf,
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

// ── Entry point ───────────────────────────────────────────────────────────────

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
            let (default_ext, compile_result) = match target.as_str() {
                "wasm" => ("wat", loom::compile_wasm(&source)),
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

        Commands::Build { manifest } => {            let toml_src = match std::fs::read_to_string(&manifest) {
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

        Commands::Lpn { input, base_dir, format } => {
            let source = match std::fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", input.display(), e);
                    process::exit(1);
                }
            };

            let base = base_dir.unwrap_or_else(|| {
                input.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf()
            });

            let (instrs, parse_errs) =
                loom::lpn::LpnParser::parse_str_lenient(&source);

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
                            println!(r#"{{"status":"ok","instruction":{:?},"ms":{}}}"#,
                                r.instruction, r.duration_ms);
                        } else {
                            println!("  ✅ {} ({}ms)", r.instruction, r.duration_ms);
                        }
                    }
                    loom::lpn::LpnStatus::Err(msg) => {
                        exit_code = 1;
                        if format == "json" {
                            println!(r#"{{"status":"err","instruction":{:?},"error":{:?}}}"#,
                                r.instruction, msg);
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

            let ok = results.iter().filter(|r| r.status == loom::lpn::LpnStatus::Ok).count();
            let err = results.iter().filter(|r| matches!(r.status, loom::lpn::LpnStatus::Err(_))).count();
            println!("\nlpn: {} ok, {} failed, {} skipped",
                ok, err,
                results.iter().filter(|r| matches!(r.status, loom::lpn::LpnStatus::Skipped(_))).count());

            if !parse_errs.is_empty() || exit_code != 0 {
                process::exit(1);
            }
        }
    }
}
