# Loom Full Software Lifecycle Specification

> **One `.loom` file drives everything.**  
> From design intent → implementation → deployment → observation → adaptation → self-healing.

---

## The Lifecycle Loop

```
┌─────────────────────────────────────────────────────────────────────┐
│                     LOOM SOURCE (.loom)                             │
│              The single source of truth for everything              │
└─────────┬───────────────────────────────────────────────────────────┘
          │
    ┌─────▼──────┐    ┌──────────┐    ┌──────────┐    ┌────────────┐
    │   DESIGN   │───▶│  BUILD   │───▶│   TEST   │───▶│   DEPLOY   │
    │            │    │          │    │          │    │            │
    │ Types      │    │ Rust     │    │ unit     │    │ Dockerfile │
    │ Contracts  │    │ TypeScript    │ contract │    │ K8s        │
    │ Effects    │    │ WASM     │    │ chaos    │    │ Terraform  │
    │ Protocols  │    │ OpenAPI  │    │ load     │    │ ArgoCD     │
    └────────────┘    └──────────┘    └──────────┘    └─────┬──────┘
                                                            │
    ┌─────────────────────────────────────────────────────┐ │
    │                                                     │ │
    │  ◀── EVOLVE ◀── SELF-HEAL ◀── ADAPT ◀── OBSERVE ◀──┘ │
    │                                                        │
    │  Schema      Circuit      Scale-out   Metrics          │
    │  migration   breaker      on SLO      Traces           │
    │  AI patch    Rollback     Feature     Logs             │
    │  PR + merge  fallback     flags       Dashboards       │
    └────────────────────────────────────────────────────────┘
```

---

## Phase 1 — Design (M1–M23, complete)

Already implemented: types, effects, contracts, units of measure, privacy labels, algebraic properties, typestate, information flow.

---

## Phase 2 — Build Targets (M24–M26)

### New emission targets

| Target | Triggered by | Output |
|--------|-------------|--------|
| Dockerfile | `@service` annotation | Multi-stage build, distroless final image |
| docker-compose | `@depends-on` | Local dev compose file with all dependencies |
| Kubernetes | `@service` + `@resource` + `@depends-on` | Deployment, Service, ConfigMap, HPA |
| Helm Chart | `@service` + `@environment` | Full chart with values per environment |
| Terraform | `@region` + `@resource` | Cloud provider resources (AWS/GCP/Azure) |
| GitHub Actions | `test:` blocks + `@environment` | CI pipeline: build → test → push → deploy |

### Syntax

```loom
module PaymentService
describe: "Handles payment processing with full audit trail"
@version(2)
@service(port=8080, protocol=http)
@environment(prod, staging, dev)
@region("us-east-1", "eu-west-1")
@resource(cpu="500m", memory="256Mi", replicas=3)
@depends-on(PostgresDatabase, RedisCache, StripeGateway)
@image("ghcr.io/pragmaworks/payment-service")
...
end
```

From `@resource(cpu="500m", memory="256Mi", replicas=3)` + `@depends-on(PostgresDatabase)` Loom derives:

```yaml
# derived k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: payment-service
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: payment-service
          resources:
            requests: { cpu: "500m", memory: "256Mi" }
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef: { name: postgres-credentials, key: url }
```

The `@depends-on` list drives: K8s service references, health check dependencies, local docker-compose links, Terraform module dependencies.

---

## Phase 3 — Test Targets (M27–M29)

### Three new block types

```loom
module PaymentService

-- Unit + integration tests (already M1–M23)
test create_payment_requires_positive_amount ::
  create_payment(-1.0 : Float<usd>) fails_with InvalidAmount
end

-- Consumer-driven contract tests (Pact-style)
contract MobileAppConsumer ::
  calls: create_payment with TransferRequest
  expects: Payment within 200ms
  expects: status 201
  does_not_use: card_number   -- never leak PCI field
end

-- Chaos resilience tests
chaos network_partition_30s ::
  given: StripeGateway unavailable for 30s
  expect: fallback to queued_payment within 5s
  expect: no data loss
  expect: retry on reconnect
end

-- Load / performance tests  
load peak_checkout_traffic ::
  ramp: 0 → 1000 users over 60s
  sustain: 1000 users for 300s
  assert: p99 latency < 200ms
  assert: error rate < 0.1%
end
```

