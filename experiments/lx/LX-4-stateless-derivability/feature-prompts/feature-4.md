# LX-4 Feature Prompt 4 — Add niche_construction to a module

## Starting Loom file

```loom
module AdaptiveMaterial

being AdaptiveMaterial
  telos: "maintain structural integrity under cyclic stress"
  end

  regulate:
    trigger: stress_level > 0.7
    action: trigger_self_repair
  end

  criticality:
    lower: 0.2
    upper: 0.9
    probe_fn: measure_integrity
  end
end

fn trigger_self_repair :: Unit -> Unit
end

fn measure_integrity :: Unit -> Float
  0.0
end

end
```

## Feature request

Add a top-level `niche_construction:` block to this module that:
- `modifies: material_microstructure`
- `affects: [CrackPropagation, FatigueAccumulation, ThermalExpansion]`
- `probe_fn: measure_crack_density`

Also add a stub `fn measure_crack_density :: Unit -> Float` that returns `0.0`.

Remember: `niche_construction:` is a **top-level** item — it goes at the module level,
NOT inside the `being` block.

## Expected result

Compiles clean.
