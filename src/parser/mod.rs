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
                // Optional `("value")` payload
                let value = if self.at(&Token::LParen) {
                    self.advance();
                    if let Some((Token::StrLit(v), _)) = self.tokens.get(self.pos) {
                        let v = v.clone();
                        self.advance();
                        let _ = self.expect(Token::RParen); // consume `)`, ignore error
                        v
                    } else {
                        let _ = self.expect(Token::RParen);
                        String::new()
                    }
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
        let end_span = self.current_span();
        Ok(LifecycleDef {
            type_name,
            states,
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
                let fitness_fn = if matches!(self.tokens.get(self.pos), Some((Token::Fitness, _))) {
                    self.advance(); // consume `fitness`
                    self.expect(Token::Colon)?;
                    let mut parts = Vec::new();
                    while !self.at(&Token::End) && self.peek().is_some() {
                        if let Some((tok, _)) = self.tokens.get(self.pos) {
                            parts.push(format!("{:?}", tok));
                            self.pos += 1;
                        }
                    }
                    Some(parts.join(" "))
                } else {
                    None
                };
                let modifiable_by = if matches!(self.tokens.get(self.pos), Some((Token::ModifiableBy, _))) {
                    self.advance(); // consume `modifiable_by`
                    self.expect(Token::Colon)?;
                    if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                        let val = val.clone();
                        self.pos += 1;
                        Some(val)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let bounded_by = if matches!(self.tokens.get(self.pos), Some((Token::BoundedBy, _))) {
                    self.advance(); // consume `bounded_by`
                    self.expect(Token::Colon)?;
                    if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                        let val = val.clone();
                        self.pos += 1;
                        Some(val)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let sec_end = self.current_span();
                self.expect(Token::End)?;
                telos = Some(TelosDef { description, fitness_fn, modifiable_by, bounded_by, span: Span::merge(&sec_start, &sec_end) });
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
                            // consume optional `when`
                            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "when") {
                                self.advance();
                            }
                            let when = if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                                let w = n.clone();
                                self.pos += 1;
                                w
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
                        let (val, _) = self.expect_ident()?;
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
                // Distinguish `type Name = base where pred` (refined) from
                // `type Name = field: T, … end` (product type).
                Ok(self.parse_type_or_refined()?)
            }
            Some(Token::Enum) => Ok(Item::Enum(self.parse_enum_def()?)),
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
        let (name, _) = self.expect_ident()?;

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

        // Collect `require:` / `ensure:` / `with` clauses.
        loop {
            if self.at(&Token::Require) {
                requires.push(self.parse_contract()?);
            } else if self.at(&Token::Ensure) {
                ensures.push(self.parse_contract()?);
            } else if self.at(&Token::With) {
                self.advance();
                let (dep, _) = self.expect_ident()?;
                with_deps.push(dep);
            } else {
                break;
            }
        }

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
        // If next is `end` it's an empty product type.
        // Otherwise parse fields.
        let is_refined = match (self.peek(), self.peek2()) {
            (Some(Token::Ident(_)), Some(Token::Where)) => true,
            _ => false,
        };

        if is_refined {
            let base_type = self.parse_type_expr()?;
            self.expect(Token::Where)?;
            let predicate = self.parse_expr()?;
            let end_span = self.current_span();
            Ok(Item::RefinedType(RefinedType {
                name,
                base_type,
                predicate,
                span: Span::merge(&start, &end_span),
            }))
        } else {
            Ok(Item::Type(self.parse_type_fields(name, start)?))
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
    /// `Result<T, E>`, and arbitrary generics `Name<T...>`.
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
        _                   => None,
    }
}
