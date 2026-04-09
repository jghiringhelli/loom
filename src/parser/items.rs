use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

impl<'src> crate::parser::Parser<'src> {
    /// Parse `invariant NAME :: bool_expr`.
    pub(in crate::parser) fn parse_invariant(&mut self) -> Result<Invariant, LoomError> {
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
    pub(in crate::parser) fn parse_test_def(&mut self) -> Result<TestDef, LoomError> {
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
    pub(in crate::parser) fn parse_interface_def(&mut self) -> Result<InterfaceDef, LoomError> {
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
    pub(in crate::parser) fn parse_lifecycle_def(&mut self) -> Result<LifecycleDef, LoomError> {
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
    pub(in crate::parser) fn parse_temporal_def(&mut self) -> Result<TemporalDef, LoomError> {
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
    pub(in crate::parser) fn parse_separation_block(
        &mut self,
    ) -> Result<SeparationBlock, LoomError> {
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
    pub(in crate::parser) fn parse_gradual_block(&mut self) -> Result<GradualBlock, LoomError> {
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
                            "input_type" => input_type = Some(val),
                            "boundary" => boundary = Some(val),
                            "output_type" => output_type = Some(val),
                            "on_cast_failure" => on_cast_failure = Some(val),
                            "blame" => blame = Some(val),
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
        Ok(GradualBlock {
            input_type,
            boundary,
            output_type,
            on_cast_failure,
            blame,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `distribution:` block.
    pub(in crate::parser) fn parse_distribution_block(
        &mut self,
    ) -> Result<DistributionBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Distribution)?;
        self.expect(Token::Colon)?;
        let mut model = String::new();
        let mut mean: Option<String> = None;
        let mut variance: Option<String> = None;
        let mut bounds: Option<String> = None;
        let mut convergence: Option<String> = None;
        let mut stability: Option<String> = None;
        let mut explicit_family: Option<DistributionFamily> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((tok, _)) = self.tokens.get(self.pos + 1) {
                    if matches!(tok, crate::lexer::Token::Colon) {
                        self.advance(); // key
                        self.advance(); // colon
                        if key == "family" {
                            explicit_family = Some(self.parse_distribution_family()?);
                        } else {
                            let val = self.parse_value_as_string()?;
                            match key.as_str() {
                                "model" => model = val,
                                "mean" => mean = Some(val),
                                "variance" => variance = Some(val),
                                "bounds" => bounds = Some(val),
                                "convergence" => convergence = Some(val),
                                "stability" => stability = Some(val),
                                _ => {}
                            }
                        }
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        let family = explicit_family.unwrap_or_else(|| {
            Self::family_from_model_string(&model, mean.as_deref(), variance.as_deref())
        });
        Ok(DistributionBlock {
            family,
            model,
            mean,
            variance,
            bounds,
            convergence,
            stability,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Derive a `DistributionFamily` from the legacy `model:` string.
    pub(in crate::parser) fn family_from_model_string(
        model: &str,
        mean: Option<&str>,
        variance: Option<&str>,
    ) -> DistributionFamily {
        match model.to_lowercase().as_str() {
            "gaussian" | "normal" => DistributionFamily::Gaussian {
                mean: mean.unwrap_or("0").to_string(),
                std_dev: variance.unwrap_or("1").to_string(),
            },
            "poisson" => DistributionFamily::Poisson {
                lambda: "1".to_string(),
            },
            "beta" => DistributionFamily::Beta {
                alpha: "1".to_string(),
                beta: "1".to_string(),
            },
            "gamma" => DistributionFamily::Gamma {
                shape: "1".to_string(),
                scale: "1".to_string(),
            },
            "exponential" => DistributionFamily::Exponential {
                lambda: "1".to_string(),
            },
            "binomial" => DistributionFamily::Binomial {
                n: "1".to_string(),
                p: "0.5".to_string(),
            },
            "pareto" => DistributionFamily::Pareto {
                alpha: "1".to_string(),
                x_min: "1".to_string(),
            },
            "cauchy" => DistributionFamily::Cauchy {
                location: "0".to_string(),
                scale: "1".to_string(),
            },
            "levy" => DistributionFamily::Levy {
                location: "0".to_string(),
                scale: "1".to_string(),
            },
            "lognormal" | "log_normal" => DistributionFamily::LogNormal {
                mean: mean.unwrap_or("0").to_string(),
                std_dev: variance.unwrap_or("1").to_string(),
            },
            "uniform" => DistributionFamily::Uniform {
                low: "0".to_string(),
                high: "1".to_string(),
            },
            "geometricbrownian" | "geometric_brownian" => DistributionFamily::GeometricBrownian {
                drift: "0".to_string(),
                volatility: "1".to_string(),
            },
            _ => DistributionFamily::Unknown(model.to_string()),
        }
    }

    /// Parse a distribution family expression: `FamilyName(key: value, ...)`.
    pub(in crate::parser) fn parse_distribution_family(
        &mut self,
    ) -> Result<DistributionFamily, LoomError> {
        let name = if let Some(n) = self.token_as_ident() {
            self.advance();
            n
        } else {
            return Err(LoomError::parse(
                "expected distribution family name".to_string(),
                self.current_span(),
            ));
        };
        let mut params: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if self.at(&Token::LParen) {
            self.advance(); // (
            while !self.at(&Token::RParen) && self.peek().is_some() {
                let key = if let Some(k) = self.token_as_ident() {
                    self.advance();
                    k
                } else {
                    break;
                };
                self.expect(Token::Colon)?;
                let val = self.parse_value_as_string()?;
                params.insert(key, val);
                if self.at(&Token::Comma) {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
        }
        let family = match name.as_str() {
            "Gaussian" | "Normal" => DistributionFamily::Gaussian {
                mean: params.remove("mean").unwrap_or_else(|| "0".to_string()),
                std_dev: params.remove("std_dev").unwrap_or_else(|| "1".to_string()),
            },
            "Poisson" => DistributionFamily::Poisson {
                lambda: params.remove("lambda").unwrap_or_else(|| "1".to_string()),
            },
            "Beta" => DistributionFamily::Beta {
                alpha: params.remove("alpha").unwrap_or_else(|| "1".to_string()),
                beta: params.remove("beta").unwrap_or_else(|| "1".to_string()),
            },
            "Dirichlet" => {
                let alpha_str = params.remove("alpha").unwrap_or_default();
                let alpha = if alpha_str.is_empty() {
                    vec![]
                } else {
                    alpha_str.split(',').map(|s| s.trim().to_string()).collect()
                };
                DistributionFamily::Dirichlet { alpha }
            }
            "Gamma" => DistributionFamily::Gamma {
                shape: params.remove("shape").unwrap_or_else(|| "1".to_string()),
                scale: params.remove("scale").unwrap_or_else(|| "1".to_string()),
            },
            "Exponential" => DistributionFamily::Exponential {
                lambda: params.remove("lambda").unwrap_or_else(|| "1".to_string()),
            },
            "Binomial" => DistributionFamily::Binomial {
                n: params.remove("n").unwrap_or_else(|| "1".to_string()),
                p: params.remove("p").unwrap_or_else(|| "0.5".to_string()),
            },
            "Pareto" => DistributionFamily::Pareto {
                alpha: params.remove("alpha").unwrap_or_else(|| "1".to_string()),
                x_min: params.remove("x_min").unwrap_or_else(|| "1".to_string()),
            },
            "Cauchy" => DistributionFamily::Cauchy {
                location: params.remove("location").unwrap_or_else(|| "0".to_string()),
                scale: params.remove("scale").unwrap_or_else(|| "1".to_string()),
            },
            "Levy" => DistributionFamily::Levy {
                location: params.remove("location").unwrap_or_else(|| "0".to_string()),
                scale: params.remove("scale").unwrap_or_else(|| "1".to_string()),
            },
            "LogNormal" => DistributionFamily::LogNormal {
                mean: params.remove("mean").unwrap_or_else(|| "0".to_string()),
                std_dev: params.remove("std_dev").unwrap_or_else(|| "1".to_string()),
            },
            "Uniform" => DistributionFamily::Uniform {
                low: params.remove("low").unwrap_or_else(|| "0".to_string()),
                high: params.remove("high").unwrap_or_else(|| "1".to_string()),
            },
            "GeometricBrownian" => DistributionFamily::GeometricBrownian {
                drift: params.remove("drift").unwrap_or_else(|| "0".to_string()),
                volatility: params
                    .remove("volatility")
                    .unwrap_or_else(|| "1".to_string()),
            },
            other => DistributionFamily::Unknown(other.to_string()),
        };
        Ok(family)
    }

    /// Parse `process:` block (M88: stochastic process type).
    ///
    /// ```text
    /// process:
    ///   kind: GeometricBrownian
    ///   always_positive: true
    ///   martingale: false
    /// end
    /// ```
    pub(in crate::parser) fn parse_stochastic_process_block(
        &mut self,
    ) -> Result<StochasticProcessBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Process)?;
        self.expect(Token::Colon)?;
        let mut kind = StochasticKind::Unknown(String::new());
        let mut always_positive: Option<bool> = None;
        let mut martingale: Option<bool> = None;
        let mut mean_reverting: Option<bool> = None;
        let mut long_run_mean: Option<String> = None;
        let mut rate: Option<String> = None;
        let mut integer_valued: Option<bool> = None;
        let mut states: Vec<String> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(key) = self.token_as_ident() {
                if let Some((crate::lexer::Token::Colon, _)) = self.tokens.get(self.pos + 1) {
                    self.advance(); // key
                    self.advance(); // colon
                    match key.as_str() {
                        "kind" => {
                            let val = self.parse_value_as_string()?;
                            kind = match val.as_str() {
                                "Wiener" => StochasticKind::Wiener,
                                "GeometricBrownian" => StochasticKind::GeometricBrownian,
                                "OrnsteinUhlenbeck" => StochasticKind::OrnsteinUhlenbeck,
                                "PoissonProcess" => StochasticKind::PoissonProcess,
                                "MarkovChain" => StochasticKind::MarkovChain,
                                other => StochasticKind::Unknown(other.to_string()),
                            };
                        }
                        "always_positive" => {
                            let val = self.parse_value_as_string()?;
                            always_positive = Some(val == "true");
                        }
                        "martingale" => {
                            let val = self.parse_value_as_string()?;
                            martingale = Some(val == "true");
                        }
                        "mean_reverting" => {
                            let val = self.parse_value_as_string()?;
                            mean_reverting = Some(val == "true");
                        }
                        "long_run_mean" => {
                            long_run_mean = Some(self.parse_value_as_string()?);
                        }
                        "rate" => {
                            rate = Some(self.parse_value_as_string()?);
                        }
                        "integer_valued" => {
                            let val = self.parse_value_as_string()?;
                            integer_valued = Some(val == "true");
                        }
                        "states" => {
                            let raw = self.parse_value_as_string()?;
                            let trimmed = raw.trim_start_matches('[').trim_end_matches(']');
                            states = trimmed
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                        _ => {
                            let _ = self.parse_value_as_string()?;
                        }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StochasticProcessBlock {
            kind,
            always_positive,
            martingale,
            mean_reverting,
            long_run_mean,
            rate,
            integer_valued,
            states,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `timing_safety:` block.
    pub(in crate::parser) fn parse_timing_safety_block(
        &mut self,
    ) -> Result<TimingSafetyBlock, LoomError> {
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
                            "leaks_bits" => leaks_bits = Some(val),
                            "method" => method = Some(val),
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
        Ok(TimingSafetyBlock {
            constant_time,
            leaks_bits,
            method,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `proposition NAME = TypeExpr [where expr]`.
    pub(in crate::parser) fn parse_proposition_def(&mut self) -> Result<PropositionDef, LoomError> {
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
        Ok(PropositionDef {
            name,
            base_type,
            predicate,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `functor NAME<TypeParams> [law: name]* end`.
    pub(in crate::parser) fn parse_functor_def(&mut self) -> Result<FunctorDef, LoomError> {
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
                laws.push(LawDecl {
                    name: law_name,
                    span: law_span,
                });
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(FunctorDef {
            name,
            type_params,
            laws,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `monad NAME<TypeParams> [law: name]* end`.
    pub(in crate::parser) fn parse_monad_def(&mut self) -> Result<MonadDef, LoomError> {
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
                laws.push(LawDecl {
                    name: law_name,
                    span: law_span,
                });
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(MonadDef {
            name,
            type_params,
            laws,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse `certificate: field = value ... end`.
    pub(in crate::parser) fn parse_certificate_def(&mut self) -> Result<CertificateDef, LoomError> {
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
                        fields.push(CertificateField {
                            name: key,
                            value: val,
                            span: field_span,
                        });
                        continue;
                    }
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CertificateDef {
            fields,
            span: Span::merge(&start, &end_span),
        })
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
    pub(in crate::parser) fn parse_aspect_def(&mut self) -> Result<AspectDef, LoomError> {
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
    pub(in crate::parser) fn parse_pointcut_expr(&mut self) -> Result<PointcutExpr, LoomError> {
        self.expect(Token::Fn)?;
        self.expect(Token::Where)?;
        self.parse_pointcut_condition()
    }

    /// Parse the condition part of a pointcut expression (after `fn where`).
    pub(in crate::parser) fn parse_pointcut_condition(
        &mut self,
    ) -> Result<PointcutExpr, LoomError> {
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
    pub(in crate::parser) fn parse_pointcut_atom(&mut self) -> Result<PointcutExpr, LoomError> {
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

    /// Parse an `ecosystem Name … end` block.
    pub(in crate::parser) fn parse_ecosystem_def(&mut self) -> Result<EcosystemDef, LoomError> {
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
                                parts.push(super::token_to_source(tok));
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
                    _ => {
                        return Err(LoomError::parse(
                            "expected string literal after telos:",
                            self.current_span(),
                        ))
                    }
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
                            _ => {
                                self.advance();
                            }
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
        let mut stochastic_process: Option<StochasticProcessBlock> = None;
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
            } else if self.at(&Token::Process)
                && matches!(
                    self.tokens.get(self.pos + 1),
                    Some((crate::lexer::Token::Colon, _))
                )
            {
                stochastic_process = Some(self.parse_stochastic_process_block()?);
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
                proofs.push(ProofAnnotation {
                    strategy,
                    span: start_proof,
                });
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

        // M99: Optional handle block — `handle computation with ... end`.
        let handle_block = if self.at(&Token::Handle) {
            Some(self.parse_handle_block()?)
        } else {
            None
        };

        // Body expressions until `end`.
        let mut body = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            // Allow `describe: "..."` anywhere in a fn body (idiomatic Loom style).
            if self.parse_describe().is_some() {
                continue;
            }
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
            stochastic_process,
            handle_block,
            body,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M71: Pathway ──────────────────────────────────────────────────────

    /// Parse `pathway Name <steps> end`.
    pub(in crate::parser) fn parse_pathway_def(&mut self) -> Result<PathwayDef, LoomError> {
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
                steps.push(PathwayStep {
                    from,
                    via,
                    to,
                    span: step_start,
                });
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(PathwayDef {
            name,
            steps,
            compensate,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M72: Symbiotic import ─────────────────────────────────────────────

    /// Parse `symbiotic: kind: mutualistic|commensal|parasitic module: M`.
    pub(in crate::parser) fn parse_symbiotic_import(&mut self) -> Result<Item, LoomError> {
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
        Ok(Item::SymbioticImport {
            module,
            kind,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M92: Store declarations ───────────────────────────────────────────────

    /// Parse a `store Name :: Kind ... end` declaration. M92.
    pub(in crate::parser) fn parse_store_def(&mut self) -> Result<StoreDef, LoomError> {
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
            } else if self.at(&Token::Ttl)
                || self.at(&Token::Retention)
                || self.at(&Token::Resolution)
                || self.at(&Token::Format)
                || self.at(&Token::Compression)
                || self.at(&Token::Capacity)
                || self.at(&Token::Eviction)
                || self.at(&Token::Index)
                || self.at(&Token::Partitions)
                || self.at(&Token::Replication)
            {
                let entry_span = self.current_span();
                let key = self.token_as_ident().unwrap_or_default();
                self.advance();
                self.expect(Token::Colon)?;
                let value = self.parse_store_config_value()?;
                config.push(StoreConfigEntry {
                    key,
                    value,
                    span: entry_span,
                });
            } else if let Some(kw) = self.token_as_ident() {
                let next_is_colon =
                    matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
                if kw == "key" && next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // "key"
                    self.advance(); // ":"
                    let ty = self.parse_type_expr()?;
                    schema.push(StoreSchemaEntry::KeyType {
                        ty,
                        span: entry_span,
                    });
                } else if kw == "value" && next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // "value"
                    self.advance(); // ":"
                    let ty = self.parse_type_expr()?;
                    schema.push(StoreSchemaEntry::ValueType {
                        ty,
                        span: entry_span,
                    });
                } else if kw == "event" {
                    let entry_span = self.current_span();
                    self.advance(); // "event"
                    let (ev_name, _) = self.expect_ident()?;
                    self.expect(Token::ColonColon)?;
                    let fields = self.parse_inline_fields()?;
                    schema.push(StoreSchemaEntry::Event {
                        name: ev_name,
                        fields,
                        span: entry_span,
                    });
                } else if kw == "schema" {
                    let entry_span = self.current_span();
                    self.advance(); // "schema"
                    let (col_name, _) = self.expect_ident()?;
                    self.expect(Token::ColonColon)?;
                    let fields = self.parse_inline_fields()?;
                    schema.push(StoreSchemaEntry::Collection {
                        name: col_name,
                        fields,
                        span: entry_span,
                    });
                } else if next_is_colon {
                    let entry_span = self.current_span();
                    self.advance(); // key
                    self.advance(); // ":"
                    let value = self.parse_store_config_value()?;
                    config.push(StoreConfigEntry {
                        key: kw,
                        value,
                        span: entry_span,
                    });
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StoreDef {
            name,
            kind,
            schema,
            config,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a store kind identifier (e.g. `Relational`, `KeyValue`, `InMemory(Relational)`).
    pub(in crate::parser) fn parse_store_kind(&mut self) -> Result<StoreKind, LoomError> {
        let kind_name = if let Some(n) = self.token_as_ident() {
            self.advance();
            n
        } else {
            let (n, _) = self.expect_ident()?;
            n
        };
        let kind = match kind_name.as_str() {
            "Relational" => StoreKind::Relational,
            "KeyValue" => StoreKind::KeyValue,
            "Graph" => StoreKind::Graph,
            "Document" => StoreKind::Document,
            "Columnar" => StoreKind::Columnar,
            "Snowflake" => StoreKind::Snowflake,
            "Hypercube" => StoreKind::Hypercube,
            "TimeSeries" => StoreKind::TimeSeries,
            "Vector" => StoreKind::Vector,
            "FlatFile" => StoreKind::FlatFile,
            "Distributed" => StoreKind::Distributed,
            "DistributedLog" => StoreKind::DistributedLog,
            "InMemory" => {
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
    pub(in crate::parser) fn parse_store_table_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Table)?;
        let (name, _) = self.expect_ident()?;
        let mut fields = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            let field_start = self.current_span();
            let field_name = match self.tokens.get(self.pos) {
                Some((Token::Ident(n), _)) => {
                    let n = n.clone();
                    self.pos += 1;
                    n
                }
                _ => break,
            };
            if !self.at(&Token::Colon) {
                break;
            }
            self.advance();
            let ty = self.parse_type_expr()?;
            let annotations = self.parse_annotations();
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
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(StoreSchemaEntry::Table {
            name,
            fields,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `node Name :: { field: Type, ... }` entry.
    pub(in crate::parser) fn parse_store_node_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::GraphNode)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::Node {
            name,
            fields,
            span: Span::merge(&start, &self.current_span()),
        })
    }

    /// Parse an `edge Name :: Source -> Target [{ fields }]` entry.
    pub(in crate::parser) fn parse_store_edge_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
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
        Ok(StoreSchemaEntry::Edge {
            name,
            source,
            target,
            fields,
            span: Span::merge(&start, &self.current_span()),
        })
    }

    /// Parse a `fact Name :: { field: Type, ... }` entry.
    pub(in crate::parser) fn parse_store_fact_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Fact)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::Fact {
            name,
            fields,
            span: Span::merge(&start, &self.current_span()),
        })
    }

    /// Parse a `dimension Name :: { field: Type, ... }` entry.
    pub(in crate::parser) fn parse_store_dimension_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Dimension)?;
        let (name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::DimensionEntry {
            name,
            fields,
            span: Span::merge(&start, &self.current_span()),
        })
    }

    /// Parse an `embedding :: { field: Type, ... }` entry.
    pub(in crate::parser) fn parse_store_embedding_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.expect(Token::Embedding)?;
        self.expect(Token::ColonColon)?;
        let fields = self.parse_inline_fields()?;
        Ok(StoreSchemaEntry::EmbeddingEntry {
            fields,
            span: Span::merge(&start, &self.current_span()),
        })
    }

    /// Parse a `mapreduce Name ... end` block inside a Distributed store.
    pub(in crate::parser) fn parse_store_mapreduce_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
        let start = self.current_span();
        self.advance(); // consume `mapreduce`
        let (name, _) = self.expect_ident()?;

        let mut map_sig = String::new();
        let mut reduce_sig = String::new();
        let mut combine_sig: Option<String> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if let Some(kw) = self.token_as_ident() {
                let next_is_colon =
                    matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
                if next_is_colon {
                    self.advance(); // consume key
                    self.advance(); // consume ':'
                    let sig = self.parse_mapreduce_sig_as_string();
                    match kw.as_str() {
                        "map" => map_sig = sig,
                        "reduce" => reduce_sig = sig,
                        "combine" => combine_sig = Some(sig),
                        _ => {}
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
    pub(in crate::parser) fn parse_store_consumer_entry(
        &mut self,
    ) -> Result<StoreSchemaEntry, LoomError> {
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
    pub(in crate::parser) fn parse_mapreduce_sig_as_string(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            if self.at(&Token::End) {
                break;
            }
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
    pub(in crate::parser) fn token_as_display_string(&self) -> String {
        match self.tokens.get(self.pos) {
            Some((Token::Arrow, _)) => "->".to_string(),
            Some((Token::LBracket, _)) => "[".to_string(),
            Some((Token::RBracket, _)) => "]".to_string(),
            Some((Token::LParen, _)) => "(".to_string(),
            Some((Token::RParen, _)) => ")".to_string(),
            Some((Token::Comma, _)) => ",".to_string(),
            Some((Token::Star, _)) => "*".to_string(),
            Some((Token::ColonColon, _)) => "::".to_string(),
            Some((Token::Colon, _)) => ":".to_string(),
            Some((Token::IntLit(n), _)) => n.to_string(),
            Some((Token::FloatLit(f), _)) => f.to_string(),
            _ => self.token_as_ident().unwrap_or_else(|| "_".to_string()),
        }
    }

    /// Parse a store config value — handles `90days`, `1second`, idents, strings.
    pub(in crate::parser) fn parse_store_config_value(&mut self) -> Result<String, LoomError> {
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
    pub(in crate::parser) fn parse_tensor_rank(&mut self) -> Result<usize, LoomError> {
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
    pub(in crate::parser) fn parse_tensor_shape(&mut self) -> Result<Vec<String>, LoomError> {
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

    // ── M98: Session type parser ──────────────────────────────────────────────

    /// Parse a `session Name ... end` top-level item.
    ///
    /// ```text
    /// session Name
    ///   roleName:
    ///     send: Type
    ///     recv: Type
    ///     ...
    ///   end
    ///   ...
    ///   duality: roleA <-> roleB
    /// end
    /// ```
    pub(in crate::parser) fn parse_session_def(&mut self) -> Result<SessionDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Session)?;
        let (name, _) = self.expect_ident()?;

        let mut roles = Vec::new();
        let mut duality: Option<(String, String)> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            // `duality: roleA <-> roleB`
            if self.token_as_ident().as_deref() == Some("duality") {
                self.advance(); // consume "duality"
                self.expect(Token::Colon)?;
                let (role_a, _) = self.expect_ident()?;
                // `<->` tokenizes as Lt + Arrow (->), not Lt + Minus + Gt.
                self.expect(Token::Lt)?;
                self.expect(Token::Arrow)?;
                let (role_b, _) = self.expect_ident()?;
                duality = Some((role_a, role_b));
            } else if let Some(role_name) = self.token_as_ident() {
                // Check that the next token is `:`
                if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                    roles.push(self.parse_session_role()?);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(SessionDef {
            name,
            roles,
            duality,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a single role block inside a session definition.
    ///
    /// ```text
    /// roleName:
    ///   send: Type
    ///   recv: Type
    ///   ...
    /// end
    /// ```
    pub(in crate::parser) fn parse_session_role(&mut self) -> Result<SessionRole, LoomError> {
        let start = self.current_span();
        let (name, _) = self.expect_any_name()?;
        self.expect(Token::Colon)?;

        let mut steps = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Send) || self.token_as_ident().as_deref() == Some("send") {
                self.advance();
                self.expect(Token::Colon)?;
                let ty = self.parse_type_expr()?;
                steps.push(SessionStep::Send(ty));
            } else if self.at(&Token::Recv) || self.token_as_ident().as_deref() == Some("recv") {
                self.advance();
                self.expect(Token::Colon)?;
                let ty = self.parse_type_expr()?;
                steps.push(SessionStep::Recv(ty));
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(SessionRole {
            name,
            steps,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M99: Effect definition parser ─────────────────────────────────────────

    /// Parse an `effect Name[<TypeParams>] ... end` top-level item.
    ///
    /// ```text
    /// effect Log
    ///   operation emit :: String -> Unit
    /// end
    ///
    /// effect State<S>
    ///   operation get :: Unit -> S
    ///   operation put :: S -> Unit
    /// end
    /// ```
    pub(in crate::parser) fn parse_effect_def(&mut self) -> Result<EffectDef, LoomError> {
        let start = self.current_span();
        self.expect(Token::Effect)?;
        let (name, _) = self.expect_ident()?;

        // Optional type parameter list: `<S>`, `<A, B>`.
        let type_params = if self.at(&Token::Lt) {
            self.advance();
            let mut params = Vec::new();
            while !self.at(&Token::Gt) && self.peek().is_some() {
                let (p, _) = self.expect_ident()?;
                params.push(p);
                if self.at(&Token::Comma) {
                    self.advance();
                }
            }
            self.expect(Token::Gt)?;
            params
        } else {
            Vec::new()
        };

        let mut operations = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Operation) || self.token_as_ident().as_deref() == Some("operation") {
                operations.push(self.parse_effect_operation()?);
            } else {
                break;
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(EffectDef {
            name,
            type_params,
            operations,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a single `operation name :: InputType -> OutputType` line.
    pub(in crate::parser) fn parse_effect_operation(
        &mut self,
    ) -> Result<EffectOperation, LoomError> {
        let start = self.current_span();
        // Consume `operation` keyword (may be Token::Operation or an ident).
        self.advance();
        let (op_name, _) = self.expect_ident()?;
        self.expect(Token::ColonColon)?;
        let sig = self.parse_fn_type_signature()?;
        let end_span = self.current_span();
        // A unary fn signature: first param is input, return_type is output.
        // If no params, treat input as Unit.
        let (input, output) = if sig.params.is_empty() {
            (TypeExpr::Base("Unit".to_string()), *sig.return_type)
        } else {
            (sig.params[0].clone(), *sig.return_type)
        };
        Ok(EffectOperation {
            name: op_name,
            input,
            output,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `handle computation with ... end` block inside a fn body.
    ///
    /// ```text
    /// handle computation with
    ///   Log.emit(msg) -> k:
    ///     print(msg)
    ///     k(unit)
    ///   end
    /// end
    /// ```
    pub(in crate::parser) fn parse_handle_block(&mut self) -> Result<HandleBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Handle)?;
        let (computation, _) = self.expect_any_name()?;
        self.expect(Token::With)?;

        let mut handlers = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            // Each handler case looks like: `Effect.op(params) -> k: ... end`
            // We need to detect a qualified name (Ident.Ident) followed by `(`.
            if self.is_handler_case_start() {
                handlers.push(self.parse_effect_handler()?);
            } else {
                break;
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(HandleBlock {
            computation,
            handlers,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Returns true if the current position looks like the start of a handler case.
    ///
    /// A handler case starts with `Ident.Ident(` or `Ident(` — a qualified or
    /// unqualified operation name followed by a parameter list.
    pub(in crate::parser) fn is_handler_case_start(&self) -> bool {
        match self.tokens.get(self.pos) {
            Some((Token::Ident(_), _)) => match self.tokens.get(self.pos + 1) {
                Some((Token::Dot, _)) => true,
                Some((Token::LParen, _)) => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Parse one handler case: `Effect.op(params) -> k: ... end`.
    pub(in crate::parser) fn parse_effect_handler(&mut self) -> Result<EffectHandler, LoomError> {
        let start = self.current_span();
        // Parse qualified operation name: `Effect.op` or just `op`.
        let first_part = if let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
            let n = n.clone();
            self.pos += 1;
            n
        } else {
            return Err(LoomError::parse("expected operation name", start.clone()));
        };
        let effect_op = if self.at(&Token::Dot) {
            self.advance();
            let (second, _) = self.expect_ident()?;
            format!("{}.{}", first_part, second)
        } else {
            first_part
        };

        // Parse parameter list: `(msg)`, `()`, `(new_state)`, etc.
        let mut params = Vec::new();
        if self.at(&Token::LParen) {
            self.advance();
            while !self.at(&Token::RParen) && self.peek().is_some() {
                if let Some(p) = self.token_as_ident() {
                    params.push(p);
                    self.advance();
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                } else {
                    break;
                }
            }
            self.expect(Token::RParen)?;
        }

        // `-> k:`
        self.expect(Token::Arrow)?;
        let continuation = if let Some(k) = self.token_as_ident() {
            self.advance();
            k
        } else {
            "k".to_string()
        };
        self.expect(Token::Colon)?;

        // Consume body tokens until `end` (opaque body).
        let mut depth = 0usize;
        while self.peek().is_some() {
            match self.peek() {
                Some(Token::End) if depth == 0 => break,
                Some(Token::End) => {
                    depth -= 1;
                    self.advance();
                }
                Some(Token::Fn) | Some(Token::Being) | Some(Token::Ecosystem) => {
                    depth += 1;
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(EffectHandler {
            effect_op,
            params,
            continuation,
            span: Span::merge(&start, &end_span),
        })
    }

    ///
    /// Accepts both forms:
    /// - `property NAME: forall ...` (name then optional colon)
    /// - `property: NAME forall ...` (colon before name, corpus-style)
    pub(in crate::parser) fn parse_property_block(&mut self) -> Result<PropertyBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Property)?;
        // Accept `property:` (colon before name) OR `property NAME` (name first).
        if self.at(&Token::Colon) {
            self.advance();
        }
        let (name, _) = self.expect_any_name()?;
        // Accept optional colon after name: `property NAME:`.
        if self.at(&Token::Colon) {
            self.advance();
        }
        self.expect(Token::Forall)?;
        let (var_name, _) = self.expect_ident()?;
        self.expect(Token::Colon)?;
        let (var_type, _) = self.expect_ident()?;
        self.expect(Token::Invariant)?;
        self.expect(Token::Colon)?;
        let invariant = self.collect_property_expr();
        let mut shrink = true;
        let mut samples: u64 = 100;
        loop {
            if self.at(&Token::Shrink) {
                self.advance();
                self.expect(Token::Colon)?;
                match self.tokens.get(self.pos) {
                    Some((Token::BoolLit(b), _)) => {
                        shrink = *b;
                        self.pos += 1;
                    }
                    _ => {
                        return Err(LoomError::parse(
                            "expected bool after shrink:",
                            self.current_span(),
                        ))
                    }
                }
            } else if self.at(&Token::Samples) {
                self.advance();
                self.expect(Token::Colon)?;
                match self.tokens.get(self.pos) {
                    Some((Token::IntLit(n), _)) => {
                        samples = *n as u64;
                        self.pos += 1;
                    }
                    _ => {
                        return Err(LoomError::parse(
                            "expected int after samples:",
                            self.current_span(),
                        ))
                    }
                }
            } else {
                break;
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(PropertyBlock {
            name,
            var_name,
            var_type,
            invariant,
            shrink,
            samples,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Collect invariant expression tokens as a string until a keyword boundary.
    pub(in crate::parser) fn collect_property_expr(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match self.tokens.get(self.pos) {
                Some((Token::Shrink, _))
                | Some((Token::Samples, _))
                | Some((Token::End, _))
                | None => break,
                Some((tok, _)) => {
                    parts.push(super::token_to_source(tok));
                    self.pos += 1;
                }
            }
        }
        parts.join(" ").trim().to_string()
    }
}
