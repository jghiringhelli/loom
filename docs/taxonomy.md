# Loom Taxonomy and Ontology Design

**Status:** M185–M190 implemented and committed. M191–M192 pending.  
**ADR:** ADR-0006  
**Date:** April 2026  
**Last updated:** 2026-04-12

---

## The Question

Does Loom need defined taxonomies and ontologies — and if so, **inside** the language,
**outside** as derived artifacts, or **both**?

Short answer: **both, for different reasons**, with a clean separation between them.

---

## Why Two Layers

| Layer | What it does | Who reads it |
|---|---|---|
| **Inside the language** | Developer declares *intent* — what kind of thing this is, what domain it belongs to | Loom type-checker, codegen, ALX evaluator |
| **Outside** (exported ontology) | Machine-readable formal relationships for tooling, visualization, cross-system reasoning | OWL reasoners, knowledge graph tools, downstream LLMs |

The inside layer is the *specification*. The outside layer is a *derivation* from it.
The same principle as `require:`/`ensure:` → Kani harnesses: you write once, you get both.

---

## Inside the Language

### 1. Module domains (`domain:`)

Classify a module by the real-world domain it models:

```loom
module AtmosphericCarbon
  domain: climate
  ...
end
```

Valid domains (open enumeration — new ones are legal):
`climate`, `energy`, `epidemics`, `materials`, `finance`, `logistics`,
`healthcare`, `agriculture`, `infrastructure`, `compute`

### 2. Being roles (`role:`)

Every `being` has a **functional role** in the system. Four canonical roles:

| Role | Meaning | Example |
|---|---|---|
| `sensor` | Reads the environment; emits signals without acting | TemperatureSensor |
| `effector` | Acts on the environment; changes state | CarbonSink |
| `regulator` | Monitors and controls other beings | AtmosphericCarbon |
| `integrator` | Aggregates signals from multiple sources | ClimateIndex |
| `memory` | Persists state across cycles | ResistanceHistory |
| `classifier` | Makes structured decisions on input | AnomalyDetector |

```loom
being AtmosphericCarbon
  role: regulator
  domain: climate
  telos: "maintain CO2 below 450ppm"
  end
  ...
end
```

### 3. Inter-being relationships (`relates_to:`)

Declare structural relationships between beings — the basis for ontology export:

```loom
being GridBalancer
  role: regulator
  relates_to:
    regulates: [SolarFarm, WindFarm, BatteryStorage]
    monitors: [GridLoad, FrequencyMonitor]
    depends_on: [WeatherForecaster]
  end
  ...
end
```

### 4. Classifier item (the micro-LLM gate)

A `classifier` is a first-class Loom item — a learned decision function that
sits between regex (too weak) and a full LLM (too expensive) for regulate triggers.

```loom
classifier AnomalyDetector
  model: bert-tiny
  input: SensorReading
  output: AnomalyLabel
  threshold: 0.85
  retrain_trigger: accuracy < 0.80
  retrain_strategy: | online_gradient
end
```

A `being` can use a classifier as a regulate trigger:

```loom
being GridBalancer
  regulate:
    trigger: classifier: AnomalyDetector, input: grid_reading
    action: investigate_anomaly
  end
end
```

**Why this matters:** BIOISOs operating in complex environments need gates that:
- Cannot be expressed as `x > threshold` (pattern is learned, not formulaic)
- Are far cheaper than a full LLM call at inference time
- Can be retrained *by the BIOISO itself* when distribution shift is detected
  (the `retrain_trigger` / `retrain_strategy` fields)

This creates a **distributed nervous system**: micro-LLMs as classifiers embedded
in regulate gates, retrained on demand, coordinated by the BIOISO's evolve/epigenetic
machinery.

---

## Outside the Language (Exported Ontology)

The Loom compiler should emit a `.owl.json` (OWL/JSON-LD) alongside each `.rs`:

```json
{
  "@context": "https://pragmaworks.dev/loom/ontology/v1",
  "@type": "loom:Module",
  "name": "AtmosphericCarbon",
  "domain": "climate",
  "beings": [
    {
      "@type": "loom:Being",
      "name": "AtmosphericCarbon",
      "role": "regulator",
      "telos": "maintain CO2 below 450ppm",
      "regulates": ["BiosphereCarbon", "OceanCarbon"],
      "lifecycle_states": ["Stable", "Stressed", "Crisis", "Recovering"],
      "criticality_window": [0.3, 0.8]
    }
  ]
}
```

### Why this matters

1. **Cross-system composition:** Two independently-developed BIOISO modules can be
   composed without reading each other's source — their ontology exports define the
   interface at the semantic level.
2. **Tooling:** An ontology browser can show which beings regulate which, visualize
   the lifecycle graph, and flag missing relationships.
3. **ALX evaluation:** The ontology export is a machine-readable summary of semantic
   claims — direct input to LX-1 measurement and ALX scoring.
4. **Downstream LLMs:** When an LLM is asked to reason about a Loom program, giving it
   the `.owl.json` is 10× cheaper than giving it the full `.loom` source.

---

## The Micro-LLM Distributed Nervous System

The classifier item enables a design pattern we call the **distributed nervous system**:

```
BIOISO
│
├── Sensors (being role: sensor)
│     └── emit SensorReading signals
│
├── Classifiers (loom item: classifier)
│     ├── Tiny learned models (bert-tiny, distilbert, small CNNs)
│     ├── Triggered by regulate: blocks
│     └── Self-retrained via evolve:/epigenetic: when accuracy drifts
│
├── Regulators (being role: regulator)
│     └── Act on classifier outputs
│
└── Integrators (being role: integrator)
      └── Aggregate classifier+sensor signals → system-level telos tracking
```

**Key property:** No central LLM. Each classifier is specialized, tiny, and local.
The BIOISO's `evolve:` and `epigenetic:` blocks handle retraining — the system improves
its own classifiers without external intervention.

**Where Loom adds value:** The `retrain_trigger` and `retrain_strategy` fields on
a `classifier` item are *contracts* — they are verified by the Loom checker to be
consistent with the being's `telos` and `canalize` declarations. A classifier whose
retraining strategy could violate the telos convergence proof is flagged at compile time.

---

## Milestone Plan

These features are proposed as the next milestone cluster (M185–M192):

| Milestone | Feature | Tier |
|---|---|---|
| M185 | `domain:` annotation on modules | Static | ✅ Done |
| M186 | `role:` annotation on beings | Static | ✅ Done |
| M187 | `relates_to:` structural relationships | Static | ✅ Done |
| M188 | `classifier` item type | Static | ✅ Done |
| M189 | `trigger: classifier:` in regulate blocks | Static | ✅ Done |
| M190 | Ontology export — `.owl.json` | Codegen | ✅ Done |
| M191 | TelosConsistencyChecker for classifier retrain_strategy | Checker | ✅ Done |
| M192 | 5 BIOISO programs updated with roles/domains/classifiers | Runtime | ✅ Done |

---

## Summary

| Dimension | Answer |
|---|---|
| Taxonomy inside language? | **Yes** — `domain:`, `role:`, `relates_to:` as first-class declarations |
| Ontology outside? | **Yes** — `.owl.json` derived from AST, emitted by compiler |
| Micro-LLM classifiers? | **Yes** — `classifier` item type, `trigger: classifier:` in regulate |
| Distributed nervous system? | **Yes** — enabled by classifier + epigenetic retrain loop |
| When? | M185–M192 — next milestone cluster after V3 proptest |
