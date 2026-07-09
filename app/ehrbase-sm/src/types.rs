//! Shared service types of the SM native API.
//!
//! The version-commit envelope (`UPDATE_VERSION<T>` / `UPDATE_AUDIT` /
//! `UPDATE_ATTESTATION`), the list-cursor convention, `EHR_SUMMARY`, and the
//! response envelope + query request/outcome types the service traits
//! exchange. Spec sources are cited per item; the wire divergences between
//! the SM classes and ITS-REST 1.0.3 are recorded inline (conformance review
//! F2, `docs/design/sm-platform/08-target-architecture.md` §3).

use std::collections::BTreeMap;

use jiff::Timestamp;
use serde::Deserialize;
use serde_json::Value;

use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_rm::prelude::{DvEhrUri, DvMultimedia, DvText, PartyProxy};

// ─── list handling (SM cursor convention) ────────────────────────────────────

/// The SM list-cursor parameters, used by every unbounded-list call
/// (`master02-overview.adoc` §List Handling).
///
/// `item_offset`: 0-based offset into the result items ("Zero signifies that
/// items starting from the first item should be returned").
/// `items_to_fetch`: number of items to fetch from the offset ("A zero means
/// 'all'"). Both optional; [`Page::all`] is the no-paging default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Page {
    /// Offset in result items at which to start returning items (0-based).
    pub item_offset: Option<u64>,
    /// Number of result items to fetch from `item_offset`; 0 (or `None`) =
    /// all.
    pub items_to_fetch: Option<u64>,
}

impl Page {
    /// The whole list — no offset, no limit.
    #[must_use]
    pub fn all() -> Self {
        Self::default()
    }

    /// The effective 0-based offset (`item_offset`, defaulting to 0).
    #[must_use]
    pub fn offset(self) -> u64 {
        self.item_offset.unwrap_or(0)
    }

    /// The effective fetch limit: `None` means all (a `Some(0)` in the SM
    /// also means 'all', normalized here).
    #[must_use]
    pub fn limit(self) -> Option<u64> {
        match self.items_to_fetch {
            None | Some(0) => None,
            some => some,
        }
    }
}

// ─── the version-commit envelope ─────────────────────────────────────────────

/// `UPDATE_AUDIT` — "The set of attributes required to document the committal
/// of an information item to a repository. Used by the server to create an
/// `AUDIT_DETAILS` object" (`update_audit.adoc`).
///
/// Deliberately partial: `AUDIT_DETAILS.time_committed` and `system_id` are
/// server-generated (`master03-common_package.adoc` §Version Update
/// Semantics). Invariant `Change_type_valid`: `change_type.defining_code`
/// must belong to the openEHR terminology *audit change type* group —
/// enforced at the service boundary via `openehr-term`, not in this type.
///
/// PORT NOTE (wire): ITS-REST `UpdateAudit.yaml` types `description` as
/// `UDvText` (plain string or `DV_TEXT`); the SM types it `String [0..1]`.
/// The native type keeps the SM shape; the adapter coerces a `DV_TEXT`
/// description to its `value` string.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAudit {
    /// Type of change; coded from the openEHR *audit change type* group.
    pub change_type: TerminologyCode,
    /// Reason for committal.
    #[serde(default)]
    pub description: Option<String>,
    /// Identity (and optional identity-management reference) of the
    /// committing user.
    pub committer: PartyProxy,
}

/// `UPDATE_ATTESTATION` — the wire form of a client-supplied attestation
/// (ITS-REST `specifications/schemas/common/UpdateAttestation.yaml`; extends
/// `UpdateAudit`).
///
/// PORT NOTE (wire, conformance review F2): the SM says attestations are
/// supplied "in their full form" as RM `ATTESTATION`
/// (`master03-common_package.adoc`), but the ITS-REST wire carries this
/// **partial** form — the server completes it into a full RM `ATTESTATION`
/// (adding `time_committed`/`system_id`, like `UPDATE_AUDIT` →
/// `AUDIT_DETAILS`). The wire wins at the boundary (ADR-010 precedence
/// rule); the native API therefore carries this type.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAttestation {
    /// Type of change; coded from the openEHR *audit change type* group
    /// (`666|attestation|` for attestations).
    pub change_type: TerminologyCode,
    /// Reason for committal (inherited from the `UpdateAudit` base).
    #[serde(default)]
    pub description: Option<String>,
    /// The attesting party.
    pub committer: PartyProxy,
    /// Optional visual representation of what was attested.
    #[serde(default)]
    pub attested_view: Option<DvMultimedia>,
    /// Proof of attestation.
    #[serde(default)]
    pub proof: Option<String>,
    /// Items attested, as EHR URIs.
    #[serde(default)]
    pub items: Option<Vec<DvEhrUri>>,
    /// Reason of this attestation.
    pub reason: DvText,
    /// True if this attestation is outstanding.
    pub is_pending: bool,
}

