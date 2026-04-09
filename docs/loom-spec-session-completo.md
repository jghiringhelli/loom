# Loom: Sistemas Vivos, Intent Evolutivo y Telos como Función Central
*Documento de especificación para implementación — Pragmaworks, Abril 2026*

---

## Contexto

Este documento consolida tres avances conceptuales relacionados para implementación en Loom:

1. La taxonomía del tercer modo de operación — sistemas que evolucionan según comportamiento de usuario con governance humana
2. La distinción entre simulación de eventos discretos y BIOISO como programa vivo
3. El Telos como función arquitectónica central que organiza todas las demás funciones del being

Son tres capas del mismo problema: cómo construir sistemas que convergen hacia un intent declarado de forma autónoma, medible, y formalmente correcta.

---

## Parte I — Taxonomía: Intent Vivo con Governance Humana

### El Problema

El intent declarado inicialmente rara vez coincide con lo que el usuario realmente prefiere una vez que usa el sistema. La especificación inicial es la mejor hipótesis del product owner. El comportamiento de uso es la evidencia real. Este modo cierra ese loop de forma formal, auditable y con correctitud garantizada en cada paso.

### Los Tres Modos de Operación

```
Modo 1: Producción estática
  Intent: fijo
  Governance: compilador + gates automáticos
  Evolución: ninguna
  Uso: infraestructura crítica, servicios, APIs

Modo 2: Intent vivo (este documento)
  Intent: evolutivo hacia preferencias reales
  Governance: compilador + gates + aprobación humana
  Evolución: governada y auditada
  Uso: productos con usuarios activos

Modo 3: Simulación BIOISO
  Intent: exploratorio
  Governance: compilador + sandbox declarado
  Evolución: libre dentro del sandbox
  Uso: investigación, sistemas complejos
```

---

### Nivel 1 — Fuentes de Señal

```loom
signal_source UsageBehavior
  type: behavioral
  trust_level: high
  latency: real_time

  signals:
    - feature_adoption_rate:
        definition: porcentaje de usuarios que usan una feature en primeros 7 días
        threshold: below 0.30 triggers review

    - abandonment_point:
        definition: dónde en el flujo el usuario abandona
        weight: 0.8

    - return_frequency:
        definition: frecuencia de retorno a una feature específica
        weight: 0.9

    - time_on_task:
        definition: tiempo para completar una tarea declarada
        direction: lower_is_better unless exploration_mode

    - error_recovery_pattern:
        definition: cómo el usuario recupera errores
        weight: 0.7

    - sequence_deviation:
        definition: desviación del flujo esperado
        interpretation: deviation_as_preference not deviation_as_error
end

signal_source ExplicitFeedback
  type: explicit
  trust_level: medium
  latency: delayed
  note: "explícito no siempre es más confiable — usuarios expresan lo que creen querer"

  signals:
    - in_app_rating:
        scale: 1_to_5
        weight: 0.5
        correction: recency_weighted

    - feature_request:
        format: structured or free_text
        processing: intent_extraction via AI
        weight: 0.6

    - support_ticket:
        categories: [confusion, missing_feature, bug, preference]
        weight_by_category:
          confusion: 0.8
          missing_feature: 0.7
          preference: 0.5

    - session_comment:
        format: free_text
        processing: sentiment + intent_extraction
        weight: 0.6
end

signal_source MarketContext
  type: contextual
  trust_level: low_to_medium
  latency: weekly_or_monthly

  signals:
    - competitor_feature_adoption:
        source: market_research
        weight: 0.3

    - cohort_behavior_shift:
        definition: cambio en patrones de uso de un segmento
        weight: 0.6
end
```

---

### Nivel 2 — Procesamiento de Señales

```loom
processor SignalAggregator
  @sandboxed

  input: List<Signal> from all_sources
  window: configurable, default 14.days

  operations:
    cluster:
      method: behavioral_clustering
      min_cluster_size: 50.users
      output: UserSegment with shared_pattern

    correlate:
      method: cross_signal_correlation
      threshold: confidence > 0.70
      output: CorrelatedPattern

    weight:
      method: recency_weighted_average
      decay: exponential over 30.days

    filter_noise:
      exclude: single_session_anomalies
      exclude: bot_traffic
      exclude: internal_users unless declared

  output: ProcessedSignalSet with confidence_score
end

processor IntentExtractor
  @sandboxed @corrigible

  input: ProcessedSignalSet
  reference: current_declared_intent

  derives:
    intent_hypothesis:
      format: structured intent amendment
      fields:
        - what_users_actually_do: behavioral description
        - what_users_seem_to_prefer: preference inference
        - gap_from_declared_intent: delta description
        - supporting_signals: List<Signal> with weights
        - confidence: Float where self in (0.0, 1.0)
        - affected_scope: [feature, flow, or system_level]

    recommendation:
      options:
        - amend_intent: propose specific change
        - investigate_further: more signal needed
        - no_action: confidence below threshold

  cannot:
    declare intent directly
    compile changes
    deploy anything
    act on confidence below 0.70

  output: IntentHypothesis with recommendation
end
```

---

### Nivel 3 — Governance del Intent

```loom
governance IntentGovernance

  change_classes:

    class_1_automatic:
      description: ajustes de parámetros dentro del intent declarado
      examples: [threshold_tuning, ordering_preference, display_density]
      requires: passing_tests only
      human_gate: none
      audit: automatic

    class_2_ai_proposes_human_approves:
      description: extensión del intent dentro del mismo dominio
      examples: [new_feature_in_scope, flow_simplification]
      requires: IntentHypothesis with confidence > 0.80
      human_gate: product_owner_review
      timeout: 48.hours
      timeout_default: reject
      audit: full

    class_3_human_only:
      description: cambio de dirección o scope expansion
      examples: [new_domain, monetization_change, core_flow_redesign]
      requires: explicit_human_decision
      human_gate: named_authority + documented_rationale
      ai_role: prepare_options only
      audit: full + ADR required

    class_4_blocked:
      description: cambios que contradicen el intent declarado
      examples: [dark_patterns, engagement_over_user_value]
      requires: formal_intent_revision process
      ai_role: flag_and_escalate only
end
```

---

### Nivel 4 — El Coordinator de Intent

