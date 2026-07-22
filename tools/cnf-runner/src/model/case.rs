//! The case core — one file per case, protocol-neutral
//! (CNF 2.0 artifact-set design; shapes extracted from
//! `CNF platform_test_schedule master03/04/06/07/08/09/15–17`).

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

use crate::ids::{
    AmbiguityId, CapabilityName, CaptureName, CaseId, CorpusKey, InstanceName, OptionTag,
    SmOperationRef,
};
use crate::model::assertion::Assertion;
use crate::model::value::TemplatedValue;
use crate::refgrammar::CaptureValueSource;
use crate::vocab::{
    CaseKind, CaseStatus, Component, FormatName, Iteration, OutcomeKind, ServerState,
    SpecComponent, Tier,
};

/// Spec-version applicability ranges (`applies:`); the range grammar is the
/// Cargo/semver requirement syntax.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applies {
    #[serde(default)]
    pub rm: Option<VersionRange>,
    #[serde(default)]
    pub base: Option<VersionRange>,
    #[serde(default)]
    pub am: Option<VersionRange>,
    #[serde(default)]
    pub aql: Option<VersionRange>,
    #[serde(default)]
    pub its_rest: Option<VersionRange>,
    #[serde(default)]
    pub term: Option<VersionRange>,
}

impl Applies {
    /// The declared (component, range) pairs.
    #[must_use]
    pub fn entries(&self) -> Vec<(SpecComponent, &VersionRange)> {
        [
            (SpecComponent::Rm, &self.rm),
            (SpecComponent::Base, &self.base),
            (SpecComponent::Am, &self.am),
            (SpecComponent::Aql, &self.aql),
            (SpecComponent::ItsRest, &self.its_rest),
            (SpecComponent::Term, &self.term),
        ]
        .into_iter()
        .filter_map(|(c, r)| r.as_ref().map(|r| (c, r)))
        .collect()
    }
}

/// A semver requirement, validated at parse.
#[derive(Debug, Clone)]
pub struct VersionRange {
    raw: String,
    req: semver::VersionReq,
}

impl VersionRange {
    /// The authored requirement text.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The parsed requirement.
    #[must_use]
    pub fn req(&self) -> &semver::VersionReq {
        &self.req
    }
}

impl<'de> Deserialize<'de> for VersionRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let req = semver::VersionReq::parse(&raw)
            .map_err(|e| D::Error::custom(format!("applies range {raw:?}: {e}")))?;
        Ok(Self { raw, req })
    }
}

/// The `requires.ehr` precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EhrRequirement {
    /// No EHR is provisioned.
    None,
    /// An EHR exists (mints `${ehr_id}`), with the stated commit state.
    Exists { commits: CommitState },
}

/// Commit-state qualifier of a provisioned EHR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitState {
    None,
    Any,
}

impl<'de> Deserialize<'de> for EhrRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Qualified {
            commits: CommitState,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Word(String),
            Qualified(Qualified),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Word(w) if w == "none" => Ok(Self::None),
            Raw::Word(w) => Err(D::Error::custom(format!(
                "requires.ehr must be `none` or {{ commits: none | any }}, got {w:?}"
            ))),
            Raw::Qualified(q) => Ok(Self::Exists { commits: q.commits }),
        }
    }
}

/// The `requires.directory` precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryRequirement {
    None,
    /// A FOLDER tree provisioned in the EHR from the named corpus set.
    Tree(CorpusKey),
}

impl<'de> Deserialize<'de> for DirectoryRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s == "none" {
            return Ok(Self::None);
        }
        CorpusKey::parse(&s)
            .map(Self::Tree)
            .map_err(D::Error::custom)
    }
}

