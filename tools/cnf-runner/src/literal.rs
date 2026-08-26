// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The decision-table literal grammar and violation categories.
//!
//! The content chapters carry structured constraint literals in their
//! decision tables (`CNF platform_test_schedule master15–17`): ranges
//! `5.0..10.0`, lists `[cm 5.0..10.0, m]`, terminology codes
//! `openehr::122 (length)` / `local::at0005`, ordinal tuples
//! `1|[local::at0005]`, quantity literals `100 mg`. Violation categories
//! name RM/schema rules, named RM invariants, ISO 8601 rules, and constraint
//! clauses. The grammar is normative (published with the schemas); every
//! table cell must parse against it.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::fmt;

use thiserror::Error;

/// Literal-grammar parse error.
#[derive(Debug, Error)]
#[error("invalid decision-table literal {text:?}: {reason}")]
pub struct LiteralError {
    text: String,
    reason: String,
}

impl LiteralError {
    fn new(text: &str, reason: impl Into<String>) -> Self {
        Self {
            text: text.to_owned(),
            reason: reason.into(),
        }
    }
}

/// A parsed decision-table literal.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// JSON null (a first-class cell value in the official `DV_QUANTITY` table).
    Null,
    /// A boolean cell.
    Bool(bool),
    /// An integer cell.
    Integer(i64),
    /// A real-number cell.
    Real(f64),
    /// A plain string cell with no grammar-significant structure.
    Text(String),
    /// A numeric range `a..b`.
    Range {
        /// The range's lower endpoint.
        lo: f64,
        /// The range's upper endpoint.
        hi: f64,
    },
    /// An ISO 8601 endpoint range `2020-01..2030-12` (dates, date-times,
    /// times, or durations — the master17.4/`DV_INTERVAL` constraint ranges).
    Iso8601Range {
        /// The lower endpoint, in its ISO 8601 lexical form.
        lo: String,
        /// The upper endpoint, in its ISO 8601 lexical form.
        hi: String,
    },
    /// A list `[x, y, …]` of literals.
    List(Vec<Literal>),
    /// A unit-scoped range `cm 5.0..10.0` (inside `C_DV_QUANTITY` list cells).
    UnitRange {
        /// The units the endpoints are expressed in.
        units: String,
        /// The range's lower endpoint, in `units`.
        lo: f64,
        /// The range's upper endpoint, in `units`.
        hi: f64,
    },
    /// A terminology code `openehr::122 (length)` / `local::at0005`.
    TermCode {
        /// The terminology id the code belongs to (`openehr`, `local`, …).
        terminology: String,
        /// The code string itself.
        code: String,
        /// The parenthesised rubric, when the cell carries one.
        rubric: Option<String>,
    },
    /// An ordinal tuple `1|[local::at0005]` (integer head — `DV_ORDINAL`).
    Ordinal {
        /// The ordinal's integer value.
        value: i64,
        /// The coded symbol the value is bound to.
        symbol: Box<Literal>,
    },
    /// A scale tuple `1.5|[local::at0005]` (real head — `DV_SCALE`).
    Scale {
        /// The scale's real value.
        value: f64,
        /// The coded symbol the value is bound to.
        symbol: Box<Literal>,
    },
    /// A quantity literal `100 mg`.
    Quantity {
        /// The quantity's magnitude.
        magnitude: f64,
        /// The UCUM units the magnitude is expressed in.
        units: String,
    },
}

impl Literal {
    /// Parse a decision-table cell from its JSON value. Strings run through
    /// the literal grammar; a string with grammar-significant markers
    /// (`..`, `::`, `|`, `[`) MUST parse as the corresponding production.
    ///
    /// # Errors
    /// Returns [`LiteralError`] when a structured-looking string fails its
    /// production.
    pub fn from_cell(value: &serde_json::Value) -> Result<Self, LiteralError> {
        match value {
            serde_json::Value::Null => Ok(Self::Null),
            serde_json::Value::Bool(b) => Ok(Self::Bool(*b)),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(Self::Integer)
                .or_else(|| n.as_f64().map(Self::Real))
                .ok_or_else(|| LiteralError::new(&n.to_string(), "number is neither i64 nor f64")),
            serde_json::Value::String(s) => Self::from_text(s),
            other => Err(LiteralError::new(
                &other.to_string(),
                "cell must be a scalar or grammar string",
            )),
        }
    }