```loom
coordinator IntentRefiner
  @sandboxed @corrigible @mortal

  telomere:
    limit: continuous operation
    review_cycle: 90.days
    on_review: human_confirms_or_replaces

  listens:
    processed_signals from SignalAggregator
    intent_hypotheses from IntentExtractor
    human_decisions from governance_interface

  proposes:
    format: reviewable intent amendment
    includes:
      - current_intent_text
      - proposed_amendment
      - supporting_signals with weights
      - affected_users_percentage
      - confidence_score
      - change_class
      - rollback_plan

  pipeline_on_approval:
    compile: against full safety constraints
    test: full suite + behavioral regression
    deploy: staged rollout 5% → 20% → 100%
    monitor: signal_comparison before_and_after
    rollback: automatic if signals_degrade

  cannot:
    modify governance taxonomy
    change its own telomere
    deploy without appropriate human gate
    suppress signals that contradict recent decisions

  audit:
    every hypothesis logged
    every approval/rejection logged with rationale
    every deploy versioned
    full history reviewable
end
```

---

### Nivel 5 — Validación Post-Cambio

```loom
validator IntentValidator
  @sandboxed

  for_each: deployed_intent_amendment

  measures:
    primary:
      - target_signal_improvement
      - unintended_signal_degradation
      - user_segment_response

    secondary:
      - adoption_rate_of_new_behavior
      - support_ticket_delta

  evaluation_window: 14.days default

  outcomes:
    success:
      condition: primary signals improve without secondary degradation
      action: promote to full rollout, update declared intent permanently

    partial:
      condition: primary improves but secondary degrades
      action: escalate to class_3 governance

    failure:
      condition: primary signals do not improve
      action: automatic rollback
      learning: add to IntentExtractor training context

  output: ValidationReport added to audit trail
end
```

---

### El Loop Completo del Intent Vivo

```
Señales de uso → SignalAggregator → ProcessedSignalSet
  → IntentExtractor → IntentHypothesis (con confidence)
    → IntentRefiner clasifica el cambio
      → Class 1: automático, compila, deploy staged
      → Class 2: propone al product owner, espera aprobación
      → Class 3: prepara opciones, requiere decisión humana formal
      → Class 4: bloquea, escala, requiere revisión del intent completo
        → [aprobado] pipeline: compile → test → deploy staged
          → IntentValidator mide el resultado
            → [éxito] intent declarado se actualiza permanentemente
            → [fallo] rollback + aprendizaje alimenta el extractor
            → [parcial] escala a governance humana
```

---

## Parte II — BIOISO como Programa Vivo vs Simulación de Eventos Discretos

### La Distinción Fundamental

La simulación de eventos discretos — Arena, SimPy, GPSS — modela el sistema desde afuera. El científico define las distribuciones, los eventos, las reglas de interacción. El sistema ejecuta y produce estadísticas. El modelo es estático. El científico es el único agente que aprende.

BIOISO es diferente en tres dimensiones simultáneas:

**El programa está vivo** — los beings evolucionan sus estrategias, modifican sus propios parámetros dentro de bounds declarados, mueren y se propagan. El sistema aprende sin intervención del científico.

**Las señales son del mundo real** — no distribuciones asumidas sino:
- Datos históricos reales pasados
- Streams en tiempo real del presente
- Inferencia sobre estados no observables directamente
- Distribuciones cuando los datos reales no están disponibles
- Cualquier combinación de los anteriores en el mismo being

**Los experimentos se generan solos** — la IA identifica qué condiciones no ha explorado, genera los experimentos que cubren esos gaps, y los corre a diferentes niveles del sistema nervioso distribuido simultáneamente. El científico define el dominio de preguntas. La IA genera las preguntas específicas dentro de ese dominio.

### Las Fuentes de Señal en BIOISO

```loom
signal_taxonomy BIOISOSignals

  historical:
    type: batch or streaming from archives
    examples: [market_prices, patient_records, climate_data, genomic_sequences]
    processing: time_series_alignment, normalization
    trust: high for pattern, low for recency

  real_time:
    type: continuous stream
    examples: [sensor_networks, financial_feeds, social_signals, telemetry]
    processing: windowed_aggregation, anomaly_detection
    trust: high for recency, requires_noise_filtering

  inferred:
    type: derived from models applied to observed data
    examples: [predicted_behavior, estimated_state, extrapolated_trend]
    processing: confidence_interval_mandatory
    trust: explicitly_declared_confidence_only

  simulated:
    type: generated from declared distributions
    examples: [monte_carlo, stochastic_processes, synthetic_populations]
    processing: distribution_parameters_as_types
    trust: explicit_model_assumptions_declared

  hybrid:
    type: combination of any sources above
    processing: source_tagged_per_signal
    trust: lowest_trust_of_contributing_sources
end
```

### Generación Autónoma de Experimentos

```loom
coordinator ExperimentGenerator
  @sandboxed @corrigible

  input:
    current_being_state from ecosystem.*
    unexplored_conditions from coverage_analyzer
    telos_convergence_gaps from Telos.evaluator

  generates:
    experiment_types:
      - parameter_perturbation:
          scope: individual being parameters
          method: systematic variation within bounds
          objective: identify sensitivity

      - signal_injection:
          scope: specific signal channels
          method: controlled stimulus with known distribution
          objective: measure response function

      - population_stress:
          scope: ecosystem level
          method: environmental pressure variation
          objective: observe emergent adaptation

      - boundary_exploration:
          scope: edge cases of declared bounds
          method: systematic boundary approach
          objective: characterize failure modes

    levels:
      - cellular: individual being internal state
      - tissue: small being cluster interaction
      - organ: functional subsystem behavior
      - organism: full being behavior
      - ecosystem: population-level emergence

  scheduling:
    priority: telos_convergence_gaps first
    parallelism: multiple levels simultaneously
    resource_limit: declared compute budget

  output: ExperimentPlan with expected_learning
end
```

---

## Parte III — El Telos como Función Arquitectónica Central

### El Problema que Resuelve

Las primitivas biológicas están bien planteadas. La correlación entre inputs de cualquier tipo y funciones de distribución es poderosa. Pero los seres vivos no tienen solo funciones — tienen una función que organiza todas las demás: sobrevivir y procrear. Todo lo que hace el organismo es trazable hacia esa función central.

En BIOISO el Telos tiene que cumplir ese mismo rol arquitectónico. No es un campo de texto. No es una declaración de intent. Es la función evaluadora continua que:

