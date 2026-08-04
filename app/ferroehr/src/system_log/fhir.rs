//! FHIR R4 `AuditEvent` rendering of the audit event model, following the
//! IHE **BALP** (Basic Audit Log Patterns) content profiles.
//!
//! This is the modern half of the dual ATNA rendering (the classic half is
//! the DICOM PS3.15 §A.5 XML in [`super::message`]): the same resolved
//! [`super::event::AuditEvent`] renders to one FHIR R4 (4.0.1)
//! `AuditEvent` JSON document. The BALP profiles pin the codings this module
//! emits (IHE BALP v1.1.4, `IHE.BasicAudit.*` StructureDefinitions):
//!
//! - `RESTful` operations: `type` = `rest`, `subtype` = the FHIR
//!   restful-interaction class + the concrete ITS-REST operation id.
//! - Role direction per BALP: on a **Read** the server is the *source* of
//!   the data (DCM 110153) and the client the *destination* (DCM 110152);
//!   on every other action the client initiates as source
//!   (`IHE.BasicAudit.PatientRead` vs `IHE.BasicAudit.PatientQuery`
//!   agent:client/agent:server fixed codings).
//! - Patient entity: `audit-entity-type` code `1` (Person) +
//!   `object-role` code `1` (Patient), the resolved EHR subject as an
//!   identifier-only reference.
//! - Query entity: `audit-entity-type` `2` + `object-role` `24`, the search
//!   expression base64-encoded in `entity.query`
//!   (`IHE.BasicAudit.Query`/`PatientQuery`).
//! - Bearer token identity: an `agent` typed
//!   `UserAgentTypes#UserOauthAgent` carrying the token `jti` in
//!   `agent.policy` (`IHE.BasicAudit.OAUTHaccessTokenUse.Minimal` — minimal
//!   by design: never the token itself).
//!
//! `meta.profile` claims the matching BALP profile only when the record
//! actually satisfies it (the BALP `RESTful` profiles fix `outcome` = `0`, so
//! failures carry no claim; the `Patient*` variants additionally require the
//! resolved patient entity). Plain typed serde structs — no FHIR crate; the
//! subset below is exactly what the BALP patterns need.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::system_log::event::{AuditEvent, EventActionCode, EventOutcome, EventType, ObjectClass};
use crate::system_log::message::AuditContext;

// ── Code systems (FHIR R4 / IHE BALP fixed bindings) ─────────────────────────

/// FHIR `audit-event-type` code system (`rest`).
pub const SYS_AUDIT_EVENT_TYPE: &str = "http://terminology.hl7.org/CodeSystem/audit-event-type";
/// FHIR restful-interaction code system (`create`/`read`/…).
pub const SYS_RESTFUL_INTERACTION: &str = "http://hl7.org/fhir/restful-interaction";
/// DICOM controlled terminology (DCM) system URI.
pub const SYS_DCM: &str = "http://dicom.nema.org/resources/ontology/DCM";
/// FHIR `audit-entity-type` code system (`1` person / `2` system object).
pub const SYS_AUDIT_ENTITY_TYPE: &str = "http://terminology.hl7.org/CodeSystem/audit-entity-type";
/// FHIR `object-role` code system (`1` patient / `24` query).
pub const SYS_OBJECT_ROLE: &str = "http://terminology.hl7.org/CodeSystem/object-role";
/// FHIR `security-source-type` code system (`4` application server).
pub const SYS_SECURITY_SOURCE_TYPE: &str =
    "http://terminology.hl7.org/CodeSystem/security-source-type";
/// IHE BALP `UserAgentTypes` code system (`UserOauthAgent`).
pub const SYS_BALP_USER_AGENT_TYPES: &str =
    "https://profiles.ihe.net/ITI/BALP/CodeSystem/UserAgentTypes";
/// The system for ITS-REST operation-id subtype codings.
///
/// NOTE: no external code system governs openEHR REST operations — our own
/// design/extension (the same ids the DICOM rendering emits under the
/// `openEHR-ITS-REST` `codeSystemName`).
pub const SYS_OPENEHR_ITS_REST: &str = "urn:openehr:its-rest:operation";

/// The canonical-URL prefix of the IHE BALP profiles (v1.1.4).
pub const BALP_PROFILE_BASE: &str = "https://profiles.ihe.net/ITI/BALP/StructureDefinition";

