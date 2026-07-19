//! The cADL definition-section parser (phase A3a).
//!
//! A hand-written recursive-descent parser over the A2 token stream
//! ([`crate::lexer`]), transcribed 1:1 from the vendored normative grammars
//! `crates/openehr-adl/vendor/grammar/{cadl2.g4, cadl2_primitives.g4}`. It
//! builds the **generated** AOM2 constraint model
//! (`openehr_am::am24::aom2::constraint_model`) directly — never a new model
//! type — producing a [`CComplexObject`] tree for a `definition` section body.
//!
//! Recursive descent (not `chumsky`) is the deliberate choice here: the cADL
//! primitive sub-grammar (`|…|` interval endpoints prefixed with relational
//! operators, the duration `pattern/interval` mix, kind-classification of an
//! interval by its endpoint token) is strongly context-sensitive and reads far
//! more clearly as straight-line code than as a combinator tree. It also
//! matches the existing outer parser idiom ([`crate::source`], hand-rolled RD
//! over `&[Spanned]`).
//!
//! Scope of A3a: full cADL object/attribute/tuple/slot/proxy/primitive
//! coverage building the AOM2 tree, with the `S*` syntax-validity codes raised
//! at position. Slot include/exclude **assertion** expressions and the `rules`
//! section are captured as raw text (BEL expression parsing is a later phase),
//! preserved in `ASSERTION.string_expression` so slots stay usable; the common
//! `archetype_id/value matches {/regex/}` form is additionally regex-compile
//! checked (`SCSRE`). Semantic (V-code) validation is a later phase.

use openehr_am::am24::aom2::constraint_model::archetype_slot::ArchetypeSlot;
use openehr_am::am24::aom2::constraint_model::c_archetype_root::CArchetypeRoot;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::am24::aom2::constraint_model::c_complex_object::{
    CComplexObject, CComplexObjectData,
};
use openehr_am::am24::aom2::constraint_model::c_complex_object_proxy::CComplexObjectProxy;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_tuple::CPrimitiveTuple;
use openehr_am::am24::aom2::constraint_model::primitive::c_boolean::CBoolean;
use openehr_am::am24::aom2::constraint_model::primitive::c_date::CDate;
use openehr_am::am24::aom2::constraint_model::primitive::c_date_time::CDateTime;
use openehr_am::am24::aom2::constraint_model::primitive::c_duration::CDuration;
use openehr_am::am24::aom2::constraint_model::primitive::c_integer::CInteger;
use openehr_am::am24::aom2::constraint_model::primitive::c_real::CReal;
use openehr_am::am24::aom2::constraint_model::primitive::c_string::CString;
use openehr_am::am24::aom2::constraint_model::primitive::c_terminology_code::CTerminologyCode;
use openehr_am::am24::aom2::constraint_model::primitive::c_time::CTime;
use openehr_am::am24::aom2::constraint_model::primitive::constraint_status::ConstraintStatus;
use openehr_am::am24::aom2::constraint_model::sibling_order::SiblingOrder;
use openehr_am::am24::beom::core::assertion::Assertion;
use openehr_base::prelude::{
    Cardinality, Interval, Iso8601Date, Iso8601DateTime, Iso8601Duration, Iso8601Time,
    MultiplicityInterval, PointInterval, ProperInterval, ProperIntervalData, TerminologyCode,
};

use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::lexer::{Spanned, Token};

/// Internal parse result: `Err(())` signals a bail-out; the concrete
/// [`SyntaxError`] is already recorded in [`Parser::errors`].
type PResult<T> = Result<T, ()>;

/// Parse a raw cADL `definition`-section body (the text between the
/// `definition` keyword and the next section) into a [`CComplexObject`].
///
/// This is the core entry point: it lexes `body` and runs the cADL grammar.
/// Error byte spans are relative to `body`.
///
/// # Errors
/// Returns every [`SyntaxError`] found (the `S*` catalogue codes of
/// `ADL2/master04.6`). Lexer failures surface as [`SyntaxErrorCode::Sunk`].
pub fn parse_definition_body(body: &str) -> Result<CComplexObject, Vec<SyntaxError>> {
    let toks = match crate::lexer::lex(body) {
        Ok(t) => t,
        Err(e) => return Err(vec![e]),
    };
    let mut parser = Parser {
        src: body,
        toks: &toks,
        pos: 0,
        errors: Vec::new(),
    };
    let root = parser.parse_root();
    match root {
        Ok(cco) if parser.errors.is_empty() => {
            if parser.pos < parser.toks.len() {
                let span = parser.span_at(parser.pos);
                parser.push(
                    SyntaxErrorCode::Sunk,
                    "unexpected trailing input after the definition object",
                    span,
                );
                Err(parser.errors)
            } else {
                Ok(cco)
            }
        }
        _ => {
            if parser.errors.is_empty() {
                parser.errors.push(SyntaxError::at(
                    SyntaxErrorCode::Sunk,
                    "empty definition",
                    0..0,
                    body,
                ));
            }
            Err(parser.errors)
        }
    }
}

/// Parse the **root artefact's** `definition` section of a whole ADL2 source
/// into a [`CComplexObject`].
///
/// Design note: [`crate::source::SourceArtefact`] stores only spans (byte +
/// token ranges), not the token stream or owned text, so this convenience
/// re-lexes just the definition body substring (`src[definition.bytes]`) via
/// [`parse_definition_body`] and re-offsets error spans back to the whole
/// file. Bodies are small, so the extra lex is cheap, and the A2
/// [`crate::source::parse_source`] API + error contract stay untouched.
/// To parse an overlay's definition, take `art.overlays[i].definition` and
/// call [`parse_definition_body`] on `&src[bytes]` directly.
///
/// # Errors
/// Returns the outer-parse errors if `src` does not outer-parse, a single
/// [`SyntaxErrorCode::Sadf`] if there is no definition section, or the cADL
/// errors (span-offset to the whole file) otherwise.
pub fn parse_definition(src: &str) -> Result<CComplexObject, Vec<SyntaxError>> {
    let artefact = crate::source::parse_source(src)?;
    let Some(def) = artefact.definition.as_ref() else {
        return Err(vec![SyntaxError::at(
            SyntaxErrorCode::Sadf,
            "no definition section",
            0..0,
            src,
        )]);
    };
    let body = src.get(def.bytes.clone()).unwrap_or_default();
    let offset = def.bytes.start;
    parse_definition_body(body).map_err(|errs| {
        errs.into_iter()
            .map(|e| {
                SyntaxError::at(
                    e.code,
                    e.message,
                    (e.span.start + offset)..(e.span.end + offset),
                    src,
                )
            })
            .collect()
    })
}

/// The recursive-descent cADL parser over a token slice.
struct Parser<'a> {
    src: &'a str,
    toks: &'a [Spanned],
    pos: usize,
    errors: Vec<SyntaxError>,
}

