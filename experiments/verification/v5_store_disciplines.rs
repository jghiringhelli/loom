#![allow(unused)]
use std::convert::TryFrom;
// == LOOM AUDIT: ECommerce ==
// Functions  : 0
// Stores     : 1 → typed persistence + CRUD + HATEOAS
// LOOM[v7:audit]: do not edit manually. Each LOOM[...] comment records a decision.

pub mod e_commerce {
    use super::*;
    use std::collections::HashMap;

    // LOOM[persist:binary]: M151 — binary snapshot persistence trait
    // Deps: serde = { version = "1", features = ["derive"] }, bincode = "1"
    pub trait BinaryPersist: serde::Serialize + for<'de> serde::Deserialize<'de> + Sized {
        /// Serialize this snapshot to a binary file using bincode.
        fn save_snapshot(&self, path: &std::path::Path) -> std::io::Result<()> {
            let bytes = bincode::serialize(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            std::fs::write(path, bytes)
        }
        /// Deserialize a snapshot from a binary file.
        fn load_snapshot(path: &std::path::Path) -> std::io::Result<Self> {
            let bytes = std::fs::read(path)?;
            bincode::deserialize(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    }

    // LOOM[persist:compressed]: M152 — gzip-compressed binary snapshot persistence
    // Deps: flate2 = "1" (add to serde + bincode from M151)
    // File extension convention: use `.snap.gz` for compressed snapshots.
    pub trait CompressedBinaryPersist: BinaryPersist {
        /// Serialize and gzip-compress this snapshot to disk.
        fn save_compressed(&self, path: &std::path::Path) -> std::io::Result<()> {
            use std::io::Write;
            let bytes = bincode::serialize(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            let file = std::fs::File::create(path)?;
            let mut encoder =
                flate2::write::GzEncoder::new(file, flate2::Compression::default());
            encoder.write_all(&bytes)?;
            encoder.finish().map(|_| ())
        }
        /// Decompress and deserialize a snapshot from a gzip file.
        fn load_compressed(path: &std::path::Path) -> std::io::Result<Self> {
            use std::io::Read;
            let file = std::fs::File::open(path)?;
            let mut decoder = flate2::read::GzDecoder::new(file);
            let mut bytes = Vec::new();
            decoder.read_to_end(&mut bytes)?;
            bincode::deserialize(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    }

    // LOOM[store:Relational]: Orders — V5 struct translation
    // Deps: serde = { version = "1", features = ["derive"] }, bincode = "1"
    // Ecosystem: sqlx (compile-time query verification) | diesel | sea-orm
    // LOOM[store:Relational]: tables → typed structs, PK/FK annotated

    // LOOM[port:StoreError]: Orders — typed error hierarchy (M126)
    #[derive(Debug, Clone, PartialEq)]
    pub enum OrdersStoreError {
        NotFound(String),
        Conflict(String),
        Connection(String),
        Constraint(String),
        Other(String),
    }
    impl std::fmt::Display for OrdersStoreError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NotFound(m) | Self::Conflict(m) | Self::Connection(m)
                | Self::Constraint(m) | Self::Other(m) => write!(f, "{}", m),
            }
        }
    }
    impl std::error::Error for OrdersStoreError {}

    // Table `Order` — primary key: id
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct Order {
        pub id: String, // LOOM[pk]
        pub customer_id: String, // LOOM[fk]
        pub total: f64,
        pub status: String,
        pub created_at: i64,
    }

    // LOOM[port:Repository]: Order — dependency inversion port (M126)
    // Domain declares this trait; adapters implement it. Callers depend only on dyn OrderRepository.
    pub trait OrderRepository: Send + Sync {
        /// Find by primary key `id`.
        fn find_by_id(&self, id: &str) -> Result<Option<Order>, OrdersStoreError>;
        /// List entities with limit/offset pagination.
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Order>, OrdersStoreError>;
        /// Persist a new or updated entity (upsert semantics).
        fn save(&self, entity: Order) -> Result<Order, OrdersStoreError>;
        /// Remove by primary key.
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError>;
        /// Check existence without loading the full entity.
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError>;
    }

    // LOOM[service:CRUD]: OrderService — orchestration layer above repository (M153)
    // Depends on dyn OrderRepository — swap adapter at composition root, never here.
    pub struct OrderService {
        repo: Box<dyn OrderRepository>,
    }
    impl OrderService {
        /// Construct a service with the given repository adapter.
        pub fn new(repo: Box<dyn OrderRepository>) -> Self { Self { repo } }

        /// Validate and create a new Order. Add domain rules here (uniqueness, constraints).
        pub fn create(&self, entity: Order) -> Result<Order, OrdersStoreError> {
            // LOOM[validation:pre_create]: add pre-conditions before persistence
            self.repo.save(entity)
        }

        /// Retrieve by primary key `id`. Returns NotFound if absent.
        pub fn get(&self, id: &str) -> Result<Order, OrdersStoreError> {
            self.repo
                .find_by_id(id)?
                .ok_or_else(|| OrdersStoreError::NotFound(format!("Order '{{}}' not found", id)))
        }

        /// List with limit/offset pagination.
        pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<Order>, OrdersStoreError> {
            self.repo.find_all(limit, offset)
        }

        /// Validate and persist an updated Order. Fails if entity does not exist.
        pub fn update(&self, entity: Order) -> Result<Order, OrdersStoreError> {
            let id = format!("{:?}", entity.id);
            if !self.repo.exists(&id)? {
                return Err(OrdersStoreError::NotFound(format!("Order '{{}}' not found", id)));
            }
            // LOOM[validation:pre_update]: add invariant guards before persistence
            self.repo.save(entity)
        }

        /// Delete by primary key. Idempotent — does not error if absent.
        pub fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.repo.delete(id)
        }

        /// Returns true if an entity with the given key exists.
        pub fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            self.repo.exists(id)
        }
    }