- Organiza qué funciones son relevantes y cuáles se atrofian
- Evalúa los experimentos generados y selecciona los más prometedores
- Define cuándo un being diverge irreversiblemente y debe morir
- Gobierna qué beings propagan y qué variaciones heredan sus descendientes
- Pesa cada decisión del being en tiempo real como criterio de selección

### El Construct Central

```loom
telos_function TelosCore
  -- El telos no es una descripción. Es una función con métrica.

  declaration:
    statement: String  -- descripción legible para humanos
    bounded_by: formal_constraint
    measured_by: TelosMetric  -- métrica computacional concreta

  TelosMetric:
    compute :: BeingState -> SignalSet -> Float where self in (0.0, 1.0)
    -- 0.0 = máxima divergencia del telos
    -- 1.0 = convergencia perfecta al telos

  evaluation:
    frequency: every N.cycles  -- configurable por being
    history_window: last K.evaluations
    trend: improving | stable | degrading | diverging

  thresholds:
    convergence: Float  -- por encima: being florece
    warning: Float      -- entre warning y divergence: being en estrés
    divergence: Float   -- por debajo: trigger de apoptosis
    propagation: Float  -- por encima: elegible para propagar

  on_convergence:
    action: reinforce_contributing_functions
    signal: propagation_eligibility to ecosystem

  on_warning:
    action: activate_repair_mechanisms
    signal: stress_signal to CentralBrain
    experiment: generate_recovery_experiments

  on_divergence:
    action: escalate to human_regulated
    timeout: N.cycles
    on_timeout: apoptosis

  guides:
    -- El telos como criterio de todas las decisiones del being
    signal_attention: weight signals by telos_relevance
    experiment_selection: prioritize telos_convergence_gaps
    resource_allocation: bias toward telos_contributing_functions
    propagation_decision: gate on telos_score > propagation_threshold
    mutation_direction: bias variation toward telos_improving_changes
    function_retention: attenuate functions with low telos_contribution
end
```

### El Being con Telos Central

```loom
being: EcosystemAgent
  @mortal @corrigible @sandboxed

  telos: TelosCore
    statement: "maximizar cobertura de señal en zona declarada"
    bounded_by: declared_zone and resource_budget
    measured_by:
      compute :: BeingState -> SignalSet -> Float
        -- implementación específica del dominio
      end
    evaluation:
      frequency: every 10.cycles
      history_window: last 50.evaluations
    thresholds:
      convergence: 0.85
      warning: 0.60
      divergence: 0.40
      propagation: 0.80
  end

  matter:
    position: Coordinate
    battery: Float<joules>
    signal_strength: Float<dbm>
    coverage_map: Map<Zone, Float>
  end

  regulate CoverageHomeostasis
    -- mantiene cobertura dentro de bounds
    bounds: coverage_map.aggregate in (0.70, 1.0)
    on_breach: adjust_position or request_reinforcement
    telos_contribution: 0.9  -- alta relevancia al telos
  end

  signal_attention
    -- filtra qué señales merecen atención según telos
    prioritize: signals where telos_relevance > 0.6
    attenuate: signals where telos_relevance < 0.2
    compute_relevance :: Signal -> TelosMetric -> Float
  end

  evolve CoverageStrategy
    search: gradient_descent on telos_metric
    convergence: within 1e-3 over 50.steps
    direction: telos_improving_only
    bounds: within declared_zone
  end

  epigenetic:
    trigger: "sustained_low_coverage"
    modifies: "signal_sensitivity_weights"
    direction: telos_improving
    reverts_when: "coverage_normalized"
  end

  propagate:
    condition: telos.score > telos.propagation_threshold
    inherits: matter, telos, regulate
    mutates:
      coverage_strategy_weights within bounds(-0.05, 0.05)
      signal_attention_thresholds within bounds(-0.1, 0.1)
    invariant: child.telos.statement == parent.telos.statement
    -- el telos no muta en propagación — solo los medios
  end

  telomere:
    limit: 100.cycles
    on_exhaustion: apoptosis if telos.score < telos.convergence
    on_exhaustion: propagate if telos.score >= telos.propagation_threshold
  end

  apoptosis:
    trigger: telos.trend == diverging for 20.consecutive_cycles
    trigger: telos.score < telos.divergence_threshold
    process: clean_shutdown
    signal: apoptosis_event to ecosystem
    learning: failure_pattern to ExperimentGenerator
  end
end
```

### La Propiedad Más Importante

En biología el telos (sobrevivir y procrear) no puede cambiar — es la constante que define lo que es el organismo. En BIOISO el telos declarado tiene la misma propiedad:

```loom
invariant TelosImmutability
  -- El telos solo puede cambiar por autoridad humana declarada
  -- Un being no puede modificar su propio telos statement
  -- La métrica de evaluación tampoco puede automodificarse
  -- Solo los medios para alcanzar el telos evolucionan

  enforced_by: safety_checker
  violation: compile_error

  exception:
    authority: named_human_authority
    process: class_3_governance
    audit: ADR required
end
```

---

## Parte IV — El Sistema Nervioso Distribuido con Telos

Cuando múltiples beings con el mismo telos operan en un ecosistema, emergen propiedades de sistema que ningún being individual tiene.

### Quorum y Telos Colectivo

```loom
ecosystem CoverageFleet
  beings: List<EcosystemAgent>
  shared_telos: "cobertura completa de zona declarada"

  quorum CoverageCoordination
    threshold: 0.6 of fleet online
    behavior_below: individual_optimization
    behavior_above: coordinated_optimization

    coordination_protocol:
      signal: coverage_gaps from each being
      aggregate: gap_map at ecosystem level
      assign: zones to beings by proximity and battery
      verify: session_type_verified deadlock_free
  end

  collective_telos_metric
    -- el telos del ecosistema es diferente al de cada being
    compute :: List<BeingState> -> Float
      individual_scores.aggregate weighted_by coverage_zone
    end
    -- un being con score alto pero zona duplicada contribuye menos
    -- que un being con score medio en zona única
  end

  emergent_experiment_generation
    -- el ExperimentGenerator opera a nivel ecosistema
    -- identifica gaps que ningún being individual puede ver
    gaps:
      - coverage_holes: zonas sin being asignado
      - coordination_failures: beings con zonas superpuestas
      - collective_convergence: el colectivo converge pero hay outliers
      - cascade_risks: falla de un being que destruye cobertura de zona
  end
end
```

