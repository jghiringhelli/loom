//! Conservation law checker. M86.
//! Grounded in Noether's theorem (1915): every continuous symmetry corresponds
//! to a conservation law. @conserved(X) declares that quantity X is preserved
//! across the function boundary — the same measure appears in inputs and outputs.

use crate::ast::*;
use crate::error::LoomError;

/// Conservation quantity name mapping — used for name-based type matching.
const CONSERVATION_HINTS: &[(&str, &[&str])] = &[
    ("Mass",        &["Float", "mass", "Mass", "kg", "g", "mol"]),
    ("Charge",      &["Charge", "charge", "Float", "coulomb", "C"]),
    ("Energy",      &["Energy", "energy", "Float", "joule", "J", "eV"]),
    ("Momentum",    &["Momentum", "momentum", "Float", "kg_m_s"]),
    ("Value",       &["Float", "usd", "eur", "gbp", "Value", "Price", "Amount"]),
    ("Information", &["Bit", "bit", "Byte", "entropy", "Info"]),
    ("AtomCount",   &["atoms", "Atoms", "mol", "Float", "Int"]),
];

/// Conservation law checker.
///
/// For each function annotated `@conserved(X)`, verifies that the named
/// quantity appears in both input parameter types and the return type.
pub struct ConservationChecker;

impl ConservationChecker {
    /// Create a new conservation checker.
    pub fn new() -> Self {
        ConservationChecker
    }

    /// Check all functions in `module` for conservation law consistency.
    ///
    /// Returns errors/warnings for conservation violations.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut diagnostics = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut diagnostics);
            }
        }
        for being in &module.being_defs {
            if let Some(fb) = &being.function {
                for fd in &fb.fns {
                    self.check_fn(fd, &mut diagnostics);
                }
            }
        }
        diagnostics
    }

    fn check_fn(&self, fd: &FnDef, diagnostics: &mut Vec<LoomError>) {
        for ann in &fd.annotations {
            if ann.key != "conserved" || ann.value.is_empty() {
                continue;
            }
            let quantity = &ann.value;
            let hints = get_hints(quantity);

            let in_inputs = self.quantity_in_inputs(fd, hints)
                || self.quantity_in_contracts(&fd.requires, quantity);
            let in_outputs = self.quantity_in_output(fd, hints)
                || self.quantity_in_contracts(&fd.ensures, quantity);

            if !in_inputs && !in_outputs {
                diagnostics.push(LoomError::type_err(
                    format!(
                        "@conserved({}): quantity '{}' does not appear in parameter or return types \
                         — conservation cannot be verified",
                        quantity, quantity
                    ),
                    fd.span.clone(),
                ));
            } else if in_inputs && !in_outputs {
                diagnostics.push(LoomError::type_err(
                    format!(
                        "@conserved({}): quantity '{}' appears in parameters but not in return type \
                         — conservation may not hold",
                        quantity, quantity
                    ),
                    fd.span.clone(),
                ));
            } else if !in_inputs && in_outputs {
                diagnostics.push(LoomError::type_err(
                    format!(
                        "@conserved({}): quantity '{}' appears in return type but not in parameters \
                         — conservation may not hold",
                        quantity, quantity
                    ),
                    fd.span.clone(),
                ));
            }
        }
    }

    fn quantity_in_inputs(&self, fd: &FnDef, hints: &[&str]) -> bool {
        fd.type_sig.params.iter().any(|p| type_matches_any(p, hints))
    }

    fn quantity_in_output(&self, fd: &FnDef, hints: &[&str]) -> bool {
        type_matches_any(&fd.type_sig.return_type, hints)
    }

    fn quantity_in_contracts(&self, contracts: &[Contract], quantity: &str) -> bool {
        contracts.iter().any(|c| format!("{:?}", c.expr).contains(quantity))
    }
}

fn get_hints(quantity: &str) -> &'static [&'static str] {
    for (q, hints) in CONSERVATION_HINTS {
        if *q == quantity {
            return hints;
        }
    }
    &[]
}

fn type_matches_any(ty: &TypeExpr, hints: &[&str]) -> bool {
    match ty {
        TypeExpr::Base(name) => hints.iter().any(|h| name.contains(h) || h.contains(name.as_str())),
        TypeExpr::Generic(name, params) => {
            hints.iter().any(|h| name.contains(h) || h.contains(name.as_str()))
                || params.iter().any(|p| type_matches_any(p, hints))
        }
        TypeExpr::Effect(_, inner) => type_matches_any(inner, hints),
        TypeExpr::Option(inner) => type_matches_any(inner, hints),
        TypeExpr::Result(ok, err) => type_matches_any(ok, hints) || type_matches_any(err, hints),
        TypeExpr::Tuple(elems) => elems.iter().any(|e| type_matches_any(e, hints)),
        _ => false,
    }
}
