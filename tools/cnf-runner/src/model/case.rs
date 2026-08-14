// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The case core — one file per case, protocol-neutral
//! (CNF 2.0 artifact-set design; shapes extracted from
//! `CNF platform_test_schedule master03/04/06/07/08/09/15–17`).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

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
    SpecComponent, Tier, XVersionedClass,
};

/// Spec-version applicability ranges (`applies:`); the range grammar is the
/// Cargo/semver requirement syntax.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applies {
    /// Reference Model versions the case applies to.
    #[serde(default)]
    pub rm: Option<VersionRange>,
    /// BASE component versions the case applies to.
    #[serde(default)]
    pub base: Option<VersionRange>,
    /// Archetype Model versions the case applies to.
    #[serde(default)]
    pub am: Option<VersionRange>,
    /// AQL (QUERY component) versions the case applies to.
    #[serde(default)]
    pub aql: Option<VersionRange>,
    /// ITS-REST API versions the case applies to.
    #[serde(default)]
    pub its_rest: Option<VersionRange>,
    /// Terminology component versions the case applies to.
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

    /// Whether every declared range is satisfied by the party's declared spec
    /// versions.
    ///
    /// An UNDECLARED or unparsable version fails the filter: the party has
    /// not claimed the release the range names, so whatever the range gates —
    /// a case, an operation binding, a version-dated header expectation — is
    /// out of scope for it. This is one rule with one polarity everywhere it
    /// is consulted (`crate::verdict` selection, `crate::run` selection,
    /// `crate::exec::headers` matcher scoping), never an exemption invented
    /// per call site.
    #[must_use]
    pub fn satisfied_by(&self, versions: &crate::party::SpecVersions) -> bool {
        self.entries().into_iter().all(|(component, range)| {
            versions
                .get(component)
                .and_then(|raw| semver::Version::parse(raw).ok())
                .is_some_and(|version| range.req().matches(&version))
        })
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
    Exists {
        /// How much committed content the provisioned EHR must hold.
        commits: CommitState,
    },
}

/// Commit-state qualifier of a provisioned EHR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitState {
    /// The EHR holds no commits at all.
    None,
    /// The EHR's commit history is irrelevant to the case.
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
    /// No directory is provisioned.
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

/// The `requires.party` precondition.
///
/// A demographic PARTY is precondition STATE for the cases that operate ON an
/// existing party without testing its creation — exactly the role
/// `requires.ehr` plays for EHR-scoped cases. Provisioning it here rather than
/// as a flow step is what keeps such a case's FLOW pure: an admin case whose
/// only driven call is `archive_parties` must not also drive the released
/// `create_party`, or the realization it evidences stops being the one it is
/// about (`validate::check_realization_markers`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartyRequirement {
    /// No party is provisioned.
    None,
    /// A PARTY exists, created from the named corpus set (mints
    /// `${party_id}`, its `VERSIONED_OBJECT` uid).
    Exists(CorpusKey),
}

impl<'de> Deserialize<'de> for PartyRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s == "none" {
            return Ok(Self::None);
        }
        CorpusKey::parse(&s)
            .map(Self::Exists)
            .map_err(D::Error::custom)
    }
}

/// The `requires.party_relationship` precondition.
///
/// A demographic `PARTY_RELATIONSHIP` is precondition STATE exactly as
/// [`PartyRequirement`] is: the cases that operate ON an existing relationship
/// — or, like the admin archive's party-only selection, on the boundary
/// between a relationship and a party — must not drive its creation in the
/// flow, or the realization they evidence stops being the one they are about.
///
/// The relationship is provisioned between two REAL parties. RM demographic
/// `master02-demographic_package.adoc` §Party Relationships fixes what the
/// endpoints are: "`PARTY_RELATIONSHIP._source_` and `_target_` are
/// represented by references … `OBJECT_REFs` containing `HIER_OBJECT_IDs` to
/// denote the Version container of a Party, rather than `OBJECT_VERSION_IDs`"
/// — so provisioning creates each endpoint party first and writes its
/// `VERSIONED_OBJECT` uid into the corresponding `PARTY_REF`, rather than
/// committing a relationship whose endpoints name nothing on the server.
///
/// The relationship create itself has NO released wire (register AMB-32; the
/// `party-relationship` `served_extensions` family) — no openEHR spec governs
/// that route, so the requirement is only usable by a party that serves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartyRelationshipRequirement {
    /// No party relationship is provisioned.
    None,
    /// A `PARTY_RELATIONSHIP` exists between two provisioned parties (mints
    /// `${party_relationship_id}`, its `VERSIONED_OBJECT` uid).
    Exists {
        /// The corpus payload of the party at the relationship's `source` end.
        source: CorpusKey,
        /// The corpus payload of the party at the relationship's `target` end.
        target: CorpusKey,
        /// The corpus `PARTY_RELATIONSHIP` payload; its `source`/`target`
        /// `PARTY_REF` ids are replaced by the two minted party uids.
        relationship: CorpusKey,
    },
}

