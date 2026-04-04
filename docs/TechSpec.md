# Tech Spec: loom

## Overview
[One paragraph translating PRD to technical approach]

## Architecture
### System Diagram
[Mermaid diagram or description of components]

### Tech Stack
- Runtime: Rust (stable, edition 2021) — see ADR-001
- Build: cargo 1.x
- Binary: `loom` (single static binary, no runtime dependencies)
- LSP: `loom-lsp` (tower-lsp, tokio async runtime)
- Framework: clap (CLI), serde/toml (project manifests), tower-lsp (LSP)

### Data Flow
[How data moves through the system]

## API Contracts
[Key endpoints, request/response shapes]

## Security & Compliance
[Auth approach, encryption, audit logging]

## Dependencies
[External services, APIs, libraries with version pins]

## Risks & Mitigations
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| | H/M/L | H/M/L | |
