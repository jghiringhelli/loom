//! Code-generation back-ends for the Loom compiler.
//!
//! Currently only the Rust emitter is implemented.  Additional back-ends
//! (WASM, JS, etc.) would each live in their own sub-module here.

pub mod openapi;
pub mod rust;
pub mod schema;
pub mod typescript;
pub mod wasm;

pub use openapi::OpenApiEmitter;
pub use rust::RustEmitter;
pub use schema::JsonSchemaEmitter;
pub use typescript::TypeScriptEmitter;
pub use wasm::WasmEmitter;
