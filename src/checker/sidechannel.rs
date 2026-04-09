//! Side-channel information flow checker for the Loom compiler. M62.
use crate::ast::*;
use crate::error::LoomError;

pub struct SideChannelChecker;

impl SideChannelChecker {
    pub fn new() -> Self {
        SideChannelChecker
    }

    pub fn check(&self, module: &Module) -> Result<(), Vec<LoomError>> {
        let mut errors = Vec::new();
        for item in &module.items {
            if let Item::Fn(fd) = item {
                self.check_fn(fd, &mut errors);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_fn(&self, fd: &FnDef, errors: &mut Vec<LoomError>) {
        let has_timing_safe = fd
            .annotations
            .iter()
            .any(|a| a.key == "timing-safe" || a.key == "timing_safe");
        if has_timing_safe && fd.timing_safety.is_none() {
            errors.push(LoomError::TypeError {
                msg: format!(
                    "side-channel: function `{}` is annotated `@timing-safe` but lacks a `timing_safety:` block",
                    fd.name
                ),
                span: fd.span.clone(),
            });
        }
        if let Some(ts) = &fd.timing_safety {
            if ts.constant_time {
                if let Some(leaks) = &ts.leaks_bits {
                    let leaks_norm = leaks.trim();
                    let is_zero = leaks_norm == "0"
                        || leaks_norm == "0.0"
                        || leaks_norm == "0.0 bits"
                        || leaks_norm == "0 bits";
                    if !is_zero {
                        errors.push(LoomError::TypeError {
                            msg: format!(
                                "side-channel: function `{}` declares `constant_time: true` but \
                                 `leaks_bits: {}` is not zero — a constant-time function must leak 0 bits",
                                fd.name, leaks
                            ),
                            span: ts.span.clone(),
                        });
                    }
                }
            }
        }
    }
}