impl<'de> Deserialize<'de> for PartyRelationshipRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Qualified {
            source: CorpusKey,
            target: CorpusKey,
            relationship: CorpusKey,
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
                "requires.party_relationship must be `none` or \
                 {{ source, target, relationship }} corpus keys, got {w:?}"
            ))),
            Raw::Qualified(q) => Ok(Self::Exists {
                source: q.source,
                target: q.target,
                relationship: q.relationship,
            }),
        }
    }
}

/// The `requires.import` precondition — an EHR-Extract already received from
/// another system.
///
/// A version this repository did not create is precondition STATE exactly as
/// [`PartyRequirement`] and [`PartyRelationshipRequirement`] are: the released
/// reads that serve an `IMPORTED_VERSION` (RM common
/// `master06-change_control_package.adoc` §Copying: "An `IMPORTED_VERSION`
/// instance is then created, its `item` set to the received
/// `ORIGINAL_VERSION`") are the SUBJECT of such a case, and driving the import
/// in the flow would make the realization it evidences the import route's
/// rather than the read's (`validate::check_realization_markers`).
///
/// The import itself has NO released wire — ITS-REST 1.1.0 publishes no
/// MESSAGE / EHR-Extract API at all (register AMB-34; the `message-extract`
/// `served_extensions` family) — so, exactly like
/// [`PartyRelationshipRequirement`], the requirement is only usable on a party
/// that serves that family, and `crate::run` records the case
/// not-applicable-with-citation on one that does not.
///
/// Which SM operation provisioning drives follows master06 §Copying's own
/// receiving situations: with an EHR already provisioned
/// (`requires.ehr`) the extract lands in it through `import_ehr_extract`
/// (Cases 2/3 — "an EHR exists" / "previous copies have been made"); with
/// none, `import_ehr` clones a whole EHR (Case 1) and the clone's id is minted
/// as `${ehr_id}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportRequirement {
    /// Nothing is imported.
    None,
    /// An EHR-Extract has been received, and the case is about the versioned
    /// object its named wrapper class carries.
    Received {
        /// The corpus `EXTRACT` payload that was imported.
        extract: CorpusKey,
        /// Which `X_VERSIONED_*` content item of that extract the minted
        /// handles name — an extract carries several at once, so the case
        /// states the one it is about.
        container: XVersionedClass,
    },
}