// ── cursor + error helpers ────────────────────────────────────────────────
impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos).map(|s| &s.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.toks.get(self.pos + ahead).map(|s| &s.token)
    }

    fn span_at(&self, idx: usize) -> std::ops::Range<usize> {
        self.toks
            .get(idx)
            .map_or(self.src.len()..self.src.len(), |s| s.span.clone())
    }

    fn cur_span(&self) -> std::ops::Range<usize> {
        self.span_at(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn push(
        &mut self,
        code: SyntaxErrorCode,
        msg: impl Into<String>,
        span: std::ops::Range<usize>,
    ) {
        self.errors.push(SyntaxError::at(code, msg, span, self.src));
    }

    /// Record an error at the current position and bail.
    fn err<T>(&mut self, code: SyntaxErrorCode, msg: impl Into<String>) -> PResult<T> {
        let span = self.cur_span();
        self.push(code, msg, span);
        Err(())
    }

    /// Consume a token matching `pred` or record `code`/`msg` and bail.
    fn expect(
        &mut self,
        pred: impl Fn(&Token) -> bool,
        code: SyntaxErrorCode,
        msg: &str,
    ) -> PResult<()> {
        if self.peek().is_some_and(&pred) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(code, msg.to_owned())
        }
    }

    fn eat(&mut self, pred: impl Fn(&Token) -> bool) -> bool {
        if self.peek().is_some_and(&pred) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

// ── productions ───────────────────────────────────────────────────────────
impl Parser<'_> {
    /// The definition root: a single `c_complex_object` (`cadl2.g4`).
    fn parse_root(&mut self) -> PResult<CComplexObject> {
        if !matches!(self.peek(), Some(Token::AlphaUcId(_))) {
            return self.err(
                SyntaxErrorCode::Sccog,
                "expected a root object type at the start of the definition",
            );
        }
        let obj = self.parse_type_object()?;
        match obj {
            CObject::CComplexObject(cco) => Ok(cco),
            _ => self.err(
                SyntaxErrorCode::Sccog,
                "definition root must be a complex object",
            ),
        }
    }

    /// `rm_type_id : ALPHA_UC_ID ( '<' rm_type_id ( ',' rm_type_id )* '>' )?`
    /// (`base_expressions.g4` `type_id`); returns the reconstructed type name.
    fn parse_rm_type_id(&mut self) -> PResult<String> {
        let Some(Token::AlphaUcId(base)) = self.peek().cloned() else {
            return self.err(
                SyntaxErrorCode::Sccog,
                "expected a reference-model type name",
            );
        };
        self.pos += 1;
        if !matches!(self.peek(), Some(Token::SymLt)) {
            return Ok(base);
        }
        // Generic type, e.g. `Interval<Quantity>`.
        self.pos += 1;
        let mut s = format!("{base}<");
        loop {
            s.push_str(&self.parse_rm_type_id()?);
            if self.eat(|t| matches!(t, Token::SymComma)) {
                s.push(',');
            } else {
                break;
            }
        }
        self.expect(
            |t| matches!(t, Token::SymGt),
            SyntaxErrorCode::Sccog,
            "expected '>' closing a generic type",
        )?;
        s.push('>');
        Ok(s)
    }

    /// A node id: `ROOT_ID_CODE | ID_CODE | AT_CODE` (`master04.2` dual coding
    /// — at-coded and id-coded archetypes both appear in this position).
    fn parse_node_id(&mut self) -> PResult<String> {
        match self.peek().cloned() {
            Some(Token::RootIdCode(c) | Token::IdCode(c) | Token::AtCode(c)) => {
                self.pos += 1;
                Ok(c)
            }
            _ => self.err(
                SyntaxErrorCode::Sccog,
                "expected a node id code (id-code or at-code)",
            ),
        }
    }

    /// `c_occurrences : SYM_OCCURRENCES SYM_MATCHES '{' multiplicity '}'`.
    fn parse_occurrences(&mut self) -> PResult<MultiplicityInterval> {
        self.pos += 1; // SYM_OCCURRENCES
        self.expect(
            |t| matches!(t, Token::SymMatches),
            SyntaxErrorCode::Soccf,
            "expecting 'matches' after 'occurrences'",
        )?;
        self.expect(
            |t| matches!(t, Token::LCurly),
            SyntaxErrorCode::Soccf,
            "expecting '{' in occurrences expression",
        )?;
        let m = self.parse_multiplicity(SyntaxErrorCode::Soccf)?;
        self.expect(
            |t| matches!(t, Token::RCurly),
            SyntaxErrorCode::Soccf,
            "expecting '}' closing occurrences",
        )?;
        Ok(m)
    }

    /// `multiplicity : INTEGER | '*' | INTEGER '..' ( INTEGER | '*' )`.
    fn parse_multiplicity(&mut self, code: SyntaxErrorCode) -> PResult<MultiplicityInterval> {
        if self.eat(|t| matches!(t, Token::SymStar)) {
            // `*` == 0..* .
            return Ok(mult(Some(0), None, false, true));
        }
        let lo = self.parse_uint(code)?;
        if self.eat(|t| matches!(t, Token::SymIvlSep)) {
            if self.eat(|t| matches!(t, Token::SymStar)) {
                return Ok(mult(Some(lo), None, false, true));
            }
            let up = self.parse_uint(code)?;
            return Ok(mult(Some(lo), Some(up), false, false));
        }
        Ok(mult(Some(lo), Some(lo), false, false))
    }

    fn parse_uint(&mut self, code: SyntaxErrorCode) -> PResult<i32> {
        match self.peek().cloned() {
            Some(Token::Integer(s)) => {
                self.pos += 1;
                s.parse::<i32>().map_err(|_| {
                    self.push(
                        code,
                        format!("invalid integer {s:?}"),
                        self.span_at(self.pos - 1),
                    );
                })
            }
            _ => self.err(code, "expected an integer"),
        }
    }

    /// A type-headed object: `c_complex_object` or `c_regular_primitive_object`
    /// (`cadl2.g4`). Distinguished by whether the `matches { … }` body (or the
    /// bare, body-less form) holds attribute defs or a single inline primitive.
    fn parse_type_object(&mut self) -> PResult<CObject> {
        let type_span = self.cur_span();
        let rm_type = self.parse_rm_type_id()?;
        // NOTE: `cadl2.g4` requires `'[' ID_CODE ']'`, but a *missing* node id
        // is a semantic defect (VCOID, `AOM2/master08`), not a syntax error —
        // the ADL Workbench parses it and flags VCOID in validation. We do the
        // same: an absent `[…]` yields an empty node id for the later phase.
        let node_id = if self.eat(|t| matches!(t, Token::LBracket)) {
            let n = self.parse_node_id()?;
            self.expect(
                |t| matches!(t, Token::RBracket),
                SyntaxErrorCode::Sccog,
                "expecting ']' after the node id",
            )?;
            n
        } else {
            String::new()
        };
        let occurrences = if matches!(self.peek(), Some(Token::SymOccurrences)) {
            Some(self.parse_occurrences()?)
        } else {
            None
        };

        let mut obj = if matches!(self.peek(), Some(Token::SymMatches)) {
            self.pos += 1; // SYM_MATCHES
            self.expect(
                |t| matches!(t, Token::LCurly),
                SyntaxErrorCode::Scoat,
                "expecting '{' after 'matches'",
            )?;
            if matches!(self.peek(), Some(Token::RCurly)) {
                // Empty object body — `expecting attribute definition(s)`.
                let span = self.cur_span();
                self.push(
                    SyntaxErrorCode::Scoat,
                    "expecting attribute definition(s)",
                    span,
                );
                return Err(());
            }
            if self.eat(|t| matches!(t, Token::SymStar)) {
                // Deprecated `matches {*}` == any (`master04.2`).
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Scoat,
                    "expecting '}' after '*'",
                )?;
                complex_object(
                    rm_type.clone(),
                    node_id.clone(),
                    Vec::new(),
                    Vec::new(),
                    None,
                )
            } else if self.body_is_inline_primitive() {
                let prim = self.parse_c_inline_primitive(node_id.clone())?;
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Scas,
                    "expecting '}' after the primitive constraint",
                )?;
                // A regular primitive object carries the declared RM type name.
                let mut prim = prim;
                common_mut(&mut prim).0.clone_from(&rm_type);
                prim
            } else {
                let (attrs, tuples, default) = self.parse_object_body()?;
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Scoat,
                    "expecting '}' closing the object body",
                )?;
                complex_object(rm_type.clone(), node_id.clone(), attrs, tuples, default)
            }
        } else if let Some(prim) = Self::primitive_any(&rm_type, &node_id) {
            // Body-less primitive type (e.g. `String[id2]`): an unconstrained
            // primitive object (`master04.5` regular primitive form).
            prim
        } else {
            // Body-less complex object: an "any" complex constraint.
            complex_object(
                rm_type.clone(),
                node_id.clone(),
                Vec::new(),
                Vec::new(),
                None,
            )
        };

        if occurrences.is_some() {
            *common_mut(&mut obj).2 = occurrences;
        }
        // Ensure the reconstructed type name / node id survive on complex objs.
        {
            let (ty, nid, _, _) = common_mut(&mut obj);
            if ty.is_empty() {
                *ty = rm_type;
            }
            if nid.is_empty() {
                *nid = node_id;
            }
        }
        let _ = type_span;
        Ok(obj)
    }

    /// The body of a complex object: `c_attribute_def+ default_value?`.
    /// Returns `(attributes, attribute_tuples, default_value)`.
    #[allow(clippy::type_complexity)]
    fn parse_object_body(
        &mut self,
    ) -> PResult<(
        Vec<CAttribute>,
        Vec<CAttributeTuple>,
        Option<serde_json::Value>,
    )> {
        let mut attrs = Vec::new();
        let mut tuples = Vec::new();
        let mut default = None;
        while !matches!(self.peek(), Some(Token::RCurly) | None) {
            match self.peek() {
                Some(Token::AlphaUnderscoreId(s)) if s == "_default" => {
                    default = Some(self.parse_default_value()?);
                    break; // default_value is last in the body (`cadl2.g4`).
                }
                Some(Token::LBracket) => tuples.push(self.parse_c_attribute_tuple()?),
                _ => attrs.push(self.parse_c_attribute()?),
            }
        }
        Ok((attrs, tuples, default))
    }

    /// `c_attribute : (ADL_PATH | rm_attribute_id) c_existence? c_cardinality?
    /// ( SYM_MATCHES ( '{' c_objects '}' | CONTAINED_REGEXP) )?`.
    fn parse_c_attribute(&mut self) -> PResult<CAttribute> {
        let (rm_attribute_name, differential_path) = match self.peek().cloned() {
            Some(Token::AdlPath(p)) => {
                self.pos += 1;
                let (parent, name) = split_diff_path(&p);
                (name, Some(parent))
            }
            Some(Token::AlphaLcId(a)) => {
                self.pos += 1;
                (a, None)
            }
            _ => {
                return self.err(
                    SyntaxErrorCode::Scoat,
                    "expecting an attribute name or differential path",
                );
            }
        };

        let existence = if matches!(self.peek(), Some(Token::SymExistence)) {
            Some(self.parse_existence()?)
        } else {
            None
        };
        let cardinality = if matches!(self.peek(), Some(Token::SymCardinality)) {
            Some(self.parse_cardinality()?)
        } else {
            None
        };
        let is_multiple = cardinality.is_some();

        let children = if self.eat(|t| matches!(t, Token::SymMatches)) {
            if let Some(Token::ContainedRegexp(raw)) = self.peek().cloned() {
                // `attr matches {/re/}` — a C_STRING regex shortcut (`cadl2.g4`).
                self.pos += 1;
                let span = self.span_at(self.pos - 1);
                let (regex, assumed) = self.contained_regexp_parts(&raw, span)?;
                vec![CObject::CString(cstring_regex(regex, assumed))]
            } else {
                self.expect(
                    |t| matches!(t, Token::LCurly),
                    SyntaxErrorCode::Scas,
                    "expecting '{' or a contained regexp after 'matches'",
                )?;
                let objs = self.parse_c_objects_body()?;
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Scas,
                    "expecting '}' closing the attribute body",
                )?;
                objs
            }
        } else {
            Vec::new()
        };

        Ok(CAttribute {
            parent: None,
            soc_parent: None,
            rm_attribute_name,
            existence,
            children,
            differential_path,
            cardinality,
            is_multiple,
        })
    }

    /// `c_existence : SYM_EXISTENCE SYM_MATCHES '{' existence '}'`;
    /// `existence : INTEGER | INTEGER '..' INTEGER`. Validated per the
    /// `SEXL*` rules (`master04.6`): only `{0}`,`{0..0}`,`{0..1}`,`{1}`,`{1..1}`.
    fn parse_existence(&mut self) -> PResult<MultiplicityInterval> {
        self.pos += 1; // SYM_EXISTENCE
        self.expect(
            |t| matches!(t, Token::SymMatches),
            SyntaxErrorCode::Sexlmg,
            "expecting 'matches' after 'existence'",
        )?;
        self.expect(
            |t| matches!(t, Token::LCurly),
            SyntaxErrorCode::Sexlmg,
            "expecting '{' in existence expression",
        )?;
        let body_span = self.cur_span();
        let lo = self.parse_uint(SyntaxErrorCode::Sexlmg)?;
        let hi = if self.eat(|t| matches!(t, Token::SymIvlSep)) {
            self.parse_uint(SyntaxErrorCode::Sexlmg)?
        } else {
            lo
        };
        self.expect(
            |t| matches!(t, Token::RCurly),
            SyntaxErrorCode::Sexlmg,
            "expecting '}' closing existence",
        )?;
        // Validity of the existence interval (`master04.6` §SEXL*).
        if lo == hi {
            if lo != 0 && lo != 1 {
                self.push(
                    SyntaxErrorCode::Sexlsg,
                    "existence single value must be 0 or 1",
                    body_span,
                );
                return Err(());
            }
        } else if lo == 0 {
            if hi != 1 {
                self.push(
                    SyntaxErrorCode::Sexlu1,
                    "existence upper limit must be 0 or 1 when lower is 0",
                    body_span,
                );
                return Err(());
            }
        } else if lo == 1 {
            if hi != 1 {
                self.push(
                    SyntaxErrorCode::Sexlu2,
                    "existence upper limit must be 1 when lower is 1",
                    body_span,
                );
                return Err(());
            }
        } else {
            self.push(
                SyntaxErrorCode::Sexlmg,
                "existence must be one of {0..0}, {0..1}, {1..1}",
                body_span,
            );
            return Err(());
        }
        Ok(mult(Some(lo), Some(hi), false, false))
    }

    /// `c_cardinality : SYM_CARDINALITY SYM_MATCHES '{' cardinality '}'`;
    /// `cardinality : multiplicity ( multiplicity_mod multiplicity_mod? )?`.
    fn parse_cardinality(&mut self) -> PResult<Cardinality> {
        self.pos += 1; // SYM_CARDINALITY
        self.expect(
            |t| matches!(t, Token::SymMatches),
            SyntaxErrorCode::Soccf,
            "expecting 'matches' after 'cardinality'",
        )?;
        self.expect(
            |t| matches!(t, Token::LCurly),
            SyntaxErrorCode::Soccf,
            "expecting '{' in cardinality expression",
        )?;
        let interval = self.parse_multiplicity(SyntaxErrorCode::Soccf)?;
        // NOTE: `ADL2/master04.3` — a container without an explicit ordering
        // modifier is an ordered list; uniqueness is off unless stated. Full
        // RM-driven defaulting is a later phase.
        let mut is_ordered = true;
        let mut is_unique = false;
        for _ in 0..2 {
            if self.eat(|t| matches!(t, Token::SymSemiColon)) {
                match self.peek() {
                    Some(Token::SymOrdered) => {
                        is_ordered = true;
                        self.pos += 1;
                    }
                    Some(Token::SymUnordered) => {
                        is_ordered = false;
                        self.pos += 1;
                    }
                    Some(Token::SymUnique) => {
                        is_unique = true;
                        self.pos += 1;
                    }
                    _ => {
                        return self.err(
                            SyntaxErrorCode::Soccf,
                            "expecting 'ordered', 'unordered', or 'unique' cardinality modifier",
                        );
                    }
                }
            } else {
                break;
            }
        }
        self.expect(
            |t| matches!(t, Token::RCurly),
            SyntaxErrorCode::Soccf,
            "expecting '}' closing cardinality",
        )?;
        Ok(Cardinality {
            interval,
            is_ordered,
            is_unique,
        })
    }

    /// `default_value : SYM_DEFAULT SYM_EQ '<' odin_text '>'` — the ODIN body
    /// is parsed by `openehr_lang::odin` and converted to canonical JSON.
    fn parse_default_value(&mut self) -> PResult<serde_json::Value> {
        self.pos += 1; // SYM_DEFAULT (`_default`)
        self.expect(
            |t| matches!(t, Token::SymEq),
            SyntaxErrorCode::Sadf,
            "expecting '=' after '_default'",
        )?;
        self.expect(
            |t| matches!(t, Token::SymLt),
            SyntaxErrorCode::Sadf,
            "expecting '<' opening the default value",
        )?;
        // Capture the balanced `<…>` ODIN body span (tracking `<`/`>` depth).
        let start = self.pos;
        let start_byte = self.cur_span().start;
        let mut depth = 1usize;
        let mut end_byte = start_byte;
        while depth > 0 {
            match self.bump() {
                Some(Token::SymLt) => depth += 1,
                Some(Token::SymGt) => {
                    depth -= 1;
                    if depth == 0 {
                        end_byte = self.span_at(self.pos - 1).start;
                    }
                }
                Some(_) => {}
                None => return self.err(SyntaxErrorCode::Sadf, "unterminated default value block"),
            }
        }
        let text = self.src.get(start_byte..end_byte).unwrap_or_default();
        let _ = start;
        match openehr_lang::odin::parse(text) {
            Ok(v) => Ok(odin_to_json(&v)),
            Err(e) => {
                self.push(
                    SyntaxErrorCode::Sadf,
                    format!("invalid ODIN default value: {e}"),
                    start_byte..end_byte,
                );
                Err(())
            }
        }
    }

    /// `c_attribute_tuple : '[' rm_attribute_id (',' rm_attribute_id)* ']'
    /// SYM_MATCHES '{' c_primitive_tuple (',' c_primitive_tuple)* '}'`.
    fn parse_c_attribute_tuple(&mut self) -> PResult<CAttributeTuple> {
        self.pos += 1; // '['
        let mut members = Vec::new();
        loop {
            let Some(Token::AlphaLcId(a)) = self.peek().cloned() else {
                return self.err(
                    SyntaxErrorCode::Scoat,
                    "expecting an attribute name in a tuple header",
                );
            };
            self.pos += 1;
            members.push(tuple_member(a));
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Scoat,
            "expecting ']' closing the tuple header",
        )?;
        self.expect(
            |t| matches!(t, Token::SymMatches),
            SyntaxErrorCode::Scoat,
            "expecting 'matches' after the tuple header",
        )?;
        self.expect(
            |t| matches!(t, Token::LCurly),
            SyntaxErrorCode::Scoat,
            "expecting '{' opening the tuple values",
        )?;
        let mut tuples = Vec::new();
        loop {
            tuples.push(self.parse_c_primitive_tuple()?);
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        self.expect(
            |t| matches!(t, Token::RCurly),
            SyntaxErrorCode::Scoat,
            "expecting '}' closing the tuple values",
        )?;
        Ok(CAttributeTuple { members, tuples })
    }

    /// `c_primitive_tuple : '[' c_primitive_tuple_item (',' …)* ']'`;
    /// `c_primitive_tuple_item : '{' c_inline_primitive_object '}' |
    /// CONTAINED_REGEXP`.
    fn parse_c_primitive_tuple(&mut self) -> PResult<CPrimitiveTuple> {
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Scoat,
            "expecting '[' opening a tuple row",
        )?;
        let mut items = Vec::new();
        loop {
            if let Some(Token::ContainedRegexp(raw)) = self.peek().cloned() {
                self.pos += 1;
                let span = self.span_at(self.pos - 1);
                let (regex, assumed) = self.contained_regexp_parts(&raw, span)?;
                items.push(CPrimitiveObject::CString(cstring_regex(regex, assumed)));
            } else {
                self.expect(
                    |t| matches!(t, Token::LCurly),
                    SyntaxErrorCode::Scoat,
                    "expecting '{' opening a tuple item",
                )?;
                let obj = self.parse_c_inline_primitive("Primitive_node_id".to_owned())?;
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Scoat,
                    "expecting '}' closing a tuple item",
                )?;
                let prim = cobject_to_primitive(obj).ok_or(())?;
                items.push(prim);
            }
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Scoat,
            "expecting ']' closing a tuple row",
        )?;
        Ok(CPrimitiveTuple { members: items })
    }

    /// `c_objects : c_regular_object_ordered+ | c_inline_primitive_object`,
    /// with the empty case raising `SCAS`.
    fn parse_c_objects_body(&mut self) -> PResult<Vec<CObject>> {
        if matches!(self.peek(), Some(Token::RCurly)) {
            return self.err(
                SyntaxErrorCode::Scas,
                "expecting a 'any', 'leaf', or new node definition",
            );
        }
        if self.eat(|t| matches!(t, Token::SymStar)) {
            return Ok(Vec::new()); // deprecated `matches {*}` == any.
        }
        if self.is_inline_primitive_start() {
            let obj = self.parse_c_inline_primitive("Primitive_node_id".to_owned())?;
            return Ok(vec![obj]);
        }
        let mut objs = Vec::new();
        while !matches!(self.peek(), Some(Token::RCurly) | None) {
            objs.push(self.parse_c_regular_object_ordered()?);
        }
        Ok(objs)
    }

    /// `c_regular_object_ordered : sibling_order? c_regular_object`.
    fn parse_c_regular_object_ordered(&mut self) -> PResult<CObject> {
        let sibling = if matches!(self.peek(), Some(Token::SymAfter | Token::SymBefore)) {
            Some(self.parse_sibling_order()?)
        } else {
            None
        };
        let mut obj = self.parse_c_regular_object()?;
        if sibling.is_some() {
            *common_mut(&mut obj).3 = sibling;
        }
        Ok(obj)
    }

    /// `sibling_order : ( SYM_AFTER | SYM_BEFORE ) '[' ID_CODE ']'`.
    fn parse_sibling_order(&mut self) -> PResult<SiblingOrder> {
        let is_before = matches!(self.peek(), Some(Token::SymBefore));
        self.pos += 1;
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Sccog,
            "expecting '[' after 'before'/'after'",
        )?;
        let sibling_node_id = self.parse_node_id()?;
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Sccog,
            "expecting ']' after the sibling node id",
        )?;
        Ok(SiblingOrder {
            is_before,
            sibling_node_id,
        })
    }

    /// `c_regular_object : c_complex_object | c_archetype_root |
    /// c_complex_object_proxy | archetype_slot | c_regular_primitive_object`.
    fn parse_c_regular_object(&mut self) -> PResult<CObject> {
        match self.peek() {
            Some(Token::SymUseArchetype) => self.parse_c_archetype_root(),
            Some(Token::SymUseNode) => self.parse_c_complex_object_proxy(),
            Some(Token::SymAllowArchetype) => self.parse_archetype_slot(),
            Some(Token::AlphaUcId(_)) => self.parse_type_object(),
            _ => self.err(
                SyntaxErrorCode::Sccog,
                "expecting a new node definition, primitive node, 'use' path, or archetype reference",
            ),
        }
    }

    /// `c_archetype_root : SYM_USE_ARCHETYPE rm_type_id '[' ID_CODE ','
    /// archetype_ref ']' c_occurrences?`.
    fn parse_c_archetype_root(&mut self) -> PResult<CObject> {
        self.pos += 1; // SYM_USE_ARCHETYPE
        let rm_type = self.parse_rm_type_id()?;
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Suas,
            "expecting '[' after 'use_archetype'",
        )?;
        // NOTE: `cadl2.g4` mandates `'[' ID_CODE ',' archetype_ref ']'`, but the
        // legacy ADL 1.5 form `use_archetype TYPE [archetype_id]` (no id-code)
        // also occurs; accept it with an empty node id (resolved on upgrade).
        let node_id = if matches!(
            self.peek(),
            Some(Token::IdCode(_) | Token::AtCode(_) | Token::RootIdCode(_))
        ) && matches!(self.peek_at(1), Some(Token::SymComma))
        {
            let n = self.parse_node_id()?;
            self.pos += 1; // ','
            n
        } else {
            String::new()
        };
        let archetype_ref = match self.peek().cloned() {
            Some(Token::ArchetypeId(a)) => {
                self.pos += 1;
                a
            }
            _ => {
                return self.err(
                    SyntaxErrorCode::Suaid,
                    "expecting [archetype_id] in 'use_archetype' statement",
                );
            }
        };
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Suaid,
            "expecting ']' after the archetype id",
        )?;
        let occurrences = if matches!(self.peek(), Some(Token::SymOccurrences)) {
            Some(self.parse_occurrences()?)
        } else {
            None
        };
        Ok(CObject::CComplexObject(CComplexObject::CArchetypeRoot(
            Box::new(CArchetypeRoot {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type,
                occurrences,
                node_id,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                attributes: Vec::new(),
                attribute_tuples: Vec::new(),
                archetype_ref,
            }),
        )))
    }

    /// `c_complex_object_proxy : SYM_USE_NODE rm_type_id '[' ID_CODE ']'
    /// c_occurrences? ADL_PATH`.
    fn parse_c_complex_object_proxy(&mut self) -> PResult<CObject> {
        self.pos += 1; // SYM_USE_NODE
        let rm_type = self.parse_rm_type_id()?;
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Sccog,
            "expecting '[' after 'use_node'",
        )?;
        let node_id = self.parse_node_id()?;
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Sccog,
            "expecting ']' after the node id",
        )?;
        let occurrences = if matches!(self.peek(), Some(Token::SymOccurrences)) {
            Some(self.parse_occurrences()?)
        } else {
            None
        };
        let target_path = match self.peek().cloned() {
            Some(Token::AdlPath(p)) => {
                self.pos += 1;
                p
            }
            _ => {
                return self.err(
                    SyntaxErrorCode::Sunpa,
                    "expecting an absolute path in 'use_node' statement",
                );
            }
        };
        Ok(CObject::CComplexObjectProxy(CComplexObjectProxy {
            parent: None,
            soc_parent: None,
            rm_type_name: rm_type,
            occurrences,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            target_path,
        }))
    }

    /// `archetype_slot : SYM_ALLOW_ARCHETYPE rm_type_id '[' ID_CODE ']'
    /// (( c_occurrences? ( SYM_MATCHES '{' c_includes? c_excludes? '}' )? ) |
    /// SYM_CLOSED )`.
    fn parse_archetype_slot(&mut self) -> PResult<CObject> {
        self.pos += 1; // SYM_ALLOW_ARCHETYPE
        let rm_type = self.parse_rm_type_id()?;
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Sccog,
            "expecting '[' after 'allow_archetype'",
        )?;
        let node_id = self.parse_node_id()?;
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Sccog,
            "expecting ']' after the node id",
        )?;

        let mut is_closed = false;
        let mut occurrences = None;
        let mut includes = Vec::new();
        let mut excludes = Vec::new();

        if self.eat(|t| matches!(t, Token::SymClosed)) {
            is_closed = true;
        } else {
            if matches!(self.peek(), Some(Token::SymOccurrences)) {
                occurrences = Some(self.parse_occurrences()?);
            }
            if self.eat(|t| matches!(t, Token::SymMatches)) {
                self.expect(
                    |t| matches!(t, Token::LCurly),
                    SyntaxErrorCode::Sccog,
                    "expecting '{' after 'matches' in a slot",
                )?;
                if self.eat(|t| matches!(t, Token::SymInclude)) {
                    includes.push(self.parse_slot_assertion()?);
                }
                if self.eat(|t| matches!(t, Token::SymExclude)) {
                    excludes.push(self.parse_slot_assertion()?);
                }
                self.expect(
                    |t| matches!(t, Token::RCurly),
                    SyntaxErrorCode::Sccog,
                    "expecting '}' closing the slot body",
                )?;
            }
        }

        Ok(CObject::ArchetypeSlot(ArchetypeSlot {
            parent: None,
            soc_parent: None,
            rm_type_name: rm_type,
            occurrences,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            includes,
            excludes,
            is_closed,
        }))
    }

    /// Capture one slot include/exclude assertion as a raw span (BEL parsing is
    /// a later phase; `master04.6`). The raw text is preserved in
    /// `ASSERTION.string_expression`; the common `archetype_id/value matches
    /// {/regex/}` form is regex-compile checked (`SCSRE`).
    ///
    /// NOTE: multiple assertions in one `include`/`exclude` block are captured
    /// as a single raw span here (the token run to the next `exclude`/`}` at
    /// brace-depth 0); splitting them requires the BEL expression grammar,
    /// which lands in a later phase.
    fn parse_slot_assertion(&mut self) -> PResult<Assertion> {
        let start = self.pos;
        let start_byte = self.cur_span().start;
        let mut end_byte = start_byte;
        let mut depth = 0i32;
        while let Some(tok) = self.peek() {
            match tok {
                Token::LCurly => depth += 1,
                Token::RCurly | Token::SymExclude if depth == 0 => break,
                Token::RCurly => depth -= 1,
                _ => {}
            }
            end_byte = self.cur_span().end;
            self.pos += 1;
        }
        if self.pos == start {
            return self.err(
                SyntaxErrorCode::Sccog,
                "expecting an assertion after 'include'/'exclude'",
            );
        }
        let text = self.src.get(start_byte..end_byte).unwrap_or_default();
        // Parse the real assertion tree (`EXPR_ARCHETYPE_REF matches
        // EXPR_ARCHETYPE_ID_CONSTRAINT`, `master05` / `master04.3`) via the BEL
        // AOM composition; `string_expression` keeps the verbatim source.
        match crate::rules::parse_slot_assertion(text) {
            Ok(assertion) => Ok(assertion),
            Err(errs) => {
                for e in errs {
                    self.errors.push(SyntaxError::at(
                        e.code,
                        e.message,
                        (e.span.start + start_byte)..(e.span.end + start_byte),
                        self.src,
                    ));
                }
                Err(())
            }
        }
    }
}

