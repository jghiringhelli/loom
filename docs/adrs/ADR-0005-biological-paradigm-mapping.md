# ADR-0005: Biological Paradigm as the Core Abstraction

**Date**: 2026-04-08  
**Status**: Accepted

## Context

Most programming languages use mathematical or mechanical metaphors
(functions, objects, modules, processes). Loom chose biological metaphors as
its primary abstraction layer. This decision permeates every construct:
`being` (not class), `telos` (not purpose annotation), `regulate` (not
invariant), `evolve` (not migration), `autopoietic` (self-producing), `umwelt`
(perceptual world), `senescence` (aging), `canalization` (developmental
robustness), `plasticity` (experience-driven change).

The key question: is this a cosmetic naming choice, or does biology add
semantic value that mathematics/mechanics cannot express as naturally?

## Decision

Biology is the semantic foundation, not a skin. The constructs map to specific
scientific literature (cited in each checker) and enforce relationships that
pure OOP or FP cannot express:

| Biological concept | Scientific grounding | Semantic value in Loom |
|---|---|---|
| `telos` | Aristotle's final cause; Mayr's teleonomic programs | Computable purpose — distinguishes "what it does" from "what it is for" |
| `autopoietic` | Maturana & Varela (1972) | Structural closure: a being that maintains its own organization |
| `regulate:` | Ashby (1956) homeostasis | Compile-time invariant with a feedback-loop semantic |
| `evolve:` | Darwin + Lamarck | Versioned interface migration with chain/cycle checking |
| `senescence:` | Campisi (2001) | Resource decay over time — computable resource model |
| `canalization:` | Waddington (1942) | Developmental robustness: resistance to perturbation |
| `umwelt:` | von Uexküll (1909) | Perceptual boundary: what signals a being can detect |
| `memory: type episodic` | Tulving (1972) | Compile-time verification that episodic claims have a `journal:` source |
| M111 evolution vectors | Cosine similarity in 12-dim type lattice | Detects convergent evolution across beings |

The nervous system architecture (ADR-0007) extends this: ganglionic, spinal,
brainstem, and cortical monitoring tiers map to different response latencies and
authority levels.

## Alternatives Considered

| Alternative | Why not chosen |
|---|---|
| Pure category theory | Correct but inaccessible; biologists and domain experts cannot read Loom programs without a math PhD |
| Object-oriented metaphor | Classes/objects don't capture telos, regulation feedback, or evolutionary versioning |
| Process calculus (π-calculus) | Strong for concurrency, weak for lifecycle and identity |
| Actor model | Captures message passing, not biological self-organization or memory hierarchy |

## Consequences

- **Domain expressiveness**: agricultural, medical, ecological, and AI systems
  can be modeled in their native vocabulary. The compiler checks biological
  contracts (autopoietic closure, telos-regulate alignment) that OOP cannot.
- **Contributor barrier**: developers unfamiliar with biology must read ADR-0005
  and the checker academic citations before adding constructs. This is intentional.
- **Novel construct cost**: every new M-milestone requires identifying the
  scientific literature, mapping it to a compile-time check, and citing the
  primary source in the checker's doc comment.
- **AI-aware**: when generating new Loom constructs, ground them in biological
  or mathematical literature. A construct without a scientific citation is a
  naming choice, not a semantic addition. Propose the citation in the PR description.
