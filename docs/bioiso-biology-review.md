# BIOISO — Biological Claims: A Review Request for a Biology Researcher

**Author:** Juan Carlos Ghiringhelli (Pragmaworks)  
**Date:** April 2026  
**Prepared for:** Biology peer review  
**Full project:** github.com/jghiringhelli/loom · doi.org/10.5281/zenodo.19073543

---

## What this document is

I am building a programming language (Loom) that implements biological computation patterns as first-class language constructs. The programming language side is not what I'm asking you to evaluate. What I want your feedback on is the **biological claims specifically** — whether the mappings I've made between biological phenomena and computational structures are genuine structural isomorphisms, or whether they are superficial analogies that break under scrutiny.

I am a software engineer, not a biologist. I have read the primary sources and I believe the mappings are correct. I expect to be wrong in specific ways. That is what I am asking you to find.

---

## The core claim

Loom implements seven biological phenomena as language constructs. The claim is not that these constructs *simulate* the biological phenomena. The claim is stronger: that they satisfy the **same formal predicates** that biologists and philosophers of biology use to define those phenomena.

The distinction matters. A simulation of epigenetics models the phenomenon computationally. An *isomorphism* means the software construct and the biological phenomenon satisfy the same definition — that the same formal description applies to both.

---

## The three "alive" definitions

The most audacious claim is that a Loom program with the full set of biological constructs satisfies three standard definitions of living systems **simultaneously**:

### Schrödinger (1944) — *What is Life?*
A living system maintains low local entropy by consuming energy from its environment.

→ In Loom: a `being:` block with `matter:` (internal state) + `regulate:` (homeostatic bounds enforcement) + `telos:` (goal-direction) specifies a system that maintains bounded internal state at the cost of declared external energy effects. The homeostatic bounds are enforced structurally, not behaviorally.

**Question for you:** Does maintaining bounded internal state through declared energy consumption satisfy Schrödinger's criterion? Or does the criterion require physical thermodynamics — actual entropy reduction in physical substrate — that a computational model cannot satisfy by definition?

---

### NASA working definition (1994)
*"Life is a self-sustaining chemical system capable of Darwinian evolution."*

→ In Loom: `autopoietic: true` (self-sustaining) + `evolve:` (gradient descent / adaptive search over declared parameter space) + `ecosystem:` with `signal:` channels (population dynamics, environmental pressure) makes this formally expressible. The "chemical system" clause is the obvious weak point.

**Question for you:** The NASA definition was written specifically to be substrate-neutral (it was designed to apply to non-carbon life). Does a computational system with self-sustaining organizational logic and adaptive search satisfy it? Or is "chemical system" load-bearing in a way that excludes computation regardless of organizational complexity?

---

### Maturana & Varela (1972) — *Autopoiesis and Cognition*
A living system is organizationally closed — it continuously regenerates its own components from within its own operations.