Emits:
- `contract:` → Pact JSON files + provider verification tests
- `chaos:` → Chaos Monkey / Toxiproxy / LitmusChaos experiment YAML
- `load:` → k6 / Locust scripts derived from OpenAPI

**Academic reference:** Consumer-Driven Contracts (Robinson, 2006). Chaos Engineering (Basiri et al., Netflix 2016). Both described as essential, both absent from type systems.

---

## Phase 4 — Deploy Targets (M30–M31)

### Canary and blue-green as types

```loom
module PaymentService
@version(2)
@migrates-from(version=1, migration=migrate_payment_v1_to_v2)
@canary(weight=5%, promote_after=1h, metric="error_rate < 0.1%")
@rollback-on(error_rate > 2%, latency_p99 > 500ms)
```

Emits:
- Argo Rollouts `Rollout` manifest with canary steps
- Flagger `Canary` object with promotion criteria
- GitHub Actions deployment workflow with gate checks
- Slack/PagerDuty notification hooks on promotion/rollback

### Environment-specific config as types

```loom
config PaymentService
  prod  :: database_pool_size = 20, cache_ttl = 300s, log_level = warn
  staging :: database_pool_size = 5,  cache_ttl = 60s,  log_level = info
  dev   :: database_pool_size = 2,  cache_ttl = 10s,  log_level = debug
end
```

Emits: K8s ConfigMaps per environment, Helm values files, `.env` for local dev.

---

## Phase 5 — Observe (M32–M34)

The three pillars of observability (Metrics, Traces, Logs) are derived automatically from the type system — not bolted on afterwards.

### Metrics — derived from SLO types

```loom
module PaymentService
@slo(p99=200ms, p50=50ms, availability=0.9999, error_budget_monthly=0.01%)
@alert(latency_p99 > 500ms for 2min  → page oncall)
@alert(error_rate > 1%    for 5min  → page oncall)
@alert(availability < 0.999 for 1h → notify manager)
@dashboard("https://grafana.internal/d/payment-service")
```

Emits:
- Prometheus recording rules (SLI computation)
- Prometheus alerting rules (SLO breach detection)
- Grafana dashboard JSON (RED method: Rate, Errors, Duration per endpoint)
- PagerDuty service + escalation policy

### Traces — derived from Effect types

Every function with an `Effect<[IO, DB, Cache], T>` return type gets automatic OpenTelemetry span wrapping. The effect chain is the trace — no `#[instrument]` annotations needed.

```loom
fn process_order @trace("order.process")
  :: OrderId -> Effect<[DB, Cache, Payment], Order]
  -- Effect chain DB → Cache → Payment becomes:
  -- span: order.process
  --   ├─ span: db.query
  --   ├─ span: cache.get  
  --   └─ span: payment.charge
```

Emits: OTel SDK instrumentation code in Rust + TypeScript output, trace context propagation headers, Jaeger/Zipkin/Tempo configuration.

### Logs — derived from describe: + privacy labels

```loom
fn create_payment
  describe: "Creates a new payment record and initiates charge"
  :: Float<usd> -> BankToken -> Effect<[Payment], Payment<Pending>]
```

Emits: structured log entry at function entry/exit with auto-redacted `@never-log` fields, correlation ID propagation, log level from `@environment`.

### Health checks — derived from @depends-on

`@depends-on(PostgresDatabase, RedisCache)` emits a `/health` endpoint that:
- Checks DB connection with timeout
- Checks Redis ping with timeout
- Returns `{ status: "healthy"|"degraded"|"unhealthy", checks: {...} }`
- K8s `livenessProbe` + `readinessProbe` derived from the same

---

## Phase 6 — Adapt: The MAPE-K Loop (M35–M36)

**Academic reference:** IBM Autonomic Computing (Kephart & Chess, 2003). The Monitor-Analyze-Plan-Execute over Knowledge base loop. Described as the future of software systems. Never shipped as a language construct.

