# State Machine: BIOISO Entity Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Spawned : loom runtime spawn / inherit_from(parent_id)

    Spawned --> Active : genome hash registered in Signal Store;\nwarm_start_params() applied if offspring

    Active --> Canary : Mutation Gate approves proposal;\nCanary Deployer begins soft-release window

    Canary --> Stable : observation window passes;\nSurvival Gauntlet (CAE + LTE) passed

    Canary --> Active : auto-rollback on canary regression\nor gauntlet failure

    Stable --> Active : next drift cycle detected;\nnew evolution tick begins

    Active --> Rollback : Mutation Gate rejects proposal\n(compile error or type mismatch)

    Rollback --> Active : previous checkpoint restored\nfrom Signal Store

    Active --> Quarantined : Membrane security event\n(genome hash mismatch or rate-limit breach)

    Quarantined --> Active : security clearance granted;\nquarantine window expired

    Active --> Hibernating : Circadian gate closes\n(known low-quality window: market close,\nseasonal forcing, startup transient)

    Hibernating --> Active : Circadian gate opens;\nwake signal received

    Stable --> Senescent : telomere counter exhausted\n(@mortal annotation threshold reached)

    Active --> Senescent : telomere counter exhausted

    Senescent --> Dead : graceful shutdown;\nfinal epigenome snapshot written to Signal Store

    Dead --> [*]

    note right of Active
        Drift Engine runs every tick.
        Epigenome Buffer tier receives all signals.
        Polycephalum (Tier 1) fires on drift ≥ threshold.
        Circadian gate may suppress synthesis — never admission.
    end note

    note right of Quarantined
        Stage 0 Membrane handles quarantine.
        Token-bucket rate limit and SHA-256
        genome hash validation enforced.
        Epigenome Security tier audit entry written.
    end note

    note right of Stable
        Mycelium gossip broadcasts the
        promoted mutation to colony peers.
        ACO pheromone deposited for
        high-fitness path reinforcement.
    end note
```
