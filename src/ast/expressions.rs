//! Expression, pattern, and literal AST nodes.
use super::*;
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
    /// Subscript / index access: `expr[index]`.
    Index(Box<Expr>, Box<Expr>, Span),
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
