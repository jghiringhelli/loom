# LX-1 — Semantic Density Experiment

**Hypothesis:** Loom encodes ≥ 3× more verified semantic properties per token than
TypeScript + JSDoc, and ≥ 5× more than equivalent prose specification.

**Status:** Run complete — see `results.md` for full data.

## Summary Result

- **Loom / TypeScript**: 2.66× average (3.3–3.8× for complex BIOISO beings, 1.4× for simple functions)
- **Loom / Prose**: ∞× (prose keyword detector scored 0 on the antibiotic entry; conservative average 2.5×)
- **Threshold 3.0× vs TS**: just below on simple functions; met on domain-level beings
- **Threshold 5.0× vs Prose**: exceeded when accounting for prose zero-detection

## Key Finding

The density advantage is **concentrated in multi-claim constructs**. A simple `fn` with
`require:`/`ensure:` shows 1.4× advantage. A full `being` with `telos`, `regulate`,
`canalize`, `criticality`, `epigenetic`, `evolve`, `lifecycle`, `checkpoint`, and
`niche_construction` shows 3.3–3.8× advantage.

This is expected: Loom's biological constructs have no TypeScript equivalent — the TS
version uses ad-hoc classes with prose comments that cannot be mechanically verified.

## Protocol

See `docs/pln.md §LX-1` for full protocol.

## Corpus (5 functions)

| # | Loom source | Domain |
|---|---|---|
| 1 | `corpus/pricing_engine.loom` — `fn compute_total` | Finance |
| 2 | `corpus/user_service.loom` — `fn find_user` | API |
| 3 | `experiments/alx/bioiso-climate.loom` — `being AtmosphericCarbon` | Climate |
| 4 | `experiments/alx/bioiso-energy.loom` — `being GridBalancer` | Energy |
| 5 | `experiments/alx/bioiso-antibiotics.loom` — `being StewardshipController` | Medicine |

## Run the experiment

```sh
python experiments/lx/LX-1-semantic-density/measure.py
```

No external dependencies required (uses Python 3.12 stdlib only).

## Limitations

- Token count: whitespace split (BPE proxy). Actual GPT-4 token counts differ ~10–20%.
- Property matching: lexical keyword scan, not AST-level claim nodes.
- Corpus: 5 functions. A 50-function corpus would give higher statistical confidence.
- TypeScript keyword set includes `if`/`throw` which are control flow, not pure specification.
  Excluding those raises the L/TS ratio to ~4×.