/// Typed prerequisites — the schedule's precondition vocabulary. Every
/// provisioned object mints a named handle usable as a flow variable.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    /// "The server should be empty (no EHRs, no commits, no OPTs)".
    #[serde(default)]
    pub server: Option<ServerState>,
    /// Corpus template keys provisioned before the flow.
    #[serde(default)]
    pub templates: Vec<CorpusKey>,
    /// EHR provisioning; `Exists` mints `${ehr_id}`.
    #[serde(default)]
    pub ehr: Option<EhrRequirement>,
    /// A FOLDER tree provisioned in the EHR (master09).
    #[serde(default)]
    pub directory: Option<DirectoryRequirement>,
    /// Corpus set keys pre-committed into the EHR by the runner (bulk setup
    /// is precondition state, never an un-anchored flow call).
    #[serde(default)]
    pub commit: Vec<CorpusKey>,
    /// Multi-instance cases state `requires` per named instance.
    #[serde(default)]
    pub instances: Option<std::collections::BTreeMap<InstanceName, Requires>>,
}

impl Requires {
    /// The capture handles this block mints (`ehr_id` when an EHR is
    /// provisioned).
    #[must_use]
    pub fn minted_handles(&self) -> Vec<CaptureName> {
        let mut handles = Vec::new();
        if matches!(self.ehr, Some(EhrRequirement::Exists { .. }))
            && let Ok(handle) = CaptureName::parse("ehr_id")
        {
            handles.push(handle);
        }
        // a provisioned FOLDER tree publishes its created VERSION uid
        if self.directory.is_some()
            && let Ok(handle) = CaptureName::parse("directory_version_uid")
        {
            handles.push(handle);
        }
        handles
    }
}

/// A parameter-matrix cell: a reserved sentinel or a literal.
#[derive(Debug, Clone, PartialEq)]
pub enum MatrixCell {
    /// Omit the field entirely.
    Absent,
    /// Synthesize a valid value via the case's recipe.
    Provided,
    /// JSON null.
    Null,
    /// A literal value (strings validated as templates elsewhere; sentinel
    /// words are reserved and never literals).
    Literal(serde_json::Value),
}

impl<'de> Deserialize<'de> for MatrixCell {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match &value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::String(s) if s == "absent" => Self::Absent,
            serde_json::Value::String(s) if s == "provided" => Self::Provided,
            _ => Self::Literal(value),
        })
    }
}

/// The inline value matrix (master06-style).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub columns: Vec<String>,
    /// Inline rows; each row binds `${row.<column>}`.
    #[serde(default)]
    pub rows: Vec<Vec<MatrixCell>>,
    /// Optional bulk-row external table for large GENERATED matrices
    /// (produced by a corpus recipe, never hand-edited).
    #[serde(default)]
    pub rows_from: Option<String>,
}

/// One fixture-set entry (master04-style external-fixture iteration).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureEntry {
    pub data_set: CorpusKey,
    pub expected: OutcomeKind,
    #[serde(default)]
    pub defect: Option<String>,
    #[serde(default)]
    pub spec_ref: Option<String>,
}

/// The data-set dimension: one mechanism serves the functional matrices and
/// the fixture sets. A "test" = one case × one data set
/// (`CNF platform_test_schedule master03-overview.adoc`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub iteration: Iteration,
    #[serde(default)]
    pub matrix: Option<Matrix>,
    #[serde(default)]
    pub fixture_set: Option<Vec<FixtureEntry>>,
}

/// The step expectation: exactly one outcome kind, or the per-fixture
/// override `${fixture.expected}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectSpec {
    Kind(OutcomeKind),
    /// `${fixture.expected}` — resolved per fixture-set row.
    FixtureExpected,
}

impl<'de> Deserialize<'de> for ExpectSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s == "${fixture.expected}" {
            return Ok(Self::FixtureExpected);
        }
        OutcomeKind::from_token(&s).map(Self::Kind).ok_or_else(|| {
            D::Error::custom(format!(
                "expect must be an outcome kind or ${{fixture.expected}}, got {s:?}"
            ))
        })
    }
}

