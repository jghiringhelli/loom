# LX-4 Feature Prompt 1 — Add require:/ensure: contract to an existing function

## Starting Loom file

```loom
module PricingEngine

fn compute_total :: OrderLine -> OrderTotal
  let subtotal = 0.0
  let tax = subtotal * 0.15
  subtotal + tax
end

end
```

## Feature request

Add the following contracts to `compute_total`:
- A `require:` that checks `quantity > 0`
- A `require:` that checks `unit_price >= 0.0`  
- An `ensure:` that checks `result >= 0.0`

## Expected result

The function should compile with:
```sh
cargo run --bin loom -- compile pricing_engine.loom
```
Output: `compiled 'pricing_engine.loom' -> 'pricing_engine.rs'`

## Checker

After compiling, the emitted Rust should contain `debug_assert!` lines for each contract.
