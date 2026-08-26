// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The closed `${…}` variable-reference grammar and the case-level capture
//! grammar.
//!
//! Case cores speak these forms and nothing else (CNF 2.0 artifact-set
//! design, case-core contract): `${row.<column>}`, `${fixture.<field>}`,
//! `${<capture>}`, `${ds:<corpus key>}`, `${ds:<corpus key>#<view>}`,
//! `${recipe:<name>(row)}`, the temporal expressions
//! `${time:before(<t>)}` / `${time:after(<t>)}` / `${time:between(<t1>,<t2>)}`,
//! and `${ixit:<field>}` for a party-declared SUT fact no released operation
//! discloses. There is no `${stepN}` form. Binding request templates may additionally
//! mark a reference optional (`${offset?}`). A string outside these forms is
//! a validator error, never runner latitude.

use std::fmt;

use thiserror::Error;

use crate::ids::{CaptureName, CorpusKey, IdError, RecipeName, ViewName};
use crate::vocab::OutcomeKind;

/// Reference-grammar parse error.
#[derive(Debug, Error)]
pub enum RefError {
    /// A `${` without a closing `}`.
    #[error("unterminated ${{…}} reference in {0:?}")]
    Unterminated(String),
    /// A reference body outside the closed grammar.
    #[error("illegal reference ${{{0}}}: {1}")]
    Illegal(String, String),
    /// An embedded identifier failed its lexical rule.
    #[error("in ${{{reference}}}: {source}")]
    BadIdent {
        /// The offending reference body.
        reference: String,
        /// The identifier error.
        source: IdError,
    },
}

/// The `${fixture.<field>}` fields — closed to the fixture-set entry
/// bindings (`data_set`, `expected`, `defect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureField {
    /// The entry's corpus key.
    DataSet,
    /// The entry's expected outcome kind.
    Expected,
    /// The entry's defect phrase (invalid fixtures).
    Defect,
}

impl FixtureField {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "data_set" => Some(Self::DataSet),
            "expected" => Some(Self::Expected),
            "defect" => Some(Self::Defect),
            _ => None,
        }
    }

    /// The field token.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::DataSet => "data_set",
            Self::Expected => "expected",
            Self::Defect => "defect",
        }
    }
}

/// The `${ixit:<field>}` fields — closed to the environment facts a party
/// DECLARES about its SUT because no released operation discloses them.
///
/// A case may read such a fact, never invent one; a party that declares none
/// makes the referencing cases not-applicable with that citation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IxitField {
    /// The SUT's own configured system identifier
    /// (`crate::ixit::Ixit::system_id`).
    SystemId,
    /// A writable location on the SUT's OWN file system that the admin
    /// dump/load operations may use (`crate::ixit::Ixit::dump_location`).
    DumpLocation,
}

impl IxitField {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "system_id" => Some(Self::SystemId),
            "dump_location" => Some(Self::DumpLocation),
            _ => None,
        }
    }

    /// The field token.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::SystemId => "system_id",
            Self::DumpLocation => "dump_location",
        }
    }
}

/// A temporal at-time expression over captured commit instants.
///
/// Resolution is fixed by the interpreter laws (before = t − 1 ms, after = t
/// + 1 ms, between = midpoint) so two runners query identical instants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeExpr {
    /// One millisecond before the named capture's commit instant.
    Before(CaptureName),
    /// One millisecond after the named capture's commit instant.
    After(CaptureName),
    /// The midpoint between two captured commit instants.
    Between(CaptureName, CaptureName),
}

/// One parsed `${…}` reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueRef {
    /// `${row.<column>}` — the current parameter-matrix row cell.
    Row(String),
    /// `${fixture.<field>}` — the current fixture-set entry.
    Fixture(FixtureField),
    /// `${<capture>}` — a case-scoped capture or `requires` handle.
    /// `optional` is the binding-template `${name?}` marker.
    Capture {
        /// The capture (or `requires` handle) the reference addresses.
        name: CaptureName,
        /// The `${name?}` marker: unresolved means "omit", not "fail".
        optional: bool,
    },
    /// `${ds:<key>}` / `${ds:<key>#<view>}` — a corpus data set or a named
    /// projection over it.
    DataSet {
        /// The corpus manifest key.
        key: CorpusKey,
        /// The named projection over the set, when one is addressed.
        view: Option<ViewName>,
    },
    /// `${ds:fixture}` — the current fixture-set entry's payload (legal only
    /// in cases carrying `parameters.fixture_set`).
    FixtureDataSet,
    /// `${recipe:<name>(row)}` — row-to-instance synthesis.
    Recipe(RecipeName),
    /// `${ixit:<field>}` — a party-declared environment fact about the SUT.
    Ixit(IxitField),
    /// `${time:…}` — temporal reference.
    Time(TimeExpr),
}