    /// Parse a string cell through the grammar.
    ///
    /// # Errors
    /// Returns [`LiteralError`] when the string carries grammar-significant
    /// markers but fails the corresponding production.
    pub fn from_text(s: &str) -> Result<Self, LiteralError> {
        let t = s.trim();
        if t.starts_with('[') {
            return Self::parse_list(t);
        }
        if let Some((value, symbol)) = t.split_once('|') {
            // Ordinal tuple `1|[local::at0005]` (integer head — `DV_ORDINAL`);
            // scale tuple `1.5|[local::at0005]` (real head — `DV_SCALE`).
            if let Ok(value) = value.trim().parse::<i64>() {
                let symbol = Self::from_text(symbol)?;
                return Ok(Self::Ordinal {
                    value,
                    symbol: Box::new(symbol),
                });
            }
            if let Ok(value) = value.trim().parse::<f64>() {
                let symbol = Self::from_text(symbol)?;
                return Ok(Self::Scale {
                    value,
                    symbol: Box::new(symbol),
                });
            }
        }
        // The term-code production fires only when the head is a term lexeme
        // (`openehr::…`, `local::…`) — URIs and paths also contain `::` but
        // their heads carry `/`/`[`/`:` and stay plain text.
        if let Some((head, _)) = t.split_once("::")
            && !head.is_empty()
            && head.trim().chars().all(is_term_lexeme_char)
        {
            return Self::parse_term_code(t);
        }
        if t.contains("..") {
            return Self::parse_scoped_range(t);
        }
        if let Some(q) = Self::try_quantity(t) {
            return Ok(q);
        }
        Ok(Self::Text(t.to_owned()))
    }

    fn parse_list(t: &str) -> Result<Self, LiteralError> {
        let inner = t
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .ok_or_else(|| LiteralError::new(t, "unterminated list"))?;
        let mut items = Vec::new();
        for item in split_top_level(inner) {
            let item = item.trim();
            if item.is_empty() {
                return Err(LiteralError::new(t, "empty list item"));
            }
            items.push(Self::from_text(item)?);
        }
        Ok(Self::List(items))
    }

    /// `a..b` or `units a..b`.
    fn parse_scoped_range(t: &str) -> Result<Self, LiteralError> {
        let (units, range) = match t.rsplit_once(' ') {
            Some((units, range)) if range.contains("..") => (Some(units.trim()), range.trim()),
            _ => (None, t),
        };
        let (lo_raw, hi_raw) = range
            .split_once("..")
            .ok_or_else(|| LiteralError::new(t, "range must be a..b"))?;
        let (lo_raw, hi_raw) = (lo_raw.trim(), hi_raw.trim());
        let (Ok(lo), Ok(hi)) = (lo_raw.parse::<f64>(), hi_raw.parse::<f64>()) else {
            if units.is_none() && is_iso8601_lexeme(lo_raw) && is_iso8601_lexeme(hi_raw) {
                return Ok(Self::Iso8601Range {
                    lo: lo_raw.to_owned(),
                    hi: hi_raw.to_owned(),
                });
            }
            return Err(LiteralError::new(
                t,
                "range bounds must both be numbers or both ISO 8601 lexemes",
            ));
        };
        match units {
            Some(units) if !units.is_empty() => Ok(Self::UnitRange {
                units: units.to_owned(),
                lo,
                hi,
            }),
            _ => Ok(Self::Range { lo, hi }),
        }
    }

    /// `openehr::122 (length)` / `local::at0005`.
    fn parse_term_code(t: &str) -> Result<Self, LiteralError> {
        let (terminology, rest) = t.split_once("::").ok_or_else(|| {
            LiteralError::new(t, "terminology code must be <terminology>::<code>")
        })?;
        let terminology = terminology.trim();
        if terminology.is_empty() || !terminology.chars().all(is_term_lexeme_char) {
            return Err(LiteralError::new(t, "terminology name must be one word"));
        }
        let (code, rubric) = match rest.split_once('(') {
            Some((code, rubric)) => {
                let rubric = rubric
                    .strip_suffix(')')
                    .ok_or_else(|| LiteralError::new(t, "unterminated rubric"))?;
                (code.trim(), Some(rubric.trim().to_owned()))
            }
            None => (rest.trim(), None),
        };
        if code.is_empty() || !code.chars().all(is_term_lexeme_char) {
            return Err(LiteralError::new(t, "code must be one word"));
        }
        Ok(Self::TermCode {
            terminology: terminology.to_owned(),
            code: code.to_owned(),
            rubric,
        })
    }

