# Security Policy

Loom is a pre-1.0 research compiler. It has not been audited and must not be used to enforce security
boundaries in production.

## Reporting a vulnerability

Please report privately through GitHub's "Report a vulnerability" (Security tab of
`github.com/jghiringhelli/loom`) rather than a public issue. Include a minimal `.loom` input or command,
the observed behaviour and the commit SHA. Expect an acknowledgement within 7 days; this is a single-maintainer
project, so fixes are best effort.

## In scope
- Panics, hangs or unbounded memory on crafted input to the lexer, parser or compiler (`loom compile`).
- Checkers that accept programs they document as rejected (soundness gaps).
- Emitted code that silently drops a declared contract, privacy label or safety annotation.
- The BIOISO runtime applying a patch that bypasses its declared gates.

## Out of scope
- Properties Loom does not claim to prove (for example `@exactly-once` against external side effects is
  checked only as a local model, not proved).
- Vulnerabilities in third-party dependencies with no Loom-specific exploit path (report upstream).
