//! Code-generation back-ends for the Loom compiler.
//!
//! Currently only the Rust emitter is implemented.  Additional back-ends
//! (WASM, JS, etc.) would each live in their own sub-module here.

pub mod rust;
pub mod wasm;

pub use rust::RustEmitter;
pub use wasm::WasmEmitter;