// ── The FHIR AuditEvent subset (R4 4.0.1) ────────────────────────────────────

/// A FHIR `Coding`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coding {
    /// The code system URI.
    pub system: String,
    /// The code.
    pub code: String,
    /// The display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Coding {
    fn new(system: &str, code: &str, display: &str) -> Self {
        Coding {
            system: system.to_owned(),
            code: code.to_owned(),
            display: Some(display.to_owned()),
        }
    }
}

/// A FHIR `CodeableConcept` (codings only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeableConcept {
    /// The codings.
    pub coding: Vec<Coding>,
}

/// A FHIR `Identifier` (value only — the identities here are plain ids).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    /// The identifier value.
    pub value: String,
}

/// A FHIR `Reference` carried as a logical identifier (no server-local
/// resource ids exist for the identities the audit trail records).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    /// The logical identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Identifier>,
    /// The display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Reference {
    fn identifier(value: &str) -> Self {
        Reference {
            identifier: Some(Identifier {
                value: value.to_owned(),
            }),
            display: None,
        }
    }
}

/// `AuditEvent.agent.network`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentNetwork {
    /// The network address (IP).
    pub address: String,
    /// The network address type: `2` = IP address (FHIR `network-type`).
    #[serde(rename = "type")]
    pub type_code: String,
}

/// `AuditEvent.agent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The participation type of the agent.
    #[serde(rename = "type")]
    pub type_concept: CodeableConcept,
    /// Who the agent is (identifier-only reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub who: Option<Reference>,
    /// Whether this agent initiated the event.
    pub requestor: bool,
    /// Applicable policies — the BALP OAuth minimal pattern records the
    /// token `jti` here (`IHE.BasicAudit.OAUTHaccessTokenUse.Minimal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<Vec<String>>,
    /// The agent's network access point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<AgentNetwork>,
}

/// `AuditEvent.source`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The logical site (the ATNA enterprise site id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    /// The reporting node (identifier-only reference; 1..1 in every BALP
    /// profile).
    pub observer: Reference,
    /// The audit source type — `4` application server
    /// (`security-source-type`).
    #[serde(rename = "type")]
    pub type_codings: Vec<Coding>,
}

/// `AuditEvent.entity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    /// What the entity is (identifier-only reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub what: Option<Reference>,
    /// The entity type (`audit-entity-type`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_coding: Option<Coding>,
    /// The entity role (`object-role`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Coding>,
    /// The base64-encoded query/search expression
    /// (`IHE.BasicAudit.Query`/`PatientQuery` `entity:query.query`, 1..1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// `Resource.meta` (profile claims only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// The claimed profiles (canonical URLs).
    pub profile: Vec<String>,
}

/// A FHIR R4 `AuditEvent` — the BALP subset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAuditEvent {
    /// Always `"AuditEvent"`.
    pub resource_type: String,
    /// Claimed BALP profiles (present only when the record satisfies one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    /// The event family (`rest`, or the DICOM event id for non-REST events).
    #[serde(rename = "type")]
    pub type_coding: Coding,
    /// The concrete event kind(s): the restful-interaction class and/or the
    /// ITS-REST operation id / DCM login code.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subtype: Vec<Coding>,
    /// The DICOM-style action code (`C`/`R`/`U`/`D`/`E`).
    pub action: String,
    /// The event time (FHIR `instant`).
    pub recorded: String,
    /// The outcome indicator (`0`/`4`/`8`/`12`, as in DICOM).
    pub outcome: String,
    /// Human description of a failure outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_desc: Option<String>,
    /// The participating agents (client, server, optional token agent).
    pub agent: Vec<Agent>,
    /// The reporting source.
    pub source: Source,
    /// The touched entities (patient / data object / query).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entity: Vec<Entity>,
}

