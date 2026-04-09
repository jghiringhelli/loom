use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

impl<'src> crate::parser::Parser<'src> {
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
    pub(in crate::parser) fn parse_for_in(&mut self) -> Result<Expr, LoomError> {
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
    pub(in crate::parser) fn parse_let(&mut self) -> Result<Expr, LoomError> {
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
    pub(in crate::parser) fn parse_pattern(&mut self) -> Result<Pattern, LoomError> {
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
    pub(in crate::parser) fn parse_pipe(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_or(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_and(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_comparison(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_additive(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_multiplicative(&mut self) -> Result<Expr, LoomError> {
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

    pub(in crate::parser) fn parse_unary(&mut self) -> Result<Expr, LoomError> {
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
    pub(in crate::parser) fn parse_postfix(&mut self) -> Result<Expr, LoomError> {
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
    pub(in crate::parser) fn parse_primary(&mut self) -> Result<Expr, LoomError> {
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
            // Keywords that can appear as expression identifiers (function call targets, etc.)
            Some((Token::Process, _)) => {
                self.advance();
                Ok(Expr::Ident("process".to_string()))
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
    pub(in crate::parser) fn parse_lambda(&mut self) -> Result<Expr, LoomError> {
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