    /// `100 mg` — a number followed by a unit word.
    fn try_quantity(t: &str) -> Option<Self> {
        let (magnitude, units) = t.split_once(' ')?;
        let magnitude: f64 = magnitude.trim().parse().ok()?;
        let units = units.trim();
        if units.is_empty() || units.contains(char::is_whitespace) {
            return None;
        }
        Some(Self::Quantity {
            magnitude,
            units: units.to_owned(),
        })
    }
}

/// A terminology/code lexeme character (word chars only — `|`/`[`/`]` are
/// tuple/list structure, never part of a code).
fn is_term_lexeme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// ISO 8601 lexical shapes accepted as range endpoints: date (`2020`,
/// `2020-01`, `2020-01-31`), date-time (`...T23[:59[:59[.9]]][Z|±hh[:mm]]`),
/// time (`10:00[:00[.5]]`), duration (`P1Y2M3DT4H5M6.7S`, `PT2H`).
fn is_iso8601_lexeme(s: &str) -> bool {
    static ISO: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        #[expect(clippy::unwrap_used, reason = "a compile-time-constant pattern")]
        regex::Regex::new(
            r"^(\d{4}(-\d{2}(-\d{2})?)?(T\d{2}(:\d{2}(:\d{2}(\.\d+)?)?)?(Z|[+-]\d{2}(:?\d{2})?)?)?|\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}(:?\d{2})?)?|P(\d+Y)?(\d+M)?(\d+W)?(\d+D)?(T(\d+H)?(\d+M)?(\d+(\.\d+)?S)?)?)$",
        )
        .unwrap()
    });
    !s.is_empty() && s != "P" && ISO.is_match(s)
}

/// Split on top-level commas (not inside nested brackets/parentheses).
fn split_top_level(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0_usize;
    for (i, c) in s.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s.get(start..i).unwrap_or_default());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(s.get(start..).unwrap_or_default());
    parts
}

/// The closed violation-category taxonomy of content decision tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationCategory {
    /// RM/schema rule (mandatory fields, typing).
    RmSchema,
    /// A named RM invariant (`limits_consistent`).
    RmInvariant,
    /// An ISO 8601 rule.
    Iso8601,
    /// A constraint clause (`C_DV_QUANTITY.list`).
    Constraint,
}

impl ViolationCategory {
    fn parse(token: &str) -> Option<Self> {
        match token {
            "rm_schema" => Some(Self::RmSchema),
            "rm_invariant" => Some(Self::RmInvariant),
            "iso8601" => Some(Self::Iso8601),
            "constraint" => Some(Self::Constraint),
            _ => None,
        }
    }

    /// The category token.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::RmSchema => "rm_schema",
            Self::RmInvariant => "rm_invariant",
            Self::Iso8601 => "iso8601",
            Self::Constraint => "constraint",
        }
    }
}

/// One entry of a `violates` cell:
/// `<category>[(<argument>)]: <description>` — e.g.
/// `constraint(C_DV_QUANTITY.list): magnitude not in range for unit` or
/// `rm_schema: magnitude and units are mandatory`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViolationRef {
    /// The closed category.
    pub category: ViolationCategory,
    /// The named rule/invariant/clause, when the category takes one.
    pub argument: Option<String>,
    /// The human description.
    pub description: String,
}

impl ViolationRef {
    /// Parse one `violates` list entry.
    ///
    /// # Errors
    /// Returns [`LiteralError`] on an unknown category, a malformed
    /// argument, or a missing description.
    pub fn parse(raw: &str) -> Result<Self, LiteralError> {
        let (head, description) = raw.split_once(':').ok_or_else(|| {
            LiteralError::new(raw, "violation must be <category>[(arg)]: <description>")
        })?;
        let description = description.trim();
        if description.is_empty() {
            return Err(LiteralError::new(raw, "violation description is empty"));
        }
        let head = head.trim();
        let (token, argument) = match head.split_once('(') {
            Some((token, arg)) => {
                let arg = arg
                    .strip_suffix(')')
                    .ok_or_else(|| LiteralError::new(raw, "unterminated category argument"))?;
                (token.trim(), Some(arg.trim().to_owned()))
            }
            None => (head, None),
        };
        let category = ViolationCategory::parse(token)
            .ok_or_else(|| LiteralError::new(raw, "unknown violation category"))?;
        match category {
            ViolationCategory::RmInvariant
            | ViolationCategory::Iso8601
            | ViolationCategory::Constraint
                if argument.is_none() =>
            {
                return Err(LiteralError::new(
                    raw,
                    "category requires a (name) argument",
                ));
            }
            ViolationCategory::RmSchema if argument.is_some() => {
                return Err(LiteralError::new(raw, "rm_schema takes no argument"));
            }
            _ => {}
        }
        Ok(Self {
            category,
            argument,
            description: description.to_owned(),
        })
    }
}

