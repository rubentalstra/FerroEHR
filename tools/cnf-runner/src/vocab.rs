//! The closed vocabularies of the CNF 2.0 schedule.
//!
//! Every enum here is normative and extensible only by schedule release
//! (CNF 2.0 artifact-set design; the outcome kinds carry the schedule
//! language of `CNF platform_test_schedule master04/06/07`). A value outside
//! these enums is unrepresentable in the typed model — the compile-time
//! property the reference runner exists to demonstrate.

use serde::{Deserialize, Serialize};

/// Verdict class of an outcome kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeClass {
    /// The operation fulfilled its service contract.
    Success,
    /// The operation was refused for the reason the kind names.
    Error,
}

macro_rules! outcome_kinds {
    ($( $(#[$doc:meta])* ($variant:ident, $token:literal, $class:ident) ),+ $(,)?) => {
        /// A protocol-neutral outcome kind — the only expectation language a
        /// case core may speak (wire realization lives in the operation
        /// bindings). Closed enum; extension only by schedule release.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum OutcomeKind {
            $( $(#[$doc])* #[serde(rename = $token)] $variant, )+
        }

        impl OutcomeKind {
            /// All kinds, in schedule order (the `vocab/outcomes.yaml` order).
            pub const ALL: &'static [OutcomeKind] = &[ $(OutcomeKind::$variant,)+ ];

            /// The kind's wire-independent token (`created`, `not_found`, …).
            #[must_use]
            pub fn token(self) -> &'static str {
                match self { $( OutcomeKind::$variant => $token, )+ }
            }

            /// Success or error class.
            #[must_use]
            pub fn class(self) -> OutcomeClass {
                match self { $( OutcomeKind::$variant => OutcomeClass::$class, )+ }
            }

            /// Parse a token.
            #[must_use]
            pub fn from_token(token: &str) -> Option<Self> {
                match token { $( $token => Some(OutcomeKind::$variant), )+ _ => None }
            }
        }
    };
}

outcome_kinds! {
    /// New resource exists ("positive response associated to the successful creation").
    (Created, "created", Success),
    /// Read/query succeeded with content.
    (Ok, "ok", Success),
    /// Fulfilled with no content (e.g. composition logically deleted at the requested time).
    (OkEmpty, "ok_empty", Success),
    /// New version of an existing resource created.
    (Updated, "updated", Success),
    /// Logical delete performed (a new VERSION, `lifecycle_state = openehr::523|deleted|`).
    (Deleted, "deleted", Success),
    /// Definition stored (stored-query PUT — wire 200, not 201).
    (Stored, "stored", Success),
    /// Accepted for later processing, not yet performed — the asynchronous
    /// branch a service MAY answer with instead of completing inline
    /// (ITS-REST `admin_ehr_delete`/`admin_ehr_delete_all`: "The server may
    /// execute this operation asynchronously (e.g. in batches), in which case
    /// returns status `202 Accepted`"). Distinct from `ok_empty`: the request
    /// was taken, the effect is not yet observable.
    (Accepted, "accepted", Success),
    /// Duplicate identity ("an EHR with the provided `ehr_id` … should be unique"; duplicate `template_id`).
    (AlreadyExists, "already_exists", Error),
    /// Target does not exist ("EHR with `<ehr_id>` does not exist").
    (NotFound, "not_found", Error),
    /// `preceding_version_uid` does not exist. DECISION (#557, 2026-07-28):
    /// stays a distinct kind from `precondition_failed` even where ITS-REST
    /// realizes both as one indistinguishable wire answer (the If-Match PUTs:
    /// a false-evaluating If-Match "MUST respond with HTTP status code `412
    /// Precondition Failed`" regardless of whether the named version ever
    /// existed — overview Requests_and_responses.md §"If-Match and accidental
    /// overwrites"). Case cores speak SM outcomes, and the SM distinguishes
    /// the causes (an unknown version vs a stale one); the fold is a
    /// REALIZATION fact recorded on each binding, and the tie-break in
    /// `exec::outcome` is the documented norm: when the wire genuinely cannot
    /// distinguish members, the expected member's status match satisfies the
    /// expectation — the status/header assertions still gate.
    (VersionNotFound, "version_not_found", Error),
    /// Version precondition evaluated false (stale `preceding_version_uid`).
    (PreconditionFailed, "precondition_failed", Error),
    /// Required version precondition absent.
    (PreconditionMissing, "precondition_missing", Error),
    /// Semantically invalid content ("information about the errors in the provided COMPOSITION").
    (ValidationFailed, "validation_failed", Error),
    /// Referenced OPT not on the server ("information about the non-existent OPT").
    (TemplateNotFound, "template_not_found", Error),
    /// Content commits against a different `template_id` than the versioned object.
    (TemplateMismatch, "template_mismatch", Error),
    /// Simplified-format commit without template identification.
    (MissingTemplateId, "missing_template_id", Error),
    /// Delete of an already-deleted version.
    (AlreadyDeleted, "already_deleted", Error),
    /// Other uniqueness/state conflict.
    (Conflict, "conflict", Error),
    /// No representation satisfies `Accept`.
    (NotAcceptable, "not_acceptable", Error),
    /// Payload media type unsupported.
    (UnsupportedMedia, "unsupported_media", Error),
    /// Malformed/unprocessable AQL.
    (InvalidQuery, "invalid_query", Error),
    /// Server aborted at max execution time.
    (Timeout, "timeout", Error),
    /// Request lacks valid authentication (the route-table-wide 401 rule).
    (Unauthenticated, "unauthenticated", Error),
    /// Authenticated principal lacks authorization for the operation (403).
    (Forbidden, "forbidden", Error),
    /// The request itself is malformed — syntactically invalid content, an
    /// unparseable identifier, a contradictory or missing required argument
    /// (ITS-REST `Requests_and_responses.md` §HTTP status codes row 400: "the
    /// service cannot or will not process the request due to something that
    /// is perceived to be a client error (e.g., malformed request syntax,
    /// syntactically invalid content)", and below the table: "Status code
    /// `400` indicates normally a bad request, as well as a generic
    /// client-side error, used when no other `4xx` error code is
    /// appropriate"). Distinct from `validation_failed`, which is the
    /// well-formed-but-semantically-invalid content branch (422).
    (BadRequest, "bad_request", Error),
    /// The method is known to the service but the target resource does not
    /// support it — including a resource a deployment's configuration has
    /// left serving none (ITS-REST `Requests_and_responses.md` §HTTP status
    /// codes row 405 and §HTTP Methods: "If a method is recognized but not
    /// allowed for the target resource, the response SHOULD be `405 Method
    /// Not Allowed` status code").
    (MethodNotAllowed, "method_not_allowed", Error),
}

/// Which optional case-core blocks are meaningful.
///
/// The schedule defines three kinds (`functional | content | performance`);
/// the performance case-core schema is a separate artifact revision (this
/// crate's W7 scope), so the assertion-machinery model closes over the two
/// kinds it validates. // TODO: add `performance` with the §8.14 workload
/// blocks when the performance schedule lands (W7, #202).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseKind {
    /// SM-operation flow case.
    Functional,
    /// Template-parameterized decision-table case (one executor serves both:
    /// a content row is a generate→commit→expect functional execution).
    Content,
}

/// Case lifecycle status. Ids are never reused; a retired case keeps its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseStatus {
    /// In force.
    #[default]
    Active,
    /// Kept for id-permanence only; never selected.
    Retired,
    /// Not yet in force; validated but never verdict-bearing.
    Draft,
}