→ In Loom: `autopoietic: true` + `crispr:` (targeted self-modification of declared structure) + `plasticity:` (adaptive weight adjustment within bounded parameters) + `@sandboxed` (effects cannot escape the being's declared operational boundary). The `@sandboxed` constraint enforces operational closure at compile time — the being's self-modification cannot reach outside its declared `matter:` and `ecosystem:`.

**Question for you:** Maturana and Varela were explicit that autopoiesis requires physical substrate — the original work was grounded in cell biology (specifically the minimal autopoietic unit as a lipid bilayer with internal chemical processes). Is organizational closure meaningful without physical instantiation? Is there a computational equivalent of "continuously regenerating its own components"?

---

## The seven biological isomorphisms

| Biological phenomenon | Loom construct | Source claim |
|---|---|---|
| **Epigenetics** | `epigenetic:` block | Behavioral modulation (gene expression changes) without genome modification. Waddington (1940). |
| **Morphogenesis** | `morphogen:` block | Reaction-diffusion spatial differentiation. Turing (1952). |
| **Replicative senescence** | `telomere:` block | Finite division limit; apoptosis on exhaustion. Hayflick (1961). |
| **CRISPR gene editing** | `crispr:` block | Targeted self-modification of declared structure. Doudna/Charpentier (2012). |
| **Quorum sensing** | `quorum:` block | Population threshold coordination. Bassler (1994). |
| **Synaptic plasticity** | `plasticity:` block | Hebbian weight adjustment. Hebb (1949). |
| **Autopoiesis** | `autopoietic: true` | Organizational closure. Maturana/Varela (1972). |

---

## What the constructs actually look like

Here is a complete Loom `being:` declaration for a neuron — the most developed example in the current codebase:

```loom
being: Neuron
  @mortal @corrigible @sandboxed
  autopoietic: true

  matter:
    membrane_potential: Float
    ion_channels: List<Channel>
    synaptic_weights: Map<NeuronId, Float>
  end

  telos: "maintain homeostatic potential through synaptic integration"

  regulate NernstEquilibrium
    bounds: membrane_potential in (-90.0, 50.0)
  end

  evolve
    search: gradient_descent
    convergence: within 1e-4 over 100 steps
  end

  epigenetic_blocks:
    epigenetic:
      trigger: "sustained_depolarization"
      modifies: "ion_channel_expression"
      reverts_when: "activity_normalized"
    end
  end

  telomere:
    limit: 50 divisions
    on_exhaustion: apoptosis
  end

  plasticity_blocks:
    plasticity:
      trigger: "co_activation"
      modifies: "synaptic_weight"
      rule: Hebbian
    end
  end
end
```

This compiles to a runnable Mesa ABM (Agent-Based Model) Python simulation and a NeuroML 2 neural structure description. The simulation runs. The safety constraints (the `@mortal @corrigible @sandboxed` annotations) are enforced at compile time — a being without a death mechanism doesn't compile.

---

## The safety architecture — and why it matters for your feedback

If a software system can satisfy formal definitions of living systems, the question immediately becomes: what prevents it from being unsafe?

The language enforces four constraints as **compile errors** on any `autopoietic: true` being:

- `@mortal` — requires a `telomere:` block; the being has a finite lifespan
- `@corrigible` — requires a `telos.modifiable_by` field; an external authority can redirect its goal
- `@sandboxed` — its self-modification effects cannot escape its declared operational boundary
- `@bounded_telos` — its goal statement cannot contain open-ended utility terms without a declared limit

The claim is that these are not behavioral rules (like Asimov's Three Laws, which can be overridden at runtime). They are **type-theoretic constraints** — a system without a death mechanism, without external goal-correction capability, without sandboxing, cannot be described in this language. It is structurally excluded, not behaviorally prohibited.

**Question for you:** Does structural exclusion of unsafe properties at the specification level provide meaningful safety guarantees for self-modifying systems? Or is there a class of unsafe emergent behaviors that these constraints cannot prevent?

---

## The specific questions I want you to challenge

1. **Are these isomorphisms or analogies?** For each of the seven constructs in the table above — is the computational construct a genuine structural isomorphism to the biological phenomenon, or does it capture only surface features? Where exactly does each analogy break?

2. **Is the synthetic life claim meaningful?** Does satisfying Schrödinger + NASA + Maturana/Varela constitute being "alive" in any meaningful biological sense? Or is there an additional criterion — physical substrate, actual metabolism, thermodynamic grounding, continuous physical instantiation — that a software system cannot satisfy regardless of organizational complexity?

3. **Is the Neuron example biologically coherent?** Does the construct above correspond to anything real in neuroscience? The Nernst equilibrium bounds, the Hebbian plasticity rule, the epigenetic trigger on sustained depolarization — are these used correctly, or are they being used in ways that a neurobiologist would find incoherent?

4. **What is the most important thing I have wrong?** Not the most subtle or the most technical — the most important. The claim that will strike a working biologist as the most obviously mistaken or imprecise.

---

## What I am not claiming

To be explicit about the limits:

- I am **not** claiming these are simulations of biology that could replace wet-lab models
- I am **not** claiming the computational beings are biologically equivalent to cells or organisms in any physical sense
- I am **not** claiming this work has applications in biological research (it may; that is not the current claim)
- I am claiming only that the **formal predicates** used by biologists and philosophers of biology to define life, epigenetics, morphogenesis, etc. apply to these constructs in the same way they apply to the biological phenomena — and I want to know where that claim is wrong

---

## Resources if you want to go deeper

| | |
|---|---|
| Full technical brief (23 pages) | `docs/state-of-loom.md` in the repo |
| Language self-specification (691 lines) | `experiments/alx/spec/loom.loom` |
| White paper (Zenodo preprint) | doi.org/10.5281/zenodo.19073543 |
| BIOISO site (not yet deployed) | bioiso.dev |

Thank you for your time. I expect the critique to be uncomfortable. That is the point.