/// Render a resolved [`AuditEvent`] to a FHIR R4 `AuditEvent` following the
/// IHE BALP patterns, with the server identity from the [`AuditContext`] and
/// the optionally-resolved patient subject id.
#[must_use]
pub fn to_fhir(event: &AuditEvent, ctx: &AuditContext, subject: Option<&str>) -> FhirAuditEvent {
    let missing = ctx.value_if_missing.as_str();

    let (type_coding, interaction) = type_and_interaction(event);

    let mut subtype = Vec::new();
    if let Some(i) = interaction {
        subtype.push(i);
    }
    match event.event_type {
        Some(EventType::Login) => subtype.push(Coding::new(SYS_DCM, "110122", "Login")),
        Some(EventType::Logout) => subtype.push(Coding::new(SYS_DCM, "110123", "Logout")),
        Some(EventType::RestOperation(op)) => {
            subtype.push(Coding::new(SYS_OPENEHR_ITS_REST, op, op));
        }
        None => {}
    }

    let agent = build_agents(event, ctx, missing);
    let entity = build_entities(event, subject, missing);
    let profile = claimed_profiles(event, subject);

    FhirAuditEvent {
        resource_type: "AuditEvent".to_owned(),
        meta: (!profile.is_empty()).then_some(Meta { profile }),
        type_coding,
        subtype,
        action: action_char(event.action).to_string(),
        recorded: event.timestamp.to_string(),
        outcome: outcome_code(event.outcome).to_owned(),
        outcome_desc: match event.outcome {
            EventOutcome::Success => None,
            _ => Some("Operation failed".to_owned()),
        },
        agent,
        source: Source {
            site: nonempty_opt(&ctx.enterprise_site_id),
            observer: Reference::identifier(&nonempty(&ctx.source_id, missing)),
            type_codings: vec![Coding::new(
                SYS_SECURITY_SOURCE_TYPE,
                "4",
                "Application Server",
            )],
        },
        entity,
    }
}

/// The `type` coding + the restful-interaction `subtype` for the event.
/// REST-dispatched resource operations are `rest`; the service-level events
/// keep their DICOM event id as the `type` (extract export/import, user
/// authentication) — the BALP `RESTful` patterns do not cover them.
fn type_and_interaction(event: &AuditEvent) -> (Coding, Option<Coding>) {
    match event.object {
        ObjectClass::Authentication => {
            (Coding::new(SYS_DCM, "110114", "User Authentication"), None)
        }
        ObjectClass::Extract => match event.action {
            EventActionCode::Create | EventActionCode::Update => {
                (Coding::new(SYS_DCM, "110107", "Import"), None)
            }
            _ => (Coding::new(SYS_DCM, "110106", "Export"), None),
        },
        ObjectClass::Query => (
            Coding::new(SYS_AUDIT_EVENT_TYPE, "rest", "RESTful Operation"),
            Some(Coding::new(
                SYS_RESTFUL_INTERACTION,
                "search-type",
                "search-type",
            )),
        ),
        _ => (
            Coding::new(SYS_AUDIT_EVENT_TYPE, "rest", "RESTful Operation"),
            Some(match event.action {
                EventActionCode::Create => Coding::new(SYS_RESTFUL_INTERACTION, "create", "create"),
                EventActionCode::Read => Coding::new(SYS_RESTFUL_INTERACTION, "read", "read"),
                EventActionCode::Update => Coding::new(SYS_RESTFUL_INTERACTION, "update", "update"),
                EventActionCode::Delete => Coding::new(SYS_RESTFUL_INTERACTION, "delete", "delete"),
                EventActionCode::Execute => {
                    Coding::new(SYS_RESTFUL_INTERACTION, "operation", "operation")
                }
            }),
        ),
    }
}

