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
    /// Compile a Loom source file to Rust.
    Compile {
        /// Path to the `.loom` source file.
        input: PathBuf,

        /// Path for the generated `.rs` output file.
        /// Defaults to the input path with the extension replaced by `.rs`.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Run all compiler checks but do not write output.
        #[arg(long)]
        check_only: bool,
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
        } => {
            // Read the source file.
            let source = match std::fs::read_to_string(&input) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read `{}`: {}", input.display(), e);
                    process::exit(1);
                }
            };

            // Run the compiler pipeline.
            match loom::compile(&source) {
                Ok(rust_src) => {
                    if check_only {
                        println!("ok — no errors");
                        return;
                    }

                    // Determine the output path.
                    let out_path = output.unwrap_or_else(|| input.with_extension("rs"));

                    if let Err(e) = std::fs::write(&out_path, &rust_src) {
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
    }
}
