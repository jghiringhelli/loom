// M110: usecase: Triple Derivation Block tests.
// Validates that Jacobson-style use cases simultaneously generate:
//   1. Implementation contracts (require:/ensure: comments)
//   2. Test stubs (#[test] fns for each acceptance criterion)
//   3. Documentation (OpenAPI description + user-facing doc comment)

use loom::ast::*;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse(src: &str) -> Result<Module, loom::error::LoomError> {
    let tokens = Lexer::tokenize(src).map_err(|es| es.into_iter().next().unwrap())?;
    Parser::new(&tokens).parse_module()
}

// ─── Test 1: full usecase: block with all fields parses ────────────────────────
#[test]
fn test_m110_usecase_parses() {
    let src = r#"
module UserService
  usecase RegisterUser:
    actor: ExternalUser
    precondition: not_user_exists
    trigger: POST
    postcondition: user_count_increased
    acceptance:
      test can_register_valid_user: email is valid password meets policy
      test rejects_duplicate_email: email already in store
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "full usecase: block should parse: {:?}", result.err());
    let module = result.unwrap();
    let uc = module.items.iter().find_map(|i| {
        if let Item::UseCase(u) = i { Some(u) } else { None }
    });
    assert!(uc.is_some(), "should have a UseCase item");
    let uc = uc.unwrap();
    assert_eq!(uc.name, "RegisterUser");
    assert_eq!(uc.actor, "ExternalUser");
    assert!(!uc.precondition.is_empty(), "precondition should be set");
    assert!(!uc.trigger.is_empty(), "trigger should be set");
    assert!(!uc.postcondition.is_empty(), "postcondition should be set");
}

// ─── Test 2: multiple test: lines in acceptance: parse ─────────────────────────
#[test]
fn test_m110_acceptance_criteria_parse() {
    let src = r#"
module Auth
  usecase LoginUser:
    actor: RegisteredUser
    precondition: user_exists
    trigger: POST
    postcondition: session_created
    acceptance:
      test valid_credentials_succeed: correct username and password
      test invalid_password_rejected: wrong password returns 401
      test locked_account_rejected: locked account returns 403
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "acceptance criteria should parse: {:?}", result.err());
    let module = result.unwrap();
    let uc = module.items.iter().find_map(|i| {
        if let Item::UseCase(u) = i { Some(u) } else { None }
    });
    assert!(uc.is_some());
    let uc = uc.unwrap();
    assert_eq!(uc.acceptance.len(), 3);
    assert_eq!(uc.acceptance[0].name, "valid_credentials_succeed");
    assert_eq!(uc.acceptance[1].name, "invalid_password_rejected");
    assert_eq!(uc.acceptance[2].name, "locked_account_rejected");
}

// ─── Test 3: empty acceptance: block → warning ─────────────────────────────────
#[test]
fn test_m110_empty_acceptance_is_warning() {
    let src = r#"
module Orders
  usecase PlaceOrder:
    actor: Customer
    precondition: cart_not_empty
    trigger: POST
    postcondition: order_persisted
    acceptance:
    end
  end
end
"#;
    // loom::compile filters [warn] prefixed errors so compilation should succeed.
    let result = loom::compile(src);
    assert!(result.is_ok(), "empty acceptance: should not block compilation: {:?}", result.err());

    // But the raw checker should emit the warning.
    let tokens = Lexer::tokenize(src).unwrap();
    let module = Parser::new(&tokens).parse_module().unwrap();
    let checker = loom::checker::UseCaseChecker::new();
    let diagnostics = checker.check(&module);
    assert!(
        diagnostics.iter().any(|e| format!("{}", e).contains("[warn]")),
        "should emit a [warn] for empty acceptance: but got {:?}", diagnostics
    );
}

// ─── Test 4: duplicate criterion name → error ──────────────────────────────────
#[test]
fn test_m110_duplicate_criterion_name_is_error() {
    let src = r#"
module Inventory
  usecase AddItem:
    actor: Warehouse
    precondition: item_not_exists
    trigger: POST
    postcondition: item_count_increased
    acceptance:
      test can_add_item: valid item data
      test can_add_item: duplicate name same criterion
    end
  end
end
"#;
    let tokens = Lexer::tokenize(src).unwrap();
    let module = Parser::new(&tokens).parse_module().unwrap();
    let checker = loom::checker::UseCaseChecker::new();
    let errors = checker.check(&module);
    assert!(
        errors.iter().any(|e| {
            let msg = format!("{}", e);
            msg.contains("duplicate acceptance criterion") && !msg.contains("[warn]")
        }),
        "duplicate criterion names should be a hard error: {:?}", errors
    );
}