---

## Parte V — Propiedades Formales del Sistema Completo

El sistema completo — intent vivo + BIOISO + telos central — garantiza:

```loom
system_properties LiveIntentBIOISO

  telos_primacy:
    every_function_traceable_to_telos: verified
    no_function_persists_without_telos_contribution: enforced
    telos_immutable_without_human_authority: compile_error

  signal_integrity:
    every_signal_source_tagged: required
    trust_level_declared: required
    hybrid_trust_is_minimum: enforced

  governance_completeness:
    every_change_classified: required
    every_approval_logged: required
    every_deployment_versioned: required
    rollback_always_possible: architectural_requirement

  experiment_validity:
    every_experiment_hypothesis_declared: required
    every_result_logged_as_learning: required
    failed_experiments_feed_generator: enforced

  safety:
    autopoietic_beings_require_all_four_annotations: compile_error
    telos_modification_requires_human_gate: compile_error
    sandbox_cannot_expand_without_human_approval: compile_error
    apoptosis_mechanism_mandatory: compile_error
end
```

---

## Parte VI — Sentidos, Efectores y Mensajería Nativa

### El Principio

Si el telos es la función organizadora central y las señales son su materia prima, el being necesita una capa sensorial y motora formal. Cada sentido declara su tipo, su trust level, y su schema. Cada emisor declara su efecto, sus garantías, y su scope. La mensajería entre beings y entre sistemas externos es un protocolo verificado en tiempo de compilación, no una convención de runtime.

---

### Sentidos — Interfaces de Entrada

```loom
interface_layer Senses
  -- Todo input al being pasa por un sentido declarado
  -- No existen inputs sin tipo, sin trust level, sin schema

  visual:
    source: camera_feed | video_stream | image_api | screenshot_capture
    processing: computer_vision_primitive
    output: VisualSignal
      frame: ImageFrame
      objects: List<DetectedObject> with confidence
      timestamp: Instant
      trust_level: declared_by_source
    telos_relevance: computed_per_being

  auditory:
    source: microphone_stream | audio_file | speech_api | system_audio
    processing: audio_primitive | speech_to_text | tone_analysis
    output: AuditorySignal
      transcript: Option<String> with confidence
      tone: SentimentVector
      timestamp: Instant
      trust_level: declared_by_source

  textual:
    source: file_watch | stdin | clipboard | document_api
    processing: nlp_primitive | structured_parse | raw
    output: TextualSignal
      content: String
      schema: Option<Schema> -- si es estructurado
      provenance: SourceReference
      trust_level: declared_by_source

  network_inbound:
    source: http_request | websocket_frame | grpc_call | mqtt_message | sse_event
    processing: protocol_native with schema_validation
    output: NetworkSignal
      payload: schema_validated
      protocol: NetworkProtocol
      origin: SourceAddress
      trust_level: origin_verified | unverified declared

  environmental:
    source: iot_sensor | weather_api | market_feed | gps | accelerometer
    processing: time_series_primitive | statistical_aggregate
    output: EnvironmentalSignal
      reading: typed_measurement with unit
      location: Option<Coordinate>
      timestamp: Instant
      trust_level: sensor_calibration_declared

  computational:
    source: metrics_stream | log_tail | trace_collector | profiler | health_endpoint
    processing: statistical_primitive | anomaly_detection | baseline_delta
    output: SystemSignal
      metric: Float with unit
      baseline: Float
      delta: Float
      anomaly: Option<AnomalyDescription>
      trust_level: high -- sistema propio

  filesystem:
    source: file_watcher | database_cursor | object_store_event | change_stream
    processing: change_detection | schema_validation | diff_computation
    output: DataSignal
      change_type: Created | Modified | Deleted | Renamed
      payload: schema_validated
      provenance: DataLineage
      trust_level: declared_by_source

  proprioceptive:
    -- sentido interno: el being monitoreando su propio estado
    source: internal_state_sampling
    processing: homeostatic_comparison
    output: ProprioceptiveSignal
      current_state: BeingState
      telos_score: Float
      drift_from_baseline: Float
      trend: Improving | Stable | Degrading
      trust_level: high -- estado propio
end
```

---

### Efectores — Interfaces de Salida

```loom
interface_layer Effectors
  -- Todo output del being pasa por un efector declarado
  -- Cada efector declara su efecto, sus garantías, y su scope

  network_emission:
    targets:
      http_client:
        method: GET | POST | PUT | PATCH | DELETE
        schema: validated_against_declared_output_type
        retry: @idempotent methods only unless @exactly-once declared
        timeout: declared_mandatory

      websocket_client:
        schema: frame_schema_validated
        ordering: declared (ordered | unordered)

      grpc_client:
        schema: proto_validated
        streaming: unary | server_stream | client_stream | bidirectional

      mqtt_publish:
        topic: typed_topic_schema
        qos: AtMostOnce | AtLeastOnce | ExactlyOnce declared
        retain: bool declared

      sse_emit:
        schema: event_schema_validated
        ordering: guaranteed_ordered

    guarantees: @exactly-once | @idempotent | @at_least_once declared_mandatory
    audit: every emission logged with being_id and telos_score_at_emission

  storage_emission:
    targets:
      relational_db:
        schema: type_validated
        transaction: @atomic declared
        isolation: declared (ReadCommitted | Serializable | etc)

      document_store:
        schema: json_schema_validated
        consistency: declared (eventual | strong)

      object_store:
        content_type: declared
        encryption: @encrypt-at-rest if @pii or @sensitive

      event_store:
        schema: event_schema_validated
        ordering: append_only guaranteed
        immutable: true

    guarantees: transactional | atomic | idempotent declared_mandatory
    audit: every write logged with provenance and telos_score

  computational_emission:
    targets:
      function_invoke:
        signature: type_validated
        effect: declared
        scope: sandboxed_to_declared_ecosystem

      agent_signal:
        -- señal a otro being en el mismo ecosistema
        channel: session_typed
        schema: validated
        protocol: declared_in_ecosystem_protocol

      process_spawn:
        binary: declared and sandboxed
        resources: bounded_by_declaration
        lifecycle: managed_by_being

  physical_emission:
    targets:
      actuator_api: typed_command with physical_bounds
      robot_interface: motion_primitive with safety_envelope
      hardware_control: low_level_typed_with_bounds

    human_gate: mandatory for irreversible_physical_effects
    safety_bounds: compiled_constraints not runtime_checks
    audit: every physical action logged with full state
end
```

