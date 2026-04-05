# Publication Plan

## URL Ecosystem

The full public presence across domains:

| Domain | Purpose | Status |
|---|---|---|
| `genspec.dev` | GS theory community home — Jekyll, GitHub Pages | ✅ Live |
| `zenodo.org` | White paper DOI: `10.5281/zenodo.19073543` | ✅ Published |
| `bioiso.dev` | Biological isomorphisms + Loom + synthetic beings — Astro site | 🆕 Registered |
| `bioiso.org` | Movement/community side of BIOISO | 🆕 Registered |
| `pragmaworks.dev` | Company home | ✅ Live |
| ForgeCraft | `github.com/jghiringhelli/forgecraft-mcp` | ✅ Live |
| GitHub | `github.com/pragmaworks/loom` | ✅ Live |

**bioiso.dev links to genspec.dev** for GS theory — does not duplicate it.
**WASM playground** lives at `bioiso.dev/playground` (or `try.bioiso.dev`).

---

## Precondition: Run the ALX

Before any public release, run `experiments/alx/` to establish self-applicability.
The ALX result is the empirical anchor for every publication claim.

**Gate:** S_realized ≥ 0.90 before proceeding.

---

## Step 1 — crates.io Release

```toml
# Cargo.toml additions
[package]
name = "loom-lang"
version = "0.1.0"
description = "AI-native functional language transpiling to Rust, TypeScript, WASM, OpenAPI 3.0, JSON Schema"
repository = "https://github.com/pragmaworks/loom"
license = "MIT OR Apache-2.0"
keywords = ["language", "compiler", "transpiler", "ai", "specification"]
categories = ["compilers", "development-tools"]
```

```powershell
cargo publish
```

Installation: `cargo add loom-lang`

---

## Step 2 — WASM Playground

Build the compiler to WASM so it runs in the browser. Users paste Loom source, see Rust + TypeScript + OpenAPI + JSON Schema side-by-side instantly.

```toml
[lib]
crate-type = ["cdylib"]

[features]
wasm = ["wasm-bindgen", "console_error_panic_hook"]
```

Host at `bioiso.dev/playground` (or `try.bioiso.dev`). The playground is the fastest path from "heard about it" to "I understand it."

---

## Step 3 — arXiv Submission

Submit `docs/publish/white-paper.md` to arXiv:
- **Primary:** cs.PL (Programming Languages)
- **Cross-list:** cs.AI, cs.LO (Logic in Computer Science)
- **Title:** *Loom: Materialising Academic Semantic Specifications as First-Class Language Constructs*
- **Abstract:** Already written. Update test count and include ALX S_realized score.

arXiv gives a citable DOI before peer review. This is the anchor for all other references.

---

## Step 4 — Article Publication

Publish `docs/publish/article.md` to:
1. **Dev.to** — developer audience, high engagement for PL content
2. **Hacker News** — "Show HN: Loom — a language that ships information flow types, units of measure, typestate, and telos as first-class constructs"
3. **Medium / Substack** — longer-form audience
4. **r/rust** — Rust community will care about the Rust emission target specifically

Lead with the Mars Orbiter. Link to the playground. Link to the arXiv paper.

---

## Step 5 — VS Code Extension

Syntax highlighting + inline checker errors via the LSP module (`src/lsp/`).

```json
{
  "name": "loom-lang",
  "displayName": "Loom",
  "description": "AI-native specification language",
  "contributes": {
    "languages": [{ "id": "loom", "extensions": [".loom"] }],
    "grammars": [{ "language": "loom", "scopeName": "source.loom" }]
  }
}
```

Publish to VS Code Marketplace. This is the "daily driver" surface — where practitioners actually write Loom.

---

## Step 6 — ForgeCraft Integration

ForgeCraft is the transition tool; Loom is the destination (GS white paper, §4). The integration:

- ForgeCraft `setup_project` generates a `spec.loom` instead of (or alongside) `CLAUDE.md`
- The Loom spec IS the architectural constitution — type-checked, not prose
- ForgeCraft's governance gates become Loom compiler gates
- ADRs become `@decision` annotations in the Loom source

This is the moment the GS white paper's claim becomes structurally true:
*"Its gates become compile errors, its ADRs become verified contracts, its commit hooks become type checker passes."*

---

## The Publishing Order

```
ALX (S ≥ 0.90)
    ↓
crates.io v0.1.0
    ↓
WASM playground live
    ↓
arXiv preprint
    ↓
Article → Dev.to → HN → Show HN
    ↓
VS Code extension
    ↓
ForgeCraft integration
```

Each step feeds the next. The playground makes the article credible. The article drives GitHub stars. Stars make the crate trustworthy. The crate enables the ForgeCraft integration.
