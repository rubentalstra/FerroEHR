// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The FHIR R4 `AuditEvent` renderer of the ATNA audit trail.
//!
//! The platform's system log decides what an audit record says — the IHE BALP
//! codings, role direction, entities and profile claims; this module decides how
//! that becomes a FHIR resource, building the typed
//! [`fhir_model::r4b::resources::AuditEvent`] from the neutral [`AuditRecord`].
//!
//! No openEHR spec governs FHIR resource representation — our own
//! design/extension. The rendered document is R4, and `AuditEvent` is unchanged
//! between the releases — R4B's own page records "No Changes"
//! (<https://hl7.org/fhir/R4B/auditevent.html>) — so the crate's `r4b`
//! generation builds it faithfully.

use fhir_model::r4b::codes::{AuditEventAction, AuditEventAgentNetworkType, AuditEventOutcome};
use fhir_model::r4b::resources::{
    AuditEvent, AuditEventAgent, AuditEventAgentNetwork, AuditEventEntity, AuditEventSource,
};
use fhir_model::r4b::types::{
    CodeableConcept, CodeableConceptInner, Coding, CodingInner, Identifier, IdentifierInner, Meta,
    MetaInner, Reference, ReferenceInner,
};
use fhir_model::{Base64Binary, Instant};

/// Why a resolved audit record could not be rendered as a FHIR `AuditEvent`.
#[derive(Debug, thiserror::Error)]
pub enum AuditRenderError {
    /// The record's instant is outside the range a FHIR `instant` can carry.
    #[error("audit timestamp is outside the representable FHIR instant range: {0}")]
    Timestamp(#[from] fhir_model::time::error::ComponentRange),
    /// A resource element the FHIR model requires was not supplied.
    #[error("building the FHIR AuditEvent: {0}")]
    Build(String),
    /// The built resource could not be serialized to JSON.
    #[error("serializing the FHIR AuditEvent: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// A FHIR `Coding`: a code in a code system, with optional display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCoding {
    /// The code system URI.
    pub system: String,
    /// The code.
    pub code: String,
    /// The display text.
    pub display: Option<String>,
}

/// The DICOM-style action code of an audited event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    /// Create (`C`).
    Create,
    /// Read (`R`).
    Read,
    /// Update (`U`).
    Update,
    /// Delete (`D`).
    Delete,
    /// Execute (`E`).
    Execute,
}

/// The DICOM-style outcome indicator of an audited event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Success (`0`).
    Success,
    /// Minor failure (`4`).
    MinorFailure,
    /// Serious failure (`8`).
    SeriousFailure,
    /// Major failure (`12`).
    MajorFailure,
}

/// One participant of an audited event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditAgent {
    /// The participation type (one coding).
    pub role: AuditCoding,
    /// Who the agent is, as a logical identifier.
    pub who: Option<String>,
    /// Whether this agent initiated the event.
    pub requestor: bool,
    /// Applicable policies (the BALP OAuth pattern records the token `jti`).
    pub policy: Vec<String>,
    /// The agent's network access point (an IP address).
    pub network_address: Option<String>,
}

/// The reporting source of an audited event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSourceRef {
    /// The logical site (the ATNA enterprise site id).
    pub site: Option<String>,
    /// The reporting node, as a logical identifier.
    pub observer: String,
    /// The audit source type codings.
    pub types: Vec<AuditCoding>,
}

/// One entity an audited event touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntityRef {
    /// What the entity is, as a logical identifier.
    pub what: Option<String>,
    /// The entity type coding.
    pub entity_type: Option<AuditCoding>,
    /// The entity role coding.
    pub role: Option<AuditCoding>,
    /// The search expression, carried verbatim (FHIR base64-encodes it).
    pub query: Option<String>,
}

/// A resolved audit record, in the neutral shape [`render`] turns into a FHIR
/// R4 `AuditEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    /// The profiles the record claims (`meta.profile`); empty claims none.
    pub profiles: Vec<String>,
    /// The event family coding (`AuditEvent.type`).
    pub event_type: AuditCoding,
    /// The concrete event kinds (`AuditEvent.subtype`).
    pub subtypes: Vec<AuditCoding>,
    /// The action performed.
    pub action: AuditAction,
    /// When the event was recorded.
    pub recorded: jiff::Timestamp,
    /// Whether the event succeeded or failed.
    pub outcome: AuditOutcome,
    /// Human description of a failure outcome.
    pub outcome_desc: Option<String>,
    /// The participating agents.
    pub agents: Vec<AuditAgent>,
    /// The reporting source.
    pub source: AuditSourceRef,
    /// The touched entities.
    pub entities: Vec<AuditEntityRef>,
}

