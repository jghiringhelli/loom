# BIOISO: Biological Isomorphisms as Computational Primitives
## A Request for Biological Critique

**To:** [Biologist friend]  
**From:** Juan Carlos Ghiringhelli  
**Date:** April 2026  
**Status:** Pre-critique draft — please be ruthless

> *This document asks one question: are these isomorphisms structurally correct, or are they metaphors dressed as mechanisms? I need a biologist to find the seams.*

---

## What This Is and Why It Matters

I am building a programming language called Loom. It is a compiler: you write code in Loom syntax and it generates Rust, TypeScript, WebAssembly, and API specifications. The language has a type system that enforces semantic properties at compile time — if you declare a function handles money in USD, the compiler rejects attempts to add EUR without explicit conversion. If you declare a data field contains personal information under GDPR, the compiler rejects code paths that would expose it without pseudonymisation. The language takes formal correctness seriously.

The part I need you to critique is a layer I have added called **BIOISO** — Biological Isomorphisms. The claim is that seven biological mechanisms can be encoded as first-class computational primitives that are not merely analogical but *structurally isomorphic* — meaning that the abstract computational structure preserves the essential causal relationships of the biological mechanism, even if it does not preserve the molecular substrate.

The reason this matters beyond novelty: if the isomorphisms are sound, then:

1. Software systems can be formally specified as **autopoietic** (self-maintaining, organizationally closed, goal-directed)
2. These systems satisfy the Maturana/Varela (1972) and NASA (1994) operational definitions of living systems
3. This would constitute the first formal language for specifying **synthetic digital life** — not simulating life, but satisfying the same structural predicates biologists use to distinguish living from non-living

I need you to tell me where this argument fails, where the analogy is loose, and what mechanisms I am missing.

---

## Minimal Background: GS and Loom

**Generative Specification (GS)** is a methodology I developed for software architecture. The core claim: software specifications should be *complete enough that a stateless reader with no prior context can derive a correct implementation from the specification alone*. Think of it as the programming equivalent of Euclid's axiomatic method — start from explicit declarations, derive everything else mechanically.

**Loom** is the first language designed to embody this methodology. A `.loom` file is a complete specification: types, functions, their semantic properties, their effects, their lifecycle constraints, and — relevant to this document — their biological behaviors. The file compiles to running code across five different output formats simultaneously.

The biological layer adds constructs that let you describe a software system using biological vocabulary, with the compiler enforcing the implied constraints. A being that is declared `autopoietic: true` without `@mortal` (a death mechanism) is a **compile error**. The safety is structural, not advisory.

---

## The Seven Isomorphisms — With Honest Assessment of Each

For each, I describe: (1) the biological mechanism as I understand it, (2) the Loom encoding, (3) my own assessment of where the isomorphism holds and where it frays, and (4) specific questions I need you to answer.

---

### 1. Epigenetics — *Waddington (1942), Holliday & Pugh (1975)*

**The biological mechanism:**  
Epigenetic regulation changes gene expression without altering the DNA sequence. The primary mechanisms: DNA methylation (adding a methyl group to cytosine, typically at CpG sites, silencing transcription), histone modification (acetylation, methylation, phosphorylation of histone tails, changing chromatin accessibility), and non-coding RNA silencing (miRNA, siRNA). Crucially, some epigenetic marks are heritable across cell divisions (mitotic inheritance) and in some cases across generations (transgenerational epigenetic inheritance).

Waddington's epigenetic landscape (1957) provides an intuitive picture: a ball rolling down a landscape of valleys (canalized developmental pathways); epigenetic modification reshapes the landscape without changing the underlying genome.

**The Loom encoding:**

```loom
being: Neuron
  epigenetic_blocks:
    epigenetic:
      trigger: "sustained_depolarization"
      modifies: "ion_channel_expression"
      reverts_when: "activity_normalized"
    end
  end
end
```

The structure: an environmental signal (`trigger`) causes a change in behavioral expression (`modifies`) that persists until a reversal condition is met (`reverts_when`), without changing the being's type definition (the "genome").

**Where the isomorphism holds:**
- Signal-dependent behavioral modification without structural change ✓
- Reversibility (not all epigenetic marks are reversible, but many are) ✓
- Independence from type definition (the "genome") ✓