/// The schedule component a case belongs to (the chapter taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Component {
    Ehr,
    EhrComposition,
    EhrContribution,
    EhrDirectory,
    #[serde(rename = "DEFINITION_ADL14")]
    DefinitionAdl14,
    #[serde(rename = "DEFINITION_ADL2")]
    DefinitionAdl2,
    DefinitionQuery,
    Query,
    Demographic,
    Admin,
    Messaging,
    Content,
    SimplifiedFormats,
    Security,
    Performance,
}

/// A capability family (the certificate's rating dimensions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Family {
    /// The platform (CDR) profile family: CORE/STANDARD/OPTIONS.
    Platform,
    /// The proposed Enterprise extension: D/M/X.
    Enterprise,
    /// The Security & Privacy family: SEC-BASIC (higher rungs are future SEC work).
    Security,
}

/// A family-scoped profile tier. Each tier knows its family so a
/// capability-matrix row declaring a foreign tier is a typed error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Tier {
    #[serde(rename = "CORE")]
    Core,
    #[serde(rename = "STANDARD")]
    Standard,
    #[serde(rename = "OPTIONS")]
    Options,
    #[serde(rename = "SEC-BASIC")]
    SecBasic,
    #[serde(rename = "D")]
    EnterpriseD,
    #[serde(rename = "M")]
    EnterpriseM,
    #[serde(rename = "X")]
    EnterpriseX,
}

