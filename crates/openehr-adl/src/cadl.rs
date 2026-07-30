//! The cADL definition-section parser.
//!
//! A hand-written recursive-descent parser over the lexer's token stream
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
//! Scope: full cADL object/attribute/tuple/slot/proxy/primitive
//! coverage building the AOM2 tree, with the `S*` syntax-validity codes raised
//! at position. Slot include/exclude **assertion** expressions and the `rules`
//! section are captured as raw text (structured BEL expression parsing is a
//! TODO),
//! preserved in `ASSERTION.string_expression` so slots stay usable; the common
//! `archetype_id/value matches {/regex/}` form is additionally regex-compile
//! checked (`SCSRE`). Semantic (V-code) validation is separate
//! (`crate::validate`).

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
use openehr_base::base_types::definitions::definitions_impl::LOCAL_TERMINOLOGY_ID;
use openehr_base::prelude::{
    Cardinality, Interval, Iso8601Date, Iso8601DateTime, Iso8601Duration, Iso8601Time,
    MultiplicityInterval, PointInterval, ProperInterval, ProperIntervalData, TerminologyCode,
};
use openehr_lang::odin::OdinValue;

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
    parse_definition_body_with(body, Dialect::Adl2)
}

/// Parse the **root artefact's** `definition` section of a whole ADL2 source
/// into a [`CComplexObject`].
///
/// Design note: [`crate::source::SourceArtefact`] stores only spans (byte +
/// token ranges), not the token stream or owned text, so this convenience
/// re-lexes just the definition body substring (`src[definition.bytes]`) via
/// [`parse_definition_body`] and re-offsets error spans back to the whole
/// file. Bodies are small, so the extra lex is cheap, and the outer
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

/// Which ADL dialect the cADL parser accepts.
///
/// `Adl2` is the spec-conformant grammar (`cadl2.g4`). `Adl14` accepts the ADL
/// 1.4 cADL language: it ADDS the 1.4-only definition forms — qualified/listed
/// terminology constraints (`[local::at0001]`, `[local:: a, b, c ; assumed]`,
/// `[openehr::524]`) and the inline dADL domain constraints
/// (`C_DV_QUANTITY <…>`, `(C_DV_ORDINAL) <…>`) — and REMOVES the constructs ADL
/// 2 introduced, because a 1.4 source is judged as 1.4 rather than as a
/// permissive superset. The removed set (each refused with a typed
/// [`SyntaxError`] naming the construct, see `Parser::adl2_only`):
/// `use_archetype`, the slot `closed` marker, the `_default` pseudo-attribute,
/// second-order attribute tuples, term-constraint strengths, and `@terminology`
/// operational bindings. `before`/`after` sibling order stays accepted —
/// `ADL1.4/master05-cadl.adoc` §Keywords L53 lists both as cADL 1.4 keywords.
///
/// NOTE: no openEHR spec governs 1.4→2 conversion — the additive half exists
/// only to feed `crate::adl14` and is our own design (see the module flag on
/// `crate::adl14`). The subtractive half IS spec-grounded: the closed cADL 1.4
/// keyword set of `ADL1.4/master05-cadl.adoc` §Keywords (L48-53). `Adl2` parsing
/// is unchanged either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// The spec-conformant ADL2 grammar.
    Adl2,
    /// ADL2 plus the 1.4-only tolerance forms (converter front end only).
    Adl14,
}

/// Parse a **1.4-dialect** definition body into a [`CComplexObject`].
///
/// The 1.4-only terminology-constraint and inline-dADL domain forms are kept in
/// a converter-internal encoding (qualified codes and lists in the
/// `C_TERMINOLOGY_CODE.constraint` string; domain blocks lowered to a
/// `DV_QUANTITY`/`DV_ORDINAL` with a `property` at-code + a tuple/attribute set)
/// that `crate::adl14::convert` rewrites into spec-valid ADL2. See the
/// `crate::adl14` module flag: no openEHR spec governs this.
///
/// # Errors
/// Returns every [`SyntaxError`] the cADL parse raises (including
/// [`SyntaxErrorCode::Sdinv`] for a malformed inline dADL domain block).
pub fn parse_definition_body_adl14(body: &str) -> Result<CComplexObject, Vec<SyntaxError>> {
    parse_definition_body_with(body, Dialect::Adl14)
}