// ── inline primitives (`cadl2_primitives.g4`) ─────────────────────────────
impl Parser<'_> {
    /// True if a type-object `matches { … }` body is a single inline primitive
    /// (a `c_regular_primitive_object`) rather than a complex object. Disambiguates
    /// the leading `[`: an object body's `[attr, …]` is a `c_attribute_tuple`
    /// (complex), whereas `[ac…]`/`[at…]` is a `c_terminology_code` (primitive).
    fn body_is_inline_primitive(&self) -> bool {
        match self.peek() {
            Some(Token::LBracket) => {
                matches!(self.peek_at(1), Some(Token::AcCode(_) | Token::AtCode(_)))
            }
            Some(Token::AlphaLcId(s)) => {
                is_strength_keyword(s) && matches!(self.peek_at(1), Some(Token::LBracket))
            }
            _ => self.is_inline_primitive_start(),
        }
    }

    /// True if the current token begins a `c_inline_primitive_object`.
    fn is_inline_primitive_start(&self) -> bool {
        match self.peek() {
            Some(
                Token::SymTrue
                | Token::SymFalse
                | Token::String(_)
                | Token::Integer(_)
                | Token::Real(_)
                | Token::Iso8601Date(_)
                | Token::Iso8601Time(_)
                | Token::Iso8601DateTime(_)
                | Token::Iso8601Duration(_)
                | Token::DateConstraintPattern(_)
                | Token::TimeConstraintPattern(_)
                | Token::DateTimeConstraintPattern(_)
                | Token::DurationConstraintPattern(_)
                | Token::SymIvlDelim
                | Token::LBracket
                | Token::SymPlus
                | Token::SymMinus,
            ) => true,
            Some(Token::AlphaLcId(s)) => is_strength_keyword(s),
            _ => false,
        }
    }

    /// `c_inline_primitive_object` (`cadl2_primitives.g4`): dispatch on the
    /// leading token; `node_id` is `Primitive_node_id` for inline forms.
    fn parse_c_inline_primitive(&mut self, node_id: String) -> PResult<CObject> {
        match self.peek().cloned() {
            Some(Token::SymTrue | Token::SymFalse) => self.parse_c_boolean(node_id),
            Some(Token::String(_)) => self.parse_c_string(node_id),
            Some(Token::LBracket) => self.parse_c_terminology_code(node_id, None),
            Some(Token::AlphaLcId(s)) if is_strength_keyword(&s) => {
                self.pos += 1;
                self.parse_c_terminology_code(node_id, Some(strength_status(&s)))
            }
            Some(Token::DateConstraintPattern(_) | Token::Iso8601Date(_)) => {
                self.parse_c_date(node_id)
            }
            Some(Token::TimeConstraintPattern(_) | Token::Iso8601Time(_)) => {
                self.parse_c_time(node_id)
            }
            Some(Token::DateTimeConstraintPattern(_) | Token::Iso8601DateTime(_)) => {
                self.parse_c_date_time(node_id)
            }
            Some(Token::DurationConstraintPattern(_) | Token::Iso8601Duration(_)) => {
                self.parse_c_duration(node_id)
            }
            Some(Token::Real(_)) => self.parse_c_real(node_id),
            Some(Token::Integer(_)) => self.parse_c_integer(node_id),
            Some(Token::SymIvlDelim) => match self.classify_bar_kind()? {
                PrimKind::Integer => self.parse_c_integer(node_id),
                PrimKind::Real => self.parse_c_real(node_id),
                PrimKind::Date => self.parse_c_date(node_id),
                PrimKind::Time => self.parse_c_time(node_id),
                PrimKind::DateTime => self.parse_c_date_time(node_id),
                PrimKind::Duration => self.parse_c_duration(node_id),
            },
            Some(Token::SymPlus | Token::SymMinus) => {
                if matches!(self.peek_at(1), Some(Token::Real(_))) {
                    self.parse_c_real(node_id)
                } else {
                    self.parse_c_integer(node_id)
                }
            }
            _ => self.err(
                SyntaxErrorCode::Sccog,
                "expecting a primitive constraint value",
            ),
        }
    }

    /// Peek inside a `|…|` interval to classify its value kind by the first
    /// endpoint token (skipping relational operators and signs).
    fn classify_bar_kind(&mut self) -> PResult<PrimKind> {
        let mut i = self.pos + 1;
        while matches!(
            self.toks.get(i).map(|s| &s.token),
            Some(
                Token::SymGt
                    | Token::SymGe
                    | Token::SymLt
                    | Token::SymLe
                    | Token::SymPlus
                    | Token::SymMinus
            )
        ) {
            i += 1;
        }
        match self.toks.get(i).map(|s| &s.token) {
            Some(Token::Integer(_)) => Ok(PrimKind::Integer),
            Some(Token::Real(_)) => Ok(PrimKind::Real),
            Some(Token::Iso8601Date(_)) => Ok(PrimKind::Date),
            Some(Token::Iso8601Time(_)) => Ok(PrimKind::Time),
            Some(Token::Iso8601DateTime(_)) => Ok(PrimKind::DateTime),
            Some(Token::Iso8601Duration(_)) => Ok(PrimKind::Duration),
            _ => self.err(SyntaxErrorCode::Sccog, "unrecognised interval value type"),
        }
    }

    fn parse_c_boolean(&mut self, node_id: String) -> PResult<CObject> {
        let mut constraint = Vec::new();
        loop {
            match self.peek() {
                Some(Token::SymTrue) => {
                    constraint.push(true);
                    self.pos += 1;
                }
                Some(Token::SymFalse) => {
                    constraint.push(false);
                    self.pos += 1;
                }
                _ => return self.err(SyntaxErrorCode::Scbav, "expecting 'True' or 'False'"),
            }
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            Some(match self.peek() {
                Some(Token::SymTrue) => {
                    self.pos += 1;
                    true
                }
                Some(Token::SymFalse) => {
                    self.pos += 1;
                    false
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Scbav,
                        "assumed value must be 'True' or 'False'",
                    );
                }
            })
        } else {
            None
        };
        Ok(CObject::CBoolean(CBoolean {
            parent: None,
            soc_parent: None,
            rm_type_name: "Boolean".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
        }))
    }

    fn parse_c_string(&mut self, node_id: String) -> PResult<CObject> {
        let mut constraint = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::String(s)) => {
                    self.pos += 1;
                    constraint.push(decode_string(&s));
                }
                _ => return self.err(SyntaxErrorCode::Scsav, "expecting a string value"),
            }
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::String(s)) => {
                    self.pos += 1;
                    Some(decode_string(&s))
                }
                _ => return self.err(SyntaxErrorCode::Scsav, "assumed value must be a string"),
            }
        } else {
            None
        };
        Ok(CObject::CString(CString {
            parent: None,
            soc_parent: None,
            rm_type_name: "String".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
        }))
    }

    /// `c_terminology_code : '[' ( AC_CODE ( ';' AT_CODE )? | AT_CODE ) ']'`
    /// (`cadl2_primitives.g4`), extended with the constraint-strength prefix
    /// (`master04.5` §Constraint strengths) and the OPT operational-binding
    /// `[codeN@terminology]` form (`master04.5` §Operational Binding
    /// Constraints — beyond the pinned grammar, but spec-normative).
    fn parse_c_terminology_code(
        &mut self,
        node_id: String,
        constraint_status: Option<ConstraintStatus>,
    ) -> PResult<CObject> {
        self.expect(
            |t| matches!(t, Token::LBracket),
            SyntaxErrorCode::Stccp,
            "expecting '[' opening a terminology code",
        )?;
        let mut constraint = match self.peek().cloned() {
            Some(Token::AcCode(c) | Token::AtCode(c)) => {
                self.pos += 1;
                c
            }
            _ => return self.err(SyntaxErrorCode::Stccp, "expecting an ac-code or at-code"),
        };
        // Optional OPT operational binding `@terminology`.
        if self.eat(|t| matches!(t, Token::SymAt)) {
            match self.peek().cloned() {
                Some(Token::AlphaLcId(t) | Token::AlphaUcId(t)) => {
                    self.pos += 1;
                    constraint = format!("{constraint}@{t}");
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Stccp,
                        "expecting a terminology id after '@'",
                    );
                }
            }
        }
        // Optional assumed at-code (only valid after an ac-code).
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::AtCode(a)) => {
                    self.pos += 1;
                    Some(local_term_code(&a))
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Stccp,
                        "assumed terminology value must be an at-code",
                    );
                }
            }
        } else {
            None
        };
        self.expect(
            |t| matches!(t, Token::RBracket),
            SyntaxErrorCode::Stccp,
            "expecting ']' closing a terminology code",
        )?;
        Ok(CObject::CTerminologyCode(CTerminologyCode {
            parent: None,
            soc_parent: None,
            rm_type_name: "Terminology_code".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
            constraint_status,
        }))
    }

    fn parse_c_integer(&mut self, node_id: String) -> PResult<CObject> {
        let constraint = self.parse_value_list::<i32>()?;
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            Some(f64::from(self.parse_signed_int(SyntaxErrorCode::Sciav)?))
        } else {
            None
        };
        Ok(CObject::CInteger(CInteger {
            parent: None,
            soc_parent: None,
            rm_type_name: "Integer".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
        }))
    }

    fn parse_c_real(&mut self, node_id: String) -> PResult<CObject> {
        let constraint = self.parse_value_list::<f64>()?;
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            Some(self.parse_signed_real(SyntaxErrorCode::Scrav)?)
        } else {
            None
        };
        Ok(CObject::CReal(CReal {
            parent: None,
            soc_parent: None,
            rm_type_name: "Real".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
        }))
    }

    fn parse_c_date(&mut self, node_id: String) -> PResult<CObject> {
        let mut pattern_constraint = None;
        let mut constraint = Vec::new();
        if let Some(Token::DateConstraintPattern(p)) = self.peek().cloned() {
            self.pos += 1;
            self.validate_date_pattern(&p, SyntaxErrorCode::Scdpt)?;
            pattern_constraint = Some(p);
        } else {
            constraint = self.parse_value_list::<Iso8601Date>()?;
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::Iso8601Date(v)) => {
                    self.pos += 1;
                    Some(Iso8601Date { value: v })
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Scdav,
                        "assumed value must be an ISO8601 date",
                    );
                }
            }
        } else {
            None
        };
        Ok(CObject::CDate(CDate {
            parent: None,
            soc_parent: None,
            rm_type_name: "Iso8601_date".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
            pattern_constraint,
        }))
    }

    fn parse_c_time(&mut self, node_id: String) -> PResult<CObject> {
        let mut pattern_constraint = None;
        let mut constraint = Vec::new();
        if let Some(Token::TimeConstraintPattern(p)) = self.peek().cloned() {
            self.pos += 1;
            self.validate_time_pattern(&p, SyntaxErrorCode::Sctpt)?;
            pattern_constraint = Some(p);
        } else {
            constraint = self.parse_value_list::<Iso8601Time>()?;
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::Iso8601Time(v)) => {
                    self.pos += 1;
                    Some(Iso8601Time { value: v })
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Sctav,
                        "assumed value must be an ISO8601 time",
                    );
                }
            }
        } else {
            None
        };
        Ok(CObject::CTime(CTime {
            parent: None,
            soc_parent: None,
            rm_type_name: "Iso8601_time".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
            pattern_constraint,
        }))
    }

    fn parse_c_date_time(&mut self, node_id: String) -> PResult<CObject> {
        let mut pattern_constraint = None;
        let mut constraint = Vec::new();
        if let Some(Token::DateTimeConstraintPattern(p)) = self.peek().cloned() {
            self.pos += 1;
            self.validate_date_time_pattern(&p, SyntaxErrorCode::Scdtpt)?;
            pattern_constraint = Some(p);
        } else {
            constraint = self.parse_value_list::<Iso8601DateTime>()?;
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::Iso8601DateTime(v)) => {
                    self.pos += 1;
                    Some(Iso8601DateTime { value: v })
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Scdtav,
                        "assumed value must be an ISO8601 date/time",
                    );
                }
            }
        } else {
            None
        };
        Ok(CObject::CDateTime(CDateTime {
            parent: None,
            soc_parent: None,
            rm_type_name: "Iso8601_date_time".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
            pattern_constraint,
        }))
    }

    /// `c_duration : ( DURATION_CONSTRAINT_PATTERN ( '/' ( duration_interval |
    /// duration ))? | value/list/interval ) assumed?` (`cadl2_primitives.g4`).
    fn parse_c_duration(&mut self, node_id: String) -> PResult<CObject> {
        let mut pattern_constraint = None;
        let mut constraint = Vec::new();
        if let Some(Token::DurationConstraintPattern(p)) = self.peek().cloned() {
            self.pos += 1;
            self.validate_duration_pattern(&p, SyntaxErrorCode::Scdupt)?;
            pattern_constraint = Some(p);
            if self.eat(|t| matches!(t, Token::SymSlash)) {
                // `pattern/interval` or `pattern/value` mixed form.
                constraint.push(self.parse_value_item::<Iso8601Duration>()?);
            }
        } else {
            constraint = self.parse_value_list::<Iso8601Duration>()?;
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::Iso8601Duration(v)) => {
                    self.pos += 1;
                    Some(Iso8601Duration { value: v })
                }
                _ => {
                    return self.err(
                        SyntaxErrorCode::Scduav,
                        "assumed value must be an ISO8601 duration",
                    );
                }
            }
        } else {
            None
        };
        Ok(CObject::CDuration(CDuration {
            parent: None,
            soc_parent: None,
            rm_type_name: "Iso8601_duration".to_owned(),
            occurrences: None,
            node_id,
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint,
            pattern_constraint,
        }))
    }

    /// Parse a comma-separated list of `Interval<V>` items (a value list, an
    /// interval, or a list of intervals — the AOM2 constraint is a flat
    /// `Vec<Interval<V>>` regardless).
    fn parse_value_list<V: CadlValue>(&mut self) -> PResult<Vec<Interval<V>>> {
        let mut out = Vec::new();
        loop {
            out.push(self.parse_value_item::<V>()?);
            if !self.eat(|t| matches!(t, Token::SymComma)) {
                break;
            }
        }
        Ok(out)
    }

    /// One `Interval<V>` item: a bare value (point interval) or a `|…|`
    /// interval.
    fn parse_value_item<V: CadlValue>(&mut self) -> PResult<Interval<V>> {
        if matches!(self.peek(), Some(Token::SymIvlDelim)) {
            self.parse_bar_interval::<V>()
        } else {
            let v = V::parse_one(self)?;
            Ok(point_interval(v))
        }
    }

    /// A `|…|` interval (`odin_values.g4` interval forms): two-sided
    /// (`|a..b|`, `|>a..<b|`), single-relop (`|>a|`,`|<=a|`), point (`|a|`),
    /// or centre±delta (`|a+/-b|`).
    fn parse_bar_interval<V: CadlValue>(&mut self) -> PResult<Interval<V>> {
        self.pos += 1; // opening '|'
        let lower_rel = self.eat_relop();
        let first = V::parse_one(self)?;
        let ivl = if self.eat(|t| matches!(t, Token::SymIvlSep)) {
            // Two-sided: [rel] first '..' ['<'] upper.
            let upper_excl = self.eat(|t| matches!(t, Token::SymLt));
            let upper = V::parse_one(self)?;
            let lower_included = !matches!(lower_rel, Some(Relop::Gt));
            proper_interval(
                Some(first),
                Some(upper),
                lower_included,
                !upper_excl,
                false,
                false,
            )
        } else if self.eat(|t| matches!(t, Token::SymPlusOrMinus)) {
            let delta = V::parse_one(self)?;
            match V::plus_minus(&first, &delta) {
                Some((lo, hi)) => proper_interval(Some(lo), Some(hi), true, true, false, false),
                // NOTE: `±` on a non-numeric type is not reducible without RM
                // type context; represented as a point at the centre for now
                // (rare — not exercised by the primitive corpus).
                None => point_interval(first),
            }
        } else {
            // Point `|a|` or single-relop `|>a|`,`|<=a|`.
            match lower_rel {
                None => point_interval(first),
                Some(Relop::Gt) => proper_interval(Some(first), None, false, false, false, true),
                Some(Relop::Ge) => proper_interval(Some(first), None, true, false, false, true),
                Some(Relop::Lt) => proper_interval(None, Some(first), false, false, true, false),
                Some(Relop::Le) => proper_interval(None, Some(first), false, true, true, false),
            }
        };
        self.expect(
            |t| matches!(t, Token::SymIvlDelim),
            SyntaxErrorCode::Sccog,
            "expecting '|' closing the interval",
        )?;
        Ok(ivl)
    }

    fn eat_relop(&mut self) -> Option<Relop> {
        let r = match self.peek() {
            Some(Token::SymGt) => Relop::Gt,
            Some(Token::SymGe) => Relop::Ge,
            Some(Token::SymLt) => Relop::Lt,
            Some(Token::SymLe) => Relop::Le,
            _ => return None,
        };
        self.pos += 1;
        Some(r)
    }

    fn parse_signed_int(&mut self, code: SyntaxErrorCode) -> PResult<i32> {
        let neg = self.eat(|t| matches!(t, Token::SymMinus));
        if !neg {
            self.eat(|t| matches!(t, Token::SymPlus));
        }
        match self.peek().cloned() {
            Some(Token::Integer(s)) => {
                self.pos += 1;
                let v = s.parse::<i64>().map_err(|_| {
                    self.push(
                        code,
                        format!("invalid integer {s:?}"),
                        self.span_at(self.pos - 1),
                    );
                })?;
                let v = if neg { -v } else { v };
                i32::try_from(v).map_err(|_| {
                    self.push(
                        code,
                        format!("integer {v} out of range"),
                        self.span_at(self.pos - 1),
                    );
                })
            }
            _ => self.err(code, "expecting an integer value"),
        }
    }

    fn parse_signed_real(&mut self, code: SyntaxErrorCode) -> PResult<f64> {
        let neg = self.eat(|t| matches!(t, Token::SymMinus));
        if !neg {
            self.eat(|t| matches!(t, Token::SymPlus));
        }
        match self.peek().cloned() {
            Some(Token::Real(s)) => {
                self.pos += 1;
                let v = s.parse::<f64>().map_err(|_| {
                    self.push(
                        code,
                        format!("invalid real {s:?}"),
                        self.span_at(self.pos - 1),
                    );
                })?;
                Ok(if neg { -v } else { v })
            }
            _ => self.err(code, "expecting a real value (with a decimal point)"),
        }
    }

    /// Split a `CONTAINED_REGEXP` token (`{ /re/ [;"assumed"] }` or the `^re^`
    /// form) into `(regex-with-/-delims, assumed?)`, compile-checking it.
    fn contained_regexp_parts(
        &mut self,
        raw: &str,
        span: std::ops::Range<usize>,
    ) -> PResult<(String, Option<String>)> {
        let body = raw
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        // Optional `;"assumed"` suffix.
        let (regex_part, assumed) = match body.split_once(';') {
            Some((r, a)) => {
                let a = a.trim();
                let assumed = a
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .map(decode_string_inner);
                (r.trim(), assumed)
            }
            None => (body, None),
        };
        let inner = regex_inner(regex_part);
        if regex::Regex::new(inner).is_err() {
            self.push(
                SyntaxErrorCode::Scsre,
                format!("{regex_part:?} is not a valid regular expression"),
                span,
            );
            return Err(());
        }
        Ok((format!("/{inner}/"), assumed))
    }

    // ── constraint-pattern validators (`master04.5` valid-pattern tables) ──

    fn validate_date_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        // Fields: year(4)-month(2)-day(2). Degradation: after a `??` field, only
        // `??`/`XX`; after `XX`, only `XX`.
        let fields: Vec<&str> = p.split('-').collect();
        if fields.len() != 3
            || !fields[0].eq_ignore_ascii_case("yyyy") && !fields[0].eq_ignore_ascii_case("yyy")
        {
            return self.pattern_err(code, p);
        }
        self.validate_pattern_degradation(&fields[1..], code, p)
    }

    fn validate_time_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        let time_core = p.split(['+', '\u{00B1}', 'Z']).next().unwrap_or(p);
        let fields: Vec<&str> = time_core.split(':').collect();
        if fields.len() != 3 || !is_present_field(fields[0], "hh") {
            return self.pattern_err(code, p);
        }
        self.validate_pattern_degradation(&fields[1..], code, p)
    }

    fn validate_date_time_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        let Some((date, time)) = p.split_once('T') else {
            return self.pattern_err(code, p);
        };
        let date_fields: Vec<&str> = date.split('-').collect();
        let time_core = time.split(['+', '\u{00B1}', 'Z']).next().unwrap_or(time);
        let time_fields: Vec<&str> = time_core.split(':').collect();
        if date_fields.len() != 3
            || time_fields.len() != 3
            || !(date_fields[0].eq_ignore_ascii_case("yyyy")
                || date_fields[0].eq_ignore_ascii_case("yyy"))
        {
            return self.pattern_err(code, p);
        }
        // Degradation flows date → time as one chain (`master04.5`): the hour
        // field may itself be `??`/`XX` once the date has degraded.
        self.validate_pattern_degradation(
            &[
                date_fields[1],
                date_fields[2],
                time_fields[0],
                time_fields[1],
                time_fields[2],
            ],
            code,
            p,
        )
    }

    /// Duration designator-order check: `P[Y][M][W][D][T[H][M][S]]`
    /// (`master04.6` §SCDUPT). The lexer already enforces order; this catches
    /// an empty pattern.
    fn validate_duration_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        let up = p.to_ascii_uppercase();
        if !up.starts_with('P') || up == "P" || up == "PT" {
            return self.pattern_err(code, p);
        }
        Ok(())
    }

    /// After a `??` (optional) field only `??`/`XX` may follow; after an `XX`
    /// (not-allowed) field only `XX`.
    fn validate_pattern_degradation(
        &mut self,
        fields: &[&str],
        code: SyntaxErrorCode,
        full: &str,
    ) -> PResult<()> {
        let mut seen_optional = false;
        let mut seen_absent = false;
        for f in fields {
            let is_absent = f.eq_ignore_ascii_case("XX");
            let is_optional = *f == "??";
            let is_present = !is_absent && !is_optional;
            if seen_absent && !is_absent {
                return self.pattern_err(code, full);
            }
            if seen_optional && is_present {
                return self.pattern_err(code, full);
            }
            seen_optional |= is_optional;
            seen_absent |= is_absent;
        }
        Ok(())
    }

    fn pattern_err<T>(&mut self, code: SyntaxErrorCode, p: &str) -> PResult<T> {
        let span = self.span_at(self.pos.saturating_sub(1));
        self.push(code, format!("invalid constraint pattern {p:?}"), span);
        Err(())
    }

    /// A body-less type-headed primitive object (`String[id2]`): recognise the
    /// fixed set of ADL primitive RM type names (`master04.5`) and build an
    /// unconstrained primitive; non-primitive names fall through to a complex
    /// object.
    #[allow(clippy::too_many_lines)] // one arm per primitive C_* struct literal
    fn primitive_any(rm_type: &str, node_id: &str) -> Option<CObject> {
        let nid = node_id.to_owned();
        let obj = match rm_type {
            "Boolean" => CObject::CBoolean(CBoolean {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
            }),
            "String" => CObject::CString(CString {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
            }),
            "Integer" => CObject::CInteger(CInteger {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
            }),
            "Real" => CObject::CReal(CReal {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
            }),
            "Iso8601_date" => CObject::CDate(CDate {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
                pattern_constraint: None,
            }),
            "Iso8601_time" => CObject::CTime(CTime {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
                pattern_constraint: None,
            }),
            "Iso8601_date_time" => CObject::CDateTime(CDateTime {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
                pattern_constraint: None,
            }),
            "Iso8601_duration" => CObject::CDuration(CDuration {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: Vec::new(),
                pattern_constraint: None,
            }),
            "Terminology_code" => CObject::CTerminologyCode(CTerminologyCode {
                parent: None,
                soc_parent: None,
                rm_type_name: rm_type.to_owned(),
                occurrences: None,
                node_id: nid,
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint: String::new(),
                constraint_status: None,
            }),
            _ => return None,
        };
        Some(obj)
    }
}

