//! CLI integration tests — invoke `loom` as a subprocess and verify behaviour.
//!
//! These tests exercise the command-line interface end-to-end:
//! - `loom compile <file>` — compile a Loom source file to Rust
//! - `loom compile --check-only` — check without writing output
//! - `loom compile --target wasm` — compile to WAT
//! - `loom build <manifest>` — multi-module project build
//! - Error cases: bad paths, invalid source, invalid target
//! - Exit codes (0 = success, 1 = error)
//!
//! The binary must be built before running: `cargo build --bin loom`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

// ── Helper ────────────────────────────────────────────────────────────────────

fn loom_bin() -> PathBuf {
    // In CI / test mode, cargo places the binary under target/debug/
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // remove test binary name
    if p.ends_with("deps") {
        p.pop();
    }
    p.push("loom");
    // Windows
    p.set_extension("exe");
    if !p.exists() {
        // Fallback: look relative to workspace root
        p = PathBuf::from("target/debug/loom.exe");
    }
    p
}

fn run(args: &[&str]) -> Output {
    Command::new(loom_bin())
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run loom binary")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// ── loom compile --check-only ─────────────────────────────────────────────────

#[test]
fn check_only_valid_file_exits_zero() {
    let out = run(&["compile", "corpus/pricing_engine.loom", "--check-only"]);
    assert!(
        out.status.success(),
        "expected exit 0, stderr={}",
        stderr(&out)
    );
    assert!(stdout(&out).contains("ok"), "expected 'ok' in output");
}

#[test]
fn check_only_does_not_write_output_file() {
    let tmp = std::env::temp_dir().join("loom_cli_check_test.rs");
    let _ = fs::remove_file(&tmp); // clean up first
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--check-only",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    assert!(!tmp.exists(), "--check-only must not write output file");
}

// ── loom compile (writes output) ─────────────────────────────────────────────

#[test]
fn compile_writes_rs_file() {
    let tmp = std::env::temp_dir().join("loom_cli_compile_out.rs");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "compile failed: {}", stderr(&out));
    assert!(tmp.exists(), "output .rs file was not created");
    let content = fs::read_to_string(&tmp).unwrap();
    assert!(
        content.contains("pub fn compute_total"),
        "output missing expected fn"
    );
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_prints_compiled_message() {
    let tmp = std::env::temp_dir().join("loom_cli_msg_test.rs");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let msg = stdout(&out);
    assert!(
        msg.contains("compiled"),
        "expected 'compiled' in stdout: {}",
        msg
    );
    let _ = fs::remove_file(&tmp);
}

// ── loom compile --target wasm ────────────────────────────────────────────────

#[test]
fn compile_wasm_target_writes_wat_file() {
    let tmp = std::env::temp_dir().join("loom_cli_wasm_out.wat");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/wasm_demo.loom",
        "--target",
        "wasm",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "wasm compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "wasm output file not created");
    let content = fs::read_to_string(&tmp).unwrap();
    assert!(content.contains("(module"), "expected WAT module in output");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_wasm_check_only() {
    let out = run(&[
        "compile",
        "corpus/wasm_demo.loom",
        "--target",
        "wasm",
        "--check-only",
    ]);
    assert!(out.status.success(), "expected success: {}", stderr(&out));
}

// ── loom compile error cases ──────────────────────────────────────────────────

#[test]
fn compile_missing_file_exits_nonzero() {
    let out = run(&["compile", "does_not_exist.loom"]);
    assert!(
        !out.status.success(),
        "expected non-zero exit for missing file"
    );
    assert!(
        stderr(&out).contains("error"),
        "expected error message in stderr"
    );
}

#[test]
fn compile_invalid_loom_exits_nonzero_with_error() {
    let tmp = std::env::temp_dir().join("loom_cli_bad.loom");
    fs::write(&tmp, "this is not valid loom source !!!").unwrap();
    let out = run(&["compile", tmp.to_str().unwrap(), "--check-only"]);
    assert!(
        !out.status.success(),
        "expected non-zero exit for bad source"
    );
    assert!(!stderr(&out).is_empty(), "expected error message in stderr");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_error_message_contains_file_name() {
    let tmp = std::env::temp_dir().join("loom_cli_named.loom");
    fs::write(&tmp, "module Bad\nfn broken :: this is bad\nend").unwrap();
    let out = run(&["compile", tmp.to_str().unwrap(), "--check-only"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("loom_cli_named.loom") || err.contains("loom_cli_named"),
        "expected file name in error message: {}",
        err
    );
    let _ = fs::remove_file(&tmp);
}

// ── loom compile --target typescript ─────────────────────────────────────────

#[test]
fn compile_typescript_target_writes_ts_file() {
    let tmp = std::env::temp_dir().join("loom_cli_ts_out.ts");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "typescript",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "typescript compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "typescript output file not created");
    let content = fs::read_to_string(&tmp).unwrap();
    assert!(
        !content.is_empty(),
        "typescript output should not be empty"
    );
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_typescript_alias_ts_works() {
    let tmp = std::env::temp_dir().join("loom_cli_ts_alias.ts");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "ts",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "ts alias failed: {}",
        stderr(&out)
    );
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_typescript_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "typescript",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "typescript check-only failed: {}",
        stderr(&out)
    );
}

// ── loom compile --target openapi ────────────────────────────────────────────

#[test]
fn compile_openapi_target_writes_yaml_file() {
    let tmp = std::env::temp_dir().join("loom_cli_openapi_out.yaml");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "openapi",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "openapi compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "openapi output file not created");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_openapi_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "openapi",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "openapi check-only failed: {}",
        stderr(&out)
    );
}

// ── loom compile --target json-schema ────────────────────────────────────────

#[test]
fn compile_json_schema_target_writes_json_file() {
    let tmp = std::env::temp_dir().join("loom_cli_schema_out.json");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "json-schema",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "json-schema compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "json-schema output file not created");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_json_schema_alias_schema() {
    let tmp = std::env::temp_dir().join("loom_cli_schema_alias.json");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "schema",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "schema alias failed: {}",
        stderr(&out)
    );
    let _ = fs::remove_file(&tmp);
}

// ── loom compile --target mermaid-* ──────────────────────────────────────────

#[test]
fn compile_mermaid_c4_target_writes_md_file() {
    let tmp = std::env::temp_dir().join("loom_cli_c4_out.md");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "mermaid-c4",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "mermaid-c4 compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "mermaid-c4 output file not created");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_mermaid_sequence_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "mermaid-sequence",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "mermaid-sequence check-only failed: {}",
        stderr(&out)
    );
}

