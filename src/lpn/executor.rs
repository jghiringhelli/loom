/// LPN executor — maps LPN instructions to existing Loom pipeline operations.
///
/// Each instruction variant dispatches to the corresponding Loom pipeline
/// function.  Results are collected as [`LpnResult`] values so a caller can
/// process the full instruction set even when individual steps fail.
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::lpn::{ast::EmitTarget, error::LpnError, LpnInstruction};

// ── Result types ──────────────────────────────────────────────────────────────

/// The outcome status of a single executed LPN instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum LpnStatus {
    /// The instruction succeeded.
    Ok,
    /// The instruction failed with an error message.
    Err(String),
    /// The instruction was skipped (e.g. not yet implemented).
    Skipped(String),
}

/// The result of executing a single LPN instruction.
#[derive(Debug, Clone)]
pub struct LpnResult {
    /// The original instruction text (for display).
    pub instruction: String,
    /// Execution status.
    pub status: LpnStatus,
    /// Optional output (e.g. emitted Rust source, or error diagnostics).
    pub output: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl LpnResult {
    fn ok(instruction: impl Into<String>, output: impl Into<String>, ms: u64) -> Self {
        Self {
            instruction: instruction.into(),
            status: LpnStatus::Ok,
            output: Some(output.into()),
            duration_ms: ms,
        }
    }

    fn err(instruction: impl Into<String>, msg: impl Into<String>, ms: u64) -> Self {
        Self {
            instruction: instruction.into(),
            status: LpnStatus::Err(msg.into()),
            output: None,
            duration_ms: ms,
        }
    }

    fn skipped(instruction: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            instruction: instruction.into(),
            status: LpnStatus::Skipped(reason.into()),
            output: None,
            duration_ms: 0,
        }
    }
}

// ── Executor ──────────────────────────────────────────────────────────────────

/// Executes LPN instructions by delegating to the Loom compiler pipeline.
///
/// All file paths in instructions are resolved relative to `base_dir`.
pub struct LpnExecutor {
    base_dir: PathBuf,
}

impl LpnExecutor {
    /// Create a new executor with the given base directory for file resolution.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Execute a single instruction and return its result.
    pub fn execute(&self, instr: &LpnInstruction) -> LpnResult {
        let label = format!("{instr:?}");
        let start = Instant::now();
        let result = self.dispatch(instr, &label, start);
        result
    }

    /// Execute a slice of instructions in order and return all results.
    pub fn execute_all(&self, instrs: &[LpnInstruction]) -> Vec<LpnResult> {
        instrs.iter().map(|i| self.execute(i)).collect()
    }

    fn dispatch(&self, instr: &LpnInstruction, label: &str, start: Instant) -> LpnResult {
        match instr {
            LpnInstruction::Emit {
                target,
                from,
                module,
            } => self.execute_emit(target, module, from.as_deref(), label, start),
            LpnInstruction::Check { kind: _, file } => self.execute_check(file, label, start),
            LpnInstruction::Fn { name, sig } => self.execute_fn_snippet(name, sig, label, start),
            LpnInstruction::Type { name, body } => {
                self.execute_type_snippet(name, body, label, start)
            }
            LpnInstruction::Enum { name, body } => {
                self.execute_enum_snippet(name, body, label, start)
            }
            LpnInstruction::Impl { target, emit, .. } => {
                // Resolve: look for target.loom near base_dir
                let candidate = self.base_dir.join(format!("{target}.loom"));
                if candidate.exists() {
                    self.execute_emit(
                        emit,
                        target,
                        Some(candidate.to_str().unwrap_or("")),
                        label,
                        start,
                    )
                } else {
                    LpnResult::skipped(label, format!("file `{}.loom` not found", target))
                }
            }
            LpnInstruction::Alx(_) => {
                LpnResult::skipped(label, "ALX experiment runner not yet wired to executor")
            }
            _ => LpnResult::skipped(label, "instruction not yet implemented in executor"),
        }
    }