/// `UPDATE_VERSION<T>` — "An object representing an update to an existing
/// `VERSION` within a `VERSIONED_OBJECT`, that can be provided by a client to
/// the platform. The back-end will construct a full `VERSION<T>` object from
/// this and server-side generated data items. If this represents the first
/// version, it will also construct a new `VERSIONED_OBJECT` first"
/// (`update_version.adoc`).
///
/// Rules (`master03-common_package.adoc` §Version Update Semantics):
/// `preceding_version_uid` is mandatory **except** for a first version;
/// `lifecycle_state` is mandatory always (e.g. `532|complete|`,
/// `553|incomplete|`, `523|deleted|`).
///
/// PORT NOTEs (wire, conformance review F2; oracle ITS-REST
/// `specifications/schemas/common/UpdateVersion.yaml`):
/// - the wire field for [`Self::audit`] is **`commit_audit`** (serde rename
///   below); the native name keeps the SM's `audit`;
/// - wire `attestations` items are the partial [`UpdateAttestation`], not
///   full RM `ATTESTATION` (see that type);
/// - the wire carries **`signature`**, absent from the SM class — kept here
///   and fed to `ehrbase-signing`.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateVersion<T = Value> {
    /// Current version in the service for which this version is an update.
    /// `None` only for a first version.
    #[serde(default)]
    pub preceding_version_uid: Option<ObjectVersionId>,
    /// Lifecycle state of the content item in this version.
    pub lifecycle_state: TerminologyCode,
    /// Attestations relating to this version (wire-partial form; see
    /// [`UpdateAttestation`]).
    #[serde(default)]
    pub attestations: Option<Vec<UpdateAttestation>>,
    /// The data item being provided in this version update.
    pub data: T,
    /// Audit details for this update (wire name `commit_audit`).
    #[serde(rename = "commit_audit")]
    pub audit: UpdateAudit,
    /// Version signature (wire-only field; not in the SM class).
    #[serde(default)]
    pub signature: Option<String>,
}

// ─── query descriptor (SM Definitions) ──────────────────────────────────────

/// `QUERY_DESCRIPTOR` — "Object describing a query in terms of its unique
/// identifier, name under which it is currently registered and registration
/// time under the current name"
/// (`docs/specs/openehr/SM/docs/UML/classes/query_descriptor.adoc`).
///
/// Returned by `I_DEFINITION_QUERY.store_query` / `list_queries` /
/// `list_matching_queries` (`i_definition_query.adoc`).
///
/// PORT NOTE (wire): `registration_time` is typed `Iso8601_date_time` in the
/// SM; we carry it as an ISO-8601 `String` (the stored `created_at` rendered),
/// consistent with the rest of the native API's date handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDescriptor {
    /// Unique qualified name of the query, e.g. `ehr::all_over_50_women`
    /// (`<namespace>::<query_name>`). Mandatory `[1..1]`.
    pub qualified_query_name: String,
    /// Query semver.org version number, if any. Optional `[0..1]`.
    pub version: Option<String>,
    /// Time the query was registered under its current name (ISO-8601).
    /// Mandatory `[1..1]`.
    pub registration_time: String,
    /// Formalism of the query — `"aql"` or any other string value. Mandatory
    /// `[1..1]`.
    pub formalism: String,
    /// Source query text to be executed (prior to parameter substitution).
    /// Optional `[0..1]`.
    pub source: Option<String>,
}

// ─── EHR summary ─────────────────────────────────────────────────────────────

/// `EHR_SUMMARY` — "Summary form of `EHR` + `EHR_STATUS` objects convenient
/// for use in service interface" (`ehr_summary.adoc`). All six attributes are
/// mandatory in the SM.
#[derive(Debug, Clone)]
pub struct EhrSummary {
    /// EHR identifier of this EHR (`EHR.ehr_id.value`).
    pub ehr_id: String,
    /// Copy of `EHR.system_id`.
    pub system_id: String,
    /// Copy of `EHR.ehr_status` (canonical JSON).
    pub ehr_status: Value,
    /// Copy of `EHR.time_created` (ISO 8601).
    pub time_created: String,
    /// Number of Contributions in this EHR.
    pub contribution_count: i64,
    /// Number of (versioned) Compositions in this EHR.
    pub composition_count: i64,
}