impl Tier {
    /// The family this tier is scoped to.
    #[must_use]
    pub fn family(self) -> Family {
        match self {
            Tier::Core | Tier::Standard | Tier::Options => Family::Platform,
            Tier::SecBasic => Family::Security,
            Tier::EnterpriseD | Tier::EnterpriseM | Tier::EnterpriseX => Family::Enterprise,
        }
    }
}

/// The machine-readable handling class of an ambiguity-register entry — the
/// pipeline branches on this (closed enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Assert loosely (e.g. AMB-1: at most a `message` string).
    LooseAssert,
    /// Handling encoded directly in bindings/cases.
    FixedHandling,
    /// Sibling cases carry `option:` tags; the ICS `options` declaration selects.
    OptionSelect,
    /// Verdicts reported, never gating.
    ReportOnly,
    /// No normative cases; statement-declared behaviour only.
    StatementDeclared,
    /// Editorial defect in the schedule text itself.
    Editorial,
}

/// A wire format role (the §8.7 format axes; media types per the ITS-REST
/// `Accept_*`/`ContentType_*` parameter files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FormatName {
    #[serde(rename = "canonical-json")]
    CanonicalJson,
    #[serde(rename = "canonical-xml")]
    CanonicalXml,
    #[serde(rename = "wt-flat")]
    WtFlat,
    #[serde(rename = "wt-structured")]
    WtStructured,
    /// The Web Template itself (`application/openehr.wt+json`, template GET only).
    #[serde(rename = "wt")]
    Wt,
}

/// A corpus payload format — the wire roles plus the template-source form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorpusFormat {
    #[serde(rename = "canonical-json")]
    CanonicalJson,
    #[serde(rename = "canonical-xml")]
    CanonicalXml,
    #[serde(rename = "wt-flat")]
    WtFlat,
    #[serde(rename = "wt-structured")]
    WtStructured,
    /// An ADL 1.4 operational template (OPT XML).
    #[serde(rename = "opt-xml")]
    OptXml,
    /// AQL query text (stored-query definitions).
    #[serde(rename = "aql-text")]
    AqlText,
    /// ADL2 artefact source text (archetypes/templates/OPTs in ADL syntax).
    #[serde(rename = "adl2-text")]
    Adl2Text,
    /// ADL 1.4 archetype source text (the `.adl` form the ADL 1.4
    /// archetype-provisioning extension routes exchange as `text/plain`).
    #[serde(rename = "adl14-text")]
    Adl14Text,
}

