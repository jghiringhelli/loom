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
                    pattern = Some(match p.as_str() {
                        "request_response" => MessagingPattern::RequestResponse,
                        "publish_subscribe" => MessagingPattern::PublishSubscribe,
                        "point_to_point" => MessagingPattern::PointToPoint,
                        "producer_consumer" => MessagingPattern::ProducerConsumer,
                        "bidirectional" => MessagingPattern::Bidirectional,
                        _ => MessagingPattern::RequestResponse,
                    });
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
}

// ── Unit tests ────────────────────────────────────────────────────────────────

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
