# LX-4 Feature Prompt 2 — Add a new lifecycle state

## Starting Loom file

```loom
module GridBalancer

being GridBalancer
  telos: "achieve full renewable energy"
  end

  regulate:
    trigger: fossil_share > 0.8
    action: incentivize_renewables
  end
end

lifecycle GridBalancer :: FossilDominated -> Transitioning -> FullRenewable

fn incentivize_renewables :: Unit -> Unit
end

end
```

## Feature request

The `GridBalancer` lifecycle currently goes `FossilDominated -> Transitioning -> FullRenewable`.
Add a new intermediate state `RenewableMajority` between `Transitioning` and `FullRenewable`,
and add a checkpoint called `CrossMajority` that:
- requires: `renewable_above_fifty` (a boolean guard function)
- on_fail: `extend_transition` (a handler function)

Also add stub definitions for `renewable_above_fifty` (returns `Bool`) and `extend_transition`
(returns `Unit`) at the module level.

## Expected result

Compiles clean. The lifecycle line becomes:
`lifecycle GridBalancer :: FossilDominated -> Transitioning -> RenewableMajority -> FullRenewable`
