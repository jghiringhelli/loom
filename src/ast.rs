//! Abstract Syntax Tree (AST) node types for the Loom language.
//!
//! All nodes carry a [`Span`] for accurate error reporting and source-map
//! generation.  The AST is a close, lossless representation of the source —
//! no desugaring is performed here.

use std::fmt;

// ── Source position ──────────────────────────────────────────────────────────

/// Byte-offset span into the source string (`start` inclusive, `end` exclusive).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Construct a new `Span` from byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// A synthetic span used for generated or placeholder nodes.
    pub fn synthetic() -> Self {
        Span { start: 0, end: 0 }
    }

    /// Merge two spans into one covering both.
    pub fn merge(a: &Span, b: &Span) -> Self {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

// ── Top-level structure ───────────────────────────────────────────────────────

/// A compiled Loom module — the top-level compilation unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name as written in source (e.g. `PricingEngine`).
    pub name: String,
    /// Optional human-readable description (`describe: "..."`).
    pub describe: Option<String>,
    /// Audit annotations (`@since`, `@decision`, `@deprecated`, `@author`).
    pub annotations: Vec<Annotation>,
    /// Optional spec name this module implements (`spec PricingSpec`).
    pub spec: Option<String>,
    /// Capabilities the module exposes to callers.
    pub provides: Option<Provides>,
    /// Capabilities the module requires from its environment (DI surface).
    pub requires: Option<Requires>,
    /// Structural invariants declared at module level (`invariant name :: cond`).
    pub invariants: Vec<Invariant>,
    /// Inline test definitions (`test name :: body_expr`).
    pub test_defs: Vec<TestDef>,
    /// Top-level definitions in declaration order.
    pub items: Vec<Item>,
    pub span: Span,
}

/// A module-level structural invariant.
///
/// Emitted as a `debug_assert!` inside `_check_invariants()`.
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    pub name: String,
    pub condition: Expr,
    pub span: Span,
}

/// An in-language unit test block.
///
/// Emitted as `#[test] fn name() { body }` inside `#[cfg(test)] mod tests`.
#[derive(Debug, Clone, PartialEq)]
pub struct TestDef {
    pub name: String,
    pub body: Expr,
    pub span: Span,
}

/// Top-level item in a module body.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Fn(FnDef),
    Type(TypeDef),
    Enum(EnumDef),
    RefinedType(RefinedType),
}

// ── Definitions ───────────────────────────────────────────────────────────────

/// Function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: String,
    /// Optional human-readable description (`describe: "..."`).
    pub describe: Option<String>,
    /// Audit annotations (`@since`, `@decision`, `@deprecated`, `@pure`, `@author`).
    pub annotations: Vec<Annotation>,
    /// User-declared type parameters (e.g. `<A, B>` in `fn map<A, B>`).
    pub type_params: Vec<String>,
    /// Full type signature (parameter types + return type).
    pub type_sig: FnTypeSignature,
    /// Consequence tiers for declared effects: `[(effect_name, tier)]`.
    /// Populated when `effect [IO@reversible, DB@irreversible]` syntax is used.
    pub effect_tiers: Vec<(String, ConsequenceTier)>,
    /// Pre-conditions (`require:` clauses).
    pub requires: Vec<Contract>,
    /// Post-conditions (`ensure:` clauses).
    pub ensures: Vec<Contract>,
    /// Effect / dependency injections (`with` clause names).
    pub with_deps: Vec<String>,
    /// Body expressions; the last one is the return value.
    pub body: Vec<Expr>,
    pub span: Span,
}

/// Consequence tier for an effectful operation — classifies side-effect severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsequenceTier {
    /// No side effects; referentially transparent.
    Pure,
    /// Side effects that can be undone (e.g. a DB transaction).
    Reversible,
    /// Side effects that cannot be undone (e.g. sending an email).
    Irreversible,
}

/// Audit annotation — key/value metadata embedded in the Loom source.
///
/// Examples: `@since("1.0")`, `@decision("use UUIDs for ids")`,
/// `@deprecated("use charge_v2")`, `@pure`.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub key: String,
    pub value: String,
}

/// Product type (record / struct).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
    /// Ordered list of `(field_name, field_type)` pairs.
    pub fields: Vec<(String, TypeExpr)>,
    pub span: Span,
}

/// Sum type (enum / union).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// A single variant of an enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    /// Payload type, if any (`| B of T`).
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

