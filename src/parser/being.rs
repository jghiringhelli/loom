use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

impl<'src> crate::parser::Parser<'src> {
    // ── M66b: Annotation Algebra ──────────────────────────────────────────────

    /// Parse an `annotation Name(params)` declaration (may be annotated with
    /// meta-annotations before it, accumulated in `pending_annotations`).
    ///
    /// ```text
    /// @separation(owns: [a, b])
    /// @timing_safety(constant_time: true)
    /// annotation concurrent_transfer(a: String, b: String)
    /// ```
    pub(in crate::parser) fn parse_annotation_decl(&mut self) -> Result<AnnotationDecl, LoomError> {
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
    pub(in crate::parser) fn parse_correctness_report(
        &mut self,
    ) -> Result<CorrectnessReport, LoomError> {
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

    /// Parse a `being Name … end` block (Aristotle's four causes).
    pub(in crate::parser) fn parse_being_def(&mut self) -> Result<BeingDef, LoomError> {
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
        let mut manifest: Option<ManifestBlock> = None;
        let mut journal: Option<JournalBlock> = None;
        let mut scenarios: Vec<ScenarioBlock> = Vec::new();
        let mut migrations: Vec<MigrationBlock> = Vec::new();
        let mut boundary: Option<BoundaryBlock> = None;
        let mut cognitive_memory: Option<CognitiveMemoryBlock> = None;
        let mut signal_attention: Option<SignalAttentionBlock> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Matter) {
                matter = Some(self.parse_being_matter_section()?);
            } else if self.at(&Token::Form) {
                form = Some(self.parse_being_form_section()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "function")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                function = Some(self.parse_being_function_section()?);
            } else if self.at(&Token::Telos) {
                telos = Some(self.parse_being_telos_section()?);
            } else if self.at(&Token::Regulate) {
                regulate_blocks.push(self.parse_being_regulate_section()?);
            } else if self.at(&Token::Evolve) {
                evolve_block = Some(self.parse_being_evolve_section()?);
            } else if self.at(&Token::Autopoietic) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::BoolLit(b), _)) = self.tokens.get(self.pos) {
                    autopoietic = *b;
                    self.pos += 1;
                }
            } else if self.at(&Token::Epigenetic) {
                epigenetic_blocks.push(self.parse_being_epigenetic_section()?);
            } else if self.at(&Token::Morphogen) {
                morphogen_blocks.push(self.parse_being_morphogen_section()?);
            } else if self.at(&Token::Telomere) {
                telomere = Some(self.parse_being_telomere_section()?);
            } else if self.at(&Token::Crispr) {
                crispr_blocks.push(self.parse_being_crispr_section()?);
            } else if self.at(&Token::Plasticity) {
                plasticity_blocks.push(self.parse_being_plasticity_section()?);
            } else if self.at(&Token::Canalize) {
                canalization = Some(self.parse_canalization_block()?);
            } else if self.at(&Token::Senescence) {
                senescence = Some(self.parse_senescence_block()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "criticality")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                criticality = Some(self.parse_criticality_block()?);
            } else if self.at(&Token::Umwelt) {
                umwelt = Some(self.parse_being_umwelt_section()?);
            } else if self.at(&Token::Resonance) {
                resonance = Some(self.parse_being_resonance_section()?);
            } else if self.at(&Token::Manifest) {
                manifest = Some(self.parse_being_manifest_section()?);
            } else if self.at(&Token::Migration) {
                migrations.push(self.parse_migration_block()?);
            } else if self.at(&Token::Journal) {
                journal = Some(self.parse_journal_block()?);
            } else if self.at(&Token::Scenario) {
                scenarios.push(self.parse_scenario_block()?);
            } else if self.at(&Token::Boundary) {
                boundary = Some(self.parse_boundary_block()?);
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "memory")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                cognitive_memory = Some(self.parse_cognitive_memory_block()?);
            } else if self.at(&Token::SignalAttention) {
                signal_attention = Some(self.parse_signal_attention_block()?);
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
            manifest,
            migrations,
            journal,
            scenarios,
            boundary,
            cognitive_memory,
            signal_attention,
            span: Span::merge(&start, &end_span),
        })
    }

    fn parse_being_matter_section(&mut self) -> Result<MatterBlock, LoomError> {
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
        Ok(MatterBlock {
            fields,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_form_section(&mut self) -> Result<FormBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `form`
        self.expect(Token::Colon)?;
        let mut types = Vec::new();
        let mut enums = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Type) {
                if let Ok(item) = self.parse_type_or_refined() {
                    if let Item::Type(td) = item {
                        types.push(td);
                    }
                }
            } else if self.at(&Token::Enum) {
                if let Ok(ed) = self.parse_enum_def() {
                    enums.push(ed);
                }
            } else {
                break;
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(FormBlock {
            types,
            enums,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_function_section(&mut self) -> Result<FunctionBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume "function"
        self.advance(); // consume ":"
        let mut fns = Vec::new();
        while self.at(&Token::Fn) && !self.at(&Token::End) {
            fns.push(self.parse_fn_def()?);
        }
        let sec_end = self.current_span();
        Ok(FunctionBlock {
            fns,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_telos_section(&mut self) -> Result<TelosDef, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `telos`
        self.expect(Token::Colon)?;
        let description = match self.tokens.get(self.pos) {
            Some((Token::StrLit(s), _)) => {
                let s = s.clone();
                self.pos += 1;
                s
            }
            _ => {
                return Err(LoomError::parse(
                    "expected string literal after telos:",
                    self.current_span(),
                ))
            }
        };
        let mut fitness_fn = None;
        let mut modifiable_by = None;
        let mut bounded_by = None;
        let mut sign = None;
        let mut metric = None;
        let mut thresholds: Option<TelosThresholds> = None;
        let mut guides: Vec<String> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Fitness, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                let mut parts = Vec::new();
                while !self.at(&Token::End) && self.peek().is_some() {
                    let is_field = matches!(
                        self.tokens.get(self.pos),
                        Some((Token::ModifiableBy, _))
                            | Some((Token::BoundedBy, _))
                            | Some((Token::MeasuredBy, _))
                            | Some((Token::Thresholds, _))
                            | Some((Token::Guides, _))
                    ) || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _))
                            if n == "sign");
                    if is_field {
                        break;
                    }
                    if let Some((tok, _)) = self.tokens.get(self.pos) {
                        parts.push(format!("{:?}", tok));
                        self.pos += 1;
                    }
                }
                fitness_fn = Some(parts.join(" "));
            } else if matches!(self.tokens.get(self.pos), Some((Token::ModifiableBy, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                    let val = val.clone();
                    self.pos += 1;
                    modifiable_by = Some(val);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::BoundedBy, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                    let val = val.clone();
                    self.pos += 1;
                    bounded_by = Some(val);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "sign")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                    let val = val.clone();
                    self.pos += 1;
                    sign = Some(val);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::MeasuredBy, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::StrLit(sig), _)) = self.tokens.get(self.pos) {
                    metric = Some(sig.clone());
                    self.pos += 1;
                } else {
                    let mut parts = Vec::new();
                    while !self.at(&Token::End) && self.peek().is_some() {
                        let at_field = matches!(
                            self.tokens.get(self.pos),
                            Some((Token::Thresholds, _))
                                | Some((Token::Guides, _))
                                | Some((Token::ModifiableBy, _))
                                | Some((Token::BoundedBy, _))
                        );
                        if at_field {
                            break;
                        }
                        if let Some((tok, _)) = self.tokens.get(self.pos) {
                            parts.push(format!("{:?}", tok));
                            self.pos += 1;
                        }
                    }
                    metric = Some(parts.join(" "));
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Thresholds, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                let mut convergence = 0.8_f64;
                let mut warning = None;
                let mut divergence = 0.4_f64;
                let mut propagation = None;
                while !self.at(&Token::End) && self.peek().is_some() {
                    let at_outer_field = matches!(
                        self.tokens.get(self.pos),
                        Some((Token::Guides, _))
                            | Some((Token::ModifiableBy, _))
                            | Some((Token::BoundedBy, _))
                            | Some((Token::MeasuredBy, _))
                    );
                    if at_outer_field {
                        break;
                    }
                    if matches!(self.tokens.get(self.pos), Some((Token::Convergence, _))) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                            convergence = *v;
                            self.pos += 1;
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "warning")
                    {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                            warning = Some(*v);
                            self.pos += 1;
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Divergence, _))) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                            divergence = *v;
                            self.pos += 1;
                        }
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Propagation, _))) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                            propagation = Some(*v);
                            self.pos += 1;
                        }
                    } else {
                        self.advance();
                    }
                }
                thresholds = Some(TelosThresholds {
                    convergence,
                    warning,
                    divergence,
                    propagation,
                });
            } else if matches!(self.tokens.get(self.pos), Some((Token::Guides, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                while !self.at(&Token::End) && self.peek().is_some() {
                    let at_outer = matches!(
                        self.tokens.get(self.pos),
                        Some((Token::Thresholds, _))
                            | Some((Token::ModifiableBy, _))
                            | Some((Token::BoundedBy, _))
                            | Some((Token::MeasuredBy, _))
                    );
                    if at_outer {
                        break;
                    }
                    if let Some((Token::Ident(g), _)) = self.tokens.get(self.pos) {
                        guides.push(g.clone());
                        self.pos += 1;
                    } else if matches!(self.tokens.get(self.pos), Some((Token::SignalAttention, _)))
                    {
                        guides.push("signal_attention".to_string());
                        self.pos += 1;
                    } else {
                        self.advance();
                    }
                }
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(TelosDef {
            description,
            fitness_fn,
            modifiable_by,
            bounded_by,
            sign,
            metric,
            thresholds,
            guides,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_regulate_section(&mut self) -> Result<RegulateBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `regulate`
        let (variable, _) = self.expect_ident()?;
        let mut target = String::new();
        let mut bounds = None;
        let mut response = Vec::new();
        let mut telos_contribution: Option<f64> = None;
        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "target") {
                self.advance();
                self.expect(Token::Colon)?;
                let (val, _) = self.expect_ident()?;
                target = val;
            } else if self.at(&Token::Bounds) {
                self.advance();
                self.expect(Token::Colon)?;
                self.expect(Token::LParen)?;
                let (low, _) = self.expect_ident()?;
                self.expect(Token::Comma)?;
                let (high, _) = self.expect_ident()?;
                self.expect(Token::RParen)?;
                bounds = Some((low, high));
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "response")
            {
                self.advance();
                self.expect(Token::Colon)?;
                while self.at(&Token::Bar) {
                    self.advance();
                    let (condition, _) = self.expect_ident()?;
                    self.expect(Token::Arrow)?;
                    let (action, _) = self.expect_ident()?;
                    response.push((condition, action));
                }
            } else if matches!(
                self.tokens.get(self.pos),
                Some((Token::TelosContribution, _))
            ) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                    telos_contribution = Some(*v);
                    self.pos += 1;
                }
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(RegulateBlock {
            variable,
            target,
            bounds,
            response,
            telos_contribution,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_evolve_section(&mut self) -> Result<EvolveBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `evolve`
        let mut search_cases = Vec::new();
        let mut constraint = String::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Toward) {
                self.advance();
                self.expect(Token::Colon)?;
                self.advance();
            } else if self.at(&Token::Search) {
                self.advance();
                self.expect(Token::Colon)?;
                while self.at(&Token::Bar) {
                    self.advance();
                    let (strategy_name, _) = self.expect_ident()?;
                    let strategy = match strategy_name.as_str() {
                        "gradient_descent" => SearchStrategy::GradientDescent,
                        "stochastic_gradient" => SearchStrategy::StochasticGradient,
                        "simulated_annealing" => SearchStrategy::SimulatedAnnealing,
                        "mcmc" => SearchStrategy::Mcmc,
                        _ => SearchStrategy::DerivativeFree,
                    };
                    let when_present = matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "when");
                    if when_present {
                        self.advance();
                    }
                    let when = if when_present {
                        let next_is_colon =
                            matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)));
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
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "constraint")
            {
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
        Ok(EvolveBlock {
            search_cases,
            constraint,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_epigenetic_section(&mut self) -> Result<EpigeneticBlock, LoomError> {
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
        Ok(EpigeneticBlock {
            signal,
            modifies,
            reverts_when,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_morphogen_section(&mut self) -> Result<MorphogenBlock, LoomError> {
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
                    if self.at(&Token::RBracket) {
                        break;
                    }
                    if let Ok((val, _)) = self.expect_ident() {
                        produces.push(val);
                    }
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                    if self.at(&Token::RBracket) {
                        break;
                    }
                }
                self.expect(Token::RBracket)?;
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(MorphogenBlock {
            signal,
            threshold,
            produces,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_telomere_section(&mut self) -> Result<TelomereBlock, LoomError> {
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
        Ok(TelomereBlock {
            limit,
            on_exhaustion,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_crispr_section(&mut self) -> Result<CrisprBlock, LoomError> {
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
        Ok(CrisprBlock {
            target,
            replace,
            guide,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_plasticity_section(&mut self) -> Result<PlasticityBlock, LoomError> {
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
                    Some((Token::Hebbian, _)) => { rule = PlasticityRule::Hebbian; self.pos += 1; }
                    Some((Token::Boltzmann, _)) => { rule = PlasticityRule::Boltzmann; self.pos += 1; }
                    Some((Token::Ident(n), _)) if n == "reinforcement_learning" => { rule = PlasticityRule::ReinforcementLearning; self.pos += 1; }
                    _ => return Err(LoomError::parse(
                        "unknown plasticity rule: expected hebbian, boltzmann, or reinforcement_learning",
                        self.current_span(),
                    )),
                }
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(PlasticityBlock {
            trigger,
            modifies,
            rule,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_umwelt_section(&mut self) -> Result<UmweltBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `umwelt`
        self.expect(Token::Colon)?;
        let mut detects: Vec<String> = Vec::new();
        let mut blind_to: Vec<String> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "detects")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                self.expect(Token::LBracket)?;
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    if let Ok((val, _)) = self.expect_ident() {
                        detects.push(val);
                    }
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "blind_to")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                self.expect(Token::LBracket)?;
                while !self.at(&Token::RBracket) && self.peek().is_some() {
                    if let Ok((val, _)) = self.expect_ident() {
                        blind_to.push(val);
                    }
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(UmweltBlock {
            detects,
            blind_to,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_resonance_section(&mut self) -> Result<ResonanceBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `resonance`
        self.expect(Token::Colon)?;
        let mut correlations: Vec<CorrelationPair> = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "correlate")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                let pair_start = self.current_span();
                self.advance();
                self.advance();
                let (signal_a, _) = self.expect_ident()?;
                if self.at(&Token::With) {
                    self.advance();
                }
                let (signal_b, _) = self.expect_ident()?;
                let via = if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "via")
                {
                    self.advance();
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
        Ok(ResonanceBlock {
            correlations,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    fn parse_being_manifest_section(&mut self) -> Result<ManifestBlock, LoomError> {
        let sec_start = self.current_span();
        self.advance(); // consume `manifest`
        self.expect(Token::Colon)?;
        let mut artifacts = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Artifact) {
                self.advance();
                let path = match self.tokens.get(self.pos) {
                    Some((Token::StrLit(s), _)) => {
                        let s = s.clone();
                        self.pos += 1;
                        s
                    }
                    _ => break,
                };
                let mut reflects = Vec::new();
                let mut freshness = None;
                let mut required_when = None;
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Reflects) {
                        self.advance();
                        self.expect(Token::Colon)?;
                        self.expect(Token::LBracket)?;
                        while !self.at(&Token::RBracket) && self.peek().is_some() {
                            let (sym, _) = self.expect_any_name()?;
                            reflects.push(sym);
                            if self.at(&Token::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(Token::RBracket)?;
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "freshness")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        self.advance();
                        self.advance();
                        let mut parts = Vec::new();
                        while !self.at(&Token::End) && self.peek().is_some() {
                            let is_known = self.at(&Token::Reflects)
                                || matches!(self.tokens.get(self.pos),
                                    Some((Token::Ident(n), _)) if n == "required_when");
                            if is_known {
                                break;
                            }
                            if let Some((tok, _)) = self.tokens.get(self.pos) {
                                parts.push(format!("{:?}", tok));
                                self.pos += 1;
                            }
                        }
                        freshness = Some(parts.join(" "));
                    } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "required_when")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
                    {
                        self.advance();
                        self.advance();
                        if let Some((Token::Ident(val), _)) = self.tokens.get(self.pos) {
                            let val = val.clone();
                            self.pos += 1;
                            required_when = Some(val);
                        }
                    } else {
                        break;
                    }
                }
                self.expect(Token::End)?;
                artifacts.push(ManifestArtifact {
                    path,
                    reflects,
                    freshness,
                    required_when,
                });
            } else {
                self.advance();
            }
        }
        let sec_end = self.current_span();
        self.expect(Token::End)?;
        Ok(ManifestBlock {
            artifacts,
            span: Span::merge(&sec_start, &sec_end),
        })
    }

    /// Parse a `migration name: … end` block inside a being.
    ///
    /// Grammar:
    /// ```text
    /// migration v1_to_v2:
    ///   from: field_name OldType
    ///   to:   field_name NewType
    ///   adapter: "fn v1 -> ..."
    ///   breaking: false
    /// end
    /// ```
    pub(in crate::parser) fn parse_migration_block(&mut self) -> Result<MigrationBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Migration)?;
        let (name, _) = self.expect_ident()?;
        // Optional colon after migration name.
        if self.at(&Token::Colon) {
            self.advance();
        }

        let mut from_field = String::new();
        let mut to_field = String::new();
        let mut adapter: Option<String> = None;
        let mut breaking = true;

        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::From) {
                self.advance(); // consume `from`
                self.expect(Token::Colon)?;
                let mut parts = Vec::new();
                while !self.at(&Token::End)
                    && !self.at(&Token::To)
                    && !matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _))
                        if n == "adapter" || n == "breaking")
                    && self.peek().is_some()
                {
                    if let Some((tok, _)) = self.tokens.get(self.pos) {
                        parts.push(format!("{:?}", tok));
                        self.pos += 1;
                    }
                }
                from_field = parts.join(" ");
            } else if self.at(&Token::To) {
                self.advance(); // consume `to`
                self.expect(Token::Colon)?;
                let mut parts = Vec::new();
                while !self.at(&Token::End)
                    && !self.at(&Token::From)
                    && !matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _))
                        if n == "adapter" || n == "breaking")
                    && self.peek().is_some()
                {
                    if let Some((tok, _)) = self.tokens.get(self.pos) {
                        parts.push(format!("{:?}", tok));
                        self.pos += 1;
                    }
                }
                to_field = parts.join(" ");
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "adapter")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `adapter`
                self.advance(); // consume `:`
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    adapter = Some(s.clone());
                    self.pos += 1;
                } else if let Some(name) = self.token_as_ident() {
                    adapter = Some(name);
                    self.pos += 1;
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "breaking")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `breaking`
                self.advance(); // consume `:`
                if let Some((Token::BoolLit(b), _)) = self.tokens.get(self.pos) {
                    breaking = *b;
                    self.pos += 1;
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(MigrationBlock {
            name,
            from_field,
            to_field,
            adapter,
            breaking,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `journal: … end` block inside a being.
    ///
    /// Grammar:
    /// ```text
    /// journal:
    ///   record: every evolve_step
    ///   record: every telos_progress
    ///   keep: last 1000
    ///   emit: "path/file.log"
    /// end
    /// ```
    pub(in crate::parser) fn parse_journal_block(&mut self) -> Result<JournalBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Journal)?;
        self.expect(Token::Colon)?;

        let mut records: Vec<JournalRecord> = Vec::new();
        let mut keep_last: Option<u64> = None;
        let mut emit_path: Option<String> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "record")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `record`
                self.advance(); // consume `:`
                                // consume optional `every`
                if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "every") {
                    self.advance();
                }
                // read record type
                if let Some((Token::Ident(event), _)) = self.tokens.get(self.pos) {
                    let record = match event.as_str() {
                        "evolve_step" => JournalRecord::EvolveStep,
                        "telos_progress" => JournalRecord::TelosProgress,
                        "state_transition" => JournalRecord::StateTransition,
                        "regulation_trigger" => JournalRecord::RegulationTrigger,
                        other => JournalRecord::Custom(other.to_string()),
                    };
                    self.pos += 1;
                    records.push(record);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "keep")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `keep`
                self.advance(); // consume `:`
                                // consume optional `last`
                if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "last") {
                    self.advance();
                }
                if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                    let n = *n as u64;
                    self.pos += 1;
                    keep_last = Some(n);
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "emit")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `emit`
                self.advance(); // consume `:`
                if let Some((Token::StrLit(s), _)) = self.tokens.get(self.pos) {
                    emit_path = Some(s.clone());
                    self.pos += 1;
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(JournalBlock {
            records,
            keep_last,
            emit_path,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `scenario name: … end` block inside a being.
    ///
    /// Grammar:
    /// ```text
    /// scenario trade_executes_on_signal:
    ///   given: market_signal == BullishCrossover
    ///   when:  being.sense() detects market_signal
    ///   then:  ensure position_size > 0
    ///   within: 3 lifecycle_ticks
    /// end
    /// ```
    pub(in crate::parser) fn parse_scenario_block(&mut self) -> Result<ScenarioBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Scenario)?;
        let (name, _) = self.expect_any_name()?;
        self.expect(Token::Colon)?;

        let mut given = String::new();
        let mut when = String::new();
        let mut then = String::new();
        let mut within: Option<(u64, String)> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "given")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `given`
                self.advance(); // consume `:`
                given = self.collect_rest_of_line();
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "when")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `when`
                self.advance(); // consume `:`
                when = self.collect_rest_of_line();
            } else if self.at(&Token::Then)
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `then`
                self.advance(); // consume `:`
                then = self.collect_rest_of_line();
            } else if self.at(&Token::Within)
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `within`
                self.advance(); // consume `:`
                if let Some((Token::IntLit(n), _)) = self.tokens.get(self.pos) {
                    let count = *n as u64;
                    self.pos += 1;
                    let unit = if let Some((Token::Ident(u), _)) = self.tokens.get(self.pos) {
                        let u = u.clone();
                        self.pos += 1;
                        u
                    } else {
                        "ticks".to_string()
                    };
                    within = Some((count, unit));
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(ScenarioBlock {
            name,
            given,
            when,
            then,
            within,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `usecase NAME:` block — M110 triple derivation.
    ///
    /// Grammar:
    /// ```text
    /// usecase RegisterUser:
    ///   actor: ExternalUser
    ///   precondition: not_user_exists
    ///   trigger: POST
    ///   postcondition: user_count_increased
    ///   acceptance:
    ///     test can_register_valid_user: email is valid password meets policy
    ///     ...
    ///   end
    /// end
    /// ```
    pub(in crate::parser) fn parse_usecase_block(&mut self) -> Result<UseCaseBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::UseCase)?;
        let (name, _) = self.expect_ident()?;
        // Optional `:` after name
        if self.at(&Token::Colon) {
            self.advance();
        }

        let mut actor = String::new();
        let mut precondition = String::new();
        let mut trigger = String::new();
        let mut postcondition = String::new();
        let mut acceptance: Vec<AcceptanceCriterion> = Vec::new();

        while !self.at(&Token::End) && self.peek().is_some() {
            // actor: IDENT  (Token::Actor from keywords, or Ident("actor") for back-compat)
            if (self.at(&Token::Actor)
                || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "actor"))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `actor`
                self.advance(); // consume `:`
                actor = self.collect_usecase_field_value();
            // precondition: <tokens>
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "precondition")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                precondition = self.collect_usecase_field_value();
            // trigger: <tokens>
            } else if (self.at(&Token::Trigger)
                || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "trigger"))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                trigger = self.collect_usecase_field_value();
            // postcondition: <tokens>
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "postcondition")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance();
                self.advance();
                postcondition = self.collect_usecase_field_value();
            // acceptance: ... end
            } else if (self.at(&Token::Acceptance)
                || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "acceptance"))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `acceptance`
                self.advance(); // consume `:`
                while !self.at(&Token::End) && self.peek().is_some() {
                    if self.at(&Token::Test) {
                        self.advance(); // consume `test`
                                        // criterion name: next ident
                        let crit_name = if let Some(n) = self.token_as_ident() {
                            self.advance();
                            n
                        } else {
                            break;
                        };
                        // optional `:`
                        if self.at(&Token::Colon) {
                            self.advance();
                        }
                        let description = self.collect_usecase_field_value();
                        acceptance.push(AcceptanceCriterion {
                            name: crit_name,
                            description,
                        });
                    } else {
                        break;
                    }
                }
                // consume the inner `end`
                if self.at(&Token::End) {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(UseCaseBlock {
            name,
            actor,
            precondition,
            trigger,
            postcondition,
            acceptance,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Collect tokens into a string until the next field keyword or `end`.
    ///
    /// Stops (without consuming) on: `Token::End`, `Token::Actor`,
    /// `Token::Trigger`, `Token::Acceptance`, `Token::Test`,
    /// or `Ident("precondition")` / `Ident("postcondition")`.
    pub(in crate::parser) fn collect_usecase_field_value(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        loop {
            match self.tokens.get(self.pos) {
                None => break,
                Some((Token::End, _)) => break,
                Some((Token::Actor, _)) => break,
                Some((Token::Trigger, _)) => {
                    // Only break if next token is `:` (it's a field label)
                    if matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) {
                        break;
                    }
                    parts.push("trigger".to_string());
                    self.pos += 1;
                }
                Some((Token::Acceptance, _)) => break,
                Some((Token::Test, _)) => break,
                Some((Token::Ident(n), _))
                    if (n == "precondition"
                        || n == "postcondition"
                        || n == "actor"
                        || n == "acceptance")
                        && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _))) =>
                {
                    break;
                }
                Some((tok, _)) => {
                    parts.push(super::token_to_source(tok));
                    self.pos += 1;
                }
            }
        }
        parts.join(" ")
    }

    /// Parse `flow label :: TypeA, TypeB, ...`.
    pub(in crate::parser) fn parse_provides_block(&mut self) -> Result<Provides, LoomError> {
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
    pub(in crate::parser) fn parse_requires_block(&mut self) -> Result<Requires, LoomError> {
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

    // ── M68: Degeneracy block ─────────────────────────────────────────────

    /// Parse `degenerate: primary: X fallback: Y [equivalence_proof: Z] end`.
    pub(in crate::parser) fn parse_degenerate_block(
        &mut self,
    ) -> Result<DegenerateBlock, LoomError> {
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
    pub(in crate::parser) fn parse_canalization_block(
        &mut self,
    ) -> Result<CanalizationBlock, LoomError> {
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
                                if let Ok((v, _)) = self.expect_ident() {
                                    despite.push(v);
                                }
                                if self.at(&Token::Comma) {
                                    self.advance();
                                }
                                if self.at(&Token::RBracket) {
                                    break;
                                }
                            }
                            self.expect(Token::RBracket)?;
                        }
                        "convergence_proof" => {
                            let (val, _) = self.expect_ident()?;
                            convergence_proof = Some(val);
                        }
                        _ => {
                            let _ = self.expect_ident();
                        }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CanalizationBlock {
            toward,
            despite,
            convergence_proof,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M74: Senescence block ─────────────────────────────────────────────

    /// Parse `senescence: onset: X degradation: Y [sasp: Z] end`.
    pub(in crate::parser) fn parse_senescence_block(
        &mut self,
    ) -> Result<SenescenceBlock, LoomError> {
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
        Ok(SenescenceBlock {
            onset,
            degradation,
            sasp,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M75: HGT adopt ───────────────────────────────────────────────────

    /// Parse `adopt: Interface from Module`.
    pub(in crate::parser) fn parse_adopt_decl(&mut self) -> Result<AdoptDecl, LoomError> {
        let start = self.current_span();
        self.expect(Token::Adopt)?;
        self.expect(Token::Colon)?;
        let (interface, _) = self.expect_ident()?;
        // consume "from" (keyword token)
        self.expect(Token::From)?;
        let (from_module, _) = self.expect_ident()?;
        let end_span = self.current_span();
        Ok(AdoptDecl {
            interface,
            from_module,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M76: Criticality block ────────────────────────────────────────────

    /// Parse `criticality: lower: N upper: N [probe_fn: X] end`.
    pub(in crate::parser) fn parse_criticality_block(
        &mut self,
    ) -> Result<CriticalityBlock, LoomError> {
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
                        "lower" => match self.peek() {
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
                            _ => {
                                let _ = self.expect_ident();
                            }
                        },
                        "upper" => match self.peek() {
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
                            _ => {
                                let _ = self.expect_ident();
                            }
                        },
                        "probe_fn" => {
                            let (val, _) = self.expect_ident()?;
                            probe_fn = Some(val);
                        }
                        _ => {
                            let _ = self.expect_ident();
                        }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(CriticalityBlock {
            lower,
            upper,
            probe_fn,
            span: Span::merge(&start, &end_span),
        })
    }

    // ── M77: Niche construction ───────────────────────────────────────────

    /// Parse `niche_construction: modifies: X affects: [A, B] [probe_fn: Z] end`.
    pub(in crate::parser) fn parse_niche_construction(
        &mut self,
    ) -> Result<NicheConstructionDef, LoomError> {
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
                                if let Ok((v, _)) = self.expect_ident() {
                                    affects.push(v);
                                }
                                if self.at(&Token::Comma) {
                                    self.advance();
                                }
                                if self.at(&Token::RBracket) {
                                    break;
                                }
                            }
                            self.expect(Token::RBracket)?;
                        }
                        "probe_fn" => {
                            let (val, _) = self.expect_ident()?;
                            probe_fn = Some(val);
                        }
                        _ => {
                            let _ = self.expect_ident();
                        }
                    }
                    continue;
                }
            }
            break;
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(NicheConstructionDef {
            modifies,
            affects,
            probe_fn,
            span: Span::merge(&start, &end_span),
        })
    }

    /// M81: Parse `sense Name ... end` top-level item.
    pub(in crate::parser) fn parse_sense_def(&mut self) -> Result<SenseDef, LoomError> {
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
                    if self.at(&Token::Comma) {
                        self.advance();
                    }
                    if self.at(&Token::RBracket) {
                        break;
                    }
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
        Ok(SenseDef {
            name,
            channels,
            range,
            unit,
            dimension,
            derived,
            span: Span::merge(&start, &end_span),
        })
    }

    pub(in crate::parser) fn parse_boundary_block(&mut self) -> Result<BoundaryBlock, LoomError> {
        let start = self.current_span();
        self.expect(Token::Boundary)?;
        self.expect(Token::Colon)?;
        let mut exports = Vec::new();
        let mut private = Vec::new();
        let mut sealed = Vec::new();
        while !self.at(&Token::End) && self.peek().is_some() {
            if self.at(&Token::Export) {
                self.advance(); // consume `export`
                self.expect(Token::Colon)?;
                while !self.at(&Token::End) && self.peek().is_some() {
                    let is_next_section = self.at(&Token::Export)
                        || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "private")
                        || self.at(&Token::Seal);
                    if is_next_section {
                        break;
                    }
                    if let Some(name) = self.token_as_ident() {
                        exports.push(name);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "private")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // consume `private`
                self.advance(); // consume `:`
                while !self.at(&Token::End) && self.peek().is_some() {
                    let is_next_section = self.at(&Token::Export)
                        || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "private")
                        || self.at(&Token::Seal);
                    if is_next_section {
                        break;
                    }
                    if let Some(name) = self.token_as_ident() {
                        private.push(name);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            } else if self.at(&Token::Seal) {
                self.advance(); // consume `seal`
                self.expect(Token::Colon)?;
                while !self.at(&Token::End) && self.peek().is_some() {
                    let is_next_section = self.at(&Token::Export)
                        || matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "private")
                        || self.at(&Token::Seal);
                    if is_next_section {
                        break;
                    }
                    if let Some(name) = self.token_as_ident() {
                        sealed.push(name);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            } else {
                self.advance();
            }
        }
        let end_span = self.current_span();
        self.expect(Token::End)?;
        Ok(BoundaryBlock {
            exports,
            private,
            sealed,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `memory: type: episodic procedural ... decay_rate: 0.1 tier: working end` block.
    ///
    /// Grammar:
    /// ```text
    /// memory:
    ///   type: episodic procedural architectural semantic insight
    ///   decay_rate: 0.05          -- optional override
    ///   tier: buffer | working | core  -- optional override
    /// end
    /// ```
    pub(in crate::parser) fn parse_cognitive_memory_block(
        &mut self,
    ) -> Result<CognitiveMemoryBlock, LoomError> {
        use crate::ast::{CognitiveMemoryBlock, CognitiveMemoryType, MemoryTier};
        let start = self.current_span();
        self.advance(); // consume "memory"
        self.expect(Token::Colon)?;

        let mut memory_types: Vec<CognitiveMemoryType> = Vec::new();
        let mut decay_rate: Option<f64> = None;
        let mut tier: Option<MemoryTier> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Type, _)))
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // "type"
                self.advance(); // ":"
                while let Some((Token::Ident(n), _)) = self.tokens.get(self.pos) {
                    let kind = match n.as_str() {
                        "episodic" => Some(CognitiveMemoryType::Episodic),
                        "semantic" => Some(CognitiveMemoryType::Semantic),
                        "procedural" => Some(CognitiveMemoryType::Procedural),
                        "architectural" => Some(CognitiveMemoryType::Architectural),
                        "insight" => Some(CognitiveMemoryType::Insight),
                        _ => None,
                    };
                    if let Some(k) = kind {
                        memory_types.push(k);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "decay_rate")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // "decay_rate"
                self.advance(); // ":"
                if let Some((Token::FloatLit(f), _)) = self.tokens.get(self.pos) {
                    decay_rate = Some(*f);
                    self.pos += 1;
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Ident(n), _)) if n == "tier")
                && matches!(self.tokens.get(self.pos + 1), Some((Token::Colon, _)))
            {
                self.advance(); // "tier"
                self.advance(); // ":"
                if let Some((Token::Ident(t), _)) = self.tokens.get(self.pos) {
                    tier = match t.as_str() {
                        "buffer" => Some(MemoryTier::Buffer),
                        "working" => Some(MemoryTier::Working),
                        "core" => Some(MemoryTier::Core),
                        _ => None,
                    };
                    if tier.is_some() {
                        self.pos += 1;
                    }
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(CognitiveMemoryBlock {
            memory_types,
            decay_rate,
            tier,
            span: Span::merge(&start, &end_span),
        })
    }

    /// Parse a `signal_attention` block inside a being.
    ///
    /// Grammar:
    /// ```text
    /// signal_attention
    ///   prioritize: 0.6   -- signals with telos_relevance > threshold get priority
    ///   attenuate: 0.2    -- signals with telos_relevance < threshold are damped
    /// end
    /// ```
    pub(in crate::parser) fn parse_signal_attention_block(
        &mut self,
    ) -> Result<SignalAttentionBlock, LoomError> {
        let start = self.current_span();
        self.advance(); // consume `signal_attention`
        let mut prioritize_above: Option<f64> = None;
        let mut attenuate_below: Option<f64> = None;

        while !self.at(&Token::End) && self.peek().is_some() {
            if matches!(self.tokens.get(self.pos), Some((Token::Prioritize, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                    prioritize_above = Some(*v);
                    self.pos += 1;
                }
            } else if matches!(self.tokens.get(self.pos), Some((Token::Attenuate, _))) {
                self.advance();
                self.expect(Token::Colon)?;
                if let Some((Token::FloatLit(v), _)) = self.tokens.get(self.pos) {
                    attenuate_below = Some(*v);
                    self.pos += 1;
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_span();
        self.expect(Token::End)?;

        Ok(SignalAttentionBlock {
            prioritize_above,
            attenuate_below,
            span: Span::merge(&start, &end_span),
        })
    }
}
