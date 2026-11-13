# M4 findings: negative-case suite for `src/checker/`

Suite: `tests/negative_checkers_test.rs` (run `cargo test -j 2 --test negative_checkers_test`;
add `-- --include-ignored` to re-confirm the gaps below).

## Coverage

- 59 of 59 checker modules in `src/checker/` have a table (`mod.rs` is the index and
  `loom_checker.rs` is the trait/adapter layer, not checkers).
- 339 must-fail programs (each asserts the `LoomError` variant and a message substring), 5 or more
  per checker, plus about 150 near-miss valid programs that must be accepted.
- Rows for checkers that are hard stages of `loom::compile` are also confirmed through `compile()`.
- 6 soundness gaps (below) are kept as `#[ignore = "soundness gap: ..."]` tests. Each was run with
  `--include-ignored` and confirmed to be ACCEPTED.

## Soundness gaps (program accepted, expected rejection)

| # | Test | Program | Expected | Actual |
|---|------|---------|----------|--------|
| 1 | gap_conservation_mass_matches_string_by_substring | `fn f @conserved(Mass) :: String -> String` | TypeError "does not appear in parameter or return types" | accepted. `type_matches_any` uses `name.contains(hint)` and the Mass hint "g" is a substring of "String". |
| 2 | gap_conservation_energy_laundered_through_float | `fn f @conserved(Energy) :: Energy -> Float` | TypeError "appears in parameters but not in return type" | accepted. "Float" is a hint for every quantity, so any Float return counts as conserving it. |
| 3 | gap_compile_accepts_contradictory_contract | `fn f :: Int -> Int` with `require: x > 5`, `ensure: x < 3` | `compile()` returns Err | accepted. `SmtBridgeChecker::detect_contradiction` works (see `neg_smt_bridge_contradictions_detected`) but is never called. The Z3 path only yields Proved, Unknown or Skipped, so `run_smt_verification` in lib.rs can never report a counterexample. With no z3 on PATH everything is Skipped. |
| 4 | gap_messaging_exactly_once_and_at_most_once_from_source | `messaging_primitive Q` with `guarantees: @exactly-once @at-most-once` | TypeError "'exactly-once' and 'at-most-once' are mutually exclusive" | accepted. The parser stores `guarantees:` tokens as `format!("{:?}", token)` strings, so MessagingChecker never sees "exactly-once". The check only fires on hand-built ASTs. |
| 5 | gap_messaging_exactly_once_and_at_least_once_from_source | same, `@exactly-once @at-least-once` | TypeError "mutually exclusive" | accepted (same root cause) |
| 6 | gap_messaging_stream_exactly_once_from_source | `pattern: stream` with `guarantees: @exactly-once` | TypeError "'stream' pattern with 'exactly-once'" | accepted (same root cause) |

Gaps 4-6 share one root cause (parser/checker contract mismatch). The AST-level versions of the same
rules pass in `neg_messaging` via `mutate` hooks.

## Other observations (not soundness gaps, but they limit what can be rejected)

- Not wired into `loom::compile` (`build_checker_pipeline`): TelosFunctionChecker,
  ConservationChecker, DisciplineChecker, InfoFlowChecker. Their tables call the checker directly.
  Programs that violate them compile successfully today.
- EvolutionVectorChecker only emits `[warn]`/`[info]` diagnostics, which the pipeline suppresses, so
  it can never reject a program. Its 5 rows are warning-level diagnostics (0 true rejections).
- Warning-level rows (diagnostic emitted, `compile` still accepts) in other tables: boundary 1,
  cognitive_memory 1, journal 3, minimal 1, property 2, provenance 1, scenario 1, usecase 3. Each of
  those checkers still has at least 5 rows that are true rejections.
- Rules guarding states the parser cannot produce (empty strings or lists, values edited after
  parsing) are tested with `mutate` hooks on the parsed AST. These guard AST-level invariants only.
- Several checkers have fewer than 5 distinct rules (senescence, hgt, nichect and errorcorrection
  have 2 each). Their 5 rows vary the program (which being, which field) rather than the rule.
- UmweltChecker reports "umwelt warning: ..." as a hard TypeError with no `[warn]` prefix, so it
  blocks compilation.
- StoreChecker has about 25 distinct hard rules; the table covers 9 (the empty-store requirements
  for each store kind).
- No crashes or panics were found, and `src/` was not modified.