```loom
module PaymentService

adapt:
  -- Scale-out on latency SLO breach
  signal: slo_breach(metric=latency_p99, threshold=200ms)
  response: scale_out(max_replicas=20, cooldown=5min)

  -- Circuit break on dependency failure
  signal: error_rate(StripeGateway) > 5% for 30s
  response: circuit_break(
    fallback = queue_for_retry,
    timeout = 30s,
    half_open_after = 60s
  )

  -- Feature flag degradation
  signal: latency_p99 > 400ms for 1min
  response: disable_feature("payment_fraud_ml_check")

  -- Automatic rollback on canary health
  signal: canary_error_rate > 2%
  response: rollback(to_version=stable, notify=oncall)
end
```

Emits:
- HPA (Horizontal Pod Autoscaler) YAML with custom metrics
- Resilience4j / Hystrix circuit breaker configuration
- LaunchDarkly / Unleash feature flag rules
- Argo Rollouts analysis templates for canary health gates

The `adapt:` block is a **typed feedback control loop**. It is checked: signals must reference declared SLO metrics or known dependency names. Responses must be compatible with the service's declared capabilities (you can't `scale_out` if you haven't declared `@resource`).

---

## Phase 7 — Evolve: Schema + API Versioning (M37)

```loom
module PaymentService
@version(2)
@migrates-from(version=1)
@sunset(version=1, after=2025-06-01, notify="API-consumers mailing list")

type Payment @version(2) =
  id: Int
  amount: Float<usd>
  currency: Currency         -- NEW in v2
  status: PaymentStatus
  card_number: String @pci @never-log @encrypt-at-rest
end

migrate Payment v1 -> v2 ::
  currency = usd             -- default all v1 records to USD
end
```

Emits:
- SQL migration script (Flyway/Liquibase format)
- Database migration Rust code (`sqlx` migrate)
- API versioning headers (`Accept: application/vnd.loom.v2+json`)
- OpenAPI with `deprecated: true` on v1 endpoints + sunset headers
- CHANGELOG.md entry

**Version-aware types** mean the compiler enforces migration completeness: if `Payment` v2 adds a `currency` field, the `migrate:` block must account for every new required field. Missing migration → compile error.

---

## Phase 8 — Self-Heal: AI-in-the-Loop (M38)

This is the most novel phase and the one that closes the loop entirely. The Loom spec is the AI's memory across sessions (GS derivability constraint). A SLO breach or production failure becomes an input to the compiler.

```loom
module PaymentService

self-heal:
  -- SLO breach → AI regenerates implementation
  on: slo_breach(latency_p99, exceeded_by=2x, duration=15min)
  action: ai_patch(
    context = [recent_traces, flame_graph, error_logs],
    constraint = "must not change public API",
    review = required,           -- generates PR, requires human approval
    test_gate = "all tests pass + slo_simulated_ok"
  )

  -- Repeated error pattern → AI proposes circuit breaker
  on: error_pattern(type=TimeoutError, rate=10%, window=5min)
  action: ai_suggest(
    target = adapt_block,
    proposal = "add circuit_break for dependency",
    notify = oncall
  )

  -- Data schema drift → AI generates migration
  on: schema_drift(source=production_db, target=Payment)
  action: ai_patch(
    context = [current_schema, target_schema],
    output = migration_script,
    review = required
  )
end
```

The `self-heal:` block is declarative intent. When a signal fires:
1. The Loom runtime emits a webhook with the signal context
2. A CI/CD-integrated AI agent (GitHub Copilot / Copilot Workspace) is invoked
3. The agent reads the `.loom` file — this is its full context, the digital twin
4. The agent generates a patch (code change, adapt block update, migration script)
5. A PR is created with: changed `.loom` source + all derived artifacts regenerated
6. The declared `test_gate` runs; if it passes and `review = optional`, auto-merge

**Why this works:** The `.loom` file contains everything the AI needs to regenerate correct output. It is the GS mold. The AI is stateless but the spec is complete — GS property 1: Self-describing, GS property 7: Executable.

---

## Complete Annotated Module Example