/// A relational operator prefixing an interval endpoint.
#[derive(Debug, Clone, Copy)]
enum Relop {
    Gt,
    Ge,
    Lt,
    Le,
}

/// The primitive kind an interval resolves to.
#[derive(Debug, Clone, Copy)]
enum PrimKind {
    Integer,
    Real,
    Date,
    Time,
    DateTime,
    Duration,
}

// ── value-kind trait: parse one endpoint value of type `V` ────────────────
/// A primitive value type that can be parsed from the token stream as an
/// interval endpoint / list element.
trait CadlValue: Clone + Sized {
    /// Parse a single value of this type from the current position.
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self>;
    /// Combine a centre and half-width for the `±` interval form; `None` if not
    /// reducible without RM type context (non-numeric types).
    fn plus_minus(centre: &Self, delta: &Self) -> Option<(Self, Self)>;
}

impl CadlValue for i32 {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        p.parse_signed_int(SyntaxErrorCode::Sciav)
    }
    fn plus_minus(centre: &Self, delta: &Self) -> Option<(Self, Self)> {
        Some((centre - delta, centre + delta))
    }
}

impl CadlValue for f64 {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        p.parse_signed_real(SyntaxErrorCode::Scrav)
    }
    fn plus_minus(centre: &Self, delta: &Self) -> Option<(Self, Self)> {
        Some((centre - delta, centre + delta))
    }
}