    // LOOM[adapter:InMemory]: InMemoryOrderRepository — testable fake (M126, Fowler 2002)
    // Implements OrderRepository port. Swap for Postgres/SQLite adapter at the composition root.
    pub struct InMemoryOrderRepository {
        store: std::sync::Mutex<std::collections::HashMap<String, Order>>,
    }
    impl Default for InMemoryOrderRepository {
        fn default() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl OrderRepository for InMemoryOrderRepository {
        fn find_by_id(&self, id: &str) -> Result<Option<Order>, OrdersStoreError> {
            Ok(self.store.lock().unwrap().get(id).cloned())
        }
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Order>, OrdersStoreError> {
            let guard = self.store.lock().unwrap();
            Ok(guard.values().skip(offset).take(limit).cloned().collect())
        }
        fn save(&self, entity: Order) -> Result<Order, OrdersStoreError> {
            let key = format!("{:?}", entity.id);
            self.store.lock().unwrap().insert(key, entity.clone());
            Ok(entity)
        }
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.store.lock().unwrap().remove(id); Ok(())
        }
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            Ok(self.store.lock().unwrap().contains_key(id))
        }
    }



    // LOOM[adapter:Postgres]: Order — sqlx PgPool (M127)
    // Uncomment + cargo add sqlx --features postgres,runtime-tokio-rustls,macros
    //
    // pub struct PostgresOrderRepository { pub pool: sqlx::PgPool }
    // impl PostgresOrderRepository {
    //     pub fn new(pool: sqlx::PgPool) -> Self { Self { pool } }
    // }
    // impl OrderRepository for PostgresOrderRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<Order>, OrdersStoreError> {
    //         // let row = sqlx::query_as!(/* ... */, "SELECT * FROM order WHERE id = $1", id)
    //         //     .fetch_optional(&self.pool).await?;
    //         // Ok(row.map(Into::into))
    //         todo!("Postgres OrderRepository::find_by_id")
    //     }
    //     fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Order>, OrdersStoreError> {
    //         // sqlx::query_as!(/* ... */, "SELECT * FROM order LIMIT $1 OFFSET $2", limit as i64, offset as i64)
    //         todo!("Postgres OrderRepository::find_all")
    //     }
    //     fn save(&self, entity: Order) -> Result<Order, OrdersStoreError> { todo!() }
    //     fn delete(&self, id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[adapter:SQLite]: Order — rusqlite (M129)
    // Uncomment + cargo add rusqlite --features bundled
    //
    // pub struct SqliteOrderRepository {
    //     conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    // }
    // impl SqliteOrderRepository {
    //     pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
    //         let conn = rusqlite::Connection::open(path)?;
    //         Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    //     }
    // }
    // impl OrderRepository for SqliteOrderRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<Order>, OrdersStoreError> {
    //         // conn.query_row("SELECT * FROM order WHERE id = ?1", [id], ...)
    //         todo!("SQLite OrderRepository::find_by_id")
    //     }
    //     fn find_all(&self, _limit: usize, _offset: usize) -> Result<Vec<Order>, OrdersStoreError> { todo!() }
    //     fn save(&self, _entity: Order) -> Result<Order, OrdersStoreError> { todo!() }
    //     fn delete(&self, _id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, _id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[implicit:Specification]: Order — composable predicates (Evans 2003)

