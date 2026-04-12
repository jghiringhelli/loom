/// M162 — `command` and `query` items (CQRS): parser + codegen tests.
///
/// `command Name field: Type ... end`  →  {Name}Command struct + {Name}Handler trait
/// `query Name field: Type ... end`   →  {Name}Query  struct + {Name}QueryHandler<R> trait

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

fn compile_check(src: &str) -> Result<String, Vec<loom::error::LoomError>> {
    loom::compile(src)
}

// ─── M162.1: command parses without error ─────────────────────────────────────

#[test]
fn m162_command_parses() {
    let src = r#"
module orders
command PlaceOrder
  order_id: Int
  amount: Float
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("PlaceOrderCommand"),
        "expected PlaceOrderCommand\n{out}"
    );
}

// ─── M162.2: command derive attrs ─────────────────────────────────────────────

#[test]
fn m162_command_derive_attrs() {
    let src = r#"
module orders
command PlaceOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("#[derive(Debug, Clone)]"),
        "missing derive attrs\n{out}"
    );
}

// ─── M162.3: command typed fields ─────────────────────────────────────────────

#[test]
fn m162_command_typed_fields() {
    let src = r#"
module orders
command PlaceOrder
  order_id: Int
  amount: Float
  express: Bool
end
end
"#;
    let out = compile(src);
    assert!(out.contains("pub order_id: i64"), "missing order_id\n{out}");
    assert!(out.contains("pub amount: f64"), "missing amount\n{out}");
    assert!(out.contains("pub express: bool"), "missing express\n{out}");
}

// ─── M162.4: command handler trait ────────────────────────────────────────────

#[test]
fn m162_command_handler_trait() {
    let src = r#"
module orders
command PlaceOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub trait PlaceOrderHandler"),
        "missing handler trait\n{out}"
    );
    assert!(
        out.contains("fn handle(&self, cmd: PlaceOrderCommand) -> Result<(), String>"),
        "missing handle signature\n{out}"
    );
}

// ─── M162.5: command audit comment ────────────────────────────────────────────

#[test]
fn m162_command_audit_comment() {
    let src = r#"
module orders
command PlaceOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[command:cqrs]"),
        "missing command audit comment\n{out}"
    );
    assert!(out.contains("M162"), "missing M162 reference\n{out}");
}

// ─── M162.6: query parses without error ───────────────────────────────────────

#[test]
fn m162_query_parses() {
    let src = r#"
module orders
query GetOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("GetOrderQuery"),
        "expected GetOrderQuery\n{out}"
    );
}

// ─── M162.7: query typed fields ───────────────────────────────────────────────

#[test]
fn m162_query_typed_fields() {
    let src = r#"
module catalog
query SearchProducts
  keyword: String
  max_price: Float
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub keyword: String"),
        "missing keyword\n{out}"
    );
    assert!(
        out.contains("pub max_price: f64"),
        "missing max_price\n{out}"
    );
}

// ─── M162.8: query handler trait is generic ───────────────────────────────────

#[test]
fn m162_query_handler_trait_generic() {
    let src = r#"
module orders
query GetOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("pub trait GetOrderQueryHandler<R>"),
        "query handler must be generic\n{out}"
    );
    assert!(
        out.contains("fn handle(&self, query: GetOrderQuery) -> R"),
        "missing generic handle signature\n{out}"
    );
}

// ─── M162.9: query audit comment ──────────────────────────────────────────────

#[test]
fn m162_query_audit_comment() {
    let src = r#"
module orders
query GetOrder
  order_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("LOOM[query:cqrs]"),
        "missing query audit comment\n{out}"
    );
}

// ─── M162.10: command and query together in one module ────────────────────────

#[test]
fn m162_command_and_query_together() {
    let src = r#"
module billing
command ChargeCard
  card_id: Int
  amount: Float
end
query GetBalance
  account_id: Int
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("ChargeCardCommand"),
        "missing command struct\n{out}"
    );
    assert!(
        out.contains("ChargeCardHandler"),
        "missing command handler\n{out}"
    );
    assert!(
        out.contains("GetBalanceQuery"),
        "missing query struct\n{out}"
    );
    assert!(
        out.contains("GetBalanceQueryHandler"),
        "missing query handler\n{out}"
    );
}

// ─── M162.11: command mixed with event ────────────────────────────────────────

#[test]
fn m162_command_mixed_with_event() {
    let src = r#"
module notifications
command SendNotification
  recipient: String
  message: String
end
event NotificationSent
  recipient: String
end
end
"#;
    let out = compile(src);
    assert!(
        out.contains("SendNotificationCommand"),
        "missing command\n{out}"
    );
    assert!(
        out.contains("NotificationSentEvent"),
        "missing event\n{out}"
    );
}

// ─── M162.12: empty command/query parses ──────────────────────────────────────

#[test]
fn m162_empty_command_and_query_parse() {
    let cmd = compile_check(
        r#"
module ops
command Noop
end
end
"#,
    );
    assert!(cmd.is_ok(), "empty command should parse: {:?}", cmd.err());

    let qry = compile_check(
        r#"
module ops
query Ping
end
end
"#,
    );
    assert!(qry.is_ok(), "empty query should parse: {:?}", qry.err());
}
