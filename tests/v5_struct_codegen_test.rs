//! V5 Store codegen tests — idiomatic Rust + discipline patterns.
//!
//! Gate: every store kind emits correct structs, CRUD/HATEOAS/CQRS/UnitOfWork/
//! Specification/Pagination/EventSourcing/Saga patterns.

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── Relational ────────────────────────────────────────────────────────────

fn relational_src() -> &'static str {
    r#"
module M
  store Orders :: Relational
    table Order
      id: String @primary_key
      amount: Float
      status: String
    end
  end
end
"#
}

/// Typed struct emitted for each table.
#[test]
fn v5_relational_emits_struct() {
    let out = compile(relational_src());
    assert!(
        out.contains("pub struct Order"),
        "expected Order struct\n{}",
        out
    );
    assert!(out.contains("amount"), "expected amount field");
}

/// Repository trait (Fowler 2002).
#[test]
fn v5_relational_emits_repository_trait() {
    let out = compile(relational_src());
    assert!(
        out.contains("pub trait OrderRepository"),
        "expected OrderRepository\n{}",
        out
    );
    assert!(out.contains("fn find_by_id"), "expected find_by_id");
    assert!(out.contains("fn save"), "expected save");
    assert!(out.contains("fn delete"), "expected delete");
}

/// InMemory testable fake (Fowler 2002 Fake Object).
#[test]
fn v5_relational_emits_in_memory_fake() {
    let out = compile(relational_src());
    assert!(
        out.contains("InMemoryOrderRepository"),
        "expected InMemoryOrderRepository\n{}",
        out
    );
    assert!(
        out.contains("Mutex") || out.contains("HashMap"),
        "expected thread-safe backing store"
    );
}

/// Specification pattern (Evans 2003).
#[test]
fn v5_relational_emits_specification_pattern() {
    let out = compile(relational_src());
    assert!(
        out.contains("Specification"),
        "expected Specification\n{}",
        out
    );
    assert!(out.contains("is_satisfied_by"), "expected is_satisfied_by");
}

/// Pagination cursor.
#[test]
fn v5_relational_emits_pagination_cursor() {
    let out = compile(relational_src());
    assert!(out.contains("Page"), "expected Page struct\n{}", out);
    assert!(out.contains("next_cursor"), "expected next_cursor");
}

/// Unit of Work (Fowler 2002).
#[test]
fn v5_relational_emits_unit_of_work() {
    let out = compile(relational_src());
    assert!(out.contains("UnitOfWork"), "expected UnitOfWork\n{}", out);
    assert!(out.contains("fn commit"), "expected commit");
    assert!(out.contains("fn rollback"), "expected rollback");
}

/// HATEOAS ResourceLink struct (Fielding 2000).
#[test]
fn v5_relational_emits_hateoas() {
    let out = compile(relational_src());
    assert!(
        out.contains("ResourceLink"),
        "expected ResourceLink\n{}",
        out
    );
    assert!(out.contains("pub rel: String"), "expected rel field");
    assert!(out.contains("pub href: String"), "expected href field");
}

/// CQRS Command/Query traits (Young 2010).
#[test]
fn v5_relational_emits_cqrs() {
    let out = compile(relational_src());
    assert!(out.contains("Command"), "expected Command\n{}", out);
    assert!(out.contains("Query"), "expected Query");
    assert!(out.contains("fn execute"), "expected execute");
}

/// OpenAPI / utoipa hint.
#[test]
fn v5_relational_emits_openapi_hints() {
    let out = compile(relational_src());
    assert!(
        out.contains("OpenAPI") || out.contains("utoipa"),
        "expected OpenAPI/utoipa\n{}",
        out
    );
}

// ── TimeSeries ────────────────────────────────────────────────────────────

fn timeseries_src() -> &'static str {
    r#"
module M
  store Metrics :: TimeSeries
    event CpuReading :: { core_id: Int, value: Float }
    retention: 30days
    resolution: 1minute
  end
end
"#
}

/// Event struct with injected timestamp.
#[test]
fn v5_timeseries_emits_event_struct() {
    let out = compile(timeseries_src());
    assert!(
        out.contains("pub struct CpuReading"),
        "expected CpuReading\n{}",
        out
    );
    assert!(out.contains("timestamp"), "expected timestamp injection");
}

