//! M153 — CRUD service layer + SQLite adapter wiring tests.
//!
//! Gate: every Relational store table emits:
//! 1. `{T}Service` struct holding `Box<dyn {T}Repository>`
//! 2. `create`, `get`, `list`, `update`, `delete`, `exists` methods
//! 3. `NotFound` error on `get`/`update` for absent entities
//! 4. SQLite adapter stub (was dead code — now wired)
//! 5. Service sits above the repository port — depends on the abstraction

fn compile(src: &str) -> String {
    loom::compile(src).expect("compile failed")
}

// ── Service struct emitted ────────────────────────────────────────────────────

#[test]
fn m153_service_struct_emitted_for_relational_table() {
    let out = compile(
        r#"
module M
  store Orders :: Relational
    table Order
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub struct OrderService"),
        "expected OrderService struct\n{out}"
    );
    assert!(
        out.contains("repo: Box<dyn OrderRepository>"),
        "expected Box<dyn OrderRepository> field\n{out}"
    );
}

#[test]
fn m153_service_has_new_constructor() {
    let out = compile(
        r#"
module M
  store Users :: Relational
    table User
      id: String @primary_key
      name: String
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub fn new(repo: Box<dyn UserRepository>")
            || out.contains("fn new(repo: Box<dyn UserRepository>"),
        "expected new(repo) constructor\n{out}"
    );
}

// ── CRUD methods ──────────────────────────────────────────────────────────────

#[test]
fn m153_service_has_create_method() {
    let out = compile(
        r#"
module M
  store Inventory :: Relational
    table Item
      id: String @primary_key
      quantity: Int
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub fn create"),
        "expected create method\n{out}"
    );
}

#[test]
fn m153_service_has_get_method() {
    let out = compile(
        r#"
module M
  store Inventory :: Relational
    table Item
      id: String @primary_key
      quantity: Int
    end
  end
end
"#,
    );
    assert!(out.contains("pub fn get"), "expected get method\n{out}");
}

#[test]
fn m153_service_has_list_method() {
    let out = compile(
        r#"
module M
  store Inventory :: Relational
    table Item
      id: String @primary_key
      quantity: Int
    end
  end
end
"#,
    );
    assert!(out.contains("pub fn list"), "expected list method\n{out}");
}

#[test]
fn m153_service_has_update_method() {
    let out = compile(
        r#"
module M
  store Catalog :: Relational
    table Product
      id: String @primary_key
      price: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub fn update"),
        "expected update method\n{out}"
    );
}

#[test]
fn m153_service_has_delete_method() {
    let out = compile(
        r#"
module M
  store Catalog :: Relational
    table Product
      id: String @primary_key
      price: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub fn delete"),
        "expected delete method\n{out}"
    );
}

#[test]
fn m153_service_has_exists_method() {
    let out = compile(
        r#"
module M
  store Catalog :: Relational
    table Product
      id: String @primary_key
      price: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub fn exists"),
        "expected exists method\n{out}"
    );
}

// ── Service semantics ─────────────────────────────────────────────────────────

#[test]
fn m153_get_returns_not_found_error() {
    let out = compile(
        r#"
module M
  store Accounts :: Relational
    table Account
      id: String @primary_key
      balance: Float
    end
  end
end
"#,
    );
    // get() must return NotFound when entity is absent
    assert!(
        out.contains("NotFound"),
        "expected NotFound error variant in service get\n{out}"
    );
}

#[test]
fn m153_update_checks_existence_before_persisting() {
    let out = compile(
        r#"
module M
  store Accounts :: Relational
    table Account
      id: String @primary_key
      balance: Float
    end
  end
end
"#,
    );
    // update() must call exists() before save()
    assert!(
        out.contains("self.repo.exists"),
        "expected exists() check in update method\n{out}"
    );
}

#[test]
fn m153_service_audit_comment_emitted() {
    let out = compile(
        r#"
module M
  store Ledger :: Relational
    table Entry
      id: String @primary_key
      amount: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("LOOM[service:CRUD]"),
        "expected LOOM[service:CRUD] audit comment\n{out}"
    );
    assert!(
        out.contains("M153"),
        "expected M153 reference in audit comment"
    );
}

// ── Multiple tables each get their own service ────────────────────────────────

#[test]
fn m153_each_table_gets_own_service() {
    let out = compile(
        r#"
module M
  store Shop :: Relational
    table Customer
      id: String @primary_key
      email: String
    end
    table Order
      id: String @primary_key
      total: Float
    end
  end
end
"#,
    );
    assert!(
        out.contains("pub struct CustomerService"),
        "expected CustomerService\n{out}"
    );
    assert!(
        out.contains("pub struct OrderService"),
        "expected OrderService\n{out}"
    );
}

// ── SQLite adapter now wired ──────────────────────────────────────────────────

#[test]
fn m153_sqlite_adapter_emitted_for_relational_store() {
    let out = compile(
        r#"
module M
  store Records :: Relational
    table Record
      id: String @primary_key
      data: String
    end
  end
end
"#,
    );
    assert!(
        out.contains("LOOM[adapter:SQLite]") || out.contains("rusqlite"),
        "expected SQLite adapter stub in Relational store output\n{out}"
    );
}

#[test]
fn m153_sqlite_adapter_references_rusqlite() {
    let out = compile(
        r#"
module M
  store Contacts :: Relational
    table Contact
      id: String @primary_key
      name: String
    end
  end
end
"#,
    );
    assert!(
        out.contains("rusqlite"),
        "expected rusqlite reference in SQLite adapter stub\n{out}"
    );
}

// ── Service not emitted for non-relational stores ─────────────────────────────

#[test]
fn m153_service_not_emitted_for_keyvalue_store() {
    let out = compile(
        r#"
module M
  store Cache :: KeyValue
    key: String
    value: String
  end
end
"#,
    );
    // KeyValue stores have KVStore trait, not a CRUD service
    assert!(
        !out.contains("pub struct CacheService"),
        "CacheService must not be emitted for KeyValue stores"
    );
}
