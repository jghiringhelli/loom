//! Recursive-descent LL(2) parser for the Loom language.
//!
//! The [`Parser`] struct consumes a slice of `(Token, Span)` pairs produced by
//! the lexer and builds an [`ast::Module`].  Phase 1 implements the full
//! production rules needed to parse the corpus examples; complex sub-forms not
//! yet exercised return a descriptive `ParseError` so they fail loudly.

use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

// ── Parser struct ─────────────────────────────────────────────────────────────

/// Recursive-descent parser backed by a token slice.
pub struct Parser<'src> {
    tokens: &'src [(Token, Span)],
    pos: usize,
    /// Collected consequence tiers from `Effect<[X@tier, ...]>` — consumed by `parse_fn_def`.
    pub pending_effect_tiers: Vec<(String, ConsequenceTier)>,
    /// Annotations parsed before a `fn` keyword at item level — merged into the fn.
    pub pending_annotations: Vec<Annotation>,
}

mod being;
mod expressions;
mod items;
mod types_parser;

impl<'src> Parser<'src> {
    /// Create a new parser for the given token slice.
    pub fn new(tokens: &'src [(Token, Span)]) -> Self {
        Parser {
            tokens,
            pos: 0,
            pending_effect_tiers: Vec::new(),
            pending_annotations: Vec::new(),
        }
    }

    // ── Token navigation ──────────────────────────────────────────────────

