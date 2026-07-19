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

/// A parsed ODIN value.
///
/// The tree mirrors `odin.g4`: an object (`attr = <…>` pairs), an
/// insertion-ordered keyed list (`["k"] = <…>`), a primitive value or list, an
/// interval, an embedded URI, an object-reference path list, or an empty block.
#[derive(Debug, Clone, PartialEq)]
pub enum OdinValue {
    /// `attr_vals` — an object with attribute → value members (insertion
    /// order preserved). A later duplicate key overwrites the earlier value.
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
    /// A range `| >? lower .. <? upper |`, or a single-bounded `| relop v |`
    /// (one endpoint `None` = unbounded on that side).
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

/// An ODIN parse or lex failure, with a 1-based line/column and a byte span.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ODIN syntax error at line {line}, column {column}: {message}")]
pub struct OdinError {
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
            message: "unrecognised token".to_owned(),
            line,
            column,
            span,
        }
    })?;
    parser::parse_tokens(&spanned).map_err(|offset| {
        let (line, column) = line_col(src, offset);
        OdinError {
            message: "unexpected token".to_owned(),
            line,
            column,
            span: offset..offset,
        }
    })
}

/// Resolve a byte offset to a 1-based `(line, column)` (columns count `char`s).
fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let clamped = offset.min(src.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in src.char_indices() {
        if idx >= clamped {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
#[allow(clippy::panic)] // test assertions panic by design
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

    #[test]
    fn illegal_escape_is_error() {
        assert!(parse(r#"a = <"bad \d">"#).is_err());
    }
}
