"""
LX-4 — Stateless Derivability Trial Runner
============================================
Calls the Anthropic API with a genuinely fresh context (no system prompt
from the current development session) and compiles the resulting Loom code.

Usage
-----
  ANTHROPIC_API_KEY=... LOOM_BIN=./target/release/loom python run_trials.py

For each of the 5 feature prompts:
  1. Loads fresh-session-prompt.md + feature-prompts/feature-N.md
  2. Calls claude-haiku-4-5 with NO system prompt (cold start)
  3. Extracts the first Loom code block from the response
  4. Writes the code to a temp .loom file
  5. Compiles it with the Loom binary
  6. Records result in evidence/trial-N.md
  7. Updates results.md

Exit code: 0 if >= 4/5 compile clean, 1 otherwise.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).parent
LOOM_BIN = Path(os.environ.get("LOOM_BIN", "./target/release/loom"))


# ---------------------------------------------------------------------------
# Anthropic API call — fresh context, no system prompt
# ---------------------------------------------------------------------------

def call_claude_fresh(prompt: str) -> str:
    """Call Claude with no system prompt — genuine cold-start context."""
    try:
        import anthropic
    except ImportError:
        raise RuntimeError("pip install anthropic")

    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])
    message = client.messages.create(
        model="claude-haiku-4-5",
        max_tokens=2048,
        # NO system prompt — cold start
        messages=[{"role": "user", "content": prompt}],
    )
    return message.content[0].text


# ---------------------------------------------------------------------------
# Loom code extraction
# ---------------------------------------------------------------------------

def extract_loom(response: str) -> str | None:
    """Extract first ```loom ... ``` block from response."""
    match = re.search(r"```loom\s*(.*?)```", response, re.DOTALL)
    if match:
        return match.group(1).strip()
    # fallback: try bare code block
    match = re.search(r"```\s*(module\s+\w+.*?)```", response, re.DOTALL)
    if match:
        return match.group(1).strip()
    return None


# ---------------------------------------------------------------------------
# Compile test
# ---------------------------------------------------------------------------

@dataclass
class CompileResult:
    success: bool
    output: str


def compile_loom(code: str) -> CompileResult:
    """Write code to temp file and compile with Loom binary."""
    with tempfile.NamedTemporaryFile(suffix=".loom", mode="w",
                                     delete=False, encoding="utf-8") as f:
        f.write(code)
        tmp = Path(f.name)

    try:
        result = subprocess.run(
            [str(LOOM_BIN), "compile", str(tmp)],
            capture_output=True,
            text=True,
            timeout=30,
        )
        output = result.stdout + result.stderr
        success = result.returncode == 0 and "LexError" not in output and "ParseError" not in output
        return CompileResult(success=success, output=output.strip())
    finally:
        tmp.unlink(missing_ok=True)
        # Also clean up generated .rs file
        rs = tmp.with_suffix(".rs")
        if rs.exists():
            rs.unlink()


# ---------------------------------------------------------------------------
# Evidence writing
# ---------------------------------------------------------------------------

def write_evidence(trial: int, feature_name: str, prompt: str,
                   response: str, extracted: str | None,
                   result: CompileResult | None, attempts: int) -> None:
    evidence_dir = HERE / "evidence"
    evidence_dir.mkdir(exist_ok=True)

    compile_status = "N/A (no Loom extracted)"
    if result is not None:
        compile_status = "PASS" if result.success else "FAIL"

    content = f"""# LX-4 Trial {trial} — {feature_name}

**Date:** 2026-04-12
**Model:** claude-haiku-4-5 (no system prompt)
**Result:** {compile_status} (attempts: {attempts})

## Prompt given

```
{prompt[:500]}...
```
(truncated — see feature-prompts/feature-{trial}.md for full prompt)

## LLM response (first 1000 chars)

```
{response[:1000]}
```

## Extracted Loom code

```loom
{extracted or '(none extracted)'}
```

## Compile output

```
{result.output if result else '(no compile attempted)'}
```
"""
    (evidence_dir / f"trial-{trial}.md").write_text(content, encoding="utf-8")


# ---------------------------------------------------------------------------
# Results summary
# ---------------------------------------------------------------------------

def update_results(trials: list[dict]) -> None:
    rows = []
    for t in trials:
        p1 = "✅" if t.get("pass_attempt_1") else "❌"
        p2 = "✅" if t.get("pass_attempt_2") else ("N/A" if t.get("pass_attempt_1") else "❌")
        rows.append(
            f"| {t['trial']} | {t['feature']} | {p1} | {p2} | {t.get('notes', '')} |"
        )

    table = "\n".join(rows)
    passes = sum(1 for t in trials if t.get("pass_attempt_1") or t.get("pass_attempt_2"))
    conclusion = (
        "LX-4 HYPOTHESIS CONFIRMED: Loom achieves stateless derivability (>=4/5)"
        if passes >= 4
        else f"LX-4 HYPOTHESIS NOT CONFIRMED: {passes}/5 passed (threshold: 4/5)"
    )

    content = f"""# LX-4 — Stateless Derivability Results