// ─── Test 5: postcondition same as precondition → warning ──────────────────────
#[test]
fn test_m110_postcondition_equals_precondition_is_warning() {
    let src = r#"
module Nothing
  usecase NoOp:
    actor: System
    precondition: ready
    trigger: GET
    postcondition: ready
    acceptance:
      test always_ready: system is ready
    end
  end
end
"#;
    let tokens = Lexer::tokenize(src).unwrap();
    let module = Parser::new(&tokens).parse_module().unwrap();
    let checker = loom::checker::UseCaseChecker::new();
    let diagnostics = checker.check(&module);
    assert!(
        diagnostics.iter().any(|e| {
            let msg = format!("{}", e);
            msg.contains("[warn]") && msg.contains("postcondition")
        }),
        "identical pre/postcondition should warn: {:?}", diagnostics
    );
}

// ─── Test 6: compile_rust emits #[test] fns for each acceptance criterion ──────
#[test]
fn test_m110_emits_test_stubs() {
    let src = r#"
module Payments
  usecase ProcessPayment:
    actor: Cardholder
    precondition: card_valid
    trigger: POST
    postcondition: payment_recorded
    acceptance:
      test valid_card_accepted: card passes Luhn check
      test expired_card_rejected: expired card returns error
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "should compile: {:?}", result.err());
    let rust_src = result.unwrap();
    assert!(
        rust_src.contains("#[test]"),
        "emitted Rust should contain #[test] stubs"
    );
    assert!(
        rust_src.contains("valid_card_accepted") || rust_src.contains("uc_process_payment_valid_card_accepted"),
        "emitted Rust should contain test fn for 'valid_card_accepted'"
    );
    assert!(
        rust_src.contains("expired_card_rejected") || rust_src.contains("uc_process_payment_expired_card_rejected"),
        "emitted Rust should contain test fn for 'expired_card_rejected'"
    );
}

// ─── Test 7: compile_rust emits require:/ensure: contract comments ──────────────
#[test]
fn test_m110_emits_contract_comments() {
    let src = r#"
module Catalog
  usecase SearchProducts:
    actor: Visitor
    precondition: query_not_empty
    trigger: GET
    postcondition: results_returned
    acceptance:
      test returns_matching_products: query matches product name
    end
  end
end
"#;
    let result = loom::compile(src);
    assert!(result.is_ok(), "should compile: {:?}", result.err());
    let rust_src = result.unwrap();
    assert!(
        rust_src.contains("require:") || rust_src.contains("ensure:"),
        "emitted Rust should contain contract comments (require:/ensure:)"
    );
    assert!(
        rust_src.contains("SearchProducts"),
        "emitted Rust should reference the use-case name"
    );
}

// ─── Test 8: two usecase: blocks in same module parse ──────────────────────────
#[test]
fn test_m110_multiple_usecases_parse() {
    let src = r#"
module UserAccount
  usecase RegisterUser:
    actor: NewVisitor
    precondition: email_not_taken
    trigger: POST
    postcondition: account_created
    acceptance:
      test registers_new_user: valid email and password
    end
  end
  usecase DeleteUser:
    actor: AdminUser
    precondition: user_exists
    trigger: DELETE
    postcondition: user_removed
    acceptance:
      test deletes_existing_user: target user exists
    end
  end
end
"#;
    let result = parse(src);
    assert!(result.is_ok(), "two usecase: blocks should parse: {:?}", result.err());
    let module = result.unwrap();
    let use_cases: Vec<_> = module.items.iter().filter_map(|i| {
        if let Item::UseCase(u) = i { Some(u) } else { None }
    }).collect();
    assert_eq!(use_cases.len(), 2, "should have two UseCase items");
    assert_eq!(use_cases[0].name, "RegisterUser");
    assert_eq!(use_cases[1].name, "DeleteUser");
}