impl fmt::Display for ViolationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.argument {
            Some(arg) => write!(f, "{}({arg}): {}", self.category.token(), self.description),
            None => write!(f, "{}: {}", self.category.token(), self.description),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_literals_parse() {
        assert!(matches!(
            Literal::from_text("5.0..10.0").unwrap(),
            Literal::Range { .. }
        ));
        let list = Literal::from_text("[cm 5.0..10.0, m]").unwrap();
        let Literal::List(items) = list else {
            panic!("expected list")
        };
        assert!(matches!(items.first(), Some(Literal::UnitRange { .. })));
        assert!(matches!(items.get(1), Some(Literal::Text(t)) if t == "m"));
        assert!(matches!(
            Literal::from_text("openehr::122 (length)").unwrap(),
            Literal::TermCode {
                rubric: Some(_),
                ..
            }
        ));
        assert!(matches!(
            Literal::from_text("local::at0005").unwrap(),
            Literal::TermCode { rubric: None, .. }
        ));
        assert!(matches!(
            Literal::from_text("1|[local::at0005]").unwrap(),
            Literal::Ordinal { .. }
        ));
        assert!(matches!(
            Literal::from_text("100 mg").unwrap(),
            Literal::Quantity { .. }
        ));
        assert!(matches!(
            Literal::from_cell(&serde_json::Value::Null).unwrap(),
            Literal::Null
        ));
        assert!(matches!(
            Literal::from_text("cm").unwrap(),
            Literal::Text(_)
        ));
    }

    #[test]
    fn iso8601_ranges_and_scale_tuples_parse() {
        assert!(matches!(
            Literal::from_text("2020-01..2030-12").unwrap(),
            Literal::Iso8601Range { .. }
        ));
        assert!(matches!(
            Literal::from_text("2000-01-01T00:00:00.0..2010-12-31T23:59:59.999999").unwrap(),
            Literal::Iso8601Range { .. }
        ));
        assert!(matches!(
            Literal::from_text("PT0S..PT2H").unwrap(),
            Literal::Iso8601Range { .. }
        ));
        assert!(matches!(
            Literal::from_text("10:00..12:00").unwrap(),
            Literal::Iso8601Range { .. }
        ));
        assert!(matches!(
            Literal::from_text("1900..2030").unwrap(),
            Literal::Range { .. }
        ));
        assert!(matches!(
            Literal::from_text("1.5|[local::at0005]").unwrap(),
            Literal::Scale { .. }
        ));
        // the pre-fix degenerate parse is now a hard error
        assert!(Literal::from_text("banana..apple").is_err());
        let list = Literal::from_text("[1.5|[local::at0005], 2.4|[local::at0006]]").unwrap();
        let Literal::List(items) = list else {
            panic!("expected list")
        };
        assert!(matches!(items.first(), Some(Literal::Scale { .. })));
    }

    #[test]
    fn structured_looking_strings_must_parse() {
        assert!(Literal::from_text("5.0..").is_err());
        assert!(Literal::from_text("[cm 5.0..10.0, m").is_err());
        assert!(Literal::from_text("openehr:: (length").is_err());
    }

    #[test]
    fn violations_parse() {
        let v =
            ViolationRef::parse("constraint(C_DV_QUANTITY.list): magnitude not in range for unit")
                .unwrap();
        assert_eq!(v.category, ViolationCategory::Constraint);
        assert_eq!(v.argument.as_deref(), Some("C_DV_QUANTITY.list"));
        let v = ViolationRef::parse("rm_schema: magnitude and units are mandatory").unwrap();
        assert_eq!(v.category, ViolationCategory::RmSchema);
        assert!(ViolationRef::parse("rm_invariant: missing name").is_err());
        assert!(ViolationRef::parse("bogus: nope").is_err());
        assert!(ViolationRef::parse("rm_schema(arg): no args allowed").is_err());
    }
}