    /// Peek at the current token without advancing.
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    /// Peek at the token one position ahead (LL(2) look-ahead).
    pub fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|(t, _)| t)
    }

    /// Return the span of the current token, or a synthetic span at EOF.
    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(Span::synthetic)
    }

    /// Advance past the current token and return the consumed `(token, span)`.
    pub fn advance(&mut self) -> Option<&(Token, Span)> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    /// Return `true` if the current token equals `expected`.
    pub fn at(&self, expected: &Token) -> bool {
        // Use Debug-based comparison for Token variants that contain data.
        match (self.peek(), expected) {
            (Some(t), e) => t == e,
            _ => false,
        }
    }

    /// Advance and return the span if the current token matches `expected`;
    /// otherwise return a [`LoomError::ParseError`].
    pub fn expect(&mut self, expected: Token) -> Result<Span, LoomError> {
        match self.tokens.get(self.pos) {
            Some((tok, span)) if tok == &expected => {
                let s = span.clone();
                self.pos += 1;
                Ok(s)
            }
            Some((tok, span)) => Err(LoomError::parse(
                format!("expected {:?}, found {:?}", expected, tok),
                span.clone(),
            )),
            None => Err(LoomError::parse(
                format!("expected {:?}, found end of input", expected),
                Span::synthetic(),
            )),
        }
    }

    /// Consume the current `Ident` token and return its string value and span.
    fn expect_ident(&mut self) -> Result<(String, Span), LoomError> {
        match self.tokens.get(self.pos) {
            Some((Token::Ident(name), span)) => {
                let name = name.clone();
                let span = span.clone();
                self.pos += 1;
                Ok((name, span))
            }
            Some((tok, span)) => Err(LoomError::parse(
                format!("expected identifier, found {:?}", tok),
                span.clone(),
            )),
            None => Err(LoomError::parse(
                "expected identifier, found end of input",
                Span::synthetic(),
            )),
        }
    }

    /// Like `expect_ident` but also accepts keyword tokens as contextual identifiers.
    /// Used where field names, variable names, or other names may shadow keywords.
    fn expect_any_name(&mut self) -> Result<(String, Span), LoomError> {
        let name = match self.tokens.get(self.pos) {
            Some((Token::Ident(n), _)) => n.clone(),
            Some((tok, _)) => {
                if let Some(s) = token_keyword_str(tok) {
                    s.to_string()
                } else {
                    return self.expect_ident();
                }
            }
            None => {
                return Err(LoomError::parse(
                    "expected identifier, found end of input",
                    Span::synthetic(),
                ))
            }
        };
        let span = self.tokens[self.pos].1.clone();
        self.pos += 1;
        Ok((name, span))
    }

    // ── Top-level ─────────────────────────────────────────────────────────

    /// Parse `describe: "..."` if present, returning the description string.
    fn parse_describe(&mut self) -> Option<String> {
        // describe: is `Ident("describe") Colon StrLit`
        if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "describe") {
            if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos + 2) {
                    let s = s.clone();
                    self.pos += 3;
                    return Some(s);
                }
            }
        }
        None
    }

    /// Parse zero or more `@key("value")` annotations.
    ///
    /// Annotation keys may contain hyphens (e.g. `@encrypt-at-rest`, `@never-log`);
    /// hyphens are consumed by joining adjacent `Ident - Ident` sequences.
    fn parse_annotations(&mut self) -> Vec<Annotation> {
        let mut annotations = Vec::new();
        while self.at(&Token::At) {
            self.advance(); // consume `@`
                            // The key starts with an identifier (or keyword used as identifier)
                            // and may continue with `-ident` segments.
            if let Some(first) = self.token_as_ident() {
                let mut key = first;
                self.advance();
                // Consume `-ident` segments to support hyphenated keys.
                while self.at(&Token::Minus) {
                    if let Some(seg) = self.peek_next_as_ident() {
                        self.advance(); // consume `-`
                        key.push('-');
                        key.push_str(&seg);
                        self.advance();
                    } else {
                        break;
                    }
                }
                // Optional `("value")`, `(Ident)`, or `(Ident.field)` payload.
                // Collect all tokens between `(` and `)` as a raw string so that
                // annotation values like `@foreign_key(Users.id)` are preserved intact.
                let value = if self.at(&Token::LParen) {
                    self.advance(); // consume `(`
                    let mut parts = Vec::new();
                    let mut depth = 1usize;
                    while depth > 0 {
                        match self.tokens.get(self.pos) {
                            Some((Token::LParen, _)) => {
                                depth += 1;
                                parts.push("(".to_string());
                                self.advance();
                            }
                            Some((Token::RParen, _)) => {
                                depth -= 1;
                                if depth > 0 {
                                    parts.push(")".to_string());
                                }
                                self.advance();
                            }
                            Some((Token::Dot, _)) => {
                                parts.push(".".to_string());
                                self.advance();
                            }
                            Some((Token::StrLit(v), _)) => {
                                parts.push(v.clone());
                                self.advance();
                            }
                            None => break,
                            _ => {
                                if let Some(s) = self.token_as_ident() {
                                    parts.push(s);
                                } else {
                                    parts.push(token_to_source(&self.tokens[self.pos].0));
                                }
                                self.advance();
                            }
                        }
                    }
                    parts.join("")
                } else {
                    String::new()
                };
                annotations.push(Annotation { key, value });
            }
        }
        annotations
    }

    /// Try to interpret the current token as an identifier string.
    /// Handles keyword tokens that may appear as annotation keys.
    fn token_as_ident(&self) -> Option<String> {
        match self.tokens.get(self.pos) {
            Some((Token::Ident(s), _)) => Some(s.clone()),
            Some((Token::Never, _)) => Some("never".to_string()),
            Some((Token::Always, _)) => Some("always".to_string()),
            Some((Token::Before, _)) => Some("before".to_string()),
            Some((Token::Temporal, _)) => Some("temporal".to_string()),
            Some((Token::Eventually, _)) => Some("eventually".to_string()),
            Some((Token::Precedes, _)) => Some("precedes".to_string()),
            Some((Token::Reaches, _)) => Some("reaches".to_string()),
            Some((Token::Transitions, _)) => Some("transitions".to_string()),
            Some((Token::Separation, _)) => Some("separation".to_string()),
            Some((Token::Owns, _)) => Some("owns".to_string()),
            Some((Token::Disjoint, _)) => Some("disjoint".to_string()),
            Some((Token::Frame, _)) => Some("frame".to_string()),
            Some((Token::Proof, _)) => Some("proof".to_string()),
            Some((Token::Aspect, _)) => Some("aspect".to_string()),
            Some((Token::Pointcut, _)) => Some("pointcut".to_string()),
            Some((Token::Around, _)) => Some("around".to_string()),
            Some((Token::After, _)) => Some("after".to_string()),
            Some((Token::Annotation, _)) => Some("annotation".to_string()),
            Some((Token::Gradual, _)) => Some("gradual".to_string()),
            Some((Token::Boundary, _)) => Some("boundary".to_string()),
            Some((Token::Blame, _)) => Some("blame".to_string()),
            Some((Token::Distribution, _)) => Some("distribution".to_string()),
            Some((Token::Proposition, _)) => Some("proposition".to_string()),
            Some((Token::Termination, _)) => Some("termination".to_string()),
            Some((Token::TimingSafety, _)) => Some("timing_safety".to_string()),
            Some((Token::Functor, _)) => Some("functor".to_string()),
            Some((Token::Monad, _)) => Some("monad".to_string()),
            Some((Token::Law, _)) => Some("law".to_string()),
            Some((Token::Certificate, _)) => Some("certificate".to_string()),
            Some((Token::Degenerate, _)) => Some("degenerate".to_string()),
            Some((Token::Fallback, _)) => Some("fallback".to_string()),
            Some((Token::Checkpoint, _)) => Some("checkpoint".to_string()),
            Some((Token::Canalize, _)) => Some("canalize".to_string()),
            Some((Token::Pathway, _)) => Some("pathway".to_string()),
            Some((Token::Senescence, _)) => Some("senescence".to_string()),
            Some((Token::Adopt, _)) => Some("adopt".to_string()),
            Some((Token::Toward, _)) => Some("toward".to_string()),
            Some((Token::Modifies, _)) => Some("modifies".to_string()),
            Some((Token::From, _)) => Some("from".to_string()),
            Some((Token::Requires, _)) => Some("requires".to_string()),
            Some((Token::Module, _)) => Some("module".to_string()),
            Some((Token::Umwelt, _)) => Some("umwelt".to_string()),
            Some((Token::Sense, _)) => Some("sense".to_string()),
            Some((Token::Resonance, _)) => Some("resonance".to_string()),
            Some((Token::Store, _)) => Some("store".to_string()),
            Some((Token::Table, _)) => Some("table".to_string()),
            Some((Token::GraphNode, _)) => Some("node".to_string()),
            Some((Token::Edge, _)) => Some("edge".to_string()),
            Some((Token::Ttl, _)) => Some("ttl".to_string()),
            Some((Token::Index, _)) => Some("index".to_string()),
            Some((Token::Retention, _)) => Some("retention".to_string()),
            Some((Token::Resolution, _)) => Some("resolution".to_string()),
            Some((Token::Format, _)) => Some("format".to_string()),
            Some((Token::Compression, _)) => Some("compression".to_string()),
            Some((Token::Capacity, _)) => Some("capacity".to_string()),
            Some((Token::Eviction, _)) => Some("eviction".to_string()),
            Some((Token::Fact, _)) => Some("fact".to_string()),
            Some((Token::Dimension, _)) => Some("dimension".to_string()),
            Some((Token::Embedding, _)) => Some("embedding".to_string()),
            Some((Token::MapReduce, _)) => Some("mapreduce".to_string()),
            Some((Token::Consumer, _)) => Some("consumer".to_string()),
            Some((Token::Offset, _)) => Some("offset".to_string()),
            Some((Token::Partitions, _)) => Some("partitions".to_string()),
            Some((Token::Replication, _)) => Some("replication".to_string()),
            Some((Token::Bounds, _)) => Some("bounds".to_string()),
            Some((Token::Process, _)) => Some("process".to_string()),
            Some((Token::Session, _)) => Some("session".to_string()),
            Some((Token::Send, _)) => Some("send".to_string()),
            Some((Token::Recv, _)) => Some("recv".to_string()),
            Some((Token::Duality, _)) => Some("duality".to_string()),
            Some((Token::Handle, _)) => Some("handle".to_string()),
            Some((Token::Operation, _)) => Some("operation".to_string()),
            Some((Token::Implements, _)) => Some("implements".to_string()),
            Some((Token::Export, _)) => Some("export".to_string()),
            Some((Token::Seal, _)) => Some("seal".to_string()),
            Some((Token::Provenance, _)) => Some("provenance".to_string()),
            Some((Token::Convergence, _)) => Some("convergence".to_string()),
            Some((Token::Divergence, _)) => Some("divergence".to_string()),
            Some((Token::Telos, _)) => Some("telos".to_string()),
            Some((Token::Matter, _)) => Some("matter".to_string()),
            Some((Token::Telomere, _)) => Some("telomere".to_string()),
            Some((Token::SignalAttention, _)) => Some("signal_attention".to_string()),
            Some((Token::Plasticity, _)) => Some("plasticity".to_string()),
            Some((Token::Regulate, _)) => Some("regulate".to_string()),
            Some((Token::Evolve, _)) => Some("evolve".to_string()),
            Some((Token::Guides, _)) => Some("guides".to_string()),
            Some((Token::BoundedBy, _)) => Some("bounded_by".to_string()),
            Some((Token::MeasuredBy, _)) => Some("measured_by".to_string()),
            Some((Token::Thresholds, _)) => Some("thresholds".to_string()),
            Some((Token::TelosFunction, _)) => Some("telos_function".to_string()),
            Some((Token::Entity, _)) => Some("entity".to_string()),
            Some((Token::IntentCoordinator, _)) => Some("intent_coordinator".to_string()),
            Some((Token::Payload, _)) => Some("payload".to_string()),
            Some((Token::States, _)) => Some("states".to_string()),
            Some((Token::ChainKw, _)) => Some("chain".to_string()),
            Some((Token::DagKw, _)) => Some("dag".to_string()),
            Some((Token::Nodes, _)) => Some("nodes".to_string()),
            Some((Token::Edges, _)) => Some("edges".to_string()),
            Some((Token::Const, _)) => Some("const".to_string()),
            Some((Token::PipelineKw, _)) => Some("pipeline".to_string()),
            Some((Token::Step, _)) => Some("step".to_string()),
            Some((Token::SagaKw, _)) => Some("saga".to_string()),
            Some((Token::Compensate, _)) => Some("compensate".to_string()),
            Some((Token::EventKw, _)) => Some("event".to_string()),
            Some((Token::CommandKw, _)) => Some("command".to_string()),
            Some((Token::QueryKw, _)) => Some("query".to_string()),
            Some((Token::CircuitBreakerKw, _)) => Some("circuit_breaker".to_string()),
            Some((Token::Threshold, _)) => Some("threshold".to_string()),
            Some((Token::RetryKw, _)) => Some("retry".to_string()),
            Some((Token::RateLimiterKw, _)) => Some("rate_limiter".to_string()),
            _ => None,
        }
    }

    /// Try to interpret the next token (pos+1) as an identifier string.
    fn peek_next_as_ident(&self) -> Option<String> {
        match self.tokens.get(self.pos + 1) {
            Some((Token::Ident(s), _)) => Some(s.clone()),
            Some((Token::Never, _)) => Some("never".to_string()),
            Some((Token::Always, _)) => Some("always".to_string()),
            Some((Token::Before, _)) => Some("before".to_string()),
            Some((Token::Temporal, _)) => Some("temporal".to_string()),
            Some((Token::Eventually, _)) => Some("eventually".to_string()),
            Some((Token::Precedes, _)) => Some("precedes".to_string()),
            Some((Token::Reaches, _)) => Some("reaches".to_string()),
            Some((Token::Transitions, _)) => Some("transitions".to_string()),
            Some((Token::Gradual, _)) => Some("gradual".to_string()),
            Some((Token::Boundary, _)) => Some("boundary".to_string()),
            Some((Token::Blame, _)) => Some("blame".to_string()),
            Some((Token::Distribution, _)) => Some("distribution".to_string()),
            Some((Token::Proposition, _)) => Some("proposition".to_string()),
            Some((Token::Termination, _)) => Some("termination".to_string()),
            Some((Token::TimingSafety, _)) => Some("timing_safety".to_string()),
            Some((Token::Functor, _)) => Some("functor".to_string()),
            Some((Token::Monad, _)) => Some("monad".to_string()),
            Some((Token::Law, _)) => Some("law".to_string()),
            Some((Token::Certificate, _)) => Some("certificate".to_string()),
            Some((Token::Degenerate, _)) => Some("degenerate".to_string()),
            Some((Token::Fallback, _)) => Some("fallback".to_string()),
            Some((Token::Checkpoint, _)) => Some("checkpoint".to_string()),
            Some((Token::Canalize, _)) => Some("canalize".to_string()),
            Some((Token::Pathway, _)) => Some("pathway".to_string()),
            Some((Token::Senescence, _)) => Some("senescence".to_string()),
            Some((Token::Adopt, _)) => Some("adopt".to_string()),
            Some((Token::Toward, _)) => Some("toward".to_string()),
            Some((Token::Modifies, _)) => Some("modifies".to_string()),
            Some((Token::From, _)) => Some("from".to_string()),
            Some((Token::Requires, _)) => Some("requires".to_string()),
            Some((Token::Module, _)) => Some("module".to_string()),
            Some((Token::Umwelt, _)) => Some("umwelt".to_string()),
            Some((Token::Sense, _)) => Some("sense".to_string()),
            Some((Token::Resonance, _)) => Some("resonance".to_string()),
            Some((Token::Store, _)) => Some("store".to_string()),
            Some((Token::Table, _)) => Some("table".to_string()),
            Some((Token::GraphNode, _)) => Some("node".to_string()),
            Some((Token::Edge, _)) => Some("edge".to_string()),
            Some((Token::Ttl, _)) => Some("ttl".to_string()),
            Some((Token::Index, _)) => Some("index".to_string()),
            Some((Token::Retention, _)) => Some("retention".to_string()),
            Some((Token::Resolution, _)) => Some("resolution".to_string()),
            Some((Token::Format, _)) => Some("format".to_string()),
            Some((Token::Compression, _)) => Some("compression".to_string()),
            Some((Token::Capacity, _)) => Some("capacity".to_string()),
            Some((Token::Eviction, _)) => Some("eviction".to_string()),
            Some((Token::Fact, _)) => Some("fact".to_string()),
            Some((Token::Dimension, _)) => Some("dimension".to_string()),
            Some((Token::Embedding, _)) => Some("embedding".to_string()),
            Some((Token::MapReduce, _)) => Some("mapreduce".to_string()),
            Some((Token::Consumer, _)) => Some("consumer".to_string()),
            Some((Token::Offset, _)) => Some("offset".to_string()),
            Some((Token::Partitions, _)) => Some("partitions".to_string()),
            Some((Token::Replication, _)) => Some("replication".to_string()),
            Some((Token::Session, _)) => Some("session".to_string()),
            Some((Token::Process, _)) => Some("process".to_string()),
            Some((Token::Send, _)) => Some("send".to_string()),
            Some((Token::Recv, _)) => Some("recv".to_string()),
            Some((Token::Duality, _)) => Some("duality".to_string()),
            Some((Token::Handle, _)) => Some("handle".to_string()),
            Some((Token::Operation, _)) => Some("operation".to_string()),
            Some((Token::Export, _)) => Some("export".to_string()),
            Some((Token::Seal, _)) => Some("seal".to_string()),
            Some((Token::Provenance, _)) => Some("provenance".to_string()),
            Some((Token::Convergence, _)) => Some("convergence".to_string()),
            Some((Token::Divergence, _)) => Some("divergence".to_string()),
            Some((Token::TelosFunction, _)) => Some("telos_function".to_string()),
            Some((Token::Entity, _)) => Some("entity".to_string()),
            Some((Token::IntentCoordinator, _)) => Some("intent_coordinator".to_string()),
            Some((Token::Payload, _)) => Some("payload".to_string()),
            Some((Token::States, _)) => Some("states".to_string()),
            Some((Token::ChainKw, _)) => Some("chain".to_string()),
            Some((Token::DagKw, _)) => Some("dag".to_string()),
            Some((Token::Nodes, _)) => Some("nodes".to_string()),
            Some((Token::Edges, _)) => Some("edges".to_string()),
            Some((Token::Const, _)) => Some("const".to_string()),
            Some((Token::PipelineKw, _)) => Some("pipeline".to_string()),
            Some((Token::Step, _)) => Some("step".to_string()),
            Some((Token::SagaKw, _)) => Some("saga".to_string()),
            Some((Token::Compensate, _)) => Some("compensate".to_string()),
            Some((Token::EventKw, _)) => Some("event".to_string()),
            Some((Token::CommandKw, _)) => Some("command".to_string()),
            Some((Token::QueryKw, _)) => Some("query".to_string()),
            Some((Token::CircuitBreakerKw, _)) => Some("circuit_breaker".to_string()),
            Some((Token::Threshold, _)) => Some("threshold".to_string()),
            Some((Token::RetryKw, _)) => Some("retry".to_string()),
            Some((Token::RateLimiterKw, _)) => Some("rate_limiter".to_string()),
            _ => None,
        }
    }

    /// Parse a complete `module … end` block.
    pub fn parse_module(&mut self) -> Result<Module, LoomError> {
        let start = self.current_span();
        self.expect(Token::Module)?;
        let (name, _) = self.expect_ident()?;

        // Optional describe: and @annotations in the module header
        let describe = self.parse_describe();
        let annotations = self.parse_annotations();

        // `import ModuleName` lines (zero or more, before the rest)
        let mut imports = Vec::new();
        while self.at(&Token::Import) {
            self.advance();
            let (imp_name, _) = self.expect_ident()?;
            imports.push(imp_name);
        }

        // Optional `spec NAME`
        let spec = if self.at(&Token::Spec) {
            self.advance();
            let (spec_name, _) = self.expect_ident()?;
            Some(spec_name)
        } else {
            None
        };

        // `implements InterfaceName` declarations
        let mut implements = Vec::new();
        while self.at(&Token::Implements) {
            self.advance();
            let (iface, _) = self.expect_ident()?;
            implements.push(iface);
        }

        // Optional `provides { … }`
        let provides = if self.at(&Token::Provides) {
            self.advance();
            Some(self.parse_provides_block()?)
        } else {
            None
        };

        // Optional `requires { … }`
        let requires = if self.at(&Token::Requires) {
            self.advance();
            Some(self.parse_requires_block()?)
        } else {
            None
        };

        // Item list until `end` — `invariant`, `test`, `interface` entries parsed separately.
        let mut items = Vec::new();
        let mut invariants = Vec::new();
        let mut test_defs = Vec::new();
        let mut interface_defs = Vec::new();
        let mut lifecycle_defs = Vec::new();
        let mut temporal_defs = Vec::new();
        let mut being_defs = Vec::new();
        let mut ecosystem_defs = Vec::new();
        let mut flow_labels = Vec::new();
        let mut aspect_defs = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Invariant) {
                invariants.push(self.parse_invariant()?);
            } else if self.at(&Token::Test) {
                test_defs.push(self.parse_test_def()?);
            } else if self.at(&Token::Interface) {
                interface_defs.push(self.parse_interface_def()?);
            } else if self.at(&Token::Lifecycle) {
                lifecycle_defs.push(self.parse_lifecycle_def()?);
            } else if self.at(&Token::Temporal) {
                temporal_defs.push(self.parse_temporal_def()?);
            } else if self.at(&Token::Being) {
                being_defs.push(self.parse_being_def()?);
            } else if self.at(&Token::Ecosystem) {
                ecosystem_defs.push(self.parse_ecosystem_def()?);
            } else if self.at(&Token::Flow) {
                flow_labels.push(self.parse_flow_label()?);
            } else if self.at(&Token::Aspect) {
                aspect_defs.push(self.parse_aspect_def()?);
            } else if self.at(&Token::Implements) {
                // `implements Name` can also appear inline in the module body
                self.advance();
                let (iface, _) = self.expect_ident()?;
                implements.push(iface);
            } else if self.at(&Token::Import) {
                // `import Name` can also appear inline in the module body
                self.advance();
                let (imp, _) = self.expect_ident()?;
                imports.push(imp);
            } else if self.at(&Token::Adopt) {
                items.push(Item::Adopt(self.parse_adopt_decl()?));
            } else if self.at(&Token::Pathway) {
                items.push(Item::Pathway(self.parse_pathway_def()?));
            } else if self.at(&Token::UseCase) {
                items.push(Item::UseCase(self.parse_usecase_block()?));
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "symbiotic")
            {
                items.push(self.parse_symbiotic_import()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "niche_construction")
            {
                items.push(Item::NicheConstruction(self.parse_niche_construction()?));
            } else if self.at(&Token::MessagingPrimitive) {
                items.push(Item::MessagingPrimitive(self.parse_messaging_primitive()?));
            } else if self.at(&Token::Discipline) {
                items.push(Item::Discipline(self.parse_discipline_decl()?));
            } else if self.at(&Token::ChainKw) {
                items.push(Item::Chain(self.parse_chain_def()?));
            } else if self.at(&Token::DagKw) {
                items.push(Item::Dag(self.parse_dag_def()?));
            } else if self.at(&Token::Const) {
                items.push(Item::Const(self.parse_const_def()?));
            } else if self.at(&Token::PipelineKw) {
                items.push(Item::Pipeline(self.parse_pipeline_def()?));
            } else if self.at(&Token::SagaKw) {
                items.push(Item::Saga(self.parse_saga_def()?));
            } else if self.at(&Token::EventKw) {
                items.push(Item::Event(self.parse_event_def()?));
            } else if self.at(&Token::CommandKw) {
                items.push(Item::Command(self.parse_command_def()?));
            } else if self.at(&Token::QueryKw) {
                items.push(Item::Query(self.parse_query_def()?));
            } else if self.at(&Token::CircuitBreakerKw) {
                items.push(Item::CircuitBreaker(self.parse_circuit_breaker_def()?));
            } else if self.at(&Token::RetryKw) {
                items.push(Item::Retry(self.parse_retry_def()?));
            } else if self.at(&Token::RateLimiterKw) {
                items.push(Item::RateLimiter(self.parse_rate_limiter_def()?));
            } else if self.at(&Token::At) {
                // `@key("value")` before a fn — accumulate as pending annotations.
                let anns = self.parse_annotations();
                self.pending_annotations.extend(anns);
            } else {
                items.push(self.parse_item()?);
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(Module {
            name,
            describe,
            annotations,
            imports,
            spec,
            interface_defs,
            implements,
            provides,
            requires,
            invariants,
            test_defs,
            lifecycle_defs,
            temporal_defs,
            being_defs,
            ecosystem_defs,
            flow_labels,
            aspect_defs,
            items,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Helper: parse optional `<A, B, ...>` type parameter list.
    fn parse_optional_type_params(&mut self) -> Result<Vec<String>, LoomError> {
        if self.at(&Token::Lt) {
            self.advance();
            let mut params = Vec::new();
            while !self.at(&Token::Gt) && self.peek().is_some() {
                let (param, _) = self.expect_ident()?;
                params.push(param);
                if self.at(&Token::Comma) {
                    self.advance();
                }
            }
            self.expect(Token::Gt)?;
            Ok(params)
        } else {
            Ok(Vec::new())
        }
    }

    /// Parse a value as a string — handles idents, string literals, numbers, booleans, `?`.
    fn parse_value_as_string(&mut self) -> Result<String, LoomError> {
        match self.tokens.get(self.pos) {
            Some((Token::StrLit(s), _)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            Some((Token::IntLit(n), _)) => {
                let s = n.to_string();
                self.pos += 1;
                Ok(s)
            }
            Some((Token::FloatLit(f), _)) => {
                let s = f.to_string();
                self.pos += 1;
                Ok(s)
            }
            Some((Token::BoolLit(b), _)) => {
                let s = b.to_string();
                self.pos += 1;
                Ok(s)
            }
            Some((Token::Question, _)) => {
                self.pos += 1;
                Ok("?".to_string())
            }
            Some((Token::LBracket, _)) => {
                // Parse a bracket list like [0.0, 1.0] into a single string.
                self.pos += 1; // consume [
                let mut parts = Vec::new();
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    let part = self.parse_value_as_string()?;
                    parts.push(part);
                    if self.at(&Token::Comma) {
                        self.pos += 1;
                    }
                }
                if self.at(&Token::RBracket) {
                    self.pos += 1; // consume ]
                }
                Ok(format!("[{}]", parts.join(", ")))
            }
            _ => {
                if let Some(name) = self.token_as_ident() {
                    self.pos += 1;
                    Ok(name)
                } else {
                    Err(LoomError::parse(
                        format!(
                            "expected value, got {:?}",
                            self.tokens.get(self.pos).map(|(t, _)| t)
                        ),
                        self.current_span(),
                    ))
                }
            }
        }
    }

    /// Parse `flow label :: TypeA, TypeB, ...`.
    fn parse_flow_label(&mut self) -> Result<FlowLabel, LoomError> {
        let start = self.current_span();
        self.expect(Token::Flow)?;
        let (label, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let mut types = Vec::new();
        let (first_type, _) = self.expect_ident()?;
        types.push(first_type);
        while self.at(&Token::Comma) {
            self.advance();
            let (t, _) = self.expect_ident()?;
            types.push(t);
        }
        let end_span = self.current_span();
        Ok(FlowLabel {
            label,
            types,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Consume tokens until the start of the next scenario clause and return
    /// the consumed tokens as a joined debug string.  Stops at `end`,
    /// `given`, `when`, `then`, `within` (next clause starters) or EOF.
    fn collect_rest_of_line(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            match self.tokens.get(self.pos) {
                None => break,
                Some((Token::End, _)) => break,
                // Stop at the start of the next scenario clause.
                Some((Token::Then, _)) | Some((Token::Within, _)) => break,
                Some((Token::Ident(n), _))
                    if matches!(n.as_str(), "given" | "when")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) =>
                {
                    break;
                }
                Some((tok, _)) => {
                    parts.push(format!("{:?}", tok));
                    self.pos += 1;
                }
            }
        }
        parts.join(" ")
    }

    // ── Items ────────────────────────────────────────────────────────────

    /// Dispatch to the correct item parser based on the leading keyword.
    fn parse_item(&mut self) -> Result<Item, LoomError> {
        match self.peek() {
            Some(Token::Fn) => Ok(Item::Fn(self.parse_fn_def()?)),
            Some(Token::Type) => Ok(self.parse_type_or_refined()?),
            Some(Token::Enum) => Ok(Item::Enum(self.parse_enum_def()?)),
            Some(Token::Proposition) => Ok(Item::Proposition(self.parse_proposition_def()?)),
            Some(Token::Functor) => Ok(Item::Functor(self.parse_functor_def()?)),
            Some(Token::Monad) => Ok(Item::Monad(self.parse_monad_def()?)),
            Some(Token::Certificate) => Ok(Item::Certificate(self.parse_certificate_def()?)),
            Some(Token::Annotation) => Ok(Item::AnnotationDecl(self.parse_annotation_decl()?)),
            Some(Token::Ident(s)) if s == "correctness_report" => {
                Ok(Item::CorrectnessReport(self.parse_correctness_report()?))
            }
            Some(Token::Adopt) => Ok(Item::Adopt(self.parse_adopt_decl()?)),
            Some(Token::Pathway) => Ok(Item::Pathway(self.parse_pathway_def()?)),
            Some(Token::Ident(s)) if s == "symbiotic" => Ok(self.parse_symbiotic_import()?),
            Some(Token::Ident(s)) if s == "niche_construction" => {
                Ok(Item::NicheConstruction(self.parse_niche_construction()?))
            }
            Some(Token::Sense) => Ok(Item::Sense(self.parse_sense_def()?)),
            Some(Token::Store) => Ok(Item::Store(self.parse_store_def()?)),
            Some(Token::Session) => Ok(Item::Session(self.parse_session_def()?)),
            // `effect Name ...` top-level definition (Token::Effect is also used in type exprs
            // but type exprs never appear at item level, so this is unambiguous).
            Some(Token::Effect) => Ok(Item::Effect(self.parse_effect_def()?)),
            Some(Token::UseCase) => Ok(Item::UseCase(self.parse_usecase_block()?)),
            Some(Token::Property) => Ok(Item::Property(self.parse_property_block()?)),
            Some(Token::Boundary) => Ok(Item::BoundaryBlock(self.parse_boundary_block()?)),
            Some(Token::MessagingPrimitive) => {
                Ok(Item::MessagingPrimitive(self.parse_messaging_primitive()?))
            }
            Some(Token::TelosFunction) => Ok(Item::TelosFunction(self.parse_telos_function_def()?)),
            Some(Token::Entity) => Ok(Item::Entity(self.parse_entity_def()?)),
            Some(Token::IntentCoordinator) => Ok(Item::IntentCoordinator(self.parse_intent_coordinator_def()?)),
            Some(Token::Discipline) => Ok(Item::Discipline(self.parse_discipline_decl()?)),
            Some(Token::ChainKw) => Ok(Item::Chain(self.parse_chain_def()?)),
            Some(Token::DagKw) => Ok(Item::Dag(self.parse_dag_def()?)),
            Some(Token::Const) => Ok(Item::Const(self.parse_const_def()?)),
            Some(Token::PipelineKw) => Ok(Item::Pipeline(self.parse_pipeline_def()?)),
            Some(Token::SagaKw) => Ok(Item::Saga(self.parse_saga_def()?)),
            Some(Token::EventKw) => Ok(Item::Event(self.parse_event_def()?)),
            Some(Token::CommandKw) => Ok(Item::Command(self.parse_command_def()?)),
            Some(Token::QueryKw) => Ok(Item::Query(self.parse_query_def()?)),
            Some(Token::CircuitBreakerKw) => Ok(Item::CircuitBreaker(self.parse_circuit_breaker_def()?)),
            Some(Token::RetryKw) => Ok(Item::Retry(self.parse_retry_def()?)),
            Some(Token::RateLimiterKw) => Ok(Item::RateLimiter(self.parse_rate_limiter_def()?)),
            Some(tok) => Err(LoomError::parse(
                format!("unexpected token at item level: {:?}", tok),
                self.current_span(),
            )),
            None => Err(LoomError::parse(
                "unexpected end of input inside module",
                Span::synthetic(),
            )),
        }
    }

    /// Parse inline `{ field: Type [@ann], ... }` field list.
    /// Supports pre-field annotations: `{ @provenance field: Type, ... }`.
    /// Field names may be keywords used as contextual identifiers (e.g. `type`, `action`).
    fn parse_inline_fields(&mut self) -> Result<Vec<FieldDef>, LoomError> {
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(&Token::RBrace) && self.peek().is_some() {
            let field_start = self.current_span();
            // Collect any pre-field annotations (e.g. @provenance, @weight, @distance)
            let pre_annotations = self.parse_annotations();
            // Field name may be a keyword used contextually (e.g. `type`, `action`).
            let field_name = if let Some(name) = self.token_as_ident() {
                self.advance();
                name
            } else {
                break;
            };
            if !self.at(&Token::Colon) {
                break;
            }
            self.advance();
            let ty = self.parse_type_expr()?;
            let mut annotations = pre_annotations;
            annotations.extend(self.parse_annotations());
            let field_end = self.current_span();
            fields.push(FieldDef {
                name: field_name,
                ty,
                annotations,
                span: Span::merge(&field_start, &field_end),
            });
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(fields)
    }

    /// Parse a `messaging_primitive Name ... end` declaration.
    ///
    /// Grammar:
    /// ```text
    /// messaging_primitive SyncRequest
    ///   pattern: request_response
    ///   guarantees: @exactly-once
    ///   timeout: mandatory
    /// end
    /// ```
    fn parse_messaging_primitive(&mut self) -> Result<MessagingPrimitiveDef, LoomError> {
        let start = self.current_span();
        self.advance(); // consume `messaging_primitive`
        let (name, _) = self.expect_ident()?;
        let mut pattern = None;
        let mut guarantees = Vec::new();
        let mut timeout_mandatory = false;

        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "pattern") {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::Ident(p), _)) = self.tokens.get(self.pos) {
                    let parsed = match p.as_str() {
                        "request_response" => Some(MessagingPattern::RequestResponse),
                        "publish_subscribe" => Some(MessagingPattern::PublishSubscribe),
                        "point_to_point" => Some(MessagingPattern::PointToPoint),
                        "producer_consumer" => Some(MessagingPattern::ProducerConsumer),
                        "bidirectional" => Some(MessagingPattern::Bidirectional),
                        "stream" => Some(MessagingPattern::Stream),
                        _ => None, // unknown pattern names are ignored rather than defaulted
                    };
                    if parsed.is_some() { pattern = parsed; }
                    self.pos += 1;
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Guarantees, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                // Collect guarantee tokens until next known field keyword
                while !self.at(&Token::End) && self.peek().is_some() {
                    let at_field = matches!(self.tokens.get(self.pos),
                        Some((Token::Ident(n), _)) if matches!(n.as_str(), "pattern" | "timeout" | "schema" | "ordering"))
                        || matches!(self.tokens.get(self.pos), Some((Token::Guarantees, _)));
                    if at_field {
                        break;
                    }
                    if let Some((tok, _)) = self.tokens.get(self.pos) {
                        let s = format!("{:?}", tok);
                        if !s.is_empty() {
                            guarantees.push(s);
                        }
                        self.pos += 1;
                    }
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "timeout")
            {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::Ident(v), _)) = self.tokens.get(self.pos) {
                    timeout_mandatory = v == "mandatory";
                    self.pos += 1;
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(MessagingPrimitiveDef {
            name,
            pattern,
            guarantees,
            timeout_mandatory,
            span: Span::merge(&start, &end_span),
        })
    }

    pub(in crate::parser) fn parse_telos_function_def(&mut self) -> Result<TelosFunctionDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::TelosFunction)?;
        let (name, _) = self.expect_ident()?;
        let mut statement = None;
        let mut bounded_by = None;
        let mut measured_by = None;
        let mut thresholds: Option<TelosThresholds> = None;
        let mut guides = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(k) = self.token_as_ident() {
                match k.as_str() {
                    "statement" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                            statement = Some(s.clone());
                            self.pos += 1;
                        }
                    }
                    "bounded_by" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some(n) = self.token_as_ident() { self.advance(); bounded_by = Some(n); }
                    }
                    "measured_by" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                            measured_by = Some(s.clone());
                            self.pos += 1;
                        } else if let Some(n) = self.token_as_ident() {
                            self.advance();
                            measured_by = Some(n);
                        }
                    }
                    "guides" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if self.at(&Token::LBracket) {
                            self.advance();
                            while !self.at(&Token::RBracket) && self.peek().is_some() {
                                if let Some(n) = self.token_as_ident() { self.advance(); guides.push(n); }
                                else { self.advance(); }
                                if self.at(&Token::Comma) { self.advance(); }
                            }
                            if self.at(&Token::RBracket) { self.advance(); }
                        }
                    }
                    "thresholds" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        let mut convergence = 0.8_f64;
                        let mut divergence = 0.2_f64;
                        let mut warning = None;
                        let mut propagation = None;
                        for _ in 0..20 {
                            match self.tokens.get(self.pos) {
                                Some((Token::Convergence, _)) => {
                                    self.advance();
                                    if self.at(&Token::Colon) { self.advance(); }
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        convergence = *f; self.pos += 1;
                                    }
                                }
                                Some((Token::Divergence, _)) => {
                                    self.advance();
                                    if self.at(&Token::Colon) { self.advance(); }
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        divergence = *f; self.pos += 1;
                                    }
                                }
                                Some((Token::Ident(k), _)) if k == "warning" => {
                                    self.advance();
                                    if self.at(&Token::Colon) { self.advance(); }
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        warning = Some(*f); self.pos += 1;
                                    }
                                }
                                Some((Token::Propagation, _)) => {
                                    self.advance();
                                    if self.at(&Token::Colon) { self.advance(); }
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        propagation = Some(*f); self.pos += 1;
                                    }
                                }
                                Some((Token::End, _)) => break,
                                Some((Token::Ident(_), _)) => break,
                                _ => { self.advance(); }
                            }
                        }
                        thresholds = Some(TelosThresholds { convergence, divergence, warning, propagation });
                        // Consume the optional `end` that closes an explicit `thresholds:` sub-block.
                        if self.at(&Token::End) { self.advance(); }
                    }
                    _ => { self.advance(); }
                }
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        if self.at(&Token::End) { self.advance(); }
        Ok(TelosFunctionDef { name, statement, bounded_by, measured_by, thresholds, guides, span: Span::merge(&start, &end_span) })
    }

    pub(in crate::parser) fn parse_entity_def(&mut self) -> Result<EntityDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Entity)?;
        let (name, _) = self.expect_ident()?;
        // Optional <NodeType, EdgeType, ...> generic params
        let mut node_type = None;
        let mut edge_type = None;
        if self.at(&Token::Lt) {
            self.advance();
            if let Some(n) = self.token_as_ident() { self.advance(); node_type = Some(n); }
            if self.at(&Token::Comma) {
                self.advance();
                if let Some(e) = self.token_as_ident() { self.advance(); edge_type = Some(e); }
            }
            // Skip any additional type params
            while !self.at(&Token::Gt) && self.peek().is_some() {
                self.advance();
            }
            if self.at(&Token::Gt) { self.advance(); }
        }
        let describe = self.parse_describe();
        // Parse @annotations — collect as string names
        let raw_annotations = self.parse_annotations();
        let annotations: Vec<String> = raw_annotations.into_iter().map(|a| a.key).collect();
        let mut alias_of = None;
        // Optional body with `alias_of:` etc. (parse until end or no end)
        if self.at(&Token::End) {
            self.advance();
        } else {
            while !self.at(&Token::End) && self.peek().is_some() {
                if let Some((Token::Ident(k), _)) = self.tokens.get(self.pos) {
                    let k = k.clone();
                    if k == "alias_of" {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some(n) = self.token_as_ident() { self.advance(); alias_of = Some(n); }
                    } else {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            if self.at(&Token::End) { self.advance(); }
        }
        let end_span = self.current_span();
        Ok(EntityDef { name, node_type, edge_type, annotations, describe, alias_of, span: Span::merge(&start, &end_span) })
    }

    pub(in crate::parser) fn parse_intent_coordinator_def(&mut self) -> Result<IntentCoordinatorDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::IntentCoordinator)?;
        let (name, _) = self.expect_ident()?;
        let mut telomere_days = None;
        let mut governance_class = GovernanceClass::AiProposes;
        let mut signals = Vec::new();
        let mut rollback_on = None;
        let mut min_confidence = None;
        let mut audit_path = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(k) = self.token_as_ident() {
                match k.as_str() {
                    "telomere" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::FloatLit(n), _)) = self.tokens.get(self.pos) {
                            telomere_days = Some(*n as u64);
                            self.pos += 1;
                        } else if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                            telomere_days = Some(*n as u64);
                            self.pos += 1;
                        }
                        // consume dot + optional unit (days/months)
                        if self.at(&Token::Dot) { self.advance(); }
                        if let Some((Token::Ident(unit), _)) = self.tokens.get(self.pos) {
                            if unit == "days" || unit == "months" { self.pos += 1; }
                        }
                    }
                    "governance" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::Ident(g), _)) = self.tokens.get(self.pos) {
                            governance_class = match g.as_str() {
                                "automatic" => GovernanceClass::Automatic,
                                "ai_proposes" => GovernanceClass::AiProposes,
                                "human_only" => GovernanceClass::HumanOnly,
                                "blocked" => GovernanceClass::Blocked,
                                _ => GovernanceClass::AiProposes,
                            };
                            self.pos += 1;
                        }
                    }
                    "signals" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if self.at(&Token::LBracket) {
                            self.advance();
                            while !self.at(&Token::RBracket) && self.peek().is_some() {
                                if let Some(n) = self.token_as_ident() {
                                    self.advance();
                                    signals.push(IntentSignalSource { name: n, trust_level: None });
                                } else { self.advance(); }
                                if self.at(&Token::Comma) { self.advance(); }
                            }
                            if self.at(&Token::RBracket) { self.advance(); }
                        }
                    }
                    "rollback_on" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                            rollback_on = Some(s.clone());
                            self.pos += 1;
                        }
                    }
                    "min_confidence" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                            min_confidence = Some(*f);
                            self.pos += 1;
                        }
                    }
                    "audit_path" => {
                        self.advance();
                        if self.at(&Token::Colon) { self.advance(); }
                        if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                            audit_path = Some(s.clone());
                            self.pos += 1;
                        }
                    }
                    _ => { self.advance(); }
                }
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        if self.at(&Token::End) { self.advance(); }
        Ok(IntentCoordinatorDef { name, telomere_days, governance_class, signals, rollback_on, min_confidence, audit_path, span: Span::merge(&start, &end_span) })
    }

    /// M141–M145: Parse a `discipline Kind for Target [params...] end` declaration.
    ///
    /// Syntax examples:
    /// ```loom
    /// discipline CQRS for OrderStore end
    /// discipline EventSourcing for OrderStore events: [OrderCreated, OrderShipped] end
    /// discipline DependencyInjection for PricingEngine binds: [IPriceRepo, IRiskCalc] end
    /// discipline CircuitBreaker for PaymentService max_attempts: 3 timeout_ms: 500 end
    /// discipline Saga for CheckoutFlow steps: [ValidateOrder, ProcessPayment] end
    /// discipline UnitOfWork for Checkout tables: [Orders, LineItems] end
    /// ```
    pub(in crate::parser) fn parse_discipline_decl(&mut self) -> Result<DisciplineDecl, LoomError> {
        let start = self.current_span();
        self.expect(Token::Discipline)?;

        // Kind name — must be an ident
        let (kind_str, _) = self.expect_ident()?;
        let kind = match kind_str.to_lowercase().as_str() {
            "cqrs" => DisciplineKind::Cqrs,
            "eventsourcing" | "event_sourcing" => DisciplineKind::EventSourcing,
            "dependencyinjection" | "dependency_injection" | "di" => DisciplineKind::DependencyInjection,
            "circuitbreaker" | "circuit_breaker" => DisciplineKind::CircuitBreaker,
            "saga" => DisciplineKind::Saga,
            "unitofwork" | "unit_of_work" => DisciplineKind::UnitOfWork,
            other => return Err(LoomError::parse(
                format!(
                    "unknown discipline kind '{}' — expected one of: CQRS, EventSourcing, \
                     DependencyInjection, CircuitBreaker, Saga, UnitOfWork",
                    other
                ),
                start,
            )),
        };

        // `for` keyword — accepts both Token::For and a "for" ident
        if self.at(&Token::For) {
            self.advance();
        } else if let Some(k) = self.token_as_ident() {
            if k == "for" { self.advance(); }
        }

        // Target name
        let (target, _) = self.expect_ident()?;

        // Keyword params until `end`
        let mut params: Vec<(String, DisciplineParam)> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                self.advance();
                if self.at(&Token::Colon) { self.advance(); }

                // List param: `key: [A, B, C]`
                if self.at(&Token::LBracket) {
                    self.advance();
                    let mut items = Vec::new();
                    while !self.at(&Token::RBracket) && self.peek().is_some() {
                        if let Some(v) = self.token_as_ident() {
                            self.advance();
                            items.push(v);
                        } else {
                            self.advance();
                        }
                        if self.at(&Token::Comma) { self.advance(); }
                    }
                    if self.at(&Token::RBracket) { self.advance(); }
                    params.push((key, DisciplineParam::List(items)));
                }
                // Numeric param: `key: 42`
                else if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                    let n = *n;
                    self.pos += 1;
                    params.push((key, DisciplineParam::Number(n)));
                }
                // Scalar/string param: `key: SomeValue` or `key: "string"`
                else if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    let s = s.clone();
                    self.pos += 1;
                    params.push((key, DisciplineParam::Scalar(s)));
                } else if let Some(v) = self.token_as_ident() {
                    self.advance();
                    params.push((key, DisciplineParam::Scalar(v)));
                }
                // else: param with no value — skip
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        if self.at(&Token::End) { self.advance(); }
        Ok(DisciplineDecl { kind, target, params, span: Span::merge(&start, &end_span) })
    }

    /// Parse `chain Name ... end` — M155 Markov chain item.
    ///
    /// ```loom
    /// chain Weather
    ///   states: [Sunny, Cloudy, Rainy]
    ///   transitions:
    ///     Sunny -> Cloudy: 0.3
    ///     Sunny -> Rainy: 0.1
    ///   end
    /// end
    /// ```
    pub(in crate::parser) fn parse_chain_def(&mut self) -> Result<ChainDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::ChainKw)?;
        let (name, _) = self.expect_ident()?;
        let mut states: Vec<String> = Vec::new();
        let mut transitions: Vec<(String, String, f64)> = Vec::new();

        // Parse optional `states: [A, B, C]`
        if self.at_keyword("states") {
            self.advance(); // consume `states`
            self.expect(Token::Colon)?;
            self.expect(Token::LBracket)?;
            while !self.at(&Token::RBracket) && self.peek().is_some() {
                if let Some(state) = self.token_as_ident() {
                    self.advance();
                    states.push(state);
                } else {
                    break;
                }
                if self.at(&Token::Comma) { self.advance(); }
            }
            self.expect(Token::RBracket)?;
        }

        // Parse optional `transitions: ... end`
        if self.at(&Token::Transitions) {
            self.advance(); // consume `transitions`
            if self.at(&Token::Colon) { self.advance(); }
            // Parse `FromState -> ToState: 0.3` lines until `end`
            while !self.at(&Token::End) && self.peek().is_some() {
                let from = match self.token_as_ident() {
                    Some(s) => { self.advance(); s }
                    None => break,
                };
                self.expect(Token::Arrow)?;
                let to = match self.token_as_ident() {
                    Some(s) => { self.advance(); s }
                    None => break,
                };
                self.expect(Token::Colon)?;
                let prob = self.parse_float_literal().unwrap_or(0.0);
                transitions.push((from, to, prob));
            }
            if self.at(&Token::End) { self.advance(); } // consume inner `end`
        }

        let end_span = self.current_span();
        if self.at(&Token::End) { self.advance(); } // consume outer `end`
        Ok(ChainDef { name, states, transitions, span: Span::merge(&start, &end_span) })
    }

    /// Parse a floating-point literal at the current position.
    fn parse_float_literal(&mut self) -> Option<f64> {
        match self.tokens.get(self.pos) {
            Some((Token::FloatLit(f), _)) => { let v = *f; self.pos += 1; Some(v) }
            Some((Token::IntLit(i), _)) => { let v = *i as f64; self.pos += 1; Some(v) }
            _ => None,
        }
    }

    /// Returns true if the current token is an identifier with the given text.
    fn at_keyword(&self, kw: &str) -> bool {
        match self.tokens.get(self.pos) {
            Some((Token::Ident(s), _)) => s == kw,
            Some((Token::States, _)) => kw == "states",
            Some((Token::Nodes, _)) => kw == "nodes",
            Some((Token::Edges, _)) => kw == "edges",
            _ => false,
        }
    }

    /// Parse `dag Name nodes: [A, B, C] edges: [A -> B, B -> C] end` — M156 DAG item.
    ///
    /// Both `nodes:` and `edges:` sections are optional.
    pub(in crate::parser) fn parse_dag_def(&mut self) -> Result<DagDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::DagKw)?;
        let name = match self.token_as_ident() {
            Some(n) => { self.advance(); n }
            None => return Err(LoomError::parse("expected dag name", self.current_span())),
        };

        let mut nodes: Vec<String> = Vec::new();
        let mut edges: Vec<(String, String)> = Vec::new();

        // Parse optional `nodes: [A, B, C]`
        if self.at_keyword("nodes") {
            self.advance();
            if self.at(&Token::Colon) { self.advance(); }
            self.expect(Token::LBracket)?;
            while !self.at(&Token::RBracket) && self.peek().is_some() {
                if let Some(n) = self.token_as_ident() {
                    self.advance();
                    nodes.push(n);
                }
                if self.at(&Token::Comma) { self.advance(); }
            }
            self.expect(Token::RBracket)?;
        }

        // Parse optional `edges: [A -> B, B -> C]`
        if self.at_keyword("edges") {
            self.advance();
            if self.at(&Token::Colon) { self.advance(); }
            self.expect(Token::LBracket)?;
            while !self.at(&Token::RBracket) && self.peek().is_some() {
                let from = match self.token_as_ident() {
                    Some(s) => { self.advance(); s }
                    None => break,
                };
                self.expect(Token::Arrow)?;
                let to = match self.token_as_ident() {
                    Some(s) => { self.advance(); s }
                    None => break,
                };
                edges.push((from, to));
                if self.at(&Token::Comma) { self.advance(); }
            }
            self.expect(Token::RBracket)?;
        }

        let end_span = self.current_span();
        if self.at(&Token::End) { self.advance(); }
        Ok(DagDef { name, nodes, edges, span: Span::merge(&start, &end_span) })
    }

    /// Parse `const Name: Type = value` — M157 constant item.
    ///
    /// The value may be an integer literal, float literal, bool literal, or
    /// a quoted string literal. No expression parsing — constants are simple literals.
    pub(in crate::parser) fn parse_const_def(&mut self) -> Result<ConstDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Const)?;
        let name = match self.token_as_ident() {
            Some(n) => { self.advance(); n }
            None => return Err(LoomError::parse("expected constant name", self.current_span())),
        };

        // Optional `:`
        let ty = if self.at(&Token::Colon) {
            self.advance();
            match self.token_as_ident() {
                Some(t) => { self.advance(); t }
                None => return Err(LoomError::parse("expected type after ':'", self.current_span())),
            }
        } else {
            String::new()
        };

        self.expect(Token::Eq)?;

        // Collect the value: int, float, bool, or string literal
        let value = match self.tokens.get(self.pos) {
            Some((Token::IntLit(n), _)) => { let v = n.to_string(); self.advance(); v }
            Some((Token::FloatLit(f), _)) => { let v = f.to_string(); self.advance(); v }
            Some((Token::BoolLit(b), _)) => { let v = b.to_string(); self.advance(); v }
            Some((Token::StrLit(s), _)) => { let v = format!("\"{}\"", s.clone()); self.advance(); v }
            _ => {
                // Fallback: collect until newline or End token
                match self.token_as_ident() {
                    Some(s) => { self.advance(); s }
                    None => return Err(LoomError::parse(
                        "expected literal value for const", self.current_span(),
                    )),
                }
            }
        };

        let end_span = self.current_span();
        Ok(ConstDef { name, ty, value, span: Span::merge(&start, &end_span) })
    }

    // ── M159: pipeline item ────────────────────────────────────────────────────

    /// Parse a `pipeline Name step name :: In -> Out ... end` declaration.
    ///
    /// Each step is a named transformation with a `::` arrow type signature:
    /// `step normalize :: String -> String`
    ///
    /// Produces a [`PipelineDef`] with ordered steps.
    fn parse_pipeline_def(&mut self) -> Result<PipelineDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::PipelineKw)?;
        let (name, _) = self.expect_ident()?;

        let mut steps = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            let step_start = self.current_span();
            self.expect(Token::Step)?;
            let (step_name, _) = self.expect_ident()?;
            self.expect(Token::ColonColon)?;

            // Parse `InputType -> OutputType`
            let (input_ty, _) = self.expect_ident()?;
            self.expect(Token::Arrow)?;
            let (output_ty, _) = self.expect_ident()?;

            let step_end = self.current_span();
            steps.push(PipelineStep {
                name: step_name,
                input_ty,
                output_ty,
                span: Span::merge(&step_start, &step_end),
            });
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(PipelineDef { name, steps, span: Span::merge(&start, &end_span) })
    }

    // ── M160: saga item ───────────────────────────────────────────────────────

    /// Parse a `saga Name step a :: In -> Out [compensate a :: In -> Out] ... end`.
    ///
    /// Steps and compensating transactions are declared in order. A `compensate`
    /// clause must follow the `step` it compensates. Steps without a compensate
    /// clause are non-compensable (all-or-nothing semantics for that step).
    ///
    /// Produces a [`SagaDef`] with ordered [`SagaStep`]s, each optionally bearing
    /// a [`SagaCompensate`].
    fn parse_saga_def(&mut self) -> Result<SagaDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::SagaKw)?;
        let (name, _) = self.expect_ident()?;

        let mut steps: Vec<SagaStep> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Step) {
                let step_start = self.current_span();
                self.expect(Token::Step)?;
                let (step_name, _) = self.expect_ident()?;
                self.expect(Token::ColonColon)?;
                let (input_ty, _) = self.expect_ident()?;
                self.expect(Token::Arrow)?;
                let (output_ty, _) = self.expect_ident()?;
                let step_end = self.current_span();

                // Check for optional compensate on the *next* token
                let compensate = if self.at(&Token::Compensate) {
                    let comp_start = self.current_span();
                    self.expect(Token::Compensate)?;
                    let (comp_step_name, _) = self.expect_ident()?;
                    self.expect(Token::ColonColon)?;
                    let (comp_in, _) = self.expect_ident()?;
                    self.expect(Token::Arrow)?;
                    let (comp_out, _) = self.expect_ident()?;
                    let comp_end = self.current_span();
                    Some(SagaCompensate {
                        step_name: comp_step_name,
                        input_ty: comp_in,
                        output_ty: comp_out,
                        span: Span::merge(&comp_start, &comp_end),
                    })
                } else {
                    None
                };

                steps.push(SagaStep {
                    name: step_name,
                    input_ty,
                    output_ty,
                    compensate,
                    span: Span::merge(&step_start, &step_end),
                });
            } else {
                // Unexpected token inside saga — advance to avoid infinite loop
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(SagaDef { name, steps, span: Span::merge(&start, &end_span) })
    }

    /// Parse `event Name field: Type ... end`.
    ///
    /// Each line inside the block is `field_name: TypeName`.
    /// Unknown tokens are skipped to tolerate annotations or whitespace.
    pub(crate) fn parse_event_def(&mut self) -> Result<EventDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::EventKw)?;
        let (name, _) = self.expect_ident()?;
        let mut fields: Vec<(String, String)> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            // field_name: TypeName
            if let Some(field_name) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // field name
                    self.advance(); // colon
                    let (ty, _) = self.expect_ident()?;
                    fields.push((field_name, ty));
                    continue;
                }
            }
            self.advance();
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(EventDef { name, fields, span: Span::merge(&start, &end_span) })
    }

    /// Parse `command Name field: Type ... end`.
    pub(crate) fn parse_command_def(&mut self) -> Result<CommandDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::CommandKw)?;
        let (name, _) = self.expect_ident()?;
        let mut fields: Vec<(String, String)> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(field_name) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance();
                    self.advance();
                    let (ty, _) = self.expect_ident()?;
                    fields.push((field_name, ty));
                    continue;
                }
            }
            self.advance();
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CommandDef { name, fields, span: Span::merge(&start, &end_span) })
    }

    /// Parse `query Name field: Type ... end`.
    pub(crate) fn parse_query_def(&mut self) -> Result<QueryDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::QueryKw)?;
        let (name, _) = self.expect_ident()?;
        let mut fields: Vec<(String, String)> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(field_name) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance();
                    self.advance();
                    let (ty, _) = self.expect_ident()?;
                    fields.push((field_name, ty));
                    continue;
                }
            }
            self.advance();
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(QueryDef { name, fields, span: Span::merge(&start, &end_span) })
    }

    /// Parse `circuit_breaker Name threshold: N timeout: N fallback: name end`.
    ///
    /// All three keys are optional — defaults: threshold=5, timeout=30, fallback="".
    pub(crate) fn parse_circuit_breaker_def(&mut self) -> Result<CircuitBreakerDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::CircuitBreakerKw)?;
        let (name, _) = self.expect_ident()?;
        let mut threshold: u32 = 5;
        let mut timeout: u64 = 30;
        let mut fallback = String::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "threshold" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                threshold = *n as u32;
                                self.advance();
                            }
                        }
                        "timeout" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                timeout = *n as u64;
                                self.advance();
                            }
                        }
                        "fallback" => {
                            if let Some(val) = self.token_as_ident() {
                                fallback = val;
                                self.advance();
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
            }
            self.advance();
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CircuitBreakerDef { name, threshold, timeout, fallback, span: Span::merge(&start, &end_span) })
    }

    /// Parse `retry Name max_attempts: N base_delay: N multiplier: N on: ErrorType end`.
    ///
    /// All keys optional — defaults: max_attempts=3, base_delay=100, multiplier=2, on="".
    pub(crate) fn parse_retry_def(&mut self) -> Result<RetryDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::RetryKw)?;
        let (name, _) = self.expect_ident()?;
        let mut max_attempts: u32 = 3;
        let mut base_delay: u64 = 100;
        let mut multiplier: u32 = 2;
        let mut on_error = String::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "max_attempts" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                max_attempts = *n as u32;
                                self.advance();
                            }
                        }
                        "base_delay" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                base_delay = *n as u64;
                                self.advance();
                            }
                        }
                        "multiplier" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                multiplier = *n as u32;
                                self.advance();
                            }
                        }
                        "on" => {
                            if let Some(val) = self.token_as_ident() {
                                on_error = val;
                                self.advance();
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
            }
            self.advance();
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(RetryDef { name, max_attempts, base_delay, multiplier, on_error, span: Span::merge(&start, &end_span) })
    }

    /// M165: Parse `rate_limiter Name [requests: N] [per: N] [burst: N] end`
    ///
    /// Implements token bucket rate limiting (Anderson 1990).
    /// All configuration keys are optional — defaults: requests=100, per=60, burst=0.
    fn parse_rate_limiter_def(&mut self) -> Result<RateLimiterDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::RateLimiterKw)?;
        let (name, _) = self.expect_ident()?;
        let mut requests: u64 = 100;
        let mut per: u64 = 60;
        let mut burst: u64 = 0;

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                self.advance();
                if self.at(&Token::Colon) {
                    self.advance();
                    match key.as_str() {
                        "requests" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                requests = *n as u64;
                                self.advance();
                            }
                        }
                        "per" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                per = *n as u64;
                                self.advance();
                            }
                        }
                        "burst" => {
                            if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                burst = *n as u64;
                                self.advance();
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
            }
            self.advance();
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(RateLimiterDef { name, requests, per, burst, span: Span::merge(&start, &end_span) })
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Result<Module, LoomError> {
        let tokens = Lexer::tokenize(src).map_err(|e| e.into_iter().next().unwrap())?;
        Parser::new(&tokens).parse_module()
    }

    #[test]
    fn parses_empty_module() {
        let m = parse("module Foo end").expect("should parse");
        assert_eq!(m.name, "Foo");
        assert!(m.items.is_empty());
    }

    #[test]
    fn parses_type_def() {
        let m = parse("module M type Point = x: Int, y: Int end end").expect("should parse");
        assert_eq!(m.items.len(), 1);
        if let Item::Type(td) = &m.items[0] {
            assert_eq!(td.name, "Point");
            assert_eq!(td.fields.len(), 2);
        } else {
            panic!("expected TypeDef");
        }
    }

    #[test]
    fn parses_enum_def() {
        let m = parse("module M enum Color = | Red | Green | Blue end end").expect("should parse");
        if let Item::Enum(ed) = &m.items[0] {
            assert_eq!(ed.name, "Color");
            assert_eq!(ed.variants.len(), 3);
        } else {
            panic!("expected EnumDef");
        }
    }
}

