//! Multi-module project compilation tests for M8.

use std::path::PathBuf;
use loom::project::{ProjectManifest, build_project};

// ── Manifest parsing ──────────────────────────────────────────────────────────

#[test]
fn manifest_parses_name_and_modules() {
    let toml = r#"
[project]
name = "my-app"
version = "0.1.0"
modules = ["src/a.loom", "src/b.loom"]
output = "out/"
"#;
    let manifest = ProjectManifest::from_str(toml).expect("parse failed");
    assert_eq!(manifest.name, "my-app");
    assert_eq!(manifest.modules.len(), 2);
}

#[test]
fn manifest_with_missing_name_errors() {
    let toml = r#"
[project]
version = "0.1.0"
modules = []
output = "out/"
"#;
    assert!(ProjectManifest::from_str(toml).is_err());
}

// ── Build: compile corpus project ─────────────────────────────────────────────

#[test]
fn build_single_module_project_succeeds() {
    // Use a temp dir so we don't pollute the working directory.
    let tmp = std::env::temp_dir().join("loom_m8_single");
    std::fs::create_dir_all(&tmp).unwrap();

    // Write a simple .loom file.
    let loom_src = r#"
module SimpleCalc
fn add :: Int -> Int -> Int
  0
end
end
"#;
    let loom_path = tmp.join("simple_calc.loom");
    std::fs::write(&loom_path, loom_src).unwrap();

    // Build via the project API.
    let out_dir = tmp.join("out");
    let result = build_project(
        &[loom_path.to_str().unwrap()],
        out_dir.to_str().unwrap(),
    );
    assert!(result.is_ok(), "build failed: {:?}", result);

    // Output file should exist.
    let rs_file = out_dir.join("simple_calc.rs");
    assert!(rs_file.exists(), "expected {:?} to exist", rs_file);

    // lib.rs should re-export the module.
    let lib_rs = out_dir.join("lib.rs");
    assert!(lib_rs.exists(), "expected lib.rs to exist");
    let lib_content = std::fs::read_to_string(&lib_rs).unwrap();
    assert!(lib_content.contains("mod simple_calc"), "expected mod re-export in lib.rs:\n{}", lib_content);

    // Cleanup.
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn build_multiple_modules_emits_all_outputs() {
    let tmp = std::env::temp_dir().join("loom_m8_multi");
    std::fs::create_dir_all(&tmp).unwrap();

    let a_src = "module Alpha\nfn f :: Int -> Int\n  0\nend\nend\n";
    let b_src = "module Beta\nfn g :: Bool -> Bool\n  true\nend\nend\n";

    let a_path = tmp.join("alpha.loom");
    let b_path = tmp.join("beta.loom");
    std::fs::write(&a_path, a_src).unwrap();
    std::fs::write(&b_path, b_src).unwrap();

    let out_dir = tmp.join("out");
    build_project(
        &[a_path.to_str().unwrap(), b_path.to_str().unwrap()],
        out_dir.to_str().unwrap(),
    ).expect("build failed");

    assert!(out_dir.join("alpha.rs").exists());
    assert!(out_dir.join("beta.rs").exists());

    let lib = std::fs::read_to_string(out_dir.join("lib.rs")).unwrap();
    assert!(lib.contains("mod alpha"), "missing mod alpha:\n{}", lib);
    assert!(lib.contains("mod beta"), "missing mod beta:\n{}", lib);

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── Error propagation ─────────────────────────────────────────────────────────

#[test]
fn build_with_compile_error_returns_error() {
    let tmp = std::env::temp_dir().join("loom_m8_err");
    std::fs::create_dir_all(&tmp).unwrap();

    // This file has a syntax error.
    let bad_src = "module Bad\nthis is not valid\nend\n";
    let path = tmp.join("bad.loom");
    std::fs::write(&path, bad_src).unwrap();

    let out_dir = tmp.join("out");
    let result = build_project(&[path.to_str().unwrap()], out_dir.to_str().unwrap());
    assert!(result.is_err(), "expected build to fail on invalid source");

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── CLI: loom build subcommand ────────────────────────────────────────────────

#[test]
fn loom_build_with_valid_toml_succeeds() {
    let tmp = std::env::temp_dir().join("loom_m8_cli");
    std::fs::create_dir_all(&tmp).unwrap();

    let src = "module Cli\nfn run :: Int -> Int\n  0\nend\nend\n";
    let loom_file = tmp.join("cli.loom");
    std::fs::write(&loom_file, src).unwrap();

    let out_dir = tmp.join("out");
    let toml_content = format!(
        "[project]\nname = \"cli-test\"\nversion = \"0.1.0\"\nmodules = [\"cli.loom\"]\noutput = \"out/\"\n"
    );
    let toml_path = tmp.join("loom.toml");
    std::fs::write(&toml_path, toml_content).unwrap();

    // Call the manifest-based build.
    let manifest = ProjectManifest::from_str(&std::fs::read_to_string(&toml_path).unwrap())
        .expect("parse failed");
    // Resolve module paths relative to the manifest's directory.
    let resolved: Vec<String> = manifest
        .modules
        .iter()
        .map(|m| tmp.join(m).to_str().unwrap().to_owned())
        .collect();
    let out_path = tmp.join(&manifest.output);

    let refs: Vec<&str> = resolved.iter().map(|s: &String| s.as_str()).collect();
    build_project(
        &refs,
        out_path.to_str().unwrap(),
    ).expect("CLI build failed");

    assert!(tmp.join("out/lib.rs").exists());
    let _ = std::fs::remove_dir_all(&tmp);
}