impl ValueRef {
    /// Parse one reference body (the text between `${` and `}`).
    ///
    /// # Errors
    /// Returns [`RefError`] when the body is outside the closed grammar.
    pub fn parse(body: &str) -> Result<Self, RefError> {
        let bad_ident = |source| RefError::BadIdent {
            reference: body.to_owned(),
            source,
        };
        let illegal = |why: &str| RefError::Illegal(body.to_owned(), why.to_owned());

        if let Some(column) = body.strip_prefix("row.") {
            if column.is_empty() || column.contains(char::is_whitespace) {
                return Err(illegal("row column must be a non-empty name"));
            }
            return Ok(Self::Row(column.to_owned()));
        }
        if let Some(field) = body.strip_prefix("fixture.") {
            return FixtureField::parse(field)
                .map(Self::Fixture)
                .ok_or_else(|| illegal("fixture field must be data_set | expected | defect"));
        }
        if body == "ds:fixture" {
            return Ok(Self::FixtureDataSet);
        }
        if let Some(rest) = body.strip_prefix("ds:") {
            let (key, view) = match rest.split_once('#') {
                Some((key, view)) => (key, Some(ViewName::parse(view).map_err(bad_ident)?)),
                None => (rest, None),
            };
            let key = CorpusKey::parse(key).map_err(bad_ident)?;
            return Ok(Self::DataSet { key, view });
        }
        if let Some(rest) = body.strip_prefix("recipe:") {
            let name = rest
                .strip_suffix("(row)")
                .ok_or_else(|| illegal("recipe reference must end in (row)"))?;
            return Ok(Self::Recipe(RecipeName::parse(name).map_err(bad_ident)?));
        }
        if let Some(rest) = body.strip_prefix("time:") {
            return Self::parse_time(rest).map(Self::Time).ok_or_else(|| {
                illegal("time expression must be before(<t>) | after(<t>) | between(<t1>,<t2>)")
            });
        }
        if let Some(field) = body.strip_prefix("ixit:") {
            return IxitField::parse(field)
                .map(Self::Ixit)
                .ok_or_else(|| illegal("ixit field must be system_id"));
        }
        if body.contains(':') || body.contains('.') {
            return Err(illegal(
                "unknown reference form (closed grammar: row./fixture./ds:/recipe:/time:/ixit:/<capture>)",
            ));
        }
        let (name, optional) = match body.strip_suffix('?') {
            Some(name) => (name, true),
            None => (body, false),
        };
        Ok(Self::Capture {
            name: CaptureName::parse(name).map_err(bad_ident)?,
            optional,
        })
    }

    fn parse_time(rest: &str) -> Option<TimeExpr> {
        let inner = |prefix: &str| -> Option<&str> {
            rest.strip_prefix(prefix)?
                .strip_prefix('(')?
                .strip_suffix(')')
        };
        if let Some(arg) = inner("before") {
            return CaptureName::parse(arg.trim()).ok().map(TimeExpr::Before);
        }
        if let Some(arg) = inner("after") {
            return CaptureName::parse(arg.trim()).ok().map(TimeExpr::After);
        }
        if let Some(args) = inner("between") {
            let (a, b) = args.split_once(',')?;
            return Some(TimeExpr::Between(
                CaptureName::parse(a.trim()).ok()?,
                CaptureName::parse(b.trim()).ok()?,
            ));
        }
        None
    }
}

impl fmt::Display for ValueRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row(c) => write!(f, "${{row.{c}}}"),
            Self::Fixture(field) => write!(f, "${{fixture.{}}}", field.token()),
            Self::Capture { name, optional } => {
                write!(f, "${{{name}{}}}", if *optional { "?" } else { "" })
            }
            Self::DataSet {
                key,
                view: Some(view),
            } => write!(f, "${{ds:{key}#{view}}}"),
            Self::DataSet { key, view: None } => write!(f, "${{ds:{key}}}"),
            Self::FixtureDataSet => f.write_str("${ds:fixture}"),
            Self::Recipe(name) => write!(f, "${{recipe:{name}(row)}}"),
            Self::Ixit(field) => write!(f, "${{ixit:{}}}", field.token()),
            Self::Time(TimeExpr::Before(t)) => write!(f, "${{time:before({t})}}"),
            Self::Time(TimeExpr::After(t)) => write!(f, "${{time:after({t})}}"),
            Self::Time(TimeExpr::Between(a, b)) => write!(f, "${{time:between({a},{b})}}"),
        }
    }
}

