# ADR-0003: Flat AST — No Scope Trees

**Date**: 2026-04-08  
**Status**: Accepted

## Context

Loom's AST must represent a module containing beings, types, functions,
ecosystems, and stores. A conventional compiler AST uses nested scope trees
(block → statement → expression). Loom's semantic model differs:

- Beings are not nested: they exist at module level, not inside each other.
- The type system is structural, not nominal. Scope is by module, not by block.
- Checker passes iterate over all beings/types/functions uniformly — a tree
  traversal framework would add complexity for no benefit.
- The primary compiler concern is cross-being relationships (M111 evolution
  vectors, M106 migration chains) not expression-level scope.

## Decision

`ast.rs` (split into `src/ast/` in a future pass) uses flat `Vec<T>` for all
top-level constructs. `Module` holds `Vec<Item>` at the top level; `BeingDef`
holds `Vec<MigrationBlock>`, `Vec<RegulateBlock>`, etc. No scope tree, no
parent pointers, no arena allocation.

Expressions retain a recursive `Expr` enum because expression evaluation is
inherently recursive (nested function calls, match arms, let bindings).

## Alternatives Considered

| Option | Reason rejected |
|---|---|
| **Arena-allocated scope tree** (salsa / rowan) | Adds a major dependency; incremental computation not yet needed; over-engineered for a batch compiler |
| **Nested modules** | Loom's current spec has one module per file; nesting adds semantic complexity with no benefit in the corpus |
| **Parent pointers in AST nodes** | Creates borrowing cycles in Rust; requires `Rc<RefCell<>>` or unsafe — not worth the tradeoff |

## Consequences

- **Simple checker passes**: each checker iterates `&module.being_defs` or
  `&module.items` with a simple `for` loop. No tree visitor framework needed.
- **No incremental compilation**: flat structures don't lend themselves to
  change-driven recomputation. Acceptable for now; revisit at production scale.
- **Cross-being checks are easy**: any checker that needs to compare two beings
  (M111: evolution vectors, M106: migration chains) can iterate all beings
  without traversal logic.
- **`BeingDef` grows with features**: every new being construct adds a field.
  Mitigated by `Option<T>` defaults and the `cognitive_memory: None` pattern.
  A future `ast/` split (ADR-0003a) will distribute the struct across files.
