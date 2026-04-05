// ALX: derived from loom.loom §"Token types" and §"Pipeline: Lexer"
// Uses logos 0.15. Keywords MUST appear before Ident in enum declaration order
// so that logos gives them priority.

use logos::Logos;
use crate::error::{LoomError, Span};

// Re-export the Token struct from ast so callers use one type.
// ALX: spec defines Token = { kind, text, span } — we replicate that here.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

/// All token kinds. Keywords come first (before Ident) so logos gives them
/// priority — otherwise identifiers shadow keywords.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")] // skip whitespace
#[logos(skip r"--[^\n]*")]   // skip line comments
pub enum TokenKind {
    // ── Keywords ──────────────────────────────────────────────────────────────
    // ALX: from TokenKind enum in loom.loom, must precede Ident.

    #[token("module")]
    Module,
    #[token("describe")]
    Describe,
    #[token("fn")]
    Fn,
    #[token("type")]
    Type,
    #[token("enum")]
    Enum,
    #[token("end")]
    End,
    #[token("let")]
    Let,
    #[token("match")]
    Match,
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("else")]
    Else,
    #[token("interface")]
    Interface,
    #[token("implements")]
    Implements,
    #[token("import")]
    Import,
    #[token("provides")]
    Provides,
    #[token("requires")]
    Requires,
    #[token("invariant")]
    Invariant,
    #[token("test")]
    Test,
    #[token("spec")]
    Spec,
    #[token("effect")]
    Effect,
    #[token("require")]
    Require,
    #[token("ensure")]
    Ensure,
    #[token("with")]
    With,
    #[token("of")]
    Of,
    #[token("flow")]
    Flow,
    #[token("lifecycle")]
    Lifecycle,
    #[token("being")]
    Being,
    #[token("telos")]
    Telos,
    #[token("form")]
    Form,
    #[token("matter")]
    Matter,
    #[token("regulate")]
    Regulate,
    #[token("evolve")]
    Evolve,
    #[token("toward")]
    Toward,
    #[token("search")]
    Search,
    #[token("fitness")]
    Fitness,
    #[token("bounds")]
    Bounds,
    #[token("ecosystem")]
    Ecosystem,
    #[token("members")]
    Members,
    #[token("signal")]
    Signal,
    #[token("from")]
    From,
    #[token("to")]
    To,
    #[token("payload")]
    Payload,
    #[token("epigenetic")]
    Epigenetic,
    #[token("modifies")]
    Modifies,
    #[token("reverts_when")]
    RevertsWhen,
    #[token("morphogen")]
    Morphogen,
    #[token("threshold")]
    Threshold,
    #[token("produces")]
    Produces,
    #[token("telomere")]
    Telomere,
    #[token("on_exhaustion")]
    OnExhaustion,
    #[token("limit")]
    Limit,
    #[token("crispr")]
    Crispr,
    #[token("replace")]
    Replace,
    #[token("guide")]
    Guide,
    #[token("quorum")]
    Quorum,
    #[token("action")]
    Action,
    #[token("plasticity")]
    Plasticity,
    #[token("trigger")]
    Trigger,
    #[token("rule")]
    Rule,
    #[token("Hebbian")]
    Hebbian,
    #[token("Boltzmann")]
    Boltzmann,
    #[token("autopoietic")]
    Autopoietic,
    #[token("modifiable_by")]
    ModifiableBy,
    #[token("bounded_by")]
    BoundedBy,
    #[token("where")]
    Where,
    #[token("as")]
    As,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("not")]
    Not,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("target")]
    Target,
    #[token("response")]
    Response,
    #[token("constraint")]
    Constraint,
    #[token("when")]
    When,
    #[token("preserve")]
    Preserve,
    #[token("gradient_descent")]
    GradientDescent,
    #[token("stochastic_gradient")]
    StochasticGradient,
    #[token("simulated_annealing")]
    SimulatedAnnealing,
    #[token("derivative_free")]
    DerivativeFree,
    #[token("mcmc")]
    Mcmc,

    // ── Operators and punctuation ─────────────────────────────────────────────

    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("::")]
    DoubleColon,
    #[token("|>")]
    PipeArrow,
    #[token("|")]
    Pipe,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token("@")]
    At,
    // ALX: Underscore given priority 3 to win over Ident regex (which has default priority 2).
    #[token("_", priority = 3)]
    Underscore,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("==")]
    Eq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("&&")]
    And2,
    #[token("||")]
    Or2,
    #[token("!")]
    Bang,
    #[token("?")]
    Question,

    // ── Literals ──────────────────────────────────────────────────────────────

    #[regex(r"-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),

    #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        // strip surrounding quotes
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    // BoolLit is handled by true/false tokens above — included here for completeness
    // ALX: spec has BoolLit variant; the true/false keyword tokens cover parsing.

    // ALX: Ident regex excludes standalone _ to avoid conflict with Underscore token.
    // Standalone _ lexes as Underscore (token priority wins).
    #[regex(r"[a-zA-Z][a-zA-Z0-9_\-]*|[_][a-zA-Z0-9_\-]+", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Skipped tokens — not emitted ─────────────────────────────────────────
    // Whitespace and comments are skipped by the #[logos(skip)] directives above.
}

// ── Lexer entry point ─────────────────────────────────────────────────────────

/// Public Lexer struct — wraps the `lex` pipeline function.
/// G2: Tests call `Lexer::tokenize(src)` rather than the bare `lex()` function.
pub struct Lexer;

impl Lexer {
    /// Tokenise `source`. Returns `Ok(Vec<Token>)` or `Err(Vec<LoomError>)`.
    pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LoomError>> {
        lex(source)
    }
}

/// Tokenise `source` into a `Vec<Token>`. Returns errors for any unrecognised
/// characters. Whitespace and comments are dropped.
pub fn lex(source: &str) -> Result<Vec<Token>, Vec<LoomError>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut lexer = TokenKind::lexer(source);
    while let Some(result) = lexer.next() {
        let range = lexer.span();
        let span = Span::new(range.start, range.end);
        let text = lexer.slice().to_string();

        match result {
            Ok(kind) => {
                tokens.push(Token { kind, text, span });
            }
            Err(_) => {
                errors.push(LoomError::new(
                    format!("unexpected character: {:?}", &source[range.clone()]),
                    span,
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}