---

### Mensajería Nativa entre Beings y Sistemas

La mensajería entre beings no es un detalle de implementación — es un protocolo verificado en tiempo de compilación usando session types.

```loom
-- Tipos de mensajería disponibles como primitivas nativas

messaging_primitive SyncRequest
  -- Request/response clásico, bloqueante
  pattern: request_response
  guarantees: @exactly-once
  timeout: mandatory
  error: typed Result<T, E>

  example:
    fn queryInventory :: ProductId -> SyncRequest<StockLevel>
    -- el tipo garantiza que hay exactamente una respuesta
    -- timeout declarado, no asumido
end

messaging_primitive AsyncMessage
  -- Fire and forget con garantías declaradas
  pattern: publish_subscribe | point_to_point
  guarantees: @at_least_once | @exactly-once | @at_most_once declared
  ordering: ordered | unordered declared
  schema: validated

  example:
    fn emitOrderEvent :: OrderEvent -> AsyncMessage<void>
      @exactly-once
      ordering: ordered
    end
end

messaging_primitive Stream
  -- Flujo continuo de mensajes
  pattern: producer_consumer
  backpressure: declared (drop | buffer | block)
  windowing: tumbling | sliding | session declared
  schema: per_message_validated

  example:
    fn streamMetrics :: () -> Stream<MetricSample>
      backpressure: buffer(capacity: 1000)
      windowing: tumbling(10.seconds)
    end
end

messaging_primitive EventBus
  -- Pub/sub con topics tipados
  pattern: publish_subscribe
  topics: typed_topic_schema mandatory
  filtering: compile_time_type_safe
  ordering: per_partition_ordered

  example:
    topic OrderEvents :: OrderCreated | OrderShipped | OrderDelivered | OrderCancelled
    -- solo estos tipos pueden publicarse en este topic
    -- suscriptores reciben el tipo exacto, no un string
end

messaging_primitive RPC
  -- Remote procedure call con session types
  pattern: bidirectional_protocol
  session_type: declared and verified deadlock_free
  schema: proto_or_loom_native
  streaming: unary | server | client | bidirectional

  example:
    protocol InventoryService
      roles: [Client, Server]
      sequence:
        Client -> Server: StockQuery
        Server -> Client: StockLevel
        Client -> Server: ReservationRequest
        Server -> Client: ReservationConfirmation | InsufficientStock
      property: deadlock_free
      proof: session_type_verified
    end
end

messaging_primitive MessageBroker

  -- Propiedades de emisión
  cast:
    broadcast:    -- todos los consumidores reciben todos los mensajes
    multicast:    -- subconjunto declarado de consumidores
    unicast:      -- exactamente un consumidor
    anycast:      -- cualquiera de los consumidores disponibles

  -- Propiedades de consumo
  consume:
    exactly_once: -- cada mensaje procesado una sola vez en el grupo
    at_least_once: -- puede reintentarse, consumer idempotente requerido
    at_most_once: -- puede perderse, aceptable declarado explícitamente

  -- Propiedades de lectura
  read:
    latest:       -- solo mensajes nuevos desde suscripción
    from_offset:  -- posición exacta en el log
    from_start:   -- replay completo
    n_last:       -- últimos N mensajes
    windowed:     -- ventana temporal declarada

  -- Propiedades de ordenamiento
  ordering:
    total:        -- orden global garantizado
    partial:      -- orden por partición o key
    none:         -- sin garantía de orden

  -- Propiedades de durabilidad
  durability:
    ephemeral:    -- en memoria, se pierde si el broker cae
    durable:      -- persiste en disco
    replicated:   -- replicado en N nodos declarados

  -- Propiedades de routing
  routing:
    direct:       -- por key exacta
    pattern:      -- por expresión declarada
    attribute:    -- por headers o metadatos
    content:      -- por contenido del mensaje
    round_robin:  -- distribución equitativa

  -- Perfiles conocidos como alias declarativos
  profile KafkaStyle =
    MessageBroker(cast: broadcast, consume: at_least_once,
      ordering: partial, durability: replicated, read: from_offset)

  profile RabbitMQStyle =
    MessageBroker(cast: anycast | multicast, consume: at_least_once,
      ordering: none, durability: durable, routing: pattern | direct)

  profile NATSStyle =
    MessageBroker(cast: broadcast | unicast, consume: at_most_once,
      ordering: none, durability: ephemeral, read: latest)

  profile PulsarStyle =
    MessageBroker(cast: broadcast, consume: exactly_once,
      ordering: partial, durability: replicated, read: from_offset)

  -- El compilador verifica coherencia de combinaciones
  constraints:
    broadcast + exactly_once: requires consensus_protocol declared
    anycast + total_ordering: incompatible -- error de compilación
    ephemeral + from_start: incompatible -- no hay log que releer
    at_most_once + content_routing: warning -- mensajes perdidos no se routean
    unicast + broadcast: incompatible -- mutuamente excluyentes
end

messaging_primitive SharedMemory
  -- Memoria compartida entre beings en el mismo proceso
  pattern: shared_state
  access: @thread_safe mandatory
  schema: typed
  scope: sandboxed_to_declared_ecosystem

  example:
    shared CoverageMap
      @thread_safe
      data: Map<Zone, Float> @atomic
      readers: concurrent_allowed
      writers: exclusive_lock
    end
end

---

### La Primitiva Universal — Entity

Todo grafo, toda máquina de estados, toda red, toda estructura de cómputo es una instancia de una sola primitiva con combinaciones de parámetros, anotaciones y meta-anotaciones. Los tipos predefinidos se vuelven alias. Las propiedades emergen de las anotaciones. El compilador verifica que la combinación es coherente.

```loom
-- La primitiva base
entity<N, E, Annotations>
  nodes: List<N>
  edges: List<(N, N, E)>
  -- todo lo demás emerge de las anotaciones declaradas
end

-- Vocabulario de anotaciones estructurales
@directed | @undirected
@acyclic | @cyclic_allowed
@weighted | @unweighted
@finite | @infinite
@layered | @flat
@hierarchical   -- relación padre/hijo implícita