// ─── response envelope (moved from `ehrbase-rest::response`, SM-1) ───────────

/// Typed metadata about a versioned resource a write/read produced, from
/// which the protocol adapter derives the spec-mandated `ETag`/`Location`
/// headers (ITS-REST 1.0.3 — `headers/ETag_*.yaml`, `headers/Location_*.yaml`).
#[derive(Debug, Clone)]
pub struct ResourceMeta {
    /// The owning EHR id (the `Location` path root; the `ETag` value for an
    /// EHR). Empty for EHR-less resources (demographic parties).
    pub ehr_id: String,
    /// The resource's identifier that is both the `ETag` value (quoted) and
    /// the `Location` path tail: an `OBJECT_VERSION_ID` for a versioned
    /// resource, the `ehr_id` for an EHR, or the contribution uid for a
    /// CONTRIBUTION.
    pub uid: String,
    /// The commit time of this version (its audit `time_committed`).
    /// ITS-REST 1.0.3 declares no `Last-Modified` response header, so this is
    /// carried for completeness/observability, not emitted at the wire.
    pub last_modified: Option<Timestamp>,
}

impl ResourceMeta {
    /// Metadata for a versioned resource: the owning EHR plus the resource
    /// `uid` used as the `ETag` and `Location` tail.
    #[must_use]
    pub fn new(ehr_id: impl Into<String>, uid: impl Into<String>) -> Self {
        Self {
            ehr_id: ehr_id.into(),
            uid: uid.into(),
            last_modified: None,
        }
    }

    /// Attach the version's commit time (carried, not emitted — see the
    /// field).
    #[must_use]
    pub fn with_last_modified(mut self, at: Timestamp) -> Self {
        self.last_modified = Some(at);
        self
    }
}

/// A service response: the canonical-JSON RM payload plus optional resource
/// metadata. A `null` `body` means "no representation" (a logically deleted
/// read, or a resource whose value is not returned); `meta` is present
/// whenever the operation produced/identified a versioned resource, letting
/// the protocol adapter set `ETag`/`Location` per the operation's spec.
#[derive(Debug, Clone)]
pub struct ServiceResponse {
    /// The canonical-JSON RM payload, or `Value::Null` when there is none.
    pub body: Value,
    /// Resource metadata for header derivation, if any.
    pub meta: Option<ResourceMeta>,
}

impl ServiceResponse {
    /// A response carrying both an RM body and resource metadata.
    #[must_use]
    pub fn new(body: Value, meta: ResourceMeta) -> Self {
        Self {
            body,
            meta: Some(meta),
        }
    }

    /// A plain response with a body but no resource metadata (reads whose
    /// spec response declares no `ETag`/`Location`).
    #[must_use]
    pub fn plain(body: Value) -> Self {
        Self { body, meta: None }
    }

    /// A bodyless response carrying resource metadata — the delete outcome
    /// that still returns the (now deleted) `version_uid` in
    /// `ETag`/`Location` (`204_COMPOSITION_deleted.yaml`).
    #[must_use]
    pub fn deleted(meta: ResourceMeta) -> Self {
        Self {
            body: Value::Null,
            meta: Some(meta),
        }
    }

    /// Whether the body is absent (a logically deleted read → `204`, or a
    /// bodyless delete outcome).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.is_null()
    }
}

// ─── query request/outcome (moved from `ehrbase-rest::backend`, SM-1) ────────

