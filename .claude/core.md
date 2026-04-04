# loom — Core

> Always loaded. Contains only what is true across all domains.
> Hard limit: 50 lines. If it grows, move the excess to a domain node.

## Domain Identity
loom — purpose not yet defined in spec.

## Tags
[UNIVERSAL] [CLI] [LIBRARY]

## Primary Entities
- It features:

- **Module system** with `provides` / `requires` interfaces for dependency injection
- **Product types** (`type Point = x: Float, y: Float end`)
- **Sum types** (`enum Color = | Red | Green | Blue end`)
- **Refined types** (`type Email = String where valid_email`)
- **Effect tracking** (`fn fetch :: Int -> Effect<[IO], User>`)
- **Design-by-contract** (`require:` / `ensure:` clauses → `debug_assert!`)
- **Pipe operator** (`a |> f |> g`)

## Install

```
cargo build --release
# binary at target/release/loom
```

## Usage

```
loom compile src/pricing.loom              # writes src/pricing.rs
loom compile src/pricing.loom -o out.rs   # custom output path
loom compile src/pricing.loom --check-only # type/effect check only
```

## Phase 1 status

| Feature | Status |
|---|---|
| Lexer (logos) | ✅ |
| Recursive-descent parser | ✅ |
| Type checker (symbol resolution) | ✅ |
| Effect checker (transitive effects) | ✅ |
| Rust code emitter | ✅ |
| CLI (`loom compile`) | ✅ |
| Full expression parser | ✅ |
| Corpus examples | ✅ |

Phase 2 will add: type inference, full pattern exhaustiveness checking,
WASM back-end, and language server support.

## Layer Map
```
[API/CLI] → [Services] → [Domain] → [Repositories] → [Infrastructure]
Dependencies point inward. Domain has zero external imports.
```

## Invariants
- Every public function has a JSDoc with typed params and returns
- No circular imports (enforced by pre-commit hook)
- Test coverage ≥80% on all changed files