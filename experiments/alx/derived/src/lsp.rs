// G-lsp: LSP module stub — provides pure utility functions used by lsp_test.rs.
// Full LSP server implementation is out of scope for this derivation.

use crate::error::{LoomError, Span};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

// ── Position utilities ────────────────────────────────────────────────────────

/// Convert a byte offset in `src` to a (line, character) LSP position.
/// Both values are 0-indexed.
pub fn byte_offset_to_position_raw(src: &str, offset: usize) -> (u32, u32) {
    let clamped = offset.min(src.len());
    let prefix = &src[..clamped];
    let line = prefix.chars().filter(|&c| c == '\n').count() as u32;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = (clamped - last_newline) as u32;
    (line, character)
}

/// Convert a `LoomError` to an LSP `Diagnostic`.
pub fn loom_error_to_diagnostic(error: &LoomError, src: &str) -> Diagnostic {
    let span = error.span();
    let (start_line, start_char) = byte_offset_to_position_raw(src, span.start);
    let (end_line, end_char) = byte_offset_to_position_raw(src, span.end);

    Diagnostic {
        range: Range {
            start: Position { line: start_line, character: start_char },
            end: Position { line: end_line, character: end_char },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("loom".to_string()),
        message: error.to_string(),
        ..Default::default()
    }
}

// ── LSP server stub ───────────────────────────────────────────────────────────

use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub struct LoomLspServer {
    #[allow(dead_code)]
    client: Client,
}

impl LoomLspServer {
    pub fn new(client: Client) -> Self {
        LoomLspServer { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LoomLspServer {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }
}