**Run date:** 2026-04-12
**Model:** claude-haiku-4-5 (cold-start, no system prompt)
**Status:** Complete

## Results Table

| Trial | Feature | Compiles 1st try | Compiles 2nd try | Notes |
|---|---|---|---|---|
{table}

**Score: {passes}/5**

## Conclusion

{conclusion}

## Evidence

See `evidence/trial-N.md` for full transcripts.
"""
    (HERE / "results.md").write_text(content, encoding="utf-8")
    print(f"\n{'='*50}")
    print(f"LX-4 score: {passes}/5")
    print(conclusion)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def load_prompt(n: int) -> str:
    prefix = (HERE / "fresh-session-prompt.md").read_text(encoding="utf-8")
    feature = (HERE / "feature-prompts" / f"feature-{n}.md").read_text(encoding="utf-8")
    return prefix + "\n" + feature


def run_trial(n: int, feature_name: str) -> dict:
    print(f"\n--- Trial {n}: {feature_name} ---")
    prompt = load_prompt(n)

    response = call_claude_fresh(prompt)
    print(f"  Response length: {len(response)} chars")

    extracted = extract_loom(response)
    if not extracted:
        print("  ❌ No Loom code block found in response")
        write_evidence(n, feature_name, prompt, response, None, None, 1)
        return {"trial": n, "feature": feature_name, "pass_attempt_1": False,
                "pass_attempt_2": False, "notes": "No Loom block extracted"}

    result = compile_loom(extracted)
    if result.success:
        print("  ✅ Compiles clean on first attempt")
        write_evidence(n, feature_name, prompt, response, extracted, result, 1)
        return {"trial": n, "feature": feature_name, "pass_attempt_1": True, "notes": ""}

    print(f"  ❌ First attempt failed. Errors: {result.output[:200]}")

    # Second attempt: ask the model to fix it
    fix_prompt = (
        f"The Loom code you provided has a compile error:\n\n"
        f"```\n{result.output[:500]}\n```\n\n"
        f"Please fix the Loom code. Return only the corrected Loom code block."
    )
    response2 = call_claude_fresh(prompt + "\n\nAssistant: " + response +
                                   "\n\nUser: " + fix_prompt)
    extracted2 = extract_loom(response2)
    if extracted2:
        result2 = compile_loom(extracted2)
        if result2.success:
            print("  ✅ Compiles clean on second attempt")
            write_evidence(n, feature_name, prompt, response + "\n---\n" + response2,
                           extracted2, result2, 2)
            return {"trial": n, "feature": feature_name, "pass_attempt_1": False,
                    "pass_attempt_2": True, "notes": "Fixed on 2nd attempt"}
        write_evidence(n, feature_name, prompt, response + "\n---\n" + response2,
                       extracted2, result2, 2)
        return {"trial": n, "feature": feature_name, "pass_attempt_1": False,
                "pass_attempt_2": False, "notes": result2.output[:100]}

    write_evidence(n, feature_name, prompt, response, extracted, result, 2)
    return {"trial": n, "feature": feature_name, "pass_attempt_1": False,
            "pass_attempt_2": False, "notes": "No Loom in 2nd response"}


FEATURES = [
    "Add require:/ensure: contracts",
    "Add lifecycle state + checkpoint",
    "Add regulate block to being",
    "Add niche_construction top-level",
    "Add canalize block",
]


def main() -> int:
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("ERROR: ANTHROPIC_API_KEY not set")
        print("Set it or run with: ANTHROPIC_API_KEY=... python run_trials.py")
        return 1

    if not LOOM_BIN.exists():
        print(f"ERROR: Loom binary not found at {LOOM_BIN}")
        print("Build with: cargo build --bin loom --release")
        return 1

    print("LX-4 Stateless Derivability Dry Runs")
    print(f"Model: claude-haiku-4-5 (cold-start)")
    print(f"Loom binary: {LOOM_BIN}")

    results = []
    for i, feature_name in enumerate(FEATURES, 1):
        results.append(run_trial(i, feature_name))

    update_results(results)

    passes = sum(1 for r in results if r.get("pass_attempt_1") or r.get("pass_attempt_2"))
    return 0 if passes >= 4 else 1


if __name__ == "__main__":
    sys.exit(main())