/// A normalized AQL query request: the paging window, the single-EHR scope,
/// and the `$parameter` bindings, gathered from the query string or the
/// request body by the protocol adapter (ITS-REST query
/// `parameters/query/{ehr_id,offset,fetch}` + `query_parameters`).
///
/// The SM's `ADHOC_QUERY_EXECUTE_SPEC`/`STORED_QUERY_EXECUTE_SPEC`
/// (`adhoc_query_execute_spec.adoc`, `stored_query_execute_spec.adoc`) are
/// realized by this type plus the trait-method arguments (query text /
/// qualified name + version); `formalism` is fixed to `"aql"` — other
/// formalisms are rejected typed, which the SM sanctions ("matching one of:
/// aql; any other string value").
#[derive(Debug, Clone, Default)]
pub struct AqlQueryRequest {
    /// The `ehr_id` scope (query param or `openEHR-EHR-id` header), if any.
    pub ehr_id: Option<String>,
    /// The `offset` paging parameter (0-based row to start from).
    pub offset: Option<i64>,
    /// The `fetch` paging parameter (max rows to return).
    pub fetch: Option<i64>,
    /// The `query_parameters` (`$name` binds, no `$` prefix).
    pub parameters: BTreeMap<String, Value>,
    /// The ABAC patient-scope subject id
    /// (`docs/enterprise/access-control.md` §6.4): when set, the engine
    /// pre-filters every VO root to EHRs whose subject equals it.
    pub subject_scope: Option<String>,
    /// Whether the executor should collect the touched EHR-id / template-id
    /// sets for the ABAC query post-check.
    pub collect_attributes: bool,
}

/// The outcome of an AQL execution: the assembled `RESULT_SET` plus — when
/// the caller asked for them ([`AqlQueryRequest::collect_attributes`]) — the
/// distinct EHR ids and template ids the query touched, for the ABAC
/// post-check.
#[derive(Debug, Clone, Default)]
pub struct QueryOutcome {
    /// The ITS-REST 1.0.3 `RESULT_SET` (canonical JSON) the adapter renders.
    pub result_set: Value,
    /// The distinct EHR ids the query touched (empty unless collected).
    pub ehr_ids: Vec<String>,
    /// The distinct template ids the query touched (empty unless collected).
    pub template_ids: Vec<String>,
}

impl QueryOutcome {
    /// An outcome with no collected attributes (the pre-ABAC shape).
    #[must_use]
    pub fn plain(result_set: Value) -> Self {
        Self {
            result_set,
            ehr_ids: Vec::new(),
            template_ids: Vec::new(),
        }
    }
}

/// The concrete PARTY resource families of the DEMOGRAPHIC group (the five
/// concrete `ACTOR`/`PARTY` leaves the routes are keyed by). Moved from
/// `ehrbase-rest::backend` (SM-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyKind {
    /// `AGENT` (`/demographic/agent`).
    Agent,
    /// `GROUP` (`/demographic/group`).
    Group,
    /// `ORGANISATION` (`/demographic/organisation`).
    Organisation,
    /// `PERSON` (`/demographic/person`).
    Person,
    /// `ROLE` (`/demographic/role`).
    Role,
}

impl PartyKind {
    /// The RM `_type` this resource family stores (`PERSON`, `ROLE`, …).
    #[must_use]
    pub fn rm_type(self) -> &'static str {
        match self {
            PartyKind::Agent => "AGENT",
            PartyKind::Group => "GROUP",
            PartyKind::Organisation => "ORGANISATION",
            PartyKind::Person => "PERSON",
            PartyKind::Role => "ROLE",
        }
    }

    /// The URL path segment of this resource family (`agent`, `person`, …).
    #[must_use]
    pub fn segment(self) -> &'static str {
        match self {
            PartyKind::Agent => "agent",
            PartyKind::Group => "group",
            PartyKind::Organisation => "organisation",
            PartyKind::Person => "person",
            PartyKind::Role => "role",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn page_normalizes_zero_fetch_to_all() {
        // SM §List Handling: "A zero means 'all'".
        let page = Page {
            item_offset: Some(3),
            items_to_fetch: Some(0),
        };
        assert_eq!(page.offset(), 3);
        assert_eq!(page.limit(), None);
    }

    #[test]
    fn update_version_deserializes_the_wire_shape() {
        // The ITS-REST `UpdateVersion.yaml` field names: `commit_audit`,
        // partial attestations, `signature` (conformance review F2).
        let v: UpdateVersion = serde_json::from_value(json!({
            "preceding_version_uid": { "value": "8849182c-82ad-4088-a07f-48ead4180515::sys::1" },
            "lifecycle_state": { "terminology_id": "openehr", "code_string": "532" },
            "data": { "_type": "COMPOSITION" },
            "signature": "sig-bytes",
            "commit_audit": {
                "change_type": { "terminology_id": "openehr", "code_string": "251" },
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "A user" }
            }
        }))
        .expect("wire-shaped UPDATE_VERSION deserializes");
        assert!(v.preceding_version_uid.is_some());
        assert_eq!(v.signature.as_deref(), Some("sig-bytes"));
        assert!(v.attestations.is_none());
    }
}