/// One ordered flow step.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowStep {
    pub step: u32,
    /// SM operation: short form resolves against `sm_operation`'s interface;
    /// a full `I_X.y` form addresses another interface.
    pub call: String,
    /// Instance selector (default `sut`); Enterprise dual-instance cases
    /// address ixit-declared instances.
    #[serde(default)]
    pub on: Option<InstanceName>,
    /// Substep variant tag (the schedule's `1.1`, `3.2` iteration sources).
    #[serde(default)]
    pub variant: Option<String>,
    /// Per-step format role (intrinsic-format cases only).
    #[serde(default)]
    pub format: Option<FormatName>,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub with: Option<Vec<(String, TemplatedValue)>>,
    pub expect: ExpectSpec,
    /// Logical captures; bindings map them to wire locations.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub capture: Option<Vec<(CaptureName, CaptureValueSource)>>,
    /// Post-step typed assertions.
    #[serde(default, rename = "assert")]
    pub assertions: Vec<Assertion>,
}

/// The content decision-table context (the OPT carrying the constraint under
/// test + the constrained node path).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstraintContext {
    pub template: CorpusKey,
    pub path: String,
    /// The decision-table columns that are *constraint axis* — cells that
    /// describe the archetype/template constraint the row bakes (e.g.
    /// `cardinality`, `month_validity`, `range.lower`, `slot_type`,
    /// `state_existence`), as opposed to the *instance axis* (the genuine RM
    /// attributes of the committed value). When non-empty the runner
    /// synthesizes one OPT per row from these cells (a per-row constraint
    /// template) rather than committing every row against one baked template;
    /// the named columns flow into the synthesizer and are excluded from the
    /// committed instance. Empty (the default) keeps the single-template
    /// model: the constraint is constant across rows and baked into
    /// `template`. (No openEHR spec governs this — our own corpus-authoring
    /// design; the constraint shapes are grounded in AM AOM1.4.)
    #[serde(default)]
    pub constraint_columns: Vec<String>,
}

/// A content decision table (master15–17 shape). Each row is one committed
/// instance + `expected: accepted | rejected` + `violates: […]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// A content row verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowVerdict {
    Accepted,
    Rejected,
}

/// One case file — the Abstract Test Suite unit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseCore {
    pub id: CaseId,
    pub kind: CaseKind,
    #[serde(default)]
    pub status: CaseStatus,
    pub component: Component,
    /// Functional cases: the SM anchor.
    #[serde(default)]
    pub sm_operation: Option<SmOperationRef>,
    /// Content cases: the RM/AM class under test.
    #[serde(default)]
    pub rm_class: Option<String>,
    /// The ISO/IEC 9646 test purpose — one narrow conformance requirement.
    pub test_purpose: String,
    /// The schedule's Description row.
    pub description: String,
    /// Citations (component + document + section); link-checked.
    pub spec_refs: Vec<String>,
    #[serde(default)]
    pub applies: Applies,
    /// Non-version run conditions, each spec-cited; a failed guard ⇒
    /// `not-applicable` with citation.
    #[serde(default)]
    pub guards: Vec<String>,
    /// The verdict-bearing capability names — kept MINIMAL (a case failure
    /// marks every listed capability Failed).
    #[serde(default)]
    pub capabilities: Vec<CapabilityName>,
    /// Informative coverage tags.
    #[serde(default)]
    pub exercises: Vec<CapabilityName>,
    /// Profile tier(s) — derivable from the capability matrix, carried for
    /// readability; consistency is CI-checked.
    #[serde(default)]
    pub profiles: Vec<Tier>,
    /// For sibling cases realizing an ambiguity-register implementation
    /// choice: the option tag the ICS `options` declaration selects.
    #[serde(default)]
    pub option: Option<OptionTag>,
    /// Case-level format axis (cases parameterized over format).
    #[serde(default)]
    pub formats: Vec<FormatName>,
    #[serde(default)]
    pub requires: Requires,
    #[serde(default)]
    pub parameters: Option<Parameters>,
    /// Ordered steps (functional cases).
    #[serde(default)]
    pub flow: Vec<FlowStep>,
    /// Content cases: the constraint context + decision table.
    #[serde(default)]
    pub constraint_context: Option<ConstraintContext>,
    #[serde(default)]
    pub decision_table: Option<DecisionTable>,
    /// Typed postconditions; default evaluation per parameter row,
    /// `aggregate: true` once after all rows.
    #[serde(default)]
    pub postconditions: Vec<Assertion>,
    /// Cases verifying this case's deeper postconditions through separate
    /// reads (the master06 create→get pattern).
    #[serde(default)]
    pub verified_by: Vec<CaseId>,
    /// Ambiguity-register entries this case is subject to.
    #[serde(default)]
    pub ambiguities: Vec<AmbiguityId>,
    /// Corpus manifest keys used (in addition to `parameters`).
    #[serde(default)]
    pub data_sets: Vec<CorpusKey>,
}