/// Convert a token back to its source-level string representation.
/// Used when collecting type expression tokens as strings (e.g., signal payload).
fn token_to_source(tok: &Token) -> String {
    match tok {
        Token::Ident(s) => s.clone(),
        Token::StrLit(s) => format!("{:?}", s),
        Token::IntLit(n) => n.to_string(),
        Token::FloatLit(f) => f.to_string(),
        Token::BoolLit(b) => b.to_string(),
        Token::Lt => "<".to_string(),
        Token::Gt => ">".to_string(),
        Token::Eq => "=".to_string(),
        Token::Ge => ">=".to_string(),
        Token::Le => "<=".to_string(),
        Token::Ne => "!=".to_string(),
        Token::And => "and".to_string(),
        Token::Or => "or".to_string(),
        Token::Not => "not".to_string(),
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Slash => "/".to_string(),
        Token::Comma => ", ".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::Star => "*".to_string(),
        Token::Question => "?".to_string(),
        Token::Dot => ".".to_string(),
        _ => format!("{:?}", tok),
    }
}

/// If `tok` is a keyword that could serve as an identifier (e.g. a field name),
/// return its source spelling; otherwise return `None`.
fn token_keyword_str(tok: &Token) -> Option<&'static str> {
    match tok {
        Token::Threshold => Some("threshold"),
        Token::Limit => Some("limit"),
        Token::Produces => Some("produces"),
        Token::Modifies => Some("modifies"),
        Token::RevertsWhen => Some("reverts_when"),
        Token::OnExhaustion => Some("on_exhaustion"),
        Token::Signal => Some("signal"),
        Token::Payload => Some("payload"),
        Token::From => Some("from"),
        Token::To => Some("to"),
        Token::Toward => Some("toward"),
        Token::Bounds => Some("bounds"),
        Token::Members => Some("members"),
        Token::Fitness => Some("fitness"),
        Token::Telos => Some("telos"),
        Token::Form => Some("form"),
        Token::Matter => Some("matter"),
        Token::Regulate => Some("regulate"),
        Token::Evolve => Some("evolve"),
        Token::Degenerate => Some("degenerate"),
        Token::Fallback => Some("fallback"),
        Token::Checkpoint => Some("checkpoint"),
        Token::Canalize => Some("canalize"),
        Token::Pathway => Some("pathway"),
        Token::Senescence => Some("senescence"),
        Token::Store => Some("store"),
        Token::Table => Some("table"),
        Token::GraphNode => Some("node"),
        Token::Edge => Some("edge"),
        Token::Ttl => Some("ttl"),
        Token::Index => Some("index"),
        Token::Retention => Some("retention"),
        Token::Resolution => Some("resolution"),
        Token::Format => Some("format"),
        Token::Compression => Some("compression"),
        Token::Capacity => Some("capacity"),
        Token::Eviction => Some("eviction"),
        Token::Fact => Some("fact"),
        Token::Dimension => Some("dimension"),
        Token::Embedding => Some("embedding"),
        Token::Adopt => Some("adopt"),
        Token::Process => Some("process"),
        Token::Plasticity => Some("plasticity"),
        Token::Telomere => Some("telomere"),
        _ => None,
    }
}
