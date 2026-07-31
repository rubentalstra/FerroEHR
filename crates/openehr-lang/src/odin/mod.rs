//! ODIN — Object Data Instance Notation: a hand-written value-tree + parser.
//!
//! ODIN is the openEHR leaf-data notation used by BMM schema files and by the
//! ADL `language`/`description`/`terminology`/`annotations` sections. This
//! module is a self-contained reader — [`parse`] takes ODIN source text and
//! returns an [`OdinValue`] tree (insertion order preserved).
//!
//! Spec oracle: `docs/specs/openehr/LANG/docs/odin/` and the vendored grammars
//! `crates/openehr-lang/vendor/grammar/{odin.g4,odin_values.g4}` (which import
//! `base_lexer.g4`). This module is **deliberately off the codegen path** — it
//! parses ODIN *instances*, it does not load the BMM meta-model (the codegen
//! input is the JSON BMM serialization under `openehr-codegen/vendor/bmm/`).

mod lexer;
mod parser;

use indexmap::IndexMap;

use crate::position::line_col;

/// A parsed ODIN value.
///
/// The tree mirrors `odin.g4`: an object (`attr = <…>` pairs), an
/// insertion-ordered keyed list (`["k"] = <…>`), a primitive value or list, an
/// interval, an embedded URI, an object-reference path list, or an empty block.
#[derive(Debug, Clone, PartialEq)]
pub enum OdinValue {
    /// `attr_vals` — an object with attribute → value members (insertion
    /// order preserved). Sibling names are unique by construction: a repeat
    /// fails the parse with [`OdinErrorKind::DuplicateAttribute`] (rule
    /// *VDATU*), it is never overwritten.
    Object(IndexMap<String, OdinValue>),
    /// `keyed_object*` — an insertion-ordered keyed list (`["k"] = <…>`).
    KeyedList(Vec<(OdinKey, OdinValue)>),
    /// A primitive list (`v1, v2, …`); a single value is *not* wrapped in a
    /// list. A trailing open marker (`, ...`) appears as a final
    /// [`OdinValue::ListContinue`] element.
    List(Vec<OdinValue>),
    /// A typed cast `(TYPE) <…>` around an inner value block.
    Typed {
        /// The `rm_type_id` (with any generic parameters, e.g.
        /// `Interval<Quantity>`).
        rm_type: String,
        /// The cast value.
        value: Box<OdinValue>,
    },
    /// An object-reference block `< /path, /path >` (`odin_path_list`).
    PathList(Vec<String>),
    /// An empty block `<>`.
    Empty,
    /// A `STRING` value (escapes decoded per `master03`).
    String(String),
    /// An `INTEGER` value.
    Integer(i64),
    /// A `REAL` value.
    Real(f64),
    /// A `Boolean` value (`True`/`False`).
    Boolean(bool),
    /// A `CHARACTER` value.
    Character(char),
    /// An `ISO8601_DATE` value (verbatim, incl. any `??` partials).
    Date(String),
    /// An `ISO8601_TIME` value (verbatim).
    Time(String),
    /// An `ISO8601_DATE_TIME` value (verbatim).
    DateTime(String),
    /// An `ISO8601_DURATION` value (verbatim).
    Duration(String),
    /// An interval value (`| … |`).
    Interval(OdinInterval),
    /// A `TERM_CODE_REF` value (`[terminology::code]`), verbatim incl.
    /// brackets.
    TermCode(String),
    /// An embedded URI value (verbatim incl. the `<>` delimiters).
    Uri(String),
    /// A single object-reference path (`odin_path`).
    Path(String),
    /// The list-continuation marker (`...`), the final element of an open list.
    ListContinue,
}

/// A keyed-list key (`key_id` in `odin.g4`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OdinKey {
    /// A string key (`["en"]`).
    String(String),
    /// An integer key (`[1]`).
    Integer(i64),
    /// An ISO8601 date key.
    Date(String),
    /// An ISO8601 time key.
    Time(String),
    /// An ISO8601 date/time key.
    DateTime(String),
}