/// HTTP method of a binding request (the ITS-REST realization layer).
///
/// The vocabulary is the method subset the released overview tabulates
/// (`ITS-REST docs/overview/Requests_and_responses.md` §HTTP Methods: GET,
/// HEAD, POST, PUT, DELETE, OPTIONS) — never a wider HTTP method set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    /// "Describe the communication options for the target resource"
    /// (`Requests_and_responses.md` §HTTP Methods) — the method the STABLE
    /// System API's one operation is served with.
    Options,
}

/// Parameter-iteration law (`CNF platform_test_schedule master04` iteration
/// semantics: "the pre-conditions and post-conditions apply to the run for X").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Iteration {
    /// The master04 law: the whole `requires` block re-established per row.
    ResetPerRow,
    /// Rows execute against one shared server state — required when an
    /// aggregate postcondition spans rows.
    SinglePass,
}

/// The `requires.server` precondition ("The server should be empty (no EHRs,
/// no commits, no OPTs)" — CNF master06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Empty,
    /// The case's ground depends on GLOBAL server state (an empty template
    /// list, a globally-absent artefact) that only an exclusively-owned SUT
    /// can establish — on a shared instance the case is not-applicable
    /// (the ixit environment declares exclusivity).
    Exclusive,
    Any,
}

/// Adjudicated fixture verdict in the corpus manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixtureVerdict {
    Valid,
    Invalid,
}

/// Runtime placeholder policy for a corpus fixture (the Robot corpus's
/// `__AUTO-GENRATED-BY-TEST__` convention, formalized).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceholderPolicy {
    /// The runner substitutes a fresh random value per use.
    #[serde(rename = "runtime-random")]
    RuntimeRandom,
}

/// The openEHR ITS-XML target namespace a canonical-XML document root must be
/// qualified with (`namespace:` of the `xml_root` assertion).
///
/// The closed set is the closure of what ITS-XML actually publishes: the
/// schemas of the `Release-1.0.2` bundle carry
/// `targetNamespace="http://schemas.openehr.org/v1"` and those of
/// `Release-2.0.0` carry `.../v2`, and both bundles set
/// `elementFormDefault="qualified"`, so a conforming instance's root is in one
/// of exactly these two namespaces — never in none. Which LINEAGE a service
/// serves is implementation-defined (register AMB-169), so [`Self::Published`]
/// is the assertion a case normally makes; the two pinned tokens exist for a
/// row that has already fixed the lineage by other means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XmlNamespace {
    /// Either published openEHR ITS-XML target namespace.
    #[serde(rename = "openehr-published")]
    Published,
    /// `http://schemas.openehr.org/v1` — the Release-1.0.2 lineage.
    #[serde(rename = "openehr-v1")]
    V1,
    /// `http://schemas.openehr.org/v2` — the Release-2.0.0 lineage.
    #[serde(rename = "openehr-v2")]
    V2,
}

impl XmlNamespace {
    /// The published token (matches the case-core schema enum).
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Published => "openehr-published",
            Self::V1 => "openehr-v1",
            Self::V2 => "openehr-v2",
        }
    }

    /// Whether a document's root-element namespace URI satisfies this
    /// expectation.
    #[must_use]
    pub fn accepts(self, uri: &str) -> bool {
        match self {
            Self::Published => uri == Self::V1_URI || uri == Self::V2_URI,
            Self::V1 => uri == Self::V1_URI,
            Self::V2 => uri == Self::V2_URI,
        }
    }

    /// The Release-1.0.2 bundle's target namespace.
    pub const V1_URI: &'static str = "http://schemas.openehr.org/v1";
    /// The Release-2.0.0 bundle's target namespace.
    pub const V2_URI: &'static str = "http://schemas.openehr.org/v2";
}

/// A named ignore-set the `equivalent` assertion may resolve
/// (normative per operation/format overlay, never runner-chosen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IgnoreSetName {
    /// The operation's server-assigned paths — enumerated per binding.
    ServerAssigned,
    /// The simplified-formats ctx defaulting set — enumerated once in
    /// `vocab/selectors.yaml` (ITS-REST `simplified_formats` master06 §ctx defaults).
    CtxDefaults,
}