    pub trait OrderSpecification {
        fn is_satisfied_by(&self, candidate: &Order) -> bool;
    }

    pub struct AndOrderSpec<A: OrderSpecification, B: OrderSpecification>(pub A, pub B);
    impl<A: OrderSpecification, B: OrderSpecification> OrderSpecification for AndOrderSpec<A,B> {
    fn is_satisfied_by(&self, c: &Order) -> bool { self.0.is_satisfied_by(c) && self.1.is_satisfied_by(c) }
    }

    pub struct NotOrderSpec<A: OrderSpecification>(pub A);
    impl<A: OrderSpecification> OrderSpecification for NotOrderSpec<A> {
    fn is_satisfied_by(&self, c: &Order) -> bool { !self.0.is_satisfied_by(c) }
    }

    // LOOM[implicit:Pagination]: Order — opaque cursor pagination

    #[derive(Debug, Clone)]
    pub struct OrderPage {
        pub items: Vec<Order>,
        pub next_cursor: Option<String>,
        pub total_count: Option<usize>,
    }

    // LOOM[implicit:OpenAPI]: Order — utoipa schema hint (OpenAPI 3.1)
    // Add `#[derive(utoipa::ToSchema)]` to Order to emit the OpenAPI schema.

    // Table `Customer` — primary key: id
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct Customer {
        pub id: String, // LOOM[pk]
        pub email: String,
        pub name: String,
        pub tier: String,
    }

    // LOOM[port:Repository]: Customer — dependency inversion port (M126)
    // Domain declares this trait; adapters implement it. Callers depend only on dyn CustomerRepository.
    pub trait CustomerRepository: Send + Sync {
        /// Find by primary key `id`.
        fn find_by_id(&self, id: &str) -> Result<Option<Customer>, OrdersStoreError>;
        /// List entities with limit/offset pagination.
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Customer>, OrdersStoreError>;
        /// Persist a new or updated entity (upsert semantics).
        fn save(&self, entity: Customer) -> Result<Customer, OrdersStoreError>;
        /// Remove by primary key.
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError>;
        /// Check existence without loading the full entity.
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError>;
    }

    // LOOM[service:CRUD]: CustomerService — orchestration layer above repository (M153)
    // Depends on dyn CustomerRepository — swap adapter at composition root, never here.
    pub struct CustomerService {
        repo: Box<dyn CustomerRepository>,
    }
    impl CustomerService {
        /// Construct a service with the given repository adapter.
        pub fn new(repo: Box<dyn CustomerRepository>) -> Self { Self { repo } }

        /// Validate and create a new Customer. Add domain rules here (uniqueness, constraints).
        pub fn create(&self, entity: Customer) -> Result<Customer, OrdersStoreError> {
            // LOOM[validation:pre_create]: add pre-conditions before persistence
            self.repo.save(entity)
        }

        /// Retrieve by primary key `id`. Returns NotFound if absent.
        pub fn get(&self, id: &str) -> Result<Customer, OrdersStoreError> {
            self.repo
                .find_by_id(id)?
                .ok_or_else(|| OrdersStoreError::NotFound(format!("Customer '{{}}' not found", id)))
        }

