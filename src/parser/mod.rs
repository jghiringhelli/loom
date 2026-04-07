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

impl<'src> Parser<'src> {
    /// Create a new parser for the given token slice.
    pub fn new(tokens: &'src [(Token, Span)]) -> Self {
        Parser { tokens, pos: 0, pending_effect_tiers: Vec::new(), pending_annotations: Vec::new() }
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
            None => return Err(LoomError::parse(
                "expected identifier, found end of input",
                Span::synthetic(),
            )),
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
                                    parts.push(token_to_source(
                                        &self.tokens[self.pos].0
                                    ));
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
            Some((Token::Store, _))       => Some("store".to_string()),
            Some((Token::Table, _))       => Some("table".to_string()),
            Some((Token::GraphNode, _))   => Some("node".to_string()),
            Some((Token::Edge, _))        => Some("edge".to_string()),
            Some((Token::Ttl, _))         => Some("ttl".to_string()),
            Some((Token::Index, _))       => Some("index".to_string()),
            Some((Token::Retention, _))   => Some("retention".to_string()),
            Some((Token::Resolution, _))  => Some("resolution".to_string()),
            Some((Token::Format, _))      => Some("format".to_string()),
            Some((Token::Compression, _)) => Some("compression".to_string()),
            Some((Token::Capacity, _))    => Some("capacity".to_string()),
            Some((Token::Eviction, _))    => Some("eviction".to_string()),
            Some((Token::Fact, _))        => Some("fact".to_string()),
            Some((Token::Dimension, _))   => Some("dimension".to_string()),
            Some((Token::Embedding, _))   => Some("embedding".to_string()),
            Some((Token::MapReduce, _))   => Some("mapreduce".to_string()),
            Some((Token::Consumer, _))    => Some("consumer".to_string()),
            Some((Token::Offset, _))      => Some("offset".to_string()),
            Some((Token::Partitions, _))  => Some("partitions".to_string()),
            Some((Token::Replication, _)) => Some("replication".to_string()),
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
            Some((Token::Store, _))       => Some("store".to_string()),
            Some((Token::Table, _))       => Some("table".to_string()),
            Some((Token::GraphNode, _))   => Some("node".to_string()),
            Some((Token::Edge, _))        => Some("edge".to_string()),
            Some((Token::Ttl, _))         => Some("ttl".to_string()),
            Some((Token::Index, _))       => Some("index".to_string()),
            Some((Token::Retention, _))   => Some("retention".to_string()),
            Some((Token::Resolution, _))  => Some("resolution".to_string()),
            Some((Token::Format, _))      => Some("format".to_string()),
            Some((Token::Compression, _)) => Some("compression".to_string()),
            Some((Token::Capacity, _))    => Some("capacity".to_string()),
            Some((Token::Eviction, _))    => Some("eviction".to_string()),
            Some((Token::Fact, _))        => Some("fact".to_string()),
            Some((Token::Dimension, _))   => Some("dimension".to_string()),
            Some((Token::Embedding, _))   => Some("embedding".to_string()),
            Some((Token::MapReduce, _))   => Some("mapreduce".to_string()),
            Some((Token::Consumer, _))    => Some("consumer".to_string()),
            Some((Token::Offset, _))      => Some("offset".to_string()),
            Some((Token::Partitions, _))  => Some("partitions".to_string()),
            Some((Token::Replication, _)) => Some("replication".to_string()),
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
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "symbiotic") {
                items.push(self.parse_symbiotic_import()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "niche_construction") {
                items.push(Item::NicheConstruction(self.parse_niche_construction()?));
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

    /// Parse `invariant NAME :: bool_expr`.
    fn parse_invariant(&mut self) -> Result<Invariant, LoomError> {
        let start = self.current_span();
        self.expect(Token::Invariant)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let condition = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(Invariant {
            name,
            condition,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `test NAME :: expr`.
    fn parse_test_def(&mut self) -> Result<TestDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Test)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let body = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(TestDef {
            name,
            body,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `interface NAME fn method :: sig ... end`.
    fn parse_interface_def(&mut self) -> Result<InterfaceDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Interface)?;
        let (name, _) = self.expect_ident()?;
        let mut methods = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Fn) {
                self.advance();
                let (method_name, _) = self.expect_ident()?;
                self.expect(Token::ColonColon)?;
                let sig = self.parse_fn_type_signature()?;
                methods.push((method_name, sig));
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(InterfaceDef {
            name,
            methods,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `lifecycle TypeName :: State1 -> State2 -> ...`.
    fn parse_lifecycle_def(&mut self) -> Result<LifecycleDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Lifecycle)?;
        let (type_name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let mut states = Vec::new();
        let (first, _) = self.expect_ident()?;
        states.push(first);
        while self.at(&Token::Arrow) {
            self.advance();
            let (state, _) = self.expect_ident()?;
            states.push(state);
        }
        // M69: Optional checkpoint body
        let mut checkpoints = Vec::new();
        if self.at(&Token::Checkpoint) {
            while !self.at(&Token::End) && self.peek().is_some() {
                if self.at(&Token::Checkpoint) {
                    let cp_start = self.current_span();
                    self.advance(); // consume `checkpoint`
                    self.expect(Token::Colon)?;
                    let (name, _) = self.expect_ident()?;
                    let mut requires_str = String::new();
                    let mut on_fail_str = String::new();
                    while !self.at(&Token::End) && self.peek().is_some() {
                        if let Some(key) = self.token_as_ident() {
                            if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                                self.advance(); // key
                                self.advance(); // colon
                                let (val, _) = self.expect_ident()?;
                                match key.as_str() {
                                    "requires" => requires_str = val,
                                    "on_fail" => on_fail_str = val,
                                    _ => {}
                                }
                                continue;
                            }
                        }
                        break;
                    }
                    let cp_end = self.current_span();
                    self.expect(Token::End)?;
                    checkpoints.push(CheckpointDef {
                        name,
                        requires: requires_str,
                        on_fail: on_fail_str,
                        span: Span::merge(&cp_start, &cp_end),
                    });
                } else {
                    self.advance();
                }
            }
            let end_span = self.current_span();
            self.expect(Token::End)?;
            return Ok(LifecycleDef {
                type_name,
                states,
                checkpoints,
                span: Span::merge(&start, &end_span),
            });
        }
        let end_span = self.current_span();
        Ok(LifecycleDef {
            type_name,
            states,
            checkpoints,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a temporal property block:
    /// ```loom
    /// temporal Name
    ///   always: <predicate>
    ///   eventually: <Type> reaches <State>
    ///   never: <State> transitions to <State>
    ///   precedes: <State> before <State>
    /// end
    /// ```
    fn parse_temporal_def(&mut self) -> Result<TemporalDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Temporal)?;
        let (name, _) = self.expect_ident()?;

        let mut properties = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            let prop_span = self.current_span();
            if self.at(&Token::Always) {
                self.advance();
                self.expect(Token::Colon)?;
                let predicate = self.parse_expr()?;
                properties.push(TemporalProperty::Always {
                    predicate,
                    span: Span::merge(&prop_span, &self.current_span()),
                });
            } else if self.at(&Token::Eventually) {
                self.advance();
                self.expect(Token::Colon)?;
                let (type_name, _) = self.expect_ident()?;
                self.expect(Token::Reaches)?;
                let (target_state, _) = self.expect_ident()?;
                properties.push(TemporalProperty::Eventually {
                    type_name,
                    target_state,
                    span: Span::merge(&prop_span, &self.current_span()),
                });
            } else if self.at(&Token::Never) {
                self.advance();
                self.expect(Token::Colon)?;
                let (from_state, _) = self.expect_ident()?;
                self.expect(Token::Transitions)?;
                self.expect(Token::To)?;
                let (to_state, _) = self.expect_ident()?;
                properties.push(TemporalProperty::Never {
                    from_state,
                    to_state,
                    span: Span::merge(&prop_span, &self.current_span()),
                });
            } else if self.at(&Token::Precedes) {
                self.advance();
                self.expect(Token::Colon)?;
                let (first, _) = self.expect_ident()?;
                self.expect(Token::Before)?;
                let (second, _) = self.expect_ident()?;
                properties.push(TemporalProperty::Precedes {
                    first,
                    second,
                    span: Span::merge(&prop_span, &self.current_span()),
                });
            } else {
                return Err(LoomError::parse(
                    format!(
                        "expected temporal property (always, eventually, never, precedes), got {:?}",
                        self.peek()
                    ),
                    self.current_span(),
                ));
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(TemporalDef {
            name,
            properties,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `separation:` block inside a function body.
    ///
    /// Syntax:
    /// ```text
    /// separation:
    ///   owns: resource_name
    ///   disjoint: A * B
    ///   frame: preserved_field
    ///   proof: assertion_name
    /// end
    /// ```
    fn parse_separation_block(&mut self) -> Result<SeparationBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Separation)?;
        self.expect(Token::Colon)?;

        let mut owns: Vec<String> = Vec::new();
        let mut disjoint: Vec<(String, String)> = Vec::new();
        let mut frame: Vec<String> = Vec::new();
        let mut proof: Option<String> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Owns) {
                self.advance();
                self.expect(Token::Colon)?;
                let (name, _) = self.expect_ident()?;
                owns.push(name);
            } else if self.at(&Token::Disjoint) {
                self.advance();
                self.expect(Token::Colon)?;
                let (left, _) = self.expect_ident()?;
                self.expect(Token::Star)?;
                let (right, _) = self.expect_ident()?;
                disjoint.push((left, right));
            } else if self.at(&Token::Frame) {
                self.advance();
                self.expect(Token::Colon)?;
                let (name, _) = self.expect_ident()?;
                frame.push(name);
            } else if self.at(&Token::Proof) {
                self.advance();
                self.expect(Token::Colon)?;
                let (assertion, _) = self.expect_ident()?;
                proof = Some(assertion);
            } else {
                return Err(LoomError::parse(
                    format!(
                        "expected separation clause (owns, disjoint, frame, proof), got {:?}",
                        self.peek()
                    ),
                    self.current_span(),
                ));
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(SeparationBlock {
            owns,
            disjoint,
            frame,
            proof,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `gradual:` block.
    fn parse_gradual_block(&mut self) -> Result<GradualBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Gradual)?;
        self.expect(Token::Colon)?;
        let mut input_type: Option<String> = None;
        let mut boundary: Option<String> = None;
        let mut output_type: Option<String> = None;
        let mut on_cast_failure: Option<String> = None;
        let mut blame: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((tok, _)) = self.tokens.get(self.pos + 1) {
                    if matches!(tok, crate::lexer::Token::Colon) {
                        self.advance(); // key
                        self.advance(); // colon
                        let val = self.parse_value_as_string()?;
                        match key.as_str() {
                            "input_type"      => input_type = Some(val),
                            "boundary"        => boundary = Some(val),
                            "output_type"     => output_type = Some(val),
                            "on_cast_failure" => on_cast_failure = Some(val),
                            "blame"           => blame = Some(val),
                            _ => {}
                        }
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(GradualBlock { input_type, boundary, output_type, on_cast_failure, blame, span: Span::merge(&start, &end_span) })
    }

    /// Parse `distribution:` block.
    fn parse_distribution_block(&mut self) -> Result<DistributionBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Distribution)?;
        self.expect(Token::Colon)?;
        let mut model = String::new();
        let mut mean: Option<String> = None;
        let mut variance: Option<String> = None;
        let mut bounds: Option<String> = None;
        let mut convergence: Option<String> = None;
        let mut stability: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((tok, _)) = self.tokens.get(self.pos + 1) {
                    if matches!(tok, crate::lexer::Token::Colon) {
                        self.advance(); // key
                        self.advance(); // colon
                        let val = self.parse_value_as_string()?;
                        match key.as_str() {
                            "model"       => model = val,
                            "mean"        => mean = Some(val),
                            "variance"    => variance = Some(val),
                            "bounds"      => bounds = Some(val),
                            "convergence" => convergence = Some(val),
                            "stability"   => stability = Some(val),
                            _ => {}
                        }
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(DistributionBlock { model, mean, variance, bounds, convergence, stability, span: Span::merge(&start, &end_span) })
    }

    /// Parse `timing_safety:` block.
    fn parse_timing_safety_block(&mut self) -> Result<TimingSafetyBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::TimingSafety)?;
        self.expect(Token::Colon)?;
        let mut constant_time = false;
        let mut leaks_bits: Option<String> = None;
        let mut method: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((tok, _)) = self.tokens.get(self.pos + 1) {
                    if matches!(tok, crate::lexer::Token::Colon) {
                        self.advance(); // key
                        self.advance(); // colon
                        let val = self.parse_value_as_string()?;
                        match key.as_str() {
                            "constant_time" => constant_time = val == "true",
                            "leaks_bits"    => leaks_bits = Some(val),
                            "method"        => method = Some(val),
                            _ => {}
                        }
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(TimingSafetyBlock { constant_time, leaks_bits, method, span: Span::merge(&start, &end_span) })
    }

    /// Parse `proposition NAME = TypeExpr [where expr]`.
    fn parse_proposition_def(&mut self) -> Result<PropositionDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Proposition)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;
        let base_type = self.parse_type_expr()?;
        let predicate = if self.at(&Token::Where) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        let end_span = self.current_span();
        Ok(PropositionDef { name, base_type, predicate, span: Span::merge(&start, &end_span) })
    }

    /// Parse `functor NAME<TypeParams> [law: name]* end`.
    fn parse_functor_def(&mut self) -> Result<FunctorDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Functor)?;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_optional_type_params()?;
        let mut laws = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Law) {
                self.advance();
                self.expect(Token::Colon)?;
                let law_span = self.current_span();
                let (law_name, _) = self.expect_ident()?;
                laws.push(LawDecl { name: law_name, span: law_span });
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(FunctorDef { name, type_params, laws, span: Span::merge(&start, &end_span) })
    }

    /// Parse `monad NAME<TypeParams> [law: name]* end`.
    fn parse_monad_def(&mut self) -> Result<MonadDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Monad)?;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_optional_type_params()?;
        let mut laws = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Law) {
                self.advance();
                self.expect(Token::Colon)?;
                let law_span = self.current_span();
                let (law_name, _) = self.expect_ident()?;
                laws.push(LawDecl { name: law_name, span: law_span });
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(MonadDef { name, type_params, laws, span: Span::merge(&start, &end_span) })
    }

    /// Parse `certificate: field = value ... end`.
    fn parse_certificate_def(&mut self) -> Result<CertificateDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Certificate)?;
        self.expect(Token::Colon)?;
        let mut fields = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((tok, _)) = self.tokens.get(self.pos + 1) {
                    if matches!(tok, crate::lexer::Token::Eq) {
                        let field_span = self.current_span();
                        self.advance(); // key
                        self.advance(); // =
                        let val = self.parse_value_as_string()?;
                        fields.push(CertificateField { name: key, value: val, span: field_span });
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CertificateDef { fields, span: Span::merge(&start, &end_span) })
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
            _ => {
                if let Some(name) = self.token_as_ident() {
                    self.pos += 1;
                    Ok(name)
                } else {
                    Err(LoomError::parse(
                        format!("expected value, got {:?}", self.tokens.get(self.pos).map(|(t,_)| t)),
                        self.current_span(),
                    ))
                }
            }
        }
    }

    // ── M66: Aspect-Oriented Specification ───────────────────────────────────

    /// Parse an `aspect Name ... end` block.
    ///
    /// ```text
    /// aspect SecurityAspect
    ///   pointcut: fn where @requires_auth
    ///   before:   verify_token
    ///   after_throwing: log_security_event
    ///   order: 1
    /// end
    /// ```
    fn parse_aspect_def(&mut self) -> Result<AspectDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Aspect)?;
        let (name, _) = self.expect_ident()?;

        let mut pointcut: Option<PointcutExpr> = None;
        let mut before: Vec<String> = Vec::new();
        let mut after: Vec<String> = Vec::new();
        let mut after_throwing: Vec<String> = Vec::new();
        let mut around: Vec<String> = Vec::new();
        let mut on_failure: Option<String> = None;
        let mut max_attempts: Option<u32> = None;
        let mut order: Option<u32> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Pointcut) {
                self.advance();
                self.expect(Token::Colon)?;
                pointcut = Some(self.parse_pointcut_expr()?);
            } else if self.at(&Token::Before) {
                self.advance();
                self.expect(Token::Colon)?;
                let (fn_name, _) = self.expect_ident()?;
                before.push(fn_name);
            } else if self.at(&Token::After) {
                // Distinguish `after:` from `after_throwing:` via peek
                self.advance();
                self.expect(Token::Colon)?;
                let (fn_name, _) = self.expect_ident()?;
                after.push(fn_name);
            } else if self.at(&Token::Around) {
                self.advance();
                self.expect(Token::Colon)?;
                let (fn_name, _) = self.expect_ident()?;
                around.push(fn_name);
            } else if let Some(kw) = self.token_as_ident() {
                match kw.as_str() {
                    "after_throwing" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (fn_name, _) = self.expect_ident()?;
                        after_throwing.push(fn_name);
                    }
                    "on_failure" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (fn_name, _) = self.expect_ident()?;
                        on_failure = Some(fn_name);
                    }
                    "max_attempts" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                            max_attempts = Some(*n as u32);
                            self.advance();
                        }
                    }
                    "order" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                            order = Some(*n as u32);
                            self.advance();
                        }
                    }
                    _ => {
                        return Err(LoomError::parse(
                            format!("unexpected aspect clause `{}`", kw),
                            self.current_span(),
                        ));
                    }
                }
            } else {
                return Err(LoomError::parse(
                    format!("expected aspect clause, got {:?}", self.peek()),
                    self.current_span(),
                ));
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(AspectDef {
            name,
            pointcut,
            before,
            after,
            after_throwing,
            around,
            on_failure,
            max_attempts,
            order,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a pointcut expression: `fn where @annotation [and/or ...]`
    fn parse_pointcut_expr(&mut self) -> Result<PointcutExpr, LoomError> {
        self.expect(Token::Fn)?;
        self.expect(Token::Where)?;
        self.parse_pointcut_condition()
    }

    /// Parse the condition part of a pointcut expression (after `fn where`).
    fn parse_pointcut_condition(&mut self) -> Result<PointcutExpr, LoomError> {
        let left = self.parse_pointcut_atom()?;
        if self.at(&Token::And) {
            self.advance();
            let right = self.parse_pointcut_condition()?;
            Ok(PointcutExpr::And(Box::new(left), Box::new(right)))
        } else if self.at(&Token::Or) {
            self.advance();
            let right = self.parse_pointcut_condition()?;
            Ok(PointcutExpr::Or(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    /// Parse a single pointcut atom: `@annotation` or `effect includes Name`.
    fn parse_pointcut_atom(&mut self) -> Result<PointcutExpr, LoomError> {
        if self.at(&Token::At) {
            self.advance();
            let (ann_name, _) = self.expect_ident()?;
            Ok(PointcutExpr::HasAnnotation(ann_name))
        } else if self.at(&Token::Effect) || self.token_as_ident().as_deref() == Some("effect") {
            self.advance();
            // `effect includes EffectName`
            if self.token_as_ident().as_deref() == Some("includes") {
                self.advance();
                let (effect_name, _) = self.expect_ident()?;
                return Ok(PointcutExpr::EffectIncludes(effect_name));
            }
            Err(LoomError::parse(
                "expected `includes` after `effect` in pointcut",
                self.current_span(),
            ))
        } else {
            Err(LoomError::parse(
                format!("expected pointcut atom, got {:?}", self.peek()),
                self.current_span(),
            ))
        }
    }

    // ── M66b: Annotation Algebra ──────────────────────────────────────────────

    /// Parse an `annotation Name(params)` declaration (may be annotated with
    /// meta-annotations before it, accumulated in `pending_annotations`).
    ///
    /// ```text
    /// @separation(owns: [a, b])
    /// @timing_safety(constant_time: true)
    /// annotation concurrent_transfer(a: String, b: String)
    /// ```
    fn parse_annotation_decl(&mut self) -> Result<AnnotationDecl, LoomError> {
        let start = self.current_span();
        self.expect(Token::Annotation)?;
        let (name, _) = self.expect_ident()?;

        // Optional typed parameter list: `(param_name: TypeName, ...)`
        let mut params: Vec<(String, String)> = Vec::new();
        if self.at(&Token::LParen) {
            self.advance();
            while !self.at(&Token::RParen) && self.peek().is_some() {
                let (param_name, _) = self.expect_ident()?;
                self.expect(Token::Colon)?;
                let (type_name, _) = self.expect_ident()?;
                params.push((param_name, type_name));
                if self.at(&Token::Comma) {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
        }

        // Meta-annotations are any @annotations accumulated before this declaration
        let meta_annotations = std::mem::take(&mut self.pending_annotations);

        let end_span = self.current_span();
        Ok(AnnotationDecl {
            name,
            params,
            meta_annotations,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M67: Correctness Report ───────────────────────────────────────────────

    /// Parse a `correctness_report: proved: ... unverified: ... end` block.
    ///
    /// ```text
    /// correctness_report:
    ///   proved:
    ///     - membrane_integrity: separation_logic_proved
    ///     - homeostasis:        refinement_bounds_verified
    ///   unverified:
    ///     - smt_check: requires_smt_feature
    /// end
    /// ```
    fn parse_correctness_report(&mut self) -> Result<CorrectnessReport, LoomError> {
        let start = self.current_span();
        // Consume `correctness_report` ident + `:`
        self.advance(); // past `correctness_report` ident
        self.expect(Token::Colon)?;

        let mut proved: Vec<ProvedClaim> = Vec::new();
        let mut unverified: Vec<(String, String)> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(section) = self.token_as_ident() {
                match section.as_str() {
                    "proved" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        // Parse `- property_name: checker_name` lines
                        // property_name may be a keyword (e.g. `separation`), so use token_as_ident
                        while self.at(&Token::Minus) {
                            let claim_span = self.current_span();
                            self.advance(); // past `-`
                            let property = if let Some(s) = self.token_as_ident() {
                                self.advance();
                                s
                            } else {
                                let (s, _) = self.expect_ident()?;
                                s
                            };
                            self.expect(Token::Colon)?;
                            let checker = if let Some(s) = self.token_as_ident() {
                                self.advance();
                                s
                            } else {
                                let (s, _) = self.expect_ident()?;
                                s
                            };
                            proved.push(ProvedClaim {
                                property,
                                checker,
                                span: claim_span,
                            });
                        }
                    }
                    "unverified" => {
                        self.advance();
                        self.expect(Token::Colon)?;
                        while self.at(&Token::Minus) {
                            self.advance(); // past `-`
                            let property = if let Some(s) = self.token_as_ident() {
                                self.advance();
                                s
                            } else {
                                let (s, _) = self.expect_ident()?;
                                s
                            };
                            self.expect(Token::Colon)?;
                            let reason = if let Some(s) = self.token_as_ident() {
                                self.advance();
                                s
                            } else {
                                let (s, _) = self.expect_ident()?;
                                s
                            };
                            unverified.push((property, reason));
                        }
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(CorrectnessReport {
            proved,
            unverified,
            span: Span::merge(&start, &end_span),
        })
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

    /// Parse a `being Name … end` block (Aristotle's four causes).
    fn parse_being_def(&mut self) -> Result<BeingDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Being)?;
        let (name, _) = self.expect_ident()?;

        let describe = self.parse_describe();

        // Safety and capability annotations before the block body.
        let annotations = self.parse_annotations();

        let mut matter = None;
        let mut form = None;
        let mut function = None;
        let mut telos = None;
        let mut regulate_blocks = Vec::new();
        let mut evolve_block = None;
        let mut autopoietic = false;
        let mut epigenetic_blocks: Vec<EpigeneticBlock> = Vec::new();
        let mut morphogen_blocks: Vec<MorphogenBlock> = Vec::new();
        let mut telomere: Option<TelomereBlock> = None;
        let mut crispr_blocks: Vec<CrisprBlock> = Vec::new();
        let mut plasticity_blocks: Vec<PlasticityBlock> = Vec::new();
        let mut canalization: Option<CanalizationBlock> = None;
        let mut senescence: Option<SenescenceBlock> = None;
        let mut criticality: Option<CriticalityBlock> = None;
        let mut umwelt: Option<UmweltBlock> = None;
        let mut resonance: Option<ResonanceBlock> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Matter) {
                let sec_start = self.current_span();
                self.advance(); // consume `matter`
                self.expect(Token::Colon)?;
                let mut fields = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    let (field_name, field_span) = self.expect_any_name()?;
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type_expr()?;
                    fields.push(FieldDef {
                        name: field_name,
                        ty,
                        annotations: Vec::new(),
                        span: field_span,
                    });
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                matter = Some(MatterBlock { fields, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Form) {
                let sec_start = self.current_span();
                self.advance(); // consume `form`
                self.expect(Token::Colon)?;
                let mut types = Vec::new();
                let mut enums = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Type) {
                        if let Ok(item) = self.parse_type_or_refined() {
                            if let Item::Type(td) = item { types.push(td); }
                        }
                    } else if self.at(&Token::Enum) {
                        if let Ok(ed) = self.parse_enum_def() { enums.push(ed); }
                    } else {
                        break;
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                form = Some(FormBlock { types, enums, span: Span::merge(&sec_start, &sec_end) });
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "function")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                let sec_start = self.current_span();
                self.advance(); // consume "function"
                self.advance(); // consume ":"
                let mut fns = Vec::new();
                while self.at(&Token::Fn) && !self.at(&Token::End) {
                    fns.push(self.parse_fn_def()?);
                }
                let sec_end = self.current_span();
                function = Some(FunctionBlock { fns, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Telos) {
                let sec_start = self.current_span();
                self.advance(); // consume `telos`
                self.expect(Token::Colon)?;
                let description = match self.tokens.get(self.pos) {
                    Some((Token::StrLit(s), _)) => {
                        let s = s.clone();
                        self.pos += 1;
                        s
                    }
                    _ => return Err(LoomError::parse(
                        "expected string literal after telos:",
                        self.current_span(),
                    )),
                };
                let mut fitness_fn = None;
                let mut modifiable_by = None;
                let mut bounded_by = None;
                let mut sign = None;
                // Loop over optional fields in any order until `end`.
                while !self.at(&Token::End) && self.peek().is_some() {
                    if matches!(self.tokens.get(self.pos), Some((Token::Fitness, _))) {
                        self.advance(); // consume `fitness`
                        self.expect(Token::Colon)?;
                        let mut parts = Vec::new();
                        while !self.at(&Token::End) && self.peek().is_some() {
                            // Stop if we hit a known telos field keyword.
                            let is_field = matches!(self.tokens.get(self.pos),
                                Some((Token::ModifiableBy, _)) | Some((Token::BoundedBy, _)))
                                || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _))
                                    if n == "sign");
                            if is_field { break; }
                            if let Some((tok, _)) = self.tokens.get(self.pos) {
                                parts.push(format!("{:?}", tok));
                                self.pos += 1;
                            }
                        }
                        fitness_fn = Some(parts.join(" "));
                    } else if matches!(self.tokens.get(self.pos), Some((Token::ModifiableBy, _))) {
                        self.advance(); // consume `modifiable_by`
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                            let val = val.clone();
                            self.pos += 1;
                            modifiable_by = Some(val);
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::BoundedBy, _))) {
                        self.advance(); // consume `bounded_by`
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                            let val = val.clone();
                            self.pos += 1;
                            bounded_by = Some(val);
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "sign")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        self.advance(); // consume `sign`
                        self.advance(); // consume `:`
                        if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                            let val = val.clone();
                            self.pos += 1;
                            sign = Some(val);
                        }
                    } else {
                        // Unknown token in telos block — skip to avoid infinite loop.
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                telos = Some(TelosDef { description, fitness_fn, modifiable_by, bounded_by, sign, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Regulate) {
                let sec_start = self.current_span();
                self.advance(); // consume `regulate`
                let (variable, _) = self.expect_ident()?;
                let mut target = String::new();
                let mut bounds = None;
                let mut response = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "target") {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (val, _) = self.expect_ident()?;
                        target = val;
                    } else if self.at(&Token::Bounds) {
                        self.advance(); // consume `bounds`
                        self.expect(Token::Colon)?;
                        self.expect(Token::LParen)?;
                        let (low, _) = self.expect_ident()?;
                        self.expect(Token::Comma)?;
                        let (high, _) = self.expect_ident()?;
                        self.expect(Token::RParen)?;
                        bounds = Some((low, high));
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "response") {
                        self.advance();
                        self.expect(Token::Colon)?;
                        while self.at(&Token::Bar) {
                            self.advance(); // consume `|`
                            let (condition, _) = self.expect_ident()?;
                            self.expect(Token::Arrow)?;
                            let (action, _) = self.expect_ident()?;
                            response.push((condition, action));
                        }
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                regulate_blocks.push(RegulateBlock {
                    variable,
                    target,
                    bounds,
                    response,
                    span: Span::merge(&sec_start, &sec_end),
                });
            } else if self.at(&Token::Evolve) {
                let sec_start = self.current_span();
                self.advance(); // consume `evolve`
                let mut search_cases = Vec::new();
                let mut constraint = String::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Toward) {
                        self.advance(); // consume `toward`
                        self.expect(Token::Colon)?;
                        self.advance(); // consume `telos` identifier
                    } else if self.at(&Token::Search) {
                        self.advance(); // consume `search`
                        self.expect(Token::Colon)?;
                        while self.at(&Token::Bar) {
                            self.advance(); // consume `|`
                            let (strategy_name, _) = self.expect_ident()?;
                            let strategy = match strategy_name.as_str() {
                                "gradient_descent"    => SearchStrategy::GradientDescent,
                                "stochastic_gradient" => SearchStrategy::StochasticGradient,
                                "simulated_annealing" => SearchStrategy::SimulatedAnnealing,
                                "derivative_free"     => SearchStrategy::DerivativeFree,
                                "mcmc"                => SearchStrategy::Mcmc,
                                _                     => SearchStrategy::DerivativeFree,
                            };
                            // consume optional `when <condition>` — only if next is `when` keyword
                            // followed by an ident that is NOT itself followed by `:` (which would
                            // indicate the start of the next clause, e.g. `constraint:`).
                            let when_present = matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "when");
                            if when_present {
                                self.advance(); // consume `when`
                            }
                            let when = if when_present {
                                // Read the condition ident, but only if not followed by `:`
                                let next_is_colon = matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
                                if !next_is_colon {
                                    if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                                        let w = n.clone();
                                        self.pos += 1;
                                        w
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            search_cases.push(SearchCase { strategy, when });
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "constraint") {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                            constraint = s.clone();
                            self.pos += 1;
                        }
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                evolve_block = Some(EvolveBlock {
                    search_cases,
                    constraint,
                    span: Span::merge(&sec_start, &sec_end),
                });
            } else if self.at(&Token::Autopoietic) {
                self.advance(); // consume `autopoietic`
                self.expect(Token::Colon)?;
                if let Some((Token::BoolLit(b), _)) = self.tokens.get(self.pos) {
                    autopoietic = *b;
                    self.pos += 1;
                }
            } else if self.at(&Token::Epigenetic) {
                let sec_start = self.current_span();
                self.advance(); // consume `epigenetic`
                self.expect(Token::Colon)?;
                let mut signal = String::new();
                let mut modifies = String::new();
                let mut reverts_when = None;
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Signal) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (val, _) = self.expect_ident()?;
                        signal = val;
                    } else if self.at(&Token::Modifies) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let mut parts = Vec::new();
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            parts.push(n.clone());
                            self.pos += 1;
                        }
                        while matches!(self.tokens.get(self.pos), Some((Token::Dot, _))) {
                            self.pos += 1;
                            if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                                parts.push(n.clone());
                                self.pos += 1;
                            }
                        }
                        modifies = parts.join(".");
                    } else if self.at(&Token::RevertsWhen) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (val, _) = self.expect_ident()?;
                        reverts_when = Some(val);
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                epigenetic_blocks.push(EpigeneticBlock { signal, modifies, reverts_when, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Morphogen) {
                let sec_start = self.current_span();
                self.advance(); // consume `morphogen`
                self.expect(Token::Colon)?;
                let mut signal = String::new();
                let mut threshold = String::new();
                let mut produces = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Signal) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (val, _) = self.expect_ident()?;
                        signal = val;
                    } else if self.at(&Token::Threshold) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        match self.tokens.get(self.pos) {
                            Some((Token::FloatLit(f), _)) => {
                                threshold = f.to_string();
                                self.pos += 1;
                            }
                            Some((Token::IntLit(i), _)) => {
                                threshold = i.to_string();
                                self.pos += 1;
                            }
                            _ => {
                                if let Ok((val, _)) = self.expect_ident() {
                                    threshold = val;
                                }
                            }
                        }
                    } else if self.at(&Token::Produces) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        self.expect(Token::LBracket)?;
                        loop {
                            if self.at(&Token::RBracket) { break; }
                            if let Ok((val, _)) = self.expect_ident() {
                                produces.push(val);
                            }
                            if self.at(&Token::Comma) { self.advance(); }
                            if self.at(&Token::RBracket) { break; }
                        }
                        self.expect(Token::RBracket)?;
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                morphogen_blocks.push(MorphogenBlock { signal, threshold, produces, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Telomere) {
                let sec_start = self.current_span();
                self.advance(); // consume `telomere`
                self.expect(Token::Colon)?;
                let mut limit = 0u64;
                let mut on_exhaustion = String::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Limit) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                            limit = *n as u64;
                            self.pos += 1;
                        }
                    } else if self.at(&Token::OnExhaustion) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let (val, _) = self.expect_any_name()?;
                        on_exhaustion = val;
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                telomere = Some(TelomereBlock { limit, on_exhaustion, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Crispr) {
                let sec_start = self.current_span();
                self.advance(); // consume `crispr`
                self.expect(Token::Colon)?;
                let mut target = String::new();
                let mut replace = String::new();
                let mut guide = String::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "target") {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let mut parts = Vec::new();
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            parts.push(n.clone());
                            self.pos += 1;
                        }
                        while matches!(self.tokens.get(self.pos), Some((Token::Dot, _))) {
                            self.pos += 1;
                            if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                                parts.push(n.clone());
                                self.pos += 1;
                            }
                        }
                        target = parts.join(".");
                    } else if self.at(&Token::Replace) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        let mut parts = Vec::new();
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            parts.push(n.clone());
                            self.pos += 1;
                        }
                        while matches!(self.tokens.get(self.pos), Some((Token::Dot, _))) {
                            self.pos += 1;
                            if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                                parts.push(n.clone());
                                self.pos += 1;
                            }
                        }
                        replace = parts.join(".");
                    } else if self.at(&Token::Guide) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            guide = n.clone();
                            self.pos += 1;
                        }
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                crispr_blocks.push(CrisprBlock {
                    target,
                    replace,
                    guide,
                    span: Span::merge(&sec_start, &sec_end),
                });
            } else if self.at(&Token::Plasticity) {
                let sec_start = self.current_span();
                self.advance(); // consume `plasticity`
                self.expect(Token::Colon)?;
                let mut trigger = String::new();
                let mut modifies = String::new();
                let mut rule = PlasticityRule::Hebbian;
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Trigger) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            trigger = n.clone();
                            self.pos += 1;
                        }
                    } else if self.at(&Token::Modifies) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            modifies = n.clone();
                            self.pos += 1;
                        }
                    } else if self.at(&Token::Rule) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        match self.tokens.get(self.pos) {
                            Some((Token::Hebbian, _)) => {
                                rule = PlasticityRule::Hebbian;
                                self.pos += 1;
                            }
                            Some((Token::Boltzmann, _)) => {
                                rule = PlasticityRule::Boltzmann;
                                self.pos += 1;
                            }
                            Some((Token::Ident(n), _)) if n == "reinforcement_learning" => {
                                rule = PlasticityRule::ReinforcementLearning;
                                self.pos += 1;
                            }
                            _ => {
                                return Err(LoomError::parse(
                                    "unknown plasticity rule: expected hebbian, boltzmann, or reinforcement_learning",
                                    self.current_span(),
                                ));
                            }
                        }
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                plasticity_blocks.push(PlasticityBlock {
                    trigger,
                    modifies,
                    rule,
                    span: Span::merge(&sec_start, &sec_end),
                });
            } else if self.at(&Token::Canalize) {
                canalization = Some(self.parse_canalization_block()?);
            } else if self.at(&Token::Senescence) {
                senescence = Some(self.parse_senescence_block()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "criticality")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                criticality = Some(self.parse_criticality_block()?);
            } else if self.at(&Token::Umwelt) {
                // M80: umwelt: block
                let sec_start = self.current_span();
                self.advance(); // consume `umwelt`
                self.expect(Token::Colon)?;
                let mut detects: Vec<String> = Vec::new();
                let mut blind_to: Vec<String> = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "detects")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        self.advance(); // consume `detects`
                        self.advance(); // consume `:`
                        self.expect(Token::LBracket)?;
                        while !self.at(&Token::RBracket) && self.peek().is_some() {
                            if let Ok((val, _)) = self.expect_ident() {
                                detects.push(val);
                            }
                            if self.at(&Token::Comma) { self.advance(); }
                        }
                        self.expect(Token::RBracket)?;
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "blind_to")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        self.advance(); // consume `blind_to`
                        self.advance(); // consume `:`
                        self.expect(Token::LBracket)?;
                        while !self.at(&Token::RBracket) && self.peek().is_some() {
                            if let Ok((val, _)) = self.expect_ident() {
                                blind_to.push(val);
                            }
                            if self.at(&Token::Comma) { self.advance(); }
                        }
                        self.expect(Token::RBracket)?;
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                umwelt = Some(UmweltBlock { detects, blind_to, span: Span::merge(&sec_start, &sec_end) });
            } else if self.at(&Token::Resonance) {
                // M82: resonance: block
                let sec_start = self.current_span();
                self.advance(); // consume `resonance`
                self.expect(Token::Colon)?;
                let mut correlations: Vec<CorrelationPair> = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    // correlate: SignalA with SignalB [via fn_name]
                    if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "correlate")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        let pair_start = self.current_span();
                        self.advance(); // consume `correlate`
                        self.advance(); // consume `:`
                        let (signal_a, _) = self.expect_ident()?;
                        // consume `with`
                        if self.at(&Token::With) { self.advance(); }
                        let (signal_b, _) = self.expect_ident()?;
                        // optional `via fn_name`
                        let via = if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "via")
                        {
                            self.advance(); // consume `via`
                            if let Some((Token::Ident(fn_name), _)) = self.tokens.get(self.pos) {
                                let fn_name = fn_name.clone();
                                self.pos += 1;
                                Some(fn_name)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let pair_end = self.current_span();
                        correlations.push(CorrelationPair {
                            signal_a,
                            signal_b,
                            via,
                            span: Span::merge(&pair_start, &pair_end),
                        });
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                resonance = Some(ResonanceBlock { correlations, span: Span::merge(&sec_start, &sec_end) });
            } else {
                // Unknown token in being body — skip to avoid infinite loop.
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(BeingDef {
            name,
            describe,
            annotations,
            matter,
            form,
            function,
            telos,
            regulate_blocks,
            evolve_block,
            epigenetic_blocks,
            morphogen_blocks,
            telomere,
            autopoietic,
            crispr_blocks,
            plasticity_blocks,
            canalization,
            senescence,
            criticality,
            umwelt,
            resonance,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse an `ecosystem Name … end` block.
    fn parse_ecosystem_def(&mut self) -> Result<EcosystemDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Ecosystem)?;
        let (name, _) = self.expect_ident()?;

        let describe = self.parse_describe();

        let mut members: Vec<String> = Vec::new();
        let mut signals: Vec<SignalDef> = Vec::new();
        let mut telos: Option<String> = None;
        let mut quorum_blocks: Vec<QuorumBlock> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Members) {
                // members: [Being1, Being2, ...]
                self.advance(); // consume `members`
                self.expect(Token::Colon)?;
                self.expect(Token::LBracket)?;
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    let (member_name, _) = self.expect_ident()?;
                    members.push(member_name);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
            } else if self.at(&Token::Signal) {
                // signal SignalName from BeingA to BeingB
                //   payload: TypeExpr
                // end
                let sig_start = self.current_span();
                self.advance(); // consume `signal`
                let (sig_name, _) = self.expect_ident()?;
                self.expect(Token::From)?;
                let (from_name, _) = self.expect_ident()?;
                self.expect(Token::To)?;
                let (to_name, _) = self.expect_ident()?;
                let mut payload = String::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Payload) {
                        self.advance(); // consume `payload`
                        self.expect(Token::Colon)?;
                        // Collect payload type tokens until `end`, reconstructing source
                        let mut parts = Vec::new();
                        while !self.at(&Token::End) && self.peek().is_some() {
                            if let Some((tok, _)) = self.tokens.get(self.pos) {
                                parts.push(token_to_source(tok));
                                self.pos += 1;
                            }
                        }
                        payload = parts.join("");
                    } else {
                        self.advance();
                    }
                }
                let sig_end = self.current_span();
                self.expect(Token::End)?;
                signals.push(SignalDef {
                    name: sig_name,
                    from: from_name,
                    to: to_name,
                    payload,
                    span: Span::merge(&sig_start, &sig_end),
                });
            } else if self.at(&Token::Telos) {
                self.advance(); // consume `telos`
                self.expect(Token::Colon)?;
                match self.tokens.get(self.pos) {
                    Some((Token::StrLit(s), _)) => {
                        telos = Some(s.clone());
                        self.pos += 1;
                    }
                    _ => return Err(LoomError::parse(
                        "expected string literal after telos:",
                        self.current_span(),
                    )),
                }
            } else if self.at(&Token::Quorum) {
                let sec_start = self.current_span();
                self.advance(); // consume `quorum`
                self.expect(Token::Colon)?;
                let mut signal = String::new();
                let mut threshold = String::new();
                let mut action = String::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Signal) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            signal = n.clone();
                            self.pos += 1;
                        }
                    } else if self.at(&Token::Threshold) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        match self.tokens.get(self.pos) {
                            Some((Token::FloatLit(f), _)) => {
                                threshold = format!("{}", f);
                                self.pos += 1;
                            }
                            Some((Token::IntLit(i), _)) => {
                                threshold = format!("{}", i);
                                self.pos += 1;
                            }
                            _ => { self.advance(); }
                        }
                    } else if self.at(&Token::Action) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                            action = n.clone();
                            self.pos += 1;
                        }
                    } else {
                        self.advance();
                    }
                }
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                quorum_blocks.push(QuorumBlock {
                    signal,
                    threshold,
                    action,
                    span: Span::merge(&sec_start, &sec_end),
                });
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(EcosystemDef {
            name,
            describe,
            members,
            signals,
            telos,
            quorum_blocks,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `flow label :: TypeA, TypeB, ...`.
    fn parse_provides_block(&mut self) -> Result<Provides, LoomError> {
        self.expect(Token::LBrace)?;
        let mut ops = Vec::new();
        while !self.at(&Token::RBrace) && self.peek().is_some() {
            let (op_name, _) = self.expect_ident()?;
            self.expect(Token::ColonColon)?;
            let sig = self.parse_fn_type_signature()?;
            ops.push((op_name, sig));
            // Optional comma separator
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Provides { ops })
    }

    /// Parse the `{ name : type, … }` block following `requires`.
    fn parse_requires_block(&mut self) -> Result<Requires, LoomError> {
        self.expect(Token::LBrace)?;
        let mut deps = Vec::new();
        while !self.at(&Token::RBrace) && self.peek().is_some() {
            let (dep_name, _) = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type_expr()?;
            deps.push((dep_name, ty));
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Requires { deps })
    }

    // ── Items ────────────────────────────────────────────────────────────

    /// Dispatch to the correct item parser based on the leading keyword.
    fn parse_item(&mut self) -> Result<Item, LoomError> {
        match self.peek() {
            Some(Token::Fn) => Ok(Item::Fn(self.parse_fn_def()?)),
            Some(Token::Type) => {
                Ok(self.parse_type_or_refined()?)
            }
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

    // ── Function definition ───────────────────────────────────────────────

    /// Parse `fn NAME[<A, B>] [describe: "..."] [@ann]* :: type_sig [require: expr]* [ensure: expr]* body* end`.
    pub fn parse_fn_def(&mut self) -> Result<FnDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Fn)?;
        let (name, _) = self.expect_any_name()?;

        // Optional describe: and @annotations before the type signature.
        // Merge any annotations accumulated at item level (before the `fn` keyword).
        let describe = self.parse_describe();
        let mut annotations = std::mem::take(&mut self.pending_annotations);
        annotations.extend(self.parse_annotations());

        // Optional type parameter list: `<A, B, C>`.
        let type_params = if self.at(&Token::Lt) {
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
            params
        } else {
            Vec::new()
        };

        self.expect(Token::ColonColon)?;
        let type_sig = self.parse_fn_type_signature()?;

        // Collect any consequence tiers parsed inside Effect<[X@tier, ...]>.
        let effect_tiers = std::mem::take(&mut self.pending_effect_tiers);

        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        let mut with_deps = Vec::new();

        // Collect `require:` / `ensure:` / `with` / `separation:` / new M59-M64 clauses.
        let mut separation: Option<SeparationBlock> = None;
        let mut gradual: Option<GradualBlock> = None;
        let mut distribution: Option<DistributionBlock> = None;
        let mut timing_safety: Option<TimingSafetyBlock> = None;
        let mut termination: Option<String> = None;
        let mut proofs: Vec<ProofAnnotation> = Vec::new();
        loop {
            if self.at(&Token::Require) {
                requires.push(self.parse_contract()?);
            } else if self.at(&Token::Ensure) {
                ensures.push(self.parse_contract()?);
            } else if self.at(&Token::With) {
                self.advance();
                let (dep, _) = self.expect_ident()?;
                with_deps.push(dep);
            } else if self.at(&Token::Separation) {
                separation = Some(self.parse_separation_block()?);
            } else if self.at(&Token::Gradual) {
                gradual = Some(self.parse_gradual_block()?);
            } else if self.at(&Token::Distribution) {
                distribution = Some(self.parse_distribution_block()?);
            } else if self.at(&Token::TimingSafety) {
                timing_safety = Some(self.parse_timing_safety_block()?);
            } else if self.at(&Token::Termination) {
                self.advance();
                self.expect(Token::Colon)?;
                let (val, _) = self.expect_ident()?;
                termination = Some(val);
            } else if self.at(&Token::Proof) {
                self.advance();
                self.expect(Token::Colon)?;
                let start_proof = self.current_span();
                let (strategy, _) = self.expect_ident()?;
                proofs.push(ProofAnnotation { strategy, span: start_proof });
            } else {
                break;
            }
        }

        // M68: Optional degenerate block.
        let degenerate = if self.at(&Token::Degenerate) {
            Some(self.parse_degenerate_block()?)
        } else {
            None
        };

        // Body expressions until `end`.
        let mut body = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            body.push(self.parse_expr()?);
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(FnDef {
            name,
            describe,
            annotations,
            type_params,
            type_sig,
            effect_tiers,
            requires,
            ensures,
            with_deps,
            separation,
            gradual,
            distribution,
            timing_safety,
            termination,
            proofs,
            degenerate,
            body,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── Type / refined type ───────────────────────────────────────────────

    /// Decide between a refined type (`type E = String where pred`) and a
    /// product type (`type Point = x: Float, y: Float end`).
    fn parse_type_or_refined(&mut self) -> Result<Item, LoomError> {
        let start = self.current_span();
        self.expect(Token::Type)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;

        // Peek ahead: if next is an ident followed by `where`, it's refined.
        // If next is `end` or (ident followed by `:`) it's a product type (field list).
        // Otherwise it's a type alias: `type X = SomeType<...>`.
        let is_refined = match (self.peek(), self.peek2()) {
            (Some(Token::Ident(_)), Some(Token::Where)) => true,
            _ => false,
        };

        let is_field_list = match (self.peek(), self.peek2()) {
            (Some(Token::Ident(_)), Some(Token::Colon)) => true,
            (Some(Token::End), _) => true,   // empty product type
            _ => false,
        };

        if is_refined {
            let base_type = self.parse_type_expr()?;
            self.expect(Token::Where)?;
            let predicate = self.parse_expr()?;
            // M73: Optional on_violation / repair_fn block.
            let mut on_violation: Option<String> = None;
            let mut repair_fn: Option<String> = None;
            let mut has_error_correction = false;
            while !self.at(&Token::End) && self.peek().is_some() {
                if let Some(key) = self.token_as_ident() {
                    if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                        match key.as_str() {
                            "on_violation" => {
                                self.advance(); // key
                                self.advance(); // colon
                                let (val, _) = self.expect_ident()?;
                                on_violation = Some(val);
                                has_error_correction = true;
                                continue;
                            }
                            "repair_fn" => {
                                self.advance(); // key
                                self.advance(); // colon
                                let (val, _) = self.expect_ident()?;
                                repair_fn = Some(val);
                                has_error_correction = true;
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                break;
            }
            let end_span = self.current_span();
            if has_error_correction {
                self.expect(Token::End)?;
            }
            Ok(Item::RefinedType(RefinedType {
                name,
                base_type,
                predicate,
                on_violation,
                repair_fn,
                span: Span::merge(&start, &end_span),
            }))
        } else if is_field_list {
            Ok(Item::Type(self.parse_type_fields(name, start)?))
        } else {
            // Type alias: `type X = TypeExpr` — no trailing `end`.
            let ty = self.parse_type_expr()?;
            let end_span = self.current_span();
            Ok(Item::TypeAlias(name, ty, Span::merge(&start, &end_span)))
        }
    }

    /// Parse the field list body of a product type (already past `type N =`).
    fn parse_type_fields(&mut self, name: String, start: Span) -> Result<TypeDef, LoomError> {
        let mut fields = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            let field_start = self.current_span();
            let (field_name, _) = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type_expr()?;
            // Parse optional field-level privacy annotations (@pii, @gdpr, etc.)
            let annotations = self.parse_annotations();
            let field_end = self.current_span();
            fields.push(FieldDef {
                name: field_name,
                ty,
                annotations,
                span: Span::merge(&field_start, &field_end),
            });
            // Optional comma between fields.
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(TypeDef {
            name,
            fields,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a full product-type definition (`type NAME = … end`).
    pub fn parse_type_def(&mut self) -> Result<TypeDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Type)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;
        self.parse_type_fields(name, start)
    }

    /// Parse an enum definition (`enum NAME = | V … end`).
    pub fn parse_enum_def(&mut self) -> Result<EnumDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Enum)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;

        let mut variants = Vec::new();
        while self.at(&Token::Bar) {
            variants.push(self.parse_enum_variant()?);
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(EnumDef {
            name,
            variants,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a single enum variant (`| NAME [of TypeExpr]`).
    fn parse_enum_variant(&mut self) -> Result<EnumVariant, LoomError> {
        let start = self.current_span();
        self.expect(Token::Bar)?;
        let (variant_name, _) = self.expect_ident()?;
        let payload = if self.at(&Token::Of) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        Ok(EnumVariant {
            name: variant_name,
            payload,
            span: start,
        })
    }

    /// Parse a refined type definition (`type NAME = base_type where pred`).
    pub fn parse_refined_type(&mut self) -> Result<RefinedType, LoomError> {
        let start = self.current_span();
        self.expect(Token::Type)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;
        let base_type = self.parse_type_expr()?;
        self.expect(Token::Where)?;
        let predicate = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(RefinedType {
            name,
            base_type,
            predicate,
            on_violation: None,
            repair_fn: None,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── Type expressions ──────────────────────────────────────────────────

    /// Parse a type expression.
    ///
    /// Handles: `Effect<[E1,E2,...], T>`, `Option<T>`, `Result<T, E>`,
    /// `Generic<params>`, `Base`, `(A, B, C)`.
    pub fn parse_type_expr(&mut self) -> Result<TypeExpr, LoomError> {
        match self.peek() {
            Some(Token::LParen) => {
                self.advance();
                if self.at(&Token::RParen) {
                    self.advance();
                    return Ok(TypeExpr::Base("Unit".to_string()));
                }
                let first = self.parse_type_expr()?;
                if self.at(&Token::Comma) {
                    // Tuple
                    let mut elems = vec![first];
                    while self.at(&Token::Comma) {
                        self.advance();
                        elems.push(self.parse_type_expr()?);
                    }
                    self.expect(Token::RParen)?;
                    return Ok(TypeExpr::Tuple(elems));
                }
                self.expect(Token::RParen)?;
                Ok(first)
            }
            Some(Token::Question) => {
                self.advance();
                Ok(TypeExpr::Dynamic)
            }
            Some(Token::Ident(_)) => {
                let (name, _) = self.expect_ident()?;
                // Check for `<` opening a parameter list.
                if self.at(&Token::Lt) {
                    self.advance(); // consume `<`
                    self.parse_generic_tail(name)
                } else {
                    Ok(TypeExpr::Base(name))
                }
            }
            Some(tok) => Err(LoomError::parse(
                format!("expected type expression, found {:?}", tok),
                self.current_span(),
            )),
            None => Err(LoomError::parse(
                "expected type expression, found end of input",
                Span::synthetic(),
            )),
        }
    }

    /// Parse the tail of `Name<...>` after the `<` has been consumed.
    ///
    /// Handles the special forms `Effect<[E...], T>`, `Option<T>`,
    /// `Result<T, E>`, `Tensor<rank, [shape], unit>`, and arbitrary generics `Name<T...>`.
    fn parse_generic_tail(&mut self, name: String) -> Result<TypeExpr, LoomError> {
        match name.as_str() {
            "Effect" => {
                // `Effect<[E1@tier, E2, ...], ReturnType>`
                self.expect(Token::LBracket)?;
                let mut effects = Vec::new();
                let mut effect_tiers_local: Vec<(String, ConsequenceTier)> = Vec::new();
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    let (eff, _) = self.expect_ident()?;
                    // Optional @tier suffix
                    if self.at(&Token::At) {
                        self.advance();
                        if let Some((Token::Ident(tier_name), _)) = self.tokens.get(self.pos) {
                            let tier = match tier_name.as_str() {
                                "pure"         => ConsequenceTier::Pure,
                                "reversible"   => ConsequenceTier::Reversible,
                                "irreversible" => ConsequenceTier::Irreversible,
                                _              => ConsequenceTier::Irreversible,
                            };
                            self.advance();
                            effect_tiers_local.push((eff.clone(), tier));
                        }
                    }
                    effects.push(eff);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
                self.expect(Token::Comma)?;
                let inner = self.parse_type_expr()?;
                self.expect(Token::Gt)?;
                // Store tiers in a thread-local so parse_fn_def can pick them up.
                // NOTE: We store them in the return value directly via a shared cell
                // set by parse_fn_def. Since parse_generic_tail can't return extra data,
                // we store them in a temporary field on the parser.
                self.pending_effect_tiers.extend(effect_tiers_local);
                Ok(TypeExpr::Effect(effects, Box::new(inner)))
            }
            "Option" => {
                let inner = self.parse_type_expr()?;
                self.expect(Token::Gt)?;
                Ok(TypeExpr::Option(Box::new(inner)))
            }
            "Result" => {
                let ok = self.parse_type_expr()?;
                self.expect(Token::Comma)?;
                let err = self.parse_type_expr()?;
                self.expect(Token::Gt)?;
                Ok(TypeExpr::Result(Box::new(ok), Box::new(err)))
            }
            "Tensor" => {
                // `Tensor<rank, [shape...], unit>`
                // rank is an integer literal; shape is bracket-enclosed idents/ints; unit is a type expr.
                let span = self.current_span();
                let rank = self.parse_tensor_rank()?;
                self.expect(Token::Comma)?;
                let shape = self.parse_tensor_shape()?;
                self.expect(Token::Comma)?;
                let unit = self.parse_type_expr()?;
                self.expect(Token::Gt)?;
                Ok(TypeExpr::Tensor { rank, shape, unit: Box::new(unit), span })
            }
            _ => {
                // Arbitrary generic: `Name<T1, T2, ...>`
                let mut params = Vec::new();
                while !self.at(&Token::Gt) && self.peek().is_some() {
                    params.push(self.parse_type_expr()?);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::Gt)?;
                Ok(TypeExpr::Generic(name, params))
            }
        }
    }

    /// Parse a curried function type signature: `T1 -> T2 -> ... -> Tn`.
    pub fn parse_fn_type_signature(&mut self) -> Result<FnTypeSignature, LoomError> {
        let mut types = vec![self.parse_type_expr()?];
        while self.at(&Token::Arrow) {
            self.advance();
            types.push(self.parse_type_expr()?);
        }
        // All but the last element are parameters; the last is the return type.
        let return_type = Box::new(types.pop().expect("at least one type in signature"));
        Ok(FnTypeSignature {
            params: types,
            return_type,
        })
    }

    // ── Contracts ─────────────────────────────────────────────────────────

    /// Parse a `require: expr` or `ensure: expr` contract clause.
    pub fn parse_contract(&mut self) -> Result<Contract, LoomError> {
        let start = self.current_span();
        // Consume `require` or `ensure` keyword.
        self.advance();
        self.expect(Token::Colon)?;
        let expr = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(Contract {
            expr,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── Expressions ───────────────────────────────────────────────────────

    /// Parse an expression — dispatches to `let`, `match`, `for`, or operator-level.
    pub fn parse_expr(&mut self) -> Result<Expr, LoomError> {
        match self.peek() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Match) => self.parse_match(),
            Some(Token::For) => self.parse_for_in(),
            _ => self.parse_pipe(),
        }
    }

    /// Parse a `for VAR in ITER { BODY }` loop expression.
    fn parse_for_in(&mut self) -> Result<Expr, LoomError> {
        let start = self.current_span();
        self.expect(Token::For)?;
        let (var, _) = self.expect_ident()?;
        self.expect(Token::In)?;
        let iter = self.parse_pipe()?;
        self.expect(Token::LBrace)?;
        let body = self.parse_expr()?;
        let end_span = self.current_span();
        self.expect(Token::RBrace)?;
        Ok(Expr::ForIn {
            var,
            iter: Box::new(iter),
            body: Box::new(body),
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `let NAME = expr` binding.
    fn parse_let(&mut self) -> Result<Expr, LoomError> {
        let start = self.current_span();
        self.expect(Token::Let)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(Expr::Let {
            name,
            value: Box::new(value),
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `match expr arm* end` expression.
    pub fn parse_match(&mut self) -> Result<Expr, LoomError> {
        let start = self.current_span();
        self.expect(Token::Match)?;
        let subject = self.parse_pipe()?; // subject is not another match/let
        let mut arms = Vec::new();
        while self.at(&Token::Bar) {
            arms.push(self.parse_match_arm()?);
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(Expr::Match {
            subject: Box::new(subject),
            arms,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a single match arm: `| pattern [if guard] -> body`.
    pub fn parse_match_arm(&mut self) -> Result<MatchArm, LoomError> {
        let start = self.current_span();
        self.expect(Token::Bar)?;
        let pattern = self.parse_pattern()?;
        let guard = if self.at(&Token::If) {
            self.advance();
            Some(self.parse_pipe()?)
        } else {
            None
        };
        self.expect(Token::Arrow)?;
        let body = self.parse_expr()?;
        let end_span = self.current_span();
        Ok(MatchArm {
            pattern,
            guard,
            body,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a pattern for use in match arms.
    fn parse_pattern(&mut self) -> Result<Pattern, LoomError> {
        match self.peek() {
            Some(Token::Ident(_)) => {
                let (name, _) = self.expect_ident()?;
                if name == "_" {
                    return Ok(Pattern::Wildcard);
                }
                // If followed by `(`, it's a variant with payload(s).
                if self.at(&Token::LParen) {
                    self.advance();
                    let mut sub = Vec::new();
                    while !self.at(&Token::RParen) && self.peek().is_some() {
                        sub.push(self.parse_pattern()?);
                        if self.at(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Pattern::Variant(name, sub))
                } else if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // Capital letter → treat as a (nullary) variant.
                    Ok(Pattern::Variant(name, Vec::new()))
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            Some(Token::IntLit(_)) => {
                if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                    let n = *n;
                    self.advance();
                    Ok(Pattern::Literal(Literal::Int(n)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::BoolLit(_)) => {
                if let Some((Token::BoolLit(b), _)) = self.tokens.get(self.pos) {
                    let b = *b;
                    self.advance();
                    Ok(Pattern::Literal(Literal::Bool(b)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::StrLit(_)) => {
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    let s = s.clone();
                    self.advance();
                    Ok(Pattern::Literal(Literal::Str(s)))
                } else {
                    unreachable!()
                }
            }
            Some(tok) => Err(LoomError::parse(
                format!("expected pattern, found {:?}", tok),
                self.current_span(),
            )),
            None => Err(LoomError::parse(
                "expected pattern, found end of input",
                Span::synthetic(),
            )),
        }
    }

    // ── Operator-precedence expressions ───────────────────────────────────

    /// Pipe: `expr |> expr` (left-associative, lowest precedence above let/match).
    fn parse_pipe(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_or()?;
        while self.at(&Token::Pipe) {
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_or()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::Pipe {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_and()?;
        while self.at(&Token::Or) {
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_and()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::BinOp {
                op: BinOpKind::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_comparison()?;
        while self.at(&Token::And) {
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_comparison()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::BinOp {
                op: BinOpKind::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(Token::Eq) => BinOpKind::Eq,
                Some(Token::Ne) => BinOpKind::Ne,
                Some(Token::Lt) => BinOpKind::Lt,
                Some(Token::Le) => BinOpKind::Le,
                Some(Token::Gt) => BinOpKind::Gt,
                Some(Token::Ge) => BinOpKind::Ge,
                _ => break,
            };
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_additive()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOpKind::Add,
                Some(Token::Minus) => BinOpKind::Sub,
                _ => break,
            };
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, LoomError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOpKind::Mul,
                Some(Token::Slash) => BinOpKind::Div,
                _ => break,
            };
            let span_start = self.current_span();
            self.advance();
            let right = self.parse_unary()?;
            let span = Span::merge(&span_start, &self.current_span());
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, LoomError> {
        if self.at(&Token::Not) {
            let span_start = self.current_span();
            self.advance();
            let operand = self.parse_unary()?;
            let span = Span::merge(&span_start, &self.current_span());
            return Ok(Expr::BinOp {
                op: BinOpKind::Eq,
                left: Box::new(operand),
                right: Box::new(Expr::Literal(Literal::Bool(false))),
                span,
            });
        }
        // Unary minus: `-expr` → `0 - expr`
        if self.at(&Token::Minus) {
            let span_start = self.current_span();
            self.advance();
            let operand = self.parse_unary()?;
            let span = Span::merge(&span_start, &self.current_span());
            return Ok(Expr::BinOp {
                op: BinOpKind::Sub,
                left: Box::new(Expr::Literal(Literal::Int(0))),
                right: Box::new(operand),
                span,
            });
        }
        self.parse_postfix()
    }

    /// Parse postfix operations: field access (`e.field`) and function call
    /// (`f(args)`).
    fn parse_postfix(&mut self) -> Result<Expr, LoomError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.at(&Token::Dot) {
                let span_start = self.current_span();
                self.advance();
                let (field, _) = self.expect_ident()?;
                let span = Span::merge(&span_start, &self.current_span());
                expr = Expr::FieldAccess {
                    object: Box::new(expr),
                    field,
                    span,
                };
            } else if self.at(&Token::LParen) {
                let span_start = self.current_span();
                self.advance();
                let mut args = Vec::new();
                while !self.at(&Token::RParen) && self.peek().is_some() {
                    args.push(self.parse_expr()?);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                let end_span = self.current_span();
                self.expect(Token::RParen)?;
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span: Span::merge(&span_start, &end_span),
                };
            } else if self.at(&Token::As) {
                let span_start = self.current_span();
                self.advance(); // consume `as`
                let ty = self.parse_type_expr()?;
                let span = Span::merge(&span_start, &self.current_span());
                expr = Expr::As(Box::new(expr), ty);
            } else if self.at(&Token::Question) {
                let span_start = self.current_span();
                self.advance(); // consume `?`
                let span = Span::merge(&span_start, &self.current_span());
                expr = Expr::Try(Box::new(expr), span);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Parse a primary expression: literal, identifier, or parenthesised expr.
    fn parse_primary(&mut self) -> Result<Expr, LoomError> {
        match self.tokens.get(self.pos) {
            Some((Token::IntLit(n), _)) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Int(n)))
            }
            Some((Token::FloatLit(f), _)) => {
                let f = *f;
                self.advance();
                Ok(Expr::Literal(Literal::Float(f)))
            }
            Some((Token::StrLit(s), _)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal::Str(s)))
            }
            Some((Token::BoolLit(b), _)) => {
                let b = *b;
                self.advance();
                Ok(Expr::Literal(Literal::Bool(b)))
            }
            Some((Token::Ident(_), _)) => {
                let (name, _) = self.expect_ident()?;
                Ok(Expr::Ident(name))
            }
            Some((Token::LParen, _)) => {
                let start = self.current_span();
                self.advance();
                if self.at(&Token::RParen) {
                    self.advance();
                    return Ok(Expr::Literal(Literal::Unit));
                }
                let first = self.parse_expr()?;
                if self.at(&Token::Comma) {
                    // Tuple: (expr, expr, ...)
                    let mut elems = vec![first];
                    while self.at(&Token::Comma) {
                        self.advance();
                        if self.at(&Token::RParen) {
                            break; // trailing comma
                        }
                        elems.push(self.parse_expr()?);
                    }
                    let end = self.current_span();
                    self.expect(Token::RParen)?;
                    Ok(Expr::Tuple(elems, Span::merge(&start, &end)))
                } else {
                    // Parenthesized expression
                    self.expect(Token::RParen)?;
                    Ok(first)
                }
            }
            Some((Token::InlineBlock(_), _)) => {
                if let Some((Token::InlineBlock(content), _)) = self.tokens.get(self.pos) {
                    let content = content.clone();
                    self.advance();
                    Ok(Expr::InlineRust(content))
                } else {
                    unreachable!()
                }
            }
            Some((Token::Bar, _)) => self.parse_lambda(),
            Some((tok, span)) => Err(LoomError::parse(
                format!("unexpected token in expression: {:?}", tok),
                span.clone(),
            )),
            None => Err(LoomError::parse(
                "unexpected end of input in expression",
                Span::synthetic(),
            )),
        }
    }

    /// Parse a lambda expression: `|param: Type, param| body`.
    ///
    /// The opening `|` is consumed here. Params are `name` or `name: Type`.
    /// The closing `|` delimits the param list; the body is a single expression.
    fn parse_lambda(&mut self) -> Result<Expr, LoomError> {
        let start = self.current_span();
        self.expect(Token::Bar)?; // consume opening `|`

        let mut params: Vec<(String, Option<TypeExpr>)> = Vec::new();

        while !self.at(&Token::Bar) && self.peek().is_some() {
            let (name, _) = self.expect_ident()?;
            let ty = if self.at(&Token::Colon) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            params.push((name, ty));
            if self.at(&Token::Comma) {
                self.advance();
            }
        }

        self.expect(Token::Bar)?; // consume closing `|`
        let body = self.parse_expr()?;
        let end_span = self.current_span();

        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M68: Degeneracy block ─────────────────────────────────────────────

    /// Parse `degenerate: primary: X fallback: Y [equivalence_proof: Z] end`.
    fn parse_degenerate_block(&mut self) -> Result<DegenerateBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Degenerate)?;
        self.expect(Token::Colon)?;
        let mut primary = String::new();
        let mut fallback = String::new();
        let mut equivalence_proof: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    let (val, _) = self.expect_ident()?;
                    match key.as_str() {
                        "primary" => primary = val,
                        "fallback" => fallback = val,
                        "equivalence_proof" => equivalence_proof = Some(val),
                        _ => {}
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(DegenerateBlock {
            primary,
            fallback,
            equivalence_proof,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M70: Canalization block ───────────────────────────────────────────

    /// Parse `canalize: toward: X despite: [A, B] [convergence_proof: Z] end`.
    fn parse_canalization_block(&mut self) -> Result<CanalizationBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Canalize)?;
        self.expect(Token::Colon)?;
        let mut toward = String::new();
        let mut despite: Vec<String> = Vec::new();
        let mut convergence_proof: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "toward" => {
                            let (val, _) = self.expect_ident()?;
                            toward = val;
                        }
                        "despite" => {
                            self.expect(Token::LBracket)?;
                            while !self.at(&Token::RBracket) && self.peek().is_some() {
                                if let Ok((v, _)) = self.expect_ident() { despite.push(v); }
                                if self.at(&Token::Comma) { self.advance(); }
                                if self.at(&Token::RBracket) { break; }
                            }
                            self.expect(Token::RBracket)?;
                        }
                        "convergence_proof" => {
                            let (val, _) = self.expect_ident()?;
                            convergence_proof = Some(val);
                        }
                        _ => { let _ = self.expect_ident(); }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CanalizationBlock { toward, despite, convergence_proof, span: Span::merge(&start, &end_span) })
    }

    // ── M71: Pathway ──────────────────────────────────────────────────────

    /// Parse `pathway Name <steps> end`.
    fn parse_pathway_def(&mut self) -> Result<PathwayDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Pathway)?;
        let (name, _) = self.expect_ident()?;
        let mut steps: Vec<PathwayStep> = Vec::new();
        let mut compensate: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            // compensate: X
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "compensate")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // compensate
                self.advance(); // colon
                let (val, _) = self.expect_ident()?;
                compensate = Some(val);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(_), _)))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Minus, _)))
                && matches!(self.tokens.get(self.pos + 2), Some((Token::LBracket, _)))
            {
                // from -[via]-> to
                let step_start = self.current_span();
                let (from, _) = self.expect_ident()?;
                self.expect(Token::Minus)?;
                self.expect(Token::LBracket)?;
                let (via, _) = self.expect_ident()?;
                self.expect(Token::RBracket)?;
                self.expect(Token::Arrow)?;
                let (to, _) = self.expect_ident()?;
                steps.push(PathwayStep { from, via, to, span: step_start });
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(PathwayDef { name, steps, compensate, span: Span::merge(&start, &end_span) })
    }

    // ── M72: Symbiotic import ─────────────────────────────────────────────

    /// Parse `symbiotic: kind: mutualistic|commensal|parasitic module: M`.
    fn parse_symbiotic_import(&mut self) -> Result<Item, LoomError> {
        let start = self.current_span();
        self.advance(); // consume "symbiotic" ident
        self.expect(Token::Colon)?;
        let mut kind = String::new();
        let mut module = String::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    let (val, _) = self.expect_ident()?;
                    match key.as_str() {
                        "kind" => kind = val,
                        "module" => module = val,
                        _ => {}
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        Ok(Item::SymbioticImport { module, kind, span: Span::merge(&start, &end_span) })
    }

    // ── M74: Senescence block ─────────────────────────────────────────────

    /// Parse `senescence: onset: X degradation: Y [sasp: Z] end`.
    fn parse_senescence_block(&mut self) -> Result<SenescenceBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Senescence)?;
        self.expect(Token::Colon)?;
        let mut onset = String::new();
        let mut degradation = String::new();
        let mut sasp: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    let (val, _) = self.expect_ident()?;
                    match key.as_str() {
                        "onset" => onset = val,
                        "degradation" => degradation = val,
                        "sasp" => sasp = Some(val),
                        _ => {}
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(SenescenceBlock { onset, degradation, sasp, span: Span::merge(&start, &end_span) })
    }

    // ── M75: HGT adopt ───────────────────────────────────────────────────

    /// Parse `adopt: Interface from Module`.
    fn parse_adopt_decl(&mut self) -> Result<AdoptDecl, LoomError> {
        let start = self.current_span();
        self.expect(Token::Adopt)?;
        self.expect(Token::Colon)?;
        let (interface, _) = self.expect_ident()?;
        // consume "from" (keyword token)
        self.expect(Token::From)?;
        let (from_module, _) = self.expect_ident()?;
        let end_span = self.current_span();
        Ok(AdoptDecl { interface, from_module, span: Span::merge(&start, &end_span) })
    }

    // ── M76: Criticality block ────────────────────────────────────────────

    /// Parse `criticality: lower: N upper: N [probe_fn: X] end`.
    fn parse_criticality_block(&mut self) -> Result<CriticalityBlock, LoomError> {
        let start = self.current_span();
        self.advance(); // consume "criticality" ident
        self.expect(Token::Colon)?;
        let mut lower = 0.0f64;
        let mut upper = 1.0f64;
        let mut probe_fn: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "lower" => {
                            match self.peek() {
                                Some(Token::FloatLit(_)) => {
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        lower = *f;
                                    }
                                    self.advance();
                                }
                                Some(Token::IntLit(_)) => {
                                    if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                        lower = *n as f64;
                                    }
                                    self.advance();
                                }
                                _ => { let _ = self.expect_ident(); }
                            }
                        }
                        "upper" => {
                            match self.peek() {
                                Some(Token::FloatLit(_)) => {
                                    if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                                        upper = *f;
                                    }
                                    self.advance();
                                }
                                Some(Token::IntLit(_)) => {
                                    if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                                        upper = *n as f64;
                                    }
                                    self.advance();
                                }
                                _ => { let _ = self.expect_ident(); }
                            }
                        }
                        "probe_fn" => {
                            let (val, _) = self.expect_ident()?;
                            probe_fn = Some(val);
                        }
                        _ => { let _ = self.expect_ident(); }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CriticalityBlock { lower, upper, probe_fn, span: Span::merge(&start, &end_span) })
    }

    // ── M77: Niche construction ───────────────────────────────────────────

    /// Parse `niche_construction: modifies: X affects: [A, B] [probe_fn: Z] end`.
    fn parse_niche_construction(&mut self) -> Result<NicheConstructionDef, LoomError> {
        let start = self.current_span();
        self.advance(); // consume "niche_construction" ident
        self.expect(Token::Colon)?;
        let mut modifies = String::new();
        let mut affects: Vec<String> = Vec::new();
        let mut probe_fn: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "modifies" => {
                            let (val, _) = self.expect_ident()?;
                            modifies = val;
                        }
                        "affects" => {
                            self.expect(Token::LBracket)?;
                            while !self.at(&Token::RBracket) && self.peek().is_some() {
                                if let Ok((v, _)) = self.expect_ident() { affects.push(v); }
                                if self.at(&Token::Comma) { self.advance(); }
                                if self.at(&Token::RBracket) { break; }
                            }
                            self.expect(Token::RBracket)?;
                        }
                        "probe_fn" => {
                            let (val, _) = self.expect_ident()?;
                            probe_fn = Some(val);
                        }
                        _ => { let _ = self.expect_ident(); }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(NicheConstructionDef { modifies, affects, probe_fn, span: Span::merge(&start, &end_span) })
    }

    /// M81: Parse `sense Name ... end` top-level item.
    fn parse_sense_def(&mut self) -> Result<SenseDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Sense)?;
        let (name, _) = self.expect_ident()?;
        let mut channels: Vec<String> = Vec::new();
        let mut range: Option<String> = None;
        let mut unit: Option<String> = None;
        let mut dimension: Option<String> = None;
        let mut derived: Option<String> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "channels")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `channels`
                self.advance(); // consume `:`
                self.expect(Token::LBracket)?;
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    if let Ok((val, _)) = self.expect_ident() {
                        channels.push(val);
                    }
                    if self.at(&Token::Comma) { self.advance(); }
                    if self.at(&Token::RBracket) { break; }
                }
                self.expect(Token::RBracket)?;
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "range")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `range`
                self.advance(); // consume `:`
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    range = Some(s.clone());
                    self.pos += 1;
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "unit")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `unit`
                self.advance(); // consume `:`
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    unit = Some(s.clone());
                    self.pos += 1;
                }
            } else if (matches!(self.tokens.get(self.pos), Some((Token::Dimension, _)))
                || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "dimension"))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                // M83: `dimension: Symbol` — SI base dimension symbol
                self.advance(); // consume `dimension` (keyword or ident)
                self.advance(); // consume `:`
                if let Ok((sym, _)) = self.expect_ident() {
                    dimension = Some(sym);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "derived")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                // M83: `derived: Formula` — dimensional formula for derived units
                self.advance(); // consume `derived`
                self.advance(); // consume `:`
                if let Ok((formula, _)) = self.expect_ident() {
                    derived = Some(formula);
                }
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(SenseDef { name, channels, range, unit, dimension, derived, span: Span::merge(&start, &end_span) })
    }

    // ── M92: Store declarations ───────────────────────────────────────────────

    /// Parse a `store Name :: Kind ... end` declaration. M92.
    fn parse_store_def(&mut self) -> Result<StoreDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Store)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let kind = self.parse_store_kind()?;

        let mut schema: Vec<StoreSchemaEntry> = Vec::new();
        let mut config: Vec<StoreConfigEntry> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Table) {
                schema.push(self.parse_store_table_entry()?);
            } else if self.at(&Token::GraphNode) {
                schema.push(self.parse_store_node_entry()?);
            } else if self.at(&Token::Edge) {
                schema.push(self.parse_store_edge_entry()?);
            } else if self.at(&Token::Fact) {
                schema.push(self.parse_store_fact_entry()?);
            } else if self.at(&Token::Dimension) {
                schema.push(self.parse_store_dimension_entry()?);
            } else if self.at(&Token::Embedding) {
                schema.push(self.parse_store_embedding_entry()?);
            } else if self.at(&Token::MapReduce) {
                schema.push(self.parse_store_mapreduce_entry()?);
            } else if self.at(&Token::Consumer) {
                schema.push(self.parse_store_consumer_entry()?);
            } else if self.at(&Token::Ttl) || self.at(&Token::Retention) || self.at(&Token::Resolution)
                || self.at(&Token::Format) || self.at(&Token::Compression) || self.at(&Token::Capacity)
                || self.at(&Token::Eviction) || self.at(&Token::Index)
                || self.at(&Token::Partitions) || self.at(&Token::Replication)
            {
                let entry_span = self.current_span();
                let key = self.token_as_ident().unwrap_or_default();
                self.advance();
                self.expect(Token::Colon)?;
                let value = self.parse_store_config_value()?;
                config.push(StoreConfigEntry { key, value, span: entry_span });
            } else if let Some(kw) = self.token_as_ident() {
                let next_is_colon = matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
                if kw == "key" && next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // "key"
                    self.advance(); // ":"
                    let ty = self.parse_type_expr()?;
                    schema.push(StoreSchemaEntry::KeyType { ty, span: entry_span });
                } else if kw == "value" && next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // "value"
                    self.advance(); // ":"
                    let ty = self.parse_type_expr()?;
                    schema.push(StoreSchemaEntry::ValueType { ty, span: entry_span });
                } else if kw == "event" {
                    let entry_span = self.current_span();
                    self.advance(); // "event"
                    let (ev_name, _) = self.expect_ident()?;
                    self.expect(Token::ColonColon)?;
                    let fields = self.parse_inline_fields()?;
                    schema.push(StoreSchemaEntry::Event { name: ev_name, fields, span: entry_span });
                } else if kw == "schema" {
                    let entry_span = self.current_span();
                    self.advance(); // "schema"
                    let (col_name, _) = self.expect_ident()?;
                    self.expect(Token::ColonColon)?;
                    let fields = self.parse_inline_fields()?;
                    schema.push(StoreSchemaEntry::Collection { name: col_name, fields, span: entry_span });
                } else if next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // key
                    self.advance(); // ":"
                    let value = self.parse_store_config_value()?;
                    config.push(StoreConfigEntry { key: kw, value, span: entry_span });
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StoreDef { name, kind, schema, config, span: Span::merge(&start, &end_span) })
    }

    /// Parse a store kind identifier (e.g. `Relational`, `KeyValue`, `InMemory(Relational)`).
    fn parse_store_kind(&mut self) -> Result<StoreKind, LoomError> {
        let kind_name = if let Some(n) = self.token_as_ident() {
            self.advance();
            n
        } else {
            let (n, _) = self.expect_ident()?;
            n
        };
        let kind = match kind_name.as_str() {
            "Relational"     => StoreKind::Relational,
            "KeyValue"       => StoreKind::KeyValue,
            "Graph"          => StoreKind::Graph,
            "Document"       => StoreKind::Document,
            "Columnar"       => StoreKind::Columnar,
            "Snowflake"      => StoreKind::Snowflake,
            "Hypercube"      => StoreKind::Hypercube,
            "TimeSeries"     => StoreKind::TimeSeries,
            "Vector"         => StoreKind::Vector,
            "FlatFile"       => StoreKind::FlatFile,
            "Distributed"    => StoreKind::Distributed,
            "DistributedLog" => StoreKind::DistributedLog,
            "InMemory"   => {
                if self.at(&Token::LParen) {
                    self.advance();
                    let inner = self.parse_store_kind()?;
                    let _ = self.expect(Token::RParen);
                    StoreKind::InMemory(Box::new(inner))
                } else {
                    StoreKind::InMemory(Box::new(StoreKind::Relational))
                }
            }
            _ => StoreKind::Document,
        };
        Ok(kind)
    }

    /// Parse a `table Name ... end` block inside a store.
    fn parse_store_table_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Table)?;
        let (name, _) = self.expect_ident()?;
        let mut fields = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            let field_start = self.current_span();
            let field_name = match self.tokens.get(self.pos) {
                Some((Token::Ident(n), _)) => { let n = n.clone(); self.pos += 1; n }
                _ => break,
            };
            if !self.at(&Token::Colon) { break; }
            self.advance();
            let ty = self.parse_type_expr()?;
            let annotations = self.parse_annotations();
            let field_end = self.current_span();
            fields.push(FieldDef { name: field_name, ty, annotations, span: Span::merge(&field_start, &field_end) });
            if self.at(&Token::Comma) { self.advance(); }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StoreSchemaEntry::Table { name, fields, span: Span::merge(&start, &end_span) })
    }

    /// Parse a `node Name :: { field: Type, ... }` entry.
    fn parse_store_node_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::GraphNode)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::Node { name, fields, span: Span::merge(&start, &self.current_span()) })
    }

    /// Parse an `edge Name :: Source -> Target [{ fields }]` entry.
    fn parse_store_edge_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Edge)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let (source, _) = self.expect_ident()?;
        self.expect(Token::Arrow)?;
        let (target, _) = self.expect_ident()?;
        let fields = if self.at(&Token::LBrace) {
            self.parse_inline_fields()?
        } else {
            Vec::new()
        };
        Ok(StoreSchemaEntry::Edge { name, source, target, fields, span: Span::merge(&start, &self.current_span()) })
    }

    /// Parse a `fact Name :: { field: Type, ... }` entry.
    fn parse_store_fact_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Fact)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::Fact { name, fields, span: Span::merge(&start, &self.current_span()) })
    }

    /// Parse a `dimension Name :: { field: Type, ... }` entry.
    fn parse_store_dimension_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Dimension)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::DimensionEntry { name, fields, span: Span::merge(&start, &self.current_span()) })
    }

    /// Parse an `embedding :: { field: Type, ... }` entry.
    fn parse_store_embedding_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Embedding)?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::EmbeddingEntry { fields, span: Span::merge(&start, &self.current_span()) })
    }

    /// Parse a `mapreduce Name ... end` block inside a Distributed store.
    fn parse_store_mapreduce_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.advance(); // consume `mapreduce`
        let (name, _) = self.expect_ident()?;

        let mut map_sig = String::new();
        let mut reduce_sig = String::new();
        let mut combine_sig: Option<String> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(kw) = self.token_as_ident() {
                let next_is_colon = matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
                if next_is_colon {
                    self.advance(); // consume key
                    self.advance(); // consume ':'
                    let sig = self.parse_mapreduce_sig_as_string();
                    match kw.as_str() {
                        "map"     => map_sig = sig,
                        "reduce"  => reduce_sig = sig,
                        "combine" => combine_sig = Some(sig),
                        _         => {}
                    }
                    continue;
                }
            }
            self.advance();
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StoreSchemaEntry::MapReduceJob(MapReduceDef {
            name,
            map_sig,
            reduce_sig,
            combine_sig,
            span: Span::merge(&start, &end_span),
        }))
    }

    /// Parse a `consumer Name :: offset: value` entry inside a DistributedLog store.
    fn parse_store_consumer_entry(&mut self) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.advance(); // consume `consumer`
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        // Expect `offset: value`
        let _ = self.token_as_ident(); // should be "offset"
        self.advance(); // consume `offset`
        self.expect(Token::Colon)?;
        let offset = self.parse_value_as_string().unwrap_or_default();
        Ok(StoreSchemaEntry::LogConsumer(LogConsumerDef {
            name,
            offset,
            span: start,
        }))
    }

    /// Collect tokens as a display string until the next `map:`/`reduce:`/`combine:` or `end`.
    fn parse_mapreduce_sig_as_string(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            if self.at(&Token::End) { break; }
            // Stop when we see map:/reduce:/combine: starting the next entry
            if let Some(kw) = self.token_as_ident() {
                if matches!(kw.as_str(), "map" | "reduce" | "combine") {
                    if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                        break;
                    }
                }
            }
            let s = self.token_as_display_string();
            parts.push(s);
            self.advance();
        }
        parts.join("")
    }

    /// Return the current token as a display string for signature capture.
    fn token_as_display_string(&self) -> String {
        match self.tokens.get(self.pos) {
            Some((Token::Arrow, _))      => "->".to_string(),
            Some((Token::LBracket, _))   => "[".to_string(),
            Some((Token::RBracket, _))   => "]".to_string(),
            Some((Token::LParen, _))     => "(".to_string(),
            Some((Token::RParen, _))     => ")".to_string(),
            Some((Token::Comma, _))      => ",".to_string(),
            Some((Token::Star, _))       => "*".to_string(),
            Some((Token::ColonColon, _)) => "::".to_string(),
            Some((Token::Colon, _))      => ":".to_string(),
            Some((Token::IntLit(n), _))  => n.to_string(),
            Some((Token::FloatLit(f), _)) => f.to_string(),
            _ => self.token_as_ident().unwrap_or_else(|| "_".to_string()),
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
            if !self.at(&Token::Colon) { break; }
            self.advance();
            let ty = self.parse_type_expr()?;
            let mut annotations = pre_annotations;
            annotations.extend(self.parse_annotations());
            let field_end = self.current_span();
            fields.push(FieldDef { name: field_name, ty, annotations, span: Span::merge(&field_start, &field_end) });
            if self.at(&Token::Comma) { self.advance(); }
        }
        self.expect(Token::RBrace)?;
        Ok(fields)
    }

    /// Parse a store config value — handles `90days`, `1second`, idents, strings.
    fn parse_store_config_value(&mut self) -> Result<String, LoomError> {
        match self.tokens.get(self.pos) {
            Some((Token::IntLit(n), _)) => {
                let n = n.to_string();
                self.pos += 1;
                if let Some((Token::Ident(suffix), _)) = self.tokens.get(self.pos) {
                    let suffix = suffix.clone();
                    self.pos += 1;
                    Ok(format!("{}{}", n, suffix))
                } else {
                    Ok(n)
                }
            }
            _ => self.parse_value_as_string(),
        }
    }

    /// Parse the rank (integer literal) in `Tensor<rank, ...>`. M87.
    ///
    /// Expects `Token::IntLit(n)` and returns `n as usize`.
    fn parse_tensor_rank(&mut self) -> Result<usize, LoomError> {
        match self.tokens.get(self.pos) {
            Some((Token::IntLit(n), _)) => {
                let rank = *n as usize;
                self.pos += 1;
                Ok(rank)
            }
            Some((_, span)) => Err(LoomError::parse(
                "expected integer literal for tensor rank",
                span.clone(),
            )),
            None => Err(LoomError::parse(
                "expected integer literal for tensor rank, found end of input",
                Span::synthetic(),
            )),
        }
    }

    /// Parse the shape list `[D1, D2, ...]` in `Tensor<rank, [shape], unit>`. M87.
    ///
    /// Accepts comma-separated identifiers or integer literals inside brackets.
    /// An empty bracket pair `[]` is valid (scalar tensor, rank 0).
    fn parse_tensor_shape(&mut self) -> Result<Vec<String>, LoomError> {
        self.expect(Token::LBracket)?;
        let mut shape = Vec::new();
        while !self.at(&Token::RBracket) && self.peek().is_some() {
            match self.tokens.get(self.pos) {
                Some((Token::Ident(n), _)) => {
                    shape.push(n.clone());
                    self.pos += 1;
                }
                Some((Token::IntLit(n), _)) => {
                    shape.push(n.to_string());
                    self.pos += 1;
                }
                _ => break,
            }
            if self.at(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RBracket)?;
        Ok(shape)
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
        Token::Comma => ", ".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::Star => "*".to_string(),
        Token::Question => "?".to_string(),
        _ => format!("{:?}", tok),
    }
}