#[test]
fn compile_mermaid_state_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "mermaid-state",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "mermaid-state check-only failed: {}",
        stderr(&out)
    );
}

#[test]
fn compile_mermaid_flow_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "mermaid-flow",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "mermaid-flow check-only failed: {}",
        stderr(&out)
    );
}

// ── loom compile --target simulation ─────────────────────────────────────────

#[test]
fn compile_simulation_target_writes_py_file() {
    let tmp = std::env::temp_dir().join("loom_cli_sim_out.py");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "simulation",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "simulation compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "simulation output file not created");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_simulation_alias_sim_works() {
    let tmp = std::env::temp_dir().join("loom_cli_sim_alias.py");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "sim",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "sim alias failed: {}",
        stderr(&out)
    );
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_simulation_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "simulation",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "simulation check-only failed: {}",
        stderr(&out)
    );
}

// ── loom compile --target neuroml ─────────────────────────────────────────────

#[test]
fn compile_neuroml_target_writes_xml_file() {
    let tmp = std::env::temp_dir().join("loom_cli_nml_out.xml");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "neuroml",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "neuroml compile failed: {}",
        stderr(&out)
    );
    assert!(tmp.exists(), "neuroml output file not created");
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_neuroml_alias_nml_works() {
    let tmp = std::env::temp_dir().join("loom_cli_nml_alias.xml");
    let _ = fs::remove_file(&tmp);
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "nml",
        "--output",
        tmp.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "nml alias failed: {}", stderr(&out));
    let _ = fs::remove_file(&tmp);
}

#[test]
fn compile_neuroml_check_only() {
    let out = run(&[
        "compile",
        "corpus/pricing_engine.loom",
        "--target",
        "neuroml",
        "--check-only",
    ]);
    assert!(
        out.status.success(),
        "neuroml check-only failed: {}",
        stderr(&out)
    );
}

// ── loom build ────────────────────────────────────────────────────────────────

#[test]
fn build_project_with_valid_manifest() {
    // Use the existing loom.toml at the workspace root — paths in it are relative
    // to the manifest, so we pass the manifest path and let loom resolve from there.
    let out = run(&["build", "loom.toml"]);
    assert!(
        out.status.success(),
        "build failed: {}\n{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("build ok"),
        "expected 'build ok' in output: {}",
        stdout(&out)
    );
}

#[test]
fn build_missing_manifest_exits_nonzero() {
    let out = run(&["build", "nonexistent_manifest.toml"]);
    assert!(
        !out.status.success(),
        "expected non-zero exit for missing manifest"
    );
    assert!(stderr(&out).contains("error"), "expected error message");
}
