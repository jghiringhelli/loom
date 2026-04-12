# LX-4 — Stateless Derivability: Reproducibility Protocol

## What is this experiment?

LX-4 tests whether Loom achieves the "stateless derivability" property:

> A fresh LLM session given only `docs/language-spec.md` + a `.loom` file
> can implement a correct feature with **zero external context** from the
> current development history.

This is the strongest version of the PLN claim: the specification artifacts alone
are sufficient for a cold-start AI to produce correct Loom code.

## Why this cannot run from within the current session

A cold-start must be **genuinely cold**:
- No prior chat history with Loom
- No memory of prior implementation sessions
- Only artifacts readable from the repo itself

Running it from within this session would be circular — the AI already knows
the Loom grammar from hundreds of prior turns.

## How to run LX-4 (instructions for operator)

### Prerequisites
1. Loom repo at a tagged version (e.g., `v0.10.0`)
2. Access to a new LLM session (e.g., GitHub Copilot chat, fresh Claude, GPT-4)

### Step 1: Load the language spec
Give the fresh session these two files ONLY:
```
docs/language-spec.md        (or docs/pln.md if spec is there)
experiments/lx/LX-4-stateless-derivability/fresh-session-prompt.md
```

Do NOT give it:
- `src/` code
- `CLAUDE.md` / Copilot instructions
- Any prior conversation history
- `Status.md`

### Step 2: Ask it to implement one of the 5 features

Use the exact prompts in `feature-prompts/feature-N.md` (N=1..5).
Each prompt contains a starting Loom file + the feature request.

### Step 3: Compile the result

```sh
cargo run --bin loom -- compile <output.loom>
```

Record:
- Compile: PASS / FAIL
- Errors (if any): paste exact output
- Attempts needed: 1 (first try) / 2 (one correction) / FAIL (gave up)

### Step 4: Record in evidence/

Create `evidence/trial-N.md` with the transcript, the emitted Loom, and the result.

## Success criteria

- 4/5 trials compile clean on first or second attempt
- 5/5 trials show the LLM understood the correct Loom syntax from spec alone

## Artifacts in this directory

```
fresh-session-prompt.md     — Instructions to give the cold-start LLM
feature-prompts/
  feature-1.md              — Add @conserved annotation
  feature-2.md              — Add lifecycle state
  feature-3.md              — Add regulate block
  feature-4.md              — Add flow secret label
  feature-5.md              — Add require: contract
evidence/
  trial-1.md                — (to be filled when run)
  trial-2.md
  trial-3.md
  trial-4.md
  trial-5.md
results.md                  — Summary table (to be filled)
```