/// Refined (constrained) type — a base type plus a compile-time predicate.
///
/// At construction sites a `TryFrom` impl is generated that asserts the
/// predicate at runtime via `debug_assert!`.
#[derive(Debug, Clone, PartialEq)]
pub struct RefinedType {
    pub name: String,
    pub base_type: TypeExpr,
    /// The predicate expression that must hold for any value of this type.
    pub predicate: Expr,
    pub span: Span,
}

// ── Type expressions ──────────────────────────────────────────────────────────

/// Type expression — the right-hand side of a type annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Primitive or user-defined type name (e.g. `Int`, `OrderLine`).
    Base(String),
    /// Parameterised generic type (e.g. `List<T>`, `Map<K, V>`).
    Generic(String, Vec<TypeExpr>),
    /// Effectful type — `Effect<[E1, E2, ...], ReturnType>`.
    Effect(Vec<String>, Box<TypeExpr>),
    /// Optional value — `Option<T>`.
    Option(Box<TypeExpr>),
    /// Fallible value — `Result<T, E>`.
    Result(Box<TypeExpr>, Box<TypeExpr>),
    /// Unnamed tuple — `(A, B, C)`.
    Tuple(Vec<TypeExpr>),
    /// Inference variable introduced by the HM engine. Never produced by the
    /// parser; fully resolved before code generation.
    TypeVar(u32),
}

/// Full function type signature, possibly curried.
///
/// For a curried function `A -> B -> C`, `params = [A, B]` and
/// `return_type = C`.
#[derive(Debug, Clone, PartialEq)]
pub struct FnTypeSignature {
    /// Parameter types in left-to-right order.
    pub params: Vec<TypeExpr>,
    pub return_type: Box<TypeExpr>,
}

// ── Contracts ─────────────────────────────────────────────────────────────────

/// A `require` or `ensure` contract — a boolean expression that must hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    pub expr: Expr,
    pub span: Span,
}

// ── Expressions ───────────────────────────────────────────────────────────────

/// Expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Let binding: `let name = value`.
    Let {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    /// Match expression: `match subject | Pattern -> body end`.
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Function call: `func(arg1, arg2, ...)`.
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Pipe operator: `left |> right`.
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Literal value.
    Literal(Literal),
    /// Identifier reference.
    Ident(String),
    /// Field access: `object.field`.
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Binary operation: `left op right`.
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Raw Rust code block — `inline { rust_code }`.
    ///
    /// The content is emitted verbatim into the Rust output. The type checker,
    /// inference engine, and effect checker treat this node as opaque.
    InlineRust(String),
    /// Type coercion: `expr as Type` — explicit numeric widening or narrowing.
    As(Box<Expr>, TypeExpr),
    /// Lambda (anonymous function): `|param, param| body`.
    ///
    /// Each parameter is `(name, optional_type_annotation)`.
    Lambda {
        params: Vec<(String, Option<TypeExpr>)>,
        body: Box<Expr>,
        span: Span,
    },
    /// For-in loop: `for VAR in ITER { BODY }` — yields `()`.
    ForIn {
        var: String,
        iter: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    /// Tuple construction: `(expr, expr, ...)`.
    Tuple(Vec<Expr>, Span),
    /// Try / propagate operator: `expr?` — maps to Rust's `?`.
    Try(Box<Expr>, Span),
}

/// Match arm: a single branch of a `match` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    /// Optional guard condition (`if cond`).
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

// ── Patterns ──────────────────────────────────────────────────────────────────

/// Pattern used in match arms.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Enum variant pattern, optionally binding sub-patterns.
    Variant(String, Vec<Pattern>),
    /// Variable binding.
    Ident(String),
    /// Wildcard — matches anything, binds nothing.
    Wildcard,
    /// Literal pattern.
    Literal(Literal),
}

// ── Literals ─────────────────────────────────────────────────────────────────

/// Literal value node.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Unit,
}

// ── Binary operators ──────────────────────────────────────────────────────────

/// Binary operator kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

// ── Interface declarations ────────────────────────────────────────────────────

/// What a module exposes as its public interface.
#[derive(Debug, Clone, PartialEq)]
pub struct Provides {
    /// List of `(operation_name, type_signature)` pairs.
    pub ops: Vec<(String, FnTypeSignature)>,
}

/// What a module requires from its environment (dependency injection surface).
#[derive(Debug, Clone, PartialEq)]
pub struct Requires {
    /// List of `(capability_name, type)` pairs.
    pub deps: Vec<(String, TypeExpr)>,
}
