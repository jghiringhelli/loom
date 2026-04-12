# LX-2 — Kani Formal Proof Harness

**Hypothesis:** Loom `require:`/`ensure:` contracts emit structurally correct
`#[cfg(kani)] #[kani::proof]` harnesses for SAT-bounded verification via Kani/CBMC.

**Status:** STRUCTURAL VERIFIED — CBMC proof deferred (Kani requires Linux)

## What was verified

1. Loom source compiles cleanly ✅
2. Emitted Rust contains `#[cfg(kani)] #[kani::proof]` harness ✅
3. Harness uses `kani::any()` for symbolic inputs ✅
4. Harness uses `kani::assume()` for `require:` preconditions ✅
5. Harness uses `kani::assert!()` for `ensure:` postconditions ✅
6. `#[cfg(kani)]` gate makes harness invisible to `cargo build`/`cargo test` ✅

## What is deferred

- Actual CBMC proof run via `cargo kani` — **Kani only builds on Linux**
  (`cargo install --locked kani-verifier` fails on Windows with compilation errors)
- CI gate: add `cargo kani` step to GitHub Actions on `ubuntu-latest` runner

## Loom source

```loom
module AddPositiveContracts

fn add_positive :: Int -> Int -> Int
  require: a > 0
  require: b > 0
  ensure:  result > a
  ensure:  result > b
  a + b
end

end
```

Source: `experiments/verification/v2_kani_clean.loom`

## Emitted Rust (harness section)

```rust
pub fn add_positive(a: i64, b: i64) -> i64 {
    // LOOM[require]: (a > 0) — debug_assert! (runtime, debug builds only)
    debug_assert!((a > 0), "precondition violated: (a > 0)");
    // LOOM[require]: (b > 0) — debug_assert! (runtime, debug builds only)
    debug_assert!((b > 0), "precondition violated: (b > 0)");
    let _loom_result = (a + b);
    // LOOM[ensure]: (_loom_result > a) — checked on return value via _loom_result
    debug_assert!((_loom_result > a), "ensure: (_loom_result > a)");
    // LOOM[ensure]: (_loom_result > b) — checked on return value via _loom_result
    debug_assert!((_loom_result > b), "ensure: (_loom_result > b)");
    _loom_result
}

// LOOM[V2:Kani]: add_positive — SAT-bounded formal proof (Kani 2021)
// Proves require:/ensure: hold for ALL inputs within solver bounds.
// Install: cargo install --locked kani-verifier   Run: cargo kani
#[cfg(kani)]
#[kani::proof]
fn kani_verify_add_positive() {
    let a: i64 = kani::any();
    let b: i64 = kani::any();
    // Preconditions — restrict symbolic input domain
    kani::assume((a > 0));
    kani::assume((b > 0));
    let result = add_positive(a, b);
    // Postconditions — Kani proves these for all valid inputs
    kani::assert!((result > a), "(result > a)");
    kani::assert!((result > b), "(result > b)");
}
```

Source: `experiments/verification/v2_kani_clean.rs`

## To run on Linux

```sh
cargo install --locked kani-verifier
# Copy v2_kani_clean.rs into a Cargo project:
cargo new kani_verify --lib
cp experiments/verification/v2_kani_clean.rs src/lib.rs
cargo kani
# Expected output:
#   VERIFICATION:- SUCCESSFUL
#   Verified 1 harness(es)
```

## Claim coverage update

See `experiments/verification/claim_coverage.md`:
- `require:/ensure: → #[cfg(kani)] harness` — EMITTED (structure verified, CBMC deferred)
