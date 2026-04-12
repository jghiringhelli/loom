#!/usr/bin/env python3
"""ALX-6 Cauchy Tail Statistical Test.

Claim: Cauchy distribution has heavier tails than CLT boundary (Normal approximation).

Method:
  1. Sample N values from Cauchy(0, 1) via inverse CDF: X = tan(pi*(u - 0.5))
  2. Sample N values from Normal(0, 1) for comparison
  3. Measure 99th-percentile absolute value for each
  4. Assert: |Cauchy_99p| >> |Normal_99p|  (known: ~64 vs ~2.3 for N=10000)

Academic reference:
  Cauchy (1853), Lorentz distribution — no defined mean or variance,
  CLT does not apply. Sample mean diverges.

Result: PROVED statistically (deterministic with fixed seed).
"""

import math
import statistics

SEED = 42
N = 10_000
CAUCHY_LOCATION = 0.0
CAUCHY_SCALE = 1.0


def lcg(state: int) -> tuple[int, float]:
    """Minimal LCG PRNG returning next state and u in (0,1)."""
    state = (1664525 * state + 1013904223) & 0xFFFFFFFF
    return state, (state + 0.5) / 2**32


def sample_cauchy(u: float) -> float:
    """Inverse CDF: X = location + scale * tan(pi * (u - 0.5))."""
    return CAUCHY_LOCATION + CAUCHY_SCALE * math.tan(math.pi * (u - 0.5))


def sample_normal_bm(u1: float, u2: float) -> float:
    """Box-Muller normal sample."""
    return math.sqrt(-2 * math.log(u1 + 1e-15)) * math.cos(2 * math.pi * u2)


def main() -> None:
    state = SEED

    cauchy_samples: list[float] = []
    normal_samples: list[float] = []

    for _ in range(N):
        state, u1 = lcg(state)
        # Clamp u1 away from 0 and 1 (tan singularities at ±0.5)
        u1_safe = 0.001 + u1 * 0.998
        cauchy_samples.append(abs(sample_cauchy(u1_safe)))

        state, u2 = lcg(state)
        state, u3 = lcg(state)
        normal_samples.append(abs(sample_normal_bm(u2, u3)))

    cauchy_samples.sort()
    normal_samples.sort()

    p99_idx = int(0.99 * N) - 1
    cauchy_99p = cauchy_samples[p99_idx]
    normal_99p = normal_samples[p99_idx]

    ratio = cauchy_99p / (normal_99p + 1e-15)

    print(f"N = {N} samples (fixed seed {SEED})")
    print(f"Cauchy(0,1)  99th-percentile |X|: {cauchy_99p:.2f}")
    print(f"Normal(0,1)  99th-percentile |X|: {normal_99p:.2f}")
    print(f"Ratio (Cauchy/Normal): {ratio:.1f}x")
    print()

    # Claim 1: Cauchy 99p is much larger than Normal 99p
    assert cauchy_99p > 10.0, (
        f"Cauchy 99th-percentile {cauchy_99p:.2f} should be > 10 (got {cauchy_99p:.2f})"
    )
    # Claim 2: ratio > 5x (typically ~20-30x)
    assert ratio > 5.0, (
        f"Cauchy/Normal ratio {ratio:.1f} should be > 5x"
    )
    # Claim 3: Normal 99p is within CLT boundary (2.0 to 3.5)
    assert 2.0 <= normal_99p <= 3.5, (
        f"Normal 99th-percentile {normal_99p:.2f} outside expected [2.0, 3.5]"
    )

    print("PASS: Cauchy tail exceeds CLT boundary (Normal 99p) by more than 5x")
    print(f"RESULT: Cauchy_99p={cauchy_99p:.1f} >> Normal_99p={normal_99p:.2f} (ratio={ratio:.0f}x)")


if __name__ == "__main__":
    main()
