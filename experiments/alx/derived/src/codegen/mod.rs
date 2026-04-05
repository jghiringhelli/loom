// ALX: derived from loom.loom §"Pipeline: Code generators"
// All emitters are infallible after checkers pass.

pub mod rust;
pub mod typescript;
pub mod openapi;
pub mod json_schema;
pub mod wasm;
pub mod simulation;
pub mod neuroml;

// G3: Re-export emitter structs so tests can write `loom::codegen::RustEmitter` etc.
pub use rust::RustEmitter;
pub use typescript::TypeScriptEmitter;
pub use openapi::OpenApiEmitter;
pub use json_schema::JsonSchemaEmitter;
pub use simulation::SimulationEmitter;
pub use neuroml::NeuroMLEmitter;

// G3: schema is an alias for json_schema — tests import `loom::codegen::schema::JsonSchemaEmitter`.
pub mod schema {
    pub use super::json_schema::JsonSchemaEmitter;
}