/// Spec-component keys of the `applies` version-applicability map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecComponent {
    Rm,
    Base,
    Am,
    Aql,
    ItsRest,
    Term,
}

/// The RM `change_type` values the schedule asserts
/// (`RM common §change_control` — `VERSION.commit_audit.change_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChangeType {
    Create,
    Modify,
    Deleted,
}

/// AQL `RESULT_SET` comparison mode (`match:` of the `result_set` assertion).
/// `Set` is bag (multiset) equality — duplicate rows significant, AQL being
/// bag-semantics absent DISTINCT (QUERY `master03-syntax.adoc` §DISTINCT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultSetMatch {
    /// Legal only when the query totally orders the expected rows
    /// (QUERY master03 §ORDER BY: absent ORDER BY, ordering is undefined).
    Ordered,
    /// Bag equality.
    Set,
    /// Row count only.
    Count,
    /// Every expected row appears (bag-wise); extra rows permitted.
    Contains,
}

/// The ITS a binding realizes. Closed to the REST ITS until another ITS
/// binding layer is registered by schedule release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItsName {
    #[serde(rename = "its-rest")]
    ItsRest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_tokens_round_trip() {
        for kind in OutcomeKind::ALL {
            assert_eq!(OutcomeKind::from_token(kind.token()), Some(*kind));
        }
        assert_eq!(OutcomeKind::ALL.len(), 26);
        assert_eq!(OutcomeKind::Created.class(), OutcomeClass::Success);
        assert_eq!(OutcomeKind::Timeout.class(), OutcomeClass::Error);
        // The schedule-release additions of #544.
        assert_eq!(OutcomeKind::Accepted.class(), OutcomeClass::Success);
        assert_eq!(OutcomeKind::BadRequest.class(), OutcomeClass::Error);
        assert_eq!(OutcomeKind::MethodNotAllowed.class(), OutcomeClass::Error);
        for token in ["accepted", "bad_request", "method_not_allowed"] {
            assert!(
                OutcomeKind::from_token(token).is_some(),
                "{token} must parse"
            );
        }
    }

    #[test]
    fn tier_families() {
        assert_eq!(Tier::Core.family(), Family::Platform);
        assert_eq!(Tier::SecBasic.family(), Family::Security);
        assert_eq!(Tier::EnterpriseD.family(), Family::Enterprise);
    }

    #[test]
    fn serde_tokens() {
        let kind: OutcomeKind =
            serde_json::from_value(serde_json::json!("already_exists")).unwrap();
        assert_eq!(kind, OutcomeKind::AlreadyExists);
        let tier: Tier = serde_json::from_value(serde_json::json!("SEC-BASIC")).unwrap();
        assert_eq!(tier, Tier::SecBasic);
        let comp: Component =
            serde_json::from_value(serde_json::json!("DEFINITION_ADL14")).unwrap();
        assert_eq!(comp, Component::DefinitionAdl14);
        let fmt: FormatName = serde_json::from_value(serde_json::json!("wt-flat")).unwrap();
        assert_eq!(fmt, FormatName::WtFlat);
    }
}

impl CaseKind {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [CaseKind] = &[CaseKind::Functional, CaseKind::Content];
}

impl CaseStatus {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [CaseStatus] =
        &[CaseStatus::Active, CaseStatus::Retired, CaseStatus::Draft];
}

impl Component {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [Component] = &[
        Component::Ehr,
        Component::EhrComposition,
        Component::EhrContribution,
        Component::EhrDirectory,
        Component::DefinitionAdl14,
        Component::DefinitionAdl2,
        Component::DefinitionQuery,
        Component::Query,
        Component::Demographic,
        Component::Admin,
        Component::Messaging,
        Component::Content,
        Component::SimplifiedFormats,
        Component::Security,
        Component::Performance,
    ];
}