impl CadlValue for Iso8601Date {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        match p.peek().cloned() {
            Some(Token::Iso8601Date(v)) => {
                p.pos += 1;
                Ok(Iso8601Date { value: v })
            }
            _ => p.err(SyntaxErrorCode::Scdav, "expecting an ISO8601 date value"),
        }
    }
    fn plus_minus(_: &Self, _: &Self) -> Option<(Self, Self)> {
        None
    }
}

impl CadlValue for Iso8601Time {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        match p.peek().cloned() {
            Some(Token::Iso8601Time(v)) => {
                p.pos += 1;
                Ok(Iso8601Time { value: v })
            }
            _ => p.err(SyntaxErrorCode::Sctav, "expecting an ISO8601 time value"),
        }
    }
    fn plus_minus(_: &Self, _: &Self) -> Option<(Self, Self)> {
        None
    }
}

impl CadlValue for Iso8601DateTime {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        match p.peek().cloned() {
            Some(Token::Iso8601DateTime(v)) => {
                p.pos += 1;
                Ok(Iso8601DateTime { value: v })
            }
            _ => p.err(
                SyntaxErrorCode::Scdtav,
                "expecting an ISO8601 date/time value",
            ),
        }
    }
    fn plus_minus(_: &Self, _: &Self) -> Option<(Self, Self)> {
        None
    }
}