impl<'de> Deserialize<'de> for ImportRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Qualified {
            extract: CorpusKey,
            container: XVersionedClass,
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
                "requires.import must be `none` or {{ extract, container }}, got {w:?}"
            ))),
            Raw::Qualified(q) => Ok(Self::Received {
                extract: q.extract,
                container: q.container,
            }),
        }
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
    /// A demographic PARTY provisioned before the flow; `Exists` mints
    /// `${party_id}`.
    #[serde(default)]
    pub party: Option<PartyRequirement>,
    /// A demographic `PARTY_RELATIONSHIP` provisioned between two parties
    /// before the flow; `Exists` mints `${party_relationship_id}`.
    #[serde(default)]
    pub party_relationship: Option<PartyRelationshipRequirement>,
    /// An EHR-Extract received from another system before the flow;
    /// `Received` mints `${imported_versioned_object_uid}` +
    /// `${imported_version_uid}` (+ `${imported_branch_version_uid}` when the
    /// extract carries a branch, and `${ehr_id}` for a whole-EHR clone).
    #[serde(default)]
    pub import: Option<ImportRequirement>,
    /// Corpus set keys pre-committed into the EHR by the runner (bulk setup
    /// is precondition state, never an un-anchored flow call).
    #[serde(default)]
    pub commit: Vec<CorpusKey>,
    /// The terminology deployment the case needs (`ixit.terminology`).
    #[serde(default)]
    pub terminology: Option<TerminologyRequirement>,
    /// The openEHR specification generation set the case's expectation rests
    /// on, matched against the addressed instance's `ixit.spec_profile`
    /// declaration at SELECTION time.
    ///
    /// No released openEHR text says which generation set a deployment runs,
    /// so a case that needs one the party does not declare is not-applicable
    /// with that citation (ISO/IEC 9646 test selection) — never driven against
    /// a deployment running the other set.
    #[serde(default)]
    pub spec_profile: Option<crate::ixit::SpecProfile>,
    /// Multi-instance cases state `requires` per named instance.
    #[serde(default)]
    pub instances: Option<std::collections::BTreeMap<InstanceName, Requires>>,
}

/// The terminology deployment a case needs, matched against the addressed
/// instance's `ixit.terminology` declaration at SELECTION time.
///
/// Released ITS-REST 1.1.0 surfaces no terminology resource, so nothing on the
/// wire tells a runner which terminology servers a deployment is wired to or
/// what it does with a value set it cannot resolve. Both are IXIT
/// declarations, and a case that needs one the party does not declare is
/// not-applicable with that citation (ISO/IEC 9646 test selection) — never
/// driven against a guess.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminologyRequirement {
    /// The unresolvable-value-set posture the case's expectation rests on
    /// (register AMB-172). Omitted when the behaviour is posture-independent.
    #[serde(default)]
    pub posture: Option<crate::ixit::TerminologyPosture>,
    /// Terminology namespaces a declared REACHABLE server must answer for.
    #[serde(default)]
    pub served: Vec<String>,
    /// Terminology namespaces a declared UNREACHABLE server must answer for —
    /// the terminology-server-down branch, declared for the whole run rather
    /// than produced by a mid-run reconfiguration.
    #[serde(default)]
    pub unreachable: Vec<String>,
    /// How many DISTINCT reachable servers the `served` namespaces must be
    /// spread across — the N≥2 simultaneous-servers requirement (BASE
    /// master12 §Overview: a deployment binds to several terminologies at
    /// once).
    #[serde(default)]
    pub distinct_servers: Option<usize>,
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
        // a provisioned PARTY publishes its VERSIONED_OBJECT uid
        if matches!(self.party, Some(PartyRequirement::Exists(_)))
            && let Ok(handle) = CaptureName::parse("party_id")
        {
            handles.push(handle);
        }
        // a provisioned PARTY_RELATIONSHIP publishes its VERSIONED_OBJECT uid
        if matches!(
            self.party_relationship,
            Some(PartyRelationshipRequirement::Exists { .. })
        ) && let Ok(handle) = CaptureName::parse("party_relationship_id")
        {
            handles.push(handle);
        }
        // a received EHR-Extract publishes the identity of the versioned
        // object it landed and of the versions inside it — all taken from the
        // extract's own content, since master06 §Copying keeps the received
        // version container's identity ("the `ORIGINAL_VERSION` instance is
        // never modified"). The BRANCH handle binds only when the named
        // container actually carries a branch version; a case referencing it
        // against a trunk-only extract fails loudly at drive time rather than
        // silently reading the trunk.
        if matches!(self.import, Some(ImportRequirement::Received { .. })) {
            for name in [
                "imported_versioned_object_uid",
                "imported_version_uid",
                "imported_branch_version_uid",
            ] {
                if let Ok(handle) = CaptureName::parse(name) {
                    handles.push(handle);
                }
            }
            // Case 1 of master06 §Copying: with no EHR provisioned the import
            // CREATES the EHR, so the clone's id is minted here.
            if !matches!(self.ehr, Some(EhrRequirement::Exists { .. }))
                && let Ok(handle) = CaptureName::parse("ehr_id")
            {
                handles.push(handle);
            }
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
    /// Column names, in order; each names one `${row.<column>}` binding.
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
    /// The corpus manifest key of the payload this row drives.
    pub data_set: CorpusKey,
    /// The outcome a conformant server must produce for this fixture.
    pub expected: OutcomeKind,
    /// For an invalid fixture: what it violates, in one phrase.
    #[serde(default)]
    pub defect: Option<String>,
    /// The spec citation grounding this row's expectation.
    #[serde(default)]
    pub spec_ref: Option<String>,
}

