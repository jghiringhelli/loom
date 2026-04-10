//! M118: Entity annotation coherence checker.
//!
//! Validates `entity<N, E>` declarations:
//! - `@directed` and `@undirected` are mutually exclusive — a graph cannot be
//!   both directed and undirected simultaneously.

use crate::ast::{Item, Module};
use crate::error::LoomError;
use crate::checker::LoomChecker;

pub struct EntityChecker;

impl EntityChecker {
    /// Construct a new [`EntityChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `entity` declarations in the module.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Entity(ent) = item {
                let has_directed = ent.annotations.iter().any(|a| a == "directed");
                let has_undirected = ent.annotations.iter().any(|a| a == "undirected");
                if has_directed && has_undirected {
                    errors.push(LoomError::type_err(
                        format!(
                            "entity '{}' declares both @directed and @undirected — \
                             a graph cannot be both simultaneously",
                            ent.name
                        ),
                        ent.span.clone(),
                    ));
                }
            }
        }
        errors
    }
}

impl LoomChecker for EntityChecker {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        self.check(module)
    }
}

impl Default for EntityChecker {
    fn default() -> Self {
        Self::new()
    }
}
