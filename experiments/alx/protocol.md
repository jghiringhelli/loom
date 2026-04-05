# ALX Protocol

## Three-Phase Protocol for the Adversarial Loom eXperiment

Modelled on the RX (Replication eXperiment) protocol from the GS methodology.

---

## Phase 1 — Blind Derivation

**Context loaded:** ONLY these two files
- `docs/language-spec.md` (Loom language reference)
- `experiments/alx/spec/loom.loom` (self-specification)

**Context explicitly NOT loaded:**
- `src/` (any existing implementation)
- `tests/` (test files — the derivation must produce correct output without seeing what it will be tested against)
- `docs/publish/` (white papers — the derivation tests the spec, not the documentation)

**Instruction to AI:**
> "You are implementing the Loom compiler in Rust. Your only inputs are the language specification document (`docs/language-spec.md`) and the compiler self-specification (`experiments/alx/spec/loom.loom`). Derive the complete implementation. Write all source files to `experiments/alx/derived/src/`. Do not reference any implementation detail not derivable from these two documents."

**Output:** `experiments/alx/derived/src/` — a complete Rust crate

---

## Phase 2 — Verification

```powershell
$env:PATH = "$HOME\.cargo\bin;$env:PATH"

# Copy test suite (tests are the acceptance criteria, not the derivation target)
Copy-Item tests\* experiments\alx\derived\tests\ -Recurse
Copy-Item Cargo.toml experiments\alx\derived\Cargo.toml

cd experiments\alx\derived
cargo test --quiet 2>&1 | Tee-Object -FilePath ..\evidence\test-output.txt

# Count results
$passing = (Get-Content ..\evidence\test-output.txt | Select-String "test result: ok" | Measure-Object).Count
$failing = (Get-Content ..\evidence\test-output.txt | Select-String "FAILED" | Measure-Object).Count
$total = $passing + $failing

Write-Host "S_realized = $passing / $total = $([math]::Round($passing/$total, 3))"
"S_realized = $passing / $total = $([math]::Round($passing/$total, 3))" | Out-File ..\evidence\s-realized.txt
```

---

## Phase 3 — Gap Analysis

For each failing test, record in `evidence/correction-log.md`:

```markdown
## Correction {N}

**Test:** `test_name_here`
**Failure:** brief description of what the derivation got wrong
**Root cause:** which section of loom.loom was insufficient
**Spec improvement:** the line(s) added to loom.loom to close the gap
**Hypothesis:** why this gap existed (ambiguous spec / missing contract / underspecified behavior)
```

After each correction, re-run Phase 1 for the affected section and Phase 2 in full.

---

## Success Criteria

| S_realized | Status |
|-----------|--------|
| ≥ 0.95 | **Publication ready** — submit white paper, release crate |
| 0.90–0.94 | **Near complete** — close remaining gaps, re-run |
| 0.80–0.89 | **Significant gaps** — spec needs targeted improvement |
| < 0.80 | **Spec incomplete** — major sections need rewriting |

---

## Evidence to Commit

After a successful ALX run:
- `evidence/test-output.txt` — full cargo test output
- `evidence/s-realized.txt` — S_realized score
- `evidence/correction-log.md` — all gaps found and closed
- `evidence/derivation-summary.md` — summary of what the AI derived correctly on first pass vs. what required corrections

The committed evidence is the falsifiable proof. Anyone can reproduce it.