// Custom deserialization for `with`/`capture` map fields preserving order:
// YAML mappings arrive as JSON objects; serde_json's preserve_order keeps
// authored order, and Vec<(K, V)> makes that explicit in the type.
impl FlowStep {
    /// The step's capture entries (empty when none).
    #[must_use]
    pub fn captures(&self) -> &[(CaptureName, CaptureValueSource)] {
        self.capture.as_deref().unwrap_or_default()
    }

    /// The step's `with` entries (empty when none).
    #[must_use]
    pub fn with_entries(&self) -> &[(String, TemplatedValue)] {
        self.with.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_parses_the_schedule_vocabulary() {
        let r: Requires = serde_json::from_value(serde_json::json!({
            "server": "any",
            "templates": ["cnf.opt.minimal_event"],
            "ehr": { "commits": "none" },
            "commit": ["cnf.set.bp-10"]
        }))
        .unwrap();
        assert_eq!(r.server, Some(ServerState::Any));
        assert!(matches!(
            r.ehr,
            Some(EhrRequirement::Exists {
                commits: CommitState::None
            })
        ));
        assert_eq!(r.minted_handles().len(), 1);

        let r: Requires = serde_json::from_value(serde_json::json!({ "ehr": "none" })).unwrap();
        assert!(matches!(r.ehr, Some(EhrRequirement::None)));
        assert!(r.minted_handles().is_empty());

        assert!(serde_json::from_value::<Requires>(serde_json::json!({ "ehr": "maybe" })).is_err());
        assert!(serde_json::from_value::<Requires>(serde_json::json!({ "srv": "empty" })).is_err());
    }

    #[test]
    fn matrix_cells_distinguish_sentinels() {
        let m: Matrix = serde_json::from_value(serde_json::json!({
            "columns": ["ehr_status", "is_queryable"],
            "rows": [["absent", "-"], ["provided", true], [null, 1.5]]
        }))
        .unwrap();
        assert!(matches!(m.rows[0][0], MatrixCell::Absent));
        assert!(matches!(m.rows[0][1], MatrixCell::Literal(_)));
        assert!(matches!(m.rows[1][0], MatrixCell::Provided));
        assert!(matches!(
            m.rows[1][1],
            MatrixCell::Literal(serde_json::Value::Bool(true))
        ));
        assert!(matches!(m.rows[2][0], MatrixCell::Null));
    }

    #[test]
    fn expect_spec_is_closed() {
        assert!(matches!(
            serde_json::from_value::<ExpectSpec>(serde_json::json!("created")).unwrap(),
            ExpectSpec::Kind(OutcomeKind::Created)
        ));
        assert!(matches!(
            serde_json::from_value::<ExpectSpec>(serde_json::json!("${fixture.expected}")).unwrap(),
            ExpectSpec::FixtureExpected
        ));
        assert!(serde_json::from_value::<ExpectSpec>(serde_json::json!("http_201")).is_err());
    }
}
