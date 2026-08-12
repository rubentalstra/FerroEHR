// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The cADL definition-section parser.
//!
//! A hand-written recursive-descent parser over the shared openEHR token
//! stream under its cADL reading ([`openehr_lang::v1_1::lexer::lex_adl`]),
//! transcribed 1:1 from the vendored normative grammars
//! `crates/openehr-adl/vendor/grammar/v2_4/{cadl2.g4, cadl2_primitives.g4}`. It
//! builds the **generated** AOM2 constraint model
//! (`openehr_am::v2_4::aom2::constraint_model`) directly — never a new model
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
//! at position. Slot include/exclude **assertion** expressions are BEL
//! expression trees built by [`crate::rules::parse_slot_assertions`] (as is
//! the `rules` section); the common `archetype_id/value matches {/regex/}`
//! form is additionally regex-compile checked (`SCSRE`). Semantic (V-code)
//! validation is separate (`crate::validate`).
//!
//! Layout. This module carries what every production shares: the dialect
//! selector, the parser state, the entry points, and the cursor/error
//! helpers. [`parser`] holds the structure / attribute / tuple productions,
//! [`refs`] the archetype-slot / archetype-root / internal-reference
//! productions, [`primitives`] the inline `C_PRIMITIVE` family, [`values`]
//! the value-list / interval / endpoint machinery, and [`patterns`] the
//! date/time constraint-pattern validators. The ADL 1.4-only productions live
//! in [`crate::adl14::lower`] (with the inline dADL domain lowering they drive
//! in [`crate::adl14::domain`]); the dialect-gated dispatch points in
//! `Parser::parse_type_object`, `Parser::parse_c_objects_body` and
//! `Parser::parse_c_regular_object` are the only coupling to them.

pub mod parser;
pub mod patterns;
pub mod primitives;
pub mod refs;
pub mod values;

use openehr_am::v2_4::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::v2_4::aom2::constraint_model::c_object::CObject;
use openehr_am::v2_4::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::v2_4::aom2::constraint_model::primitive::c_string::CString;

use crate::aom::build::{cobject_to_primitive, cstring_regex};
use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::odin::regex_inner;
use openehr_lang::v1_1::lexer::{Spanned, Token};

/// Internal parse result: `Err(())` signals a bail-out; the concrete
/// [`SyntaxError`] is already recorded in [`Parser::errors`].
pub(crate) type PResult<T> = Result<T, ()>;

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

