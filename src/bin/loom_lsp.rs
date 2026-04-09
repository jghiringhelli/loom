//! `loom-lsp` binary — stdio Language Server for Loom.
//!
//! Reads JSON-RPC messages from stdin and writes responses to stdout,
//! following the Language Server Protocol.

use loom::lsp::LoomLspServer;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| LoomLspServer::new(client)).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
