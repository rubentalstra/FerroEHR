//! The BEL recursive-descent parser, generic over a [`BelBuilder`].
//!
//! Grammar: `crates/openehr-lang/vendor/grammar/base_expressions.g4` (the
//! normative BEL syntax; `docs/specs/openehr/LANG/docs/BEL/masterAppA-syntax`).
//! Operator precedence follows that grammar: `implies` < `or` < `xor` < `and` <
//! `not` < comparison/`matches` < `+ -` < `* / %` < `^` < unary < primary.

use crate::bel::lexer::{Spanned, Token};
use crate::bel::{BelBuilder, BelError, BelLiteral};
use crate::beom::core::operator_kind::OperatorKind;

/// The parser cursor over a lexed token slice, driving a `&mut B` builder.
pub(crate) struct Parser<'a, 'b, B: BelBuilder> {
    src: &'a str,
    toks: &'a [Spanned],
    pos: usize,
    builder: &'b mut B,
}

impl<'a, 'b, B: BelBuilder> Parser<'a, 'b, B> {
    pub(crate) fn new(src: &'a str, toks: &'a [Spanned], builder: &'b mut B) -> Self {
        Self {
            src,
            toks,
            pos: 0,
            builder,
        }
    }

    // ── cursor helpers ────────────────────────────────────────────────────
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos).map(|s| &s.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.toks.get(self.pos + ahead).map(|s| &s.token)
    }

    fn at(&self) -> usize {
        self.toks
            .get(self.pos)
            .map_or(self.src.len(), |s| s.span.start)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, want: &Token) -> bool {
        if self.peek() == Some(want) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, BelError> {
        Err(BelError::Parse {
            at: self.at(),
            message: message.into(),
        })
    }

    // ── statements ────────────────────────────────────────────────────────
    /// `statement_block : statement+` — every statement until end of input.
    pub(crate) fn parse_statement_block(&mut self) -> Result<Vec<B::Stmt>, BelError> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            out.push(self.parse_statement()?);
        }
        Ok(out)
    }

    /// `statement : declaration | assignment | assertion`.
    fn parse_statement(&mut self) -> Result<B::Stmt, BelError> {
        match self.peek() {
            Some(Token::Variable(_)) => match self.peek_at(1) {
                Some(Token::Assign) => self.parse_assignment(),
                Some(Token::Colon) => self.parse_variable_declaration(),
                _ => self.parse_assertion(),
            },
            // `tag :` — a tagged assertion (constant declarations, which share
            // the `NAME : Type` shape, do not appear in archetype rules).
            Some(Token::LowerId(_) | Token::UpperId(_))
                if self.peek_at(1) == Some(&Token::Colon) =>
            {
                self.parse_assertion()
            }
            _ => self.parse_assertion(),
        }
    }

    /// `binding | local_assignment : local_variable ':=' ( bound_path | expression )`.
    fn parse_assignment(&mut self) -> Result<B::Stmt, BelError> {
        let name = self.expect_variable_name()?;
        if !self.eat(&Token::Assign) {
            return self.err("expected ':=' in assignment");
        }
        let source = self.parse_expr()?;
        Ok(self.builder.assignment(&name, source))
    }

    /// `variable_declaration : local_variable ':' type_id ( ':=' expression )?`.
    fn parse_variable_declaration(&mut self) -> Result<B::Stmt, BelError> {
        let name = self.expect_variable_name()?;
        if !self.eat(&Token::Colon) {
            return self.err("expected ':' in variable declaration");
        }
        let Some(Token::UpperId(type_id)) = self.bump() else {
            return self.err("expected a type name after ':' in a declaration");
        };
        let init = if self.eat(&Token::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(self.builder.variable_declaration(&name, &type_id, init))
    }

    /// `assertion : ( ( ALPHA_LC_ID | ALPHA_UC_ID ) ':' )? boolean_expr`.
    fn parse_assertion(&mut self) -> Result<B::Stmt, BelError> {
        let tag = match (self.peek(), self.peek_at(1)) {
            (Some(Token::LowerId(t) | Token::UpperId(t)), Some(Token::Colon)) => {
                let t = t.clone();
                self.pos += 2; // tag + ':'
                Some(t)
            }
            _ => None,
        };
        let expr = self.parse_expr()?;
        Ok(self.builder.assertion(tag, expr))
    }

    fn expect_variable_name(&mut self) -> Result<String, BelError> {
        match self.bump() {
            Some(Token::Variable(v)) => Ok(v.trim_start_matches('$').to_owned()),
            _ => self.err("expected a $variable"),
        }
    }

    // ── expressions (precedence climbing) ─────────────────────────────────
    /// `expression`/`boolean_expr` entry: lowest-precedence `implies`.
    fn parse_expr(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_or()?;
        while self.eat(&Token::Implies) {
            let right = self.parse_or()?;
            left = self.builder.binary(kind("implies"), "implies", left, right);
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_xor()?;
        while self.eat(&Token::Or) {
            let right = self.parse_xor()?;
            left = self.builder.binary(kind("or"), "or", left, right);
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_and()?;
        while self.eat(&Token::Xor) {
            let right = self.parse_and()?;
            left = self.builder.binary(kind("xor"), "xor", left, right);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_not()?;
        while self.eat(&Token::And) {
            let right = self.parse_not()?;
            left = self.builder.binary(kind("and"), "and", left, right);
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<B::Expr, BelError> {
        if self.eat(&Token::Not) {
            let operand = self.parse_not()?;
            return Ok(self.builder.unary(kind("not"), "not", operand));
        }
        self.parse_comparison()
    }

    /// A comparison / `matches` leaf over arithmetic operands.
    fn parse_comparison(&mut self) -> Result<B::Expr, BelError> {
        let left = self.parse_additive()?;
        if self.peek() == Some(&Token::Matches) {
            self.pos += 1;
            let (raw, at) = self.constraint_rhs()?;
            let rhs = self.builder.constraint(&raw, at)?;
            return Ok(self.builder.binary(kind("matches"), "matches", left, rhs));
        }
        if let Some((op, sym)) = self.peek().and_then(relational) {
            self.pos += 1;
            let right = self.parse_additive()?;
            return Ok(self.builder.binary(kind(op), sym, left, right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let (op, sym) = match self.peek() {
                Some(Token::Plus) => ("plus", "+"),
                Some(Token::Minus) => ("minus", "-"),
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_multiplicative()?;
            left = self.builder.binary(kind(op), sym, left, right);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<B::Expr, BelError> {
        let mut left = self.parse_exponent()?;
        loop {
            let (op, sym) = match self.peek() {
                Some(Token::Star) => ("multiply", "*"),
                Some(Token::Slash) => ("divide", "/"),
                Some(Token::Percent) => ("modulo", "%"),
                _ => break,
            };
            self.pos += 1;
            let right = self.parse_exponent()?;
            left = self.builder.binary(kind(op), sym, left, right);
        }
        Ok(left)
    }

    /// `^` — right-associative (`base_expressions.g4` `<assoc=right>`).
    fn parse_exponent(&mut self) -> Result<B::Expr, BelError> {
        let left = self.parse_unary()?;
        if self.eat(&Token::Caret) {
            let right = self.parse_exponent()?;
            return Ok(self.builder.binary(kind("exponent"), "^", left, right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<B::Expr, BelError> {
        if self.eat(&Token::Minus) {
            let operand = self.parse_unary()?;
            return Ok(self.builder.unary(kind("minus"), "-", operand));
        }
        if self.eat(&Token::Plus) {
            return self.parse_unary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<B::Expr, BelError> {
        match self.peek().cloned() {
            Some(Token::LParen) => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                if !self.eat(&Token::RParen) {
                    return self.err("expected ')'");
                }
                Ok(inner)
            }
            Some(Token::ForAll) => self.parse_for_all(),
            Some(Token::ThereExists) => self.parse_there_exists(),
            Some(Token::Exists) => {
                self.pos += 1;
                let operand = self.parse_ref_leaf()?;
                Ok(self.builder.unary(kind("exists"), "exists", operand))
            }
            Some(Token::True) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::Boolean(true)))
            }
            Some(Token::False) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::Boolean(false)))
            }
            Some(Token::Integer(s)) => {
                self.pos += 1;
                let v = s
                    .parse::<i64>()
                    .map_err(|e| self.parse_err(format!("invalid integer {s:?}: {e}")))?;
                Ok(self.builder.literal(BelLiteral::Integer(v)))
            }
            Some(Token::Real(s)) => {
                self.pos += 1;
                let v = s
                    .parse::<f64>()
                    .map_err(|e| self.parse_err(format!("invalid real {s:?}: {e}")))?;
                Ok(self.builder.literal(BelLiteral::Real(v)))
            }
            Some(Token::String(s)) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::String(decode_string(&s))))
            }
            Some(Token::Character(s)) => {
                self.pos += 1;
                let c = decode_char(&s);
                Ok(self.builder.literal(BelLiteral::Character(c)))
            }
            Some(Token::Date(s)) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::Date(s)))
            }
            Some(Token::Time(s)) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::Time(s)))
            }
            Some(Token::DateTime(s)) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::DateTime(s)))
            }
            Some(Token::Duration(s)) => {
                self.pos += 1;
                Ok(self.builder.literal(BelLiteral::Duration(s)))
            }
            Some(Token::LowerId(name)) if self.peek_at(1) == Some(&Token::LParen) => {
                self.parse_function_call(&name)
            }
            _ => self.parse_ref_leaf(),
        }
    }

    /// A reference leaf: `$variable`, `$var/path`, an archetype/data path, or a
    /// bare identifier used as a value reference.
    fn parse_ref_leaf(&mut self) -> Result<B::Expr, BelError> {
        match self.bump() {
            Some(Token::Variable(v)) => Ok(self.builder.variable_ref(v.trim_start_matches('$'))),
            Some(Token::VariableWithPath(v)) => self.builder.path_ref(&v),
            Some(Token::Path(p)) => self.builder.path_ref(&p),
            Some(Token::LowerId(id) | Token::UpperId(id)) => self.builder.path_ref(&id),
            _ => self.err("expected a value reference (path, $variable, or name)"),
        }
    }

    fn parse_function_call(&mut self, name: &str) -> Result<B::Expr, BelError> {
        self.pos += 1; // name
        self.pos += 1; // '('
        let mut args = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        if !self.eat(&Token::RParen) {
            return self.err("expected ')' closing function arguments");
        }
        Ok(self.builder.function_call(name, args))
    }

    /// `for_all_expr : SYM_FOR_ALL VARIABLE_ID ( ':' | 'in' ) value_ref '|'? boolean_expr`.
    fn parse_for_all(&mut self) -> Result<B::Expr, BelError> {
        self.pos += 1; // for_all
        let var = self.expect_variable_name()?;
        if !self.eat(&Token::Colon) && !self.eat(&Token::In) {
            return self.err("expected ':' or 'in' after the for_all variable");
        }
        let collection = self.parse_ref_leaf()?;
        self.eat(&Token::Bar); // optional '|'
        let condition = self.parse_expr()?;
        self.builder.for_all(&var, collection, condition)
    }

    /// `there_exists_expr` — existential quantification (mapped to the same
    /// quantifier node as `for_all` with an `exists` operator via the builder).
    ///
    /// The `∃` symbol is ALSO the symbolic rendering of the path-existence
    /// operator in ADL 1.4 assertions (ADL1.4 `master06-assertions.adoc`
    /// §Keywords equates `exists` ↔ `∃`), whose operand is a path, not a
    /// `$variable` binding — so a non-variable operand dispatches to the
    /// same unary the `exists` keyword builds.
    fn parse_there_exists(&mut self) -> Result<B::Expr, BelError> {
        self.pos += 1; // there_exists
        if !matches!(self.peek(), Some(Token::Variable(_))) {
            let operand = self.parse_ref_leaf()?;
            return Ok(self.builder.unary(kind("exists"), "exists", operand));
        }
        let var = self.expect_variable_name()?;
        if !self.eat(&Token::Colon) && !self.eat(&Token::In) {
            return self.err("expected ':' or 'in' after the there_exists variable");
        }
        let collection = self.parse_ref_leaf()?;
        self.eat(&Token::Bar);
        let condition = self.parse_expr()?;
        self.builder.for_all(&var, collection, condition)
    }

    /// Capture the verbatim source of a `matches` right-hand side: either a
    /// single `CONTAINED_REGEXP` token, or a `{ … }` block captured by
    /// brace-depth over the token stream. Returns `(raw_text, byte_offset)`.
    fn constraint_rhs(&mut self) -> Result<(String, usize), BelError> {
        if let Some(Token::ContainedRegexp(raw)) = self.peek().cloned() {
            let at = self.at();
            self.pos += 1;
            return Ok((raw, at));
        }
        let Some(open) = self.toks.get(self.pos) else {
            return self.err("expected '{' or a regex after 'matches'");
        };
        if open.token != Token::LCurly {
            return self.err("expected '{' or a regex after 'matches'");
        }
        let start = open.span.start;
        let mut depth = 0i32;
        while let Some(s) = self.toks.get(self.pos) {
            let end = s.span.end;
            if s.token == Token::LCurly {
                depth += 1;
            } else if s.token == Token::RCurly {
                depth -= 1;
            }
            self.pos += 1;
            if depth == 0 {
                let raw = self.src.get(start..end).unwrap_or_default().to_owned();
                return Ok((raw, start));
            }
        }
        self.err("unterminated '{ … }' constraint after 'matches'")
    }

    fn parse_err(&self, message: impl Into<String>) -> BelError {
        BelError::Parse {
            at: self.at(),
            message: message.into(),
        }
    }
}

