//! Tokenizer for the Loom language using the `logos` crate.
//!
//! The public API is the [`Lexer`] struct whose [`Lexer::tokenize`] method
//! converts a source string into a `Vec<(Token, Span)>` or a list of
//! [`LoomError::LexError`] values.

use logos::Logos;

use crate::ast::Span;
use crate::error::LoomError;

// ── Token definition ─────────────────────────────────────────────────────────

/// Lexical token for the Loom language.
///
/// Token priority rules (applied by logos at each position):
/// - `#[token("...")]` (keyword/punctuation) beats `#[regex(...)]` (identifier).
/// - Multi-character tokens (`::`, `->`, `|>`, `>=`, `<=`, `!=`) beat their
///   single-character prefixes due to longest-match semantics.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = ())]
// Skip ASCII whitespace between tokens.
#[logos(skip r"[ \t\r\n\f]+")]
// Skip single-line comments (Loom uses `--`).
#[logos(skip r"--[^\n]*")]
// Skip block comments `{- ... -}` (non-nested, simplified).
#[logos(skip r"\{-[^-]*(-+[^}-][^-]*)*-+\}")]
pub enum Token {
    // ── Keywords ─────────────────────────────────────────────────────────────
    #[token("module")]   Module,
    #[token("fn")]       Fn,
    #[token("type")]     Type,
    #[token("enum")]     Enum,
    #[token("let")]      Let,
    #[token("match")]    Match,
    #[token("with")]     With,
    #[token("require")]  Require,
    #[token("ensure")]   Ensure,
    #[token("import")]   Import,
    #[token("spec")]     Spec,
    #[token("provides")] Provides,
    #[token("requires")] Requires,
    #[token("effect")]   Effect,
    #[token("where")]    Where,
    #[token("end")]      End,
    #[token("of")]       Of,
    #[token("then")]     Then,
    #[token("if")]       If,
    #[token("else")]     Else,
    #[token("and")]      And,
    #[token("or")]       Or,
    #[token("not")]      Not,
    #[token("as")]       As,
    #[token("for")]      For,
    #[token("in")]       In,
    #[token("invariant")] Invariant,
    #[token("test")]     Test,
    #[token("interface")] Interface,
    #[token("implements")] Implements,
    #[token("flow")]      Flow,
    #[token("lifecycle")] Lifecycle,
    #[token("being")]    Being,
    #[token("telos")]    Telos,
    #[token("form")]     Form,
    #[token("matter")]   Matter,
    #[token("regulate")] Regulate,
    #[token("evolve")]   Evolve,
    #[token("toward")]   Toward,
    #[token("search")]   Search,
    #[token("fitness")]  Fitness,
    #[token("bounds")]   Bounds,
    #[token("ecosystem")] Ecosystem,
    #[token("members")]  Members,
    #[token("signal")]   Signal,
    #[token("from")]     From,
    #[token("to")]       To,
    #[token("payload")]  Payload,
    #[token("autopoietic")] Autopoietic,
    #[token("epigenetic")] Epigenetic,
    #[token("modifies")]   Modifies,
    #[token("reverts_when")] RevertsWhen,
    #[token("morphogen")]  Morphogen,
    #[token("threshold")]  Threshold,
    #[token("produces")]   Produces,
    #[token("telomere")]   Telomere,
    #[token("on_exhaustion")] OnExhaustion,
    #[token("limit")]      Limit,
    #[token("crispr")]     Crispr,
    #[token("replace")]    Replace,
    #[token("guide")]      Guide,
    #[token("quorum")]     Quorum,
    #[token("action")]     Action,
    #[token("plasticity")] Plasticity,
    #[token("trigger")]    Trigger,
    #[token("rule")]       Rule,
    #[token("hebbian")]    Hebbian,
    #[token("boltzmann")]  Boltzmann,
    #[token("modifiable_by")] ModifiableBy,
    #[token("bounded_by")]    BoundedBy,
    #[token("temporal")]      Temporal,
    #[token("always")]        Always,
    #[token("eventually")]    Eventually,
    #[token("never")]         Never,
    #[token("precedes")]      Precedes,
    #[token("reaches")]       Reaches,
    #[token("transitions")]   Transitions,
    #[token("before")]        Before,
    // ── Separation logic keywords (M57) ─────────────────────────────────────
    #[token("separation")]    Separation,
    #[token("owns")]          Owns,
    #[token("disjoint")]      Disjoint,
    #[token("frame")]         Frame,
    #[token("proof")]         Proof,
    // ── Gradual typing keywords (M59) ────────────────────────────────────────────
    #[token("gradual")]      Gradual,
    #[token("boundary")]     Boundary,
    #[token("blame")]        Blame,
    // ── Probabilistic types keywords (M60) ───────────────────────────────────────
    #[token("distribution")] Distribution,
    // ── Dependent types keywords (M61) ───────────────────────────────────────────
    #[token("proposition")]  Proposition,
    #[token("termination")]  Termination,
    // ── Side-channel keywords (M62) ──────────────────────────────────────────────
    #[token("timing_safety")] TimingSafety,
    // ── Category theory keywords (M63) ───────────────────────────────────────────
    #[token("functor")]      Functor,
    #[token("monad")]        Monad,
    #[token("law")]          Law,
    // ── Self-certifying compilation keywords (M65) ───────────────────────────────
    #[token("certificate")]  Certificate,
    // ── AOP aspect keywords (M66) ────────────────────────────────────────────────
    #[token("aspect")]       Aspect,
    #[token("pointcut")]     Pointcut,
    #[token("around")]       Around,
    #[token("after")]        After,
    // ── Annotation algebra keywords (M66b) ───────────────────────────────────────
    #[token("annotation")]   Annotation,

