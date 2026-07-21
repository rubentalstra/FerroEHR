//! The typed assertion vocabulary (`flow[].assert` + `postconditions`).
//!
//! Nine assertion forms, closed by schedule release. Semantics per the
//! CNF 2.0 artifact-set design: `equivalent` is the master07 "content
//! check" with normative ignore-sets; `version` asserts RM versioning facts
//! (`RM common §change_control`); `result_set` compares under the normative
//! AQL `RESULT_SET` equivalence rules (QUERY master03/04 + the ITS-REST query
//! schemas); `unique` is aggregate (evaluated once after all rows);
//! `message_exemplar` is informative only, never pass/fail.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

use crate::ids::CaseId;
use crate::model::value::TemplatedValue;
use crate::refgrammar::{RefError, Template, ValueRef};
use crate::vocab::{ChangeType, FormatName, IgnoreSetName, ResultSetMatch};

/// The `equivalent` assertion's comparison target.
#[derive(Debug, Clone, PartialEq)]
pub enum EquivalentTarget {
    /// The content committed earlier in this row (`to: committed`).
    Committed,
    /// A corpus data set or a capture (`to: ${ds:…}` / `to: ${capture}`).
    Ref(ValueRef),
}

impl<'de> Deserialize<'de> for EquivalentTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s == "committed" {
            return Ok(Self::Committed);
        }
        let template = Template::parse(&s).map_err(D::Error::custom)?;
        let reference = template.as_single_ref().ok_or_else(|| {
            D::Error::custom("equivalent target must be `committed` or a single ${…} reference")
        })?;
        match reference {
            ValueRef::DataSet { .. }
            | ValueRef::Capture {
                optional: false, ..
            } => Ok(Self::Ref(reference.clone())),
            _ => Err(D::Error::custom(
                "equivalent target reference must be ${ds:…} or ${<capture>}",
            )),
        }
    }
}

/// One `ignoring:` entry: a named normative ignore-set or an explicit path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoreSpec {
    /// A named set (`server_assigned` resolves from the operation's binding;
    /// `ctx_defaults` from the selectors vocabulary).
    Named(IgnoreSetName),
    /// An explicit RM path.
    Path(String),
}

impl<'de> Deserialize<'de> for IgnoreSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "server_assigned" => Ok(Self::Named(IgnoreSetName::ServerAssigned)),
            "ctx_defaults" => Ok(Self::Named(IgnoreSetName::CtxDefaults)),
            _ if s.contains('/') => Ok(Self::Path(s)),
            _ => Err(D::Error::custom(format!(
                "ignoring entry {s:?} is neither a named ignore-set (server_assigned | ctx_defaults) nor an explicit path"
            ))),
        }
    }
}

/// Scalar-or-list acceptance for `ignoring:`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IgnoreList(pub Vec<IgnoreSpec>);

impl<'de> Deserialize<'de> for IgnoreList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(IgnoreSpec),
            Many(Vec<IgnoreSpec>),
        }
        Ok(match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(one) => Self(vec![one]),
            OneOrMany::Many(many) => Self(many),
        })
    }
}

/// A template that must be exactly one `${…}` reference.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleRef(pub ValueRef);

impl<'de> Deserialize<'de> for SingleRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let template = Template::parse(&s).map_err(D::Error::custom)?;
        let reference = template
            .as_single_ref()
            .ok_or_else(|| D::Error::custom(format!("{s:?} must be a single ${{…}} reference")))?;
        Ok(Self(reference.clone()))
    }
}

/// Expected rows of a `result_set` assertion.
#[derive(Debug, Clone, PartialEq)]
pub enum RowsSpec {
    /// Rows from a named corpus view (`rows: { from: "${ds:<key>#<view>}" }`).
    From(ValueRef),
    /// Inline expected rows.
    Inline(Vec<Vec<serde_json::Value>>),
}

