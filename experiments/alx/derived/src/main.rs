// ALX: CLI entry point — G10 gap correction.
// Supports: loom compile <file> [--check-only] [--output <path>] [--target rust|wasm]
//           loom build <manifest>

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: loom <command> [args]");
        eprintln!("Commands:");
        eprintln!("  compile <file.loom> [--check-only] [--output <out>] [--target rust|wasm]");
        eprintln!("  build <manifest.toml>");
        process::exit(1);
    }

    match args[1].as_str() {
        "compile" => cmd_compile(&args[2..]),
        "build"   => cmd_build(&args[2..]),
        other => {
            eprintln!("error: unknown command '{}'", other);
            process::exit(1);
        }
    }
}

fn cmd_compile(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: compile requires a file argument");
        process::exit(1);
    }

    let file = &args[0];
    let mut check_only = false;
    let mut output: Option<String> = None;
    let mut target = "rust".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--check-only" => { check_only = true; }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].clone());
                } else {
                    eprintln!("error: --output requires a path argument");
                    process::exit(1);
                }
            }
            "--target" => {
                i += 1;
                if i < args.len() {
                    target = args[i].clone();
                } else {
                    eprintln!("error: --target requires rust or wasm");
                    process::exit(1);
                }
            }
            flag => {
                eprintln!("error: unknown flag '{}'", flag);
                process::exit(1);
            }
        }
        i += 1;
    }

    let src = std::fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("error: {}: {}", file, e);
        process::exit(1);
    });

    let result = match target.as_str() {
        "wasm" => loom::compile_wasm(&src),
        _ => loom::compile(&src),
    };

    let compiled = result.unwrap_or_else(|errs| {
        for e in &errs {
            eprintln!("error: {}: {:?}", file, e);
        }
        process::exit(1);
    });

    if check_only {
        println!("ok");
        return;
    }

    if let Some(out_path) = &output {
        std::fs::write(out_path, &compiled).unwrap_or_else(|e| {
            eprintln!("error: failed to write '{}': {}", out_path, e);
            process::exit(1);
        });
        println!("compiled {}", out_path);
    } else {
        print!("{}", compiled);
    }
}

fn cmd_build(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: build requires a manifest path");
        process::exit(1);
    }

    let manifest_path = &args[0];
    let manifest_src = std::fs::read_to_string(manifest_path).unwrap_or_else(|e| {
        eprintln!("error: {}: {}", manifest_path, e);
        process::exit(1);
    });

    let manifest = loom::project::ProjectManifest::from_str(&manifest_src).unwrap_or_else(|e| {
        eprintln!("error: failed to parse manifest '{}': {}", manifest_path, e);
        process::exit(1);
    });

    // Resolve module paths relative to the manifest's directory
    let manifest_dir = std::path::Path::new(manifest_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    let resolved: Vec<String> = manifest.modules.iter().map(|m| {
        manifest_dir.join(m).to_string_lossy().into_owned()
    }).collect();

    let file_refs: Vec<&str> = resolved.iter().map(|s| s.as_str()).collect();

    loom::project::build_project(&file_refs, &manifest.output).unwrap_or_else(|errs| {
        for e in &errs {
            eprintln!("error: {:?}", e);
        }
        process::exit(1);
    });

    println!("build ok");
}