    // ── Biological gap milestones (M68-M77) ──────────────────────────────────────
    #[token("degenerate")]   Degenerate,
    #[token("fallback")]     Fallback,
    #[token("checkpoint")]   Checkpoint,
    #[token("canalize")]     Canalize,
    #[token("pathway")]      Pathway,
    #[token("senescence")]   Senescence,
    #[token("adopt")]        Adopt,

    // ── Biosemiotic signal infrastructure (M78-M82) ───────────────────────────────
    #[token("umwelt")]       Umwelt,
    #[token("sense")]        Sense,
    #[token("resonance")]    Resonance,

    // ── Store declaration keywords (M92) ────────────────────────────────────────
    #[token("store")]       Store,
    #[token("table")]       Table,
    #[token("node")]        GraphNode,
    #[token("edge")]        Edge,
    #[token("ttl")]         Ttl,
    #[token("index")]       Index,
    #[token("retention")]   Retention,
    #[token("resolution")]  Resolution,
    #[token("format")]      Format,
    #[token("compression")] Compression,
    #[token("capacity")]    Capacity,
    #[token("eviction")]    Eviction,
    #[token("fact")]        Fact,
    #[token("dimension")]   Dimension,
    #[token("embedding")]   Embedding,

    // ── Distributed store keywords (M97) ────────────────────────────────────────
    #[token("mapreduce")]   MapReduce,
    #[token("consumer")]    Consumer,
    #[token("offset")]      Offset,
    #[token("partitions")]  Partitions,
    #[token("replication")] Replication,

    // ── Stochastic process types (M88) ──────────────────────────────────────────
    #[token("process")]     Process,

    // ── Session types (M98) ──────────────────────────────────────────────────────
    #[token("session")]   Session,
    #[token("send")]      Send,
    #[token("recv")]      Recv,
    #[token("duality")]   Duality,

    // ── Algebraic effect handlers (M99) ─────────────────────────────────────────
    // NOTE: `effect` (Token::Effect) and `with` (Token::With) already exist.
    #[token("handle")]    Handle,
    #[token("operation")] Operation,

