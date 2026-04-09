//! LSP integration tests for M4.
//!
//! Tests the pure utility functions (position conversion, diagnostic mapping)
//! and verifies the server can be instantiated.

use loom::lsp::{byte_offset_to_position_raw, loom_error_to_diagnostic};
use loom::LoomError;
use loom::ast::Span;

// ── Position conversion ───────────────────────────────────────────────────────

#[test]
fn offset_on_first_line_returns_line_zero() {
    let src = "module M end";
    let (line, ch) = byte_offset_to_position_raw(src, 7);
    assert_eq!(line, 0);
    assert_eq!(ch, 7);
}

#[test]
fn offset_at_start_of_second_line() {
    let src = "module M\nend";
    // 'e' in 'end' is at byte offset 9 (0-indexed)
    let (line, ch) = byte_offset_to_position_raw(src, 9);
    assert_eq!(line, 1);
    assert_eq!(ch, 0);
}

#[test]
fn offset_in_middle_of_second_line() {
    let src = "module M\nend";
    // 'n' in 'end' is at byte offset 10
    let (line, ch) = byte_offset_to_position_raw(src, 10);
    assert_eq!(line, 1);
    assert_eq!(ch, 1);
}

#[test]
fn offset_at_start_of_source_returns_zero_zero() {
    let (line, ch) = byte_offset_to_position_raw("hello", 0);
    assert_eq!(line, 0);
    assert_eq!(ch, 0);
}

#[test]
fn offset_beyond_end_is_clamped_to_last_position() {
    let src = "hello";
    let (line, ch) = byte_offset_to_position_raw(src, 100);
    assert_eq!(line, 0);
    assert_eq!(ch, 5);
}

// ── Diagnostic conversion ─────────────────────────────────────────────────────

#[test]
fn lex_error_produces_error_severity_diagnostic() {
    use tower_lsp::lsp_types::DiagnosticSeverity;

    let src = "module M end";
    let error = LoomError::LexError {
        msg: "unexpected character".to_string(),
        span: Span::new(7, 8),
    };
    let diag = loom_error_to_diagnostic(&error, src);
    assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    assert!(diag.message.contains("unexpected character"));
    assert_eq!(diag.range.start.line, 0);
    assert_eq!(diag.range.start.character, 7);
}

#[test]
fn multiline_span_maps_to_correct_range() {
    use tower_lsp::lsp_types::DiagnosticSeverity;

    // Error starts at "end" on line 1, col 0 (byte offset 9) and ends at col 3
    let src = "module M\nend";
    let error = LoomError::ParseError {
        msg: "bad token".to_string(),
        span: Span::new(9, 12),
    };
    let diag = loom_error_to_diagnostic(&error, src);
    assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(diag.range.start.line, 1);
    assert_eq!(diag.range.start.character, 0);
    assert_eq!(diag.range.end.line, 1);
    assert_eq!(diag.range.end.character, 3);
}

#[test]
fn diagnostic_source_is_loom() {
    let src = "x";
    let error = LoomError::TypeError {
        msg: "type error".to_string(),
        span: Span::new(0, 1),
    };
    let diag = loom_error_to_diagnostic(&error, src);
    assert_eq!(diag.source, Some("loom".to_string()));
}

// ── Server instantiation ──────────────────────────────────────────────────────

#[tokio::test]
async fn server_can_be_created() {
    use tower_lsp::LspService;
    use loom::lsp::LoomLspServer;

    let (service, socket) = LspService::build(|client| LoomLspServer::new(client)).finish();
    // Services are created lazily — just verify construction doesn't panic.
    drop(socket);
    drop(service);
}
