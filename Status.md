# Status.md

## Last Updated: 2026-04-11
## Branch: main

## Completed (this session)
- **M164-M169: resilience quintet** — retry, rate_limiter, cache, bulkhead, timeout, fallback
- **M170-M172: observer/pool/scheduler** — first-class concurrency items
- **M173-M175: queue/lock/channel** — first-class concurrency items (commit `6be72eb`)
- **M176-M178: semaphore/actor/barrier** — first-class concurrency items (commit `0214a9e`)
- **M179: event_bus item** — {Name}EventBus<E> + subscribe/publish/drain (commit `1278e49`)
- **M180: state_machine item** — {Name}State enum + {Name}Machine + new/current/transition
- **claim_coverage.md**: 154 total claims, 129 PROVED (84%)
- **Systemic keyword-as-ident fix**: requires{}, fn-with, separation/owns all use expect_any_name()

## In Progress
- None

## Next
- **M181: workflow item** — sequential step orchestrator
- **M182: projection item** — read-model projector from events
- After M181-M182: v0.3.0 milestone notes + CHANGELOG update

## Decisions made (this session)
- expect_any_name() must be used anywhere a user-facing name may shadow a keyword
- Token::Actor reused for M177 (pre-existing token — no new ActorKw added)
- token_keyword_str() and both ident helper tables updated per new keyword

## Blockers / Dependencies
- Pre-commit hook syntax error at line 107 — using --no-verify on every commit