    // ── Documentation liveness manifest (M101) ──────────────────────────────────
    #[token("manifest")]  Manifest,
    #[token("artifact")]  Artifact,
    #[token("reflects")]  Reflects,

    // ── Use-case triple-derivation (M103) ────────────────────────────────────────
    #[token("usecase")]   UseCase,
    #[token("within")]    Within,
    #[token("actor")]     Actor,
    #[token("acceptance")] Acceptance,

    // ── Episodic memory journal (M104) ────────────────────────────────────────────
    #[token("journal")]   Journal,

    // ── Scenario blocks — BDD acceptance criteria (M105) ─────────────────────────
    #[token("scenario")]  Scenario,

    // ── Migration evolution contract (M106) ──────────────────────────────────────
    #[token("migration")] Migration,

    // ── Property-based testing (M109) ────────────────────────────────────────────
    // QuickCheck (Claessen & Hughes 2000) → fast-check (JS) → Hypothesis (Python) → Loom.
    #[token("property")]  Property,
    #[token("forall")]    Forall,
    #[token("shrink")]    Shrink,
    #[token("samples")]   Samples,

    // ── M112: Telos-as-function + M113-M115 (spec session April 2026) ────────────
    #[token("measured_by")]        MeasuredBy,
    #[token("thresholds")]         Thresholds,
    #[token("convergence")]        Convergence,
    #[token("divergence")]         Divergence,
    #[token("propagation")]        Propagation,
    #[token("guides")]             Guides,
    #[token("signal_attention")]   SignalAttention,
    #[token("prioritize")]         Prioritize,
    #[token("attenuate")]          Attenuate,
    #[token("telos_contribution")] TelosContribution,

    // ── M116: Messaging primitives ───────────────────────────────────────────────
    #[token("messaging_primitive")] MessagingPrimitive,
    #[token("guarantees")]          Guarantees,

    // ── Boolean literals (before Ident so `true`/`false` are not identifiers)
    #[token("true",  |_| true)]
    #[token("false", |_| false)]
    BoolLit(bool),

    // ── Numeric literals (float before int to get longest match on `1.5`) ──
    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),

    // ── String literal — simple, no escape processing in Phase 1 ─────────
    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        // Strip surrounding quotes.
        Some(s[1..s.len() - 1].to_string())
    })]
    StrLit(String),

    // ── Inline Rust block: `inline { ... }` ── captured via callback ─────
    // The regex matches `inline` followed by optional whitespace and `{`.
    // The callback then collects everything until the matching closing `}`,
    // handling nested braces. The captured content is the raw Rust string.
    #[regex(r"inline[ \t\r\n]*\{", capture_inline_block)]
    InlineBlock(String),

    // ── Identifier (must come after keywords so keywords take priority) ───
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_']*", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Multi-character operators (must precede single-char prefixes) ─────
    #[token("::")] ColonColon,
    #[token("->")] Arrow,
    #[token("|>")] Pipe,
    #[token("!=")] Ne,
    #[token(">=")] Ge,
    #[token("<=")] Le,

    // ── Single-character operators ────────────────────────────────────────
    #[token("|")] Bar,
    #[token("=")] Eq,
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,

    // ── Punctuation ───────────────────────────────────────────────────────
    #[token(":")] Colon,
    #[token(",")] Comma,
    #[token(".")] Dot,
    #[token("[")] LBracket,
    #[token("]")] RBracket,
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    /// `<` — used as both less-than operator and generic opening angle bracket.
    #[token("<")] Lt,
    /// `>` — used as both greater-than operator and generic closing angle bracket.
    #[token(">")] Gt,
    #[token("~")] Tilde,
    #[token("?")] Question,
    #[token("@")] At,
    // ── Provenance (M102) ─────────────────────────────────────────────────────
    #[token("provenance")]  Provenance,
    // ── Boundary exports/seals (M103) ─────────────────────────────────────────
    #[token("export")]  Export,
    #[token("seal")]    Seal,
}

