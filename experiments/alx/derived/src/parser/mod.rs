// ALX: derived from loom.loom §"Pipeline: Parser" and language-spec.md §13 (Grammar).
// Recursive-descent LL(2) parser. Peeks up to 2 tokens ahead for disambiguation.
// Entry point: parse_module(). Dispatches on current token kind.

use crate::ast::*;
use crate::error::{LoomError, Span};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Annotations accumulated at @ tokens; merged into the next definition.
    pending_annotations: Vec<Annotation>,
}

// ── Type-or-refined helper ────────────────────────────────────────────────────

enum TypeOrRefined {
    Type(TypeDef),
    Refined(RefinedType),
}

impl Parser {
    /// Create a new parser from a slice of tokens.
    /// G2: Tests call `Parser::new(&tokens)` with a reference, so we accept `&[Token]`.
    pub fn new(tokens: &[Token]) -> Self {
        Parser {
            tokens: tokens.to_vec(),
            pos: 0,
            pending_annotations: Vec::new(),
        }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn current_span(&self) -> Span {
        self.peek().map(|t| t.span).unwrap_or_default()
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, LoomError> {
        match self.peek() {
            Some(t) if std::mem::discriminant(&t.kind) == std::mem::discriminant(kind) => {
                Ok(self.advance().unwrap())
            }
            Some(t) => Err(LoomError::new(
                format!("expected {:?}, got {:?}", kind, t.kind),
                t.span,
            )),
            None => Err(LoomError::new(
                format!("expected {:?} but reached end of input", kind),
                Span::default(),
            )),
        }
    }

    fn expect_ident(&mut self) -> Result<String, LoomError> {
        match self.peek().map(|t| t.kind.clone()) {
            Some(TokenKind::Ident(s)) => {
                self.advance();
                Ok(s)
            }
            // Many keywords can be used as field/variable names in certain positions
            Some(_) => {
                // Try treating any token as an identifier if it has text
                match self.peek() {
                    Some(t) if !t.text.is_empty() => {
                        let text = t.text.clone();
                        self.advance();
                        Ok(text)
                    }
                    other => {
                        let span = other.map(|t| t.span).unwrap_or_default();
                        Err(LoomError::new("expected identifier", span))
                    }
                }
            }
            None => Err(LoomError::new("expected identifier, got EOF", Span::default())),
        }
    }

    fn skip_if(&mut self, kind: &TokenKind) -> bool {
        if let Some(t) = self.peek() {
            if std::mem::discriminant(&t.kind) == std::mem::discriminant(kind) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek()
            .map(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(kind))
            .unwrap_or(false)
    }

    fn take_pending_annotations(&mut self) -> Vec<Annotation> {
        std::mem::take(&mut self.pending_annotations)
    }

    // ── Module ────────────────────────────────────────────────────────────────

    pub fn parse_module(&mut self) -> Result<Module, LoomError> {
        // module Name
        self.expect(&TokenKind::Module)?;
        let name_tok = self.advance().ok_or_else(|| LoomError::zero("expected module name"))?;
        let name = name_tok.text.clone();
        let start = name_tok.span.start;

        let mut module = Module::new(name, Span::new(start, start));

        // Optional describe:
        if self.check(&TokenKind::Describe) {
            self.advance();
            self.expect(&TokenKind::Colon)?;
            module.describe = Some(self.parse_string_lit()?);
        }

        // Collect top-level items until 'end' or EOF
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            // Annotations accumulate
            if self.check(&TokenKind::At) {
                let ann = self.parse_annotation()?;
                self.pending_annotations.push(ann);
                continue;
            }

            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Fn) => {
                    let f = self.parse_fn_def()?;
                    module.items.push(Item::Fn(f));
                }
                Some(TokenKind::Type) => {
                    // Could be type_def or refined_type — peek at structure
                    let item = self.parse_type_or_refined()?;
                    match item {
                        TypeOrRefined::Type(t) => module.items.push(Item::Type(t)),
                        TypeOrRefined::Refined(r) => module.items.push(Item::RefinedType(r)),
                    }
                }
                Some(TokenKind::Enum) => {
                    let e = self.parse_enum_def()?;
                    module.items.push(Item::Enum(e));
                }
                Some(TokenKind::Interface) => {
                    let idef = self.parse_interface_def()?;
                    module.interface_defs.push(idef);
                }
                Some(TokenKind::Lifecycle) => {
                    let lc = self.parse_lifecycle_def()?;
                    module.lifecycle_defs.push(lc);
                }
                Some(TokenKind::Flow) => {
                    let fl = self.parse_flow_label()?;
                    module.flow_labels.push(fl);
                }
                Some(TokenKind::Being) => {
                    let b = self.parse_being_def()?;
                    module.being_defs.push(b);
                }
                Some(TokenKind::Ecosystem) => {
                    let eco = self.parse_ecosystem_def()?;
                    module.ecosystem_defs.push(eco);
                }
                Some(TokenKind::Import) => {
                    self.advance();
                    let name = self.expect_ident()?;
                    // optional 'as Alias'
                    if self.check(&TokenKind::As) {
                        self.advance();
                        let _alias = self.expect_ident()?;
                    }
                    module.imports.push(name);
                }
                Some(TokenKind::Implements) => {
                    self.advance();
                    let name = self.expect_ident()?;
                    module.implements.push(name);
                }
                Some(TokenKind::Invariant) => {
                    let inv = self.parse_invariant()?;
                    module.invariants.push(inv);
                }
                Some(TokenKind::Test) => {
                    let tst = self.parse_test()?;
                    module.test_defs.push(tst);
                }
                Some(TokenKind::Provides) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let mut ops = Vec::new();
                    if self.check(&TokenKind::LBrace) {
                        self.advance(); // consume '{'
                        while !self.is_at_end() && !self.check(&TokenKind::RBrace) {
                            let op_name = self.expect_ident()?;
                            self.expect(&TokenKind::DoubleColon)?;
                            let (type_sig, _) = self.parse_fn_type_sig()?;
                            ops.push((op_name, type_sig));
                            self.skip_if(&TokenKind::Comma);
                        }
                        self.skip_if(&TokenKind::RBrace);
                    } else {
                        // Fallback: single interface name
                        let n = self.expect_ident()?;
                        ops.push((n, FnTypeSignature { params: vec![], return_type: TypeExpr::Base("()".to_string()) }));
                    }
                    module.provides = Some(Provides { ops, span: Span::default() });
                }
                Some(TokenKind::Requires) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let mut deps = Vec::new();
                    if self.check(&TokenKind::LBrace) {
                        self.advance(); // consume '{'
                        while !self.is_at_end() && !self.check(&TokenKind::RBrace) {
                            let dep_name = self.expect_ident()?;
                            self.expect(&TokenKind::Colon)?;
                            let dep_type = self.parse_type_expr()?;
                            deps.push((dep_name, dep_type));
                            self.skip_if(&TokenKind::Comma);
                        }
                        self.skip_if(&TokenKind::RBrace);
                    } else {
                        // Fallback: comma-separated names only
                        loop {
                            let n = self.expect_ident()?;
                            deps.push((n, TypeExpr::Base("Unknown".to_string())));
                            if !self.skip_if(&TokenKind::Comma) { break; }
                        }
                    }
                    module.requires = Some(Requires { deps, span: Span::default() });
                }
                Some(TokenKind::Describe) => {
                    // module-level describe: (already consumed above or repeated)
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    module.describe = Some(self.parse_string_lit()?);
                }
                _ => {
                    // Skip unknown token to avoid infinite loop
                    self.advance();
                }
            }
        }

        // Optional trailing 'end'
        self.skip_if(&TokenKind::End);

        // Any leftover pending annotations belong to the module itself
        let leftover = self.take_pending_annotations();
        module.annotations.extend(leftover);

        module.span = Span::new(start, self.current_span().end);
        Ok(module)
    }

    // ── Annotations ──────────────────────────────────────────────────────────

    fn parse_annotation(&mut self) -> Result<Annotation, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::At)?;
        let key = self.expect_ident()?;
        let mut value = String::new();
        // Optional (value) or (key=val ...)
        if self.check(&TokenKind::LParen) {
            self.advance();
            // collect everything until RParen
            let mut depth = 1;
            while !self.is_at_end() && depth > 0 {
                match self.peek().map(|t| t.kind.clone()) {
                    Some(TokenKind::LParen) => { depth += 1; value.push('('); self.advance(); }
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth > 0 { value.push(')'); }
                        self.advance();
                    }
                    _ => {
                        if let Some(t) = self.advance() {
                            if !value.is_empty() { value.push(' '); }
                            value.push_str(&t.text);
                        }
                    }
                }
            }
        }
        let span = Span::new(start, self.current_span().end);
        // Strip surrounding quotes from string literal values (e.g. @since("v1.0") → v1.0)
        let value = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            value[1..value.len()-1].to_string()
        } else {
            value
        };
        Ok(Annotation::with_value(key, value))
    }

    // ── String literals ───────────────────────────────────────────────────────

    fn parse_string_lit(&mut self) -> Result<String, LoomError> {
        match self.peek().map(|t| t.kind.clone()) {
            Some(TokenKind::StringLit(s)) => {
                self.advance();
                Ok(s)
            }
            other => Err(LoomError::new(
                format!("expected string literal, got {:?}", other),
                self.current_span(),
            )),
        }
    }

    // ── Type expressions ──────────────────────────────────────────────────────
    // ALX: language-spec.md §3.5 lists all type expression forms.

    pub fn parse_type_expr(&mut self) -> Result<TypeExpr, LoomError> {
        let base = self.parse_type_atom()?;
        // Check for function arrow ->
        if self.check(&TokenKind::Arrow) {
            self.advance();
            let ret = self.parse_type_expr()?;
            return Ok(TypeExpr::Fn(Box::new(base), Box::new(ret)));
        }
        Ok(base)
    }

    fn parse_type_atom(&mut self) -> Result<TypeExpr, LoomError> {
        // Tuple: (A, B) or (A, B, C)
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut types = Vec::new();
            types.push(self.parse_type_expr()?);
            while self.skip_if(&TokenKind::Comma) {
                types.push(self.parse_type_expr()?);
            }
            self.expect(&TokenKind::RParen)?;
            if types.len() == 1 {
                return Ok(types.remove(0));
            }
            return Ok(TypeExpr::Tuple(types));
        }

        // Effect<[E1,E2], T>
        if self.check_ident("Effect") {
            self.advance();
            self.expect(&TokenKind::LAngle)?;
            self.expect(&TokenKind::LBracket)?;
            let mut effects = Vec::new();
            if !self.check(&TokenKind::RBracket) {
                effects.push(self.parse_effect_name()?);
                while self.skip_if(&TokenKind::Comma) && !self.check(&TokenKind::RBracket) {
                    effects.push(self.parse_effect_name()?);
                }
            }
            self.expect(&TokenKind::RBracket)?;
            self.skip_if(&TokenKind::Comma);
            let ret = self.parse_type_expr()?;
            // ALX: spec uses ] to close Effect<[...], T] — treat ] as closing
            if self.check(&TokenKind::RBracket) {
                self.advance();
            } else {
                self.skip_if(&TokenKind::RAngle);
            }
            return Ok(TypeExpr::Effect(effects, Box::new(ret)));
        }

        // List<T>
        if self.check_ident("List") {
            self.advance();
            if self.check(&TokenKind::LAngle) {
                self.advance();
                let inner = self.parse_type_expr()?;
                self.skip_if(&TokenKind::RAngle);
                return Ok(TypeExpr::Generic("List".into(), vec![inner]));
            }
            return Ok(TypeExpr::Base("List".into()));
        }

        // Option<T>
        if self.check_ident("Option") {
            self.advance();
            if self.check(&TokenKind::LAngle) {
                self.advance();
                let inner = self.parse_type_expr()?;
                self.skip_if(&TokenKind::RAngle);
                return Ok(TypeExpr::Option(Box::new(inner)));
            }
            return Ok(TypeExpr::Base("Option".into()));
        }

        // Result<T, E>
        if self.check_ident("Result") {
            self.advance();
            if self.check(&TokenKind::LAngle) {
                self.advance();
                let ok = self.parse_type_expr()?;
                self.skip_if(&TokenKind::Comma);
                let err = self.parse_type_expr()?;
                self.skip_if(&TokenKind::RAngle);
                return Ok(TypeExpr::Result(Box::new(ok), Box::new(err)));
            }
            return Ok(TypeExpr::Base("Result".into()));
        }

        // Name or Name<T>
        let name = self.parse_type_name_token()?;

        // Check for generic param <...>
        if self.check(&TokenKind::LAngle) {
            self.advance();
            let mut params = Vec::new();
            params.push(self.parse_type_expr()?);
            while self.skip_if(&TokenKind::Comma) {
                params.push(self.parse_type_expr()?);
            }
            self.skip_if(&TokenKind::RAngle);
            return Ok(TypeExpr::Generic(name, params));
        }

        Ok(TypeExpr::Base(name))
    }

    fn parse_type_name_token(&mut self) -> Result<String, LoomError> {
        match self.peek().map(|t| (t.kind.clone(), t.text.clone())) {
            Some((TokenKind::Ident(s), _)) => { self.advance(); Ok(s) }
            Some((_, text)) if !text.is_empty() => { self.advance(); Ok(text) }
            _ => Err(LoomError::new("expected type name", self.current_span())),
        }
    }

    fn check_ident(&self, name: &str) -> bool {
        match self.peek().map(|t| &t.kind) {
            Some(TokenKind::Ident(s)) => s == name,
            Some(_) => {
                // Also check text field for keywords used as type names
                self.peek().map(|t| t.text == name).unwrap_or(false)
            }
            None => false,
        }
    }

    fn parse_effect_name(&mut self) -> Result<String, LoomError> {
        let base = self.expect_ident()?;
        // Optional @tier annotation within effect list
        if self.check(&TokenKind::At) {
            self.advance();
            let tier = self.expect_ident()?;
            return Ok(format!("{}@{}", base, tier));
        }
        Ok(base)
    }

    // ── Type definition ───────────────────────────────────────────────────────

    fn parse_type_or_refined(&mut self) -> Result<TypeOrRefined, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Type)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params_opt()?;
        self.expect(&TokenKind::Assign)?;

        // Peek for Where — if the RHS is a base type followed by 'where', it's refined
        // ALX: language-spec.md §3.3: `type Email = String where valid_email end`
        // Check if this is: type Name = BaseType where predicate end
        // We need LL(2) here: after '=' we peek at the structure
        // Simple heuristic: if immediately after '=' we see an ident then 'where', refined
        let maybe_refined = self.try_parse_refined_body(start, name.clone(), type_params.clone());
        if let Some(r) = maybe_refined {
            return Ok(TypeOrRefined::Refined(r));
        }

        // Otherwise parse struct fields
        let fields = self.parse_field_list()?;
        self.skip_if(&TokenKind::End);

        Ok(TypeOrRefined::Type(TypeDef {
            name,
            type_params,
            fields,
            span: Span::new(start, self.current_span().end),
        }))
    }

    fn try_parse_refined_body(
        &mut self,
        start: usize,
        name: String,
        _type_params: Vec<String>,
    ) -> Option<RefinedType> {
        // Save position
        let saved_pos = self.pos;

        // Try: parse one type name, then check for 'where'
        if let Ok(base_te) = self.parse_type_atom() {
            if self.check(&TokenKind::Where) {
                self.advance(); // consume 'where'
                // Collect predicate until 'end' or top-level keyword
                let mut pred_tokens = Vec::new();
                while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_top_level_keyword() {
                    pred_tokens.push(self.advance().unwrap().text);
                }
                self.skip_if(&TokenKind::End);
                let predicate = pred_tokens.join(" ");
                return Some(RefinedType {
                    name,
                    base_type: base_te,
                    predicate,
                    span: Span::new(start, self.current_span().end),
                });
            }
        }
        // Backtrack
        self.pos = saved_pos;
        None
    }

    fn parse_type_params_opt(&mut self) -> Result<Vec<String>, LoomError> {
        if !self.check(&TokenKind::LAngle) {
            return Ok(Vec::new());
        }
        self.advance();
        let mut params = Vec::new();
        params.push(self.expect_ident()?);
        while self.skip_if(&TokenKind::Comma) {
            params.push(self.expect_ident()?);
        }
        self.expect(&TokenKind::RAngle)?;
        Ok(params)
    }

    fn parse_field_list(&mut self) -> Result<Vec<FieldDef>, LoomError> {
        let mut fields = Vec::new();
        // Fields can be newline-separated or comma-separated
        // Parse until we hit 'end', being keyword, or EOF
        while !self.is_at_end()
            && !self.check(&TokenKind::End)
            && !self.check(&TokenKind::Being)
            && !self.is_block_keyword()
        {
            // Try to parse a field: name: Type @annotations...
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Ident(_)) | Some(TokenKind::At) => {
                    // Might be annotation for next field, or field itself
                    if self.check(&TokenKind::At) {
                        break; // annotations belong to next definition
                    }
                    let field = self.parse_field()?;
                    fields.push(field);
                    self.skip_if(&TokenKind::Comma);
                }
                _ => break,
            }
        }
        Ok(fields)
    }

    fn is_block_keyword(&self) -> bool {
        // ALX: "function" is not in spec's TokenKind enum; it lexes as Ident("function")
        if let Some(t) = self.peek() {
            if let TokenKind::Ident(s) = &t.kind {
                if s == "function" { return true; }
            }
        }
        matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Matter)
            | Some(TokenKind::Form)
            | Some(TokenKind::Telos)
            | Some(TokenKind::Regulate)
            | Some(TokenKind::Evolve)
            | Some(TokenKind::Epigenetic)
            | Some(TokenKind::Morphogen)
            | Some(TokenKind::Telomere)
            | Some(TokenKind::Crispr)
            | Some(TokenKind::Plasticity)
            | Some(TokenKind::Autopoietic)
        )
    }

    fn parse_field(&mut self) -> Result<FieldDef, LoomError> {
        let start = self.current_span().start;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type_expr()?;
        // Collect inline annotations
        let mut annotations = Vec::new();
        while self.check(&TokenKind::At) {
            annotations.push(self.parse_annotation()?);
        }
        Ok(FieldDef {
            name,
            ty,
            annotations,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Enum definition ───────────────────────────────────────────────────────

    fn parse_enum_def(&mut self) -> Result<EnumDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Enum)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params_opt()?;
        self.expect(&TokenKind::Assign)?;

        let mut variants = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            // | VariantName [of Type]
            if self.check(&TokenKind::Pipe) {
                self.advance();
            }
            let vspan = self.current_span();
            let vname = self.expect_ident()?;
            let payload = if self.check(&TokenKind::Of) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: vname,
                payload,
                span: Span::new(vspan.start, self.current_span().end),
            });
            self.skip_if(&TokenKind::Pipe);
        }
        self.skip_if(&TokenKind::End);

        Ok(EnumDef {
            name,
            type_params,
            variants,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Function definition ───────────────────────────────────────────────────

    fn parse_fn_def(&mut self) -> Result<FnDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;

        // Collect annotations: fn name @ann @ann :: sig
        let mut annotations = self.take_pending_annotations();
        while self.check(&TokenKind::At) {
            annotations.push(self.parse_annotation()?);
        }

        // Optional describe: before :: (alternate syntax)
        let mut pre_describe: Option<String> = None;
        if self.check(&TokenKind::Describe) {
            self.advance();
            self.skip_if(&TokenKind::Colon);
            pre_describe = Some(self.parse_string_lit()?);
            // allow more annotations after describe:
            while self.check(&TokenKind::At) {
                annotations.push(self.parse_annotation()?);
            }
        }

        // Optional type params: <A, B>
        let type_params = self.parse_type_params_opt()?;

        // :: type-signature
        self.expect(&TokenKind::DoubleColon)?;
        let (type_sig, effect_tiers) = self.parse_fn_type_sig()?;

        // Body options: describe:, require:, ensure:, body exprs, end
        let mut describe = pre_describe;
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        let mut body = Vec::new();
        let mut inline_body: Option<String> = None;
        let mut with_deps: Vec<String> = Vec::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Describe) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    describe = Some(self.parse_string_lit()?);
                }
                Some(TokenKind::Require) => {
                    let cspan = self.current_span();
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let expr = self.collect_line();
                    requires.push(Contract {
                        expr,
                        span: Span::new(cspan.start, self.current_span().end),
                    });
                }
                Some(TokenKind::Ensure) => {
                    let cspan = self.current_span();
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let expr = self.collect_line();
                    ensures.push(Contract {
                        expr,
                        span: Span::new(cspan.start, self.current_span().end),
                    });
                }
                // Detect `inline { ... }` verbatim block
                Some(TokenKind::Ident(ref s)) if s == "inline" => {
                    self.advance(); // consume 'inline'
                    if self.check(&TokenKind::LBrace) {
                        self.advance(); // consume '{'
                        inline_body = Some(self.collect_brace_body());
                    } else {
                        body.push("inline".to_string());
                    }
                }
                Some(TokenKind::With) => {
                    self.advance(); // consume 'with'
                    if let Ok(dep) = self.expect_ident() {
                        with_deps.push(dep);
                    }
                }
                Some(TokenKind::Let) => {
                    let stmt = self.collect_let_statement();
                    if !stmt.is_empty() { body.push(stmt); }
                }
                Some(TokenKind::Match) => {
                    let stmt = self.collect_match_expression();
                    if !stmt.is_empty() { body.push(stmt); }
                }
                Some(TokenKind::For) => {
                    let stmt = self.collect_for_expression();
                    if !stmt.is_empty() { body.push(stmt); }
                }
                _ => {
                    let stmt = self.collect_expression_to_boundary();
                    if !stmt.is_empty() {
                        body.push(stmt);
                    } else if !self.is_at_end() && !self.check(&TokenKind::End) {
                        self.advance(); // avoid infinite loop
                    }
                }
            }
        }
        self.skip_if(&TokenKind::End);

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
            inline_body,
            span: Span::new(start, self.current_span().end),
        })
    }

    /// Collect tokens inside a brace block (already consumed the opening `{`),
    /// handling nested braces. Uses token spans to preserve original spacing
    /// so that e.g. `42i64` is not split into `42 i64`.
    fn collect_brace_body(&mut self) -> String {
        let mut result = String::new();
        let mut depth = 1usize;
        let mut prev_end: usize = 0;
        let mut first = true;

        while !self.is_at_end() && depth > 0 {
            let (kind, text, span) = match self.peek() {
                Some(t) => (t.kind.clone(), t.text.clone(), t.span),
                None => break,
            };

            match kind {
                crate::lexer::TokenKind::LBrace => {
                    depth += 1;
                    self.advance();
                    if !first && span.start > prev_end { result.push(' '); }
                    result.push_str(&text);
                    prev_end = span.end;
                    first = false;
                }
                crate::lexer::TokenKind::RBrace => {
                    depth -= 1;
                    self.advance();
                    if depth > 0 {
                        if !first && span.start > prev_end { result.push(' '); }
                        result.push_str(&text);
                        prev_end = span.end;
                        first = false;
                    }
                }
                _ => {
                    self.advance();
                    if !first && span.start > prev_end { result.push(' '); }
                    result.push_str(&text);
                    prev_end = span.end;
                    first = false;
                }
            }
        }
        result
    }

    /// Parse `Type -> Type -> ... -> RetType`, extracting effect tiers if present.
    /// Uses parse_type_atom (not parse_type_expr) to avoid consuming '->' as part
    /// of a type expression — Loom uses curried notation: each '->' separates params.
    fn parse_fn_type_sig(&mut self) -> Result<(FnTypeSignature, Vec<String>), LoomError> {
        let mut all_types = Vec::new();
        let mut effect_tiers = Vec::new();

        all_types.push(self.parse_type_atom()?);
        while self.check(&TokenKind::Arrow) {
            self.advance();
            all_types.push(self.parse_type_atom()?);
        }

        // Extract effects from return type if it's Effect<[...], T>
        if let Some(TypeExpr::Effect(effs, _)) = all_types.last() {
            effect_tiers = effs.clone();
        }

        if all_types.is_empty() {
            return Err(LoomError::zero("empty function type signature"));
        }
        let return_type = all_types.pop().unwrap();
        Ok((
            FnTypeSignature {
                params: all_types,
                return_type,
            },
            effect_tiers,
        ))
    }

    /// Collect tokens until a newline-equivalent boundary.
    /// ALX: body expressions are one expression per logical line.
    fn collect_line(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut prev_end: usize = 0;
        let mut first = true;
        while let Some(t) = self.peek() {
            let span = t.span;
            if matches!(t.kind, TokenKind::Require | TokenKind::Ensure | TokenKind::Describe | TokenKind::End) {
                break;
            }
            let text = self.advance().unwrap().text;
            if !first && span.start > prev_end { parts.push(" ".to_string()); }
            parts.push(text);
            prev_end = span.end;
            first = false;
        }
        parts.join("")
    }

    fn collect_until_end_or_keyword(&mut self) -> String {
        // Legacy method - now delegates to collect_expression_to_boundary
        self.collect_expression_to_boundary()
    }

    fn collect_let_statement(&mut self) -> String {
        self.advance(); // consume 'let'
        let mut result = String::from("let");
        let mut prev_end: usize = 0;
        let mut first = true;
        let mut paren_depth: i32 = 0;
        let mut last_was_expr_end = false;

        while let Some(t) = self.peek() {
            let span = t.span;
            let is_value = matches!(t.kind,
                TokenKind::Ident(_) | TokenKind::IntLit(_) | TokenKind::FloatLit(_) |
                TokenKind::StringLit(_) | TokenKind::True | TokenKind::False
            );

            // At depth 0, after an expr end, if next token is a fresh value → stop
            if paren_depth == 0 && last_was_expr_end && is_value {
                break;
            }

            match t.kind.clone() {
                TokenKind::End | TokenKind::Require | TokenKind::Ensure | TokenKind::Describe => break,
                TokenKind::Let | TokenKind::Match | TokenKind::With | TokenKind::For => break,
                TokenKind::LParen | TokenKind::LBracket => {
                    paren_depth += 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { result.push(' '); }
                    result.push_str(&text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = false;
                }
                TokenKind::RParen | TokenKind::RBracket => {
                    paren_depth -= 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { result.push(' '); }
                    result.push_str(&text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = paren_depth == 0;
                }
                _ => {
                    let text = self.advance().unwrap().text;
                    if first || span.start > prev_end { result.push(' '); }
                    result.push_str(&text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = is_value && paren_depth == 0;
                }
            }
        }
        result
    }

    fn collect_expression_to_boundary(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut prev_end: usize = 0;
        let mut first = true;
        let mut paren_depth: i32 = 0;
        let mut last_was_expr_end = false;

        while let Some(t) = self.peek() {
            let span = t.span;
            let is_value = matches!(t.kind,
                TokenKind::Ident(_) | TokenKind::IntLit(_) | TokenKind::FloatLit(_) |
                TokenKind::StringLit(_) | TokenKind::True | TokenKind::False
            );

            // At depth 0, after an expr end, if next token is a fresh value → stop
            if paren_depth == 0 && last_was_expr_end && is_value {
                break;
            }

            match t.kind.clone() {
                TokenKind::End | TokenKind::Require | TokenKind::Ensure | TokenKind::Describe => break,
                TokenKind::Let | TokenKind::Match | TokenKind::With | TokenKind::For => break,
                TokenKind::LParen | TokenKind::LBracket => {
                    paren_depth += 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = false;
                }
                TokenKind::RParen | TokenKind::RBracket => {
                    paren_depth -= 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = paren_depth == 0;
                }
                _ => {
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                    last_was_expr_end = is_value && paren_depth == 0;
                }
            }
        }
        parts.join("")
    }

    fn collect_match_expression(&mut self) -> String {
        self.advance(); // consume 'match'
        // Collect subject tokens until first '|' or 'end'
        let mut subject_parts: Vec<String> = Vec::new();
        let mut prev_end: usize = 0;
        let mut first = true;
        while let Some(t) = self.peek() {
            let span = t.span;
            match t.kind.clone() {
                TokenKind::Pipe | TokenKind::End => break,
                _ => {
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { subject_parts.push(" ".to_string()); }
                    subject_parts.push(text);
                    prev_end = span.end;
                    first = false;
                }
            }
        }
        let subject = subject_parts.join("").trim().to_string();

        // Collect arms: | Pattern [if guard] -> body
        let mut arms: Vec<String> = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            if self.skip_if(&TokenKind::Pipe) {
                // Collect pattern tokens until 'if', '->', '|', or 'end'
                let mut pat_parts: Vec<String> = Vec::new();
                let mut guard: Option<String> = None;
                let mut prev_e: usize = 0;
                let mut fst = true;
                while let Some(t) = self.peek() {
                    let span = t.span;
                    match t.kind.clone() {
                        TokenKind::Arrow => { self.advance(); break; }
                        TokenKind::If => {
                            self.advance(); // consume 'if'
                            let mut g_parts: Vec<String> = Vec::new();
                            let mut gpe: usize = 0;
                            let mut gf = true;
                            while let Some(t2) = self.peek() {
                                let s2 = t2.span;
                                match t2.kind.clone() {
                                    TokenKind::Arrow => { self.advance(); break; }
                                    TokenKind::Pipe | TokenKind::End => break,
                                    _ => {
                                        let txt = self.advance().unwrap().text;
                                        if !gf && s2.start > gpe { g_parts.push(" ".to_string()); }
                                        g_parts.push(txt);
                                        gpe = s2.end; gf = false;
                                    }
                                }
                            }
                            guard = Some(g_parts.join("").trim().to_string());
                            break;
                        }
                        TokenKind::Pipe | TokenKind::End => break,
                        _ => {
                            let text = self.advance().unwrap().text;
                            if !fst && span.start > prev_e { pat_parts.push(" ".to_string()); }
                            pat_parts.push(text);
                            prev_e = span.end; fst = false;
                        }
                    }
                }
                // Collect arm body until next '|' or 'end'
                let mut body_parts: Vec<String> = Vec::new();
                let mut bpe: usize = 0;
                let mut bf = true;
                while let Some(t) = self.peek() {
                    let span = t.span;
                    match t.kind.clone() {
                        TokenKind::Pipe | TokenKind::End => break,
                        _ => {
                            let text = self.advance().unwrap().text;
                            if !bf && span.start > bpe { body_parts.push(" ".to_string()); }
                            body_parts.push(text);
                            bpe = span.end; bf = false;
                        }
                    }
                }
                let pat = pat_parts.join("").trim().to_string();
                let body_str = body_parts.join("").trim().to_string();
                let arm = if let Some(g) = guard {
                    format!("{} if ({}) => {}", pat, g, body_str)
                } else {
                    format!("{} => {}", pat, body_str)
                };
                arms.push(arm);
            } else {
                self.advance(); // skip unexpected
            }
        }
        self.skip_if(&TokenKind::End);

        if arms.is_empty() {
            format!("match {} {{}}", subject)
        } else {
            format!("match {} {{ {} }}", subject, arms.join(", "))
        }
    }

    fn collect_for_expression(&mut self) -> String {
        self.advance(); // consume 'for'
        let mut parts: Vec<String> = Vec::new();
        let mut prev_end: usize = 0;
        let mut first = true;
        let mut brace_depth: i32 = 0;
        while let Some(t) = self.peek() {
            let span = t.span;
            match t.kind.clone() {
                TokenKind::LBrace => {
                    brace_depth += 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                }
                TokenKind::RBrace => {
                    brace_depth -= 1;
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                    if brace_depth <= 0 { break; }
                }
                TokenKind::End | TokenKind::Require | TokenKind::Ensure | TokenKind::Describe
                    if brace_depth <= 0 => { break; }
                _ => {
                    let text = self.advance().unwrap().text;
                    if !first && span.start > prev_end { parts.push(" ".to_string()); }
                    parts.push(text);
                    prev_end = span.end; first = false;
                }
            }
        }
        format!("for {}", parts.join(""))
    }

    // ── Interface definition ──────────────────────────────────────────────────

    fn parse_interface_def(&mut self) -> Result<InterfaceDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Interface)?;
        let name = self.expect_ident()?;
        let type_params = self.parse_type_params_opt()?;

        let mut methods = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            if self.check(&TokenKind::Fn) {
                // Parse fn name + type sig only — no body parsing for interfaces
                self.advance(); // consume 'fn'
                let fn_name = self.expect_ident()?;
                let fn_type_params = self.parse_type_params_opt()?;
                // Optional :: before type sig (interfaces may omit it)
                self.skip_if(&TokenKind::DoubleColon);
                let (type_sig, effect_tiers) = self.parse_fn_type_sig()?;
                let fn_span = Span::new(start, self.current_span().end);
                methods.push(FnDef {
                    name: fn_name,
                    type_params: fn_type_params,
                    type_sig,
                    effect_tiers,
                    describe: None,
                    annotations: Vec::new(),
                    requires: Vec::new(),
                    ensures: Vec::new(),
                    body: Vec::new(),
                    inline_body: None,
                    with_deps: Vec::new(),
                    span: fn_span,
                });
            } else {
                self.advance();
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(InterfaceDef {
            name,
            type_params,
            methods,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Lifecycle definition ──────────────────────────────────────────────────

    fn parse_lifecycle_def(&mut self) -> Result<LifecycleDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Lifecycle)?;
        let type_name = self.expect_ident()?;
        self.expect(&TokenKind::DoubleColon)?;
        let mut states = Vec::new();
        states.push(self.expect_ident()?);
        while self.check(&TokenKind::Arrow) {
            self.advance();
            states.push(self.expect_ident()?);
        }
        Ok(LifecycleDef {
            type_name,
            states,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Flow label ────────────────────────────────────────────────────────────

    fn parse_flow_label(&mut self) -> Result<FlowLabel, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Flow)?;
        let label = self.expect_ident()?;
        self.expect(&TokenKind::DoubleColon)?;
        let mut types = Vec::new();
        types.push(self.expect_ident()?);
        while self.skip_if(&TokenKind::Comma) {
            types.push(self.expect_ident()?);
        }
        Ok(FlowLabel {
            label,
            types,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Invariant ─────────────────────────────────────────────────────────────

    fn parse_invariant(&mut self) -> Result<Invariant, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Invariant)?;
        let name = self.expect_ident()?;
        // optional :: or :
        self.skip_if(&TokenKind::DoubleColon);
        self.skip_if(&TokenKind::Colon);
        let mut cond_tokens = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_top_level_keyword() {
            cond_tokens.push(self.advance().unwrap().text);
        }
        self.skip_if(&TokenKind::End);
        Ok(Invariant {
            name,
            condition: cond_tokens.join(" "),
            span: Span::new(start, self.current_span().end),
        })
    }

    fn is_top_level_keyword(&self) -> bool {
        matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Fn)
            | Some(TokenKind::Type)
            | Some(TokenKind::Enum)
            | Some(TokenKind::Interface)
            | Some(TokenKind::Being)
            | Some(TokenKind::Ecosystem)
            | Some(TokenKind::Invariant)
            | Some(TokenKind::Test)
            | Some(TokenKind::Lifecycle)
            | Some(TokenKind::Flow)
            | Some(TokenKind::Import)
            | Some(TokenKind::Implements)
        )
    }

    // ── Test ──────────────────────────────────────────────────────────────────

    fn parse_test(&mut self) -> Result<TestDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Test)?;
        let name = self.expect_ident()?;
        self.skip_if(&TokenKind::DoubleColon);
        self.skip_if(&TokenKind::Colon);

        // Detect `inline { ... }` and use span-aware collection
        let body = if self.check_ident("inline") {
            self.advance(); // consume 'inline'
            if self.check(&TokenKind::LBrace) {
                self.advance(); // consume '{'
                let content = self.collect_brace_body();
                format!("inline {{ {} }}", content)
            } else {
                "inline".to_string()
            }
        } else {
            let mut body_tokens = Vec::new();
            let mut prev_end: usize = 0;
            let mut first = true;
            while !self.is_at_end() && !self.check(&TokenKind::End) {
                let tok = self.peek().unwrap();
                let span = tok.span;
                let text = tok.text.clone();
                self.advance();
                if !first && span.start > prev_end { body_tokens.push(" ".to_string()); }
                body_tokens.push(text);
                prev_end = span.end;
                first = false;
            }
            body_tokens.join("")
        };

        self.skip_if(&TokenKind::End);
        Ok(TestDef {
            name,
            body,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Being definition ──────────────────────────────────────────────────────
    // ALX: from loom.loom §"AST: Biological constructs" and language-spec.md §14

    fn parse_being_def(&mut self) -> Result<BeingDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Being)?;
        let name = self.expect_ident()?;

        let mut being = BeingDef {
            name,
            describe: None,
            annotations: self.take_pending_annotations(),
            matter: None,
            form: None,
            function: None,
            telos: None,
            regulate_blocks: Vec::new(),
            evolve_block: None,
            epigenetic_blocks: Vec::new(),
            morphogen_blocks: Vec::new(),
            telomere: None,
            crispr_blocks: Vec::new(),
            plasticity_blocks: Vec::new(),
            autopoietic: false,
            span: Span::new(start, start),
        };

        // Parse being header: describe:, @annotations, then sub-blocks
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Describe) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    being.describe = Some(self.parse_string_lit()?);
                }
                Some(TokenKind::At) => {
                    being.annotations.push(self.parse_annotation()?);
                }
                Some(TokenKind::Autopoietic) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    // 'true' or 'false'
                    match self.peek().map(|t| t.kind.clone()) {
                        Some(TokenKind::True) => { being.autopoietic = true; self.advance(); }
                        Some(TokenKind::False) => { being.autopoietic = false; self.advance(); }
                        _ => { being.autopoietic = true; } // default true if just 'autopoietic:' line
                    }
                }
                Some(TokenKind::Matter) => {
                    being.matter = Some(self.parse_matter_block()?);
                }
                Some(TokenKind::Form) => {
                    being.form = Some(self.parse_form_block()?);
                }
                // "function" is not in spec TokenKind enum — it lexes as Ident("function")
                _ if self.peek().map(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "function")).unwrap_or(false) => {
                    being.function = Some(self.parse_function_block()?);
                }
                Some(TokenKind::Telos) => {
                    being.telos = Some(self.parse_telos_def()?);
                }
                Some(TokenKind::Regulate) => {
                    being.regulate_blocks.push(self.parse_regulate_block()?);
                }
                Some(TokenKind::Evolve) => {
                    being.evolve_block = Some(self.parse_evolve_block()?);
                }
                Some(TokenKind::Epigenetic) => {
                    being.epigenetic_blocks.push(self.parse_epigenetic_block()?);
                }
                Some(TokenKind::Morphogen) => {
                    being.morphogen_blocks.push(self.parse_morphogen_block()?);
                }
                Some(TokenKind::Telomere) => {
                    being.telomere = Some(self.parse_telomere_block()?);
                }
                Some(TokenKind::Crispr) => {
                    being.crispr_blocks.push(self.parse_crispr_block()?);
                }
                Some(TokenKind::Plasticity) => {
                    being.plasticity_blocks.push(self.parse_plasticity_block()?);
                }
                _ => {
                    // Skip unrecognised tokens inside being
                    if self.is_top_level_keyword() {
                        break;
                    }
                    self.advance();
                }
            }
        }
        self.skip_if(&TokenKind::End);
        being.span = Span::new(start, self.current_span().end);
        Ok(being)
    }

    fn parse_matter_block(&mut self) -> Result<MatterBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Matter)?;
        self.skip_if(&TokenKind::Colon);
        // fields until 'end' — accept keyword-named fields (e.g. `threshold: Float`)
        let mut fields = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) {
            // Stop if we hit a genuine being sub-block that is NOT followed by ':'
            // (i.e., a sub-block opener vs a keyword used as a field name).
            // Heuristic: peek at the token after the current one; if it's ':', treat as field.
            if self.is_block_keyword() {
                // Look ahead: if next-next token is ':', this keyword is a field name
                let is_field = self.tokens.get(self.pos + 1)
                    .map(|t| matches!(t.kind, TokenKind::Colon))
                    .unwrap_or(false);
                if !is_field {
                    break; // genuine sub-block, stop field list
                }
            }
            match self.peek().map(|t| t.kind.clone()) {
                None => break,
                _ => {
                    let f = self.parse_field()?;
                    fields.push(f);
                    self.skip_if(&TokenKind::Comma);
                }
            }
        }
        self.skip_if(&TokenKind::End);
        Ok(MatterBlock {
            fields,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_form_block(&mut self) -> Result<FormBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Form)?;
        self.skip_if(&TokenKind::Colon);
        let mut types = Vec::new();
        let mut enums = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Type) => {
                    match self.parse_type_or_refined()? {
                        TypeOrRefined::Type(t) => types.push(t),
                        TypeOrRefined::Refined(_) => {} // ALX: refined inside form: rare, skip
                    }
                }
                Some(TokenKind::Enum) => {
                    enums.push(self.parse_enum_def()?);
                }
                _ => break,
            }
        }
        self.skip_if(&TokenKind::End);
        Ok(FormBlock {
            types,
            enums,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_function_block(&mut self) -> Result<FunctionBlock, LoomError> {
        let start = self.current_span().start;
        // "function" lexes as Ident, not a keyword token
        match self.peek().map(|t| t.kind.clone()) {
            Some(TokenKind::Ident(s)) if s == "function" => { self.advance(); }
            _ => {
                return Err(LoomError::new("expected 'function' keyword", self.current_span()));
            }
        }
        self.skip_if(&TokenKind::Colon);
        let mut fns = Vec::new();
        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            if self.check(&TokenKind::Fn) {
                fns.push(self.parse_fn_def()?);
            } else {
                break;
            }
        }
        self.skip_if(&TokenKind::End);
        Ok(FunctionBlock {
            fns,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_telos_def(&mut self) -> Result<TelosDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Telos)?;
        self.skip_if(&TokenKind::Colon);
        let description = self.parse_string_lit()?;
        let mut fitness_fn = None;
        let mut modifiable_by = None;
        let mut bounded_by = None;

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Fitness) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let mut toks = Vec::new();
                    while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
                        match self.peek().map(|t| &t.kind) {
                            Some(TokenKind::ModifiableBy) | Some(TokenKind::BoundedBy) => break,
                            _ => toks.push(self.advance().unwrap().text),
                        }
                    }
                    fitness_fn = Some(toks.join(" "));
                }
                Some(TokenKind::ModifiableBy) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    modifiable_by = Some(self.expect_ident()?);
                }
                Some(TokenKind::BoundedBy) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    bounded_by = Some(self.expect_ident()?);
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(TelosDef {
            description,
            fitness_fn,
            modifiable_by,
            bounded_by,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_regulate_block(&mut self) -> Result<RegulateBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Regulate)?;
        let variable = self.expect_ident()?;
        let mut target = String::new();
        let mut bounds = None;
        let mut response = Vec::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Target) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let mut toks = Vec::new();
                    while !self.is_at_end() {
                        match self.peek().map(|t| &t.kind) {
                            Some(TokenKind::Bounds) | Some(TokenKind::Response)
                            | Some(TokenKind::End) => break,
                            _ => toks.push(self.advance().unwrap().text),
                        }
                    }
                    target = toks.join(" ");
                }
                Some(TokenKind::Bounds) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    self.skip_if(&TokenKind::LParen);
                    self.skip_if(&TokenKind::LBracket);
                    let lo = self.collect_until(&[TokenKind::Comma]);
                    self.skip_if(&TokenKind::Comma);
                    let hi = self.collect_until(&[TokenKind::RParen, TokenKind::RBracket]);
                    self.skip_if(&TokenKind::RParen);
                    self.skip_if(&TokenKind::RBracket);
                    bounds = Some((lo, hi));
                }
                Some(TokenKind::Response) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
                        if self.skip_if(&TokenKind::Pipe) {
                            let mut line = Vec::new();
                            while !self.is_at_end() {
                                match self.peek().map(|t| &t.kind) {
                                    Some(TokenKind::Pipe) | Some(TokenKind::End) => break,
                                    _ => {
                                        if self.is_block_keyword() { break; }
                                        line.push(self.advance().unwrap().text);
                                    }
                                }
                            }
                            response.push(line.join(" "));
                        } else {
                            break;
                        }
                    }
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(RegulateBlock {
            variable,
            target,
            bounds,
            response,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn collect_until(&mut self, stops: &[TokenKind]) -> String {
        let mut toks = Vec::new();
        while let Some(t) = self.peek() {
            if stops.iter().any(|s| std::mem::discriminant(&t.kind) == std::mem::discriminant(s)) {
                break;
            }
            toks.push(self.advance().unwrap().text);
        }
        toks.join(" ")
    }

    fn parse_evolve_block(&mut self) -> Result<EvolveBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Evolve)?;
        let mut search_cases = Vec::new();
        let mut constraint = String::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Toward) => {
                    // toward: is ignored — EvolveBlock has no toward field
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    let _ = self.expect_ident();
                }
                Some(TokenKind::Search) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    while self.skip_if(&TokenKind::Pipe) {
                        let strategy = self.parse_search_strategy()?;
                        let when_cond = if self.check(&TokenKind::When) {
                            self.advance();
                            let mut toks = Vec::new();
                            while !self.is_at_end() {
                                match self.peek().map(|t| &t.kind) {
                                    Some(TokenKind::Pipe) | Some(TokenKind::Constraint)
                                    | Some(TokenKind::End) => break,
                                    _ => {
                                        if self.is_block_keyword() { break; }
                                        toks.push(self.advance().unwrap().text);
                                    }
                                }
                            }
                            toks.join(" ")
                        } else {
                            String::new()
                        };
                        search_cases.push(SearchCase { strategy, when: when_cond });
                    }
                }
                Some(TokenKind::Constraint) => {
                    self.advance();
                    self.skip_if(&TokenKind::Colon);
                    constraint = self.parse_string_lit().unwrap_or_default();
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(EvolveBlock {
            search_cases,
            constraint,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_search_strategy(&mut self) -> Result<SearchStrategy, LoomError> {
        let tok = self.peek().map(|t| t.kind.clone());
        match tok {
            Some(TokenKind::GradientDescent) => { self.advance(); Ok(SearchStrategy::GradientDescent) }
            Some(TokenKind::StochasticGradient) => { self.advance(); Ok(SearchStrategy::StochasticGradient) }
            Some(TokenKind::SimulatedAnnealing) => { self.advance(); Ok(SearchStrategy::SimulatedAnnealing) }
            Some(TokenKind::DerivativeFree) => { self.advance(); Ok(SearchStrategy::DerivativeFree) }
            Some(TokenKind::Mcmc) => { self.advance(); Ok(SearchStrategy::Mcmc) }
            _ => {
                // Try as ident
                let s = self.expect_ident()?;
                Ok(match s.as_str() {
                    "gradient_descent" => SearchStrategy::GradientDescent,
                    "stochastic_gradient" => SearchStrategy::StochasticGradient,
                    "simulated_annealing" => SearchStrategy::SimulatedAnnealing,
                    "derivative_free" => SearchStrategy::DerivativeFree,
                    "mcmc" => SearchStrategy::Mcmc,
                    _ => SearchStrategy::GradientDescent, // ALX: default fallback
                })
            }
        }
    }

    fn parse_epigenetic_block(&mut self) -> Result<EpigeneticBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Epigenetic)?;
        self.skip_if(&TokenKind::Colon);
        let mut signal = String::new();
        let mut modifies = String::new();
        let mut reverts_when = None;

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Signal) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    signal = self.expect_ident()?;
                }
                Some(TokenKind::Modifies) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    modifies = self.expect_ident()?;
                }
                Some(TokenKind::RevertsWhen) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    let mut toks = Vec::new();
                    while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
                        toks.push(self.advance().unwrap().text);
                    }
                    reverts_when = Some(toks.join(" "));
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(EpigeneticBlock {
            signal,
            modifies,
            reverts_when,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_morphogen_block(&mut self) -> Result<MorphogenBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Morphogen)?;
        self.skip_if(&TokenKind::Colon);
        let mut signal = String::new();
        let mut threshold = String::new();
        let mut produces: Vec<String> = Vec::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Signal) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    signal = self.expect_ident()?;
                }
                Some(TokenKind::Threshold) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    if let Some(t) = self.peek() {
                        threshold = t.text.clone();
                        self.advance();
                    }
                }
                Some(TokenKind::Produces) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    // Parse optional list [A, B, ...] or single ident
                    if self.check(&TokenKind::LBracket) {
                        self.advance();
                        while !self.is_at_end() && !self.check(&TokenKind::RBracket) {
                            if let Ok(name) = self.expect_ident() {
                                produces.push(name);
                            }
                            self.skip_if(&TokenKind::Comma);
                        }
                        self.skip_if(&TokenKind::RBracket);
                    } else if let Ok(name) = self.expect_ident() {
                        produces.push(name);
                    }
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(MorphogenBlock {
            signal,
            threshold,
            produces,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_telomere_block(&mut self) -> Result<TelomereBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Telomere)?;
        self.skip_if(&TokenKind::Colon);
        let mut limit = 0i64;
        let mut on_exhaustion = String::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Limit) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    if let Some(TokenKind::IntLit(n)) = self.peek().map(|t| t.kind.clone()) {
                        limit = n; self.advance();
                    }
                }
                Some(TokenKind::OnExhaustion) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    on_exhaustion = self.expect_ident()?;
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(TelomereBlock {
            limit,
            on_exhaustion,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_crispr_block(&mut self) -> Result<CrisprBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Crispr)?;
        self.skip_if(&TokenKind::Colon);
        let mut target = String::new();
        let mut replace = String::new();
        let mut guide = String::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Target) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    target = self.expect_ident()?;
                }
                Some(TokenKind::Replace) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    replace = self.expect_ident()?;
                }
                Some(TokenKind::Guide) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    guide = self.expect_ident()?;
                }
                Some(TokenKind::Preserve) => {
                    // preserve: is parsed but ignored — CrisprBlock has no preserve field
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
                        self.advance();
                    }
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(CrisprBlock {
            target,
            replace,
            guide,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_plasticity_block(&mut self) -> Result<PlasticityBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Plasticity)?;
        self.skip_if(&TokenKind::Colon);
        let mut trigger = String::new();
        let mut modifies = String::new();
        let mut rule = PlasticityRule::Hebbian;

        while !self.is_at_end() && !self.check(&TokenKind::End) && !self.is_block_keyword() {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Trigger) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    let mut toks = Vec::new();
                    while !self.is_at_end() {
                        match self.peek().map(|t| &t.kind) {
                            Some(TokenKind::Modifies) | Some(TokenKind::End)
                            | Some(TokenKind::Rule) => break,
                            _ => {
                                if self.is_block_keyword() { break; }
                                toks.push(self.advance().unwrap().text);
                            }
                        }
                    }
                    trigger = toks.join(" ");
                }
                Some(TokenKind::Modifies) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    modifies = self.expect_ident().unwrap_or_default();
                }
                Some(TokenKind::Rule) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    let rule_name = self.expect_ident().unwrap_or_default();
                    rule = match rule_name.to_lowercase().as_str() {
                        "hebbian" => PlasticityRule::Hebbian,
                        "boltzmann" => PlasticityRule::Boltzmann,
                        "reinforcement_learning" | "rl" => PlasticityRule::ReinforcementLearning,
                        _ => PlasticityRule::Hebbian,
                    };
                }
                Some(TokenKind::Hebbian) => { self.advance(); rule = PlasticityRule::Hebbian; }
                Some(TokenKind::Boltzmann) => { self.advance(); rule = PlasticityRule::Boltzmann; }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(PlasticityBlock {
            trigger,
            modifies,
            rule,
            span: Span::new(start, self.current_span().end),
        })
    }

    // ── Ecosystem definition ──────────────────────────────────────────────────

    fn parse_ecosystem_def(&mut self) -> Result<EcosystemDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Ecosystem)?;
        let name = self.expect_ident()?;
        let mut describe = None;
        let mut members = Vec::new();
        let mut signals = Vec::new();
        let mut telos = None;
        let mut quorum_blocks = Vec::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Describe) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    describe = Some(self.parse_string_lit()?);
                }
                Some(TokenKind::Members) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    self.skip_if(&TokenKind::LBracket);
                    members.push(self.expect_ident()?);
                    while self.skip_if(&TokenKind::Comma) {
                        members.push(self.expect_ident()?);
                    }
                    self.skip_if(&TokenKind::RBracket);
                }
                Some(TokenKind::Signal) => {
                    signals.push(self.parse_signal_def()?);
                }
                Some(TokenKind::Telos) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    telos = Some(self.parse_string_lit()?);
                    self.skip_if(&TokenKind::End);
                }
                Some(TokenKind::Quorum) => {
                    quorum_blocks.push(self.parse_quorum_block()?);
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(EcosystemDef {
            name,
            describe,
            members,
            signals,
            telos,
            quorum_blocks,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_signal_def(&mut self) -> Result<SignalDef, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Signal)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::From)?;
        let from = self.expect_ident()?;
        self.expect(&TokenKind::To)?;
        let to = self.expect_ident()?;
        self.expect(&TokenKind::Payload)?;
        self.skip_if(&TokenKind::Colon);
        let payload_te = self.parse_type_expr()?;
        let payload = format!("{:?}", payload_te);
        self.skip_if(&TokenKind::End);

        Ok(SignalDef {
            name,
            from,
            to,
            payload,
            span: Span::new(start, self.current_span().end),
        })
    }

    fn parse_quorum_block(&mut self) -> Result<QuorumBlock, LoomError> {
        let start = self.current_span().start;
        self.expect(&TokenKind::Quorum)?;
        self.skip_if(&TokenKind::Colon);
        let mut signal = String::new();
        let mut threshold = String::new();
        let mut action = String::new();

        while !self.is_at_end() && !self.check(&TokenKind::End) {
            match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::Signal) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    signal = self.expect_ident()?;
                }
                Some(TokenKind::Threshold) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    // Store threshold as string regardless of whether it's float or ident
                    if let Some(t) = self.peek() {
                        threshold = t.text.clone();
                        self.advance();
                    }
                }
                Some(TokenKind::Action) => {
                    self.advance(); self.skip_if(&TokenKind::Colon);
                    action = self.expect_ident()?;
                }
                _ => { self.advance(); }
            }
        }
        self.skip_if(&TokenKind::End);

        Ok(QuorumBlock {
            signal,
            threshold,
            action,
            span: Span::new(start, self.current_span().end),
        })
    }
}

// ── Public parse entry point ──────────────────────────────────────────────────

/// Parse a token stream into a Module AST.
pub fn parse(tokens: Vec<Token>) -> Result<Module, LoomError> {
    let mut parser = Parser::new(&tokens);
    parser.parse_module()
}