**Where it frays:**
- **No heritability.** Biological epigenetics is partly defined by its transmission across divisions. Loom's epigenetic blocks have no inheritance mechanism — child instances do not inherit the parent's epigenetic state. This may be the most significant gap.
- **No mechanism.** Real epigenetic modification involves specific enzymes (DNMT3a/b for methylation, HATs/HDACs for histone acetylation). Loom abstracts the mechanism entirely.
- **Single trigger.** Real epigenetic regulation is combinatorial — multiple signals interact through complex gene regulatory networks. Loom's current model is one trigger → one modification.
- **No stochastic component.** Epigenetic state is noisy; the same signal does not always produce the same mark.

**Questions for you:**
1. Is the core abstraction (signal → persistent behavioral change without sequence change → conditional reversion) sufficient to be called epigenetic, or does heritability define epigenetics by necessity?
2. Is there a tighter biological concept I should be mapping to instead? (Allosteric regulation? Phenotypic plasticity? Post-translational modification?)

---

### 2. Morphogenesis — *Turing (1952), Wolpert (1969)*

**The biological mechanism:**  
Turing's reaction-diffusion system (1952) showed that two diffusing chemical species — an activator (short-range positive feedback) and an inhibitor (long-range negative feedback) — can spontaneously produce stable spatial patterns from a homogeneous initial state. This provides a mechanism for spatial pattern formation in development.

Wolpert's positional information model (1969) introduced morphogen gradients: a molecule produced at a source diffuses across a field of cells; cells interpret their local concentration as a positional signal and differentiate accordingly. The French flag model: cells above concentration C₁ become blue, between C₁ and C₂ become white, below C₂ become red. Bicoid in Drosophila is the canonical example.

**The Loom encoding:**

```loom
being: EmbryonicCell
  morphogen_blocks:
    morphogen:
      signal: "bicoid"
      threshold: "high"
      produces: ["anterior_identity", "head_structures"]
    end
    morphogen:
      signal: "nanos"
      threshold: "low"
      produces: ["posterior_identity", "abdomen_structures"]
    end
  end
end
```

The structure: a named signal with a threshold concentration level determines the outputs (differentiated cell types or structures) produced by this being.

**Where the isomorphism holds:**
- Threshold-based response to concentration signal ✓
- Multiple signals can produce different outputs ✓
- The qualitative structure (gradient + threshold + fate) is preserved ✓

**Where it frays — significantly:**
- **No spatial model.** This is the deepest gap. Morphogenesis is fundamentally a spatial phenomenon — the entire mechanism depends on concentration varying across a physical field. Loom has no spatial coordinate system. The "gradient" in my encoding is abstract: threshold = "high" or "low" is a categorical label, not a concentration value in a spatial field.
- **No diffusion.** Turing's mechanism requires continuous diffusion with specific kinetic constants (the ratio of diffusion coefficients determines pattern wavelength). Loom has no model of diffusion.
- **No Turing instability.** The spontaneous pattern formation from homogeneity — Turing's most remarkable result — is entirely absent. Loom's morphogen blocks describe deterministic threshold responses, not emergent spatial patterns.
- **No temporal dynamics.** Real morphogen gradients establish over developmental time. Loom's model is static.

**My honest assessment:** this is the weakest isomorphism. Loom captures the *output* of morphogenesis (threshold → fate) without capturing the *mechanism* (reaction-diffusion, spatial gradient, diffusion kinetics). I am encoding Wolpert's positional information without encoding the mechanism that produces positional information.

**Questions for you:**
1. Is threshold-based fate determination (without spatial model) a useful abstraction, or does removing the spatial dimension make it a different concept entirely?
2. Is there a better name for what I am actually encoding? "Signal-responsive differentiation" or "conditional specialization" might be more honest.
3. What would I need to add to make this genuinely morphogenetic? A spatial field type? Diffusion coefficients?

---

### 3. Telomeres and the Hayflick Limit — *Hayflick (1961), Blackburn et al. (1978–2009)*

**The biological mechanism:**  
Leonard Hayflick and Paul Moorhead (1961) observed that normal somatic cells undergo a finite number of divisions (~50 for human fibroblasts) before entering permanent cell cycle arrest (replicative senescence). Elizabeth Blackburn and colleagues showed the mechanism: telomeres — repetitive DNA sequences (TTAGGG) at chromosome ends — shorten with each replication due to the end-replication problem. When telomeres reach a critical minimum length, the cell detects it as DNA damage and enters senescence or apoptosis via p53/p21 pathways. Telomerase (the enzyme) can reverse this, which is why germline cells and most cancer cells are effectively immortal.