```loom
-- ── IDENTITY ──────────────────────────────────────────────────────
module PaymentService
describe: "Handles payment processing with full audit trail"
@version(2)
@migrates-from(version=1)

-- ── INFRASTRUCTURE ────────────────────────────────────────────────
@service(port=8080, protocol=http)
@environment(prod, staging, dev)
@region("us-east-1", "eu-west-1")
@resource(cpu="500m", memory="256Mi", replicas=3)
@depends-on(PostgresDatabase, RedisCache, StripeGateway)
@image("ghcr.io/pragmaworks/payment-service")

-- ── DEPLOYMENT ────────────────────────────────────────────────────
@canary(weight=5%, promote_after=1h, metric="error_rate < 0.1%")
@rollback-on(error_rate > 2%, latency_p99 > 500ms)

-- ── OBSERVABILITY ─────────────────────────────────────────────────
@slo(p99=200ms, p50=50ms, availability=0.9999)
@alert(latency_p99 > 500ms for 2min → page oncall)
@alert(error_rate > 1% for 5min → page oncall)

-- ── SEMANTIC TYPES ────────────────────────────────────────────────
flow secret :: CardNumber, CVV, BankToken
flow tainted :: WebhookPayload

lifecycle Payment :: Pending -> Authorized -> Captured -> Refunded

type Payment =
  id:          Int
  amount:      Float<usd>
  card_number: String @pci @never-log @encrypt-at-rest
  status:      PaymentStatus
end

enum PaymentError = | NotFound | InvalidAmount | InsufficientFunds | GatewayTimeout end

-- ── FUNCTIONS ─────────────────────────────────────────────────────
fn create_payment @exactly-once @trace("payment.create")
  :: Float<usd> -> BankToken -> Effect<[DB, Payment], Payment<Pending>]
  require: amount > 0.0
  ensure: result.amount == amount
  amount
end

fn capture_payment @idempotent @trace("payment.capture")
  :: Payment<Authorized> -> Effect<[DB, StripeGateway], Payment<Captured>]
  payment
end

-- ── CONTRACTS ─────────────────────────────────────────────────────
contract MobileConsumer ::
  calls: create_payment with TransferRequest
  expects: Payment within 200ms
  does_not_use: card_number
end

-- ── CHAOS ─────────────────────────────────────────────────────────
chaos gateway_failure ::
  given: StripeGateway unavailable for 30s
  expect: fallback to queued_payment
  expect: no data loss
end

-- ── LOAD ──────────────────────────────────────────────────────────
load checkout_peak ::
  ramp: 0 → 500 users over 60s
  assert: p99 latency < 200ms
  assert: error_rate < 0.1%
end

-- ── ADAPTATION ────────────────────────────────────────────────────
adapt:
  signal: slo_breach(latency_p99)
  response: scale_out(max_replicas=20, cooldown=5min)

  signal: error_rate(StripeGateway) > 5% for 30s
  response: circuit_break(fallback=queue_for_retry, timeout=30s)
end

-- ── SELF-HEALING ──────────────────────────────────────────────────
self-heal:
  on: slo_breach(latency_p99, exceeded_by=2x, duration=15min)
  action: ai_patch(constraint="no public API changes", review=required)
end

end
```

**Single file. Every artifact derived.**

---

## Full Emission Matrix

| Loom construct | Rust | TypeScript | WASM | OpenAPI | JSON Schema | K8s | Terraform | CI/CD | Grafana | Pact | k6 | Chaos |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `type` | struct | interface | - | schema | schema | - | - | - | - | - | - | - |
| `fn` | fn | function | export | path | - | - | - | - | - | - | ✓ | - |
| `effect` | Result | Promise | - | x-effects | - | - | - | - | - | - | - | - |
| `@service` | - | - | - | info | - | Deployment+Service | module | workflow | - | - | - | - |
| `@resource` | - | - | - | - | - | resources+HPA | var | - | - | - | - | - |
| `@depends-on` | - | - | - | - | - | env+probe | depends_on | - | - | - | - | - |
| `@slo` | - | - | - | x-slo | - | - | - | gate | dashboard | - | assert | - |
| `@alert` | - | - | - | - | - | PrometheusRule | - | - | alert | - | - | - |
| `@canary` | - | - | - | - | - | Rollout | - | workflow | - | - | - | - |
| `@exactly-once` | doc | JSDoc | - | x-retry | - | - | - | - | - | - | - | - |
| `flow secret` | branded | branded | - | x-sensitivity | x-sensitivity | - | - | - | - | - | - | - |
| `lifecycle` | phantom | union | - | x-lifecycle | enum | - | - | - | - | - | - | - |
| `@pci` | attr | JSDoc | - | x-pci | x-pci | - | - | - | - | - | - | - |
| `contract:` | - | - | - | - | - | - | - | - | - | Pact JSON | - | - |
| `chaos:` | - | - | - | - | - | - | - | - | - | - | - | LitmusChaos |
| `load:` | - | - | - | - | - | - | - | - | - | - | k6 script | - |
| `adapt:` | - | - | - | - | - | HPA+CB config | - | - | - | - | - | - |
| `self-heal:` | - | - | - | - | - | - | - | AI webhook | - | - | - | - |
| `@trace` | OTel span | OTel span | - | - | - | - | - | - | trace | - | - | - |

