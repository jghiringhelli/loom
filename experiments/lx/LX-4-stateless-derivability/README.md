# LX-4 — Stateless Derivability Experiment

**Hypothesis:** A fresh LLM session given only a `.loom` file + `docs/language-spec.md`
can implement a new feature that compiles clean with zero checker errors, for ≥ 4/5 trials.

**Status:** Runnable now — all language constructs exist.

## Protocol

See `docs/pln.md §LX-4` for full protocol.

## Features to test (5)

| # | Module | Feature request | Expected outcome |
|---|---|---|---|
| 1 | *(TBD)* | Add a `@conserved(Energy)` annotation to an existing function | Compiles, UnitsChecker passes |
| 2 | *(TBD)* | Add a new lifecycle state to an existing `lifecycle` declaration | Compiles, TypestateChecker passes |
| 3 | *(TBD)* | Add a `regulate` block to an existing `being:` | Compiles, TelosChecker passes |
| 4 | *(TBD)* | Add a new `flow secret` label to a field | Compiles, InfoFlowChecker passes |
| 5 | *(TBD)* | Add a `require:` contract to an existing function | Compiles, TypeChecker passes |

## Results

| Trial | Feature | Compiles clean | Checker passes | Semantic correct | Notes |
|---|---|---|---|---|---|
| *(pending)* | | | | | |

## Conclusion

*(pending)*