/// Renders a resolved audit record as a FHIR R4 `AuditEvent` JSON document.
///
/// # Errors
///
/// [`AuditRenderError::Timestamp`] when the record's instant cannot be
/// expressed as a FHIR `instant`, [`AuditRenderError::Build`] when a required
/// resource element is missing, and [`AuditRenderError::Serialize`] when the
/// built resource cannot be serialized.
#[expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6, settled by #1885): the rendered FHIR \
              document leaves the typed model as JSON its carriers pass through untouched"
)]
pub fn render(record: &AuditRecord) -> Result<serde_json::Value, AuditRenderError> {
    let recorded = fhir_model::time::OffsetDateTime::from_unix_timestamp_nanos(
        record.recorded.as_nanosecond(),
    )?;

    let mut builder = AuditEvent::builder()
        .r#type(coding(&record.event_type))
        .subtype(record.subtypes.iter().map(|c| Some(coding(c))).collect())
        .action(action_code(record.action))
        .recorded(Instant(recorded))
        .outcome(outcome_code(record.outcome))
        .agent(record.agents.iter().map(agent).map(Some).collect())
        .source(source(&record.source))
        .entity(record.entities.iter().map(entity).map(Some).collect());
    if !record.profiles.is_empty() {
        builder = builder.meta(meta(&record.profiles));
    }
    if let Some(description) = &record.outcome_desc {
        builder = builder.outcome_desc(description.clone());
    }
    let resource = builder
        .build()
        // NOTE: carrying `fhir_model`'s builder error would leak a
        // dependency type into ours, making its patch bumps breaking.
        .map_err(|e| AuditRenderError::Build(e.to_string()))?;
    Ok(serde_json::to_value(resource)?)
}

/// The FHIR `Coding` for one neutral coding.
fn coding(source: &AuditCoding) -> Coding {
    CodingInner {
        id: None,
        extension: Vec::new(),
        system: Some(source.system.clone()),
        system_ext: None,
        version: None,
        version_ext: None,
        code: Some(source.code.clone()),
        code_ext: None,
        display: source.display.clone(),
        display_ext: None,
        user_selected: None,
        user_selected_ext: None,
    }
    .into()
}

/// A `Resource.meta` claiming the given profiles and nothing else.
fn meta(profiles: &[String]) -> Meta {
    MetaInner {
        id: None,
        extension: Vec::new(),
        version_id: None,
        version_id_ext: None,
        last_updated: None,
        last_updated_ext: None,
        source: None,
        source_ext: None,
        profile: profiles.iter().cloned().map(Some).collect(),
        profile_ext: Vec::new(),
        security: Vec::new(),
        security_ext: Vec::new(),
        tag: Vec::new(),
        tag_ext: Vec::new(),
    }
    .into()
}

/// A `Reference` carrying only a logical identifier (the audit trail records
/// identities, never server-local resource ids).
fn identifier_reference(value: &str) -> Reference {
    let identifier = IdentifierInner {
        id: None,
        extension: Vec::new(),
        r#use: None,
        r#use_ext: None,
        r#type: None,
        r#type_ext: None,
        system: None,
        system_ext: None,
        value: Some(value.to_owned()),
        value_ext: None,
        period: None,
        period_ext: None,
        assigner: None,
        assigner_ext: None,
    };
    ReferenceInner {
        id: None,
        extension: Vec::new(),
        reference: None,
        reference_ext: None,
        r#type: None,
        r#type_ext: None,
        identifier: Some(Identifier::from(identifier)),
        identifier_ext: None,
        display: None,
        display_ext: None,
    }
    .into()
}

/// A `CodeableConcept` carrying exactly one coding.
fn single_concept(source: &AuditCoding) -> CodeableConcept {
    CodeableConceptInner {
        id: None,
        extension: Vec::new(),
        coding: vec![Some(coding(source))],
        coding_ext: Vec::new(),
        text: None,
        text_ext: None,
    }
    .into()
}

/// The FHIR `AuditEvent.agent` for one participant.
fn agent(source: &AuditAgent) -> AuditEventAgent {
    AuditEventAgent {
        id: None,
        extension: Vec::new(),
        modifier_extension: Vec::new(),
        r#type: Some(single_concept(&source.role)),
        r#type_ext: None,
        role: Vec::new(),
        role_ext: Vec::new(),
        who: source.who.as_deref().map(identifier_reference),
        who_ext: None,
        alt_id: None,
        alt_id_ext: None,
        name: None,
        name_ext: None,
        requestor: source.requestor,
        requestor_ext: None,
        location: None,
        location_ext: None,
        policy: source.policy.iter().cloned().map(Some).collect(),
        policy_ext: Vec::new(),
        media: None,
        media_ext: None,
        network: source.network_address.as_ref().map(|address| {
            // NOTE: every address the audit trail records is an IP address —
            // FHIR `network-type` code `2` (no openEHR spec governs this —
            // our own design/extension).
            AuditEventAgentNetwork {
                id: None,
                extension: Vec::new(),
                modifier_extension: Vec::new(),
                address: Some(address.clone()),
                address_ext: None,
                r#type: Some(AuditEventAgentNetworkType::N2),
                r#type_ext: None,
            }
        }),
        network_ext: None,
        purpose_of_use: Vec::new(),
        purpose_of_use_ext: Vec::new(),
    }
}

