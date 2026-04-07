# BIOISO GS Attributes

Every BIOISO entity (a `being:` block) is required to satisfy the seven GS properties
plus three additional properties discovered during Loom's biological layer development.
This document maps each property to its Loom primitive and milestone.

---

## The Seven GS Properties (source: GS White Paper §86)

Each property names a specific failure mode. The primitive is the structural prevention.

| Property | Failure mode prevented | Loom primitive | Milestone |
|---|---|---|---|
| **Self-describing** | Agent did not know the system's own conventions | `telos:`, `soul:`, `describe:`, `manifest:` | M101 |
| **Bounded** | Agent modified code outside the feature's scope | `umwelt:`, `boundary:`, session types | M103, M98 |
| **Verifiable** | Output was untestable | `require:`/`ensure:`, 11 checkers, SMT bridge | M100 |
| **Defended** | Agent could commit broken or harmful code | `@bounded_telos`, `@corrigible`, `@sandboxed`, `@mortal`, info-flow lattice, AOP | M55, M66 |
| **Auditable** | Decisions left no trace; AI "improved" intentional choices | `@transparent`, `journal:`, `provenance:` | M102, M104 |
| **Composable** | Coupled modules that should have been independent | Effects, sessions, `boundary:`, AOP | M98, M99, M103 |
| **Executable** | Generated code that never ran against a live environment | `scenario:` (being-level acceptance criteria) | M105 |

---

## Three Additional Properties (discovered)

Beyond the seven GS standard properties, BIOISO entities have specific requirements
arising from their biological grounding and the demands of long-lived autonomous agents.

### 8. Evolvable
**Definition:** A being can change its interface between versions without silently breaking
dependents. Every breaking change is declared, not discovered.

**Why BIOISO needs it:** An autonomous agent that spawns children, mutates via `crispr:`,
and runs for arbitrary time must be able to evolve without orphaning its ecosystem.
This is the biological equivalent of mutation + selection: change must be heritable and
backward-compatible or explicitly breaking with an adapter path.

**Loom primitive:** `migration:` block — declares interface changes with `from:/to:/adapter:` fields.

```loom
being TradingAgent
  migration v1_to_v2:
    from: sense_interval Float<seconds>
    to:   sense_interval Duration
    adapter: fn v1 -> Duration::from_seconds(v1) end
  end
end
```

**Milestone:** M106

---

### 9. Minimal
**Definition:** The specification contains only what is load-bearing. No unused sense channels,
no unreferenced regulate: bounds, no declared telos fields that are never evolved toward,
no imported interfaces with zero implementations. Dead declarations are as harmful as dead code —
they consume token budget and mislead the stateless reader.

**Why BIOISO needs it:** The user's own formulation: *"minimal should be something we evolve to,
discard unused stuff."* A being that accumulates unused declarations over generations becomes
unreadable. Biological systems under selection pressure shed metabolically expensive machinery
that no longer contributes to survival. Loom should enforce the same pressure at compile time.

**Loom primitive:** Minimal checker — rejects beings with unreachable or unused declared elements.

```
error: sense channel `infrared` is declared but never read in any evolve: or regulate: block
  --> src/trader.loom:14
hint: remove the sense channel or add a regulation that uses it
```

**Milestone:** M107

---

### 10. Diagram-emitting
**Definition:** The being's structure produces correct relationship-memory artifacts
automatically — diagrams cannot drift from code because they are derived from it.

**Why BIOISO needs it:** GS requires relationship memory (C4, sequence, state, flow diagrams)
as committed artifacts. If diagrams are written by hand, they drift. Loom's compile pipeline
must be able to emit them from the program's own structure:
- C4 container diagram from module/being hierarchy
- Sequence diagram from `session` types (M98)
- State diagram from `lifecycle:` / `typestate:` declarations
- Flow diagram from `lifecycle:` step ordering

Output format: Mermaid (plain text, version-controllable, renders on GitHub).

```loom
-- compile_mermaid_sequence() on a being with session types emits:
-- sequenceDiagram
--   participant Exchange
--   participant Agent
--   Exchange->>Agent: MarketData
--   Agent->>Exchange: OrderRequest
--   Exchange->>Agent: Fill
```

**Milestone:** M108

---

## Coverage Matrix

```
Property          | Fully covered | Partial  | Gap → Milestone
──────────────────┼───────────────┼──────────┼────────────────
Self-describing   |               | ✓ telos: | M101 manifest:
Bounded           |               | ✓ umwelt | M103 boundary:
Verifiable        | ✓             |          |
Defended          | ✓             |          |
Auditable         |               | ✓ @trans | M102, M104
Composable        |               | ✓ M98/99 | M103
Executable        |               |          | M105 scenario:
─ ─ ─ ─ ─ ─ ─ ─ ─┼─ ─ ─ ─ ─ ─ ─ ─┼─ ─ ─ ─ ─ ─┼─ ─ ─ ─ ─ ─ ─ ─
Evolvable         |               |          | M106 migration:
Minimal           |               |          | M107 checker
Diagram-emitting  |               |          | M108 Mermaid emit
```

Additionally:
- **M109** `property:` — language-level `forall(x: T)` property-based testing primitive
- **M110** `usecase:` — triple derivation (contract + test stubs + docs from one block)

---

## Enforcement rule

A `being:` block that is `autopoietic: true` MUST satisfy all 10 properties.
The compiler enforces 1–9 structurally. Property 10 (diagram-emitting) is enforced by
`compile_mermaid_*()` targets being available and referenced from `manifest:`.

Non-autopoietic beings must satisfy properties 1–7 minimum.