impl<'de> Deserialize<'de> for RowsSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct FromSpec {
            from: SingleRef,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            From(FromSpec),
            Inline(Vec<Vec<serde_json::Value>>),
        }
        match Raw::deserialize(deserializer)? {
            Raw::From(FromSpec { from }) => match from.0 {
                reference @ ValueRef::DataSet { .. } => Ok(Self::From(reference)),
                other => Err(D::Error::custom(format!(
                    "result_set rows.from must be a ${{ds:…}} reference, got {other}"
                ))),
            },
            Raw::Inline(rows) => Ok(Self::Inline(rows)),
        }
    }
}

/// A `result_set` column expectation (identity: the `AS` alias, else the
/// 0-based index — ITS-REST `ResultSetColumn.yaml`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnSpec {
    /// The expected column name (alias).
    pub name: String,
}

/// A typed assertion (tag: the `assert` field).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "assert", rename_all = "snake_case", deny_unknown_fields)]
pub enum Assertion {
    /// Body parses as the named RM type and validates against the ITS schema
    /// for the active format.
    InstanceOf {
        rm_type: String,
        #[serde(default)]
        format: Option<FormatName>,
    },
    /// RM-path-addressed field check; exactly one predicate.
    Field {
        path: String,
        #[serde(default)]
        equals: Option<TemplatedValue>,
        /// The server-set predicate: the stored value must differ from a
        /// client-supplied one (ITS-REST overview `Requests_and_responses`:
        /// `AUDIT_DETAILS.time_committed` is always server-set).
        #[serde(default)]
        not_equals: Option<TemplatedValue>,
        #[serde(default)]
        exists: Option<bool>,
        #[serde(default)]
        absent: Option<bool>,
        #[serde(default)]
        matches: Option<String>,
    },
    /// The master07 "content check": retrieved equals committed, modulo the
    /// declared server-assigned set — normative per operation, never
    /// runner-chosen.
    Equivalent {
        to: EquivalentTarget,
        #[serde(default)]
        ignoring: IgnoreList,
    },
    /// `ORIGINAL_VERSION.signature` facts (RM common §`change_control`,
    /// `Digital Signature`: the signature is over the canonical form of the
    /// version data; verification behaviour is conformance, algorithm
    /// strength is not). The wire seam is the versioned-object version read
    /// (the `ORIGINAL_VERSION` envelope), resolved by the interpreter.
    Signature {
        #[serde(default)]
        of: Option<SingleRef>,
        #[serde(default)]
        for_each: Option<SingleRef>,
        /// The version carries a non-empty signature.
        #[serde(default)]
        present: Option<bool>,
        /// The signature verifies over the canonical version form against
        /// the statement-declared key material.
        #[serde(default)]
        verifiable: Option<bool>,
        /// The stored signature equals a known value (the client-verbatim
        /// storage rule for imported/committed signed versions).
        #[serde(default)]
        equals: Option<TemplatedValue>,
    },
    /// RM versioning facts.
    Version {
        #[serde(default)]
        of: Option<SingleRef>,
        #[serde(default)]
        for_each: Option<SingleRef>,
        #[serde(default)]
        change_type: Option<ChangeType>,
        #[serde(default)]
        lifecycle_state: Option<String>,
        #[serde(default)]
        count: Option<u64>,
        #[serde(default)]
        uid_pattern: Option<Template>,
    },
    /// AQL results under the normative equivalence rules.
    ResultSet {
        #[serde(rename = "match")]
        match_mode: ResultSetMatch,
        #[serde(default)]
        rows: Option<RowsSpec>,
        #[serde(default)]
        count: Option<u64>,
        #[serde(default)]
        columns: Option<Vec<ColumnSpec>>,
    },
    /// Values captured across rows are pairwise distinct. Aggregate:
    /// evaluated once after all rows; requires `iteration: single_pass`.
    Unique { over: SingleRef, aggregate: bool },
    /// Scalar service returns (no RM body).
    Returns {
        #[serde(default)]
        equals: Option<serde_json::Value>,
        #[serde(default)]
        matches: Option<String>,
    },
    /// Informative only — never a pass/fail criterion.
    MessageExemplar { text: String },
    /// A prose postcondition whose machine verification lives in a linked
    /// case or an in-case verification step.
    State {
        text: String,
        #[serde(default)]
        verified_by: Option<CaseId>,
    },
}