/// The FHIR `AuditEvent.source`.
fn source(source: &AuditSourceRef) -> AuditEventSource {
    AuditEventSource {
        id: None,
        extension: Vec::new(),
        modifier_extension: Vec::new(),
        site: source.site.clone(),
        site_ext: None,
        observer: identifier_reference(&source.observer),
        observer_ext: None,
        r#type: source.types.iter().map(|c| Some(coding(c))).collect(),
        r#type_ext: Vec::new(),
    }
}

/// The FHIR `AuditEvent.entity` for one touched entity.
fn entity(source: &AuditEntityRef) -> AuditEventEntity {
    AuditEventEntity {
        id: None,
        extension: Vec::new(),
        modifier_extension: Vec::new(),
        what: source.what.as_deref().map(identifier_reference),
        what_ext: None,
        r#type: source.entity_type.as_ref().map(coding),
        r#type_ext: None,
        role: source.role.as_ref().map(coding),
        role_ext: None,
        lifecycle: None,
        lifecycle_ext: None,
        security_label: Vec::new(),
        security_label_ext: Vec::new(),
        name: None,
        name_ext: None,
        description: None,
        description_ext: None,
        query: source
            .query
            .as_ref()
            .map(|q| Base64Binary(q.as_bytes().to_vec())),
        query_ext: None,
        detail: Vec::new(),
        detail_ext: Vec::new(),
    }
}

/// The FHIR `audit-event-action` code.
const fn action_code(action: AuditAction) -> AuditEventAction {
    match action {
        AuditAction::Create => AuditEventAction::C,
        AuditAction::Read => AuditEventAction::R,
        AuditAction::Update => AuditEventAction::U,
        AuditAction::Delete => AuditEventAction::D,
        AuditAction::Execute => AuditEventAction::E,
    }
}

/// The FHIR `audit-event-outcome` code.
const fn outcome_code(outcome: AuditOutcome) -> AuditEventOutcome {
    match outcome {
        AuditOutcome::Success => AuditEventOutcome::N0,
        AuditOutcome::MinorFailure => AuditEventOutcome::N4,
        AuditOutcome::SeriousFailure => AuditEventOutcome::N8,
        AuditOutcome::MajorFailure => AuditEventOutcome::N12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> AuditRecord {
        AuditRecord {
            profiles: vec!["https://example.test/Profile".to_owned()],
            event_type: AuditCoding {
                system: "http://terminology.hl7.org/CodeSystem/audit-event-type".to_owned(),
                code: "rest".to_owned(),
                display: Some("RESTful Operation".to_owned()),
            },
            subtypes: vec![AuditCoding {
                system: "http://hl7.org/fhir/restful-interaction".to_owned(),
                code: "read".to_owned(),
                display: Some("read".to_owned()),
            }],
            action: AuditAction::Read,
            recorded: "2026-07-06T12:00:00Z"
                .parse::<jiff::Timestamp>()
                .expect("timestamp"),
            outcome: AuditOutcome::Success,
            outcome_desc: None,
            agents: vec![AuditAgent {
                role: AuditCoding {
                    system: "http://dicom.nema.org/resources/ontology/DCM".to_owned(),
                    code: "110152".to_owned(),
                    display: Some("Destination Role ID".to_owned()),
                },
                who: Some("john doe".to_owned()),
                requestor: true,
                policy: vec!["jti-1".to_owned()],
                network_address: Some("10.216.24.150".to_owned()),
            }],
            source: AuditSourceRef {
                site: Some("site-1".to_owned()),
                observer: "ferroehr".to_owned(),
                types: vec![AuditCoding {
                    system: "http://terminology.hl7.org/CodeSystem/security-source-type".to_owned(),
                    code: "4".to_owned(),
                    display: Some("Application Server".to_owned()),
                }],
            },
            entities: vec![AuditEntityRef {
                what: None,
                entity_type: None,
                role: None,
                query: Some("eu.ferroehr::q1".to_owned()),
            }],
        }
    }

    #[test]
    fn round_trips_through_the_typed_r4b_model() {
        let rendered = render(&record()).expect("render");
        let parsed: AuditEvent = serde_json::from_value(rendered.clone()).expect("parse");
        assert_eq!(
            serde_json::to_value(parsed).expect("re-serialize"),
            rendered
        );
    }

    #[test]
    fn the_query_entity_is_base64_encoded_by_the_model() {
        let rendered = render(&record()).expect("render");
        assert_eq!(rendered["entity"][0]["query"], "ZXUuZmVycm9laHI6OnEx");
    }

    #[test]
    fn no_profiles_claims_no_meta() {
        let mut record = record();
        record.profiles.clear();
        assert!(render(&record).expect("render").get("meta").is_none());
    }
}