impl CadlValue for Iso8601Duration {
    fn parse_one(p: &mut Parser<'_>) -> PResult<Self> {
        match p.peek().cloned() {
            Some(Token::Iso8601Duration(v)) => {
                p.pos += 1;
                Ok(Iso8601Duration { value: v })
            }
            _ => p.err(
                SyntaxErrorCode::Scduav,
                "expecting an ISO8601 duration value",
            ),
        }
    }
    fn plus_minus(_: &Self, _: &Self) -> Option<(Self, Self)> {
        None
    }
}

// ── free helpers ──────────────────────────────────────────────────────────

/// Build a [`MultiplicityInterval`].
fn mult(
    lower: Option<i32>,
    upper: Option<i32>,
    lower_unbounded: bool,
    upper_unbounded: bool,
) -> MultiplicityInterval {
    MultiplicityInterval {
        lower,
        upper,
        lower_unbounded,
        upper_unbounded,
        lower_included: !lower_unbounded,
        upper_included: !upper_unbounded,
    }
}

/// A closed point interval `{v}`.
fn point_interval<T: Clone>(v: T) -> Interval<T> {
    Interval::PointInterval(PointInterval {
        lower: Some(v.clone()),
        upper: Some(v),
        lower_unbounded: false,
        upper_unbounded: false,
        lower_included: true,
        upper_included: true,
    })
}

/// A proper interval with explicit bounds/inclusivity.
// The four flags mirror `ProperIntervalData`'s own boolean fields 1:1.
#[allow(clippy::fn_params_excessive_bools)]
fn proper_interval<T>(
    lower: Option<T>,
    upper: Option<T>,
    lower_included: bool,
    upper_included: bool,
    lower_unbounded: bool,
    upper_unbounded: bool,
) -> Interval<T> {
    Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
        lower,
        upper,
        lower_unbounded,
        upper_unbounded,
        lower_included,
        upper_included,
    }))
}