/// The data-set dimension: one mechanism serves the functional matrices and
/// the fixture sets. A "test" = one case × one data set
/// (`CNF platform_test_schedule master03-overview.adoc`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Whether each row re-establishes the preconditions or all rows share
    /// one server state.
    pub iteration: Iteration,
    /// The inline value matrix, when the rows are authored in the case file.
    #[serde(default)]
    pub matrix: Option<Matrix>,
    /// The external-fixture rows, when the rows are corpus payloads.
    #[serde(default)]
    pub fixture_set: Option<Vec<FixtureEntry>>,
}

/// The step expectation: exactly one outcome kind, or the per-fixture
/// override `${fixture.expected}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectSpec {
    /// One fixed outcome kind for every row.
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
    /// 1-based position in the flow; execution follows this order.
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
    /// The SMART `scope` claim this step's principal presents (ITS-REST
    /// `docs/smart_app_launch/master08-scopes.adoc` §Resource Scopes). The
    /// addressed instance must be a `bearer_mint` principal: the runner signs
    /// a fresh access token carrying exactly these scopes, because the CDR is
    /// a resource server and the conformance stack runs no Authorization
    /// Server to obtain one from (master06 §Supported Authentication Flows).
    ///
    /// **Declaring the key at all** — including as an empty list, which is the
    /// scope-less token the fail-closed deny branch needs — marks the step as
    /// SMART-lane, so a party whose ixit declares no `smart` block records the
    /// case not-applicable instead of driving it (ISO/IEC 9646 test
    /// selection).
    #[serde(default)]
    pub scopes: Option<Vec<TemplatedValue>>,
    /// The step's named arguments, in declaration order — each value is a
    /// template resolved against the case's bindings before the call.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub with: Option<Vec<(String, TemplatedValue)>>,
    /// The outcome this step must produce.
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
    /// The corpus key of the operational template carrying the constraint.
    pub template: CorpusKey,
    /// The constrained node's archetype path within that template.
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
    /// Column names, in order (instance axis + constraint axis + verdict).
    pub columns: Vec<String>,
    /// One row per committed instance, cells positional against `columns`.
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// A content row verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowVerdict {
    /// A conformant server must accept the row's instance.
    Accepted,
    /// A conformant server must reject the row's instance.
    Rejected,
}

