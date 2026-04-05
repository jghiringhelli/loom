// G7: project module — multi-file project support (M8).
// ProjectManifest parses a TOML project file.
// build_project compiles each .loom source file to Rust and writes outputs.

use crate::error::LoomError;

// ── Project manifest ──────────────────────────────────────────────────────────

#[derive(Debug)]
struct ProjectToml {
    project: ProjectConfig,
}

#[derive(Debug)]
struct ProjectConfig {
    name: String,
    modules: Vec<String>,
    output: String,
}

/// A project manifest parsed from a loom.toml file.
pub struct ProjectManifest {
    pub name: String,
    pub modules: Vec<String>,
    pub output: String,
}

impl ProjectManifest {
    /// Parse a TOML project manifest from a string.
    pub fn from_str(s: &str) -> Result<Self, String> {
        parse_toml_manifest(s)
    }
}

/// Minimal TOML parser for the project manifest format.
/// Parses [project] section with name, version, modules, output fields.
fn parse_toml_manifest(s: &str) -> Result<ProjectManifest, String> {
    let mut name: Option<String> = None;
    let mut modules: Vec<String> = Vec::new();
    let mut output = String::from("out/");
    let mut in_project_section = false;

    for line in s.lines() {
        let line = line.trim();

        if line == "[project]" {
            in_project_section = true;
            continue;
        }
        if line.starts_with('[') {
            in_project_section = false;
            continue;
        }
        if !in_project_section || line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "name" => {
                    name = Some(val.trim_matches('"').to_string());
                }
                "output" => {
                    output = val.trim_matches('"').to_string();
                }
                "modules" => {
                    // Parse array: ["a.loom", "b.loom"]
                    let inner = val.trim().trim_start_matches('[').trim_end_matches(']');
                    for item in inner.split(',') {
                        let item = item.trim().trim_matches('"');
                        if !item.is_empty() {
                            modules.push(item.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    match name {
        Some(name) => Ok(ProjectManifest { name, modules, output }),
        None => Err("project manifest missing required field: name".to_string()),
    }
}

// ── Build project ─────────────────────────────────────────────────────────────

/// Compile a list of .loom source files to Rust and write outputs to `out_dir`.
/// Writes `{stem}.rs` for each input file and `lib.rs` that re-exports all modules.
pub fn build_project(files: &[&str], out_dir: &str) -> Result<(), Vec<LoomError>> {
    std::fs::create_dir_all(out_dir).map_err(|e| {
        vec![LoomError::zero(format!("failed to create output dir '{}': {}", out_dir, e))]
    })?;

    let mut module_names: Vec<String> = Vec::new();

    for &file_path in files {
        let src = std::fs::read_to_string(file_path).map_err(|e| {
            vec![LoomError::zero(format!("failed to read '{}': {}", file_path, e))]
        })?;

        let rust_out = crate::compile(&src)?;

        let path = std::path::Path::new(file_path);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        let out_path = std::path::Path::new(out_dir).join(format!("{}.rs", stem));
        std::fs::write(&out_path, rust_out).map_err(|e| {
            vec![LoomError::zero(format!(
                "failed to write '{}': {}",
                out_path.display(),
                e
            ))]
        })?;

        module_names.push(stem.to_string());
    }

    // Write lib.rs that re-exports each compiled module.
    let lib_rs: String = module_names
        .iter()
        .map(|m| format!("pub mod {};\n", m))
        .collect();
    let lib_path = std::path::Path::new(out_dir).join("lib.rs");
    std::fs::write(&lib_path, lib_rs).map_err(|e| {
        vec![LoomError::zero(format!(
            "failed to write lib.rs: {}",
            e
        ))]
    })?;

    Ok(())
}
