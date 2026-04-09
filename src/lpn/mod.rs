/// LPN — Loom Protocol Notation.
///
/// A minimal AI-to-AI communication protocol for orchestrating the Loom
/// compiler pipeline.  Three tiers of complexity:
///
/// | Tier | Example | Use |
/// |------|---------|-----|
/// | 1 | `EMIT rust ScalpingAgent FROM scalper.loom` | Single atomic op |
/// | 2 | `IMPL Foo USING [M41,M55] EMIT rust VERIFY compile` | Multi-step op |
/// | 3 | `ALX n=7 domain=biotech coverage>=0.95 evidence=store` | Named experiment |
///
/// ## File format
///
/// LPN source files use the `.lp` extension.  Each non-blank, non-comment
/// line is one instruction.  Lines starting with `#` are comments.
///
/// ```text
/// # setup the payment module
/// EMIT rust PaymentAPI FROM examples/02-payment-api.loom
/// CHECK all examples/02-payment-api.loom
/// IMPL ScalpingAgent USING [M41,M55,M84-M89] EMIT rust VERIFY compile+types
/// ```
///
/// ## API
///
/// ```rust,ignore
/// use loom::lpn::{LpnParser, LpnExecutor};
/// use std::path::PathBuf;
///
/// let instrs = LpnParser::parse_str(src);
/// let executor = LpnExecutor::new(PathBuf::from("."));
/// let results = executor.execute_all(&instrs);
/// for r in &results {
///     println!("{}: {:?}", r.instruction, r.status);
/// }
/// ```
pub mod ast;
pub mod error;
pub mod executor;
pub mod parser;

pub use ast::{CheckKind, EmitTarget, ExperimentParams, LpnInstruction, MilestoneRef, VerifyStep};
pub use error::LpnError;
pub use executor::{LpnExecutor, LpnResult, LpnStatus};
pub use parser::LpnParser;