/// One case file — the Abstract Test Suite unit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseCore {
    /// The globally unique case id; never reused, even after retirement.
    pub id: CaseId,
    /// Which executor drives the case (functional flow or content table).
    pub kind: CaseKind,
    /// Lifecycle status; only `Active` cases are selected for a run.
    #[serde(default)]
    pub status: CaseStatus,
    /// The schedule chapter (service component) the case belongs to.
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
    /// Spec-version windows outside which the case does not apply.
    #[serde(default)]
    pub applies: Applies,
    /// Non-version run conditions, each spec-cited; a failed guard ⇒
    /// `not-applicable` with citation.
    ///
    /// Prose, for the conditions no runner rule expresses. A condition the
    /// runner already decides structurally is never restated here — capability
    /// scoping is [`CaseCore::capabilities`] alone, and the `guard-scope`
    /// validate gate refuses a guard that states it.
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
    /// The preconditions the runner provisions before the flow runs.
    #[serde(default)]
    pub requires: Requires,
    /// Row parameterization (value matrix or fixture set), when present.
    #[serde(default)]
    pub parameters: Option<Parameters>,
    /// Ordered steps (functional cases).
    #[serde(default)]
    pub flow: Vec<FlowStep>,
    /// Content cases: the constraint context + decision table.
    #[serde(default)]
    pub constraint_context: Option<ConstraintContext>,
    /// Content cases: the rows, each one committed instance plus its verdict.
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

    /// Whether the step declares a SMART `scope` claim — true even for an
    /// empty list (the scope-less token is a deliberate declaration, not the
    /// absence of one).
    #[must_use]
    pub fn declares_scopes(&self) -> bool {
        self.scopes.is_some()
    }

    /// The declared SMART scope templates (empty when the key is absent).
    #[must_use]
    pub fn scope_templates(&self) -> &[TemplatedValue] {
        self.scopes.as_deref().unwrap_or_default()
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
    fn a_provisioned_party_relationship_mints_its_container_handle() {
        let r: Requires = serde_json::from_value(serde_json::json!({
            "party_relationship": {
                "source": "cnf.demographic.person.v1",
                "target": "cnf.demographic.organisation.v1",
                "relationship": "cnf.demographic.party_relationship.v1"
            }
        }))
        .unwrap();
        assert!(matches!(
            r.party_relationship,
            Some(PartyRelationshipRequirement::Exists { .. })
        ));
        assert_eq!(
            r.minted_handles()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["party_relationship_id".to_owned()]
        );

        let r: Requires =
            serde_json::from_value(serde_json::json!({ "party_relationship": "none" })).unwrap();
        assert!(matches!(
            r.party_relationship,
            Some(PartyRelationshipRequirement::None)
        ));
        assert!(r.minted_handles().is_empty());

        // Both ends and the relationship itself are mandatory: a partial block
        // would provision a relationship with an unresolved endpoint.
        assert!(
            serde_json::from_value::<Requires>(serde_json::json!({
                "party_relationship": { "relationship": "cnf.demographic.party_relationship.v1" }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<Requires>(
                serde_json::json!({ "party_relationship": "cnf.demographic.party_relationship.v1" })
            )
            .is_err()
        );
    }

    #[test]
    fn a_received_extract_mints_the_identities_it_carries() {
        // Cases 2/3 (an EHR is provisioned): the import lands in it, so only
        // the container/version handles are minted.
        let into_existing: Requires = serde_json::from_value(serde_json::json!({
            "ehr": { "commits": "any" },
            "import": {
                "extract": "cnf.messaging.ehr_extract.v1",
                "container": "X_VERSIONED_COMPOSITION"
            }
        }))
        .unwrap();
        assert_eq!(
            into_existing
                .minted_handles()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "ehr_id".to_owned(),
                "imported_versioned_object_uid".to_owned(),
                "imported_version_uid".to_owned(),
                "imported_branch_version_uid".to_owned(),
            ]
        );

        // Case 1 (no EHR provisioned): the clone's id is minted by the import.
        let clone: Requires = serde_json::from_value(serde_json::json!({
            "import": {
                "extract": "cnf.messaging.ehr_extract.v1",
                "container": "X_VERSIONED_EHR_STATUS"
            }
        }))
        .unwrap();
        assert!(
            clone
                .minted_handles()
                .iter()
                .any(|h| h.to_string() == "ehr_id"),
            "a whole-EHR clone mints the EHR it created"
        );

        let none: Requires =
            serde_json::from_value(serde_json::json!({ "import": "none" })).unwrap();
        assert_eq!(none.import, Some(ImportRequirement::None));
        assert!(none.minted_handles().is_empty());

        // Both keys are mandatory, and the container class is closed.
        assert!(
            serde_json::from_value::<Requires>(serde_json::json!({
                "import": { "extract": "cnf.messaging.ehr_extract.v1" }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<Requires>(serde_json::json!({
                "import": {
                    "extract": "cnf.messaging.ehr_extract.v1",
                    "container": "X_VERSIONED_THING"
                }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<Requires>(
                serde_json::json!({ "import": "cnf.messaging.ehr_extract.v1" })
            )
            .is_err()
        );
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
