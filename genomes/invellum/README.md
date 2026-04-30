# Invellum BIOISO Genome Lineage

Each subdirectory `gen{N}/` is one generation boundary — one Railway deploy.

## What a generation is

A generation begins at a Railway deploy and ends at the next deploy.
Between deploys, signals accumulate in the live Postgres database.
At the next deploy boundary, the fitness oracle is run against the accumulated signal window,
the delta is recorded here, and the mutation that caused it is tagged.

## Manifest format (`gen{N}/manifest.json`)

```json
{
  "generation": N,
  "timestamp": "ISO-8601 of when this manifest was written",
  "railway_deploy_id": "Railway deploy ID or null",
  "origin": "human | bioiso",
  "fitness_before": { ... snapshot from oracle run BEFORE this generation's changes ... },
  "fitness_after":  { ... snapshot from oracle run AFTER 7+ days of signal accumulation ... },
  "mutations": [
    {
      "domain": "onboarding | retention | feed_engagement | network_growth | conversion",
      "type": "ParameterAdjust | StructuralRewire | human",
      "description": "what changed",
      "commit": "git SHA",
      "signal_target": "which signal was the primary target",
      "result": "improved | degraded | neutral | pending"
    }
  ],
  "notes": "free text — why this generation, what was the hypothesis"
}
```

## Commit convention

- Human-initiated changes: normal commits (no prefix needed)
- BIOISO-proposed changes: prefix commit message with `[bioiso gen:N domain:X]`
  - Example: `[bioiso gen:3 domain:onboarding] param: move discover before connect in onboarding step order`

## Running the oracle

```sh
cd invellum/invellum-backend
DATABASE_URL=<staging or local db url> npx ts-node scripts/bioiso-fitness-oracle.ts
DATABASE_URL=<url> npx ts-node scripts/bioiso-fitness-oracle.ts --days 7 --out fitness.json
```

## Hard stops (never override)

Per the fitness oracle doc (docs/invellum-fitness-oracle.md §1.4):
- d1_retention must stay ≥ 0.55
- d7_retention must stay ≥ 0.25
- monthly_churn must stay ≤ 0.08

Any mutation that degrades these is rejected regardless of T-tier.

## Generation log

| Gen | Date       | Origin  | Primary domain   | Primary signal       | Δ score | Notes                      |
|-----|------------|---------|------------------|----------------------|---------|----------------------------|
| 0   | 2026-04-28 | human   | —                | baseline             | —       | Oracle installed, T0       |