/// The client/server agents (+ the OAuth token agent when a `jti` is known).
///
/// Role direction per the BALP fixed codings: on a **Read** the server is the
/// source of the data (DCM 110153 "Source Role ID") and the client the
/// destination (110152); on every other action the initiating client is the
/// source (`IHE.BasicAudit.PatientRead` vs `PatientQuery` agent slices).
fn build_agents(event: &AuditEvent, ctx: &AuditContext, missing: &str) -> Vec<Agent> {
    let read = matches!(event.action, EventActionCode::Read);
    let (client_role, server_role) = if read {
        (
            Coding::new(SYS_DCM, "110152", "Destination Role ID"),
            Coding::new(SYS_DCM, "110153", "Source Role ID"),
        )
    } else {
        (
            Coding::new(SYS_DCM, "110153", "Source Role ID"),
            Coding::new(SYS_DCM, "110152", "Destination Role ID"),
        )
    };

    let user = nonempty(&event.user_id, missing);
    let client_ip = event
        .client_ip
        .clone()
        .unwrap_or_else(|| missing.to_owned());

    let mut agents = vec![
        // The requesting client, carrying the authenticated user identity
        // (the FHIR twin of the DICOM source ActiveParticipant).
        Agent {
            type_concept: CodeableConcept {
                coding: vec![client_role],
            },
            who: Some(Reference::identifier(&user)),
            requestor: true,
            policy: None,
            network: Some(AgentNetwork {
                address: client_ip,
                type_code: "2".to_owned(),
            }),
        },
        // This server.
        Agent {
            type_concept: CodeableConcept {
                coding: vec![server_role],
            },
            who: Some(Reference::identifier(&nonempty(&ctx.source_id, missing))),
            requestor: false,
            policy: None,
            network: Some(AgentNetwork {
                address: nonempty(&ctx.server_ip, missing),
                type_code: "2".to_owned(),
            }),
        },
    ];

    if let Some(jti) = event.token_id.as_deref().filter(|s| !s.is_empty()) {
        // IHE.BasicAudit.OAUTHaccessTokenUse.Minimal: the token identity is
        // ONLY the jti, carried in agent.policy — never the token contents.
        agents.push(Agent {
            type_concept: CodeableConcept {
                coding: vec![Coding::new(
                    SYS_BALP_USER_AGENT_TYPES,
                    "UserOauthAgent",
                    "User OAuth Agent participant",
                )],
            },
            who: None,
            requestor: true,
            policy: Some(vec![jti.to_owned()]),
            network: None,
        });
    }

    agents
}

/// The entity list: the patient (when resolved), the touched data object,
/// and/or the query expression.
fn build_entities(event: &AuditEvent, subject: Option<&str>, missing: &str) -> Vec<Entity> {
    use crate::system_log::codes::AtnaObject;

    let mut entities = Vec::new();

    if event.object.is_query() {
        // NOTE: the audit layer carries the qualified stored-query name, not
        // ad-hoc AQL text; it is base64-encoded per the BALP query pattern
        // ("base64 encoding preserves exactly what was requested").
        let expression = event
            .object_id
            .clone()
            .unwrap_or_else(|| missing.to_owned());
        entities.push(Entity {
            what: None,
            type_coding: Some(Coding::new(SYS_AUDIT_ENTITY_TYPE, "2", "System Object")),
            role: Some(Coding::new(SYS_OBJECT_ROLE, "24", "Query")),
            query: Some(base64::engine::general_purpose::STANDARD.encode(expression.as_bytes())),
        });
        return entities;
    }

    if event.object.is_patient_centric()
        && let Some(subject) = subject
    {
        entities.push(Entity {
            what: Some(Reference::identifier(subject)),
            type_coding: Some(Coding::new(SYS_AUDIT_ENTITY_TYPE, "1", "Person")),
            role: Some(Coding::new(SYS_OBJECT_ROLE, "1", "Patient")),
            query: None,
        });
    }

    if let Some(id) = event.object_id.as_deref().filter(|s| !s.is_empty()) {
        entities.push(Entity {
            what: Some(Reference::identifier(id)),
            type_coding: Some(Coding::new(SYS_AUDIT_ENTITY_TYPE, "2", "System Object")),
            role: None,
            query: None,
        });
    }

    entities
}

/// The BALP profile(s) this record actually satisfies. The `RESTful` profiles
/// fix `outcome = 0`, so failures claim nothing; the `Patient*` variants
/// require the resolved patient entity.
fn claimed_profiles(event: &AuditEvent, subject: Option<&str>) -> Vec<String> {
    use crate::system_log::codes::AtnaObject;

    let mut profiles = Vec::new();
    if event.outcome == EventOutcome::Success {
        let patient = event.object.is_patient_centric() && subject.is_some();
        let name = match (event.object, event.action) {
            (ObjectClass::Authentication | ObjectClass::Extract, _) => None,
            (ObjectClass::Query, _) => Some(if patient { "PatientQuery" } else { "Query" }),
            (_, EventActionCode::Create) => Some(if patient { "PatientCreate" } else { "Create" }),
            (_, EventActionCode::Read) => Some(if patient { "PatientRead" } else { "Read" }),
            (_, EventActionCode::Update) => Some(if patient { "PatientUpdate" } else { "Update" }),
            (_, EventActionCode::Delete) => Some(if patient { "PatientDelete" } else { "Delete" }),
            (_, EventActionCode::Execute) => None,
        };
        if let Some(name) = name {
            profiles.push(format!("{BALP_PROFILE_BASE}/IHE.BasicAudit.{name}"));
        }
    }
    if event.token_id.as_deref().is_some_and(|s| !s.is_empty()) {
        profiles.push(format!(
            "{BALP_PROFILE_BASE}/IHE.BasicAudit.OAUTHaccessTokenUse.Minimal"
        ));
    }
    profiles
}