/// Mutable references to the four `C_OBJECT` common fields (`rm_type_name`,
/// `node_id`, `occurrences`, `sibling_order`) across every [`CObject`] variant.
#[allow(clippy::type_complexity)]
fn common_mut(
    o: &mut CObject,
) -> (
    &mut String,
    &mut String,
    &mut Option<MultiplicityInterval>,
    &mut Option<SiblingOrder>,
) {
    match o {
        CObject::CComplexObject(CComplexObject::CComplexObject(d)) => (
            &mut d.rm_type_name,
            &mut d.node_id,
            &mut d.occurrences,
            &mut d.sibling_order,
        ),
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(b)) => (
            &mut b.rm_type_name,
            &mut b.node_id,
            &mut b.occurrences,
            &mut b.sibling_order,
        ),
        CObject::ArchetypeSlot(s) => (
            &mut s.rm_type_name,
            &mut s.node_id,
            &mut s.occurrences,
            &mut s.sibling_order,
        ),
        CObject::CComplexObjectProxy(p) => (
            &mut p.rm_type_name,
            &mut p.node_id,
            &mut p.occurrences,
            &mut p.sibling_order,
        ),
        CObject::CBoolean(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CDate(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CDateTime(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CDuration(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CInteger(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CReal(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CString(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CTerminologyCode(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
        CObject::CTime(c) => (
            &mut c.rm_type_name,
            &mut c.node_id,
            &mut c.occurrences,
            &mut c.sibling_order,
        ),
    }
}

/// Build a [`CComplexObjectData`] wrapped as a [`CObject`].
fn complex_object(
    rm_type_name: String,
    node_id: String,
    attributes: Vec<CAttribute>,
    attribute_tuples: Vec<CAttributeTuple>,
    default_value: Option<serde_json::Value>,
) -> CObject {
    CObject::CComplexObject(CComplexObject::CComplexObject(CComplexObjectData {
        parent: None,
        soc_parent: None,
        rm_type_name,
        occurrences: None,
        node_id,
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value,
        attributes,
        attribute_tuples,
    }))
}

/// A tuple member `C_ATTRIBUTE` (name only; the values live in the tuples).
fn tuple_member(rm_attribute_name: String) -> CAttribute {
    CAttribute {
        parent: None,
        soc_parent: None,
        rm_attribute_name,
        existence: None,
        children: Vec::new(),
        differential_path: None,
        cardinality: None,
        is_multiple: false,
    }
}

/// A `C_STRING` carrying a single regex constraint (`/re/`, delimiters kept).
fn cstring_regex(regex: String, assumed: Option<String>) -> CString {
    CString {
        parent: None,
        soc_parent: None,
        rm_type_name: "String".to_owned(),
        occurrences: None,
        node_id: "Primitive_node_id".to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        assumed_value: assumed,
        is_enumerated_type_constraint: None,
        constraint: vec![regex],
    }
}

/// A local (archetype-internal) at-code terminology value.
fn local_term_code(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "local".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// Convert a parsed inline primitive [`CObject`] to a [`CPrimitiveObject`].
fn cobject_to_primitive(o: CObject) -> Option<CPrimitiveObject> {
    Some(match o {
        CObject::CBoolean(c) => CPrimitiveObject::CBoolean(c),
        CObject::CDate(c) => CPrimitiveObject::CDate(c),
        CObject::CDateTime(c) => CPrimitiveObject::CDateTime(c),
        CObject::CDuration(c) => CPrimitiveObject::CDuration(c),
        CObject::CInteger(c) => CPrimitiveObject::CInteger(c),
        CObject::CReal(c) => CPrimitiveObject::CReal(c),
        CObject::CString(c) => CPrimitiveObject::CString(c),
        CObject::CTerminologyCode(c) => CPrimitiveObject::CTerminologyCode(c),
        CObject::CTime(c) => CPrimitiveObject::CTime(c),
        _ => return None,
    })
}

/// Parse a `matches { … }` primitive-constraint right-hand side (the verbatim
/// `{ c_primitive_object }` text, braces included) into a [`CPrimitiveObject`].
///
/// This is the reusable entry the rules / slot-assertion BEL builder
/// ([`crate::rules`]) drives to construct an `EXPR_CONSTRAINT` leaf
/// (`AOM2` master05; primitive grammar `master04.5`).
///
/// # Errors
/// Returns the cADL `S*` catalogue errors if the constraint is malformed or its
/// body is not a single primitive constraint.
pub(crate) fn parse_inline_primitive_text(raw: &str) -> Result<CPrimitiveObject, Vec<SyntaxError>> {
    let toks = crate::lexer::lex(raw).map_err(|e| vec![e])?;
    let mut parser = Parser {
        src: raw,
        toks: &toks,
        pos: 0,
        errors: Vec::new(),
    };
    let parsed: PResult<CObject> = (|| {
        parser.expect(
            |t| matches!(t, Token::LCurly),
            SyntaxErrorCode::Sccog,
            "expecting '{' opening a constraint",
        )?;
        let obj = parser.parse_c_inline_primitive("Primitive_node_id".to_owned())?;
        parser.expect(
            |t| matches!(t, Token::RCurly),
            SyntaxErrorCode::Sccog,
            "expecting '}' closing a constraint",
        )?;
        Ok(obj)
    })();
    match parsed {
        Ok(obj) if parser.errors.is_empty() => cobject_to_primitive(obj).ok_or_else(|| {
            vec![SyntaxError::at(
                SyntaxErrorCode::Sccog,
                "constraint body is not a primitive constraint",
                0..raw.len(),
                raw,
            )]
        }),
        _ => {
            if parser.errors.is_empty() {
                parser.errors.push(SyntaxError::at(
                    SyntaxErrorCode::Sccog,
                    "invalid primitive constraint",
                    0..raw.len(),
                    raw,
                ));
            }
            Err(parser.errors)
        }
    }
}

/// Parse a contained-regexp right-hand side (`{ /re/ }` / `{ ^re^ }`, verbatim)
/// into a [`CString`] — the regex matcher for an archetype-id slot assertion
/// (`EXPR_ARCHETYPE_ID_CONSTRAINT`, `AOM2` master05; `master04.3` §Archetype
/// Slots). The regex is compile-checked (`SCSRE`).
///
/// # Errors
/// [`SyntaxErrorCode::Scsre`] if the regex does not compile.
pub(crate) fn parse_contained_regexp_text(raw: &str) -> Result<CString, Vec<SyntaxError>> {
    let body = raw
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    let (regex_part, assumed) = match body.split_once(';') {
        Some((r, a)) => {
            let assumed = a
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .map(decode_string_inner);
            (r.trim(), assumed)
        }
        None => (body, None),
    };
    let inner = regex_inner(regex_part);
    if regex::Regex::new(inner).is_err() {
        return Err(vec![SyntaxError::at(
            SyntaxErrorCode::Scsre,
            format!("{inner:?} is not a valid regular expression"),
            0..raw.len(),
            raw,
        )]);
    }
    Ok(cstring_regex(regex_part.to_owned(), assumed))
}

/// Split a differential ADL path into `(parent_path, last_attribute_name)`.
fn split_diff_path(p: &str) -> (String, String) {
    match p.rfind('/') {
        Some(idx) => {
            let parent = &p[..idx];
            let name = &p[idx + 1..];
            (parent.to_owned(), name.to_owned())
        }
        None => (String::new(), p.to_owned()),
    }
}

/// The inner regex of a `/re/` or `^re^` delimited pattern.
fn regex_inner(delimited: &str) -> &str {
    let d = delimited.trim();
    if (d.starts_with('/') && d.ends_with('/') && d.len() >= 2)
        || (d.starts_with('^') && d.ends_with('^') && d.len() >= 2)
    {
        &d[1..d.len() - 1]
    } else {
        d
    }
}

/// Whether a date/time pattern field is the "present" placeholder (e.g. `hh`).
fn is_present_field(f: &str, present: &str) -> bool {
    f.eq_ignore_ascii_case(present)
}

/// The strength-keyword set (`master04.5` §Constraint strengths).
fn is_strength_keyword(s: &str) -> bool {
    matches!(s, "required" | "extensible" | "preferred" | "example")
}

/// Map a constraint-strength keyword to its ordinal `CONSTRAINT_STATUS`
/// (`master09.05`: required(0) < extensible(1) < preferred(2) < example(3)).
fn strength_status(s: &str) -> ConstraintStatus {
    // `required` (and any unrecognised keyword) maps to the default status 0.
    ConstraintStatus(match s {
        "extensible" => 1,
        "preferred" => 2,
        "example" => 3,
        _ => 0,
    })
}

/// Decode a double-quoted `master03` string literal (delimiters included).
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    decode_string_inner(inner)
}

/// Decode `master03` escape sequences in the (undelimited) string body.
fn decode_string_inner(inner: &str) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') | None => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => {
                let hex: String = chars.clone().take(4).collect();
                if hex.len() == 4
                    && let Ok(cp) = u32::from_str_radix(&hex, 16)
                    && let Some(ch) = char::from_u32(cp)
                {
                    out.push(ch);
                    for _ in 0..4 {
                        chars.next();
                    }
                } else {
                    out.push('\\');
                    out.push('u');
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

/// Convert an [`openehr_lang::odin::OdinValue`] to canonical JSON for a
/// `C_DEFINED_OBJECT.default_value`.
///
/// NOTE: interval ODIN values are represented as `null` here (rare in
/// definition-section default values; a fuller typed encoding lands with the
/// template/OPT phase — no openEHR spec mandates the intermediate JSON shape,
/// our own design).
fn odin_to_json(v: &openehr_lang::odin::OdinValue) -> serde_json::Value {
    use openehr_lang::odin::OdinValue;
    match v {
        OdinValue::String(s)
        | OdinValue::Date(s)
        | OdinValue::Time(s)
        | OdinValue::DateTime(s)
        | OdinValue::Duration(s)
        | OdinValue::TermCode(s)
        | OdinValue::Uri(s)
        | OdinValue::Path(s) => serde_json::Value::String(s.clone()),
        OdinValue::Integer(i) => serde_json::Value::from(*i),
        OdinValue::Real(r) => serde_json::Value::from(*r),
        OdinValue::Boolean(b) => serde_json::Value::from(*b),
        OdinValue::Character(c) => serde_json::Value::String(c.to_string()),
        OdinValue::Empty | OdinValue::Interval(_) => serde_json::Value::Null,
        OdinValue::ListContinue => serde_json::Value::String("...".to_owned()),
        OdinValue::List(items) => {
            serde_json::Value::Array(items.iter().map(odin_to_json).collect())
        }
        OdinValue::PathList(ps) => serde_json::Value::Array(
            ps.iter()
                .map(|p| serde_json::Value::String(p.clone()))
                .collect(),
        ),
        OdinValue::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), odin_to_json(v)))
                .collect(),
        ),
        OdinValue::KeyedList(items) => serde_json::Value::Object(
            items
                .iter()
                .map(|(k, v)| (odin_key_str(k), odin_to_json(v)))
                .collect(),
        ),
        OdinValue::Typed { rm_type, value } => {
            let mut inner = odin_to_json(value);
            if let serde_json::Value::Object(m) = &mut inner {
                m.insert(
                    "_type".to_owned(),
                    serde_json::Value::String(rm_type.clone()),
                );
            }
            inner
        }
    }
}

fn odin_key_str(k: &openehr_lang::odin::OdinKey) -> String {
    use openehr_lang::odin::OdinKey;
    match k {
        OdinKey::String(s) | OdinKey::Date(s) | OdinKey::Time(s) | OdinKey::DateTime(s) => {
            s.clone()
        }
        OdinKey::Integer(i) => i.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::too_many_lines)] // test assertions panic by design
mod tests {
    use super::*;

    fn parse(body: &str) -> CComplexObject {
        parse_definition_body(body).unwrap_or_else(|e| panic!("parse failed: {e:?}"))
    }

    fn data(cco: &CComplexObject) -> &CComplexObjectData {
        match cco {
            CComplexObject::CComplexObject(d) => d,
            CComplexObject::CArchetypeRoot(_) => panic!("expected plain complex object"),
        }
    }

    #[test]
    fn minimal_object_with_node_id() {
        let cco = parse("OBSERVATION[id1]");
        let d = data(&cco);
        assert_eq!(d.rm_type_name, "OBSERVATION");
        assert_eq!(d.node_id, "id1");
        assert!(d.attributes.is_empty());
    }

    #[test]
    fn at_coded_root() {
        let cco = parse("OBSERVATION[at0000] matches {\n value matches { DV_TEXT[at0001] }\n}");
        let d = data(&cco);
        assert_eq!(d.node_id, "at0000");
        assert_eq!(d.attributes.len(), 1);
        assert_eq!(d.attributes[0].rm_attribute_name, "value");
    }

    #[test]
    fn occurrences_and_cardinality() {
        let cco = parse(
            "OBSERVATION[id1] matches {\n\
             items cardinality matches {1..*; unordered} matches {\n\
             ELEMENT[id2] occurrences matches {0..1} matches { value matches { DV_TEXT[id3] } }\n\
             }\n}",
        );
        let d = data(&cco);
        let items = &d.attributes[0];
        assert_eq!(items.rm_attribute_name, "items");
        let card = items.cardinality.as_ref().expect("cardinality");
        assert_eq!(card.interval.lower, Some(1));
        assert!(card.interval.upper_unbounded);
        assert!(!card.is_ordered);
        assert!(items.is_multiple);
        let elem = &items.children[0];
        let (_, node_id, occ, _) = obj_common(elem);
        assert_eq!(node_id, "id2");
        let occ = occ.as_ref().expect("occurrences");
        assert_eq!(occ.lower, Some(0));
        assert_eq!(occ.upper, Some(1));
    }

    fn obj_common(
        obj: &CObject,
    ) -> (
        String,
        String,
        Option<MultiplicityInterval>,
        Option<SiblingOrder>,
    ) {
        // Read-only mirror of `common_mut` for tests.
        let mut obj = obj.clone();
        let (ty, nid, occ, sib) = common_mut(&mut obj);
        (ty.clone(), nid.clone(), occ.clone(), sib.clone())
    }

    #[test]
    fn integer_forms() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {55}\n\
             b matches {10, 20, 30}\n\
             c matches {|0..100|}\n\
             d matches {|>0..<100|}\n\
             e matches {|>=10|}\n\
             f matches {|-10..-5|}\n\
             }",
        );
        let d = data(&cco);
        // a: point 55
        match &d.attributes[0].children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::PointInterval(p) => assert_eq!(p.lower, Some(55)),
                Interval::ProperInterval(_) => panic!("expected point"),
            },
            _ => panic!("expected CInteger"),
        }
        // b: three points
        match &d.attributes[1].children[0] {
            CObject::CInteger(ci) => assert_eq!(ci.constraint.len(), 3),
            _ => panic!("expected CInteger"),
        }
        // c: |0..100|
        match &d.attributes[2].children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert_eq!(pi.lower, Some(0));
                    assert_eq!(pi.upper, Some(100));
                    assert!(pi.lower_included && pi.upper_included);
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
        // d: |>0..<100| exclusive both
        match &d.attributes[3].children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert!(!pi.lower_included && !pi.upper_included);
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
        // e: |>=10| lower bounded, upper unbounded
        match &d.attributes[4].children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert_eq!(pi.lower, Some(10));
                    assert!(pi.lower_included);
                    assert!(pi.upper_unbounded);
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
        // f: |-10..-5| negative endpoints
        match &d.attributes[5].children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert_eq!(pi.lower, Some(-10));
                    assert_eq!(pi.upper, Some(-5));
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
    }

    #[test]
    fn string_boolean_and_regex() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             s1 matches {\"something\"}\n\
             s2 matches {/cardio.*/}\n\
             s3 matches {\"a\", \"b\"}\n\
             b1 matches {True}\n\
             b2 matches {True, False}\n\
             }",
        );
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint, vec!["something".to_owned()]),
            _ => panic!("expected CString"),
        }
        match &d.attributes[1].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint, vec!["/cardio.*/".to_owned()]),
            _ => panic!("expected CString regex"),
        }
        match &d.attributes[2].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint.len(), 2),
            _ => panic!("expected CString list"),
        }
        match &d.attributes[3].children[0] {
            CObject::CBoolean(cb) => assert_eq!(cb.constraint, vec![true]),
            _ => panic!("expected CBoolean"),
        }
        match &d.attributes[4].children[0] {
            CObject::CBoolean(cb) => assert_eq!(cb.constraint, vec![true, false]),
            _ => panic!("expected CBoolean list"),
        }
    }

    #[test]
    fn date_time_patterns_and_durations() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             d1 matches {yyyy-mm-??}\n\
             d2 matches {|2000-01-01..2000-02-01|}\n\
             t1 matches {hh:mm:ss}\n\
             dur1 matches {PWD}\n\
             dur2 matches {PWD/|P38W..P39W4D|}\n\
             dur3 matches {|<=PT1H|}\n\
             }",
        );
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CDate(c) => assert_eq!(c.pattern_constraint.as_deref(), Some("yyyy-mm-??")),
            _ => panic!("expected CDate pattern"),
        }
        match &d.attributes[1].children[0] {
            CObject::CDate(c) => assert_eq!(c.constraint.len(), 1),
            _ => panic!("expected CDate interval"),
        }
        match &d.attributes[2].children[0] {
            CObject::CTime(c) => assert_eq!(c.pattern_constraint.as_deref(), Some("hh:mm:ss")),
            _ => panic!("expected CTime pattern"),
        }
        match &d.attributes[3].children[0] {
            CObject::CDuration(c) => {
                assert_eq!(c.pattern_constraint.as_deref(), Some("PWD"));
                assert!(c.constraint.is_empty());
            }
            _ => panic!("expected CDuration pattern"),
        }
        match &d.attributes[4].children[0] {
            CObject::CDuration(c) => {
                assert_eq!(c.pattern_constraint.as_deref(), Some("PWD"));
                assert_eq!(c.constraint.len(), 1);
            }
            _ => panic!("expected CDuration pattern+interval"),
        }
        match &d.attributes[5].children[0] {
            CObject::CDuration(c) => assert_eq!(c.constraint.len(), 1),
            _ => panic!("expected CDuration interval"),
        }
    }

    #[test]
    fn terminology_codes() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {[ac1]}\n\
             b matches {[at0004]}\n\
             c matches {[ac2; at0022]}\n\
             d matches {preferred [at0004]}\n\
             e matches {[ac1@snomed_ct]}\n\
             }",
        );
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CTerminologyCode(t) => assert_eq!(t.constraint, "ac1"),
            _ => panic!("expected CTerminologyCode"),
        }
        match &d.attributes[2].children[0] {
            CObject::CTerminologyCode(t) => {
                assert_eq!(t.constraint, "ac2");
                assert_eq!(
                    t.assumed_value.as_ref().map(|a| a.code_string.as_str()),
                    Some("at0022")
                );
            }
            _ => panic!("expected CTerminologyCode with assumed"),
        }
        match &d.attributes[3].children[0] {
            CObject::CTerminologyCode(t) => {
                assert_eq!(t.constraint_status, Some(ConstraintStatus(2)));
            }
            _ => panic!("expected CTerminologyCode with strength"),
        }
        match &d.attributes[4].children[0] {
            CObject::CTerminologyCode(t) => assert_eq!(t.constraint, "ac1@snomed_ct"),
            _ => panic!("expected CTerminologyCode with binding"),
        }
    }

    #[test]
    fn attribute_and_ordinal_tuples() {
        let cco = parse(
            "OBSERVATION[id1] matches {\n\
             value matches {\n\
             DV_QUANTITY[id2] matches {\n\
             [magnitude, units] matches {\n\
             [{|>=0.0|}, {\"mmol/l\"}],\n\
             [{0.0}, {\"mg/dl\"}]\n\
             }\n}\n}\n\
             ord matches {\n\
             DV_ORDINAL[id3] matches {\n\
             [value, symbol] matches {\n\
             [{0}, {[at11]}],\n\
             [{1}, {[at12]}]\n\
             }\n}\n}\n}",
        );
        let d = data(&cco);
        // DV_QUANTITY under value
        let q = &d.attributes[0].children[0];
        match q {
            CObject::CComplexObject(CComplexObject::CComplexObject(qd)) => {
                assert_eq!(qd.attribute_tuples.len(), 1);
                let t = &qd.attribute_tuples[0];
                assert_eq!(t.members.len(), 2);
                assert_eq!(t.members[0].rm_attribute_name, "magnitude");
                assert_eq!(t.tuples.len(), 2);
                assert_eq!(t.tuples[0].members.len(), 2);
                assert!(matches!(t.tuples[0].members[0], CPrimitiveObject::CReal(_)));
                assert!(matches!(
                    t.tuples[0].members[1],
                    CPrimitiveObject::CString(_)
                ));
            }
            _ => panic!("expected DV_QUANTITY complex object"),
        }
        // DV_ORDINAL tuple with terminology members
        let o = &d.attributes[1].children[0];
        match o {
            CObject::CComplexObject(CComplexObject::CComplexObject(od)) => {
                let t = &od.attribute_tuples[0];
                assert!(matches!(
                    t.tuples[0].members[0],
                    CPrimitiveObject::CInteger(_)
                ));
                assert!(matches!(
                    t.tuples[0].members[1],
                    CPrimitiveObject::CTerminologyCode(_)
                ));
            }
            _ => panic!("expected DV_ORDINAL complex object"),
        }
    }

    #[test]
    fn slot_use_node_use_archetype_and_sibling() {
        let cco = parse(
            "SECTION[id1] matches {\n\
             items cardinality matches {0..*} matches {\n\
             allow_archetype OBSERVATION[id2] occurrences matches {0..1} matches {\n\
             include\n\
             archetype_id/value matches {/openEHR-EHR-OBSERVATION\\.foo.*\\.v1/}\n\
             exclude\n\
             archetype_id/value matches {/.*/}\n\
             }\n\
             use_archetype CLUSTER[id3, openEHR-EHR-CLUSTER.device.v1]\n\
             after[id3] use_node ELEMENT[id4] /items[id5]/value\n\
             allow_archetype SECTION[id6] closed\n\
             }\n}",
        );
        let d = data(&cco);
        let children = &d.attributes[0].children;
        // slot
        match &children[0] {
            CObject::ArchetypeSlot(s) => {
                assert_eq!(s.node_id, "id2");
                assert_eq!(s.includes.len(), 1);
                assert!(
                    s.includes[0]
                        .string_expression
                        .as_ref()
                        .unwrap()
                        .contains("archetype_id")
                );
                assert_eq!(s.excludes.len(), 1);
                assert!(!s.is_closed);
            }
            _ => panic!("expected ArchetypeSlot"),
        }
        // use_archetype -> C_ARCHETYPE_ROOT
        match &children[1] {
            CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => {
                assert_eq!(r.node_id, "id3");
                assert_eq!(r.archetype_ref, "openEHR-EHR-CLUSTER.device.v1");
            }
            _ => panic!("expected CArchetypeRoot"),
        }
        // use_node -> proxy, with a sibling order
        match &children[2] {
            CObject::CComplexObjectProxy(p) => {
                assert_eq!(p.target_path, "/items[id5]/value");
                let so = p.sibling_order.as_ref().expect("sibling order");
                assert!(!so.is_before);
                assert_eq!(so.sibling_node_id, "id3");
            }
            _ => panic!("expected CComplexObjectProxy"),
        }
        // closed slot (id-coded, no matches)
        match &children[3] {
            CObject::ArchetypeSlot(s) => assert!(s.is_closed),
            _ => panic!("expected closed ArchetypeSlot"),
        }
    }

    #[test]
    fn regular_primitive_type_object() {
        let cco = parse("WHOLE[id1] matches {\n a matches {\n String [id2]\n }\n}");
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CString(cs) => {
                assert_eq!(cs.node_id, "id2");
                assert_eq!(cs.rm_type_name, "String");
                assert!(cs.constraint.is_empty());
            }
            _ => panic!("expected regular primitive CString"),
        }
    }

    #[test]
    fn differential_path_attribute() {
        let cco = parse(
            "OBSERVATION[id1.1] matches {\n\
             /data[id2]/items[id4.1]/value matches { DV_TEXT[id5] }\n\
             }",
        );
        let d = data(&cco);
        let a = &d.attributes[0];
        assert_eq!(a.rm_attribute_name, "value");
        assert_eq!(
            a.differential_path.as_deref(),
            Some("/data[id2]/items[id4.1]")
        );
    }

    #[test]
    fn existence_and_scas_errors() {
        // Existence {5} is invalid -> SEXLSG.
        let errs = parse_definition_body(
            "WHOLE[id1] matches {\n a existence matches {5} matches { DV_TEXT[id2] }\n}",
        )
        .expect_err("should fail");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Sexlsg),
            "{errs:?}"
        );

        // Empty attribute body -> SCAS.
        let errs = parse_definition_body("ENTRY[id1] matches {\n value matches {}\n}")
            .expect_err("should fail");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Scas),
            "{errs:?}"
        );

        // Empty object body -> SCOAT.
        let errs = parse_definition_body(
            "ENTRY[id1] matches {\n value matches { ELEMENT[id2] matches {} }\n}",
        )
        .expect_err("should fail");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Scoat),
            "{errs:?}"
        );
    }

    #[test]
    fn assumed_values() {
        let cco =
            parse("WHOLE[id1] matches {\n a matches {|0..10|; 5}\n s matches {\"x\"; \"y\"}\n}");
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CInteger(ci) => assert_eq!(ci.assumed_value, Some(5.0)),
            _ => panic!("expected CInteger with assumed"),
        }
        match &d.attributes[1].children[0] {
            CObject::CString(cs) => assert_eq!(cs.assumed_value.as_deref(), Some("y")),
            _ => panic!("expected CString with assumed"),
        }
    }

    // ── structural spot-checks against real vendored corpus files ──────────
    // These parse whole `.adls` sources (via `parse_definition`) and assert the
    // deep AOM2 tree against expectations hand-derived by reading each file.

    /// Fetch the named attribute of a complex object.
    fn attr<'a>(d: &'a CComplexObjectData, name: &str) -> &'a CAttribute {
        d.attributes
            .iter()
            .find(|a| a.rm_attribute_name == name)
            .unwrap_or_else(|| panic!("no attribute {name:?}"))
    }

    /// The first child of an attribute, as a plain complex object.
    fn first_cco(a: &CAttribute) -> &CComplexObjectData {
        match &a.children[0] {
            CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d,
            other => panic!("expected complex object child, got {other:?}"),
        }
    }

    #[test]
    fn corpus_ordinal_tuple_structure() {
        let src = include_str!(
            "../tests/corpus/adl2-reference/features/aom_structures/tuples/openEHR-EHR-OBSERVATION.ordinal_tuple.v1.0.0.adls"
        );
        let cco = parse_definition(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let root = data(&cco);
        assert_eq!(root.rm_type_name, "OBSERVATION");
        assert_eq!(root.node_id, "id1");

        // OBSERVATION/data/HISTORY[id3]/events/POINT_EVENT[id4]/data/
        // ITEM_LIST[id2]/items/ELEMENT[id10]/value/DV_ORDINAL[id11].
        let history = first_cco(attr(root, "data"));
        assert_eq!(history.node_id, "id3");
        let point_event = first_cco(attr(history, "events"));
        assert_eq!(point_event.node_id, "id4");
        let pe_occ = point_event
            .occurrences
            .as_ref()
            .expect("POINT_EVENT occurrences");
        assert_eq!(pe_occ.lower, Some(0));
        assert_eq!(pe_occ.upper, Some(1));
        let item_list = first_cco(attr(point_event, "data"));
        assert_eq!(item_list.node_id, "id2");
        // `events` is an ordered/unordered container with a cardinality.
        let events = attr(history, "events");
        assert!(events.cardinality.is_some());
        let element = first_cco(attr(item_list, "items"));
        assert_eq!(element.node_id, "id10");
        // `items` cardinality {1..6; ordered}.
        let items = attr(item_list, "items");
        let card = items.cardinality.as_ref().expect("items cardinality");
        assert_eq!(card.interval.lower, Some(1));
        assert_eq!(card.interval.upper, Some(6));
        assert!(card.is_ordered);
        let ordinal = first_cco(attr(element, "value"));
        assert_eq!(ordinal.rm_type_name, "DV_ORDINAL");
        assert_eq!(ordinal.node_id, "id11");

        // The `[value, symbol]` tuple with three ordinal rows.
        assert_eq!(ordinal.attribute_tuples.len(), 1);
        let tuple = &ordinal.attribute_tuples[0];
        assert_eq!(tuple.members.len(), 2);
        assert_eq!(tuple.members[0].rm_attribute_name, "value");
        assert_eq!(tuple.members[1].rm_attribute_name, "symbol");
        assert_eq!(tuple.tuples.len(), 3);
        match &tuple.tuples[0].members[0] {
            CPrimitiveObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::PointInterval(p) => assert_eq!(p.lower, Some(0)),
                Interval::ProperInterval(_) => panic!("expected point 0"),
            },
            other => panic!("expected CInteger, got {other:?}"),
        }
        match &tuple.tuples[0].members[1] {
            CPrimitiveObject::CTerminologyCode(t) => assert_eq!(t.constraint, "at11"),
            other => panic!("expected CTerminologyCode, got {other:?}"),
        }
    }

    #[test]
    fn corpus_slot_structure() {
        let src = include_str!(
            "../tests/corpus/adl2-reference/validity/slots/openEHR-EHR-SECTION.slot_parent.v1.0.0.adls"
        );
        let cco = parse_definition(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let root = data(&cco);
        assert_eq!(root.rm_type_name, "SECTION");
        assert_eq!(root.node_id, "id1");

        // SECTION/items cardinality {1..*; unordered} matches { allow_archetype
        // OBSERVATION[id2] occurrences {0..1} matches { include… exclude… } }.
        let items = attr(root, "items");
        assert!(items.is_multiple);
        let card = items.cardinality.as_ref().expect("items cardinality");
        assert_eq!(card.interval.lower, Some(1));
        assert!(card.interval.upper_unbounded);
        assert!(!card.is_ordered);

        match &items.children[0] {
            CObject::ArchetypeSlot(s) => {
                assert_eq!(s.rm_type_name, "OBSERVATION");
                assert_eq!(s.node_id, "id2");
                let occ = s.occurrences.as_ref().expect("slot occurrences");
                assert_eq!(occ.lower, Some(0));
                assert_eq!(occ.upper, Some(1));
                assert_eq!(s.includes.len(), 1);
                assert!(
                    s.includes[0]
                        .string_expression
                        .as_deref()
                        .unwrap_or_default()
                        .contains("archetype_id/value")
                );
                assert_eq!(s.excludes.len(), 1);
                assert!(!s.is_closed);
            }
            other => panic!("expected ArchetypeSlot, got {other:?}"),
        }
    }

    #[test]
    fn corpus_primitive_types_structure() {
        let src = include_str!(
            "../tests/corpus/adl2-reference/features/aom_structures/primitive_types/openehr-TEST_PKG-WHOLE.primitive_types.v1.0.0.adls"
        );
        let cco = parse_definition(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let root = data(&cco);
        assert_eq!(root.node_id, "id1");
        // integer_attr3 == {|0..100|}.
        match &attr(root, "integer_attr3").children[0] {
            CObject::CInteger(ci) => match &ci.constraint[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert_eq!(pi.lower, Some(0));
                    assert_eq!(pi.upper, Some(100));
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
        // date_attr3 == {yyyy-mm-??} (a pattern).
        match &attr(root, "date_attr3").children[0] {
            CObject::CDate(c) => assert_eq!(c.pattern_constraint.as_deref(), Some("yyyy-mm-??")),
            _ => panic!("expected CDate pattern"),
        }
        // duration_attr22 == {PWD/PT0S} (pattern + value).
        match &attr(root, "duration_attr22").children[0] {
            CObject::CDuration(c) => {
                assert_eq!(c.pattern_constraint.as_deref(), Some("PWD"));
                assert_eq!(c.constraint.len(), 1);
            }
            _ => panic!("expected CDuration pattern+value"),
        }
    }
}