impl Family {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [Family] = &[Family::Platform, Family::Enterprise, Family::Security];
}

impl Tier {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [Tier] = &[
        Tier::Core,
        Tier::Standard,
        Tier::Options,
        Tier::SecBasic,
        Tier::EnterpriseD,
        Tier::EnterpriseM,
        Tier::EnterpriseX,
    ];
}

impl Disposition {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [Disposition] = &[
        Disposition::LooseAssert,
        Disposition::FixedHandling,
        Disposition::OptionSelect,
        Disposition::ReportOnly,
        Disposition::StatementDeclared,
        Disposition::Editorial,
    ];
}

impl FormatName {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [FormatName] = &[
        FormatName::CanonicalJson,
        FormatName::CanonicalXml,
        FormatName::WtFlat,
        FormatName::WtStructured,
        FormatName::Wt,
    ];
}

impl CorpusFormat {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [CorpusFormat] = &[
        CorpusFormat::CanonicalJson,
        CorpusFormat::CanonicalXml,
        CorpusFormat::WtFlat,
        CorpusFormat::WtStructured,
        CorpusFormat::OptXml,
        CorpusFormat::AqlText,
        CorpusFormat::Adl2Text,
        CorpusFormat::Adl14Text,
    ];
}

impl HttpMethod {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [HttpMethod] = &[
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Delete,
        HttpMethod::Head,
        HttpMethod::Options,
    ];
}

impl Iteration {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [Iteration] = &[Iteration::ResetPerRow, Iteration::SinglePass];
}

impl ServerState {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [ServerState] =
        &[ServerState::Empty, ServerState::Exclusive, ServerState::Any];
}

impl FixtureVerdict {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [FixtureVerdict] = &[FixtureVerdict::Valid, FixtureVerdict::Invalid];
}

impl PlaceholderPolicy {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [PlaceholderPolicy] = &[PlaceholderPolicy::RuntimeRandom];
}

impl SpecComponent {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [SpecComponent] = &[
        SpecComponent::Rm,
        SpecComponent::Base,
        SpecComponent::Am,
        SpecComponent::Aql,
        SpecComponent::ItsRest,
        SpecComponent::Term,
    ];

    /// The authored key of this component in an `applies` map / a statement's
    /// `spec_versions` — the one spelling every renderer and citation uses.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            SpecComponent::Rm => "rm",
            SpecComponent::Base => "base",
            SpecComponent::Am => "am",
            SpecComponent::Aql => "aql",
            SpecComponent::ItsRest => "its_rest",
            SpecComponent::Term => "term",
        }
    }
}

impl ChangeType {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [ChangeType] =
        &[ChangeType::Create, ChangeType::Modify, ChangeType::Deleted];
}

impl ResultSetMatch {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [ResultSetMatch] = &[
        ResultSetMatch::Ordered,
        ResultSetMatch::Set,
        ResultSetMatch::Count,
        ResultSetMatch::Contains,
    ];
}

impl ItsName {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &'static [ItsName] = &[ItsName::ItsRest];
}

#[cfg(test)]
#[allow(clippy::panic)] // test assertions
mod all_consts_tests {
    use super::*;

    /// Every `ALL` list is exhaustive: a match over the enum with no wildcard
    /// would fail to compile on a new variant, and this test pins the counts
    /// so schema emission can trust the lists.
    #[test]
    fn all_lists_are_exhaustive() {
        assert_eq!(CaseKind::ALL.len(), 2);
        assert_eq!(Component::ALL.len(), 15);
        assert_eq!(Tier::ALL.len(), 7);
        assert_eq!(Disposition::ALL.len(), 6);
        assert_eq!(FormatName::ALL.len(), 5);
        assert_eq!(CorpusFormat::ALL.len(), 8);
    }
}