const fn action_char(action: EventActionCode) -> char {
    match action {
        EventActionCode::Create => 'C',
        EventActionCode::Read => 'R',
        EventActionCode::Update => 'U',
        EventActionCode::Delete => 'D',
        EventActionCode::Execute => 'E',
    }
}

const fn outcome_code(outcome: EventOutcome) -> &'static str {
    match outcome {
        EventOutcome::Success => "0",
        EventOutcome::MinorFailure => "4",
        EventOutcome::SeriousFailure => "8",
        EventOutcome::MajorFailure => "12",
    }
}

fn nonempty(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

fn nonempty_opt(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;

    fn ctx() -> AuditContext {
        AuditContext {
            source_id: "ferroehr".to_owned(),
            enterprise_site_id: "1f332a66-0000-0000-0000-000000000001".to_owned(),
            server_ip: "10.42.23.77".to_owned(),
            value_if_missing: "UNKNOWN".to_owned(),
        }
    }

    fn event(action: EventActionCode, object: ObjectClass, outcome: EventOutcome) -> AuditEvent {
        let mut e = AuditEvent::new(action, object, outcome);
        e.user_id = "john doe".to_owned();
        e.client_ip = Some("10.216.24.150".to_owned());
        e.timestamp = "2026-07-06T12:00:00Z".parse::<Timestamp>().unwrap();
        e
    }

    fn json(event: &AuditEvent, subject: Option<&str>) -> serde_json::Value {
        serde_json::to_value(to_fhir(event, &ctx(), subject)).expect("serialize")
    }

    #[test]
    fn golden_patient_composition_read() {
        // A successful composition read with a resolved subject claims the
        // BALP PatientRead profile; on a Read the server is the data source
        // (DCM 110153) and the client the destination (110152).
        let mut e = event(
            EventActionCode::Read,
            ObjectClass::Composition,
            EventOutcome::Success,
        );
        e.object_id = Some("8fa1::ferroehr::1".to_owned());
        e.event_type = Some(EventType::RestOperation("composition_get"));
        let v = json(&e, Some("patient-42"));
        insta::assert_json_snapshot!("fhir_patient_composition_read", v);
    }

    #[test]
    fn patient_read_claims_patient_profile_and_roles() {
        let mut e = event(
            EventActionCode::Read,
            ObjectClass::Composition,
            EventOutcome::Success,
        );
        e.object_id = Some("8fa1::ferroehr::1".to_owned());
        let v = json(&e, Some("patient-42"));
        assert_eq!(
            v["meta"]["profile"][0],
            "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientRead"
        );
        // Read: client = destination (110152), server = source (110153).
        assert_eq!(v["agent"][0]["type"]["coding"][0]["code"], "110152");
        assert_eq!(v["agent"][1]["type"]["coding"][0]["code"], "110153");
        // The patient entity: person(1)/patient(1) with the subject id.
        assert_eq!(v["entity"][0]["type"]["code"], "1");
        assert_eq!(v["entity"][0]["role"]["code"], "1");
        assert_eq!(v["entity"][0]["what"]["identifier"]["value"], "patient-42");
        // The data entity: the composition uid.
        assert_eq!(
            v["entity"][1]["what"]["identifier"]["value"],
            "8fa1::ferroehr::1"
        );
    }

    #[test]
    fn create_uses_source_role_for_client() {
        let e = event(
            EventActionCode::Create,
            ObjectClass::Ehr,
            EventOutcome::Success,
        );
        let v = json(&e, Some("patient-42"));
        // Non-read: client initiates as source (110153).
        assert_eq!(v["agent"][0]["type"]["coding"][0]["code"], "110153");
        assert_eq!(v["agent"][1]["type"]["coding"][0]["code"], "110152");
        assert_eq!(v["subtype"][0]["code"], "create");
        assert_eq!(
            v["meta"]["profile"][0],
            "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientCreate"
        );
    }

    #[test]
    fn unresolved_subject_falls_back_to_non_patient_profile() {
        let e = event(
            EventActionCode::Read,
            ObjectClass::Ehr,
            EventOutcome::Success,
        );
        let v = json(&e, None);
        assert_eq!(
            v["meta"]["profile"][0],
            "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read"
        );
        // No patient entity without a resolved subject.
        assert!(v["entity"].as_array().is_none_or(Vec::is_empty));
    }

    #[test]
    fn failure_claims_no_profile() {
        // The BALP RESTful profiles fix outcome = 0; a failure record keeps
        // the shape but claims nothing.
        let e = event(
            EventActionCode::Read,
            ObjectClass::Composition,
            EventOutcome::MinorFailure,
        );
        let v = json(&e, Some("patient-42"));
        assert!(v.get("meta").is_none());
        assert_eq!(v["outcome"], "4");
        assert_eq!(v["outcomeDesc"], "Operation failed");
    }

    #[test]
    fn query_entity_is_base64_search_criteria() {
        let mut e = event(
            EventActionCode::Execute,
            ObjectClass::Query,
            EventOutcome::Success,
        );
        e.object_id = Some("eu.ferroehr::q1".to_owned());
        let v = json(&e, None);
        assert_eq!(v["type"]["code"], "rest");
        assert_eq!(v["subtype"][0]["code"], "search-type");
        assert_eq!(v["entity"][0]["role"]["code"], "24");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(v["entity"][0]["query"].as_str().expect("query"))
            .expect("base64");
        assert_eq!(decoded, b"eu.ferroehr::q1");
        assert_eq!(
            v["meta"]["profile"][0],
            "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Query"
        );
    }

    #[test]
    fn bearer_token_adds_oauth_minimal_agent() {
        let mut e = event(
            EventActionCode::Read,
            ObjectClass::Composition,
            EventOutcome::Success,
        );
        e.token_id = Some("jti-abc-123".to_owned());
        let v = json(&e, Some("patient-42"));
        let agents = v["agent"].as_array().expect("agents");
        assert_eq!(agents.len(), 3);
        assert_eq!(agents[2]["type"]["coding"][0]["code"], "UserOauthAgent");
        assert_eq!(agents[2]["policy"][0], "jti-abc-123");
        assert_eq!(agents[2]["requestor"], true);
        // Both the activity profile and the token-use profile are claimed.
        let profiles = v["meta"]["profile"].as_array().expect("profiles");
        assert!(profiles.iter().any(|p| {
            p.as_str()
                .is_some_and(|s| s.ends_with("IHE.BasicAudit.OAUTHaccessTokenUse.Minimal"))
        }));
    }

    #[test]
    fn login_record_is_user_authentication() {
        let mut e = event(
            EventActionCode::Execute,
            ObjectClass::Authentication,
            EventOutcome::Success,
        );
        e.event_type = Some(EventType::Login);
        let v = json(&e, None);
        assert_eq!(v["type"]["code"], "110114");
        assert_eq!(v["subtype"][0]["code"], "110122");
        assert!(v.get("meta").is_none());
        insta::assert_json_snapshot!("fhir_login_success", v);
    }

    #[test]
    fn extract_directions_use_export_import() {
        let out = event(
            EventActionCode::Read,
            ObjectClass::Extract,
            EventOutcome::Success,
        );
        assert_eq!(json(&out, None)["type"]["code"], "110106");
        let inbound = event(
            EventActionCode::Create,
            ObjectClass::Extract,
            EventOutcome::Success,
        );
        assert_eq!(json(&inbound, None)["type"]["code"], "110107");
    }

    #[test]
    fn round_trips_through_serde() {
        let mut e = event(
            EventActionCode::Read,
            ObjectClass::Composition,
            EventOutcome::Success,
        );
        e.object_id = Some("8fa1::ferroehr::1".to_owned());
        e.token_id = Some("jti-1".to_owned());
        let rendered = to_fhir(&e, &ctx(), Some("patient-42"));
        let text = serde_json::to_string(&rendered).expect("serialize");
        let parsed: FhirAuditEvent = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, rendered);
    }
}