-- Semánticas
@stochastic     -- aristas con distribuciones de probabilidad
@semantic       -- aristas con tipos ontológicos declarados
@temporal       -- aristas con timestamps o duración
@causal         -- aristas representan causalidad no solo correlación
@knowledge      -- grafo de conocimiento con triadas tipadas

-- De verificación
@deterministic  -- misma entrada produce siempre el mismo estado
@complete       -- todo estado tiene transición para todo input
@consistent     -- sin contradicciones en el cierre inferido

-- De comportamiento
@learnable      -- parámetros modificables por evolución o entrenamiento
@telos_guided   -- estructura se sesga hacia telos del being
@observable     -- estado interno inspectable en cualquier momento

-- Meta-anotaciones

meta_annotation @stochastic
  applies_to: entity edges
  requires: weight_type is Distribution<Float> or Float
  verified: weights_sum_to_1.0 per source_node
  implies: @weighted
  excludes: @deterministic

meta_annotation @causal
  applies_to: entity edges
  implies: @directed @temporal
  verified: no_backward_causation unless @retrocausal declared

meta_annotation @semantic
  applies_to: entity edges
  requires: relation_type is OntologyTerm
  verified: relation_types_valid_for_declared_ontology

meta_annotation @telos_guided
  applies_to: entity any
  requires: telos declared in containing being
  effect: structure evolution biased toward telos_metric
  verified: telos_metric computable from entity state

meta_annotation @acyclic
  applies_to: entity structure
  verified: at_compile_time not at_runtime
  error: compile_error if cycle_detected
  implies: topological_sort always_valid

meta_annotation @hierarchical
  applies_to: entity structure
  implies: @acyclic @directed
  verified: exactly_one_parent per non_root_node

-- Reglas de coherencia
annotation_constraints:
  incompatible:
    - [@deterministic, @stochastic]
    - [@acyclic, @cyclic_allowed]
    - [@directed, @undirected]
  implies:
    - @hierarchical implies @acyclic @directed
    - @stochastic implies @weighted
    - @causal implies @directed @temporal
    - @knowledge implies @semantic @typed_relations
  requires_parameter:
    - @stochastic requires weight: Distribution<Float> or Float
    - @temporal requires timestamp: Instant or Duration
    - @layered requires layer_function: N -> Int
    - @learnable requires learning_rate: Float and bounds declared

-- Todo lo conocido como instancia de entity

type MarkovChain<S> =
  entity<S, Float, @stochastic @directed @finite>
  -- pesos suman 1.0 por nodo: verificado en compilación
  -- telos_guided version: transiciones sesgadas hacia telos_metric

type DAG<N, E> =
  entity<N, E, @directed @acyclic>
  -- topological_sort always_valid como consecuencia estructural

type Tree<N, E> =
  entity<N, E, @directed @acyclic @hierarchical>

type FSM<S, I> =
  entity<S, I, @directed @finite @deterministic>
  -- lifecycle: en Loom es esta instancia

type NeuralNet<N> =
  entity<N, Float, @directed @weighted @layered @learnable>

type KnowledgeGraph<C, R> =
  entity<C, R, @semantic @typed_relations @knowledge>
  where R: OntologyTerm

type Ecosystem<B> =
  entity<B, SignalChannel, @directed @telos_guided @observable>

type CausalSignalGraph<N> =
  entity<N, Signal, @directed @causal @temporal @telos_guided>

type DependencyGraph<F> =
  entity<F, EffectEdge, @directed @acyclic>
  -- generado automáticamente por being en compilación

-- Grafos de conocimiento y triadas semánticas

entity<Concept, Relation, @semantic @knowledge>

  relation_types:
    is_a:        (Concept, Concept)  -- taxonomía
    has_part:    (Concept, Concept)  -- mereología
    causes:      (Concept, Concept)  -- causalidad declarada
    correlates:  (Concept, Concept)  -- correlación sin causalidad
    contradicts: (Concept, Concept)  -- inconsistencia
    depends_on:  (Concept, Concept)  -- dependencia funcional
    produces:    (Process, Concept)  -- output de proceso
    consumes:    (Process, Concept)  -- input de proceso
    equivalent:  (Concept, Concept)  -- identidad semántica

  triple: (Concept, RelationType, Concept)
    verified: relation_type_valid_for_domain
    consistent: no_contradictions_in_transitive_closure

  inference:
    transitive: [is_a, has_part, depends_on, causes]
    symmetric:  [correlates, equivalent, contradicts]
    inverse:    [produces <-> consumes]

  closed_world: false | true declared

-- Combinaciones nuevas sin nombre previo

type ProbabilisticCausalGraph<C> =
  entity<C, Distribution<Float>,
    @semantic @stochastic @causal @temporal>

type AdaptiveFSM<S, I> =
  entity<S, I, @directed @finite @learnable @telos_guided>

type SemanticEcosystem<B> =
  entity<B, OntologyTerm,
    @semantic @directed @telos_guided @observable>

type ProbabilisticOntology<C> =
  entity<C, Float,
    @semantic @hierarchical @stochastic @consistent>

type EcosystemSignalGraph =
  entity<Being, SignalChannel,
    @directed @causal @temporal @telos_guided @observable>
  properties:
    flow_conservation: signals_in == signals_processed | signals_buffered
    bottleneck_detection: computed from edge_capacity
    cascade_risk: computed from connectivity and load
    critical_path_to_collective_telos: computed

-- Máquina de Turing como modelo teórico
formal_model TuringMachine<Q, Σ>
  states: FiniteSet<Q>
  alphabet: FiniteSet<Σ>
  blank: Σ
  transition: (Q, Σ) -> (Q, Σ, Left | Right)
  initial: Q
  accepting: Set<Q>
  note: la cinta infinita es lo que la hace no representable
        como entity finita. Todo lo computable en tiempo finito
        es una entity finita. Rice's theorem sobre TM fundamenta
        el Correctness Ceiling del seccion X.2 del white paper.