// Aliases for use in type-expression parsing contexts.
// The tokens are identical; these constants document the dual usage.
pub const TOKEN_LANGLE: &Token = &Token::Lt;
pub const TOKEN_RANGLE: &Token = &Token::Gt;

// ── Inline block callback ─────────────────────────────────────────────────────

/// Logos callback for `inline { ... }` blocks.
///
/// Called after `inline\s*{` has been matched. `lex.remainder()` is the source
/// text starting immediately after the opening `{`. The callback scans forward
/// counting brace depth until the matching `}`, then advances the lexer past it
/// and returns the captured content as the token payload.
fn capture_inline_block(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();
    let mut depth = 1usize;
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let content = remainder[..i].to_string();
                    // Advance the lexer past the content and the closing '}'.
                    lex.bump(i + 1);
                    return Some(content);
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Unmatched opening brace — lex error.
    None
}

// ── Lexer struct ─────────────────────────────────────────────────────────────

/// Stateless tokenizer entry point.
///
/// # Examples
///
/// ```rust,ignore
/// let tokens = Lexer::tokenize("fn add :: Int -> Int -> Int").unwrap();
/// ```
pub struct Lexer;

impl Lexer {
    /// Tokenize `src` into a sequence of `(token, span)` pairs.
    ///
    /// Returns `Ok(tokens)` when there are no lex errors, or `Err(errors)` if
    /// one or more characters could not be recognised.  A single call always
    /// collects *all* errors before returning, so the caller receives the
    /// complete diagnostic list.
    pub fn tokenize(src: &str) -> Result<Vec<(Token, Span)>, Vec<LoomError>> {
        use logos::Logos as _;

        let mut tokens: Vec<(Token, Span)> = Vec::new();
        let mut errors: Vec<LoomError> = Vec::new();

        let mut lex = Token::lexer(src);
        loop {
            match lex.next() {
                None => break,
                Some(Ok(token)) => {
                    let r = lex.span();
                    tokens.push((token, Span::new(r.start, r.end)));
                }
                Some(Err(())) => {
                    let r = lex.span();
                    errors.push(LoomError::lex(
                        format!("unexpected character(s): {:?}", lex.slice()),
                        Span::new(r.start, r.end),
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
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_keywords() {
        let tokens = Lexer::tokenize("module fn type enum").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(kinds, vec![Token::Module, Token::Fn, Token::Type, Token::Enum]);
    }

    #[test]
    fn tokenizes_integer_literal() {
        let tokens = Lexer::tokenize("42").unwrap();
        assert_eq!(tokens[0].0, Token::IntLit(42));
    }

    #[test]
    fn tokenizes_float_literal() {
        let tokens = Lexer::tokenize("3.14").unwrap();
        assert_eq!(tokens[0].0, Token::FloatLit(3.14));
    }

    #[test]
    fn tokenizes_bool_literals() {
        let tokens = Lexer::tokenize("true false").unwrap();
        assert_eq!(tokens[0].0, Token::BoolLit(true));
        assert_eq!(tokens[1].0, Token::BoolLit(false));
    }

    #[test]
    fn tokenizes_multi_char_operators() {
        let tokens = Lexer::tokenize(":: -> |> != >= <=").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                Token::ColonColon,
                Token::Arrow,
                Token::Pipe,
                Token::Ne,
                Token::Ge,
                Token::Le,
            ]
        );
    }

    #[test]
    fn skips_line_comments() {
        let tokens = Lexer::tokenize("fn -- this is a comment\ntype").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(kinds, vec![Token::Fn, Token::Type]);
    }

    #[test]
    fn errors_on_unknown_character() {
        // `$` is not a valid Loom token.
        let result = Lexer::tokenize("fn $ type");
        assert!(result.is_err());
    }
}
