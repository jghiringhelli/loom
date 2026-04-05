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
}

impl<'src> Parser<'src> {
    /// Create a new parser for the given token slice.
    pub fn new(tokens: &'src [(Token, Span)]) -> Self {
        Parser { tokens, pos: 0 }
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

    // ── Top-level ─────────────────────────────────────────────────────────

    /// Parse a complete `module … end` block.
    pub fn parse_module(&mut self) -> Result<Module, LoomError> {
        let start = self.current_span();
        self.expect(Token::Module)?;
        let (name, _) = self.expect_ident()?;

        // Optional `spec NAME`
        let spec = if self.at(&Token::Spec) {
            self.advance();
            let (spec_name, _) = self.expect_ident()?;
            Some(spec_name)
        } else {
            None
        };

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

        // Item list until `end`
        let mut items = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            items.push(self.parse_item()?);
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(Module {
            name,
            spec,
            provides,
            requires,
            items,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse the `{ name :: type_sig, … }` block following `provides`.
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

    /// Parse `fn NAME[<A, B>] :: type_sig [require: expr]* [ensure: expr]* body* end`.
    pub fn parse_fn_def(&mut self) -> Result<FnDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Fn)?;
        let (name, _) = self.expect_ident()?;

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
            type_params,
            type_sig,
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
            let (field_name, _) = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type_expr()?;
            fields.push((field_name, ty));
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
                // `Effect<[E1, E2, ...], ReturnType>`
                self.expect(Token::LBracket)?;
                let mut effects = Vec::new();
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    let (eff, _) = self.expect_ident()?;
                    effects.push(eff);
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
                self.expect(Token::Comma)?;
                let inner = self.parse_type_expr()?;
                self.expect(Token::Gt)?;
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
            // NOTE: `not` binds tighter than comparison operators.
            // `not x = ""` parses as `(not x) = ""`, NOT `not (x = "")`.
            // To negate a comparison write: `not (x = "")` using parentheses
            // (not yet supported — TODO Phase 4 parenthesised expressions).
            // This means `require: not name = ""` should be written
            // as a plain Boolean expression or an explicit `false` comparison.
            let operand = self.parse_unary()?;
            let span = Span::merge(&span_start, &self.current_span());
            // Represent `not e` as `e == false`.
            return Ok(Expr::BinOp {
                op: BinOpKind::Eq,
                left: Box::new(operand),
                right: Box::new(Expr::Literal(Literal::Bool(false))),
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
