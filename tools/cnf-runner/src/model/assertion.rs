// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The typed assertion vocabulary (`flow[].assert` + `postconditions`).
//!
//! Ten assertion forms, closed by schedule release. Semantics per the
//! CNF 2.0 artifact-set design: `equivalent` is the master07 "content
//! check" with normative ignore-sets; `version` asserts RM versioning facts
//! (`RM common §change_control`); `result_set` compares under the normative
//! AQL `RESULT_SET` equivalence rules (QUERY master03/04 + the ITS-REST query
//! schemas); `xml_root` judges a served canonical-XML document against the
//! published ITS-XML element declarations (ITS-REST overview `Resources.md`
//! §"XML Format"); `unique` is aggregate (evaluated once after all rows);
//! `message_exemplar` is informative only, never pass/fail.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

use crate::ids::CaseId;
use crate::model::value::TemplatedValue;
use crate::refgrammar::{RefError, Template, ValueRef};
use crate::vocab::{ChangeType, FormatName, IgnoreSetName, ResultSetMatch, XmlNamespace};

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
        /// The RM class name the body must parse as.
        rm_type: String,
        /// The wire format to parse in; defaults to the step's active format.
        #[serde(default)]
        format: Option<FormatName>,
    },
    /// RM-path-addressed field check; exactly one predicate.
    Field {
        /// The RM path addressing the field under test.
        path: String,
        /// The value the field must equal.
        #[serde(default)]
        equals: Option<TemplatedValue>,
        /// The server-set predicate: the stored value must differ from a
        /// client-supplied one (ITS-REST overview `Requests_and_responses`:
        /// `AUDIT_DETAILS.time_committed` is always server-set).
        #[serde(default)]
        not_equals: Option<TemplatedValue>,
        /// The field must be present (`true`) at the path.
        #[serde(default)]
        exists: Option<bool>,
        /// The field must be absent (`true`) at the path.
        #[serde(default)]
        absent: Option<bool>,
        /// A regex the field's serialized value must match.
        #[serde(default)]
        matches: Option<String>,
    },
    /// The master07 "content check": retrieved equals committed, modulo the
    /// declared server-assigned set — normative per operation, never
    /// runner-chosen.
    Equivalent {
        /// What the retrieved body is compared against.
        to: EquivalentTarget,
        /// The declared server-assigned paths excluded from the comparison.
        #[serde(default)]
        ignoring: IgnoreList,
    },
    /// `ORIGINAL_VERSION.signature` facts (RM common §`change_control`,
    /// `Digital Signature`: the signature is over the canonical form of the
    /// version data; verification behaviour is conformance, algorithm
    /// strength is not). The wire seam is the versioned-object version read
    /// (the `ORIGINAL_VERSION` envelope), resolved by the interpreter.
    Signature {
        /// The single version the assertion judges.
        #[serde(default)]
        of: Option<SingleRef>,
        /// A captured set whose every member the assertion judges.
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
        /// The stored signature differs from a known (non-empty) value — the
        /// distinct-signature-per-version fact: the signature is computed over
        /// the version's canonical form, which includes `uid`, so two distinct
        /// versions necessarily carry distinct signatures (RM common
        /// `master06-change_control_package.adoc` §Digital Signature — "the
        /// entire Version object (… the signature attribute will be Void …)"
        /// is serialised and hashed; `version.adoc` `canonical_form`: "all
        /// attributes except signature").
        #[serde(default)]
        distinct_from: Option<TemplatedValue>,
    },
    /// RM versioning facts.
    Version {
        /// The single version the assertion judges.
        #[serde(default)]
        of: Option<SingleRef>,
        /// A captured set whose every member the assertion judges.
        #[serde(default)]
        for_each: Option<SingleRef>,
        /// The `commit_audit.change_type` the version must carry.
        #[serde(default)]
        change_type: Option<ChangeType>,
        /// The `lifecycle_state` value the version must carry.
        #[serde(default)]
        lifecycle_state: Option<String>,
        /// The exact number of versions the versioned object must hold.
        #[serde(default)]
        count: Option<u64>,
        /// A template the version's `uid` must match once resolved.
        #[serde(default)]
        uid_pattern: Option<Template>,
    },
    /// AQL results under the normative equivalence rules.
    ResultSet {
        /// How the expected rows are compared against the served ones.
        #[serde(rename = "match")]
        match_mode: ResultSetMatch,
        /// The expected rows (inline, or a reference to a corpus row set).
        #[serde(default)]
        rows: Option<RowsSpec>,
        /// The exact row count the result set must carry.
        #[serde(default)]
        count: Option<u64>,
        /// The expected columns, identified by `AS` alias.
        #[serde(default)]
        columns: Option<Vec<ColumnSpec>>,
    },
    /// Values captured across rows are pairwise distinct. Aggregate:
    /// evaluated once after all rows; requires `iteration: single_pass`.
    Unique {
        /// The captured value whose per-row instances must be pairwise distinct.
        over: SingleRef,
        /// Evaluate once after every row instead of per row.
        aggregate: bool,
    },
    /// Scalar service returns (no RM body).
    Returns {
        /// The exact value the scalar return must equal.
        #[serde(default)]
        equals: Option<serde_json::Value>,
        /// A regex the serialized body must match.
        #[serde(default)]
        matches: Option<String>,
        /// A regex the serialized body must NOT match — the negative
        /// containment predicate (e.g. a listing that must EXCLUDE a
        /// superseded row). Composes with `matches`: both are checked.
        #[serde(default)]
        omits: Option<String>,
    },
    /// The served canonical-XML document's ROOT element, judged against the
    /// published ITS-XML schemas: its local name and the namespace it is
    /// qualified with.
    ///
    /// The released ground is one sentence, ITS-REST overview `Resources.md`
    /// §"XML Format": "When resources are serialized in **canonical XML**
    /// format, both request payloads and responses MUST conform to the
    /// [published XSDs]". Conformance to a schema is not satisfied by matching
    /// a complexType: the instance's root must be a globally declared element
    /// of the schema set, and since every ITS-XML schema declares
    /// `elementFormDefault="qualified"` over a `targetNamespace`, that element
    /// is namespace-qualified. So the two facts this assertion carries are the
    /// two the MUST fixes for a resource the schemas DO publish an element for
    /// — nothing more. A resource with no published element is out of scope by
    /// construction (register AMB-167), and no case may assert a root for one.
    ///
    /// `matches`-style regex over the raw body cannot express this: it cannot
    /// tell the root element from a descendant, and it cannot resolve a prefix
    /// to its namespace URI, so a `<oe:composition xmlns:oe="…">` document and
    /// an unqualified `<composition>` document are indistinguishable to it.
    ///
    /// Where the published element's declared type is ABSTRACT, the same MUST
    /// fixes a third fact: `xsi:type`. XML Schema Part 1 forbids an element
    /// instance from using an abstract type directly — the instance must
    /// select a non-abstract derived type with `xsi:type`
    /// (<https://www.w3.org/TR/xmlschema-1/#xsi_type>, §2.6.1 + §3.4.6). Two
    /// published document elements are declared that way, identically in both
    /// vendored lineages: `<xs:element name="version" type="VERSION"/>` over
    /// `<xs:complexType name="VERSION" abstract="true">`
    /// (`crates/openehr-its/schemas/xml/its-xml-1.0.2-nsv1/ALL/Version.xsd`,
    /// `its-xml-2.0.0-nsv2/RM/latest/documents/Version.xsd` +
    /// `RM/latest/Common.xsd`) and `<xs:element name="items"
    /// type="LOCATABLE"/>` over the abstract `LOCATABLE` (`ALL/Structure.xsd`,
    /// `RM/latest/documents/Structure.xsd` + `RM/latest/Common.xsd`). On such a
    /// root the concrete class is the ONLY thing that distinguishes, say, an
    /// `ORIGINAL_VERSION` response from an `IMPORTED_VERSION` one, so `xsi_type`
    /// is how a row judges it. It is asserted only where the schemas declare
    /// the type abstract — on a concretely-typed element the attribute is
    /// decoration, not dispatch, and no released sentence requires it.
    XmlRoot {
        /// The expected root element's local name (a globally declared element
        /// of the published XSDs).
        name: String,
        /// The expected namespace of the root element; omitted only where a
        /// row deliberately judges the name alone.
        #[serde(default)]
        namespace: Option<XmlNamespace>,
        /// The LOCAL name of the concrete type the root must name with
        /// `xsi:type` — asserted only on a published element whose declared
        /// type is abstract (see the form's doc comment). The attribute value
        /// is a `QName`, so the assertion resolves its prefix through the
        /// document's in-scope bindings (an unprefixed `QName` resolves against
        /// the DEFAULT namespace — the `QName`-in-content rule) and compares the
        /// local part; when the row also asserts `namespace`, the type's own
        /// namespace must satisfy the same expectation, because the ITS-XML
        /// complexTypes are declared in each schema's `targetNamespace`.
        #[serde(default)]
        xsi_type: Option<String>,
    },
    /// Informative only — never a pass/fail criterion.
    MessageExemplar {
        /// The exemplar message text, recorded for readers of the schedule.
        text: String,
    },
    /// A prose postcondition whose machine verification lives in a linked
    /// case or an in-case verification step.
    State {
        /// The postcondition in prose.
        text: String,
        /// The case that machine-verifies this postcondition, if separate.
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
                distinct_from,
            } => {
                if of.is_some() == for_each.is_some() {
                    return Err(
                        "signature assertion needs exactly one of `of` | `for_each`".to_owned()
                    );
                }
                if present.is_none()
                    && verifiable.is_none()
                    && equals.is_none()
                    && distinct_from.is_none()
                {
                    return Err(
                        "signature assertion carries no fact (present | verifiable | equals | distinct_from)"
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
            Self::Returns {
                equals,
                matches,
                omits,
            } => {
                // Exactly one positive predicate (equals | matches); `omits`
                // is the composable negative side (alone or beside matches,
                // never beside equals — whole-value equality already pins the
                // body).
                match (equals.is_some(), matches.is_some(), omits.is_some()) {
                    (true, false, false) | (false, true, _) | (false, false, true) => {}
                    _ => {
                        return Err(
                            "returns must carry exactly one of equals | matches [+ omits] | omits"
                                .to_owned(),
                        );
                    }
                }
                if let Some(re) = matches {
                    regex::Regex::new(re).map_err(|e| format!("returns matches regex: {e}"))?;
                }
                if let Some(re) = omits {
                    regex::Regex::new(re).map_err(|e| format!("returns omits regex: {e}"))?;
                }
            }
            Self::XmlRoot {
                name,
                xsi_type,
                namespace: _,
            } => check_xml_root_invariants(name, xsi_type.as_deref())?,
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

/// The structural invariants of an `xml_root` assertion: both names it carries
/// are LOCAL names, because a prefix is a document's own choice and the
/// namespace both resolve against is asserted by `namespace:`.
fn check_xml_root_invariants(name: &str, xsi_type: Option<&str>) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("xml_root assertion needs the expected root element local name".to_owned());
    }
    if name.contains(':') {
        return Err(format!(
            "xml_root name {name:?} must be the LOCAL name — the prefix is a document's own choice and the namespace is asserted by `namespace:`"
        ));
    }
    let Some(rm_type) = xsi_type else {
        return Ok(());
    };
    if rm_type.trim().is_empty() {
        return Err("xml_root xsi_type must name the concrete type, or be omitted".to_owned());
    }
    if rm_type.contains(':') {
        return Err(format!(
            "xml_root xsi_type {rm_type:?} must be the LOCAL name — the QName's prefix is a document's own choice and its namespace rides on `namespace:`"
        ));
    }
    Ok(())
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
            distinct_from,
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
            if let Some(v) = distinct_from {
                out.extend(v.refs().into_iter().cloned());
            }
        }
        Assertion::Unique { over, .. } => out.push(over.0.clone()),
        Assertion::InstanceOf { .. }
        | Assertion::Returns { .. }
        | Assertion::XmlRoot { .. }
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
    fn signature_distinct_from_is_a_fact() {
        // `distinct_from` alone satisfies the carries-a-fact invariant, and its
        // reference participates in reference collection (the capture feeding
        // it is validated like any other).
        let a = parse(serde_json::json!({
            "assert": "signature", "of": "${v2_uid}", "distinct_from": "${sig_first}"
        }));
        assert!(a.check_invariants().is_ok());
        assert!(
            assertion_refs(&a).iter().any(
                |r| matches!(r, ValueRef::Capture { name, .. } if name.as_str() == "sig_first")
            )
        );

        // A signature assertion with no fact at all still bites.
        let a = parse(serde_json::json!({ "assert": "signature", "of": "${v}" }));
        assert!(a.check_invariants().is_err());
    }

    #[test]
    fn xml_root_takes_a_local_name_and_a_published_namespace() {
        let a = parse(serde_json::json!({
            "assert": "xml_root", "name": "composition", "namespace": "openehr-published"
        }));
        assert!(a.check_invariants().is_ok());
        assert!(assertion_refs(&a).is_empty());

        // The name alone is a legal, narrower row.
        let a = parse(serde_json::json!({ "assert": "xml_root", "name": "composition" }));
        assert!(a.check_invariants().is_ok());

        // A prefixed name is a document's own choice, never the assertion's.
        let a = parse(serde_json::json!({ "assert": "xml_root", "name": "oe:composition" }));
        assert!(a.check_invariants().is_err());

        let a = parse(serde_json::json!({ "assert": "xml_root", "name": "  " }));
        assert!(a.check_invariants().is_err());

        // The namespace vocabulary is closed.
        assert!(
            serde_json::from_value::<Assertion>(serde_json::json!({
                "assert": "xml_root", "name": "composition", "namespace": "http://example.org"
            }))
            .is_err()
        );
    }

    /// The `xsi_type` half — for a published element declared over an ABSTRACT
    /// type (`<xs:element name="version" type="VERSION"/>` over
    /// `<xs:complexType name="VERSION" abstract="true">`, `ALL/Version.xsd`),
    /// where XML Schema Part 1 §2.6.1 + §3.4.6 make naming the concrete type
    /// part of conforming to the schema.
    #[test]
    fn xml_root_takes_the_concrete_type_of_an_abstract_root() {
        let a = parse(serde_json::json!({
            "assert": "xml_root", "name": "version",
            "namespace": "openehr-published", "xsi_type": "ORIGINAL_VERSION"
        }));
        assert!(a.check_invariants().is_ok());
        assert!(assertion_refs(&a).is_empty());

        // The QName's prefix is the document's own choice, like the root's.
        let a = parse(serde_json::json!({
            "assert": "xml_root", "name": "version", "xsi_type": "oe:ORIGINAL_VERSION"
        }));
        assert!(a.check_invariants().is_err());

        let a = parse(serde_json::json!({
            "assert": "xml_root", "name": "version", "xsi_type": " "
        }));
        assert!(a.check_invariants().is_err());
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