/// An ODIN interval value (`odin_values.g4` `*_interval_value`).
#[derive(Debug, Clone, PartialEq)]
pub enum OdinInterval {
    /// A range `| >? lower .. <? upper |`, or a single-bounded `| relop v |`.
    /// A `None` endpoint is unbounded on that side — either because the form
    /// is one-sided, or because the endpoint was written as one of the
    /// `infinity` / `-infinity` / `*` markers of
    /// `AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types.
    Range {
        /// Lower bound (`None` = unbounded below).
        lower: Option<Box<OdinValue>>,
        /// Whether the lower bound is inclusive.
        lower_included: bool,
        /// Upper bound (`None` = unbounded above).
        upper: Option<Box<OdinValue>>,
        /// Whether the upper bound is inclusive.
        upper_included: bool,
    },
    /// A `| centre +/- delta |` interval (endpoints not pre-computed, since a
    /// date `±` duration cannot be reduced without type context).
    PlusMinus {
        /// The centre value.
        centre: Box<OdinValue>,
        /// The half-width.
        delta: Box<OdinValue>,
    },
}

/// What kind of ODIN failure occurred — the discriminant a caller branches on
/// (the `message` is display text, never a decision input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OdinErrorKind {
    /// An unrecognised character or an illegal string escape.
    UnrecognisedToken,
    /// A syntactically unexpected token.
    UnexpectedToken,
    /// Two sibling attributes of one object node share a name — rule *VDATU*
    /// of `LANG/docs/odin/master05-content` §General Structure ("sibling
    /// attributes occurring within an object node must be uniquely named with
    /// respect to each other"), the principle "Sibling attribute names must be
    /// unique" of `AM/docs/ADL1.4/master04-dadl` §General Form. Carries the
    /// repeated name.
    DuplicateAttribute(String),
}

/// An ODIN parse or lex failure, with a 1-based line/column and a byte span.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ODIN syntax error at line {line}, column {column}: {message}")]
pub struct OdinError {
    /// The failure kind.
    pub kind: OdinErrorKind,
    /// A human-readable message.
    pub message: String,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// Byte range of the offending input.
    pub span: std::ops::Range<usize>,
}

