//! M136–M140: Messaging primitives refinement tests.
//!
//! Covers:
//!   M136 — Stream pattern AST + parser
//!   M137 — PointToPoint codegen (AMQP ack model)
//!   M138 — Delivery guarantee mutual exclusion checker
//!   M139 — Stream pattern codegen (Reactive Streams backpressure)
//!   M140 — RequestResponse timeout mandatory checker

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_module(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

fn compile_emit(src: &str) -> String {
    use loom::codegen::rust::RustEmitter;
    let tokens = Lexer::tokenize(src).expect("lex failed");
    let module = Parser::new(&tokens).parse_module().expect("parse failed");
    RustEmitter::new().emit(&module)
}

fn check_messaging(src: &str) -> Vec<String> {
    use loom::checker::{LoomChecker, MessagingChecker};
    let module = parse_module(src);
    MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect()
}

/// Build a minimal [`Module`] containing a single `MessagingPrimitiveDef`.
/// All other fields are set to their default/empty values.
fn module_with_mp(mp: MessagingPrimitiveDef) -> Module {
    let tokens = Lexer::tokenize("module _Empty end").expect("lex stub");
    let mut m = Parser::new(&tokens).parse_module().expect("parse stub");
    m.items.clear();
    m.items.push(Item::MessagingPrimitive(mp));
    m
}

// ── M136: Stream pattern parses ───────────────────────────────────────────────

#[test]
fn stream_pattern_parses_to_stream_variant() {
    let module = parse_module(
        r#"
module T
messaging_primitive DataFeed
  pattern: stream
end
end
"#,
    );
    let mp = module
        .items
        .iter()
        .find_map(|i| {
            if let Item::MessagingPrimitive(m) = i {
                Some(m)
            } else {
                None
            }
        })
        .expect("no messaging primitive");
    assert_eq!(mp.pattern, Some(MessagingPattern::Stream));
}

// ── M139: Stream codegen ──────────────────────────────────────────────────────

#[test]
fn stream_pattern_emits_source_trait() {
    let out = compile_emit(
        r#"
module T
messaging_primitive MarketFeed
  pattern: stream
end
end
"#,
    );
    assert!(
        out.contains("MarketFeedSource"),
        "should emit Source trait: {}",
        out
    );
    assert!(
        out.contains("fn request("),
        "should emit request (backpressure): {}",
        out
    );
    assert!(
        out.contains("fn poll_next("),
        "should emit poll_next: {}",
        out
    );
    assert!(out.contains("fn cancel("), "should emit cancel: {}", out);
}

#[test]
fn stream_pattern_emits_sink_trait() {
    let out = compile_emit(
        r#"
module T
messaging_primitive PriceStream
  pattern: stream
end
end
"#,
    );
    assert!(
        out.contains("PriceStreamSink"),
        "should emit Sink trait: {}",
        out
    );
    assert!(out.contains("fn on_next("), "should emit on_next: {}", out);
    assert!(
        out.contains("fn on_complete("),
        "should emit on_complete: {}",
        out
    );
    assert!(
        out.contains("fn on_error("),
        "should emit on_error: {}",
        out
    );
}

#[test]
fn stream_codegen_includes_reactive_streams_attribution() {
    let out = compile_emit(
        r#"
module T
messaging_primitive OrderBook
  pattern: stream
end
end
"#,
    );
    assert!(
        out.contains("Reactive Streams"),
        "should attribute Reactive Streams spec: {}",
        out
    );
}

// ── M137: PointToPoint codegen ────────────────────────────────────────────────

#[test]
fn point_to_point_emits_sender_trait() {
    let out = compile_emit(
        r#"
module T
messaging_primitive JobQueue
  pattern: point_to_point
end
end
"#,
    );
    assert!(
        out.contains("JobQueueSender"),
        "should emit Sender trait: {}",
        out
    );
    assert!(out.contains("fn send("), "should emit send: {}", out);
}

#[test]
fn point_to_point_emits_receiver_with_ack_nack() {
    let out = compile_emit(
        r#"
module T
messaging_primitive TaskQueue
  pattern: point_to_point
end
end
"#,
    );
    assert!(
        out.contains("TaskQueueReceiver"),
        "should emit Receiver trait: {}",
        out
    );
    assert!(out.contains("fn ack("), "should emit ack: {}", out);
    assert!(out.contains("fn nack("), "should emit nack: {}", out);
    assert!(out.contains("fn poll("), "should emit poll: {}", out);
}

#[test]
fn point_to_point_codegen_includes_amqp_attribution() {
    let out = compile_emit(
        r#"
module T
messaging_primitive WorkQueue
  pattern: point_to_point
end
end
"#,
    );
    assert!(out.contains("AMQP"), "should attribute AMQP model: {}", out);
}

// ── M138: Delivery guarantee mutual exclusion ─────────────────────────────────

#[test]
fn checker_rejects_exactly_once_with_at_most_once() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "OrderBus".to_string(),
        pattern: Some(MessagingPattern::PublishSubscribe),
        guarantees: vec!["exactly-once".to_string(), "at-most-once".to_string()],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "should reject exactly-once + at-most-once"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("exactly-once") && e.contains("at-most-once")),
        "error should mention both guarantees: {:?}",
        errors
    );
}