/// If `tok` is a keyword that could serve as an identifier (e.g. a field name),
/// return its source spelling; otherwise return `None`.
fn token_keyword_str(tok: &Token) -> Option<&'static str> {
    match tok {
        Token::Threshold    => Some("threshold"),
        Token::Limit        => Some("limit"),
        Token::Produces     => Some("produces"),
        Token::Modifies     => Some("modifies"),
        Token::RevertsWhen  => Some("reverts_when"),
        Token::OnExhaustion => Some("on_exhaustion"),
        Token::Signal       => Some("signal"),
        Token::Payload      => Some("payload"),
        Token::From         => Some("from"),
        Token::To           => Some("to"),
        Token::Toward       => Some("toward"),
        Token::Bounds       => Some("bounds"),
        Token::Members      => Some("members"),
        Token::Fitness      => Some("fitness"),
        Token::Telos        => Some("telos"),
        Token::Form         => Some("form"),
        Token::Matter       => Some("matter"),
        Token::Regulate     => Some("regulate"),
        Token::Evolve       => Some("evolve"),
        Token::Degenerate   => Some("degenerate"),
        Token::Fallback     => Some("fallback"),
        Token::Checkpoint   => Some("checkpoint"),
        Token::Canalize     => Some("canalize"),
        Token::Pathway      => Some("pathway"),
        Token::Senescence   => Some("senescence"),
        Token::Store       => Some("store"),
        Token::Table       => Some("table"),
        Token::GraphNode   => Some("node"),
        Token::Edge        => Some("edge"),
        Token::Ttl         => Some("ttl"),
        Token::Index       => Some("index"),
        Token::Retention   => Some("retention"),
        Token::Resolution  => Some("resolution"),
        Token::Format      => Some("format"),
        Token::Compression => Some("compression"),
        Token::Capacity    => Some("capacity"),
        Token::Eviction    => Some("eviction"),
        Token::Fact        => Some("fact"),
        Token::Dimension   => Some("dimension"),
        Token::Embedding   => Some("embedding"),
        Token::Adopt        => Some("adopt"),
        _                   => None,
    }
}