/// One segment of a templated string.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    /// Literal text.
    Lit(String),
    /// A `${…}` reference.
    Ref(ValueRef),
}

/// A string value that may interleave literal text with `${…}` references
/// (`"${versioned_object_uid}::<system>::2"`). Parsing validates every
/// embedded reference against the closed grammar.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    raw: String,
    segments: Vec<Segment>,
}

impl Template {
    /// Parse a raw string, validating every `${…}` occurrence.
    ///
    /// # Errors
    /// Returns [`RefError`] on an unterminated or illegal reference.
    pub fn parse(raw: &str) -> Result<Self, RefError> {
        let mut segments = Vec::new();
        let mut rest = raw;
        while let Some(start) = rest.find("${") {
            let (lit, tail) = rest.split_at(start);
            if !lit.is_empty() {
                segments.push(Segment::Lit(lit.to_owned()));
            }
            let body_and_more = tail.get(2..).unwrap_or_default();
            let end = body_and_more
                .find('}')
                .ok_or_else(|| RefError::Unterminated(raw.to_owned()))?;
            let body = body_and_more.get(..end).unwrap_or_default();
            segments.push(Segment::Ref(ValueRef::parse(body)?));
            rest = body_and_more.get(end + 1..).unwrap_or_default();
        }
        if !rest.is_empty() {
            segments.push(Segment::Lit(rest.to_owned()));
        }
        Ok(Self {
            raw: raw.to_owned(),
            segments,
        })
    }

    /// The raw authored text.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The parsed segments.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Every reference in the template, in order.
    pub fn refs(&self) -> impl Iterator<Item = &ValueRef> {
        self.segments.iter().filter_map(|s| match s {
            Segment::Ref(r) => Some(r),
            Segment::Lit(_) => None,
        })
    }

    /// Whether the whole template is exactly one reference (no literal text).
    #[must_use]
    pub fn as_single_ref(&self) -> Option<&ValueRef> {
        match self.segments.as_slice() {
            [Segment::Ref(r)] => Some(r),
            _ => None,
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl serde::Serialize for Template {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> serde::Deserialize<'de> for Template {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// What part of the step outcome a case-level capture reads. Sources are
/// closed: a logical field mapped by the binding, the full response body, or
/// the committed audit time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureField {
    /// `<outcome>.body` — the full response representation.
    Body,
    /// `<outcome>.commit_time` — the committed audit time (the anchor for
    /// temporal at-time cases).
    CommitTime,
    /// `<outcome>.<field>` (`list` for the `<field>[]` list-capture form).
    Field {
        /// The field of the outcome's capture mapping.
        name: CaptureName,
        /// The `<field>[]` form: capture every match as a list.
        list: bool,
    },
}

/// A case-level capture source: `<outcome kind>.<capture field>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureValueSource {
    /// The outcome kind whose mapping supplies the value.
    pub outcome: OutcomeKind,
    /// What is read.
    pub field: CaptureField,
}

impl CaptureValueSource {
    /// Parse `created.ehr_id`, `ok.body`, `created.version_uids[]`, ….
    ///
    /// # Errors
    /// Returns [`RefError`] when the source is outside the closed grammar.
    pub fn parse(raw: &str) -> Result<Self, RefError> {
        let illegal = |why: &str| RefError::Illegal(raw.to_owned(), why.to_owned());
        let (outcome, field) = raw
            .split_once('.')
            .ok_or_else(|| illegal("capture source must be <outcome>.<field>"))?;
        let outcome = OutcomeKind::from_token(outcome)
            .ok_or_else(|| illegal("capture source outcome must be an outcome kind"))?;
        let field = match field {
            "body" => CaptureField::Body,
            "commit_time" => CaptureField::CommitTime,
            other => {
                let (name, list) = match other.strip_suffix("[]") {
                    Some(name) => (name, true),
                    None => (other, false),
                };
                CaptureField::Field {
                    name: CaptureName::parse(name).map_err(|source| RefError::BadIdent {
                        reference: raw.to_owned(),
                        source,
                    })?,
                    list,
                }
            }
        };
        Ok(Self { outcome, field })
    }
}

impl fmt::Display for CaptureValueSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token = self.outcome.token();
        match &self.field {
            CaptureField::Body => write!(f, "{token}.body"),
            CaptureField::CommitTime => write!(f, "{token}.commit_time"),
            CaptureField::Field { name, list } => {
                write!(f, "{token}.{name}{}", if *list { "[]" } else { "" })
            }
        }
    }
}