---

## Academic References for New Phases

| Phase | Construct | Origin | Never shipped because |
|---|---|---|---|
| Build | Infrastructure as types | Terraform (2014) + K8s (2014) | Separate tools, no type connection to code |
| Test | Consumer-driven contracts | Robinson (2006) | Tooling fragile, not language-native |
| Test | Chaos engineering | Netflix (2016) | Separate runbook, not typed intent |
| Observe | SLO as types | Google SRE Book (2016) | Config files, no compiler enforcement |
| Observe | OTel from effects | OpenTelemetry (2019) | Manual instrumentation, no type inference |
| Adapt | MAPE-K feedback loop | IBM Kephart & Chess (2003) | No language-level construct, only middleware |
| Adapt | Control theory in software | Hellerstein et al. (2004) | Mathematical — never crossed into PLT |
| Evolve | Typed schema versioning | Avro/Protobuf (partial) | Not integrated with application types |
| Self-heal | Autonomic computing | IBM (2001) | Required human-in-the-loop; AI removes this barrier |

---

## Proposed Milestone Index (Phase 8–12)

| M | Phase | Feature | Key emission targets |
|---|---|---|---|
| M24 | Session Types | Multi-party protocol choreography | OpenAPI + AsyncAPI + state machines |
| M25 | RDF/OWL | Ontology emission target | Turtle, JSON-LD, SHACL, SPARQL |
| M26 | Temporal Logic | `@always`, `@eventually`, `@leads-to` | TLA+ specs, model checker input |
| M27 | Build targets | `@service`, `@resource`, `@depends-on` | Dockerfile, K8s, Terraform, Helm |
| M28 | CI/CD emission | `@environment`, `@canary`, `@rollback-on` | GitHub Actions, Argo Rollouts |
| M29 | Contract tests | `contract:` block | Pact JSON, provider verification tests |
| M30 | Chaos tests | `chaos:` block | LitmusChaos, Toxiproxy YAML |
| M31 | Load tests | `load:` block | k6 scripts from OpenAPI |
| M32 | SLO observability | `@slo`, `@alert` | Prometheus rules, Grafana dashboards |
| M33 | Trace emission | `@trace` + Effect types | OpenTelemetry SDK instrumentation |
| M34 | Health checks | `@depends-on` → `/health` | K8s probes, liveness/readiness |
| M35 | MAPE-K adapt | `adapt:` block | HPA, circuit breaker, feature flags |
| M36 | Schema evolution | `@version`, `migrate:` | SQL migrations, API version headers |
| M37 | Self-heal | `self-heal:` block | AI webhook, PR generation, test gates |
| M38 | Differential Privacy | `@dp(ε=...)` | Noise injection, budget checker |
| M39 | Dependent types (lite) | `NonEmpty<T>`, `Ranged<lo,hi,T>` | Eliminate runtime panics |
| M40 | CRDT types | `@crdt(or-set)`, `@crdt(lww)` | Merge functions, sync protocol |

---

## The Core Claim

Every existing tool in the stack — Terraform, Kubernetes, Prometheus, Grafana, Pact, k6, LitmusChaos, Argo Rollouts, OpenTelemetry, Flyway, PagerDuty — is already doing the right thing. What they have never had is a **typed source of truth** that they all derive from simultaneously.

When you change a type in the Loom spec, every artifact regenerates: the Rust impl, the TypeScript SDK, the OpenAPI, the K8s deployment, the Grafana dashboard, the Pact contract, the chaos experiment. The spec is the system. The system is the spec.

This is what the GS white paper calls **a mold**: a complete specification from which all correct artifacts are derived, by any sufficiently capable reader — human or AI.