        /// List with limit/offset pagination.
        pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<Customer>, OrdersStoreError> {
            self.repo.find_all(limit, offset)
        }

        /// Validate and persist an updated Customer. Fails if entity does not exist.
        pub fn update(&self, entity: Customer) -> Result<Customer, OrdersStoreError> {
            let id = format!("{:?}", entity.id);
            if !self.repo.exists(&id)? {
                return Err(OrdersStoreError::NotFound(format!("Customer '{{}}' not found", id)));
            }
            // LOOM[validation:pre_update]: add invariant guards before persistence
            self.repo.save(entity)
        }

        /// Delete by primary key. Idempotent — does not error if absent.
        pub fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.repo.delete(id)
        }

        /// Returns true if an entity with the given key exists.
        pub fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            self.repo.exists(id)
        }
    }



    // LOOM[adapter:InMemory]: InMemoryCustomerRepository — testable fake (M126, Fowler 2002)
    // Implements CustomerRepository port. Swap for Postgres/SQLite adapter at the composition root.
    pub struct InMemoryCustomerRepository {
        store: std::sync::Mutex<std::collections::HashMap<String, Customer>>,
    }
    impl Default for InMemoryCustomerRepository {
        fn default() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl CustomerRepository for InMemoryCustomerRepository {
        fn find_by_id(&self, id: &str) -> Result<Option<Customer>, OrdersStoreError> {
            Ok(self.store.lock().unwrap().get(id).cloned())
        }
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Customer>, OrdersStoreError> {
            let guard = self.store.lock().unwrap();
            Ok(guard.values().skip(offset).take(limit).cloned().collect())
        }
        fn save(&self, entity: Customer) -> Result<Customer, OrdersStoreError> {
            let key = format!("{:?}", entity.id);
            self.store.lock().unwrap().insert(key, entity.clone());
            Ok(entity)
        }
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.store.lock().unwrap().remove(id); Ok(())
        }
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            Ok(self.store.lock().unwrap().contains_key(id))
        }
    }



    // LOOM[adapter:Postgres]: Customer — sqlx PgPool (M127)
    // Uncomment + cargo add sqlx --features postgres,runtime-tokio-rustls,macros
    //
    // pub struct PostgresCustomerRepository { pub pool: sqlx::PgPool }
    // impl PostgresCustomerRepository {
    //     pub fn new(pool: sqlx::PgPool) -> Self { Self { pool } }
    // }
    // impl CustomerRepository for PostgresCustomerRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<Customer>, OrdersStoreError> {
    //         // let row = sqlx::query_as!(/* ... */, "SELECT * FROM customer WHERE id = $1", id)
    //         //     .fetch_optional(&self.pool).await?;
    //         // Ok(row.map(Into::into))
    //         todo!("Postgres CustomerRepository::find_by_id")
    //     }
    //     fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Customer>, OrdersStoreError> {
    //         // sqlx::query_as!(/* ... */, "SELECT * FROM customer LIMIT $1 OFFSET $2", limit as i64, offset as i64)
    //         todo!("Postgres CustomerRepository::find_all")
    //     }
    //     fn save(&self, entity: Customer) -> Result<Customer, OrdersStoreError> { todo!() }
    //     fn delete(&self, id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[adapter:SQLite]: Customer — rusqlite (M129)
    // Uncomment + cargo add rusqlite --features bundled
    //
    // pub struct SqliteCustomerRepository {
    //     conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    // }
    // impl SqliteCustomerRepository {
    //     pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
    //         let conn = rusqlite::Connection::open(path)?;
    //         Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    //     }
    // }
    // impl CustomerRepository for SqliteCustomerRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<Customer>, OrdersStoreError> {
    //         // conn.query_row("SELECT * FROM customer WHERE id = ?1", [id], ...)
    //         todo!("SQLite CustomerRepository::find_by_id")
    //     }
    //     fn find_all(&self, _limit: usize, _offset: usize) -> Result<Vec<Customer>, OrdersStoreError> { todo!() }
    //     fn save(&self, _entity: Customer) -> Result<Customer, OrdersStoreError> { todo!() }
    //     fn delete(&self, _id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, _id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[implicit:Specification]: Customer — composable predicates (Evans 2003)

    pub trait CustomerSpecification {
        fn is_satisfied_by(&self, candidate: &Customer) -> bool;
    }

    pub struct AndCustomerSpec<A: CustomerSpecification, B: CustomerSpecification>(pub A, pub B);
    impl<A: CustomerSpecification, B: CustomerSpecification> CustomerSpecification for AndCustomerSpec<A,B> {
    fn is_satisfied_by(&self, c: &Customer) -> bool { self.0.is_satisfied_by(c) && self.1.is_satisfied_by(c) }
    }

    pub struct NotCustomerSpec<A: CustomerSpecification>(pub A);
    impl<A: CustomerSpecification> CustomerSpecification for NotCustomerSpec<A> {
    fn is_satisfied_by(&self, c: &Customer) -> bool { !self.0.is_satisfied_by(c) }
    }

    // LOOM[implicit:Pagination]: Customer — opaque cursor pagination

    #[derive(Debug, Clone)]
    pub struct CustomerPage {
        pub items: Vec<Customer>,
        pub next_cursor: Option<String>,
        pub total_count: Option<usize>,
    }

    // LOOM[implicit:OpenAPI]: Customer — utoipa schema hint (OpenAPI 3.1)
    // Add `#[derive(utoipa::ToSchema)]` to Customer to emit the OpenAPI schema.

    // Table `OrderItem` — primary key: id
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct OrderItem {
        pub id: String, // LOOM[pk]
        pub order_id: String, // LOOM[fk]
        pub sku: String, // LOOM[indexed]
        pub quantity: i64,
        pub price: f64,
    }

    // LOOM[port:Repository]: OrderItem — dependency inversion port (M126)
    // Domain declares this trait; adapters implement it. Callers depend only on dyn OrderItemRepository.
    pub trait OrderItemRepository: Send + Sync {
        /// Find by primary key `id`.
        fn find_by_id(&self, id: &str) -> Result<Option<OrderItem>, OrdersStoreError>;
        /// List entities with limit/offset pagination.
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<OrderItem>, OrdersStoreError>;
        /// Persist a new or updated entity (upsert semantics).
        fn save(&self, entity: OrderItem) -> Result<OrderItem, OrdersStoreError>;
        /// Remove by primary key.
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError>;
        /// Check existence without loading the full entity.
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError>;
    }

    // LOOM[service:CRUD]: OrderItemService — orchestration layer above repository (M153)
    // Depends on dyn OrderItemRepository — swap adapter at composition root, never here.
    pub struct OrderItemService {
        repo: Box<dyn OrderItemRepository>,
    }
    impl OrderItemService {
        /// Construct a service with the given repository adapter.
        pub fn new(repo: Box<dyn OrderItemRepository>) -> Self { Self { repo } }

        /// Validate and create a new OrderItem. Add domain rules here (uniqueness, constraints).
        pub fn create(&self, entity: OrderItem) -> Result<OrderItem, OrdersStoreError> {
            // LOOM[validation:pre_create]: add pre-conditions before persistence
            self.repo.save(entity)
        }

        /// Retrieve by primary key `id`. Returns NotFound if absent.
        pub fn get(&self, id: &str) -> Result<OrderItem, OrdersStoreError> {
            self.repo
                .find_by_id(id)?
                .ok_or_else(|| OrdersStoreError::NotFound(format!("OrderItem '{{}}' not found", id)))
        }

        /// List with limit/offset pagination.
        pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<OrderItem>, OrdersStoreError> {
            self.repo.find_all(limit, offset)
        }

        /// Validate and persist an updated OrderItem. Fails if entity does not exist.
        pub fn update(&self, entity: OrderItem) -> Result<OrderItem, OrdersStoreError> {
            let id = format!("{:?}", entity.id);
            if !self.repo.exists(&id)? {
                return Err(OrdersStoreError::NotFound(format!("OrderItem '{{}}' not found", id)));
            }
            // LOOM[validation:pre_update]: add invariant guards before persistence
            self.repo.save(entity)
        }

        /// Delete by primary key. Idempotent — does not error if absent.
        pub fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.repo.delete(id)
        }

        /// Returns true if an entity with the given key exists.
        pub fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            self.repo.exists(id)
        }
    }



    // LOOM[adapter:InMemory]: InMemoryOrderItemRepository — testable fake (M126, Fowler 2002)
    // Implements OrderItemRepository port. Swap for Postgres/SQLite adapter at the composition root.
    pub struct InMemoryOrderItemRepository {
        store: std::sync::Mutex<std::collections::HashMap<String, OrderItem>>,
    }
    impl Default for InMemoryOrderItemRepository {
        fn default() -> Self { Self { store: std::sync::Mutex::new(std::collections::HashMap::new()) } }
    }
    impl OrderItemRepository for InMemoryOrderItemRepository {
        fn find_by_id(&self, id: &str) -> Result<Option<OrderItem>, OrdersStoreError> {
            Ok(self.store.lock().unwrap().get(id).cloned())
        }
        fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<OrderItem>, OrdersStoreError> {
            let guard = self.store.lock().unwrap();
            Ok(guard.values().skip(offset).take(limit).cloned().collect())
        }
        fn save(&self, entity: OrderItem) -> Result<OrderItem, OrdersStoreError> {
            let key = format!("{:?}", entity.id);
            self.store.lock().unwrap().insert(key, entity.clone());
            Ok(entity)
        }
        fn delete(&self, id: &str) -> Result<(), OrdersStoreError> {
            self.store.lock().unwrap().remove(id); Ok(())
        }
        fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> {
            Ok(self.store.lock().unwrap().contains_key(id))
        }
    }



    // LOOM[adapter:Postgres]: OrderItem — sqlx PgPool (M127)
    // Uncomment + cargo add sqlx --features postgres,runtime-tokio-rustls,macros
    //
    // pub struct PostgresOrderItemRepository { pub pool: sqlx::PgPool }
    // impl PostgresOrderItemRepository {
    //     pub fn new(pool: sqlx::PgPool) -> Self { Self { pool } }
    // }
    // impl OrderItemRepository for PostgresOrderItemRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<OrderItem>, OrdersStoreError> {
    //         // let row = sqlx::query_as!(/* ... */, "SELECT * FROM order_item WHERE id = $1", id)
    //         //     .fetch_optional(&self.pool).await?;
    //         // Ok(row.map(Into::into))
    //         todo!("Postgres OrderItemRepository::find_by_id")
    //     }
    //     fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<OrderItem>, OrdersStoreError> {
    //         // sqlx::query_as!(/* ... */, "SELECT * FROM order_item LIMIT $1 OFFSET $2", limit as i64, offset as i64)
    //         todo!("Postgres OrderItemRepository::find_all")
    //     }
    //     fn save(&self, entity: OrderItem) -> Result<OrderItem, OrdersStoreError> { todo!() }
    //     fn delete(&self, id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[adapter:SQLite]: OrderItem — rusqlite (M129)
    // Uncomment + cargo add rusqlite --features bundled
    //
    // pub struct SqliteOrderItemRepository {
    //     conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    // }
    // impl SqliteOrderItemRepository {
    //     pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
    //         let conn = rusqlite::Connection::open(path)?;
    //         Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    //     }
    // }
    // impl OrderItemRepository for SqliteOrderItemRepository {
    //     fn find_by_id(&self, id: &str) -> Result<Option<OrderItem>, OrdersStoreError> {
    //         // conn.query_row("SELECT * FROM order_item WHERE id = ?1", [id], ...)
    //         todo!("SQLite OrderItemRepository::find_by_id")
    //     }
    //     fn find_all(&self, _limit: usize, _offset: usize) -> Result<Vec<OrderItem>, OrdersStoreError> { todo!() }
    //     fn save(&self, _entity: OrderItem) -> Result<OrderItem, OrdersStoreError> { todo!() }
    //     fn delete(&self, _id: &str) -> Result<(), OrdersStoreError> { todo!() }
    //     fn exists(&self, _id: &str) -> Result<bool, OrdersStoreError> { todo!() }
    // }

    // LOOM[implicit:Specification]: OrderItem — composable predicates (Evans 2003)

    pub trait OrderItemSpecification {
        fn is_satisfied_by(&self, candidate: &OrderItem) -> bool;
    }

    pub struct AndOrderItemSpec<A: OrderItemSpecification, B: OrderItemSpecification>(pub A, pub B);
    impl<A: OrderItemSpecification, B: OrderItemSpecification> OrderItemSpecification for AndOrderItemSpec<A,B> {
    fn is_satisfied_by(&self, c: &OrderItem) -> bool { self.0.is_satisfied_by(c) && self.1.is_satisfied_by(c) }
    }

    pub struct NotOrderItemSpec<A: OrderItemSpecification>(pub A);
    impl<A: OrderItemSpecification> OrderItemSpecification for NotOrderItemSpec<A> {
    fn is_satisfied_by(&self, c: &OrderItem) -> bool { !self.0.is_satisfied_by(c) }
    }

    // LOOM[implicit:Pagination]: OrderItem — opaque cursor pagination

    #[derive(Debug, Clone)]
    pub struct OrderItemPage {
        pub items: Vec<OrderItem>,
        pub next_cursor: Option<String>,
        pub total_count: Option<usize>,
    }

    // LOOM[implicit:OpenAPI]: OrderItem — utoipa schema hint (OpenAPI 3.1)
    // Add `#[derive(utoipa::ToSchema)]` to OrderItem to emit the OpenAPI schema.

    // LOOM[implicit:UnitOfWork]: Orders — atomic transaction scope (Fowler 2002)
    // Ecosystem: sqlx::Transaction | diesel::Connection::transaction
    pub struct OrdersUnitOfWork {
        pub order: InMemoryOrderRepository,
        pub customer: InMemoryCustomerRepository,
        pub order_item: InMemoryOrderItemRepository,
    }
    impl Default for OrdersUnitOfWork {
        fn default() -> Self { Self {
            order: InMemoryOrderRepository::default(),
            customer: InMemoryCustomerRepository::default(),
            order_item: InMemoryOrderItemRepository::default(),
        } }
    }
    impl OrdersUnitOfWork {
        pub fn begin() -> Self { Self::default() }
        pub fn commit(self) -> Result<(), String> {
            // wire to real transaction backend
            Ok(())
        }
        pub fn rollback(self) { drop(self); }
    }

    // LOOM[implicit:HATEOAS]: Orders — HAL resource links (Fielding 2000 REST)
    // Ecosystem: utoipa (OpenAPI derive), axum, actix-web
    #[derive(Debug, Clone)]
    pub struct ResourceLink {
        pub rel: String,
        pub href: String,
        pub method: Option<String>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct OrdersLinks {
        pub links: Vec<ResourceLink>,
    }
    impl OrdersLinks {
        pub fn add(&mut self, rel: &str, href: &str) {
            self.links.push(ResourceLink { rel: rel.to_string(), href: href.to_string(), method: None });
        }
        pub fn with_method(&mut self, rel: &str, href: &str, method: &str) {
            self.links.push(ResourceLink { rel: rel.to_string(), href: href.to_string(), method: Some(method.to_string()) });
        }
        pub fn self_link(mut self, href: &str) -> Self { self.add("self", href); self }
    }

    // LOOM[implicit:CQRS]: Orders — Command/Query split (Young 2010, Meyer CQS)

    pub trait OrdersCommand {
        type Error;
        fn execute(self) -> Result<(), Self::Error>;
    }

    pub trait OrdersQuery {
        type Output;
        type Error;
        fn execute(&self) -> Result<Self::Output, Self::Error>;
    }

    // LOOM[persist:snapshot]: M151 — OrdersSnapshot (binary persistence)
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct OrdersSnapshot {
        /// Unix timestamp (seconds) when this snapshot was created.
        pub created_at_secs: i64,
        pub order: Vec<Order>,
        pub customer: Vec<Customer>,
        pub order_item: Vec<OrderItem>,
    }
    impl BinaryPersist for OrdersSnapshot {}
    // LOOM[persist:compressed]: M152 — gzip-compressed snapshot (dep: flate2 = "1")
    impl CompressedBinaryPersist for OrdersSnapshot {}

}
