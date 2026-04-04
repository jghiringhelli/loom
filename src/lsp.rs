//! Language Server Protocol server for Loom.
//!
//! Implements `textDocument/didOpen`, `textDocument/didChange`,
//! `textDocument/hover`, and `textDocument/definition` via tower-lsp.
//!
//! Two pure helper functions (`byte_offset_to_position_raw` and
//! `loom_error_to_diagnostic`) are public so they can be unit-tested directly.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::LoomError;

// ── Server struct ─────────────────────────────────────────────────────────────

pub struct LoomLspServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl LoomLspServer {
    pub fn new(client: Client) -> Self {
        LoomLspServer {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let diagnostics = match crate::compile(text) {
            Ok(_) => vec![],
            Err(errors) => errors
                .iter()
                .map(|e| loom_error_to_diagnostic(e, text))
                .collect(),
        };
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

// ── LanguageServer impl ───────────────────────────────────────────────────────

#[tower_lsp::async_trait]
impl LanguageServer for LoomLspServer {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "loom-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "loom-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents
                .write()
                .await
                .insert(uri.clone(), text.clone());
            self.publish_diagnostics(uri, &text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = &params
            .text_document_position_params
            .text_document
            .uri;
        let pos = &params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(text) = docs.get(uri) {
            let offset = position_to_byte_offset(text, pos.line, pos.character);
            if let Some(word) = extract_word_at(text, offset) {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(format!(
                        "`{}` — type: (inference in M4.1)",
                        word
                    ))),
                    range: None,
                }));
            }
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        _params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        Ok(None)
    }
}

// ── Public pure helpers ───────────────────────────────────────────────────────

/// Convert a [`LoomError`] to an LSP [`Diagnostic`].
pub fn loom_error_to_diagnostic(error: &LoomError, source: &str) -> Diagnostic {
    let span = error.span();
    let (start_line, start_char) = byte_offset_to_position_raw(source, span.start);
    let (end_line, end_char) = byte_offset_to_position_raw(source, span.end);
    Diagnostic {
        range: Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("loom".to_string()),
        message: error.to_string(),
        ..Default::default()
    }
}

/// Convert a byte offset to `(line, character)` (both 0-indexed).
///
/// Offsets beyond the end of `source` are clamped.
pub fn byte_offset_to_position_raw(source: &str, offset: usize) -> (u32, u32) {
    let clamped = offset.min(source.len());
    let prefix = &source[..clamped];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = prefix[last_newline..].chars().count() as u32;
    (line, character)
}

/// Convert an LSP position (line, character) to a byte offset in `source`.
fn position_to_byte_offset(source: &str, line: u32, character: u32) -> usize {
    let mut current_line = 0u32;
    let mut offset = 0;
    for ch in source.chars() {
        if current_line == line {
            break;
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset += ch.len_utf8();
    }
    for ch in source[offset..].chars().take(character as usize) {
        offset += ch.len_utf8();
    }
    offset
}

/// Return the identifier-like word that spans `offset`, or `None`.
fn extract_word_at(source: &str, offset: usize) -> Option<&str> {
    if offset > source.len() {
        return None;
    }
    let start = source[..offset]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| source[..=i].len())
        .unwrap_or(0);
    let end = source[offset..]
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| offset + i)
        .unwrap_or(source.len());
    let word = &source[start..end];
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}
