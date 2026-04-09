//! M101: Manifest checker — Documentation Liveness Primitive.
//!
//! Verifies that documentation artifacts declared in `manifest:` blocks inside
//! `being:` declarations satisfy two rules:
//! 1. The declared file path must exist on the filesystem (error if absent).
//! 2. Every symbol listed in `reflects:` must be exported by the compilation
//!    unit (warning if not found — prefixed with `[warn]`).

use crate::ast::*;
use crate::error::LoomError;
use std::collections::HashSet;

/// Manifest checker — documentation liveness verification.
///
/// Iterates all `being:` declarations in the module and, for each `manifest:`
/// block, checks file existence and symbol availability.
pub struct ManifestChecker;

impl ManifestChecker {
    /// Create a new manifest checker.
    pub fn new() -> Self {
        ManifestChecker
    }

    /// Check all being declarations for manifest conformance.
    ///
    /// Returns accumulated errors. Missing files are hard errors; unknown
    /// symbols in `reflects:` are warnings (prefixed with `[warn]`).
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        let exported = self.collect_exported_names(module);
        for being in &module.being_defs {
            if let Some(manifest) = &being.manifest {
                self.check_manifest(manifest, &exported, being, &mut errors);
            }
        }
        errors
    }

    /// Collect all exported symbol names from the compilation unit.
    fn collect_exported_names(&self, module: &Module) -> HashSet<String> {
        let mut names = HashSet::new();
        for item in &module.items {
            match item {
                Item::Fn(f)           => { names.insert(f.name.clone()); }
                Item::Type(t)         => { names.insert(t.name.clone()); }
                Item::Enum(e)         => { names.insert(e.name.clone()); }
                Item::RefinedType(r)  => { names.insert(r.name.clone()); }
                Item::Session(s)      => { names.insert(s.name.clone()); }
                Item::Effect(e)       => { names.insert(e.name.clone()); }
                Item::Store(s)        => { names.insert(s.name.clone()); }
                Item::Proposition(p)  => { names.insert(p.name.clone()); }
                Item::Functor(f)      => { names.insert(f.name.clone()); }
                Item::Monad(m)        => { names.insert(m.name.clone()); }
                _ => {}
            }
        }
        for being in &module.being_defs {
            names.insert(being.name.clone());
        }
        for eco in &module.ecosystem_defs {
            names.insert(eco.name.clone());
        }
        names
    }

    fn check_manifest(
        &self,
        manifest: &ManifestBlock,
        exported: &HashSet<String>,
        being: &BeingDef,
        errors: &mut Vec<LoomError>,
    ) {
        for artifact in &manifest.artifacts {
            // Rule 1: file must exist on disk.
            if !std::path::Path::new(&artifact.path).exists() {
                errors.push(LoomError::parse(
                    format!(
                        "being '{}': manifest artifact '{}' does not exist on disk",
                        being.name, artifact.path
                    ),
                    manifest.span.clone(),
                ));
            }

            // Rule 2: symbols in reflects: must be in the exported set.
            // Symbols not found → warning (non-fatal).
            for symbol in &artifact.reflects {
                if !exported.contains(symbol) {
                    errors.push(LoomError::parse(
                        format!(
                            "[warn] being '{}': manifest artifact '{}' reflects unknown symbol '{}'",
                            being.name, artifact.path, symbol
                        ),
                        manifest.span.clone(),
                    ));
                }
            }
        }
    }
}