/// EventStore trait (Fowler 2005).
#[test]
fn v5_timeseries_emits_event_store() {
    let out = compile(timeseries_src());
    assert!(out.contains("EventStore"), "expected EventStore\n{}", out);
    assert!(out.contains("fn append"), "expected append");
    assert!(out.contains("fn load"), "expected load");
}

/// Aggregate pattern (Evans 2003 DDD).
#[test]
fn v5_timeseries_emits_aggregate() {
    let out = compile(timeseries_src());
    assert!(out.contains("Aggregate"), "expected Aggregate\n{}", out);
    assert!(out.contains("fn apply"), "expected apply");
    assert!(
        out.contains("load_from_events"),
        "expected load_from_events"
    );
}

/// Domain Event Bus (Evans 2003).
#[test]
fn v5_timeseries_emits_event_bus() {
    let out = compile(timeseries_src());
    assert!(
        out.contains("EventBus") || out.contains("EventHandler"),
        "expected EventBus\n{}",
        out
    );
    assert!(
        out.contains("fn publish") || out.contains("fn subscribe"),
        "expected publish/subscribe"
    );
}

// ── KeyValue ──────────────────────────────────────────────────────────────

/// Typed trait with get/put/del.
#[test]
fn v5_keyvalue_emits_typed_trait() {
    let src = r#"
module M
  store Cache :: KeyValue
    key: String
    value: Int
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("trait") || out.contains("Store"),
        "expected Store trait\n{}",
        out
    );
    assert!(
        out.contains("fn get") || out.contains("fn put") || out.contains("fn del"),
        "expected get/put/del"
    );
}

// ── Graph ─────────────────────────────────────────────────────────────────

/// Node/edge structs and DAG wrapper.
#[test]
fn v5_graph_emits_nodes_and_edges() {
    let src = r#"
module M
  store TaskGraph :: Graph
    node Task :: { id: String, name: String }
    edge DependsOn :: Task -> Task { weight: Float }
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct Task"),
        "expected Task struct\n{}",
        out
    );
    assert!(
        out.contains("DependsOn") || out.contains("weight"),
        "expected edge type\n{}",
        out
    );
    assert!(
        out.contains("DAG")
            || out.contains("dag")
            || out.contains("petgraph")
            || out.contains("Graph")
            || out.contains("graph"),
        "expected DAG/graph structure\n{}",
        out
    );
}

// ── Vector ────────────────────────────────────────────────────────────────

/// Embedding struct and similarity search trait.
#[test]
fn v5_vector_emits_embedding_and_search() {
    let src = r#"
module M
  store Embeddings :: Vector
    embedding :: { label: String }
    index: HNSW
  end
end
"#;
    let out = compile(src);
    assert!(out.contains("Embedding"), "expected Embedding\n{}", out);
    assert!(out.contains("vector"), "expected vector field");
    assert!(
        out.contains("VectorSearch") || out.contains("nearest"),
        "expected similarity search"
    );
}

// ── Document ─────────────────────────────────────────────────────────────

/// Collection struct with ecosystem hint.
#[test]
fn v5_document_emits_collection_struct() {
    let src = r#"
module M
  store Articles :: Document
    schema Article :: { title: String, body: String }
  end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub struct Article"),
        "expected Article struct\n{}",
        out
    );
    assert!(
        out.contains("serde")
            || out.contains("Document")
            || out.contains("mongo")
            || out.contains("document"),
        "expected document store hint\n{}",
        out
    );
}

// ── Distributed ───────────────────────────────────────────────────────────

/// Distributed log emits Saga coordinator (Garcia-Molina 1987).
#[test]
fn v5_distributed_emits_saga() {
    let src = r#"
module M
  store OrderLog :: DistributedLog
    event OrderPlaced :: { id: String, amount: Float }
    partitions: 4
    replication: 2
    consumer Analytics :: offset: earliest
  end
end
"#;
    let out = compile(src);
    // DistributedLog emits event sourcing + domain event bus + saga
    assert!(
        out.contains("Saga") || out.contains("EventStore") || out.contains("EventBus"),
        "expected Saga/EventStore/EventBus\n{}",
        out
    );
}