/// Parse ODIN source text into an [`OdinValue`] tree.
///
/// Accepts either a set of top-level `attr = <…>` pairs (an
/// [`OdinValue::Object`], the usual ADL-section shape) or a bare
/// object-value block.
///
/// # Errors
/// Returns an [`OdinError`] (with line/column) on the first lexical or
/// syntactic error.
pub fn parse(src: &str) -> Result<OdinValue, OdinError> {
    let spanned = lexer::lex(src).map_err(|span| {
        let (line, column) = line_col(src, span.start);
        OdinError {
            kind: OdinErrorKind::UnrecognisedToken,
            message: "unrecognised token".to_owned(),
            line,
            column,
            span,
        }
    })?;
    parser::parse_tokens(&spanned).map_err(|located| {
        let (line, column) = line_col(src, located.offset);
        let (kind, message) = match located.failure {
            parser::Failure::Syntax(_) => (
                OdinErrorKind::UnexpectedToken,
                "unexpected token".to_owned(),
            ),
            parser::Failure::DuplicateAttribute { name, .. } => (
                OdinErrorKind::DuplicateAttribute(name.clone()),
                format!("duplicate sibling attribute {name:?} (VDATU)"),
            ),
        };
        OdinError {
            kind,
            message,
            line,
            column,
            span: located.offset..located.offset,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(src: &str) -> IndexMap<String, OdinValue> {
        match parse(src).unwrap_or_else(|e| panic!("parse failed: {e}")) {
            OdinValue::Object(m) => m,
            other => panic!("expected object, got {other:?}"),
        }
    }

    #[test]
    fn simple_string_attribute() {
        let m = obj(r#"lifecycle_state = <"published">"#);
        assert_eq!(
            m.get("lifecycle_state"),
            Some(&OdinValue::String("published".to_owned()))
        );
    }

    #[test]
    fn term_code_and_uri_leaves() {
        let m = obj("original_language = <[ISO_639-1::en]>");
        assert_eq!(
            m.get("original_language"),
            Some(&OdinValue::TermCode("[ISO_639-1::en]".to_owned()))
        );
        let m = obj("target = <http://loinc.org/id/9272-6>");
        assert_eq!(
            m.get("target"),
            Some(&OdinValue::Uri("<http://loinc.org/id/9272-6>".to_owned()))
        );
    }

    #[test]
    fn nested_object_and_keyed_list() {
        let src = r#"
            details = <
                ["en"] = <
                    language = <[ISO_639-1::en]>
                    keywords = <"ADL", "test">
                >
            >
        "#;
        let m = obj(src);
        let OdinValue::KeyedList(entries) = m.get("details").expect("details") else {
            panic!("expected keyed list");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, OdinKey::String("en".to_owned()));
        let OdinValue::Object(inner) = &entries[0].1 else {
            panic!("expected inner object");
        };
        assert_eq!(
            inner.get("keywords"),
            Some(&OdinValue::List(vec![
                OdinValue::String("ADL".to_owned()),
                OdinValue::String("test".to_owned()),
            ]))
        );
    }

    #[test]
    fn integer_real_boolean_and_intervals() {
        let m = obj("a = <5>\nb = <-3.5>\nc = <True>");
        assert_eq!(m.get("a"), Some(&OdinValue::Integer(5)));
        assert_eq!(m.get("b"), Some(&OdinValue::Real(-3.5)));
        assert_eq!(m.get("c"), Some(&OdinValue::Boolean(true)));

        let m = obj("range = <|0..10|>");
        let OdinValue::Interval(OdinInterval::Range {
            lower,
            lower_included,
            upper,
            upper_included,
        }) = m.get("range").expect("range")
        else {
            panic!("expected range interval");
        };
        assert_eq!(lower.as_deref(), Some(&OdinValue::Integer(0)));
        assert_eq!(upper.as_deref(), Some(&OdinValue::Integer(10)));
        assert!(*lower_included && *upper_included);
    }

    #[test]
    fn open_list_and_partial_date() {
        let m = obj(r#"c = <"a", ...>"#);
        assert_eq!(
            m.get("c"),
            Some(&OdinValue::List(vec![
                OdinValue::String("a".to_owned()),
                OdinValue::ListContinue,
            ]))
        );
        let m = obj("d = <2004-06-??>");
        assert_eq!(m.get("d"), Some(&OdinValue::Date("2004-06-??".to_owned())));
    }

    #[test]
    fn typed_cast_and_reference_path() {
        let m = obj("q = (Interval<Quantity>) <|0..1|>");
        let OdinValue::Typed { rm_type, value } = m.get("q").expect("q") else {
            panic!("expected typed cast");
        };
        assert_eq!(rm_type, "Interval<Quantity>");
        assert!(matches!(**value, OdinValue::Interval(_)));

        let m = obj("ref = </data[id2]/items>");
        assert_eq!(
            m.get("ref"),
            Some(&OdinValue::PathList(vec!["/data[id2]/items".to_owned()]))
        );
    }

    /// Qualified (namespaced) type identifiers, both spellings the chapter
    /// exemplifies: "Namespaces are included by prepending package names,
    /// separated by the '.' character … as in the qualified type names
    /// `org.openehr.rm.ehr.content.ENTRY` and
    /// `Core.Abstractions.Relationships.Relationship`"
    /// (`LANG/docs/odin/master05-content` §Adding Type Information, verbatim
    /// in `AM/docs/ADL1.4/master04-dadl` §Adding Type Information). The
    /// vendored `odin.g4` admits only the bare `ALPHA_UC_ID`; the docs text
    /// wins, and the qualified name is preserved flat.
    #[test]
    fn namespaced_type_casts_parse_and_keep_the_dotted_name() {
        for qualified in [
            "org.openehr.rm.ehr.content.ENTRY",
            "Core.Abstractions.Relationships.Relationship",
            "org.openehr.rm.data_types.text.DV_TEXT",
        ] {
            let m = obj(&format!("a = ({qualified}) <b = <1>>"));
            let OdinValue::Typed { rm_type, value } = m.get("a").expect("a") else {
                panic!("expected a typed cast for {qualified}");
            };
            assert_eq!(rm_type, qualified);
            assert!(matches!(**value, OdinValue::Object(_)));
        }
    }

    /// The `master04-dadl` principle box allows "Dot-separated namespace
    /// identifiers AND template parameters" on the same identifier, and a
    /// template parameter is itself a type identifier — so both the generic
    /// and its parameters may be qualified, and the unqualified generic form
    /// is unchanged.
    #[test]
    fn namespaced_and_plain_generic_type_casts_parse() {
        for (src, expected) in [
            ("Interval<Quantity>", "Interval<Quantity>"),
            (
                "List<org.openehr.rm.ehr.content.ENTRY>",
                "List<org.openehr.rm.ehr.content.ENTRY>",
            ),
            (
                "org.openehr.base.foundation_types.Interval<org.openehr.base.Quantity>",
                "org.openehr.base.foundation_types.Interval<org.openehr.base.Quantity>",
            ),
            (
                "Hash<org.openehr.rm.HOTEL,String>",
                "Hash<org.openehr.rm.HOTEL,String>",
            ),
        ] {
            let m = obj(&format!("a = ({src}) <...>"));
            let OdinValue::Typed { rm_type, .. } = m.get("a").expect("a") else {
                panic!("expected a typed cast for {src}");
            };
            assert_eq!(rm_type, expected);
        }
    }

    /// The qualification names a package path, so the segment it qualifies
    /// stays the `ALPHA_UC_ID` type identifier: a lower-case terminal segment
    /// is an attribute path, not a type, and must not be read as a cast.
    #[test]
    fn a_lowercase_terminal_segment_is_not_a_type_identifier() {
        assert!(parse("a = (org.openehr.rm.content) <b = <1>>").is_err());
    }
    #[test]
    fn illegal_escape_is_error() {
        assert!(parse(r#"a = <"bad \d">"#).is_err());
    }

    /// `AM/docs/ADL1.4/master04-dadl` §Empty Sections ("Empty sections can
    /// appear anywhere"; "Nested empty sections can be used") +
    /// `LANG/docs/odin/master05-content` §Void Objects.
    #[test]
    fn empty_sections_appear_anywhere() {
        let m = obj("address = <...>");
        assert_eq!(m.get("address"), Some(&OdinValue::Empty));

        // nested: inside an object, inside a keyed list, and behind a cast.
        let m = obj("a = <b = <...> c = <d = <...>>>\ne = <[1] = <...>>\nf = (PENSION) <...>");
        let OdinValue::Object(inner) = m.get("a").expect("a") else {
            panic!("expected object");
        };
        assert_eq!(inner.get("b"), Some(&OdinValue::Empty));
        let OdinValue::KeyedList(entries) = m.get("e").expect("e") else {
            panic!("expected keyed list");
        };
        assert_eq!(entries[0].1, OdinValue::Empty);
        assert_eq!(
            m.get("f"),
            Some(&OdinValue::Typed {
                rm_type: "PENSION".to_owned(),
                value: Box::new(OdinValue::Empty),
            })
        );
    }

    /// `AM/docs/ADL1.4/master04-dadl` §Partial Date/Times, the `T??:??:??`
    /// family the ODIN chapter omits.
    #[test]
    fn partial_date_time_family() {
        for src in [
            "2004-06-11T10:30:??",
            "2004-06-11T10:??:??",
            "2004-06-11T??:??:??",
            "2004-06-??T??:??:??",
            "2004-??-??T??:??:??",
        ] {
            let m = obj(&format!("d = <{src}>"));
            assert_eq!(
                m.get("d"),
                Some(&OdinValue::DateTime(src.to_owned())),
                "{src}"
            );
        }
    }

    /// `AM/docs/ADL1.4/master08-adl` §Revision History Section writes
    /// `time_committed = <2004-11-02 09:31:04+1000>`; the space form is
    /// accepted and normalised to the ISO `T` form (see the lexer NOTE).
    #[test]
    fn space_separated_date_time_normalises() {
        let m = obj("time_committed = <2004-11-02 09:31:04+1000>");
        assert_eq!(
            m.get("time_committed"),
            Some(&OdinValue::DateTime("2004-11-02T09:31:04+1000".to_owned()))
        );
    }

    /// `AM/docs/ADL1.4/master04-dadl` §Integer Data lists `29e6` as integer
    /// data; §Boolean Data + §Symbols make the boolean words case-insensitive.
    #[test]
    fn integer_exponents_and_case_insensitive_booleans() {
        let m = obj("a = <29e6>\nb = <2900e-2>\nc = <TRUE>\nd = <fAlSe>");
        assert_eq!(m.get("a"), Some(&OdinValue::Integer(29_000_000)));
        assert_eq!(m.get("b"), Some(&OdinValue::Integer(29)));
        assert_eq!(m.get("c"), Some(&OdinValue::Boolean(true)));
        assert_eq!(m.get("d"), Some(&OdinValue::Boolean(false)));
        // an inexact negative exponent has no integer value.
        assert!(parse("a = <29e-2>").is_err());
    }

    /// `AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types:
    /// `infinity` / `-infinity` / `*` are allowable endpoint values, e.g.
    /// `|0..infinity|`.
    #[test]
    fn unbounded_interval_endpoints() {
        for src in ["|0..infinity|", "|0..*|", "|0..INFINITY|"] {
            let m = obj(&format!("r = <{src}>"));
            let Some(OdinValue::Interval(OdinInterval::Range {
                lower,
                lower_included,
                upper,
                upper_included,
            })) = m.get("r")
            else {
                panic!("expected range interval for {src}");
            };
            assert_eq!(lower.as_deref(), Some(&OdinValue::Integer(0)), "{src}");
            assert!(*lower_included, "{src}");
            assert_eq!(upper.as_deref(), None, "{src}");
            assert!(!*upper_included, "{src}");
        }

        let m = obj("r = <|-infinity..5.0|>");
        let Some(OdinValue::Interval(OdinInterval::Range { lower, upper, .. })) = m.get("r") else {
            panic!("expected range interval");
        };
        assert_eq!(lower.as_deref(), None);
        assert_eq!(upper.as_deref(), Some(&OdinValue::Real(5.0)));
    }

    /// `AM/docs/ADL1.4/master04-dadl` §Lists of Built-in Types +
    /// `LANG/docs/odin/master07-leaf_data` §Lists of Built-in Types both list
    /// `[at0200], ...` as leaf data (see the lexer NOTE on the chapter's own
    /// yacc disagreeing).
    #[test]
    fn local_term_codes_are_leaf_values() {
        let m = obj("a = <[at0200]>\nb = <[at0200], ...>\nc = <[at0010.2]>");
        assert_eq!(
            m.get("a"),
            Some(&OdinValue::TermCode("[at0200]".to_owned()))
        );
        assert_eq!(
            m.get("b"),
            Some(&OdinValue::List(vec![
                OdinValue::TermCode("[at0200]".to_owned()),
                OdinValue::ListContinue,
            ]))
        );
        assert_eq!(
            m.get("c"),
            Some(&OdinValue::TermCode("[at0010.2]".to_owned()))
        );
        // integer / date container keys still lex as keys, not local codes.
        let map = obj("k = <[1] = <\"x\">>");
        let Some(OdinValue::KeyedList(entries)) = map.get("k") else {
            panic!("expected keyed list");
        };
        assert_eq!(entries[0].0, OdinKey::Integer(1));
    }

    /// Rule *VDATU* (`LANG/docs/odin/master05-content` §General Structure) /
    /// the "Sibling attribute names must be unique" principle of
    /// `AM/docs/ADL1.4/master04-dadl` §General Form.
    #[test]
    fn duplicate_sibling_attributes_are_refused() {
        let err = parse("a = <1>\nb = <2>\na = <3>").expect_err("duplicate `a`");
        assert_eq!(err.kind, OdinErrorKind::DuplicateAttribute("a".to_owned()));

        // …at any depth.
        let err = parse("outer = <x = <1> x = <2>>").expect_err("duplicate nested `x`");
        assert_eq!(err.kind, OdinErrorKind::DuplicateAttribute("x".to_owned()));

        // the same name under DIFFERENT parents is not a duplicate.
        assert!(parse("p = <x = <1>>\nq = <x = <2>>").is_ok());
    }

    /// `LANG/docs/odin/master07-leaf_data` §String Data (verbatim in
    /// `AM/docs/ADL1.4/master04-dadl` §String Data): multi-line string
    /// contents drop the white-space leaders of the continuation lines.
    #[test]
    fn multi_line_string_leaders_are_stripped() {
        let m = obj(
            "    text = <\"And now the STORM-BLAST came, and he\n        Was tyrannous and strong :\n        And chased us south along.\">",
        );
        assert_eq!(
            m.get("text"),
            Some(&OdinValue::String(
                "And now the STORM-BLAST came, and he\nWas tyrannous and strong :\nAnd chased us south along."
                    .to_owned()
            ))
        );
        // `&quot;` is carried through verbatim (see the lexer NOTE).
        let m = obj(r#"t = <"a &quot;phrase&quot;.">"#);
        assert_eq!(
            m.get("t"),
            Some(&OdinValue::String("a &quot;phrase&quot;.".to_owned()))
        );
    }
}