/// Parse a raw cADL `definition`-section body (the text between the
/// `definition` keyword and the next section) into a [`CComplexObject`], using
/// the grammar of `dialect`.
///
/// This is the core entry point: it lexes `body` and runs the cADL grammar.
/// Error byte spans are relative to `body`.
///
/// Under [`Dialect::Adl14`] the 1.4-only terminology-constraint and inline-dADL
/// domain forms are accepted and kept in a converter-internal encoding
/// (qualified codes and lists in the `C_TERMINOLOGY_CODE.constraint` string;
/// domain blocks lowered to a `DV_QUANTITY`/`DV_ORDINAL` with a `property`
/// at-code + a tuple/attribute set) that `crate::adl14::convert` rewrites into
/// spec-valid ADL2. See the `crate::adl14` module flag: no openEHR spec governs
/// that conversion.
///
/// # Errors
/// Returns every [`SyntaxError`] found (the `S*` catalogue codes of
/// `ADL2/master04.6`). Lexer failures surface as [`SyntaxErrorCode::Sunk`], a
/// malformed inline dADL domain block as [`SyntaxErrorCode::Sdinv`].
pub fn parse_definition_body(
    body: &str,
    dialect: Dialect,
) -> Result<CComplexObject, Vec<SyntaxError>> {
    let toks = match openehr_lang::v1_1::lexer::lex_adl(body) {
        Ok(t) => t,
        Err(failure) => return Err(vec![crate::error::lexical(&failure, body)]),
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
pub(crate) struct Parser<'a> {
    pub(crate) src: &'a str,
    pub(crate) toks: &'a [Spanned],
    pub(crate) pos: usize,
    pub(crate) errors: Vec<SyntaxError>,
    pub(crate) dialect: Dialect,
}

// ── cursor + error helpers ────────────────────────────────────────────────
impl Parser<'_> {
    pub(crate) fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos).map(|s| &s.token)
    }

    pub(crate) fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.toks.get(self.pos + ahead).map(|s| &s.token)
    }

    pub(crate) fn span_at(&self, idx: usize) -> std::ops::Range<usize> {
        self.toks
            .get(idx)
            .map_or(self.src.len()..self.src.len(), |s| s.span.clone())
    }

    pub(crate) fn cur_span(&self) -> std::ops::Range<usize> {
        self.span_at(self.pos)
    }

    pub(crate) fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    pub(crate) fn push(
        &mut self,
        code: SyntaxErrorCode,
        msg: impl Into<String>,
        span: std::ops::Range<usize>,
    ) {
        self.errors.push(SyntaxError::at(code, msg, span, self.src));
    }

    /// Record an error at the current position and bail.
    pub(crate) fn err<T>(&mut self, code: SyntaxErrorCode, msg: impl Into<String>) -> PResult<T> {
        let span = self.cur_span();
        self.push(code, msg, span);
        Err(())
    }

    /// Take a decoded string/character literal, or report its escape defect at
    /// `span` and bail.
    ///
    /// NOTE: the `S*` code space is a verbatim 1:1 mirror of the openEHR
    /// catalogue (`ADL2/master04.6-cadl_validity_rules.adoc` §Syntax Validity
    /// Rules) and carries no code for a lexical defect INSIDE a literal, so
    /// this reuses `SUNK` ("Syntax error (unknown cause)") — the same bucket
    /// every other lexical failure reports under
    /// ([`openehr_lang::v1_1::lexer::lex_adl`]) — and
    /// names the defect in the message. Inventing a code would break the 1:1
    /// mirror.
    pub(crate) fn decoded_literal(
        &mut self,
        decoded: Result<String, openehr_lang::v1_1::escape::EscapeError>,
        span: std::ops::Range<usize>,
    ) -> PResult<String> {
        match decoded {
            Ok(text) => Ok(text),
            Err(defect) => {
                self.push(SyntaxErrorCode::Sunk, defect.to_string(), span);
                Err(())
            }
        }
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
    pub(crate) fn adl2_only<T>(&mut self, code: SyntaxErrorCode, construct: &str) -> PResult<T> {
        self.err(
            code,
            format!("{construct} is an ADL 2 construct and is not valid in ADL 1.4"),
        )
    }

    /// True if the cursor is at a NEGATED matches operator (`~matches`,
    /// `~is_in`, `∉`) — lexically `SymNot SymMatches` or the single `∉`.
    pub(crate) fn at_negated_matches(&self) -> bool {
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
    pub(crate) fn at_regex_match_operator(&self) -> bool {
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
    /// rules section, which this crate parses through `openehr_lang::v1_1::bel`. So
    /// the negated operator is refused HERE, in cADL constraint position, and
    /// accepted there — never silently read as an affirmative `matches`, which
    /// would invert the constraint.
    pub(crate) fn negated_matches_reject<T>(&mut self, code: SyntaxErrorCode) -> PResult<T> {
        self.err(
            code,
            "a negated matches operator ('~matches' / '~is_in' / '∉') is not a cADL production; \
             negation is available as the prefix 'not' operator of a slot include/exclude assertion",
        )
    }

    /// Consume a token matching `pred` or record `code`/`msg` and bail.
    pub(crate) fn expect(
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

    pub(crate) fn eat(&mut self, pred: impl Fn(&Token) -> bool) -> bool {
        if self.peek().is_some_and(&pred) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

// ── re-entrant sub-parsers (driven by `crate::rules`) ─────────────────────

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
    let toks = openehr_lang::v1_1::lexer::lex_adl(raw)
        .map_err(|failure| vec![crate::error::lexical(&failure, raw)])?;
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
        Ok(obj) if parser.errors.is_empty() => cobject_to_primitive(&obj).ok_or_else(|| {
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
    let quoted_assumed = match body.split_once(';') {
        Some((r, a)) => (
            r.trim(),
            a.trim().strip_prefix('"').and_then(|s| s.strip_suffix('"')),
        ),
        None => (body, None),
    };
    let (regex_part, quoted_assumed) = quoted_assumed;
    // The regex body itself is NEVER escape-decoded (`master03` §Special
    // Character Sequences, final paragraph); only the `;"assumed"` suffix is.
    let assumed = match quoted_assumed {
        Some(text) => Some(openehr_lang::v1_1::escape::decode(text).map_err(|defect| {
            vec![SyntaxError::at(
                SyntaxErrorCode::Sunk,
                defect.to_string(),
                0..raw.len(),
                raw,
            )]
        })?),
        None => None,
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

#[cfg(test)]
mod tests {
    use openehr_am::v2_4::aom2::constraint_model::c_attribute::CAttribute;
    use openehr_am::v2_4::aom2::constraint_model::c_complex_object::{
        CComplexObject, CComplexObjectData,
    };
    use openehr_am::v2_4::aom2::constraint_model::c_object::CObject;
    use openehr_am::v2_4::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
    use openehr_base::prelude::{Interval, ProperInterval};

    use crate::error::SyntaxErrorCode;
    use crate::parse::{Dialect, parse_definition_body};

    fn data(cco: &CComplexObject) -> &CComplexObjectData {
        match cco {
            CComplexObject::CComplexObject(d) => d,
            CComplexObject::CArchetypeRoot(_) => panic!("expected plain complex object"),
        }
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
            let errs = parse_definition_body(src, Dialect::Adl14)
                .err()
                .unwrap_or_else(|| panic!("the 1.4 dialect must refuse:\n{src}"));
            assert!(
                errs.iter().any(|e| &e.code == code),
                "expected {code} for:\n{src}\ngot {:?}",
                errs.iter().map(|e| e.code).collect::<Vec<_>>()
            );
            assert!(
                parse_definition_body(src, Dialect::Adl2).is_ok(),
                "the ADL2 dialect must still accept:\n{src}"
            );
        }
    }

    /// `before`/`after` are cADL 1.4 keywords (`ADL1.4/master05-cadl.adoc`
    /// §Keywords L53), so a sibling order is legal in a 1.4 text and must not be
    /// gated with the ADL2-only constructs.
    #[test]
    fn adl14_accepts_sibling_order() {
        let cco = parse_definition_body(
            "CLUSTER[at0000] matches {\n\
             items cardinality matches {0..*} matches {\n\
             ELEMENT[at0001] matches {*}\n\
             before [at0001] ELEMENT[at0002] matches {*}\n\
             }\n\
             }",
            Dialect::Adl14,
        )
        .expect("sibling order is legal ADL 1.4 cADL");
        let CComplexObject::CComplexObject(d) = &cco else {
            panic!("expected a plain complex object root");
        };
        let CObject::CComplexObject(CComplexObject::CComplexObject(second)) =
            &d.attributes.as_deref().unwrap_or_default()[0]
                .children
                .as_deref()
                .unwrap_or_default()[1]
        else {
            panic!("expected the re-ordered ELEMENT");
        };
        let order = second.sibling_order.as_ref().expect("a sibling order");
        assert!(order.is_before);
        assert_eq!(order.sibling_node_id, "at0001");
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
            let errs = parse_definition_body(src, Dialect::Adl2)
                .expect_err("an operator with no cADL production must be refused");
            assert!(
                errs.iter().any(|e| e.code == code),
                "expected {code} for {src:?}, got {:?}",
                errs.iter().map(|e| e.code).collect::<Vec<_>>()
            );
        }
    }

    // ── structural spot-checks against real vendored corpus files ──────────
    // These parse whole `.adls` sources (outer parse + the definition span,
    // exactly what every caller of the entry points above does) and assert the
    // deep AOM2 tree against expectations hand-derived by reading each file.

    /// The root artefact's `definition` section of a whole ADL2 source: the
    /// outer parse of [`crate::source::parse_source`] plus
    /// [`parse_definition_body`] over the definition span.
    fn parse_source_definition(src: &str) -> CComplexObject {
        let artefact = crate::source::parse_source(src, Dialect::Adl2)
            .unwrap_or_else(|e| panic!("outer parse: {e:?}"));
        let def = artefact
            .definition
            .as_ref()
            .unwrap_or_else(|| panic!("no definition section"));
        let body = src.get(def.bytes.clone()).unwrap_or_default();
        parse_definition_body(body, Dialect::Adl2).unwrap_or_else(|e| panic!("parse: {e:?}"))
    }

    /// Fetch the named attribute of a complex object.
    fn attr<'a>(d: &'a CComplexObjectData, name: &str) -> &'a CAttribute {
        d.attributes
            .iter()
            .flatten()
            .find(|a| a.rm_attribute_name == name)
            .unwrap_or_else(|| panic!("no attribute {name:?}"))
    }

    /// The first child of an attribute, as a plain complex object.
    fn first_cco(a: &CAttribute) -> &CComplexObjectData {
        match &a.children.as_deref().unwrap_or_default()[0] {
            CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d,
            other => panic!("expected complex object child, got {other:?}"),
        }
    }

    #[test]
    fn corpus_ordinal_tuple_structure() {
        let src = include_str!(
            "../../tests/corpus/adl2-reference/features/aom_structures/tuples/openEHR-EHR-OBSERVATION.ordinal_tuple.v1.0.0.adls"
        );
        let cco = parse_source_definition(src);
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
        assert_eq!(ordinal.attribute_tuples.as_ref().map_or(0, Vec::len), 1);
        let tuple = &ordinal.attribute_tuples.as_deref().unwrap_or_default()[0];
        assert_eq!(tuple.members.as_ref().map_or(0, Vec::len), 2);
        assert_eq!(
            tuple.members.as_deref().unwrap_or_default()[0].rm_attribute_name,
            "value"
        );
        assert_eq!(
            tuple.members.as_deref().unwrap_or_default()[1].rm_attribute_name,
            "symbol"
        );
        assert_eq!(tuple.tuples.as_ref().map_or(0, Vec::len), 3);
        match &tuple.tuples.as_deref().unwrap_or_default()[0].members[0] {
            CPrimitiveObject::CInteger(ci) => {
                match &ci.constraint.as_deref().unwrap_or_default()[0] {
                    Interval::PointInterval(p) => assert_eq!(p.lower, Some(0)),
                    Interval::ProperInterval(_) => panic!("expected point 0"),
                }
            }
            other => panic!("expected CInteger, got {other:?}"),
        }
        match &tuple.tuples.as_deref().unwrap_or_default()[0].members[1] {
            CPrimitiveObject::CTerminologyCode(t) => assert_eq!(t.constraint, "at11"),
            other => panic!("expected CTerminologyCode, got {other:?}"),
        }
    }

    #[test]
    fn corpus_slot_structure() {
        let src = include_str!(
            "../../tests/corpus/adl2-reference/validity/slots/openEHR-EHR-SECTION.slot_parent.v1.0.0.adls"
        );
        let cco = parse_source_definition(src);
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

        match &items.children.as_deref().unwrap_or_default()[0] {
            CObject::ArchetypeSlot(s) => {
                assert_eq!(s.rm_type_name, "OBSERVATION");
                assert_eq!(s.node_id, "id2");
                let occ = s.occurrences.as_ref().expect("slot occurrences");
                assert_eq!(occ.lower, Some(0));
                assert_eq!(occ.upper, Some(1));
                assert_eq!(s.includes.as_ref().map_or(0, Vec::len), 1);
                assert!(
                    s.includes.as_deref().unwrap_or_default()[0]
                        .string_expression
                        .as_deref()
                        .unwrap_or_default()
                        .contains("archetype_id/value")
                );
                assert_eq!(s.excludes.as_ref().map_or(0, Vec::len), 1);
                assert!(!s.is_closed);
            }
            other => panic!("expected ArchetypeSlot, got {other:?}"),
        }
    }

    #[test]
    fn corpus_primitive_types_structure() {
        let src = include_str!(
            "../../tests/corpus/adl2-reference/features/aom_structures/primitive_types/openehr-TEST_PKG-WHOLE.primitive_types.v1.0.0.adls"
        );
        let cco = parse_source_definition(src);
        let root = data(&cco);
        assert_eq!(root.node_id, "id1");
        // integer_attr3 == {|0..100|}.
        match &attr(root, "integer_attr3")
            .children
            .as_deref()
            .unwrap_or_default()[0]
        {
            CObject::CInteger(ci) => match &ci.constraint.as_deref().unwrap_or_default()[0] {
                Interval::ProperInterval(ProperInterval::ProperInterval(pi)) => {
                    assert_eq!(pi.lower, Some(0));
                    assert_eq!(pi.upper, Some(100));
                }
                _ => panic!("expected proper interval"),
            },
            _ => panic!("expected CInteger"),
        }
        // date_attr3 == {yyyy-mm-??} (a pattern).
        match &attr(root, "date_attr3")
            .children
            .as_deref()
            .unwrap_or_default()[0]
        {
            CObject::CDate(c) => assert_eq!(c.pattern_constraint.as_deref(), Some("yyyy-mm-??")),
            _ => panic!("expected CDate pattern"),
        }
        // duration_attr22 == {PWD/PT0S} (pattern + value).
        match &attr(root, "duration_attr22")
            .children
            .as_deref()
            .unwrap_or_default()[0]
        {
            CObject::CDuration(c) => {
                assert_eq!(c.pattern_constraint.as_deref(), Some("PWD"));
                assert_eq!(c.constraint.as_ref().map_or(0, Vec::len), 1);
            }
            _ => panic!("expected CDuration pattern+value"),
        }
    }
}
