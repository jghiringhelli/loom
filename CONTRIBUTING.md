# Contributing to Loom

Thank you for contributing to Loom! This document describes how to submit issues, propose features, and open pull requests.

---

## Code of Conduct

Loom follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be kind and constructive.

---

## Ways to contribute

- **Bug reports** — open an issue with a minimal reproducible Loom program
- **Language features** — open a discussion first; new syntax affects the parser, checker, and all emission targets
- **New emission targets** — LLVM IR, C, Python, Lean4, Coq are all welcome
- **Verification pipeline** — Kani, Prusti, Dafny, TLA+ integrations
- **Standard library modules** — add domains to `src/stdlib/`
- **Examples and tutorials** — add `.loom` files to `examples/`
- **Editor extensions** — VS Code, Neovim, Zed via the built-in LSP (`loom lsp`)

---

## Development setup

```sh
git clone https://github.com/pragmaworks/loom
cd loom
cargo build
cargo test -- --test-threads=1
```

All 904 tests must pass. Run the full suite before opening a PR.

---

## Branch workflow

- `main` — protected; requires PR + passing CI
- Feature branches: `feat/short-description`
- Bug branches: `fix/short-description`
- Docs branches: `docs/short-description`

**Never commit directly to main.**

---

## Commit convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(parser): add support for dependent type bounds
fix(codegen): emit correct float literal for ensure clauses
docs(readme): update milestone count to 116
test(lpn): add property tests for milestone range expansion
refactor(codegen): split rust.rs into domain files
```

Scopes: `parser`, `checker`, `codegen`, `lexer`, `ast`, `stdlib`, `lpn`, `lsp`, `alx`, `cli`, `docs`.

---

## Testing requirements

Every PR must:
- Add tests for any new behaviour (test file in `tests/`)
- Keep all 904 existing tests green
- Follow the naming convention: `test_rejects_<bad_input>` / `test_emits_<expected_output>`
- Cover the adversarial surface: contracts on invalid inputs, effect mismatches, privacy violations

Run only the affected test file during development:

```sh
cargo test --test lpn_test -- --test-threads=1
cargo test --test codegen_test -- --test-threads=1
```

---

## Adding a new milestone

1. Add the milestone to [`docs/roadmap.md`](docs/roadmap.md) with status `Declared`
2. Write the test (RED phase — must fail)
3. Implement the feature (GREEN phase)
4. Update milestone status to ✅ in `roadmap.md`
5. Update `README.md` milestone count

---

## Adding a new emission target

1. Add a file `src/codegen/<target>.rs`
2. Add a public function `compile_<target>(src: &str) -> Result<String, LoomError>`
3. Register it in `src/lib.rs`
4. Add a CLI flag in `src/main.rs`
5. Add tests in `tests/<target>_test.rs`
6. Add an example in `examples/`

---

## Style

- No `unsafe` without a comment explaining why and what invariant it relies on
- No `unwrap()` or `expect()` in library code — return `Result` or `Option`
- All public functions have JSDoc-style `///` comments
- Maximum function length: 50 lines. If a function does two things, split it.
- `cargo clippy -- -D warnings` must pass

---

## PR checklist

Before opening a PR, verify:

- [ ] `cargo build` succeeds with no errors
- [ ] `cargo test -- --test-threads=1` passes (all 904+)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] New tests added for new behaviour
- [ ] `README.md` updated if milestone count changed
- [ ] `docs/roadmap.md` updated if a milestone was completed
- [ ] Commit messages follow Conventional Commits

---

## Questions

Open a [GitHub Discussion](https://github.com/pragmaworks/loom/discussions) for design questions, or an [Issue](https://github.com/pragmaworks/loom/issues) for bugs.
