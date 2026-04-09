use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

impl<'src> crate::parser::Parser<'src> {
    // ── Type / refined type ───────────────────────────────────────────────

    /// Decide between a refined type (`type E = String where pred`) and a
    /// product type (`type Point = x: Float, y: Float end`).
    pub(in crate::parser) fn parse_type_or_refined(&mut self) -> Result<Item, LoomError> {
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
            (Some(Token::End), _) => true, // empty product type
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
    pub(in crate::parser) fn parse_type_fields(
        &mut self,
        name: String,
        start: Span,
    ) -> Result<TypeDef, LoomError> {
        let mut fields = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            let field_start = self.current_span();
            // Parse optional field-level annotations before the field name (@pii, @gdpr, etc.)
            let leading_annotations = self.parse_annotations();
            if self.at(&Token::End) {
                break;
            }
            let (field_name, _) = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type_expr()?;
            // Also accept trailing annotations after the type expression.
            let mut trailing_annotations = self.parse_annotations();
            let mut annotations = leading_annotations;
            annotations.append(&mut trailing_annotations);
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
    pub(in crate::parser) fn parse_enum_variant(&mut self) -> Result<EnumVariant, LoomError> {
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
    pub(in crate::parser) fn parse_generic_tail(
        &mut self,
        name: String,
    ) -> Result<TypeExpr, LoomError> {
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
                                "pure" => ConsequenceTier::Pure,
                                "reversible" => ConsequenceTier::Reversible,
                                "irreversible" => ConsequenceTier::Irreversible,
                                _ => ConsequenceTier::Irreversible,
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
                Ok(TypeExpr::Tensor {
                    rank,
                    shape,
                    unit: Box::new(unit),
                    span,
                })
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
}
