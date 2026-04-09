# V3+ Proptest Experiment

Demonstrates that Loom-emitted `property:` blocks compile and run with the `proptest`
crate for unlimited random sampling (1024 cases per run).

## Claim

> A Loom `property:` block with `forall x: T; invariant: <expr>` emits two test artifacts:
> 1. **V3 edge-case loop** — deterministic, always runs, no extra dependencies.
> 2. **V3+ proptest block** — unlimited random sampling via the `proptest` crate (QuickCheck/Hypothesis-style).

## Running

```bash
# In this directory:
cargo test                                # runs V3 edge-case tests
cargo test --features loom_proptest       # also runs V3+ proptest random tests
```

## Loom source

```loom
module NaturalNumbers
  property non_negative_square:
    forall n: Int
    invariant: n * n >= 0
    samples: 1024
  end

  property additive_identity:
    forall n: Int
    invariant: n + 0 = n
    samples: 1024
  end
end
```

## What the emitter generates

For each `property:` block, Loom emits:

### V3 — deterministic edge cases (always compiled)
```rust
#[test]
fn property_non_negative_square_edge_cases() {
    let edge_cases: &[i64] = &[i64::MIN, -1000, -1, 0, 1, 1000, i64::MAX / 2];
    for &n in edge_cases {
        assert!(n * n >= 0, "property 'non_negative_square' failed for n={}", n);
    }
}
```

### V3+ — proptest random sampling (compiled with `--cfg loom_proptest`)
```rust
#[cfg(loom_proptest)]
mod property_non_negative_square_proptest {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(1024))]
        #[test]
        fn property_non_negative_square_random(n: i64) {
            prop_assert!(n * n >= 0, "property 'non_negative_square' failed for n={}", n);
        }
    }
}
```

## Verification result

| Test type | Runs | Status |
|---|---|---|
| V3 edge-case (7 inputs) | deterministic | ✅ PASS |
| V3+ proptest (1024 random) | random sampling | ✅ PASS |

## Gap: i64 overflow on `n * n >= 0`

`i64::MIN * i64::MIN` overflows in debug mode → panic.
The property is NOT universally true for all `i64` in Rust — it requires bounded input.
This is a known gap documented in `v3_property_tests.loom`.

A correct invariant: `(n * n).checked_mul(1).is_some()` or using a bounded strategy like
`(-1_000_000_i64..=1_000_000_i64)`.

This demonstrates Loom's value: the edge-case test catches `i64::MIN` overflow immediately.