impl serde::Serialize for CaptureValueSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for CaptureValueSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ref(body: &str) -> ValueRef {
        ValueRef::parse(body).unwrap()
    }

    #[test]
    fn closed_forms_parse() {
        assert_eq!(parse_ref("row.ehr_id"), ValueRef::Row("ehr_id".into()));
        assert_eq!(
            parse_ref("fixture.expected"),
            ValueRef::Fixture(FixtureField::Expected)
        );
        assert!(matches!(
            parse_ref("first_ehr_id"),
            ValueRef::Capture {
                optional: false,
                ..
            }
        ));
        assert!(matches!(
            parse_ref("offset?"),
            ValueRef::Capture { optional: true, .. }
        ));
        assert!(matches!(
            parse_ref("ds:cnf.set.bp-10"),
            ValueRef::DataSet { view: None, .. }
        ));
        assert!(matches!(
            parse_ref("ds:cnf.set.bp-10#magnitude_ge_140_by_uid"),
            ValueRef::DataSet { view: Some(_), .. }
        ));
        assert!(matches!(
            parse_ref("recipe:ehr_status(row)"),
            ValueRef::Recipe(_)
        ));
        assert!(matches!(
            parse_ref("time:before(t1)"),
            ValueRef::Time(TimeExpr::Before(_))
        ));
        assert!(matches!(
            parse_ref("time:between(t1,t2)"),
            ValueRef::Time(TimeExpr::Between(..))
        ));
        assert_eq!(
            parse_ref("ixit:system_id"),
            ValueRef::Ixit(IxitField::SystemId)
        );
    }

    #[test]
    fn ixit_references_round_trip_and_stay_closed() {
        let r = parse_ref("ixit:system_id");
        assert_eq!(r.to_string(), "${ixit:system_id}");
        // The rendered form parses back inside a template.
        let template = Template::parse(&r.to_string()).unwrap();
        assert_eq!(template.as_single_ref(), Some(&r));
        // The field set is closed: no invented environment facts.
        assert!(ValueRef::parse("ixit:hardware_class").is_err());
        assert!(ValueRef::parse("ixit:").is_err());
    }

    #[test]
    fn illegal_forms_rejected() {
        assert!(ValueRef::parse("step2.body").is_err()); // no ${stepN} form
        assert!(ValueRef::parse("fixture.payload").is_err());
        assert!(ValueRef::parse("recipe:ehr_status").is_err());
        assert!(ValueRef::parse("time:around(t1)").is_err());
        assert!(ValueRef::parse("ds:Not.A.Key").is_err());
    }

    #[test]
    fn templates_scan_all_refs() {
        let t = Template::parse("${versioned_object_uid}::<system>::2").unwrap();
        assert_eq!(t.refs().count(), 1);
        assert!(t.as_single_ref().is_none());
        assert!(Template::parse("${unclosed").is_err());
        assert!(Template::parse("prefix ${step2.body} suffix").is_err());
        let single = Template::parse("${ds:cnf.composition.minimal_event.v1}").unwrap();
        assert!(single.as_single_ref().is_some());
    }

    #[test]
    fn capture_sources() {
        let s = CaptureValueSource::parse("created.version_uids[]").unwrap();
        assert_eq!(s.outcome, OutcomeKind::Created);
        assert!(matches!(s.field, CaptureField::Field { list: true, .. }));
        assert!(matches!(
            CaptureValueSource::parse("ok.body").unwrap().field,
            CaptureField::Body
        ));
        assert!(matches!(
            CaptureValueSource::parse("created.commit_time")
                .unwrap()
                .field,
            CaptureField::CommitTime
        ));
        assert!(CaptureValueSource::parse("nonsense.ehr_id").is_err());
        assert!(CaptureValueSource::parse("created").is_err());
    }
}