fn parse_definition_body_with(
    body: &str,
    dialect: Dialect,
) -> Result<CComplexObject, Vec<SyntaxError>> {
    let toks = match crate::lexer::lex(body) {
        Ok(t) => t,
        Err(e) => return Err(vec![e]),
    };
    let mut parser = Parser {
        src: body,
        toks: &toks,
        pos: 0,
        errors: Vec::new(),
        dialect,
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

/// The recursive-descent cADL parser over a token slice.
struct Parser<'a> {
    src: &'a str,
    toks: &'a [Spanned],
    pos: usize,
    errors: Vec<SyntaxError>,
    dialect: Dialect,
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

    /// Refuse an ADL 2-only cADL construct met while parsing in the
    /// [`Dialect::Adl14`] dialect.
    ///
    /// A 1.4 source is judged AS 1.4. The cADL 1.4 keyword set is CLOSED —
    /// `ADL1.4/master05-cadl.adoc` §Keywords (L48-53) lists exactly
    /// `matches`/`~matches`/`is_in`/`~is_in`, `occurrences`/`existence`/
    /// `cardinality`, `ordered`/`unordered`/`unique`, `infinity`,
    /// `use_node`/`allow_archetype`, `include`/`exclude`, `before`/`after` — so a
    /// construct that ADL 2 added is a syntax error in a 1.4 text, not a
    /// silently-accepted superset.
    ///
    /// NOTE: the `S*` code space is a verbatim 1:1 mirror of the openEHR
    /// catalogue (`ADL2/master04.6-cadl_validity_rules.adoc` §Syntax Validity
    /// Rules) and carries no "wrong ADL generation" code, so each gate reuses the
    /// catalogue code for its parse POSITION (`SCOAT` in an object body, `SCCOG`
    /// in object position, `STCCP` inside a term-code constraint) and names the
    /// construct plus the generation in the message. Inventing a code would break
    /// the 1:1 mirror [`SyntaxErrorCode`] documents.
    fn adl2_only<T>(&mut self, code: SyntaxErrorCode, construct: &str) -> PResult<T> {
        self.err(
            code,
            format!("{construct} is an ADL 2 construct and is not valid in ADL 1.4"),
        )
    }

    /// True if the cursor is at a NEGATED matches operator (`~matches`,
    /// `~is_in`, `∉`) — lexically `SymNot SymMatches` or the single `∉`.
    fn at_negated_matches(&self) -> bool {
        matches!(self.peek(), Some(Token::SymNotMatches))
            || (matches!(self.peek(), Some(Token::SymNot))
                && matches!(self.peek_at(1), Some(Token::SymMatches)))
    }

    /// True if the cursor is at a regex-match operator (`=~` or `!~`).
    ///
    /// Neither is a token in any normative lexical specification, so both
    /// arrive here as a token PAIR: `=~` as `SymEq SymNot` and `!~` as
    /// `SymNot SymNot` (`!` and `~` both fold into `SYM_NOT` —
    /// `adl_keywords.g4` `SYM_NOT : [Nn][Oo][Tt] | '!' | '∼' | '~' | '¬'`).
    fn at_regex_match_operator(&self) -> bool {
        matches!(self.peek(), Some(Token::SymEq | Token::SymNot))
            && matches!(self.peek_at(1), Some(Token::SymNot))
    }

    /// Refuse a negated matches operator with a message that says where the
    /// negation IS admitted.
    ///
    /// `ADL1.4/master05-cadl.adoc` §Keywords lists `~matches`/`~is_in` (L47)
    /// and glosses them at L91-98 ("Occasionally, the matches operator needs to
    /// be used in the negative, usually at a leaf block"), but NO production of
    /// any normative grammar admits them:
    /// * the chapter's own §Syntax has one `SYM_MATCHES` in `c_attribute`,
    ///   `c_complex_object` and `archetype_slot` (L1069-1120) and its §Symbols
    ///   lexer (L1300-1354) has no `~` symbol and no `∉` token at all;
    /// * the vendored ANTLR set is identical — `cadl14.g4` uses `SYM_MATCHES`
    ///   only, and `adl_keywords.g4` `SYM_MATCHES` folds `matches`/`is_in`/`∈`
    ///   with no negated counterpart.
    ///
    /// The only negation the grammars DO admit is prefix `not`/`~`/`!`/`¬` on a
    /// whole boolean expression (`base_expressions.g4` `boolean_expr : SYM_NOT
    /// boolean_expr`), i.e. inside a slot `include`/`exclude` assertion or the
    /// rules section, which this crate parses through `openehr_lang::bel`. So
    /// the negated operator is refused HERE, in cADL constraint position, and
    /// accepted there — never silently read as an affirmative `matches`, which
    /// would invert the constraint.
    fn negated_matches_reject<T>(&mut self, code: SyntaxErrorCode) -> PResult<T> {
        self.err(
            code,
            "a negated matches operator ('~matches' / '~is_in' / '∉') is not a cADL production; \
             negation is available as the prefix 'not' operator of a slot include/exclude assertion",
        )
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
                // The lexeme and span are already in the pushed diagnostic; a
                // `ParseIntError` adds nothing to it.
                let Ok(v) = s.parse::<i32>() else {
                    self.push(
                        code,
                        format!("invalid integer {s:?}"),
                        self.span_at(self.pos - 1),
                    );
                    return Err(());
                };
                Ok(v)
            }
            _ => self.err(code, "expected an integer"),
        }
    }

    /// A type-headed object: `c_complex_object` or `c_regular_primitive_object`
    /// (`cadl2.g4`). Distinguished by whether the `matches { … }` body (or the
    /// bare, body-less form) holds attribute defs or a single inline primitive.
    #[expect(
        clippy::too_many_lines,
        reason = "one linear parse of a type-headed object — node bracket, optional OPT ref, body; the steps are sequential, not extractable units"
    )]
    fn parse_type_object(&mut self) -> PResult<CObject> {
        let type_span = self.cur_span();
        let rm_type = self.parse_rm_type_id()?;
        // NOTE: `cadl2.g4` requires `'[' ID_CODE ']'`, but a *missing* node id
        // is a semantic defect (VCOID, `AOM2/master08`), not a syntax error —
        // the ADL Workbench parses it and flags VCOID in validation. We do the
        // same: an absent `[…]` yields an empty node id that validation flags.
        // OPT-inlined `C_ARCHETYPE_ROOT` form `TYPE[id, archetype_ref] …`: a
        // flattened slot-filler / external reference carries the full archetype
        // id inside the node bracket and an inline body (OPT2 master03
        // §Artefact Structure + §Flattening; the same `'[' ID_CODE ','
        // archetype_ref ']'` shape as `cadl14.g4` `c_archetype_root`, kept for
        // the OPT serialisation the `operational_template` printer round-trips).
        let mut archetype_ref: Option<String> = None;
        let node_id = if self.eat(|t| matches!(t, Token::LBracket)) {
            let n = self.parse_node_id()?;
            if self.eat(|t| matches!(t, Token::SymComma)) {
                match self.peek().cloned() {
                    Some(Token::ArchetypeId(a)) => {
                        self.pos += 1;
                        archetype_ref = Some(a);
                    }
                    _ => {
                        return self.err(
                            SyntaxErrorCode::Suaid,
                            "expecting an archetype id after ',' in a node reference",
                        );
                    }
                }
            }
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

        if self.at_negated_matches() {
            return self.negated_matches_reject(SyntaxErrorCode::Sccog);
        }
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

        // An `archetype_ref` in the node bracket makes this a `C_ARCHETYPE_ROOT`
        // (OPT-inlined filler / external reference), carrying the body as its
        // flattened structure (OPT2 master03 §Flattening).
        if let Some(archetype_ref) = archetype_ref {
            obj = into_archetype_root(obj, archetype_ref);
        }
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
                    // ADL2-only: the `_default` pseudo-attribute is introduced by
                    // `ADL2/master06-default_values.adoc` §Default Values; the 1.4
                    // cADL keyword set (master05 §Keywords L48-53) has no such
                    // construct and no 1.4 chapter defines default values.
                    if self.dialect == Dialect::Adl14 {
                        return self
                            .adl2_only(SyntaxErrorCode::Scoat, "the '_default' pseudo-attribute");
                    }
                    default = Some(self.parse_default_value()?);
                    break; // default_value is last in the body (`cadl2.g4`).
                }
                Some(Token::LBracket) => {
                    // ADL2-only: second-order attribute tuples
                    // (`ADL2/master04.4-cadl_second_order.adoc` §Second Order
                    // Constraints). ADL 1.4 expresses the same co-constraint with
                    // an inline dADL domain type — `AOM2/master04.3` §Tuple
                    // Constraints: "The tuple constraint type replaces all
                    // domain-specific constraint types defined in ADL/AOM 1.4,
                    // including `C_DV_QUANTITY` and `C_DV_ORDINAL`."
                    if self.dialect == Dialect::Adl14 {
                        return self.adl2_only(
                            SyntaxErrorCode::Scoat,
                            "a second-order attribute tuple '[attr, …] matches {…}'",
                        );
                    }
                    tuples.push(self.parse_c_attribute_tuple()?);
                }
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

        if self.at_negated_matches() {
            return self.negated_matches_reject(SyntaxErrorCode::Scoat);
        }
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
        // NOTE: `ADL2/master04.3` §Cardinality — when a `cardinality` clause omits
        // the ordering/uniqueness modifier, the constraint defaults to an ordered,
        // non-unique container (`ordered`, `not unique`). This is the spec default
        // applied verbatim; the RM container kind (List/Set/Bag) is not consulted
        // here because the cardinality stated in the archetype is authoritative for
        // the constraint (no openEHR rule refines a stated cardinality from the RM
        // container shape at parse time).
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
        // The optional RM-type cast of the typed form
        // `_default = (DV_CODED_TEXT) < … >` (`master06-default_values.adoc`
        // §Syntax); it lands as the value's `_type` in the canonical JSON.
        let mut cast: Option<String> = None;
        if self.eat(|t| matches!(t, Token::LParen)) {
            let Some(Token::AlphaUcId(name)) = self.peek() else {
                return self.err(
                    SyntaxErrorCode::Sadf,
                    "expecting an RM type name in the '_default = (TYPE)' cast",
                );
            };
            cast = Some(name.clone());
            self.pos += 1;
            self.expect(
                |t| matches!(t, Token::RParen),
                SyntaxErrorCode::Sadf,
                "expecting ')' closing the '_default' type cast",
            )?;
        }
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
            Ok(v) => {
                if odin_contains_interval(&v) {
                    self.push(
                        SyntaxErrorCode::Sdinv,
                        "interval values are not supported in a '_default' value",
                        start_byte..end_byte,
                    );
                    return Err(());
                }
                let mut json = odin_to_json(&v);
                if let Some(t) = cast
                    && let serde_json::Value::Object(m) = &mut json
                {
                    m.insert("_type".to_owned(), serde_json::Value::String(t));
                }
                Ok(json)
            }
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
        // 1.4-only (converter front end; no openEHR spec — see `crate::adl14`):
        // a qualified/listed terminology constraint (`[local::at1]`,
        // `[local:: a, b ; c]`, `[openehr::524]`). A single `[local::code]` is
        // one `TermCodeRef` token; a list lexes as `[` ident `::` codes …. Both
        // must reach `parse_adl14_term_object` rather than the ADL2 inline
        // terminology-code path (which expects a bare `[at1]`/`[ac1]`).
        if self.dialect == Dialect::Adl14 && self.is_adl14_qualified_code_start() {
            let obj = self.parse_adl14_term_object()?;
            return Ok(vec![obj]);
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
    ///
    /// NOTE: the `before`/`after` sibling-order markers are NOT dialect-gated.
    /// `ADL1.4/master05-cadl.adoc` §Keywords L53 lists `before` and `after` among
    /// "the keywords … recognised in cADL" for the 1.4 formalism itself, so a
    /// sibling order in a 1.4 text is legal 1.4 cADL — even though no 1.4 chapter
    /// elaborates its semantics beyond that list. Refusing it here would
    /// contradict the 1.4 keyword set.
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
    /// One object inside an attribute body.
    ///
    /// NOTE: the `=~` / `!~` regex-match operators the chapter's prose shows
    /// (`ADL1.4/master05-cadl.adoc` §Regular Expression L691-693:
    /// `string_attr matches {=~ /regular expression}` …
    /// `{!~ /regular expression}`) are refused here, because no normative
    /// grammar defines them and the prose that does is itself malformed — both
    /// example regexes are UNTERMINATED (opening `/` with no closing `/`),
    /// while the sentence that follows says the first two forms are "identical",
    /// i.e. `=~` adds nothing. The chapter's own §Syntax gives
    /// `c_string_spec : V_STRING | string_list_value | string_list_value ','
    /// SYM_LIST_CONTINUE | V_REGEXP` (L1244-1249) with `V_REGEXP` the bare
    /// delimited form (L1471-1476), its §Symbols lexer has no `=~`/`!~` token,
    /// and `cadl14_primitives.g4` `c_string` is `( string_value |
    /// string_list_value | regex_constraint )` with `regex_constraint :
    /// SLASH_REGEX | CARET_REGEX`. Guessing a negated-regex semantics from
    /// defective prose would be a silent wrong answer, so the operator is a
    /// typed refusal naming the defect.
    fn parse_c_regular_object(&mut self) -> PResult<CObject> {
        if self.at_regex_match_operator() {
            return self.err(
                SyntaxErrorCode::Sccog,
                "the '=~' / '!~' regex-match operators are not a cADL production; \
                 write the regex constraint directly, as '{/re/}' or '{^re^}'",
            );
        }
        if self.dialect == Dialect::Adl14 {
            // 1.4-only object forms (converter front end; no openEHR spec —
            // see `crate::adl14`): a bare qualified/listed terminology
            // constraint, or an inline dADL domain block `(TYPE) <…>`.
            match self.peek() {
                Some(Token::TermCodeRef(_) | Token::LBracket) => {
                    return self.parse_adl14_term_object();
                }
                Some(Token::LParen) => return self.parse_adl14_domain_object(true),
                // ADL2-only: `use_archetype` (`C_ARCHETYPE_ROOT`). The 1.4 cADL
                // keyword set (master05 §Keywords L51) has `use_node` and
                // `allow_archetype` only — an external archetype reference is
                // expressed in 1.4 by an `allow_archetype` slot whose assertions
                // name the archetype id (master05 §Archetype Slots).
                Some(Token::SymUseArchetype) => {
                    return self.adl2_only(SyntaxErrorCode::Sccog, "'use_archetype'");
                }
                // Bare `C_DV_QUANTITY <…>` / `(C_CODE_PHRASE) <…>` (no parens): a
                // domain type immediately followed by an ODIN block would
                // otherwise be misread as a generic type by `parse_rm_type_id`.
                Some(Token::AlphaUcId(_)) if self.is_adl14_domain_block_start() => {
                    return self.parse_adl14_domain_object(false);
                }
                _ => {}
            }
        }
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

    /// True if the cursor is at a BARE (unparenthesised) 1.4 inline dADL domain
    /// block: a type name immediately followed by `<` whose first inner token is
    /// an attribute name.
    ///
    /// `ADL1.4/master09-customising_adl.adoc` §Introduction fixes the shape —
    /// "the dADL section must be 'typed', i.e. it must start with a type name"
    /// followed by the ODIN object — so the discriminator against a generic cADL
    /// type (`HISTORY<ITEM_LIST>`, whose `<` is followed by a TYPE name) is the
    /// case of the token after `<`. Matching on the shape rather than on a
    /// known-type list is what lets an unsupported domain type
    /// (`C_CODE_PHRASE <…>`) reach the typed refusal in
    /// [`Parser::parse_adl14_domain_object`] instead of being mis-parsed as a
    /// generic type.
    fn is_adl14_domain_block_start(&self) -> bool {
        matches!(self.peek(), Some(Token::AlphaUcId(_)))
            && matches!(self.peek_at(1), Some(Token::SymLt))
            && matches!(self.peek_at(2), Some(Token::AlphaLcId(_)))
    }

    /// True if the cursor is at a 1.4 qualified/listed terminology constraint
    /// (`[terminology::…]`): either a single-token `TermCodeRef`, or a
    /// `[` ident `::` opening (a code list the lexer split into loose tokens).
    fn is_adl14_qualified_code_start(&self) -> bool {
        match self.peek() {
            Some(Token::TermCodeRef(_)) => true,
            Some(Token::LBracket) => {
                matches!(
                    self.peek_at(1),
                    Some(Token::AlphaLcId(_) | Token::AlphaUcId(_))
                ) && matches!(self.peek_at(2), Some(Token::SymColon))
            }
            _ => false,
        }
    }

    /// 1.4-only: a bare terminology-code object — a single qualified code
    /// (`[local::at0001]` / `[openehr::524]`, one `TermCodeRef` token) or a
    /// code list (`[local:: a, b, c ; assumed]`, loose tokens). The full 1.4
    /// form is preserved verbatim in `C_TERMINOLOGY_CODE.constraint` for
    /// `crate::adl14::convert` to rewrite. No openEHR spec governs this — our
    /// own design (1.4→2 converter front end).
    ///
    /// The list form additionally carries the two catalogue rules on the code
    /// list itself — STCDC (duplicates) and STCAC (an assumed code outside the
    /// list), both raised at position below.
    #[allow(clippy::too_many_lines)] // one linear parse: bracket, codes, assumed, the two list rules
    fn parse_adl14_term_object(&mut self) -> PResult<CObject> {
        let constraint = if let Some(Token::TermCodeRef(raw)) = self.peek().cloned() {
            self.pos += 1;
            // `[terminology::code]` → `terminology::code`.
            raw.trim_start_matches('[').trim_end_matches(']').to_owned()
        } else {
            // `[` terminology `::` code ( `,` code )* ( `;` assumed )? `]`.
            let start = self.cur_span().start;
            self.expect(
                |t| matches!(t, Token::LBracket),
                SyntaxErrorCode::Stccp,
                "expecting '[' opening a terminology code",
            )?;
            let Some(Token::AlphaLcId(terminology) | Token::AlphaUcId(terminology)) = self.bump()
            else {
                return self.err(SyntaxErrorCode::Stccp, "expecting a terminology id");
            };
            // `::`
            self.expect(
                |t| matches!(t, Token::SymColon),
                SyntaxErrorCode::Stccp,
                "expecting ':' in a qualified code",
            )?;
            self.expect(
                |t| matches!(t, Token::SymColon),
                SyntaxErrorCode::Stccp,
                "expecting '::' in a qualified code",
            )?;
            let mut codes: Vec<String> = Vec::new();
            let mut assumed: Option<String> = None;
            loop {
                match self.peek().cloned() {
                    // External codes may be bare integers (`[openehr:: 253, …]`)
                    // as well as at/ac/id codes (`[local:: at0136, …]`).
                    Some(
                        Token::AtCode(c) | Token::AcCode(c) | Token::IdCode(c) | Token::Integer(c),
                    ) => {
                        self.pos += 1;
                        codes.push(c);
                    }
                    _ => return self.err(SyntaxErrorCode::Stccp, "expecting an at/ac code"),
                }
                if self.eat(|t| matches!(t, Token::SymComma)) {
                    continue;
                }
                if self.eat(|t| matches!(t, Token::SymSemiColon)) {
                    match self.peek().cloned() {
                        Some(Token::AtCode(a)) => {
                            self.pos += 1;
                            assumed = Some(a);
                        }
                        _ => {
                            return self
                                .err(SyntaxErrorCode::Stccp, "assumed value must be an at-code");
                        }
                    }
                }
                break;
            }
            let list_span = start..self.cur_span().end;
            self.expect(
                |t| matches!(t, Token::RBracket),
                SyntaxErrorCode::Stccp,
                "expecting ']' closing a terminology code",
            )?;
            // STCDC: "duplicate code(s) found in code list"
            // (`ADL2/master04.6-cadl_validity_rules.adoc` §Syntax Validity
            // Rules). A repeated member adds no value to the constrained set
            // and is a defect in the source, not a silently-collapsed set.
            let mut seen: Vec<&str> = Vec::with_capacity(codes.len());
            let duplicates: Vec<&str> = codes
                .iter()
                .filter(|c| {
                    let dup = seen.contains(&c.as_str());
                    seen.push(c.as_str());
                    dup
                })
                .map(String::as_str)
                .collect();
            if !duplicates.is_empty() {
                self.push(
                    SyntaxErrorCode::Stcdc,
                    format!(
                        "duplicate code(s) found in code list: {}",
                        duplicates.join(", ")
                    ),
                    list_span.clone(),
                );
                return Err(());
            }
            // STCAC: "assumed value code $1 not found in code list" (same
            // catalogue). `ADL1.4/master05-cadl.adoc` §Assumed Values L1012
            // requires the assumed value to be "a value of the same type as
            // that implied by the preceding part of the constraint" — for a
            // listed term constraint the implied type is the listed set, so an
            // assumed code outside it can never be assumed.
            if let Some(a) = assumed.as_deref()
                && !codes.iter().any(|c| c == a)
            {
                self.push(
                    SyntaxErrorCode::Stcac,
                    format!("assumed value code {a} not found in code list"),
                    list_span,
                );
                return Err(());
            }
            let mut s = format!("{terminology}::{}", codes.join(","));
            if let Some(a) = assumed {
                s.push(';');
                s.push_str(&a);
            }
            s
        };
        Ok(CObject::CTerminologyCode(CTerminologyCode {
            parent: None,
            soc_parent: None,
            rm_type_name: "Terminology_code".to_owned(),
            occurrences: None,
            node_id: "Primitive_node_id".to_owned(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint,
            constraint_status: None,
        }))
    }

    /// 1.4-only: an inline dADL domain constraint `(C_DV_QUANTITY) <…>` /
    /// `C_DV_ORDINAL <…>`. The ODIN block is parsed via `openehr_lang::odin`
    /// and lowered to a `DV_QUANTITY`/`DV_ORDINAL` `C_COMPLEX_OBJECT` (the RM
    /// type the domain constrainer targets), carrying the `property` external
    /// code as a `C_TERMINOLOGY_CODE`, the `list` rows as an attribute tuple
    /// (multi-member) or plain attributes (single member), and the
    /// `assumed_value` object as the per-leaf `C_PRIMITIVE_OBJECT.assumed_value`s
    /// it decomposes into. No openEHR spec governs the 1.4→2 lowering — our own
    /// design (converter front end); `AOM2/master04.3` §Tuple Constraints is the
    /// ADL2 target ("The tuple constraint type replaces all domain-specific
    /// constraint types defined in ADL/AOM 1.4, including `C_DV_QUANTITY` and
    /// `C_DV_ORDINAL`").
    ///
    /// A domain type this lowering does not model is refused with a typed
    /// [`SyntaxErrorCode::Sdinv`] naming the type, never lowered to some other
    /// type: `ADL1.4/master09-customising_adl.adoc` §Introduction admits any
    /// `C_DOMAIN_TYPE` descendant here ("This approach can be used for any custom
    /// type which represents a constraint on a reference model type"), and each
    /// one targets a DIFFERENT RM type, so guessing is a silent wrong answer.
    // TODO: lower the remaining openEHR Archetype Profile domain constrainers
    // (`C_CODE_PHRASE` → `CODE_PHRASE`, `C_DV_STATE` → `DV_STATE`;
    // `ADL1.4/master09-customising_adl.adoc` §Custom Syntax) so they stop being
    // refused — tracked as the ADL1.4 master09 chapter audit.
    fn parse_adl14_domain_object(&mut self, parenthesised: bool) -> PResult<CObject> {
        let start = self.cur_span().start;
        if parenthesised {
            self.pos += 1; // '('
        }
        let Some(Token::AlphaUcId(rm_type)) = self.bump() else {
            return self.err(
                SyntaxErrorCode::Sccog,
                "expecting a domain constrainer type",
            );
        };
        if parenthesised {
            self.expect(
                |t| matches!(t, Token::RParen),
                SyntaxErrorCode::Sccog,
                "expecting ')' after the domain type",
            )?;
        }
        // The ODIN block spans from the opening '<' to its matching '>'.
        //
        // The `<`/`>` nesting depth is counted OUTSIDE `|…|` intervals only: a
        // one-sided interval endpoint carries its own `<`/`>` relational
        // operator (`magnitude = <|>0.0|>`), which would otherwise close the
        // block early and hand `openehr_lang::odin` a truncated text. The 1.4
        // chapter flags exactly this hazard in its own scanner specification —
        // `ADL1.4/master05-cadl.adoc` §Symbols L1448-1453,
        // `V_C_DOMAIN_TYPE`: "this is an attempt to match a dADL section inside
        // cADL. It will probably never work 100% properly since there can be
        // '>' inside '||' ranges" — so the interval delimiter is tracked
        // instead of scanning raw characters.
        let open = self.pos;
        if !matches!(self.peek(), Some(Token::SymLt)) {
            return self.err(
                SyntaxErrorCode::Sdinv,
                "expecting '<' opening a domain block",
            );
        }
        let mut depth = 0usize;
        let mut in_interval = false;
        let mut close = None;
        while let Some(tok) = self.peek() {
            match tok {
                Token::SymIvlDelim => in_interval = !in_interval,
                Token::SymLt if !in_interval => depth += 1,
                Token::SymGt if !in_interval => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(self.pos);
                        self.pos += 1;
                        break;
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }
        let Some(close) = close else {
            return self.err(SyntaxErrorCode::Sdinv, "unterminated domain block '<…>'");
        };
        let span = start..self.span_at(close).end;
        if !is_adl14_domain_type(&rm_type) {
            self.push(
                SyntaxErrorCode::Sdinv,
                format!(
                    "inline dADL domain constrainer {rm_type:?} is not supported; only \
                     'C_DV_QUANTITY' and 'C_DV_ORDINAL' are lowered"
                ),
                span,
            );
            return Err(());
        }
        let block = self
            .src
            .get(self.span_at(open).start..self.span_at(close).end)
            .unwrap_or_default();
        let Ok(odin) = openehr_lang::odin::parse(block) else {
            self.push(SyntaxErrorCode::Sdinv, "invalid dADL in domain block", span);
            return Err(());
        };
        match lower_adl14_domain(&rm_type, &odin) {
            Ok(obj) => Ok(obj),
            Err(DomainLoweringError::Empty) => {
                self.push(
                    SyntaxErrorCode::Sdinv,
                    "empty or unsupported inline dADL domain block",
                    span,
                );
                Err(())
            }
            Err(DomainLoweringError::AssumedValueUnmatched(attrs)) => {
                self.push(
                    SyntaxErrorCode::Sdinv,
                    format!(
                        "the domain block's 'assumed_value' ({attrs}) satisfies none of its \
                         'list' rows"
                    ),
                    span,
                );
                Err(())
            }
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
        // ADL 1.4 `use_node TYPE /path` carries no `[id]` bracket (the converter
        // synthesises one). Accept the missing bracket in the 1.4 dialect;
        // `cadl2.g4` mandates it otherwise. No openEHR spec governs 1.4→2 — see
        // `crate::adl14`.
        let node_id =
            if self.dialect == Dialect::Adl14 && !matches!(self.peek(), Some(Token::LBracket)) {
                String::new()
            } else {
                self.expect(
                    |t| matches!(t, Token::LBracket),
                    SyntaxErrorCode::Sccog,
                    "expecting '[' after 'use_node'",
                )?;
                let n = self.parse_node_id()?;
                self.expect(
                    |t| matches!(t, Token::RBracket),
                    SyntaxErrorCode::Sccog,
                    "expecting ']' after the node id",
                )?;
                n
            };
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
    ///
    /// In the ADL 1.4 dialect the `[node_id]` is OPTIONAL: ADL1.4
    /// master05-cadl.adoc §Archetype Slots writes the anonymous form
    /// (`allow_archetype OBSERVATION occurrences ∈ {0..1} ∈ {…}`) in its own
    /// normative examples, and §cADL node types shows the identified form
    /// (`allow_archetype ENTRY[at2002]`) — both are legal 1.4 source. The
    /// AOM 1.4 node-id rule (anonymous where no sibling disambiguation is
    /// needed) is enforced by VCOID in the 1.4 validation pass, not here;
    /// `cadl2.g4` mandates the bracket in ADL 2.
    fn parse_archetype_slot(&mut self) -> PResult<CObject> {
        self.pos += 1; // SYM_ALLOW_ARCHETYPE
        let rm_type = self.parse_rm_type_id()?;
        let node_id =
            if self.dialect == Dialect::Adl14 && !matches!(self.peek(), Some(Token::LBracket)) {
                String::new()
            } else {
                self.expect(
                    |t| matches!(t, Token::LBracket),
                    SyntaxErrorCode::Sccog,
                    "expecting '[' after 'allow_archetype'",
                )?;
                let n = self.parse_node_id()?;
                self.expect(
                    |t| matches!(t, Token::RBracket),
                    SyntaxErrorCode::Sccog,
                    "expecting ']' after the node id",
                )?;
                n
            };

        let mut is_closed = false;
        let mut occurrences = None;
        let mut includes = Vec::new();
        let mut excludes = Vec::new();

        if matches!(self.peek(), Some(Token::SymClosed)) {
            // ADL2-only: the `closed` slot marker (`ADL2/master04.3` §Archetype
            // Slots; `ARCHETYPE_SLOT.is_closed`, redefinition rule VDSSC). The 1.4
            // cADL keyword set (master05 §Keywords L51-52) has `allow_archetype`
            // with `include`/`exclude` only.
            if self.dialect == Dialect::Adl14 {
                return self
                    .adl2_only(SyntaxErrorCode::Sccog, "the archetype-slot 'closed' marker");
            }
            self.pos += 1;
            is_closed = true;
        } else {
            if matches!(self.peek(), Some(Token::SymOccurrences)) {
                occurrences = Some(self.parse_occurrences()?);
            }
            if self.at_negated_matches() {
                return self.negated_matches_reject(SyntaxErrorCode::Sccog);
            }
            if self.eat(|t| matches!(t, Token::SymMatches)) {
                self.expect(
                    |t| matches!(t, Token::LCurly),
                    SyntaxErrorCode::Sccog,
                    "expecting '{' after 'matches' in a slot",
                )?;
                if self.eat(|t| matches!(t, Token::SymInclude)) {
                    includes.extend(self.parse_slot_assertions()?);
                }
                if self.eat(|t| matches!(t, Token::SymExclude)) {
                    excludes.extend(self.parse_slot_assertions()?);
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

    /// Parse the assertion block after a slot `include`/`exclude` keyword
    /// (`master04.3` §Archetype Slots; cADL grammar `c_includes : SYM_INCLUDE
    /// assertion+`).
    ///
    /// The block is captured as a raw span (the token run to the next
    /// `exclude`/`}` at brace-depth 0) and handed to
    /// [`crate::rules::parse_slot_assertions`], which parses it via the BEL
    /// composition into one or more AOM [`Assertion`] trees
    /// (`EXPR_ARCHETYPE_REF matches EXPR_ARCHETYPE_ID_CONSTRAINT`, `master05`);
    /// the verbatim source is preserved in each `string_expression` and the
    /// `archetype_id/value matches {/regex/}` regex is compile-checked (`SCSRE`).
    /// A block may carry more than one assertion (grammar `assertion+`), so every
    /// parsed assertion is returned.
    fn parse_slot_assertions(&mut self) -> PResult<Vec<Assertion>> {
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
        // Parse the real assertion tree(s) (`EXPR_ARCHETYPE_REF matches
        // EXPR_ARCHETYPE_ID_CONSTRAINT`, `master05` / `master04.3`) via the BEL
        // AOM composition; `string_expression` keeps the verbatim source.
        match crate::rules::parse_slot_assertions(text) {
            Ok(assertions) => Ok(assertions),
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
                | Token::Character(_)
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
            Some(Token::Character(_)) => self.parse_c_character(node_id),
            Some(Token::LBracket) => self.parse_c_terminology_code(node_id, None),
            Some(Token::AlphaLcId(s)) if is_strength_keyword(&s) => {
                // ADL2-only: constraint strengths (`required`/`extensible`/
                // `preferred`/`example`) are `C_TERMINOLOGY_CODE.constraint_status`,
                // introduced by `AOM2/master04.2` §Constraint Strengths ("Uniquely
                // in the AOM, a Terminology code constraint may not be required").
                // The 1.4 cADL keyword set (master05 §Keywords L48-53) has no
                // strength vocabulary and AOM 1.4 has no such attribute.
                if self.dialect == Dialect::Adl14 {
                    return self.adl2_only(
                        SyntaxErrorCode::Stccp,
                        format!("the term-constraint strength {s:?}").as_str(),
                    );
                }
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
    /// endpoint token (skipping relational operators, signs, and the unbounded
    /// endpoint markers — `|-infinity..5.0|` is typed by its UPPER endpoint).
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
                    | Token::SymInfinity
                    | Token::SymStar
                    | Token::SymIvlSep
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
            if !self.eat(|t| matches!(t, Token::SymComma)) || self.eat_list_continue() {
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

    /// `c_character` — a `Character` constraint written as a list of
    /// single-quoted characters (`ADL1.4/master05-cadl.adoc` §Constraints on
    /// Character L813-825, identically `ADL2/master04.5` §Constraints on
    /// Character L160-172: "`color_name matches {'r', 'g', 'b'}`"). The
    /// single-character regex form (L827-841) needs no separate production —
    /// it is a `CONTAINED_REGEXP` and lands on the same carrier.
    ///
    /// NOTE: there is no `C_CHARACTER` class to build. Neither vendored AOM
    /// generation defines one — the AM 1.4.0 BMM's constraint model has
    /// `C_BOOLEAN`/`C_STRING`/`C_INTEGER`/`C_REAL`/`C_DATE`/`C_TIME`/
    /// `C_DATE_TIME`/`C_DURATION`/`C_ORDINAL`/`C_CODED_TEXT`/`C_QUANTITY` and
    /// the AM 2.4.0 BMM's has the AOM2 successors of the same set, neither with
    /// a character constrainer, and `cadl14_primitives.g4` `c_primitive_object`
    /// likewise has no `c_character` alternative. The carrier is therefore
    /// `C_STRING` — whose value space, a set of literal strings, contains the
    /// single-character strings exactly — with the constrained RM type name
    /// (`Character`) kept on the object so nothing downstream has to guess.
    /// No openEHR spec governs this mapping — our own design/extension.
    fn parse_c_character(&mut self, node_id: String) -> PResult<CObject> {
        let mut constraint = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Token::Character(c)) => {
                    self.pos += 1;
                    constraint.push(decode_character(&c));
                }
                _ => return self.err(SyntaxErrorCode::Scsav, "expecting a character value"),
            }
            if !self.eat(|t| matches!(t, Token::SymComma)) || self.eat_list_continue() {
                break;
            }
        }
        let assumed_value = if self.eat(|t| matches!(t, Token::SymSemiColon)) {
            match self.peek().cloned() {
                Some(Token::Character(c)) => {
                    self.pos += 1;
                    Some(decode_character(&c))
                }
                _ => {
                    return self.err(SyntaxErrorCode::Scsav, "assumed value must be a character");
                }
            }
        } else {
            None
        };
        Ok(CObject::CString(CString {
            parent: None,
            soc_parent: None,
            rm_type_name: "Character".to_owned(),
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
            if !self.eat(|t| matches!(t, Token::SymComma)) || self.eat_list_continue() {
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
        //
        // ADL2-only: the `[acN@terminology]` operational binding belongs to the
        // ADL2 terminology-integration surface (`ADL2/master08-terminology_integration.adoc`
        // §Terminology Bindings / OPT2 operational form). The 1.4 cADL keyword set
        // (master05 §Keywords L48-53) has no `@` operator, and 1.4 expresses
        // external terminology through the qualified `[terminology::code, …]` form
        // of `ADL1.4/master09-customising_adl.adoc` §Custom Syntax.
        if matches!(self.peek(), Some(Token::SymAt)) && self.dialect == Dialect::Adl14 {
            return self.adl2_only(
                SyntaxErrorCode::Stccp,
                "an '@terminology' operational binding on a term-code constraint",
            );
        }
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

    /// Consume a trailing list-continuation marker (`, ...`) — the comma is
    /// already eaten by the caller, so this only tests for the `...`.
    ///
    /// Every ODIN list production ends with the alternative
    /// `',' SYM_LIST_CONTINUE` (`odin_values.g4` `string_list_value` and its
    /// siblings), and the cADL 1.4 grammar spells it out for strings as
    /// `c_string_spec : … | string_list_value ',' SYM_LIST_CONTINUE`
    /// (`ADL1.4/master05-cadl.adoc` §Syntax L1244-1249).
    ///
    /// NOTE: the marker adds no member and no openness flag. Its only normative
    /// gloss in the vendored specs is a LIST INDICATOR — `ADL1.4/master04-dadl`
    /// §Data of any primitive type L686 and `LANG/docs/odin/master07-leaf_data`
    /// §Lists L208, both: "Lists which happen to have only one datum are
    /// indicated by using a comma followed by a list continuation marker of
    /// three dots". AOM 1.4's `C_STRING.list_open` ("the list … is not
    /// considered exhaustive") is the only model property the marker could
    /// otherwise set, but no vendored ADL 1.4 text binds the syntax to it, and
    /// the AOM2 `C_STRING` this parser builds has no such property at all
    /// (AM 2.4.0 BMM: `constraint` / `default_value` / `assumed_value` only).
    /// Inferring openness from the marker would silently turn a stated
    /// constraint into "any value"; the list-indicator reading is the one the
    /// spec states, so the constraint is exactly the listed values.
    fn eat_list_continue(&mut self) -> bool {
        self.eat(|t| matches!(t, Token::SymListContinue))
    }

    /// Parse a comma-separated list of `Interval<V>` items (a value list, an
    /// interval, or a list of intervals — the AOM2 constraint is a flat
    /// `Vec<Interval<V>>` regardless).
    fn parse_value_list<V: CadlValue>(&mut self) -> PResult<Vec<Interval<V>>> {
        let mut out = Vec::new();
        loop {
            out.push(self.parse_value_item::<V>()?);
            if !self.eat(|t| matches!(t, Token::SymComma)) || self.eat_list_continue() {
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
    /// (`|a..b|`, `|>a..<b|`, `|a>..<b|`), single-relop (`|>a|`,`|<=a|`), point
    /// (`|a|`), or centre±delta (`|a+/-b|`). Either endpoint may be an unbounded
    /// marker (see [`Parser::parse_endpoint`]).
    ///
    /// The exclusive LOWER bound of a two-sided interval has two spellings and
    /// both are accepted: the prefix `>` of the normative grammar
    /// (`odin_values.g4` `integer_interval_value : '|' SYM_GT? integer_value
    /// '..' SYM_LT? integer_value '|'`) and the postfix `>` the ADL 1.4 chapters
    /// write — `ADL1.4/master04-dadl.adoc` §Intervals of Ordered Primitive Types
    /// L611-614 (`|N>..M|` "the two-sided range N > x <= M", `|N>..<M|`) and the
    /// cADL chapter's own worked example `length matches {|0>..<1000|}`
    /// (`ADL1.4/master05-cadl.adoc` §Interval of Integer L769). Accepting both is
    /// a superset that rejects nothing either source admits.
    fn parse_bar_interval<V: CadlValue>(&mut self) -> PResult<Interval<V>> {
        self.check_interval_timezone_symmetry()?;
        self.pos += 1; // opening '|'
        let lower_rel = self.eat_relop();
        let first = self.parse_endpoint::<V>()?;
        // Only a `>` that immediately precedes the `..` is the postfix
        // exclusive-lower marker; anywhere else it is not part of the form.
        let lower_excl_postfix = matches!(self.peek(), Some(Token::SymGt))
            && matches!(self.peek_at(1), Some(Token::SymIvlSep));
        if lower_excl_postfix {
            self.pos += 1;
        }
        let ivl = if self.eat(|t| matches!(t, Token::SymIvlSep)) {
            // Two-sided: [rel] first ['>'] '..' ['<'] upper.
            let upper_excl = self.eat(|t| matches!(t, Token::SymLt));
            let upper = self.parse_endpoint::<V>()?;
            let lower_included =
                first.is_some() && !lower_excl_postfix && !matches!(lower_rel, Some(Relop::Gt));
            let lower_unbounded = first.is_none();
            let upper_unbounded = upper.is_none();
            proper_interval(
                first,
                upper,
                lower_included,
                !upper_excl && !upper_unbounded,
                lower_unbounded,
                upper_unbounded,
            )
        } else if self.eat(|t| matches!(t, Token::SymPlusOrMinus)) {
            let delta = self.parse_endpoint::<V>()?;
            match (&first, &delta) {
                (Some(centre), Some(delta)) => match V::plus_minus(centre, delta) {
                    Some((lo, hi)) => proper_interval(Some(lo), Some(hi), true, true, false, false),
                    // NOTE: `±` on a non-numeric type is not reducible without
                    // RM type context; represented as a point at the centre for
                    // now (rare — not exercised by the primitive corpus).
                    None => point_interval(centre.clone()),
                },
                // An unbounded centre or half-width constrains nothing.
                _ => proper_interval(None, None, false, false, true, true),
            }
        } else {
            // Point `|a|` or single-relop `|>a|`,`|<=a|`.
            match (lower_rel, first) {
                (None, Some(v)) => point_interval(v),
                (Some(Relop::Gt), Some(v)) => {
                    proper_interval(Some(v), None, false, false, false, true)
                }
                (Some(Relop::Ge), Some(v)) => {
                    proper_interval(Some(v), None, true, false, false, true)
                }
                (Some(Relop::Lt), Some(v)) => {
                    proper_interval(None, Some(v), false, false, true, false)
                }
                (Some(Relop::Le), Some(v)) => {
                    proper_interval(None, Some(v), false, true, true, false)
                }
                // A one-sided form whose single endpoint is itself an unbounded
                // marker (`|>=-infinity|`, `|<*|`) constrains nothing at all.
                (_, None) => proper_interval(None, None, false, false, true, true),
            }
        };
        self.expect(
            |t| matches!(t, Token::SymIvlDelim),
            SyntaxErrorCode::Sccog,
            "expecting '|' closing the interval",
        )?;
        Ok(ivl)
    }

    /// The timezone-symmetry rule for a two-sided date/time interval, checked
    /// with the cursor still on the opening `|`.
    ///
    /// `ADL1.4/master05-cadl.adoc` §Intervals L932: "Within any interval
    /// containing two literal date/time values (i.e. not one-sided intervals),
    /// if a timezone is used on one, it must be used on both, to ensure
    /// comparability. The timezones need not be identical." So the rule fires
    /// exactly when the `|…|` window holds TWO literal time / date-time values
    /// whose timezone presence differs; a one-sided form (one value) and a
    /// date interval (dates carry no timezone) are outside its scope.
    ///
    /// NOTE: the openEHR `S*` catalogue
    /// (`ADL2/master04.6-cadl_validity_rules.adoc` §Syntax Validity Rules) has
    /// no dedicated code for this rule, and this crate never invents one — so
    /// the refusal reuses the catalogue code this parser already uses for a bad
    /// time / date-time value in a constraint position (`SCTAV` / `SCDTAV`) and
    /// names the rule in the message.
    fn check_interval_timezone_symmetry(&mut self) -> PResult<()> {
        let mut endpoints: Vec<(bool, SyntaxErrorCode)> = Vec::new();
        let mut i = self.pos + 1;
        while let Some(spanned) = self.toks.get(i) {
            match &spanned.token {
                Token::SymIvlDelim => break,
                Token::Iso8601Time(v) => {
                    endpoints.push((iso_has_timezone(v), SyntaxErrorCode::Sctav));
                }
                Token::Iso8601DateTime(v) => {
                    endpoints.push((iso_has_timezone(v), SyntaxErrorCode::Scdtav));
                }
                _ => {}
            }
            i += 1;
        }
        if let [(lower_tz, code), (upper_tz, _)] = endpoints[..]
            && lower_tz != upper_tz
        {
            let span = self.cur_span().start..self.span_at(i).end;
            self.push(
                code,
                "a two-sided date/time interval must use a timezone on both endpoints or on \
                 neither (the timezones need not be identical)",
                span,
            );
            return Err(());
        }
        Ok(())
    }

    /// One interval endpoint: a value, or an unbounded marker yielding `None`.
    ///
    /// The markers are `infinity`, `-infinity` and `*`. `infinity` is a cADL
    /// keyword in its own right (`ADL1.4/master05-cadl.adoc` §Keywords L50 and
    /// §Symbols L1349 `[Ii][Nn][Ff][Ii][Nn][Ii][Tt][Yy] -> SYM_INFINITY`) and
    /// the chapter's own §Interval of Integer example is
    /// `rate matches {|0..infinity|}  -- allow 0 - infinity, i.e. same as >= 0`
    /// (L771); L761 defers the interval syntax itself to dADL, whose §Intervals
    /// of Ordered Primitive Types lists `infinity` / `-infinity` / `*` as
    /// allowable endpoint values (`ADL1.4/master04-dadl.adoc` L628-643). The
    /// sign carried by `-infinity` is not recorded separately: the side of the
    /// interval the endpoint sits on already fixes the direction — the same
    /// representation `openehr_lang::odin` uses for the identical markers.
    fn parse_endpoint<V: CadlValue>(&mut self) -> PResult<Option<V>> {
        if self.eat(|t| matches!(t, Token::SymStar)) {
            return Ok(None);
        }
        let save = self.pos;
        self.eat(|t| matches!(t, Token::SymMinus));
        if self.eat(|t| matches!(t, Token::SymInfinity)) {
            return Ok(None);
        }
        self.pos = save;
        V::parse_one(self).map(Some)
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
                // The lexeme and span are already in the pushed diagnostic; a
                // `ParseIntError`/`TryFromIntError` adds nothing to it.
                let Ok(v) = s.parse::<i64>() else {
                    self.push(
                        code,
                        format!("invalid integer {s:?}"),
                        self.span_at(self.pos - 1),
                    );
                    return Err(());
                };
                let v = if neg { -v } else { v };
                let Ok(narrowed) = i32::try_from(v) else {
                    self.push(
                        code,
                        format!("integer {v} out of range"),
                        self.span_at(self.pos - 1),
                    );
                    return Err(());
                };
                Ok(narrowed)
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
                // The lexeme and span are already in the pushed diagnostic; a
                // `ParseFloatError` adds nothing to it.
                let v = s.parse::<f64>().ok().ok_or_else(|| {
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
        Ok((format!("/{}/", escape_regex_delimiter(inner)), assumed))
    }

    // ── constraint-pattern validators (`master04.5` valid-pattern tables) ──

    fn validate_date_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        // Fields: year(4)-month(2)-day(2). Degradation: after a `??` field, only
        // `??`/`XX`; after `XX`, only `XX`. A date pattern carries NO timezone
        // modifier — `master05` §Patterns L896 admits one on "any of the time or
        // date/time (but not date) patterns".
        let fields: Vec<&str> = p.split('-').collect();
        let [year, month, day] = fields.as_slice() else {
            return self.pattern_err(code, p);
        };
        if !is_year_field(year) {
            return self.pattern_err(code, p);
        }
        self.validate_pattern_degradation(&[month, day], code, p)
    }

    fn validate_time_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        let fields: Vec<&str> = pattern_time_core(p).split(':').collect();
        let [hour, minute, second] = fields.as_slice() else {
            return self.pattern_err(code, p);
        };
        if !is_present_field(hour, "hh") {
            return self.pattern_err(code, p);
        }
        self.validate_pattern_degradation(&[minute, second], code, p)
    }

    fn validate_date_time_pattern(&mut self, p: &str, code: SyntaxErrorCode) -> PResult<()> {
        // The date/time separator is `T` or a space: the chapter's own
        // `V_ISO8601_DATE_TIME_CONSTRAINT_PATTERN` spells `…[dD?X][dD?X][ T]…`
        // (`ADL1.4/master05-cadl.adoc` §Symbols L1422) and its assumed-value
        // example uses the space form (`yyyy-mm-dd hh:mm:XX; 1800-01-01T00:00:00`,
        // §Assumed Values L1018).
        let Some((date, time)) = p.split_once(['T', ' ']) else {
            return self.pattern_err(code, p);
        };
        let date_fields: Vec<&str> = date.split('-').collect();
        let time_fields: Vec<&str> = pattern_time_core(time).split(':').collect();
        let [year, date_month, date_day] = date_fields.as_slice() else {
            return self.pattern_err(code, p);
        };
        let [hour, minute, second] = time_fields.as_slice() else {
            return self.pattern_err(code, p);
        };
        if !is_year_field(year) {
            return self.pattern_err(code, p);
        }
        // Degradation flows date → time as one chain (`master04.5`): the hour
        // field may itself be `??`/`XX` once the date has degraded.
        self.validate_pattern_degradation(&[date_month, date_day, hour, minute, second], code, p)
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
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per ADL primitive C_* struct literal (master04.5); the length is the size of the primitive set"
    )]
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
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the four flags mirror `ProperIntervalData`'s own boolean fields 1:1 — collapsing them into a struct would just restate that type"
)]
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

/// Convert a parsed complex object into a [`CArchetypeRoot`] carrying
/// `archetype_ref` (the OPT-inlined slot-filler / external-reference form,
/// OPT2 master03). A non-complex `obj` (a primitive) cannot bear an archetype
/// ref; it is returned unchanged (validation flags the misuse).
fn into_archetype_root(obj: CObject, archetype_ref: String) -> CObject {
    let CObject::CComplexObject(CComplexObject::CComplexObject(d)) = obj else {
        return obj;
    };
    CObject::CComplexObject(CComplexObject::CArchetypeRoot(Box::new(CArchetypeRoot {
        parent: None,
        soc_parent: None,
        rm_type_name: d.rm_type_name,
        occurrences: d.occurrences,
        node_id: d.node_id,
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: d.default_value,
        attributes: d.attributes,
        attribute_tuples: d.attribute_tuples,
        archetype_ref,
    })))
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
        terminology_id: LOCAL_TERMINOLOGY_ID.to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

// ── ADL 1.4 inline dADL domain lowering (converter front end) ──────────────
//
// NOTE: no openEHR spec governs 1.4→2 conversion — the whole `adl14` pipeline
// (including this lowering) is our own design (archie's converter is prior
// art). `C_DV_QUANTITY`/`C_DV_ORDINAL` are ADL 1.4-only inline dADL
// constrainers with no ADL2/AOM2 class; ADL2 expresses the same constraint as
// a `DV_QUANTITY`/`DV_ORDINAL` `C_COMPLEX_OBJECT` with an attribute tuple
// (`AOM2/master04.4` §Second-Order Constraints). We lower to that shape and
// leave code renumbering + `property` binding synthesis to
// `crate::adl14::convert`.

fn is_adl14_domain_type(id: &str) -> bool {
    matches!(id, "C_DV_QUANTITY" | "C_DV_ORDINAL")
}

/// Peel any `(TYPE)` casts off an ODIN value: the cast of
/// `LANG/docs/odin/master05-content` §Adding Type Information is a
/// dynamic-binding hint for the parser, not part of the datum, so the domain
/// lowering reads straight through it (the cast stays on the tree for a caller
/// that wants it).
fn untyped(v: &OdinValue) -> &OdinValue {
    let mut cur = v;
    while let OdinValue::Typed { value, .. } = cur {
        cur = value;
    }
    cur
}

/// Why an inline dADL domain block could not be lowered (each maps to `SDINV`).
enum DomainLoweringError {
    /// An empty `<>` block, a bare scalar, or a type this lowering does not model.
    Empty,
    /// The block's `assumed_value` satisfies none of its `list` rows (the
    /// attribute names carried for the message).
    AssumedValueUnmatched(String),
}

/// Lower a parsed 1.4 inline dADL domain block into a `DV_QUANTITY`/`DV_ORDINAL`
/// complex object.
///
/// # Errors
/// [`DomainLoweringError`] for an empty/unusable block or an `assumed_value` that
/// matches no `list` row; the caller turns both into `SDINV`.
fn lower_adl14_domain(rm_type: &str, odin: &OdinValue) -> Result<CObject, DomainLoweringError> {
    let OdinValue::Object(map) = untyped(odin) else {
        // Empty `<>` or a bare scalar — nothing to constrain.
        return Err(DomainLoweringError::Empty);
    };
    if map.is_empty() {
        return Err(DomainLoweringError::Empty);
    }
    let target_rm = match rm_type {
        "C_DV_ORDINAL" => "DV_ORDINAL",
        "C_DV_QUANTITY" => "DV_QUANTITY",
        // Unreachable: the parse site gates the type before calling in. Kept as a
        // typed refusal rather than a fallback so no other domain constrainer can
        // ever be silently lowered to the wrong RM type.
        _ => return Err(DomainLoweringError::Empty),
    };
    let mut attributes: Vec<CAttribute> = Vec::new();
    let mut attribute_tuples: Vec<CAttributeTuple> = Vec::new();

    // `property = <[openehr::122]>` → a `property` at-code constraint (the
    // external code is rewritten to a synthesised at-code + binding by the
    // converter).
    if let Some(OdinValue::TermCode(tc)) = map.get("property").map(untyped) {
        let constraint = tc.trim_start_matches('[').trim_end_matches(']').to_owned();
        attributes.push(cattr_single(
            "property",
            CObject::CTerminologyCode(CTerminologyCode {
                parent: None,
                soc_parent: None,
                rm_type_name: "Terminology_code".to_owned(),
                occurrences: None,
                node_id: "Primitive_node_id".to_owned(),
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: None,
                is_enumerated_type_constraint: None,
                constraint,
                constraint_status: None,
            }),
        ));
    }

    // `list = <["1"] = <units=<…> magnitude=<…>> …>` → per-attribute
    // constraints. One distinct attribute → a plain constraint merging every
    // row's values; two or more → an attribute tuple, one `C_PRIMITIVE_TUPLE`
    // per row (`AOM2/master04.4`).
    if let Some(list) = map.get("list") {
        let rows = domain_list_rows(list);
        // Distinct attribute names in first-appearance order.
        let mut names: Vec<String> = Vec::new();
        for row in &rows {
            for (k, _) in row {
                if !names.iter().any(|n| n == k) {
                    names.push(k.clone());
                }
            }
        }
        if names.len() >= 2 {
            let members: Vec<CAttribute> = names.iter().map(|n| cattr_empty(n)).collect::<Vec<_>>();
            let mut tuples: Vec<CPrimitiveTuple> = Vec::new();
            for row in &rows {
                let mut prim_members = Vec::new();
                for n in &names {
                    let v = row
                        .iter()
                        .find(|(k, _)| k == n)
                        .and_then(|(_, v)| domain_value_to_primitive(n, v))
                        .unwrap_or_else(|| any_primitive(n));
                    prim_members.push(v);
                }
                tuples.push(CPrimitiveTuple {
                    members: prim_members,
                });
            }
            attribute_tuples.push(CAttributeTuple { members, tuples });
        } else if let Some(name) = names.first() {
            // Single attribute: merge every row's values into one constraint.
            let values: Vec<CPrimitiveObject> = rows
                .iter()
                .filter_map(|row| row.iter().find(|(k, _)| k == name))
                .filter_map(|(_, v)| domain_value_to_primitive(name, v))
                .collect();
            if let Some(merged) = merge_primitives(values) {
                attributes.push(cattr_single(name, primitive_to_cobject(merged)));
            }
        }
    }

    // `assumed_value = <units=<"C"> magnitude=<8.0> …>` — the 1.4 domain
    // constrainer's assumed value is an INSTANCE of the constrained RM type
    // (AOM 1.4 `C_DV_QUANTITY.assumed_value: DV_QUANTITY`). AOM2 has no
    // `assumed_value` on `C_COMPLEX_OBJECT` — `AOM2/master04.2` §Assumed_value
    // puts it on `C_PRIMITIVE_OBJECT`/`C_TERMINOLOGY_CODE`, and §Assumed_value
    // L175 expressly separates it from `default_value` ("default values do appear
    // in data, while assumed values don't"), so `default_value` is NOT a legal
    // carrier. The instance is therefore decomposed into its per-attribute leaves
    // and each leaf lands on the `C_PRIMITIVE_OBJECT.assumed_value` of the
    // constraint this lowering already produced for that attribute.
    if let Some(OdinValue::Object(assumed)) = map.get("assumed_value").map(untyped) {
        apply_domain_assumed_values(assumed, &mut attributes, &mut attribute_tuples)?;
    }

    Ok(complex_object(
        target_rm.to_owned(),
        String::new(),
        attributes,
        attribute_tuples,
        None,
    ))
}

/// Land the leaves of a domain block's `assumed_value` object on the constraints
/// `attributes`/`attribute_tuples` already carry.
///
/// A leaf whose attribute is a plain constraint sets that constraint's
/// `assumed_value` directly. A leaf whose attribute is a tuple member sets the
/// member of the ONE tuple row the whole assumed combination satisfies — a tuple
/// row is a co-constrained alternative (`AOM2/master04.3` §Tuple Constraints), so
/// the assumed instance belongs to exactly one row, never to all of them.
///
/// NOTE: a leaf for an attribute the block does not constrain at all (e.g.
/// `precision` in an `assumed_value` whose `list` rows carry only
/// `units`/`magnitude`) has no AOM2 carrier — `assumed_value` is a field OF a
/// `C_PRIMITIVE_OBJECT` (`AOM2/master04.2` §`Assumed_value`), and an unconstrained
/// attribute has no constraint object to hold it. Such a leaf is dropped rather
/// than carried on a fabricated "any" constraint, which has no ADL2 rendering.
/// No openEHR spec governs 1.4→2 conversion — our own design.
///
/// # Errors
/// [`DomainLoweringError::AssumedValueUnmatched`] when tuple members are present
/// and no row admits the assumed combination — the 1.4 source states an assumed
/// value outside its own `list`, which the parse refuses loudly rather than
/// binding to an arbitrary row.
fn apply_domain_assumed_values(
    assumed: &indexmap::IndexMap<String, OdinValue>,
    attributes: &mut [CAttribute],
    attribute_tuples: &mut [CAttributeTuple],
) -> Result<(), DomainLoweringError> {
    // The assumed leaves, as the primitive shape the constraint side uses.
    let leaves: Vec<(String, CPrimitiveObject)> = assumed
        .iter()
        .filter_map(|(name, value)| {
            domain_value_to_primitive(name, value).map(|p| (name.clone(), p))
        })
        .collect();
    if leaves.is_empty() {
        return Ok(());
    }

    // Plain attributes first.
    for (name, leaf) in &leaves {
        if let Some(attr) = attributes.iter_mut().find(|a| &a.rm_attribute_name == name)
            && let Some(child) = attr.children.first_mut()
        {
            set_assumed_on_cobject(child, leaf);
        }
    }

    // Tuple members: pick the single row the whole combination satisfies.
    for tuple in attribute_tuples.iter_mut() {
        let positions: Vec<(usize, &CPrimitiveObject)> = tuple
            .members
            .iter()
            .enumerate()
            .filter_map(|(idx, m)| {
                leaves
                    .iter()
                    .find(|(name, _)| name == &m.rm_attribute_name)
                    .map(|(_, leaf)| (idx, leaf))
            })
            .collect();
        if positions.is_empty() {
            continue;
        }
        let row = tuple.tuples.iter().position(|row| {
            positions.iter().all(|(idx, leaf)| {
                row.members
                    .get(*idx)
                    .is_some_and(|constraint| primitive_admits(constraint, leaf))
            })
        });
        let Some(row) = row else {
            let named = positions
                .iter()
                .filter_map(|(idx, _)| tuple.members.get(*idx))
                .map(|m| m.rm_attribute_name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(DomainLoweringError::AssumedValueUnmatched(named));
        };
        for (idx, leaf) in positions {
            if let Some(row) = tuple.tuples.get_mut(row)
                && let Some(member) = row.members.get_mut(idx)
            {
                set_assumed_on_primitive(member, leaf);
            }
        }
    }
    Ok(())
}

/// True if `constraint` admits the single value `value` carries.
///
/// Deliberately CONSERVATIVE: it answers `false` only where non-membership is
/// positively decidable (a string not in a value list, a number outside every
/// interval). A mismatched kind or an unconstrained (`{*}`) constraint answers
/// `true`, so the refusal it feeds can never fire on a case this lowering does
/// not fully understand.
fn primitive_admits(constraint: &CPrimitiveObject, value: &CPrimitiveObject) -> bool {
    match (constraint, value) {
        (CPrimitiveObject::CString(c), CPrimitiveObject::CString(v)) => {
            c.constraint.is_empty()
                || v.constraint
                    .iter()
                    .all(|want| c.constraint.iter().any(|have| have == want))
        }
        (CPrimitiveObject::CReal(c), CPrimitiveObject::CReal(v)) => {
            c.constraint.is_empty()
                || v.constraint
                    .iter()
                    .filter_map(interval_point_f64)
                    .all(|p| c.constraint.iter().any(|iv| real_interval_contains(iv, p)))
        }
        (CPrimitiveObject::CInteger(c), CPrimitiveObject::CInteger(v)) => {
            c.constraint.is_empty()
                || v.constraint
                    .iter()
                    .filter_map(interval_point_i32)
                    .all(|p| c.constraint.iter().any(|iv| int_interval_contains(iv, p)))
        }
        _ => true,
    }
}

/// Set `leaf`'s single value as the `assumed_value` of the primitive `target`.
fn set_assumed_on_primitive(target: &mut CPrimitiveObject, leaf: &CPrimitiveObject) {
    match (target, leaf) {
        (CPrimitiveObject::CString(t), CPrimitiveObject::CString(l)) => {
            t.assumed_value = l.constraint.first().cloned();
        }
        (CPrimitiveObject::CReal(t), CPrimitiveObject::CReal(l)) => {
            t.assumed_value = l.constraint.first().and_then(interval_point_f64);
        }
        (CPrimitiveObject::CInteger(t), CPrimitiveObject::CInteger(l)) => {
            t.assumed_value = l
                .constraint
                .first()
                .and_then(interval_point_i32)
                .map(f64::from);
        }
        (CPrimitiveObject::CBoolean(t), CPrimitiveObject::CBoolean(l)) => {
            t.assumed_value = l.constraint.first().copied();
        }
        // Kind mismatch (or a leaf kind the domain lowering never produces):
        // leave the constraint untouched rather than coerce across types.
        _ => {}
    }
}

/// Set `leaf`'s single value as the `assumed_value` of the primitive object
/// `target` wraps, if it is one.
fn set_assumed_on_cobject(target: &mut CObject, leaf: &CPrimitiveObject) {
    match target {
        CObject::CString(t) => {
            if let CPrimitiveObject::CString(l) = leaf {
                t.assumed_value = l.constraint.first().cloned();
            }
        }
        CObject::CReal(t) => {
            if let CPrimitiveObject::CReal(l) = leaf {
                t.assumed_value = l.constraint.first().and_then(interval_point_f64);
            }
        }
        CObject::CInteger(t) => {
            if let CPrimitiveObject::CInteger(l) = leaf {
                t.assumed_value = l
                    .constraint
                    .first()
                    .and_then(interval_point_i32)
                    .map(f64::from);
            }
        }
        CObject::CBoolean(t) => {
            if let CPrimitiveObject::CBoolean(l) = leaf {
                t.assumed_value = l.constraint.first().copied();
            }
        }
        _ => {}
    }
}

/// The single point value of a real interval (`{v}` / `{v..v}`), else `None`.
fn interval_point_f64(iv: &Interval<f64>) -> Option<f64> {
    match iv {
        Interval::PointInterval(p) => p.lower,
        Interval::ProperInterval(_) => None,
    }
}

/// The single point value of an integer interval (`{v}` / `{v..v}`), else `None`.
fn interval_point_i32(iv: &Interval<i32>) -> Option<i32> {
    match iv {
        Interval::PointInterval(p) => p.lower,
        Interval::ProperInterval(_) => None,
    }
}

/// The `(lower, upper)` bounds of an interval as `f64`, each `None` when open or
/// unbounded, plus the two inclusivity flags. A `MultiplicityInterval` variant
/// (structurally possible on the generic enum but never produced for a domain
/// leaf constraint) yields fully-open bounds, so membership is undecided and the
/// conservative `true` answer stands.
fn interval_bounds_f64<T: Copy + Into<f64>>(
    iv: &Interval<T>,
) -> (Option<f64>, Option<f64>, bool, bool) {
    let (lower, upper, lower_unbounded, upper_unbounded, lower_included, upper_included) = match iv
    {
        Interval::PointInterval(p) => (
            p.lower,
            p.upper,
            p.lower_unbounded,
            p.upper_unbounded,
            p.lower_included,
            p.upper_included,
        ),
        Interval::ProperInterval(ProperInterval::ProperInterval(p)) => (
            p.lower,
            p.upper,
            p.lower_unbounded,
            p.upper_unbounded,
            p.lower_included,
            p.upper_included,
        ),
        Interval::ProperInterval(ProperInterval::MultiplicityInterval(_)) => {
            return (None, None, true, true);
        }
    };
    (
        if lower_unbounded {
            None
        } else {
            lower.map(Into::into)
        },
        if upper_unbounded {
            None
        } else {
            upper.map(Into::into)
        },
        lower_included,
        upper_included,
    )
}

/// True if the real interval `iv` contains `v` (honouring open/closed bounds).
fn real_interval_contains(iv: &Interval<f64>, v: f64) -> bool {
    bounds_admit(v, interval_bounds_f64(iv))
}

/// True if the integer interval `iv` contains `v` (honouring open/closed bounds).
fn int_interval_contains(iv: &Interval<i32>, v: i32) -> bool {
    bounds_admit(f64::from(v), interval_bounds_f64(iv))
}

/// Interval membership over `f64` bounds, shared by the real/integer tests.
fn bounds_admit(v: f64, bounds: (Option<f64>, Option<f64>, bool, bool)) -> bool {
    let (lower, upper, lower_included, upper_included) = bounds;
    if let Some(lo) = lower
        && (v < lo || (!lower_included && (v - lo).abs() < f64::EPSILON))
    {
        return false;
    }
    if let Some(hi) = upper
        && (v > hi || (!upper_included && (v - hi).abs() < f64::EPSILON))
    {
        return false;
    }
    true
}

/// The `["1"] = <…> …` rows of a domain `list`, each an ordered
/// `(attribute, value)` vec. The corpus always uses a keyed list; a bare object
/// is treated as a single row.
fn domain_list_rows(list: &OdinValue) -> Vec<Vec<(String, OdinValue)>> {
    let row_of = |m: &indexmap::IndexMap<String, OdinValue>| -> Vec<(String, OdinValue)> {
        m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    };
    match untyped(list) {
        OdinValue::KeyedList(entries) => entries
            .iter()
            .filter_map(|(_, v)| match untyped(v) {
                OdinValue::Object(m) => Some(row_of(m)),
                _ => None,
            })
            .collect(),
        OdinValue::Object(m) => vec![row_of(m)],
        _ => Vec::new(),
    }
}

/// An ODIN leaf value → a `C_PRIMITIVE_OBJECT` for a domain attribute. The
/// attribute name disambiguates integer-vs-real intervals (`precision` is
/// integral, `magnitude` real).
fn domain_value_to_primitive(attr: &str, v: &OdinValue) -> Option<CPrimitiveObject> {
    match untyped(v) {
        OdinValue::String(s) => Some(CPrimitiveObject::CString(cstring_values(
            std::slice::from_ref(s),
        ))),
        OdinValue::Integer(i) => Some(CPrimitiveObject::CInteger(cinteger_values(vec![
            point_int(*i),
        ]))),
        OdinValue::Real(r) => Some(CPrimitiveObject::CReal(creal_values(vec![point_real(*r)]))),
        OdinValue::Interval(iv) => {
            if attr == "precision" {
                Some(CPrimitiveObject::CInteger(cinteger_values(vec![
                    odin_interval_to_int(iv),
                ])))
            } else {
                Some(CPrimitiveObject::CReal(creal_values(vec![
                    odin_interval_to_real(iv),
                ])))
            }
        }
        OdinValue::List(items) => {
            let mut merged: Vec<CPrimitiveObject> = Vec::new();
            for it in items {
                if let Some(p) = domain_value_to_primitive(attr, it) {
                    merged.push(p);
                }
            }
            merge_primitives(merged)
        }
        _ => None,
    }
}

fn any_primitive(attr: &str) -> CPrimitiveObject {
    if attr == "units" {
        CPrimitiveObject::CString(cstring_values(&[]))
    } else if attr == "precision" {
        CPrimitiveObject::CInteger(cinteger_values(Vec::new()))
    } else {
        CPrimitiveObject::CReal(creal_values(Vec::new()))
    }
}

/// Merge same-typed primitive constraints into a single object holding the
/// union of their value lists.
fn merge_primitives(mut items: Vec<CPrimitiveObject>) -> Option<CPrimitiveObject> {
    if items.is_empty() {
        return None;
    }
    if items.len() == 1 {
        return items.pop();
    }
    let mut strings: Vec<String> = Vec::new();
    let mut reals: Vec<Interval<f64>> = Vec::new();
    let mut ints: Vec<Interval<i32>> = Vec::new();
    let mut kind = 0u8;
    for it in items {
        match it {
            CPrimitiveObject::CString(c) => {
                kind = 1;
                strings.extend(c.constraint);
            }
            CPrimitiveObject::CReal(c) => {
                kind = 2;
                reals.extend(c.constraint);
            }
            CPrimitiveObject::CInteger(c) => {
                kind = 3;
                ints.extend(c.constraint);
            }
            other => return Some(other),
        }
    }
    Some(match kind {
        1 => CPrimitiveObject::CString(cstring_values(&strings)),
        2 => CPrimitiveObject::CReal(creal_values(reals)),
        _ => CPrimitiveObject::CInteger(cinteger_values(ints)),
    })
}

fn primitive_to_cobject(p: CPrimitiveObject) -> CObject {
    match p {
        CPrimitiveObject::CString(c) => CObject::CString(c),
        CPrimitiveObject::CReal(c) => CObject::CReal(c),
        CPrimitiveObject::CInteger(c) => CObject::CInteger(c),
        CPrimitiveObject::CBoolean(c) => CObject::CBoolean(c),
        CPrimitiveObject::CDate(c) => CObject::CDate(c),
        CPrimitiveObject::CDateTime(c) => CObject::CDateTime(c),
        CPrimitiveObject::CDuration(c) => CObject::CDuration(c),
        CPrimitiveObject::CTerminologyCode(c) => CObject::CTerminologyCode(c),
        CPrimitiveObject::CTime(c) => CObject::CTime(c),
    }
}

fn cattr_single(name: &str, child: CObject) -> CAttribute {
    CAttribute {
        parent: None,
        soc_parent: None,
        rm_attribute_name: name.to_owned(),
        existence: None,
        children: vec![child],
        differential_path: None,
        cardinality: None,
        is_multiple: false,
    }
}

fn cattr_empty(name: &str) -> CAttribute {
    CAttribute {
        parent: None,
        soc_parent: None,
        rm_attribute_name: name.to_owned(),
        existence: None,
        children: Vec::new(),
        differential_path: None,
        cardinality: None,
        is_multiple: false,
    }
}

fn cstring_values(values: &[String]) -> CString {
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
        assumed_value: None,
        is_enumerated_type_constraint: None,
        constraint: values.to_vec(),
    }
}

fn creal_values(constraint: Vec<Interval<f64>>) -> CReal {
    CReal {
        parent: None,
        soc_parent: None,
        rm_type_name: "Real".to_owned(),
        occurrences: None,
        node_id: "Primitive_node_id".to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        assumed_value: None,
        is_enumerated_type_constraint: None,
        constraint,
    }
}

fn cinteger_values(constraint: Vec<Interval<i32>>) -> CInteger {
    CInteger {
        parent: None,
        soc_parent: None,
        rm_type_name: "Integer".to_owned(),
        occurrences: None,
        node_id: "Primitive_node_id".to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        assumed_value: None,
        is_enumerated_type_constraint: None,
        constraint,
    }
}

fn point_real(v: f64) -> Interval<f64> {
    Interval::PointInterval(PointInterval {
        lower: Some(v),
        upper: Some(v),
        lower_unbounded: false,
        upper_unbounded: false,
        lower_included: true,
        upper_included: true,
    })
}

fn point_int(v: i64) -> Interval<i32> {
    // Domain-list integer constraints (precision, counts) are small clinical
    // values; saturate defensively into `i32` (AOM2 uses `Integer` = `i32`).
    let v = i32::try_from(v).unwrap_or(if v.is_negative() { i32::MIN } else { i32::MAX });
    Interval::PointInterval(PointInterval {
        lower: Some(v),
        upper: Some(v),
        lower_unbounded: false,
        upper_unbounded: false,
        lower_included: true,
        upper_included: true,
    })
}

fn odin_interval_to_real(iv: &openehr_lang::odin::OdinInterval) -> Interval<f64> {
    let (lower, li, upper, ui) = odin_range_bounds(iv, odin_as_real, |r| r);
    proper_or_point_real(lower, li, upper, ui)
}

fn odin_interval_to_int(iv: &openehr_lang::odin::OdinInterval) -> Interval<i32> {
    let (lower, li, upper, ui) =
        odin_range_bounds(iv, |v| odin_as_real(v).map(real_to_i32), real_to_i32);
    if lower == upper && lower.is_some() {
        return point_int(i64::from(lower.unwrap_or_default()));
    }
    Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
        lower,
        upper,
        lower_unbounded: lower.is_none(),
        upper_unbounded: upper.is_none(),
        lower_included: li,
        upper_included: ui,
    }))
}

fn proper_or_point_real(
    lower: Option<f64>,
    li: bool,
    upper: Option<f64>,
    ui: bool,
) -> Interval<f64> {
    if lower == upper && lower.is_some() {
        return point_real(lower.unwrap_or_default());
    }
    Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
        lower,
        upper,
        lower_unbounded: lower.is_none(),
        upper_unbounded: upper.is_none(),
        lower_included: li,
        upper_included: ui,
    }))
}

/// The `(lower, lower_included, upper, upper_included)` of an ODIN interval,
/// each endpoint converted with `conv` (a `None` endpoint stays unbounded).
///
/// The `|N +/- M|` form lowers to the closed interval `[N-M, N+M]`, per
/// `AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types —
/// "`|N +/-M|` -- interval of N ± M", whose worked example glosses
/// `|5.0 +/-0.5|` as "4.5 - 5.5" — and identically
/// `LANG/docs/odin/master07-leaf_data` §Intervals of Ordered Primitive Types.
/// The arithmetic is done in `f64` and mapped back with `from_real`, since the
/// AOM2 targets of this lowering (`C_REAL` / `C_INTEGER`) are numeric; a
/// non-numeric centre or half-width (a date ± duration, which cannot be
/// reduced without type context) yields an unbounded interval rather than a
/// fabricated endpoint.
fn odin_range_bounds<T>(
    iv: &openehr_lang::odin::OdinInterval,
    conv: impl Fn(&OdinValue) -> Option<T>,
    from_real: impl Fn(f64) -> T,
) -> (Option<T>, bool, Option<T>, bool) {
    match iv {
        openehr_lang::odin::OdinInterval::Range {
            lower,
            lower_included,
            upper,
            upper_included,
        } => (
            lower.as_deref().and_then(&conv),
            *lower_included,
            upper.as_deref().and_then(&conv),
            *upper_included,
        ),
        openehr_lang::odin::OdinInterval::PlusMinus { centre, delta } => {
            match (odin_as_real(centre), odin_as_real(delta)) {
                (Some(c), Some(d)) => (Some(from_real(c - d)), true, Some(from_real(c + d)), true),
                _ => (None, true, None, true),
            }
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "archetype domain-constraint magnitudes are small integers; f64 represents them exactly"
)]
fn odin_as_real(v: &OdinValue) -> Option<f64> {
    match v {
        OdinValue::Real(r) => Some(*r),
        OdinValue::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is clamped to the i32 range on the very next line, so the cast cannot truncate"
)]
fn real_to_i32(r: f64) -> i32 {
    r.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
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
        dialect: Dialect::Adl2,
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
    match p.rsplit_once('/') {
        Some((parent, name)) => (parent.to_owned(), name.to_owned()),
        None => (String::new(), p.to_owned()),
    }
}

/// The inner regex of a `/re/` or `^re^` delimited pattern.
fn regex_inner(delimited: &str) -> &str {
    let d = delimited.trim();
    for delimiter in ['/', '^'] {
        if let Some(inner) = d
            .strip_prefix(delimiter)
            .and_then(|rest| rest.strip_suffix(delimiter))
        {
            return inner;
        }
    }
    d
}

/// Backslash-escape every UNESCAPED `/` in a regex body.
///
/// `AOM2/master04.5` §`C_STRING` types `constraint` as "a list of literal
/// strings and / or regular expression strings **delimited by the '/'
/// character**", so the AOM carrier is always the `/…/` form — which makes the
/// `^…^` delimiter a purely lexical alternative that has to be normalised on
/// the way in. The chapter states the two forms' equivalence with its own
/// worked pair (`ADL1.4/master05-cadl.adoc` §Regular Expression L696-702:
/// "If the delimiter character is required in the pattern, it must be quoted
/// with the backslash ('\\') character, or else alternative delimiters can be
/// used … The following two patterns are equivalent: `{/km\\/h|mi\\/h/}` …
/// `{^km/h|mi/h^}`"), so escaping on normalisation is the spec's own mapping
/// and keeps parse → print → parse lossless. An already-escaped `\/` (a
/// slash-delimited source) is left alone, so the transform is idempotent.
fn escape_regex_delimiter(inner: &str) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => {
                escaped = true;
                out.push(ch);
            }
            '/' => out.push_str("\\/"),
            _ => out.push(ch),
        }
    }
    out
}

/// Whether a date/time pattern field is the "present" placeholder (e.g. `hh`)
/// or a literal date/time number substituted for it.
///
/// `ADL1.4/master05-cadl.adoc` §Patterns L894: "In the above patterns, the
/// 'yyyy' etc match strings can be replaced by literal date/time numbers. For
/// example, `yyyy-??-XX` could be transformed into `1995-??-XX`". A literal
/// field constrains the value to exactly that number, so it is "present" in
/// the same sense the placeholder is — which is what the degradation rules
/// (L860-861) range over.
fn is_present_field(f: &str, present: &str) -> bool {
    f.eq_ignore_ascii_case(present) || is_literal_field(f, 2)
}

/// Whether an ISO8601 time / date-time literal carries a timezone modifier
/// (`Z` or a `±hh[:mm]` offset). Only the part after the `T` is examined, so
/// the `-` separators of the date part are never mistaken for a sign
/// (`base_lexer.g4` `ISO8601_DATE_TIME` / `ISO8601_TIME`).
fn iso_has_timezone(v: &str) -> bool {
    let tail = v.split_once('T').map_or(v, |(_, t)| t);
    tail.ends_with('Z') || tail.contains('+') || tail.contains('-')
}

/// The year field: the `yyyy`/`yyy` placeholder or a literal 4-digit year
/// (`master05` §Patterns L894).
fn is_year_field(f: &str) -> bool {
    f.eq_ignore_ascii_case("yyyy") || f.eq_ignore_ascii_case("yyy") || is_literal_field(f, 4)
}

/// Whether `f` is exactly `width` ASCII digits — a literal-substituted field.
fn is_literal_field(f: &str, width: usize) -> bool {
    f.len() == width && f.bytes().all(|b| b.is_ascii_digit())
}

/// The time part of a pattern with its timezone modifier stripped.
///
/// The modifier is `Z` or a sign-led `hh`/`hh:mm`/`hhmm` group; the sign is
/// `+`, `-` or the literal `±` — `master05` §Patterns L852 ("the addition of a
/// patterns such as `+hh:mm`, `+hhmm`, and `-hh`") and the
/// `<<timezone_constraints>>` table L900-906, whose `±` rows are glossed
/// "commencing with '+' or '-'". A time never contains `+`/`-`/`Z` otherwise,
/// so the split is unambiguous.
fn pattern_time_core(t: &str) -> &str {
    t.split(['+', '-', '\u{00B1}', 'Z']).next().unwrap_or(t)
}

/// The strength-keyword set (`master04.5` §Constraint strengths).
fn is_strength_keyword(s: &str) -> bool {
    matches!(s, "required" | "extensible" | "preferred" | "example")
}

/// Map a constraint-strength keyword to its ordinal `CONSTRAINT_STATUS`
/// (`master09.05`: required(0) < extensible(1) < preferred(2) < example(3)).
fn strength_status(s: &str) -> ConstraintStatus {
    // `required` (and any unrecognised keyword) maps to the default status.
    match s {
        "extensible" => ConstraintStatus::Extensible,
        "preferred" => ConstraintStatus::Preferred,
        "example" => ConstraintStatus::Example,
        _ => ConstraintStatus::Required,
    }
}

/// Decode a double-quoted `master03` string literal (delimiters included).
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    decode_string_inner(inner)
}

/// Decode a single-quoted `CHARACTER` literal (delimiters included) into the
/// one-character string that carries it (`base_lexer.g4` `CHARACTER`).
fn decode_character(raw: &str) -> String {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
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

/// Whether an ODIN value tree contains an interval anywhere.
///
/// An interval has no canonical-JSON encoding here (see [`odin_to_json`]), so
/// a `_default` carrying one is refused outright rather than silently reduced
/// to `null` — the loss would turn "this default is an interval" into "this
/// node has a null default", which no reader can tell from a real absence.
fn odin_contains_interval(v: &OdinValue) -> bool {
    match v {
        OdinValue::Interval(_) => true,
        OdinValue::List(items) => items.iter().any(odin_contains_interval),
        OdinValue::Object(map) => map.values().any(odin_contains_interval),
        OdinValue::KeyedList(items) => items.iter().any(|(_, val)| odin_contains_interval(val)),
        OdinValue::Typed { value, .. } => odin_contains_interval(value),
        _ => false,
    }
}

/// Convert an [`openehr_lang::odin::OdinValue`] to canonical JSON for a
/// `C_DEFINED_OBJECT.default_value`.
///
/// NOTE: `AOM2/master04` types `C_DEFINED_OBJECT.default_value` as an instance
/// of the constrained RM type, and no openEHR spec mandates an intermediate
/// JSON shape for it — the canonical-JSON encoding used here is our own
/// design/extension. An `<>` / `<...>` empty block is a genuine "no value" and
/// maps to `null`.
///
// TODO: encode ODIN interval values (`|0..5|`) as a typed default instead of
// `null` — [`odin_contains_interval`] refuses them at the parse for now, so a
// `_default = <|0..5|>` is an error rather than a silent null.
fn odin_to_json(v: &OdinValue) -> serde_json::Value {
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
                assert_eq!(t.constraint_status, Some(ConstraintStatus::Preferred));
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

    /// `AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types
    /// defines `|N +/-M|` as "interval of N ± M" and glosses its own example
    /// `|5.0 +/-0.5|` as "4.5 - 5.5" — so an inline 1.4 domain block's
    /// `magnitude` lowers to the CLOSED interval `[N-M, N+M]`, not to the
    /// centre alone.
    #[test]
    fn adl14_plus_minus_domain_interval_lowers_to_both_bounds() {
        let cco = parse_definition_body_adl14(
            "OBSERVATION[at0000] matches {\n\
             value matches {\n\
             C_DV_QUANTITY <\n\
             list = <\n\
             [\"1\"] = <\n\
             magnitude = <|5.0 +/-0.5|>\n\
             >\n\
             >\n\
             >\n\
             }\n\
             }",
        )
        .expect("the 1.4 inline domain block must parse");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let CObject::CComplexObject(CComplexObject::CComplexObject(quantity)) =
            &d.attributes[0].children[0]
        else {
            panic!("expected the lowered DV_QUANTITY object");
        };
        let magnitude = quantity
            .attributes
            .iter()
            .find(|a| a.rm_attribute_name == "magnitude")
            .expect("magnitude attribute");
        let CObject::CReal(real) = &magnitude.children[0] else {
            panic!("expected a C_REAL magnitude constraint");
        };
        let [Interval::ProperInterval(ProperInterval::ProperInterval(range))] =
            real.constraint.as_slice()
        else {
            panic!("expected one proper interval, got {:?}", real.constraint);
        };
        assert_eq!(range.lower, Some(4.5));
        assert_eq!(range.upper, Some(5.5));
        assert!(range.lower_included);
        assert!(range.upper_included);
    }

    /// An interval has no ODIN/JSON default-value encoding
    /// (`ADL2/master06-default_values.adoc` §Default Values: a `_default` is an
    /// object INSTANCE, and an interval is a constraint), so a `_default`
    /// carrying one is refused rather than silently reduced to null. This is the
    /// ADL2 dialect's own behaviour — a 1.4 text has no `_default` at all
    /// (`ADL1.4/master05-cadl.adoc` §Keywords L48-53), and the 1.4 refusal of the
    /// construct is `tests/corpus/adl14-cadl/…SCOAT_adl2_default_value.v1.adl`.
    #[test]
    fn adl2_default_value_rejects_an_interval() {
        let errs = parse_definition_body(
            "OBSERVATION[id1] matches {\n\
             data matches {\n\
             HISTORY[id2] matches {\n\
             _default = (DV_QUANTITY) <\n\
             units = <\"mm[Hg]\">\n\
             magnitude = <|0.0..5.0|>\n\
             >\n\
             }\n\
             }\n\
             }",
        )
        .expect_err("an interval in a `_default` must be refused");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Sdinv),
            "expected SDINV, got {:?}",
            errs.iter().map(|e| e.code).collect::<Vec<_>>()
        );
    }

    /// Every ADL2-only cADL construct is refused in the 1.4 dialect with the
    /// typed code for its parse position, and is still accepted in ADL 2 — the
    /// gate must narrow the 1.4 dialect only (`ADL1.4/master05-cadl.adoc`
    /// §Keywords L48-53 is the closed 1.4 keyword set).
    #[test]
    fn adl2_only_constructs_are_refused_in_the_14_dialect_only() {
        let cases: &[(&str, SyntaxErrorCode)] = &[
            // `_default` (ADL2/master06-default_values.adoc).
            (
                "OBSERVATION[at0000] matches {\n\
                 data matches {\n\
                 HISTORY[at0001] matches {\n\
                 _default = (DV_QUANTITY) <units = <\"mm[Hg]\"> magnitude = <5.0>>\n\
                 }\n\
                 }\n\
                 }",
                SyntaxErrorCode::Scoat,
            ),
            // Second-order attribute tuple (ADL2/master04.4).
            (
                "OBSERVATION[at0000] matches {\n\
                 value matches {\n\
                 DV_QUANTITY matches {\n\
                 [magnitude, units] matches {\n\
                 [{|>=0.0|}, {\"mm[Hg]\"}]\n\
                 }\n\
                 }\n\
                 }\n\
                 }",
                SyntaxErrorCode::Scoat,
            ),
            // `use_archetype` (ADL2/master04.3 §Archetype Slots).
            (
                "SECTION[at0000] matches {\n\
                 items cardinality matches {0..*} matches {\n\
                 use_archetype CLUSTER[at0001, openEHR-EHR-CLUSTER.dimensions.v1]\n\
                 }\n\
                 }",
                SyntaxErrorCode::Sccog,
            ),
            // The slot `closed` marker (ADL2/master04.3 §Archetype Slots).
            (
                "SECTION[at0000] matches {\n\
                 items cardinality matches {0..*} matches {\n\
                 allow_archetype CLUSTER[at0001] closed\n\
                 }\n\
                 }",
                SyntaxErrorCode::Sccog,
            ),
            // Constraint strength (AOM2/master04.2 §Constraint Strengths).
            (
                "ELEMENT[at0000] matches {\n\
                 value matches {\n\
                 DV_CODED_TEXT matches {\n\
                 defining_code matches {preferred [ac0001]}\n\
                 }\n\
                 }\n\
                 }",
                SyntaxErrorCode::Stccp,
            ),
            // `@terminology` operational binding (ADL2/master08).
            (
                "ELEMENT[at0000] matches {\n\
                 value matches {\n\
                 DV_CODED_TEXT matches {\n\
                 defining_code matches {[ac0001@snomed]}\n\
                 }\n\
                 }\n\
                 }",
                SyntaxErrorCode::Stccp,
            ),
        ];
        for (src, code) in cases {
            let errs = parse_definition_body_adl14(src)
                .err()
                .unwrap_or_else(|| panic!("the 1.4 dialect must refuse:\n{src}"));
            assert!(
                errs.iter().any(|e| &e.code == code),
                "expected {code} for:\n{src}\ngot {:?}",
                errs.iter().map(|e| e.code).collect::<Vec<_>>()
            );
            assert!(
                parse_definition_body(src).is_ok(),
                "the ADL2 dialect must still accept:\n{src}"
            );
        }
    }

    /// `before`/`after` are cADL 1.4 keywords (`ADL1.4/master05-cadl.adoc`
    /// §Keywords L53), so a sibling order is legal in a 1.4 text and must not be
    /// gated with the ADL2-only constructs.
    #[test]
    fn adl14_accepts_sibling_order() {
        let cco = parse_definition_body_adl14(
            "CLUSTER[at0000] matches {\n\
             items cardinality matches {0..*} matches {\n\
             ELEMENT[at0001] matches {*}\n\
             before [at0001] ELEMENT[at0002] matches {*}\n\
             }\n\
             }",
        )
        .expect("sibling order is legal ADL 1.4 cADL");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let CObject::CComplexObject(CComplexObject::CComplexObject(second)) =
            &d.attributes[0].children[1]
        else {
            panic!("expected the re-ordered ELEMENT");
        };
        let order = second.sibling_order.as_ref().expect("a sibling order");
        assert!(order.is_before);
        assert_eq!(order.sibling_node_id, "at0001");
    }

    /// A domain block's `assumed_value` decomposes onto the leaf constraints the
    /// lowering produced: `AOM2/master04.2` §`Assumed_value` puts `assumed_value` on
    /// `C_PRIMITIVE_OBJECT` (never on a `C_COMPLEX_OBJECT`, and never on
    /// `default_value` — L175 separates the two notions), and
    /// `AOM2/master04.3` §Tuple Constraints makes a tuple ROW one co-constrained
    /// alternative, so the assumed instance binds to exactly the row it satisfies.
    #[test]
    fn adl14_domain_assumed_value_lands_on_the_matching_tuple_row() {
        let cco = parse_definition_body_adl14(
            "ELEMENT[at0000] matches {\n\
             value matches {\n\
             C_DV_QUANTITY <\n\
             assumed_value = <units = <\"C\"> precision = <0> magnitude = <8.0>>\n\
             list = <\n\
             [\"1\"] = <units = <\"C\"> magnitude = <|>=4.0|>>\n\
             [\"2\"] = <units = <\"F\"> magnitude = <|>=40.0|>>\n\
             >\n\
             >\n\
             }\n\
             }",
        )
        .expect("the 1.4 inline domain block must parse");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let CObject::CComplexObject(CComplexObject::CComplexObject(quantity)) =
            &d.attributes[0].children[0]
        else {
            panic!("expected the lowered DV_QUANTITY object");
        };
        let tuple = &quantity.attribute_tuples[0];
        assert_eq!(
            tuple
                .members
                .iter()
                .map(|m| m.rm_attribute_name.as_str())
                .collect::<Vec<_>>(),
            ["units", "magnitude"]
        );
        // Row 0 (`"C"`, >=4.0) admits the assumed combination; row 1 (`"F"`,
        // >=40.0) does not and must be left untouched.
        let CPrimitiveObject::CString(units0) = &tuple.tuples[0].members[0] else {
            panic!("units is a string constraint");
        };
        assert_eq!(units0.assumed_value.as_deref(), Some("C"));
        let CPrimitiveObject::CReal(magnitude0) = &tuple.tuples[0].members[1] else {
            panic!("magnitude is a real constraint");
        };
        assert_eq!(magnitude0.assumed_value, Some(8.0));
        let CPrimitiveObject::CString(units1) = &tuple.tuples[1].members[0] else {
            panic!("units is a string constraint");
        };
        assert_eq!(units1.assumed_value, None);
    }

    /// A single-attribute domain block merges its rows into one plain constraint,
    /// so the `assumed_value` lands directly on that leaf's
    /// `C_PRIMITIVE_OBJECT.assumed_value` (`AOM2/master04.2` §`Assumed_value`).
    #[test]
    fn adl14_domain_assumed_value_lands_on_a_plain_attribute() {
        let cco = parse_definition_body_adl14(
            "ELEMENT[at0000] matches {\n\
             value matches {\n\
             C_DV_QUANTITY <\n\
             assumed_value = <units = <\"F\">>\n\
             list = <\n\
             [\"1\"] = <units = <\"C\">>\n\
             [\"2\"] = <units = <\"F\">>\n\
             >\n\
             >\n\
             }\n\
             }",
        )
        .expect("the 1.4 inline domain block must parse");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let CObject::CComplexObject(CComplexObject::CComplexObject(quantity)) =
            &d.attributes[0].children[0]
        else {
            panic!("expected the lowered DV_QUANTITY object");
        };
        let units = quantity
            .attributes
            .iter()
            .find(|a| a.rm_attribute_name == "units")
            .expect("units attribute");
        let CObject::CString(c) = &units.children[0] else {
            panic!("expected a C_STRING units constraint");
        };
        assert_eq!(c.constraint, vec!["C".to_owned(), "F".to_owned()]);
        assert_eq!(c.assumed_value.as_deref(), Some("F"));
    }

    /// An assumed value satisfying no `list` row is refused: the 1.4 source states
    /// an assumed instance outside its own constraint, and no tuple row can carry
    /// it (`AOM2/master04.3` §Tuple Constraints).
    #[test]
    fn adl14_domain_assumed_value_outside_every_row_is_refused() {
        let errs = parse_definition_body_adl14(
            "ELEMENT[at0000] matches {\n\
             value matches {\n\
             C_DV_QUANTITY <\n\
             assumed_value = <units = <\"kPa\"> magnitude = <8.0>>\n\
             list = <\n\
             [\"1\"] = <units = <\"mm[Hg]\"> magnitude = <|>=0.0|>>\n\
             [\"2\"] = <units = <\"cm[H2O]\"> magnitude = <|>=0.0|>>\n\
             >\n\
             >\n\
             }\n\
             }",
        )
        .expect_err("an unmatched assumed value must be refused");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Sdinv),
            "expected SDINV, got {:?}",
            errs.iter().map(|e| e.code).collect::<Vec<_>>()
        );
    }

    /// An inline dADL domain block whose type is not one this lowering models is
    /// refused by NAME, never lowered to a different RM type
    /// (`ADL1.4/master09-customising_adl.adoc` §Introduction admits any
    /// `C_DOMAIN_TYPE` descendant, each targeting a different RM type).
    #[test]
    fn adl14_unsupported_domain_type_is_refused_by_name() {
        let errs = parse_definition_body_adl14(
            "ELEMENT[at0000] matches {\n\
             value matches {\n\
             DV_CODED_TEXT matches {\n\
             defining_code matches {\n\
             (C_CODE_PHRASE) <\n\
             terminology_id = <value = <\"local\">>\n\
             code_list = <[\"1\"] = <\"at0001\">>\n\
             >\n\
             }\n\
             }\n\
             }\n\
             }",
        )
        .expect_err("an unmodelled domain constrainer must be refused");
        let sdinv = errs
            .iter()
            .find(|e| e.code == SyntaxErrorCode::Sdinv)
            .expect("SDINV");
        assert!(
            sdinv.message.contains("C_CODE_PHRASE"),
            "the message must name the type, got {:?}",
            sdinv.message
        );
    }

    #[test]
    fn adl14_anonymous_archetype_slot() {
        // ADL1.4 master05-cadl.adoc §Archetype Slots writes the slot WITHOUT
        // a node id in its own normative examples ("allow_archetype
        // OBSERVATION occurrences ∈ {0..1} ∈ { include ... }"); §cADL node
        // types shows the identified form (`allow_archetype ENTRY[at2002]`).
        // Both must parse in the 1.4 dialect; ADL 2 keeps the bracket
        // mandatory (cadl2.g4).
        let cco = parse_definition_body_adl14(
            "SECTION[at0000] matches {\n\
             items cardinality matches {0..*; unordered} matches {\n\
             allow_archetype OBSERVATION occurrences matches {0..1} matches {\n\
             include\n\
             archetype_id/value matches {/openEHR-EHR-OBSERVATION\\.bp_measurement\\.v1/}\n\
             }\n\
             allow_archetype ENTRY[at2002] matches {\n\
             include\n\
             archetype_id/value matches {/.*/}\n\
             }\n\
             }\n\
             }",
        )
        .expect("the spec's own anonymous slot form must parse as ADL 1.4");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let items = &d.attributes[0];
        let CObject::ArchetypeSlot(anon) = &items.children[0] else {
            panic!("expected the anonymous slot");
        };
        assert_eq!(anon.rm_type_name, "OBSERVATION");
        assert!(anon.node_id.is_empty(), "anonymous slot has no node id");
        assert_eq!(anon.includes.len(), 1);
        let CObject::ArchetypeSlot(named) = &items.children[1] else {
            panic!("expected the identified slot");
        };
        assert_eq!(named.node_id, "at2002");

        // The bracket stays MANDATORY in ADL 2 (cadl2.g4 archetype_slot).
        assert!(
            parse_definition_body(
                "SECTION[id1] matches {\n\
                 items cardinality matches {0..*} matches {\n\
                 allow_archetype OBSERVATION occurrences matches {0..1}\n\
                 }\n\
                 }",
            )
            .is_err()
        );
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

    /// The `(lower, upper, lower_included, upper_included)` of the single
    /// interval an integer constraint carries; an unbounded endpoint is `None`.
    fn int_bounds(o: &CObject) -> (Option<f64>, Option<f64>, bool, bool) {
        match o {
            CObject::CInteger(ci) => interval_bounds_f64(&ci.constraint[0]),
            other => panic!("expected CInteger, got {other:?}"),
        }
    }

    /// `ADL1.4/master05-cadl.adoc` §Interval of Integer L771 —
    /// `rate matches {|0..infinity|}` — plus the `-infinity` and `*` endpoint
    /// spellings of `master04-dadl` §Intervals of Ordered Primitive Types
    /// L625-630.
    #[test]
    fn infinity_endpoints_are_unbounded() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {|0..infinity|}\n\
             b matches {|-infinity..5|}\n\
             c matches {|0..*|}\n\
             d matches {|0..10|}\n\
             }",
        );
        let d = data(&cco);
        assert_eq!(
            int_bounds(&d.attributes[0].children[0]),
            (Some(0.0), None, true, false)
        );
        assert_eq!(
            int_bounds(&d.attributes[1].children[0]),
            (None, Some(5.0), false, true)
        );
        assert_eq!(
            int_bounds(&d.attributes[2].children[0]),
            (Some(0.0), None, true, false)
        );
        assert_eq!(
            int_bounds(&d.attributes[3].children[0]),
            (Some(0.0), Some(10.0), true, true)
        );
    }

    /// Both spellings of the exclusive lower bound denote the same interval:
    /// the prefix `>` of `odin_values.g4` and the postfix `>` of
    /// `ADL1.4/master04-dadl.adoc` §Intervals L611-614 (used by `master05` L769
    /// as `length matches {|0>..<1000|}`).
    #[test]
    fn both_exclusive_lower_spellings_agree() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {|>0..<1000|}\n\
             b matches {|0>..<1000|}\n\
             c matches {|0>..1000|}\n\
             }",
        );
        let d = data(&cco);
        assert_eq!(
            int_bounds(&d.attributes[0].children[0]),
            (Some(0.0), Some(1000.0), false, false)
        );
        assert_eq!(
            int_bounds(&d.attributes[1].children[0]),
            (Some(0.0), Some(1000.0), false, false)
        );
        assert_eq!(
            int_bounds(&d.attributes[2].children[0]),
            (Some(0.0), Some(1000.0), false, true)
        );
    }

    /// `ADL1.4/master05-cadl.adoc` §Regular Expression L696-702: the `^…^` and
    /// the backslash-escaped `/…/` spellings "are equivalent", so the caret form
    /// normalises onto the AOM's `/`-delimited carrier WITH the inner delimiters
    /// escaped — and the result re-parses to itself (parse → print → parse is
    /// lossless; the printer emits `C_STRING.constraint` verbatim).
    #[test]
    fn caret_regex_normalises_losslessly() {
        let cco = parse("WHOLE[id1] matches {\n u matches {^km/h|mi/h^}\n}");
        let printed = match &data(&cco).attributes[0].children[0] {
            CObject::CString(cs) => cs.constraint[0].clone(),
            other => panic!("expected CString regex, got {other:?}"),
        };
        assert_eq!(printed, r"/km\/h|mi\/h/");
        // The chapter's own equivalent slash spelling yields the same carrier…
        let slash = parse(
            r"WHOLE[id1] matches {\n u matches {/km\/h|mi\/h/}\n}"
                .replace("\\n", "\n")
                .as_str(),
        );
        match &data(&slash).attributes[0].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint[0], printed),
            other => panic!("expected CString regex, got {other:?}"),
        }
        // …and re-parsing the printed form reproduces it unchanged.
        let again = parse(&format!(
            "WHOLE[id1] matches {{\n u matches {{{printed}}}\n}}"
        ));
        match &data(&again).attributes[0].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint[0], printed),
            other => panic!("expected CString regex, got {other:?}"),
        }
    }

    /// `ADL1.4/master05-cadl.adoc` §Constraints on Character L813-825 —
    /// `color_name matches {'r', 'g', 'b'}`. No AOM generation defines a
    /// `C_CHARACTER`, so the carrier is `C_STRING` with the constrained RM type
    /// name kept (see [`Parser::parse_c_character`]).
    #[test]
    fn character_lists_land_on_c_string() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {'r'}\n\
             b matches {'r', 'g', 'b'}\n\
             c matches {'r', 'g'; 'r'}\n\
             }",
        );
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CString(cs) => {
                assert_eq!(cs.rm_type_name, "Character");
                assert_eq!(cs.constraint, vec!["r".to_owned()]);
            }
            other => panic!("expected CString, got {other:?}"),
        }
        match &d.attributes[1].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint.len(), 3),
            other => panic!("expected CString, got {other:?}"),
        }
        match &d.attributes[2].children[0] {
            CObject::CString(cs) => assert_eq!(cs.assumed_value.as_deref(), Some("r")),
            other => panic!("expected CString, got {other:?}"),
        }
    }

    /// The `, ...` list-continuation marker (`ADL1.4/master05-cadl.adoc` §Syntax
    /// L1244-1249 for strings, `master04-dadl` §Syntax L985-1160 for every other
    /// primitive list) is a list INDICATOR, not a member and not an openness
    /// flag — see [`Parser::eat_list_continue`].
    #[test]
    fn list_continuation_adds_no_member() {
        let cco = parse(
            "WHOLE[id1] matches {\n\
             a matches {\"en\", ...}\n\
             b matches {1, 2, ...}\n\
             c matches {True, ...}\n\
             }",
        );
        let d = data(&cco);
        match &d.attributes[0].children[0] {
            CObject::CString(cs) => assert_eq!(cs.constraint, vec!["en".to_owned()]),
            other => panic!("expected CString, got {other:?}"),
        }
        match &d.attributes[1].children[0] {
            CObject::CInteger(ci) => assert_eq!(ci.constraint.len(), 2),
            other => panic!("expected CInteger, got {other:?}"),
        }
        match &d.attributes[2].children[0] {
            CObject::CBoolean(cb) => assert_eq!(cb.constraint, vec![true]),
            other => panic!("expected CBoolean, got {other:?}"),
        }
    }

    /// `ADL1.4/master05-cadl.adoc` §Intervals L932: a two-sided date/time
    /// interval must carry a timezone on both endpoints or on neither; a
    /// one-sided interval is outside the rule by its own wording.
    #[test]
    fn interval_timezone_symmetry_is_enforced() {
        let asymmetric = parse_definition_body(
            "WHOLE[id1] matches {\n d matches {|2004-05-20T00:00:00Z..2005-05-19T23:59:59|}\n}",
        )
        .expect_err("an asymmetric-timezone interval must be refused");
        assert!(
            asymmetric.iter().any(|e| e.code == SyntaxErrorCode::Scdtav),
            "expected SCDTAV, got {:?}",
            asymmetric.iter().map(|e| e.code).collect::<Vec<_>>()
        );
        let time_asymmetric = parse_definition_body(
            "WHOLE[id1] matches {\n t matches {|09:30:00+0200..10:30:00|}\n}",
        )
        .expect_err("an asymmetric-timezone time interval must be refused");
        assert!(
            time_asymmetric
                .iter()
                .any(|e| e.code == SyntaxErrorCode::Sctav),
            "expected SCTAV, got {:?}",
            time_asymmetric.iter().map(|e| e.code).collect::<Vec<_>>()
        );
        // Both endpoints with a timezone, both without, and the one-sided form
        // all parse.
        parse(
            "WHOLE[id1] matches {\n\
             a matches {|2004-05-20T00:00:00Z..2005-05-19T23:59:59Z|}\n\
             b matches {|2004-05-20T00:00:00..2005-05-19T23:59:59|}\n\
             c matches {|>09:30:00+0200|}\n\
             }",
        );
    }

    /// The negated-matches family and the `=~`/`!~` regex-match operators are
    /// named by `ADL1.4/master05-cadl.adoc` (§Keywords L47/L95-98, §Regular
    /// Expression L691-693) but defined by no grammar production, so each is a
    /// typed refusal rather than a silent affirmative reading.
    #[test]
    fn operators_without_a_production_are_refused() {
        for (src, code) in [
            (
                "WHOLE[id1] matches {\n v matches {\n TEXT ~matches {\"a\"}\n }\n}",
                SyntaxErrorCode::Sccog,
            ),
            (
                "WHOLE[id1] matches {\n v ~is_in {\n TEXT matches {*}\n }\n}",
                SyntaxErrorCode::Scoat,
            ),
            (
                "WHOLE[id1] matches {\n v \u{2209} {\n TEXT matches {*}\n }\n}",
                SyntaxErrorCode::Scoat,
            ),
            (
                "WHOLE[id1] matches {\n v matches {=~ /[a-z]+/}\n}",
                SyntaxErrorCode::Sccog,
            ),
            (
                "WHOLE[id1] matches {\n v matches {!~ /[a-z]+/}\n}",
                SyntaxErrorCode::Sccog,
            ),
        ] {
            let errs = parse_definition_body(src)
                .expect_err("an operator with no cADL production must be refused");
            assert!(
                errs.iter().any(|e| e.code == code),
                "expected {code} for {src:?}, got {:?}",
                errs.iter().map(|e| e.code).collect::<Vec<_>>()
            );
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