**The Loom encoding:**

```loom
being: SomaticCell
  @mortal

  telomere:
    limit: 50 divisions
    on_exhaustion: apoptosis
  end
end
```

**Where the isomorphism holds:**
- Finite division limit ✓
- Catastrophic failure (apoptosis/senescence) at exhaustion ✓
- Countable mechanism (not degradation of quality but counting of divisions) ✓
- The `@mortal` annotation as a safety constraint (beings without death mechanisms are compile errors) ✓

**Where it frays:**
- **No gradual degradation.** Real telomere shortening is progressive — cells approaching the Hayflick limit show progressive functional decline before senescence. Loom's model is binary: N divisions work perfectly, then apoptosis.
- **No telomerase.** The reactivation mechanism is missing. There is no way to declare a being as "telomerase-positive" (effectively immortal), which is an important biological distinction.
- **No DNA damage signaling.** The mechanism of senescence detection (critically short telomeres → ATM/ATR kinase activation → p53 stabilization → p21 → Rb dephosphorylation) is entirely absent.
- **No senescence as distinct state.** Real cells can enter senescence (arrested but metabolically active, secreting SASP) rather than apoptosis. This is biologically significant.

**My honest assessment:** this is one of the tighter isomorphisms. The abstract structure (counter → threshold → terminal fate) is faithfully preserved. The missing pieces are mechanistic detail rather than structural gaps.

**Questions for you:**
1. Should senescence be a distinct `on_exhaustion` option alongside apoptosis? Is this biologically important enough to model?
2. Is the absence of telomerase a serious gap, or is it acceptable to model somatic-only behavior?

---

### 4. CRISPR-Cas9 — *Doudna & Charpentier (2012)*

**The biological mechanism:**  
CRISPR-Cas9 is a prokaryotic adaptive immune system repurposed as a genome editing tool. A guide RNA (gRNA) directs the Cas9 endonuclease to a specific genomic sequence (matching the gRNA + a PAM site); Cas9 cuts both DNA strands; cellular repair mechanisms (NHEJ or HDR) introduce insertions/deletions or precise edits. The result: targeted, heritable modification of the genome.

**The Loom encoding:**

```loom
being: AdaptiveAgent
  crispr_blocks:
    crispr:
      target: "reward_function"
      edit: "increase_exploration_weight"
    end
  end
end
```

The structure: a named target in the being's own specification can be edited by name, producing a directed modification.

