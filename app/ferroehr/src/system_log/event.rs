// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The transport-agnostic audit **event model** for the SM System Log
//! component (`I_SYSTEM_LOG`).
//!
//! NOTE: the vendored SM `I_SYSTEM_LOG` interface is an empty stub
//! (`docs/specs/openehr/SM/docs/UML/classes/i_system_log.adoc` names it with no
//! methods and no description), the only normative statement being the platform
//! overview's "System Log | IHE ATNA-compliant system log"
//! (`docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`
//! §Overview), so this event model is entirely our own design.
//!
//! It is the minimal shape the ITS-REST audit middleware needs to hand a
//! resolved operation record to the platform's ATNA emitter. Nothing here
//! depends on `openehr-its`, HTTP, or DICOM; the DICOM / RFC-3881 renderings of
//! these enums live in [`super::codes`], the wire model in [`super::message`].

use jiff::Timestamp;

/// DICOM `EventActionCode` (PS3.15 §A.5.1): the CRUD/execute class of the op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventActionCode {
    /// `C` — create.
    Create,
    /// `R` — read.
    Read,
    /// `U` — update.
    Update,
    /// `D` — delete.
    Delete,
    /// `E` — execute (query).
    Execute,
}

/// DICOM `EventOutcomeIndicator` (PS3.15 §A.5.1), derived from the HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOutcome {
    /// `0` — success.
    Success,
    /// `4` — minor failure (the action failed; e.g. 4xx incl. 401/403).
    MinorFailure,
    /// `8` — serious failure (the action was not performed; e.g. 5xx).
    SeriousFailure,
    /// `12` — major failure (the node/service is compromised/unavailable).
    MajorFailure,
}

/// The class of resource an operation touches — determines the DICOM `EventID`
/// and the participant-object rendering (see [`super::codes`] and
/// [`super::message`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectClass {
    /// EHR / `EHR_STATUS` → "Patient Record"; a Patient-Number participant.
    Ehr,
    /// Composition → "composition"; Patient-Number + object-URI participants.
    Composition,
    /// Contribution → "contribution"; Patient-Number + object-URI participants.
    Contribution,
    /// Directory (FOLDER) → "directory"; Patient-Number + object-URI participants.
    Directory,
    /// Ad-hoc / stored query execution → "query"; a Search-Criteria participant.
    Query,
    /// Operational template provisioning (DEFINITION `adl1.4`/`adl2` templates)
    /// → "template"; an object-URI participant (the template id).
    Template,
    /// Demographic party (PERSON / AGENT / ORGANISATION / GROUP / ROLE /
    /// versioned party) → "demographic"; an object-URI participant (the party
    /// uid). Demographic data is person-identifiable, so it is audited under
    /// the Patient-Record `EventID` family.
    Demographic,
    /// EHR-Extract communication (SM-5 export/import of whole-EHR or
    /// spec-driven extracts) → "extract"; Patient-Number + object-URI
    /// participants. Extract communication carries patient-identifiable
    /// clinical data across systems and is audited for **non-repudiation** —
    /// the security chapter requires that "logging of communication of
    /// Extracts … can be used to guarantee non-repudiation of information
    /// passed between systems" (BASE
    /// `architecture_overview/master07-security.adoc` §Non-repudiation).
    // The SM-5 EHR-Extract export/import path emits an
    // `AuditEvent { object: Extract, .. }` on completion
    // (`FerroEhrService::emit_extract_audit`); this variant is its resource class.
    Extract,
    /// Generic application activity (unclassified extension operations) →
    /// "Application Activity"; no clinical object.
    ApplicationActivity,
    /// A user-authentication event (a genuine login, or a rejected 401/403
    /// access attempt) → DICOM "User Authentication" (110114); no clinical
    /// object.
    Authentication,
}

/// The concrete kind of event within its `EventID` family — rendered as the
/// DICOM `EventTypeCode` (DICOM PS3.15 §A.5 `EventIdentification`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// DCM 110122 "Login" — a user-authentication attempt (success or failure).
    Login,
    /// DCM 110123 "Logout" — a session end (reserved; no session teardown
    /// surface emits it yet).
    Logout,
    /// The concrete ITS-REST operation, carried as the generated operation id.
    /// NOTE: no external code system governs openEHR REST operations — our own
    /// design/extension: the id is emitted under the `openEHR-ITS-REST` code
    /// system name.
    RestOperation(&'static str),
}

/// A fully-resolved audit event, ready to be rendered into a DICOM
/// `AuditMessage` ([`super::message`]).
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// The CRUD/execute class.
    pub action: EventActionCode,
    /// The resource class (drives `EventID` + participant objects).
    pub object: ObjectClass,
    /// The concrete event kind within the `EventID` family (the DICOM
    /// `EventTypeCode`): the login marker for authentication records, the
    /// ITS-REST operation id for operation records. `None` for emitters with
    /// no finer classification (e.g. the service-layer extract audit).
    pub event_type: Option<EventType>,
    /// The response outcome.
    pub outcome: EventOutcome,
    /// The requesting user (Basic username / OAuth `sub`; `UNKNOWN` when absent).
    pub user_id: String,
    /// Whether the source participant is the requestor (always true here).
    pub user_is_requestor: bool,
    /// The client network address (`X-Forwarded-For` first hop / peer), if known.
    pub client_ip: Option<String>,
    /// The owning EHR id, for optional background subject enrichment.
    pub ehr_id: Option<String>,
    /// The resolved object identifier (resource URI / version uid / query name).
    pub object_id: Option<String>,
    /// The bearer token's `jti` claim when the request was Bearer-authenticated
    /// — the minimal token identity the FHIR `AuditEvent` rendering records
    /// (never the token itself; token contents are never logged).
    pub token_id: Option<String>,
    /// The audited request's resolved tenant, when tenancy is on and the
    /// request carried one. Informational on the stored record (the node's
    /// audit trail is an operator surface, not tenant-scoped).
    pub tenant_id: Option<uuid::Uuid>,
    /// The event time.
    pub timestamp: Timestamp,
}

impl AuditEvent {
    /// Construct an event with the given class + outcome, defaulting the runtime
    /// fields (the caller fills user/ip/ids). `timestamp` is set to now.
    #[must_use]
    pub fn new(action: EventActionCode, object: ObjectClass, outcome: EventOutcome) -> Self {
        Self {
            action,
            object,
            event_type: None,
            outcome,
            user_id: String::new(),
            user_is_requestor: true,
            client_ip: None,
            ehr_id: None,
            object_id: None,
            token_id: None,
            tenant_id: None,
            timestamp: Timestamp::now(),
        }
    }
}

/// The result of enqueuing an event onto the system log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitOutcome {
    /// Successfully enqueued for the drain task.
    Enqueued,
    /// Dropped (queue full / drain gone) under `fail_mode=open`.
    Dropped,
    /// Rejected (queue full / drain gone) under `fail_mode=closed` — the REST
    /// layer must return `503`.
    Rejected,
}