/// An [`OperatorKind`] from its `OPERATOR_KIND` constant name
/// (`master04-expression_object_model.adoc`); an unknown token is tolerated as
/// `OperatorKind::Other`.
fn kind(name: &str) -> OperatorKind {
    OperatorKind::from_wire(name)
}

/// Map a relational/equality token to its `(operator_kind, symbol)`.
fn relational(t: &Token) -> Option<(&'static str, &'static str)> {
    match t {
        Token::Eq => Some(("eq", "=")),
        Token::Ne => Some(("ne", "!=")),
        Token::Lt => Some(("lt", "<")),
        Token::Le => Some(("le", "<=")),
        Token::Gt => Some(("gt", ">")),
        Token::Ge => Some(("ge", ">=")),
        _ => None,
    }
}

/// Decode a double-quoted BEL string literal (strip delimiters, unescape the
/// `master03` escapes `\r \n \t \\ \" \'`).
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    unescape(inner)
}

/// Decode a single-quoted character literal to a `char`.
fn decode_char(raw: &str) -> char {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);
    unescape(inner).chars().next().unwrap_or('\u{fffd}')
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('r') => out.push('\r'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                // `\\` and a trailing `\` both yield a literal backslash.
                Some('\\') | None => out.push('\\'),
                Some(other) => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    out
}
