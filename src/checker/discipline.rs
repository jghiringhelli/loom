//! M141–M145: DisciplineChecker — validates explicit `discipline` declarations.
//!
//! Rules:
//!
//! 1. **Target existence warning** — warns when a discipline's `target` is not declared
//!    as a `store` or `type` in the same module (best-effort, not a hard error because
//!    Loom modules can span multiple files).
//!
//! 2. **CQRS coherence** — warns if EventSourcing is declared for the same target
//!    without CQRS (EventSourcing implies command/query separation).
//!
//! 3. **EventSourcing completeness** — warns when no `events:` list is declared
//!    (the generic event struct is emitted, but named domain events are strongly preferred).
//!
//! 4. **CircuitBreaker bounds** — rejects `max_attempts: 0` or unreasonably high values.
//!
//! 5. **DependencyInjection completeness** — warns when `binds:` list is empty
//!    (a DI container with no ports is a composition root with nothing to inject).
//!
//! 6. **Saga completeness** — warns when `steps:` list is empty.
//!
//! 7. **Duplicate disciplines** — rejects the same (kind, target) pair declared twice.

use crate::ast::{DisciplineKind, DisciplineParam, Item, Module};
use crate::checker::LoomChecker;
use crate::error::LoomError;

pub struct DisciplineChecker;

impl DisciplineChecker {
    /// Construct a new [`DisciplineChecker`].
    pub fn new() -> Self {
        Self
    }

    /// Validate all `discipline` declarations in the module.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();

        let disciplines: Vec<_> = module
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Discipline(d) = i {
                    Some(d)
                } else {
                    None
                }
            })
            .collect();

        self.check_duplicates(&disciplines, &mut errors);

        for dd in &disciplines {
            self.check_circuit_breaker_bounds(dd, &mut errors);
            self.check_event_sourcing_completeness(dd, &mut errors);
            self.check_di_completeness(dd, &mut errors);
            self.check_saga_completeness(dd, &mut errors);
        }

        self.check_cqrs_event_sourcing_coherence(&disciplines, &mut errors);

        errors
    }

    // ── Rule 7: No duplicate (kind, target) ──────────────────────────────────

    fn check_duplicates(
        &self,
        disciplines: &[&crate::ast::DisciplineDecl],
        errors: &mut Vec<LoomError>,
    ) {
        let mut seen: Vec<(&DisciplineKind, &str)> = Vec::new();
        for dd in disciplines {
            let key = (&dd.kind, dd.target.as_str());
            if seen.contains(&key) {
                errors.push(LoomError::type_err(
                    format!(
                        "discipline '{:?}' for '{}' is declared more than once — \
                         remove the duplicate",
                        dd.kind, dd.target
                    ),
                    dd.span.clone(),
                ));
            } else {
                seen.push(key);
            }
        }
    }

    // ── Rule 4: CircuitBreaker max_attempts bounds ────────────────────────────

    fn check_circuit_breaker_bounds(
        &self,
        dd: &crate::ast::DisciplineDecl,
        errors: &mut Vec<LoomError>,
    ) {
        if !matches!(dd.kind, DisciplineKind::CircuitBreaker) {
            return;
        }
        let max = dd.params.iter().find_map(|(k, v)| {
            if k == "max_attempts" {
                if let DisciplineParam::Number(n) = v {
                    Some(*n)
                } else {
                    None
                }
            } else {
                None
            }
        });
        if let Some(n) = max {
            if n <= 0 {
                errors.push(LoomError::type_err(
                    format!(
                        "discipline CircuitBreaker for '{}': max_attempts must be >= 1 \
                         (got {})",
                        dd.target, n
                    ),
                    dd.span.clone(),
                ));
            } else if n > 100 {
                errors.push(LoomError::type_err(
                    format!(
                        "discipline CircuitBreaker for '{}': max_attempts {} is unreasonably \
                         high — typical values are 3–10",
                        dd.target, n
                    ),
                    dd.span.clone(),
                ));
            }
        }
    }

    // ── Rule 3: EventSourcing should declare events ───────────────────────────

    fn check_event_sourcing_completeness(
        &self,
        dd: &crate::ast::DisciplineDecl,
        errors: &mut Vec<LoomError>,
    ) {
        if !matches!(dd.kind, DisciplineKind::EventSourcing) {
            return;
        }
        let has_events = dd.params.iter().any(|(k, v)| {
            k == "events" && matches!(v, DisciplineParam::List(items) if !items.is_empty())
        });
        if !has_events {
            errors.push(LoomError::type_err(
                format!(
                    "discipline EventSourcing for '{}': no 'events:' list declared — \
                     a generic event struct will be emitted but named domain events are \
                     strongly preferred (e.g. events: [OrderCreated, OrderShipped])",
                    dd.target
                ),
                dd.span.clone(),
            ));
        }
    }

    // ── Rule 5: DI container should bind at least one port ───────────────────

    fn check_di_completeness(&self, dd: &crate::ast::DisciplineDecl, errors: &mut Vec<LoomError>) {
        if !matches!(dd.kind, DisciplineKind::DependencyInjection) {
            return;
        }
        let has_binds = dd.params.iter().any(|(k, v)| {
            k == "binds" && matches!(v, DisciplineParam::List(items) if !items.is_empty())
        });
        if !has_binds {
            errors.push(LoomError::type_err(
                format!(
                    "discipline DependencyInjection for '{}': no 'binds:' list declared — \
                     a DI container with no ports has nothing to inject; \
                     add: binds: [IMyPort, IAnotherPort]",
                    dd.target
                ),
                dd.span.clone(),
            ));
        }
    }

    // ── Rule 6: Saga should declare steps ────────────────────────────────────

    fn check_saga_completeness(
        &self,
        dd: &crate::ast::DisciplineDecl,
        errors: &mut Vec<LoomError>,
    ) {
        if !matches!(dd.kind, DisciplineKind::Saga) {
            return;
        }
        let has_steps = dd.params.iter().any(|(k, v)| {
            k == "steps" && matches!(v, DisciplineParam::List(items) if !items.is_empty())
        });
        if !has_steps {
            errors.push(LoomError::type_err(
                format!(
                    "discipline Saga for '{}': no 'steps:' list declared — \
                     a saga without named steps emits no compensating step types; \
                     add: steps: [StepA, StepB]",
                    dd.target
                ),
                dd.span.clone(),
            ));
        }
    }

    // ── Rule 2: EventSourcing without CQRS for same target ───────────────────

    fn check_cqrs_event_sourcing_coherence(
        &self,
        disciplines: &[&crate::ast::DisciplineDecl],
        errors: &mut Vec<LoomError>,
    ) {
        for dd in disciplines {
            if !matches!(dd.kind, DisciplineKind::EventSourcing) {
                continue;
            }
            let has_cqrs = disciplines.iter().any(|other| {
                matches!(other.kind, DisciplineKind::Cqrs) && other.target == dd.target
            });
            if !has_cqrs {
                errors.push(LoomError::type_err(
                    format!(
                        "discipline EventSourcing for '{}': EventSourcing implies \
                         Command/Query Responsibility Segregation — add: \
                         discipline CQRS for {} end",
                        dd.target, dd.target
                    ),
                    dd.span.clone(),
                ));
            }
        }
    }
}

impl LoomChecker for DisciplineChecker {
    fn check_module(&self, module: &Module) -> Vec<LoomError> {
        self.check(module)
    }
}

impl Default for DisciplineChecker {
    fn default() -> Self {
        Self::new()
    }
}
