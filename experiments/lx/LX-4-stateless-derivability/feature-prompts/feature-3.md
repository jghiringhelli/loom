# LX-4 Feature Prompt 3 — Add a regulate block to an existing being

## Starting Loom file

```loom
module PathogenController

being PathogenController
  telos: "suppress R0 below epidemic threshold"
  end

  regulate:
    trigger: r0 > 1.5
    action: activate_containment
  end

  criticality:
    lower: 0.0
    upper: 0.8
    probe_fn: measure_spread_rate
  end
end

fn activate_containment :: Unit -> Unit
end

fn measure_spread_rate :: Unit -> Float
  0.0
end

end
```

## Feature request

Add a second `regulate:` block to the `PathogenController` being that:
- triggers when `r0 < 0.3` (pathogen is being suppressed too aggressively)
- calls `relax_containment` as the action

Also add a stub `fn relax_containment :: Unit -> Unit` at module level.

## Expected result

Compiles clean. The being now has two `regulate:` blocks.