impl Assertion {
    /// Structural invariants beyond serde shape.
    ///
    /// # Errors
    /// Returns a message when a predicate-count or aggregate invariant is
    /// violated.
    pub fn check_invariants(&self) -> Result<(), String> {
        match self {
            Self::Field { .. } => self.check_field_invariants(),
            Self::Version { .. } => self.check_version_invariants(),
            other => other.check_other_invariants(),
        }
    }

    fn check_field_invariants(&self) -> Result<(), String> {
        match self {
            Self::Field {
                equals,
                not_equals,
                exists,
                absent,
                matches,
                path,
            } => {
                let predicates = usize::from(equals.is_some())
                    + usize::from(not_equals.is_some())
                    + usize::from(exists.is_some())
                    + usize::from(absent.is_some())
                    + usize::from(matches.is_some());
                if predicates != 1 {
                    return Err(format!(
                        "field assertion on {path:?} must carry exactly one of equals | exists | absent | matches"
                    ));
                }
                if let Some(re) = matches {
                    regex::Regex::new(re).map_err(|e| format!("field matches regex: {e}"))?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_version_invariants(&self) -> Result<(), String> {
        match self {
            Self::Version {
                of,
                for_each,
                change_type,
                lifecycle_state,
                count,
                uid_pattern,
            } => {
                if of.is_some() && for_each.is_some() {
                    return Err(
                        "version assertion: `of` and `for_each` are mutually exclusive".to_owned(),
                    );
                }
                if change_type.is_none()
                    && lifecycle_state.is_none()
                    && count.is_none()
                    && uid_pattern.is_none()
                {
                    return Err("version assertion carries no fact (change_type | lifecycle_state | count | uid_pattern)".to_owned());
                }
                if count.is_none() && of.is_none() && for_each.is_none() {
                    return Err(
                        "version assertion needs `of`/`for_each` (only `count` may stand alone)"
                            .to_owned(),
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_other_invariants(&self) -> Result<(), String> {
        match self {
            Self::Signature {
                of,
                for_each,
                present,
                verifiable,
                equals,
            } => {
                if of.is_some() == for_each.is_some() {
                    return Err(
                        "signature assertion needs exactly one of `of` | `for_each`".to_owned()
                    );
                }
                if present.is_none() && verifiable.is_none() && equals.is_none() {
                    return Err(
                        "signature assertion carries no fact (present | verifiable | equals)"
                            .to_owned(),
                    );
                }
            }
            Self::ResultSet {
                match_mode,
                rows,
                count,
                ..
            } => match match_mode {
                ResultSetMatch::Count => {
                    if count.is_none() {
                        return Err("result_set match:count requires `count`".to_owned());
                    }
                }
                _ => {
                    if rows.is_none() {
                        return Err("result_set requires `rows` (except match:count)".to_owned());
                    }
                }
            },
            Self::Unique { aggregate, over } => {
                if !aggregate {
                    return Err(
                        "unique is defined only as an aggregate assertion (aggregate: true)"
                            .to_owned(),
                    );
                }
                if !matches!(
                    over.0,
                    ValueRef::Capture {
                        optional: false,
                        ..
                    }
                ) {
                    return Err("unique `over` must be a ${<capture>} reference".to_owned());
                }
            }
            Self::Returns { equals, matches } => {
                if equals.is_some() == matches.is_some() {
                    return Err("returns must carry exactly one of equals | matches".to_owned());
                }
                if let Some(re) = matches {
                    regex::Regex::new(re).map_err(|e| format!("returns matches regex: {e}"))?;
                }
            }
            Self::InstanceOf { .. }
            | Self::Equivalent { .. }
            | Self::MessageExemplar { .. }
            | Self::State { .. }
            | Self::Field { .. }
            | Self::Version { .. } => {}
        }
        Ok(())
    }

    /// Whether this assertion is evaluated once after all rows.
    #[must_use]
    pub fn is_aggregate(&self) -> bool {
        matches!(self, Self::Unique { .. })
    }
}

/// Every `${…}` reference used by an assertion (for the closed-grammar and
/// resolution checks).
#[must_use]
pub fn assertion_refs(assertion: &Assertion) -> Vec<ValueRef> {
    let mut out: Vec<ValueRef> = Vec::new();
    match assertion {
        Assertion::Field {
            equals, not_equals, ..
        } => {
            for v in [equals, not_equals].into_iter().flatten() {
                out.extend(v.refs().into_iter().cloned());
            }
        }
        Assertion::Equivalent { to, .. } => {
            if let EquivalentTarget::Ref(r) = to {
                out.push(r.clone());
            }
        }
        Assertion::Version {
            of,
            for_each,
            uid_pattern,
            ..
        } => {
            if let Some(SingleRef(r)) = of {
                out.push(r.clone());
            }
            if let Some(SingleRef(r)) = for_each {
                out.push(r.clone());
            }
            if let Some(t) = uid_pattern {
                out.extend(t.refs().cloned());
            }
        }
        Assertion::ResultSet { rows, .. } => {
            if let Some(RowsSpec::From(r)) = rows {
                out.push(r.clone());
            }
        }
        Assertion::Signature {
            of,
            for_each,
            equals,
            ..
        } => {
            if let Some(SingleRef(r)) = of {
                out.push(r.clone());
            }
            if let Some(SingleRef(r)) = for_each {
                out.push(r.clone());
            }
            if let Some(v) = equals {
                out.extend(v.refs().into_iter().cloned());
            }
        }
        Assertion::Unique { over, .. } => out.push(over.0.clone()),
        Assertion::InstanceOf { .. }
        | Assertion::Returns { .. }
        | Assertion::MessageExemplar { .. }
        | Assertion::State { .. } => {}
    }
    out
}

/// Parse-time hook so `RefError` conversion stays local to this module.
impl From<RefError> for String {
    fn from(e: RefError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(v: serde_json::Value) -> Assertion {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn pilot_assertions_parse() {
        let a = parse(serde_json::json!({
            "assert": "unique", "over": "${new_ehr_id}", "aggregate": true
        }));
        assert!(a.check_invariants().is_ok());
        assert!(a.is_aggregate());

        let a = parse(serde_json::json!({
            "assert": "version", "of": "${v2_uid}",
            "uid_pattern": "${versioned_object_uid}::<system>::2"
        }));
        assert!(a.check_invariants().is_ok());

        let a = parse(serde_json::json!({ "assert": "version", "count": 2 }));
        assert!(a.check_invariants().is_ok());

        let a = parse(serde_json::json!({
            "assert": "equivalent", "to": "committed", "ignoring": "server_assigned"
        }));
        assert!(a.check_invariants().is_ok());

        let a = parse(serde_json::json!({
            "assert": "result_set", "match": "ordered",
            "rows": { "from": "${ds:cnf.set.bp-10#magnitude_ge_140_by_uid}" },
            "columns": [{ "name": "uid" }]
        }));
        assert!(a.check_invariants().is_ok());
    }

    #[test]
    fn invariants_bite() {
        let a = parse(serde_json::json!({ "assert": "version", "of": "${v}" }));
        assert!(a.check_invariants().is_err()); // no fact

        let a = parse(serde_json::json!({
            "assert": "field", "path": "x", "exists": true, "absent": true
        }));
        assert!(a.check_invariants().is_err()); // two predicates

        let a = parse(serde_json::json!({
            "assert": "unique", "over": "${x}", "aggregate": false
        }));
        assert!(a.check_invariants().is_err());

        assert!(
            serde_json::from_value::<Assertion>(serde_json::json!({
                "assert": "equivalent", "to": "${row.thing}"
            }))
            .is_err()
        ); // equivalent target must be ds/capture

        assert!(
            serde_json::from_value::<Assertion>(serde_json::json!({
                "assert": "totally_new", "x": 1
            }))
            .is_err()
        ); // closed vocabulary
    }
}
