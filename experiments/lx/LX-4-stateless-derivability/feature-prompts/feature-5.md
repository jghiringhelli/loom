# LX-4 Feature Prompt 5 — Add canalize block with convergence proof

## Starting Loom file

```loom
module AtmosphericCarbon

being AtmosphericCarbon
  telos: "maintain CO2 below 450ppm"
  end

  regulate:
    trigger: co2_level > 420.0
    action: activate_carbon_sink
  end

  criticality:
    lower: 0.3
    upper: 0.8
    probe_fn: measure_stability
  end
end

fn activate_carbon_sink :: Unit -> Unit
end

fn measure_stability :: Unit -> Float
  0.0
end

end
```

## Feature request

Add a `canalize:` block inside the `AtmosphericCarbon` being that:
- `toward: preindustrial_equilibrium`
- `despite: [volcanic_eruption, deforestation, fossil_fuel_emission]`
- `convergence_proof: lyapunov_proof` (optional — include it)

`toward` and `despite` items must be valid identifiers (no spaces, no special chars).

## Expected result

Compiles clean. The being now has a `canalize:` block after the `regulate:` block.