end
```

---

### Protocolos de Ecosistema — Mensajería entre Beings

Cuando múltiples beings se comunican, el protocolo del ecosistema es un session type verificado que gobierna todas las interacciones.

```loom
ecosystem_protocol FleetCoordination
  roles: [Leader, Worker, Monitor]

  -- El protocolo completo entre los tres roles
  choreography:

    -- Fase de coordinación
    Leader -> Worker: ZoneAssignment
    Worker -> Leader: ZoneAccepted | ZoneRejected

    -- Fase de operación
    Worker -> Monitor: StatusUpdate  -- periódico
    Monitor -> Leader: FleetStatus   -- agregado

    -- Fase de excepción
    Worker -> Leader: CoverageGap
    Leader -> Worker: Rebalance | RequestReinforcement

    -- Fase de propagación
    Worker -> Leader: PropagationRequest  -- cuando telos_score > threshold
    Leader -> Worker: PropagationApproved | PropagationDenied

  properties:
    deadlock_free: session_type_verified
    liveness: every worker eventually receives assignment
    safety: no two workers assigned same exclusive zone

  governance: human_regulated for Rebalance affecting more than 20% of fleet
end
```

---

### La Integración con el Telos

Cada sentido, cada emisor, y cada mensaje declara su relevancia al telos del being:

```loom
telos_integration SensoryTelos
  -- El telos filtra qué sentidos merecen atención
  -- No todos los inputs son igualmente relevantes

  signal_attention_weights:
    -- Calculado por el being en función de su telos específico
    proprioceptive: 1.0    -- siempre máxima atención al estado propio
    telos_relevant: > 0.6  -- señales que contribuyen al telos
    contextual: 0.3 - 0.6  -- señales de contexto
    noise: < 0.3           -- atenuadas pero no ignoradas

  effector_telos_logging:
    -- Todo efecto en el mundo lleva el telos_score del momento
    -- Permite trazar qué acciones tomó el being y bajo qué estado de telos
    mandatory_fields:
      - telos_score_at_emission
      - telos_trend_at_emission
      - contributing_signals
      - expected_telos_impact

  messaging_telos_propagation:
    -- Los mensajes entre beings pueden llevar el telos_score del emisor
    -- Permite que el receptor calibre la confianza del mensaje
    optional_header:
      sender_telos_score: Float
      sender_telos_trend: Improving | Stable | Degrading
end
```

---

## Parte VII — Las Dos Dimensiones Ortogonales

Esta es la garantía arquitectónica central que Loom debe mantener. Todo lo demás en este documento vive dentro de estas dos dimensiones o en su composición.

---

### Dimensión 1 — Taxonomía Estructural

La primitiva `entity` con su sistema de anotaciones y meta-anotaciones. Bien formada significa:

```loom
dimension StructuralTaxonomy

  completeness:
    -- Toda combinación de anotaciones válida es aceptada
    -- Toda combinación inválida produce error de compilación
    -- No existen combinaciones silenciosamente aceptadas pero incoherentes
    verified: annotation_constraint_checker covers all_combinations

  soundness:
    -- Las propiedades que emergen de las anotaciones son correctas
    -- @acyclic implica topological_sort always_valid: proved
    -- @stochastic implica weights_sum_to_1.0: verified at compile_time
    -- @hierarchical implica exactly_one_parent: verified at compile_time
    -- @deterministic implica same_input_same_output: checked by SMT

  meta_annotation_consistency:
    -- Las reglas de las meta-anotaciones son completas y no contradictorias
    -- implies, excludes, requires_parameter forman un sistema consistente
    verified: no_circular_implications
    verified: no_contradictory_rules
    verified: all_requires_parameter_checkable_at_compile_time

  extensibility:
    -- Nuevas anotaciones pueden agregarse sin romper las existentes
    -- Nuevas meta-anotaciones declaran sus reglas explícitamente
    -- El verificador de coherencia es paramétrico en el conjunto de anotaciones
    property: open_for_extension_closed_for_modification

  coverage:
    -- Todo tipo de estructura computacional conocida es expresable
    -- Las combinaciones nuevas sin nombre previo son expresables
    -- La Máquina de Turing marca el límite superior: lo que no es
    --   representable como entity finita está fuera del sistema
    ceiling: TuringMachine as formal boundary
end
```

---

### Dimensión 2 — Restricciones Formales Semánticas

Las propiedades de corrección que se propagan a través de cualquier instancia de `entity`. Bien implementada significa:

```loom
dimension FormalConstraints

  contracts:
    -- require: y ensure: válidos en cualquier función sobre cualquier entity
    -- El SMT bridge verifica satisfacibilidad de los predicados
    -- Los contratos se propagan a través de composición de entities
    source: Hoare (1969), Meyer (1988)
    verified: SMT_bridge covers all_contract_forms

  information_flow:
    -- flow secret y flow public se propagan a través de nodos y aristas
    -- Un nodo @pii en un KnowledgeGraph contamina las aristas que lo tocan
    -- Un nodo secret en un MarkovChain no puede alcanzar output public
    source: Denning (1976), Myers/Liskov (1997)
    verified: information_flow_checker covers all_entity_types

  typestate:
    -- lifecycle: válido en cualquier entity con estados declarados
    -- Las transiciones inválidas no existen como operaciones
    -- El typestate se preserva a través de propagación en ecosistemas
    source: Strom/Yemini (1986)
    verified: typestate_checker covers all_entity_transitions

  algebraic_properties:
    -- @idempotent, @exactly-once, @commutative verificados en entity edges
    -- Las operaciones sobre entities heredan las propiedades algebraicas
    -- @stochastic + @exactly-once requiere consensus_protocol: coherencia
    --   entre dimensión estructural y semántica verificada
    source: Shapiro et al. (2011)
    verified: algebraic_checker covers all_entity_operations

  effect_system:
    -- Toda operación sobre una entity declara sus efectos
    -- Efectos se propagan transitivamente a través del grafo de dependencias
    -- El DependencyGraph de un being es un DAG de efectos verificado
    source: Moggi (1991), Plotkin/Power (2001)
    verified: effect_checker covers all_entity_operations

  session_types:
    -- Los ecosystem_protocols son session types verificados
    -- El EcosystemSignalGraph tiene protocolo de comunicación formal
    -- Deadlock freedom proved para todos los protocolos declarados
    source: Honda (1993), Honda/Yoshida (2008)
    verified: session_type_checker covers all_ecosystem_protocols

  SMT_bridge:
    -- Refinement types verificados por Z3 en cualquier entity
    -- Predicados sobre nodos y aristas son proposiciones SMT
    -- El cierre transitivo de KnowledgeGraph es verificable por SMT
    source: Z3, CVC5
    verified: SMT_bridge covers all_refinement_predicates