#[test]
fn checker_rejects_exactly_once_with_at_least_once() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "EventBus".to_string(),
        pattern: Some(MessagingPattern::PublishSubscribe),
        guarantees: vec!["exactly-once".to_string(), "at-least-once".to_string()],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "should reject exactly-once + at-least-once"
    );
}

#[test]
fn checker_accepts_at_least_once_without_conflict() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "EventBus".to_string(),
        pattern: Some(MessagingPattern::PublishSubscribe),
        guarantees: vec!["at-least-once".to_string(), "ordered".to_string()],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "at-least-once + ordered should be valid: {:?}",
        errors
    );
}

// ── M139: Stream + exactly-once is incoherent ─────────────────────────────────

#[test]
fn checker_rejects_stream_with_exactly_once() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "DataFeed".to_string(),
        pattern: Some(MessagingPattern::Stream),
        guarantees: vec!["exactly-once".to_string()],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "stream + exactly-once should be rejected"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("stream") && e.contains("exactly-once")),
        "error should mention both stream and exactly-once: {:?}",
        errors
    );
}

#[test]
fn checker_accepts_stream_with_at_least_once() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "DataStream".to_string(),
        pattern: Some(MessagingPattern::Stream),
        guarantees: vec!["at-least-once".to_string()],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "stream + at-least-once should be valid: {:?}",
        errors
    );
}

// ── M140: RequestResponse timeout enforcement ─────────────────────────────────

#[test]
fn checker_rejects_request_response_without_timeout_mandatory() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "UserApi".to_string(),
        pattern: Some(MessagingPattern::RequestResponse),
        guarantees: vec![],
        timeout_mandatory: false,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "request_response without timeout:mandatory should warn"
    );
    assert!(
        errors.iter().any(|e| e.contains("timeout")),
        "error should mention timeout: {:?}",
        errors
    );
}

#[test]
fn checker_accepts_request_response_with_timeout_mandatory() {
    use loom::checker::{LoomChecker, MessagingChecker};
    let mp = MessagingPrimitiveDef {
        name: "UserApi".to_string(),
        pattern: Some(MessagingPattern::RequestResponse),
        guarantees: vec![],
        timeout_mandatory: true,
        span: Span::synthetic(),
    };
    let module = module_with_mp(mp);
    let errors: Vec<String> = MessagingChecker::new()
        .check_module(&module)
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "request_response with timeout:mandatory should pass: {:?}",
        errors
    );
}

// ── Pre-existing patterns still work ─────────────────────────────────────────

#[test]
fn request_response_codegen_unchanged() {
    let out = compile_emit(
        r#"
module T
messaging_primitive OrderService
  pattern: request_response
  timeout: mandatory
end
end
"#,
    );
    assert!(
        out.contains("OrderServiceClient"),
        "should emit Client trait: {}",
        out
    );
    assert!(out.contains("fn call("), "should emit call method: {}", out);
}

#[test]
fn publish_subscribe_codegen_unchanged() {
    let out = compile_emit(
        r#"
module T
messaging_primitive EventBus
  pattern: publish_subscribe
end
end
"#,
    );
    assert!(
        out.contains("EventBusSubscriber"),
        "should emit Subscriber trait: {}",
        out
    );
    assert!(
        out.contains("EventBusBus"),
        "should emit Bus struct: {}",
        out
    );
}

#[test]
fn producer_consumer_codegen_unchanged() {
    let out = compile_emit(
        r#"
module T
messaging_primitive WorkQueue
  pattern: producer_consumer
end
end
"#,
    );
    assert!(
        out.contains("WorkQueueProducer"),
        "should emit Producer trait: {}",
        out
    );
    assert!(
        out.contains("WorkQueueConsumer"),
        "should emit Consumer trait: {}",
        out
    );
}
