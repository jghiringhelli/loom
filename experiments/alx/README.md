# ALX: Adversarial Loom eXperiment

> *The discipline is expressive enough to govern its own construction.*
> — GS White Paper, §7.7 (Self-Applicability)

## What This Is

The ALX is the self-applicability proof for Loom: the compiler is specified in its own language, then a stateless AI reader derives the Rust implementation from that specification alone.

If the derivation passes the test suite → Loom is self-consistent, publication-ready, and the convergence claim of the GS white paper holds at the language level.

The AX experiment proved GS self-applicability for ForgeCraft (a natural-language specification tool). ALX proves it for Loom (a formal specification language). The difference: ALX operates at a strictly higher tier — the spec is type-checked, not prose-governed.

## The Claim

`experiments/alx/spec/loom.loom` contains the complete specification of the Loom compiler in Loom syntax. A stateless AI assistant, given only this file and the Loom language reference (`docs/language-spec.md`), must be able to derive a correct Rust implementation.

**Success criterion:** S_realized ≥ 0.90 — 90%+ of the test suite passes from a blind derivation, with no human correction of individual implementation decisions.

## Relationship to $I \propto (1-S)/S$

The completeness hypothesis states that correction iterations are a decreasing function of specification completeness. The ALX instruments the right-hand side:

- `S` = the completeness of `loom.loom` as measured by how many tests pass from blind derivation
- `I` = correction iterations required to reach a passing suite
- At S → 1: I → 0 (one-shot derivation)
- At S < 1: gaps in the spec produce gaps in the derivation, requiring targeted corrections

Each correction becomes a spec improvement. The series converges. The git history of this experiment is the empirical record.

## Artifacts

| File | Purpose |
|------|---------|
| `spec/loom.loom` | The self-specification — Loom compiler in Loom syntax |
| `runner/run.ps1` | Execution script — runs the three-phase ALX protocol |
| `evidence/` | Generated: test output, S_realized score, correction log |
| `protocol.md` | The three-phase protocol (analogous to RX's runner protocol) |

## Three-Phase Protocol

### Phase 1 — Blind Derivation
1. Load fresh context: `docs/language-spec.md` + `experiments/alx/spec/loom.loom` only
2. Do not load any existing `.rs` files from `src/`
3. Instruct AI: *"Derive the complete Rust implementation of this compiler from the specification above. Produce all files in `src/`. Do not reference any implementation that is not derivable from the spec."*
4. Write output to `experiments/alx/derived/src/`

### Phase 2 — Verification
```powershell
$env:PATH = "$HOME\.cargo\bin;$env:PATH"
cd experiments/alx/derived
cargo test --quiet 2>&1 | Tee-Object -FilePath ..\evidence\test-output.txt
```
Count passing/failing. Record S_realized.

### Phase 3 — Gap Analysis
For each failing test:
1. Identify which spec section was insufficient to derive the correct behavior
2. Add a one-line improvement to `spec/loom.loom` that closes the gap
3. Record the correction in `evidence/correction-log.md`
4. Repeat from Phase 1 for the affected section only

## Running the ALX

```powershell
# From the repo root
.\experiments\alx\runner\run.ps1
```

The script checks for a fresh context (no cached derivation), runs Phase 1 via Claude Code, then Phase 2 automatically. Phase 3 is guided interactively.

## Reproducibility

Any reader can reproduce ALX by:
1. Cloning this repository
2. Running `.\experiments\alx\runner\run.ps1` with a valid Anthropic API key
3. Comparing their `evidence/test-output.txt` to the committed result

The committed evidence is at `experiments/alx/evidence/`. The spec is at `experiments/alx/spec/loom.loom`. The derivation is deterministic given the spec; variance between runs measures spec completeness, not model variance.

## Connection to Publication

The ALX result is the empirical anchor for the publication claim. The Loom white paper claims self-applicability. The ALX is the falsifiable proof:

- If ALX passes → Loom is ready for arXiv submission + crates.io release
- If ALX reveals gaps → those gaps are specification improvements, not implementation bugs
- The correction log becomes the appendix of the published white paper

---

*This experiment was designed following the AX/RX methodology described in the GS White Paper (Ghiringhelli, 2026, §7.7). The ALX is the next rung: AX proved GS self-applicability in natural language. ALX proves it in formal specification.*