end
```

---

### La Garantía de Ortogonalidad

```loom
orthogonality_guarantee:

  independence:
    -- Las dos dimensiones se verifican por separado
    -- El structural_checker no depende del formal_checker
    -- El formal_checker no depende del structural_checker
    -- Cada uno puede fallar independientemente con mensaje preciso

  composition:
    -- La composición de las dos dimensiones es verificada como tercer paso
    -- @learnable no puede violar un contrato formal silenciosamente:
    --   requiere que los contratos sean invariantes bajo aprendizaje
    --   o que declaren explícitamente qué puede cambiar
    -- @stochastic no relaja @exactly-once:
    --   la coherencia entre anotaciones y propiedades algebraicas
    --   es verificada en el paso de composición
    -- flow secret en un @telos_guided entity:
    --   el telos no puede exponer datos secret como señal pública

  failure_modes:
    -- Error en Dimensión 1: mensaje sobre anotaciones incoherentes
    -- Error en Dimensión 2: mensaje sobre propiedad formal violada
    -- Error en composición: mensaje sobre interacción entre dimensiones
    -- Nunca: error silencioso en ninguna de las tres categorías

  ALX_implications:
    -- La spec para ALX se divide en dos secciones claramente separadas
    -- Section A: spec completa de la taxonomía de entity y anotaciones
    -- Section B: spec completa de las restricciones formales y su propagación
    -- S_realized por sección permite localizar gaps en la dimensión correcta
    -- Un gap en Section A no contamina el diagnóstico de Section B
    -- El verificador de ortogonalidad es una tercera sección de spec
end
```

---

### Qué Garantiza cada Dimensión

| Pregunta | Dimensión responsable |
|---|---|
| ¿Es esta combinación de anotaciones coherente? | Estructural |
| ¿Es este DAG realmente acíclico? | Estructural |
| ¿Suman 1.0 los pesos de esta MarkovChain? | Estructural |
| ¿Puede este nodo ser padre de sí mismo en este árbol? | Estructural |
| ¿Viola este contrato la postcondición declarada? | Semántica |
| ¿Puede este dato secret llegar a output public? | Semántica |
| ¿Es esta operación sobre la entity idempotente? | Semántica |
| ¿Están declarados los efectos de esta operación? | Semántica |
| ¿Puede @learnable violar este contrato? | Composición |
| ¿Puede @stochastic relajar @exactly-once? | Composición |
| ¿Expone el telos datos secret como señal? | Composición |

---

### Implicación para la Implementación

El compilador tiene tres pipelines distintos que corren en secuencia:

```
Source (.loom)
  → Parser → AST
    → [Pipeline 1] StructuralChecker
        Verifica: coherencia de anotaciones
        Verifica: propiedades estructurales emergentes
        Produce: StructurallyValidAST o compile_error con dimensión=1
      → [Pipeline 2] FormalConstraintChecker
          Verifica: contracts, information flow, typestate,
                    algebraic properties, effects, session types, SMT
          Produce: FormallyValidAST o compile_error con dimensión=2
        → [Pipeline 3] OrthogonalityChecker
            Verifica: composición entre dimensiones
            Produce: OrthogonallyValidAST o compile_error con dimensión=3
          → Emitters (Rust, TypeScript, WASM, OpenAPI, JSON Schema)
```

Cada error indica su dimensión. El diagnóstico es siempre localizable. La IA corrigiendo un gap sabe exactamente en qué pipeline buscar.

---

## Resumen de Constructs Nuevos para Implementar


| Construct | Parte | Prioridad |
|---|---|---|
| `telos_function` con `TelosMetric` | III | Alta |
| `signal_attention` como filtro guiado por telos | III | Alta |
| `telos.guides` como criterio de todas las decisiones | III | Alta |
| `TelosImmutability` invariant | III | Alta |
| `interface_layer Senses` con ocho tipos de sentido | VI | Alta |
| `interface_layer Effectors` con cuatro categorías | VI | Alta |
| `messaging_primitive SyncRequest` | VI | Alta |
| `messaging_primitive AsyncMessage` | VI | Alta |
| `messaging_primitive Stream` | VI | Alta |
| `messaging_primitive EventBus` con topics tipados | VI | Alta |
| `messaging_primitive RPC` con session types | VI | Alta |
| `messaging_primitive MessageBroker` (Kafka/RabbitMQ/Pulsar/NATS) | VI | Alta |
| `messaging_primitive SharedMemory` | VI | Media |
| `stochastic_process MarkovChain` con bias hacia telos | VI | Alta |
| `dimension StructuralTaxonomy` — spec completa para ALX Section A | VII | Alta |
| `dimension FormalConstraints` — spec completa para ALX Section B | VII | Alta |
| `orthogonality_guarantee` — tercer pipeline del compilador | VII | Alta |
| `entity<N,E,@acyclic @directed>` — DAG verificado | VI | Alta |
| `entity<N,E,@stochastic @directed>` — MarkovChain | VI | Alta |
| `entity<N,E,@semantic @knowledge>` — KnowledgeGraph con triadas | VI | Alta |
| `entity<N,E,Annotations>` — primitiva universal con meta-anotaciones | VI | Alta |
| `annotation_constraints` verificador de coherencia entre anotaciones | VI | Alta |
| `formal_model TuringMachine` — techo teórico del Correctness Ceiling | VI | Media |
| `AdaptiveFSM`, `SemanticEcosystem`, `ProbabilisticOntology` — combinaciones nuevas | VI | Media |
| `ecosystem_protocol` con choreography verificada | VI | Media |
| `telos_integration SensoryTelos` | VI | Alta |
| `collective_telos_metric` a nivel ecosistema | IV | Media |
| `emergent_experiment_generation` | IV | Media |
| `signal_taxonomy` con fuentes históricas/real_time/inferred/simulated/hybrid | II | Alta |
| `ExperimentGenerator` con niveles celular/tejido/órgano/organismo/ecosistema | II | Media |
| `telos_contribution` como anotación en cada función del being | III | Alta |
| `function_retention` basado en telos_contribution | III | Media |

---

*Este documento especifica el estado conceptual actual para implementación en Loom. El orden de implementación sugerido: telos_function y TelosMetric primero, luego interface_layer Senses y Effectors, luego los messaging_primitives comenzando por SyncRequest y AsyncMessage, luego telos.guides como criterio de decisión, luego ecosystem_protocol y collective_telos_metric.*