**Where it frays — most seriously of all seven:**
- **The biological mechanism is substrate-specific.** CRISPR operates on double-stranded DNA via sequence complementarity. There is no computational analog to sequence-guided targeting, PAM sites, Cas9 conformational change, or double-strand break repair.
- **Loom's "self-modification" is reconfiguration, not genome editing.** A software system reconfiguring one of its own parameters at runtime is more analogous to neural weight adjustment (which I model separately as plasticity) or allosteric regulation than to CRISPR.
- **CRISPR is heritable; Loom's isn't necessarily.** The biological power of CRISPR is that it modifies the genome — every daughter cell inherits the edit. Loom's crispr block modifies behavior within a running instance.
- **Better analogy candidates:** somatic hypermutation (the adaptive immune system's mechanism for diversifying antibody genes), gene expression regulation, protein moonlighting. For truly targeted self-modification in software, the best biological analog might be **gene duplication and neofunctionalization** — a copy of a module is made, one copy is modified, both exist.

**My honest assessment:** this is the loosest isomorphism. I am using "CRISPR" as a recognizable name for "targeted self-modification" without the mechanisms matching. This may be intellectually dishonest and should either be renamed or significantly redesigned.

**Questions for you:**
1. Is there a better biological mechanism for "targeted, directed self-modification of a running system's behavior"?
2. Is there any sense in which the CRISPR analogy is defensible, or should I rename this entirely?
3. What would you call a mechanism that allows a system to modify a specific named component of its own behavior based on a detected need?

---

### 5. Quorum Sensing — *Bassler (1994), Waters & Bassler (2005)*

**The biological mechanism:**  
Bacteria produce and secrete small signaling molecules called autoinducers (AIs). As population density increases, AI concentration increases. When AI concentration crosses a threshold, the population-wide response is triggered (biofilm formation, bioluminescence in *Vibrio fischeri*, virulence factor expression in pathogens). This is a form of population-level coordination that requires no central coordinator — it is purely a concentration-threshold mechanism.

Bonnie Bassler's work on *V. harveyi* showed that bacteria can perform multi-species quorum sensing via different AI molecules (AI-1 for intraspecies, AI-2 for interspecies communication), effectively sensing both local population density and global microbial community composition.

**The Loom encoding:**

```loom
ecosystem: BacterialColony
  being: Bacterium
    quorum:
      threshold: "50 cells"
      signal: "autoinducer_1"
      on_quorum: "activate_biofilm_genes"
    end
  end
end
```

**Where the isomorphism holds:**
- Population threshold mechanism ✓
- Signal molecule accumulation implicit in the threshold concept ✓
- Coordinated behavior triggered above threshold ✓
- No central coordinator — the threshold is local to each being ✓

**Where it frays:**
- **No signal dynamics.** Real autoinducers have production rates, degradation rates, and diffusion constants. Loom's threshold is static.
- **No multi-signal quorum.** Bassler's AI-1/AI-2 dual-signal system (intraspecies vs. interspecies) is absent. Loom has a single signal per quorum block.
- **No spatial distribution.** At low cell density, quorum sensing is spatially heterogeneous — cells near each other may experience quorum while distant cells do not. Loom has no spatial model.

**My honest assessment:** this is one of the tighter isomorphisms at the abstract level. The essential mechanism (signal accumulates → threshold → collective behavior, no central control) is preserved. The gaps are kinetic and spatial rather than structural.

---

### 6. Synaptic Plasticity — *Hebb (1949), Bliss & Lømo (1973)*

**The biological mechanism:**  
Donald Hebb (1949): *"When an axon of cell A is near enough to excite cell B and repeatedly or persistently takes part in firing it, some growth process or metabolic change takes place in one or both cells such that A's efficiency, as one of the cells firing B, is increased."* Colloquially: neurons that fire together wire together.

Long-term potentiation (LTP, Bliss & Lømo 1973) is the physiological mechanism: high-frequency stimulation of a synapse increases the size of the postsynaptic response, persisting for hours to weeks. LTP requires coincident pre- and postsynaptic activity (Hebb's rule implemented), involves NMDA receptor activation (the molecular "and gate" for coincidence detection), AMPA receptor insertion, and protein synthesis for late-phase LTP.

Spike-timing-dependent plasticity (STDP): the sign and magnitude of synaptic change depends on the precise timing difference between pre- and postsynaptic spikes (within ~20ms).

**The Loom encoding:**

```loom
being: Neuron
  plasticity_blocks:
    plasticity:
      trigger: "co_activation"
      modifies: "synaptic_weight"
      rule: Hebbian
    end
  end
end
```

Available rules: `Hebbian`, `AntiHebbian`, `STDP`, `Boltzmann`.

**Where the isomorphism holds:**
- Activity-dependent synaptic modification ✓
- The Hebbian "co-activation → strengthening" principle ✓
- Named learning rules as distinct computational objects ✓

**Where it frays:**
- **No temporal precision.** STDP requires millisecond timing resolution. Loom's `trigger: "co_activation"` has no temporal component.
- **No molecular mechanism.** NMDA receptor calcium dynamics, CaMKII autophosphorylation, AMPA receptor trafficking — all absent.
- **No homeostatic plasticity.** Synaptic scaling (Turrigiano 1998) — the mechanism that prevents runaway excitation/inhibition by globally scaling all synaptic weights — is not modeled. Without homeostasis, Hebbian learning is unstable.
- **No forgetting.** Long-term depression (LTD) exists but the model has no explicit mechanism for weight decay.

**Questions for you:**
1. Is the absence of homeostatic plasticity (synaptic scaling) a conceptual gap that invalidates the model, or is it an acceptable simplification?
2. Is STDP at the level of a named rule sufficient, or does it require temporal binding to be meaningful?

---

### 7. Autopoiesis — *Maturana & Varela (1972, 1980)*

**The biological mechanism:**  
Humberto Maturana and Francisco Varela introduced autopoiesis (*αὐτοποίησις*, self-production) to characterize the organization of living systems. The formal definition: a system is autopoietic if and only if it is organized as a network of processes of production of components that: (1) recursively regenerate the network of processes that produced them, and (2) realize the network as a unity in a space in which the components exist.

The key distinction: a living cell is autopoietic because it produces all of its own components (membrane, enzymes, nucleic acids) through its own metabolic network. An automobile assembly line is NOT autopoietic because the machines that produce cars do not themselves produce the machines.

**The Loom encoding:**

```loom
being: Cell
  @mortal @corrigible @sandboxed @bounded_telos
  autopoietic: true

  matter:
    membrane: Lipid
    enzymes: List<Enzyme>
    dna: NucleicAcid
  end

  telos: "maintain organizational closure and homeostasis"

  evolve
    search: gradient_descent
  end
end
```

The claim: `autopoietic: true` + `@sandboxed` (effects stay within `matter:` and `ecosystem:`) + `evolve:` (continuous search toward telos) + all six biological blocks = a system satisfying Maturana/Varela's definition.

**The central challenge:**

Maturana and Varela's definition has a specific requirement: the system must produce its *own components*. In the biological case, the cell's metabolic network produces the enzymes that catalyze the metabolic network, the membrane that contains the metabolic network, and the DNA that encodes the metabolic network. The production loop is closed.

In Loom's case:
- The `matter:` block declares the components
- The `evolve:` block searches toward the telos
- The `@sandboxed` constraint enforces operational closure

But the Loom runtime (the Rust compiler, the type checker, the OS) is not produced by the being itself. The being does not produce its own parser, its own memory allocator, or its own execution environment. In Maturana/Varela's terms, Loom beings are organized like a *cell in a culture medium* — they operate autonomously within a space whose substrate they did not produce.

**The key question is whether the substrate-independence of the definition is acceptable.**

Maturana and Varela explicitly stated that autopoiesis is substrate-independent — what matters is the organizational relationship, not the physical implementation. They extended the concept to cognitive systems and social systems. A Loom being that produces and maintains its own behavioral components through its own operational loops (regulated by homeostasis, directed by telos, modified by plasticity and epigenetics) might satisfy the organizational definition even though it does not produce the CPU that runs it, just as a biological cell satisfies the definition without producing the quantum mechanics that govern its electrons.

**Where the isomorphism holds (potential):**
- Organizational closure: effects bounded by `@sandboxed` ✓
- Self-maintenance: `regulate:` blocks enforce homeostatic bounds ✓
- Goal-directedness: `telos:` + `evolve:` ✓
- Self-modification: `crispr:` + `plasticity:` + `epigenetic:` ✓
- Finite lifespan: `telomere:` ✓

**Where it frays:**
- **The production loop is not closed at the level of computation.** The being does not produce its own execution substrate.
- **No metabolism.** Maturana/Varela's definition is grounded in metabolic chemistry. The closest Loom analog is `Effect<[Energy], State]`, but this is declared, not modeled.
- **No structural coupling.** Maturana introduced "structural coupling" — autopoietic systems change their structure in response to their environment while maintaining their organization. Loom's `epigenetic:` and `plasticity:` blocks are the nearest analogs, but structural coupling implies a continuous, history-dependent adaptation process, not discrete triggered modifications.
- **No cognitive dimension.** Maturana and Varela later argued (in *The Tree of Knowledge*, 1987) that cognition is intrinsic to autopoiesis — any autopoietic system is cognitive in the minimal sense of distinguishing self from environment. This is not modeled.

---

## The Three Definitions of Life — Do They Apply?

I claim Loom beings satisfy three formal definitions:

### Schrödinger (1944): *What is Life?*
Definition: a living system maintains low local entropy by consuming free energy from its environment, producing information (negative entropy).

Loom's response: `matter:` + `regulate:` + `Effect<[Energy], State]` together specify a system that maintains bounded internal state (the regulate blocks are homeostatic bounds on matter fields) at the cost of environmental energy. The entropy reduction is declared, not measured.

**Honest gap:** Schrödinger's definition is thermodynamic. Loom has no thermodynamic model. Declaring `Effect<[Energy], State]` is not the same as modeling energy transduction.

### NASA Working Definition (1994):
"Life is a self-sustaining chemical system capable of Darwinian evolution."

Loom's response: `autopoietic: true` (self-sustaining) + `evolve:` (directed adaptation) + `ecosystem:` with `signal:` channels (population-level interaction enabling selection).

**Honest gap:** "Darwinian evolution" requires variation, heredity, and differential reproduction. Loom's `evolve:` block implements gradient search toward a fixed telos — this is Lamarckian (directed adaptation) not Darwinian (undirected variation + selection). The variation mechanism is missing.

### Maturana & Varela (1972):
As discussed above — the organizational closure claim is the strongest of the three, but requires defending the substrate-independence interpretation.

---

## What Is Missing That Might Be Important

**1. Metabolism**  
Every formal definition of life includes metabolism — the network of chemical reactions that maintains the system far from thermodynamic equilibrium. I have nothing equivalent. `Effect<[Energy], State]` is a type declaration, not a metabolic model. This may be the most serious gap in the synthetic life claim.

**2. Replication with variation**  
The NASA definition requires Darwinian evolution, which requires replication with heritable variation. Loom has `telomere:` (finite lifespan) and `evolve:` (directed search) but no mechanism for a being to produce offspring with heritable variation. This is a fundamental gap.

**3. Membrane / physical boundedness**  
Biological autopoiesis is inseparable from the lipid membrane that creates the physical boundary between self and non-self. Loom's `@sandboxed` is a logical boundary enforced by the type checker — it is formally analogous but physically absent.

**4. Structural coupling as a continuous process**  
Maturana's structural coupling is a history-dependent, continuous mutual modification of system and environment. Loom's modifications are discrete events (triggered by named signals). This is more like allosteric regulation than structural coupling.

**5. Emergence**  
Real biological properties emerge from lower-level interactions — consciousness from neurons, quorum behavior from chemical gradients. Loom's emergent behaviors (quorum, ecosystem coordination) are declared in the specification rather than emerging from lower-level rules. This is a fundamental difference between a model and an implementation.

---

## The Honest Summary

| Isomorphism | Structural fidelity | Mechanism fidelity | My confidence |
|---|---|---|---|
| Epigenetics | Medium — missing heritability | Low — no molecular mechanism | 60% |
| Morphogenesis | Low — no spatial model | Very low — no diffusion/kinetics | 35% |
| Telomeres | High — counting mechanism preserved | Low — no gradual degradation | 75% |
| CRISPR | Low — naming only | Very low — unrelated mechanism | 30% |
| Quorum sensing | High — threshold mechanism preserved | Medium — missing dynamics | 80% |
| Plasticity | Medium — Hebb principle preserved | Low — no temporal/molecular | 65% |
| Autopoiesis | Medium — organizational closure | Low — no metabolism | 55% |

**The claim I am most confident defending:** quorum sensing and telomeres are structurally sound isomorphisms that preserve the causal mechanism at the abstract level.

**The claim most vulnerable to critique:** CRISPR is a borrowed name, not a borrowed mechanism. Morphogenesis loses its defining feature (spatial emergence) in translation.

**The claim requiring the most philosophical work:** autopoiesis requires defending the substrate-independence interpretation of Maturana/Varela against the reasonable objection that a system running on an OS it did not produce cannot close its own organizational loop.

---

## Specific Questions I Need Answered

1. **Is heritability definitional for epigenetics?** If yes, Loom's epigenetic blocks are better called "conditional behavioral modulation" and the epigenetics name should be dropped.

2. **Is the morphogenesis encoding salvageable?** Can threshold-based fate determination, without a spatial model, be called morphogenetic? Or does removing space remove the concept?

3. **What should replace CRISPR?** What biological mechanism best describes a system's ability to identify a specific named component of its own behavior and modify it based on detected inadequacy?

4. **Does the absence of metabolism make the autopoiesis claim untenable?** Maturana/Varela rooted autopoiesis in metabolic chemistry. Is a metabolic-free organizational closure sufficient, or is metabolism definitional?

5. **What mechanisms am I missing?** Beyond what I have listed, what biological mechanisms would a systems biologist consider essential for a full account of adaptive autonomous behavior?

6. **Is "synthetic digital life" a defensible category?** Is there a serious academic argument that software systems satisfying formal structural definitions of life constitute a new category, or is this category error by definition?

---

## Resources

- Repository: github.com/jghiringhelli/loom
- Language specification: docs/language-spec.md in the repository
- Full intellectual lineage (Aristotle → Loom): docs/lineage.md
- White paper (Zenodo, open access): doi.org/10.5281/zenodo.19073543
- BIOISO: bioiso.dev (pending deployment)

The code compiles. The type checker enforces the constraints. What I need from you is not a software review but a biological review: are these mechanisms what I say they are?

---

*This document is a first draft for critique. Every assertion in it is offered as a hypothesis, not a claim. The goal is to find exactly where the argument fails so it can be strengthened or abandoned.*