    fn execute_emit(
        &self,
        target: &EmitTarget,
        module: &str,
        from: Option<&str>,
        label: &str,
        start: Instant,
    ) -> LpnResult {
        let path = match self.resolve_source(from, module) {
            Ok(p) => p,
            Err(e) => return LpnResult::err(label, e.to_string(), elapsed(start)),
        };
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                return LpnResult::err(
                    label,
                    LpnError::Io {
                        path: path.display().to_string(),
                        cause: e.to_string(),
                    }
                    .to_string(),
                    elapsed(start),
                )
            }
        };
        let result = match target {
            EmitTarget::Rust => crate::compile(&source),
            EmitTarget::TypeScript => crate::compile_typescript(&source),
            EmitTarget::Wasm => crate::compile_wasm(&source),
            EmitTarget::OpenApi => crate::compile_openapi(&source),
            EmitTarget::Schema => crate::compile_json_schema(&source),
        };
        match result {
            Ok(out) => LpnResult::ok(label, out, elapsed(start)),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                LpnResult::err(label, msg, elapsed(start))
            }
        }
    }

    fn execute_check(&self, file: &str, label: &str, start: Instant) -> LpnResult {
        let path = self.base_dir.join(file);
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                return LpnResult::err(
                    label,
                    LpnError::Io {
                        path: path.display().to_string(),
                        cause: e.to_string(),
                    }
                    .to_string(),
                    elapsed(start),
                )
            }
        };
        // Running compile() exercises all checkers; we discard the output.
        match crate::compile(&source) {
            Ok(_) => LpnResult::ok(label, "all checks passed", elapsed(start)),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                LpnResult::err(label, msg, elapsed(start))
            }
        }
    }

    fn execute_fn_snippet(&self, name: &str, sig: &str, label: &str, start: Instant) -> LpnResult {
        let src = format!("module Snippet\nfn {name} :: {sig}\n  Unit\nend\n");
        match crate::compile(&src) {
            Ok(_) => LpnResult::ok(label, format!("fn {name} :: {sig} — ok"), elapsed(start)),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                LpnResult::err(label, msg, elapsed(start))
            }
        }
    }

    fn execute_type_snippet(
        &self,
        name: &str,
        body: &str,
        label: &str,
        start: Instant,
    ) -> LpnResult {
        // Convert inline `field:Type field2:Type` to newline-separated fields
        let fields = body.replace(' ', "\n  ");
        let src = format!("module Snippet\ntype {name} =\n  {fields}\nend\n");
        match crate::compile(&src) {
            Ok(_) => LpnResult::ok(label, format!("type {name} — ok"), elapsed(start)),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                LpnResult::err(label, msg, elapsed(start))
            }
        }
    }

    fn execute_enum_snippet(
        &self,
        name: &str,
        body: &str,
        label: &str,
        start: Instant,
    ) -> LpnResult {
        let src = format!("module Snippet\nenum {name} =\n  {body}\nend\n");
        match crate::compile(&src) {
            Ok(_) => LpnResult::ok(label, format!("enum {name} — ok"), elapsed(start)),
            Err(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                LpnResult::err(label, msg, elapsed(start))
            }
        }
    }

    /// Resolve a source file path: use `from` if given, else search for
    /// `{module}.loom` under `base_dir`.
    fn resolve_source(&self, from: Option<&str>, module: &str) -> Result<PathBuf, LpnError> {
        if let Some(f) = from {
            let p = if Path::new(f).is_absolute() {
                PathBuf::from(f)
            } else {
                self.base_dir.join(f)
            };
            if p.exists() {
                return Ok(p);
            }
            return Err(LpnError::Io {
                path: p.display().to_string(),
                cause: "file not found".into(),
            });
        }
        // Search heuristic: look in known directories
        for dir in &["examples", "experiments", "corpus", "."] {
            let candidate = self.base_dir.join(dir).join(format!("{module}.loom"));
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(LpnError::Io {
            path: format!("{module}.loom"),
            cause: "could not locate source file".into(),
        })
    }
}

fn elapsed(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}
