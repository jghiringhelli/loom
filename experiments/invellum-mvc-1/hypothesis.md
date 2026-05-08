# Hypothesis — Invellum Meiosis MVC Experiment 1

## H1 — Open-domain meiosis is non-degenerate

The T5 evolution loop, when applied to a real product domain with externally-generated bug signals, produces **measurable fitness improvement** with **no safety floor violations**.

**Prediction:**
- `gen1 - gen0` overall fitness delta > 0
- No `safety.status: "VIOLATION"` at any oracle run during the deploy window
- All 3 bug issues closed via merged PRs

**Falsified if:**
- Fitness delta is zero or negative (mutations don't help)
- A floor is breached (mutations broke something measurable)
- Issues remain open after 14 days (the loop is too slow to be useful)

## H2 — Bundle vs. trickle is the better generation cadence

There are two ways to run this experiment:

**Bundle generation**: all 3 fixes merged in a single Railway deploy, treated as one gen1.
**Trickle generation**: each fix merged separately, each merge becomes its own generation (gen1, gen2, gen3).

H2 claims **bundle is the right call here** because:
- Each fix targets a different domain (`onboarding`, `feed_engagement`, `ui-styling`) — no interaction effects to disambiguate
- Trickle would require 3× the wait time (≥21 days vs ≥7 days) for the same statistical power
- The fitness oracle's signal is too noisy at small N to attribute deltas to a single mutation

**Falsified if:**
- Two fixes interact and one degrades a signal the other improves (we'd want to disambiguate)
- The oracle becomes much more sensitive (can attribute deltas at trickle granularity)

If H2 is falsified mid-experiment, the protocol switches to trickle for gen2+.

## H3 — GitHub issues are a sufficient async channel

The protocol depends on GitHub issues as the only communication layer between the meta-loop and the AI agent. H3 claims this is sufficient — i.e. an issue body with reproduction, code references, and a fix path is enough context for Claude Code (or whichever AI agent) to apply the correct mutation.

**Prediction:**
- AI sessions referencing only the issue body (not the meta-issue, not the experiment README) produce correct fixes for ≥2 of 3 issues on first PR
- Where re-work is needed, it is for taste/style reasons, not correctness

**Falsified if:**
- AI agents need additional out-of-band context (e.g. clarifications in PR review) for >50% of fixes
- The fix path described in the issue body proves wrong on contact with the code

If H3 is falsified, issue bodies need richer GS-style scaffolding (e.g. mandated `monitoring-spec.md` cross-references).

## Variables held constant

- The fitness oracle is **read-only** for this experiment. No mutations to it.
- Skok floors are **not negotiable**. A floor breach reverts the experiment, not the floor.
- Cialdini copy mutations are **out of scope**. Held at T2 until a `@credibility_safe` workflow is built.
- The CLAUDE.md protocol is **frozen** for the duration. Any drift gets tracked separately.

## Decision after results

| Result | Decision |
|--------|----------|
| H1 confirmed, no safety violations | Promote MVC Experiment 1 to canonical gen1; design Experiment 2 with more bugs |
| H1 confirmed, one near-miss on a floor | Tighten the safety check threshold; rerun gen0 baseline |
| H1 falsified, no violations | Investigate which step broke; oracle, agent, or protocol is the issue |
| H1 falsified, violation | Hard revert; treat as falsified protocol; write post-mortem before any continuation |
